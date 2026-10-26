//! Toom-3 multiplication for large operands, on top of Karatsuba ([`mul::mul`]), and
//! [`decode_toom3`], the divide-and-conquer decoder using it.

use crate::dc::{self, Multiplier};
use crate::mul::{self, add_assign, sub_assign};
use crate::{BufferTooSmall, Error, encode};

#[cfg(all(test, miri))]
#[path = "toom3_miri_tests.rs"]
mod miri_tests;

/// Operands shorter than this use [`mul::mul`]. Tuned on whole decodes on an M5 Pro.
const THRESHOLD: usize = 128;

/// [`THRESHOLD`] when [`mul::mul`] has the AVX-512 IFMA leaf. Tuned on whole decodes on
/// Zen 5.
const IFMA_THRESHOLD: usize = 1536;

/// Smallest threshold for which `tests::scratch_fits` holds.
const MIN_THRESHOLD: usize = 32;
const _: () = assert!(THRESHOLD >= MIN_THRESHOLD && IFMA_THRESHOLD >= MIN_THRESHOLD);

/// Like `decode_karatsuba`, but uses Toom-3 for large products.
pub fn decode_toom3(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    let ifma = mul::ifma_threshold() != 0;
    decode_toom3_with(input, output, if ifma { IFMA_THRESHOLD } else { THRESHOLD })
}

/// [`decode_toom3`] with Toom-3 for operands of at least `threshold` limbs. Panics if
/// `threshold` is below 32.
pub fn decode_toom3_with(input: &str, output: &mut [u8], threshold: usize) -> Result<usize, Error> {
    assert!(
        threshold >= MIN_THRESHOLD,
        "Toom-3 threshold below {MIN_THRESHOLD}"
    );
    dc::decode_with(input, output, &mut Toom3 { threshold })
}

/// Like `encode_karatsuba`, but uses Toom-3 for large products.
pub fn encode_toom3(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    let threshold = if mul::ifma_threshold() != 0 {
        IFMA_THRESHOLD
    } else {
        THRESHOLD
    };
    encode::dc::internals::encode_with(input, output, &mut Toom3 { threshold })
}

/// [`mul_with`] as a [`Multiplier`].
struct Toom3 {
    threshold: usize,
}

impl Multiplier for Toom3 {
    fn scratch_len(&self, n: usize) -> usize {
        mul::scratch_len(n)
    }

    fn mul(&mut self, out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]) {
        mul_with(out, a, b, scratch, self.threshold);
    }
}

/// `out = a * b`, with the same contract as [`mul::mul`], including the scratch bound,
/// using Toom-3 for operands of at least `threshold >= MIN_THRESHOLD` limbs.
///
/// A Toom-3 step on `n` limbs uses `6k + 6` scratch limbs for `k = ceil(n / 3)`, then
/// recurses on at most `k + 1` limbs. Cutting `b` into blocks of `a.len() < n` limbs uses
/// `2 * a.len()`, then recurses on `a.len()`. Both fit in `mul::scratch_len` by induction;
/// `tests::scratch_fits` checks this.
fn mul_with(out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64], threshold: usize) {
    debug_assert_eq!(out.len(), a.len() + b.len());
    let (a, b) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if a.len() < threshold {
        mul::mul(out, a, b, scratch);
    } else if a.len() > 2 * b.len().div_ceil(3) {
        toom3(out, a, b, scratch, threshold);
    } else {
        // Each block product suits Toom-3.
        mul::mul_blocks(out, a, b, scratch, |o, x, y, s| {
            mul_with(o, x, y, s, threshold)
        });
    }
}

/// Requires `2 * ceil(b.len() / 3) < a.len() <= b.len()`.
fn toom3(out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64], threshold: usize) {
    // a = a2·X^2 + a1·X + a0, and likewise b, with non-empty `a2` and `b2`. Then
    // a·b = r(X) for r(t) = c4·t^4 + ... + c0, found from its values at 0, 1, -1, 2, ∞.
    let k = b.len().div_ceil(3);
    let (a0, a12) = a.split_at(k);
    let (a1, a2) = a12.split_at(k);
    let (b0, b12) = b.split_at(k);
    let (b1, b2) = b12.split_at(k);

    let (v1, scratch) = scratch.split_at_mut(2 * k + 2);
    let (vm1, scratch) = scratch.split_at_mut(2 * k + 2);
    let (v2, scratch) = scratch.split_at_mut(2 * k + 2);
    // Until they are needed, `out[2k..]` holds the operands at 1, and `v1` those at
    // -1 and 2. `out` has at least `4k + 2` limbs since `a2` and `b2` are non-empty.
    let (pa1, pb1) = out[2 * k..4 * k + 2].split_at_mut(k + 1);
    let (pa, pb) = v1.split_at_mut(k + 1);
    // r(-1) = (a0 - a1 + a2)(b0 - b1 + b2), as magnitude `vm1` and sign `neg`.
    let neg = eval_1(pa1, pa, a0, a1, a2) != eval_1(pb1, pb, b0, b1, b2);
    mul_with(vm1, pa, pb, scratch, threshold);
    // r(2) = (a0 + 2·a1 + 4·a2)(b0 + 2·b1 + 4·b2)
    eval_2(pa, pa1, a0, a2);
    eval_2(pb, pb1, b0, b2);
    mul_with(v2, pa, pb, scratch, threshold);
    // r(1) = (a0 + a1 + a2)(b0 + b1 + b2)
    mul_with(v1, pa1, pb1, scratch, threshold);
    // r(0) = c0 and r(∞) = c4 go to their place in `out`.
    let (v0, rest) = out.split_at_mut(2 * k);
    let (mid, vinf) = rest.split_at_mut(2 * k);
    mul_with(v0, a0, b0, scratch, threshold);
    mul_with(vinf, a2, b2, scratch, threshold);
    mid.fill(0);

    // Every value below is a sum of non-negative c_i, so nothing wraps.
    let mut wrap = false;
    // (r(2) - r(-1)) / 3 = c1 + c2 + 3·c3 + 5·c4
    wrap |= if neg {
        add_assign(v2, vm1)
    } else {
        sub_assign(v2, vm1)
    };
    div3(v2);
    // (r(1) ∓ |r(-1)|) / 2 are c1 + c3 and c0 + c2 + c4, in an order set by the sign.
    wrap |= sub_assign(v1, vm1);
    shr1(v1);
    wrap |= add_assign(vm1, v1);
    let (odd, even) = if neg { (vm1, v1) } else { (v1, vm1) };
    wrap |= sub_assign(even, v0); // c2 + c4
    wrap |= sub_assign(v2, odd); // c2 + 2·c3 + 5·c4
    wrap |= sub_assign(v2, even); // 2·c3 + 4·c4
    shr1(v2); // c3 + 2·c4
    wrap |= sub_assign(even, vinf); // c2
    wrap |= sub_assign(v2, vinf) | sub_assign(v2, vinf); // c3
    wrap |= sub_assign(odd, v2); // c1
    debug_assert!(!wrap);
    add_at(out, k, odd);
    add_at(out, 2 * k, even);
    add_at(out, 3 * k, v2);
}

/// Writes `x0 + x1 + x2` to `p1` and `|x0 - x1 + x2|` to `pm1`, and returns whether
/// `x0 - x1 + x2 < 0`. Requires `p1` and `pm1` to be longer than `x0`, `x1` and `x2`.
fn eval_1(p1: &mut [u64], pm1: &mut [u64], x0: &[u64], x1: &[u64], x2: &[u64]) -> bool {
    set(p1, x0);
    let mut wrap = add_assign(p1, x2);
    let neg = less(p1, x1);
    if neg {
        set(pm1, x1);
        wrap |= sub_assign(pm1, p1);
    } else {
        pm1.copy_from_slice(p1);
        wrap |= sub_assign(pm1, x1);
    }
    wrap |= add_assign(p1, x1);
    debug_assert!(!wrap);
    neg
}

/// Writes `x0 + 2·x1 + 4·x2` to `p2`, given `p1 = x0 + x1 + x2` of the same length.
fn eval_2(p2: &mut [u64], p1: &[u64], x0: &[u64], x2: &[u64]) {
    // 2·(p1 + x2) - x0
    p2.copy_from_slice(p1);
    let carry = add_assign(p2, x2) | shl1(p2);
    let borrow = sub_assign(p2, x0);
    debug_assert!(!carry && !borrow);
}

/// `out = x`, zero-extended. Requires `out` to be at least as long as `x`.
fn set(out: &mut [u64], x: &[u64]) {
    let (lo, hi) = out.split_at_mut(x.len());
    lo.copy_from_slice(x);
    hi.fill(0);
}

/// Whether `x < y`. Requires `x` to be at least as long as `y`.
fn less(x: &[u64], y: &[u64]) -> bool {
    let (lo, hi) = x.split_at(y.len());
    hi.iter().all(|&l| l == 0) && lo.iter().rev().lt(y.iter().rev())
}

/// `out[at..] += x`, where the sum is known to fit in `out`.
fn add_at(out: &mut [u64], at: usize, x: &[u64]) {
    // So the limbs of `x` past the end of `out` are zero.
    let (x, excess) = x.split_at(x.len().min(out.len() - at));
    debug_assert!(excess.iter().all(|&l| l == 0));
    let carry = add_assign(&mut out[at..], x);
    debug_assert!(!carry);
}

/// `x /= 3`. Requires `x` to be a multiple of 3.
fn div3(x: &mut [u64]) {
    // Exact division from the low end: each quotient limb is a limb times 3^-1 mod 2^64.
    const INV3: u64 = 0xAAAA_AAAA_AAAA_AAAB;
    let mut borrow = 0;
    for l in x {
        let (d, b) = l.overflowing_sub(borrow);
        *l = d.wrapping_mul(INV3);
        // 3·l = d + hi·2^64, so `hi` more comes off the next limb.
        borrow = l.carrying_mul(3, 0).1 + b as u64;
    }
    debug_assert_eq!(borrow, 0);
}

/// `x >>= 1`.
fn shr1(x: &mut [u64]) {
    let mut hi = 0;
    for l in x.iter_mut().rev() {
        (*l, hi) = ((*l >> 1) | hi, *l << 63);
    }
}

/// `x <<= 1`. Returns whether a set bit was shifted out.
fn shl1(x: &mut [u64]) -> bool {
    let mut lo = 0;
    for l in x {
        (*l, lo) = ((*l << 1) | lo, *l >> 63);
    }
    lo != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{Decode, decode_vec, random_string};
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    #[test]
    fn scratch_fits() {
        for n in MIN_THRESHOLD..1 << 20 {
            let k = n.div_ceil(3);
            assert!(
                6 * k + 6 + mul::scratch_len(k + 1) <= mul::scratch_len(n),
                "toom3, {n}"
            );
            // Blocks are cut for operands of at most `2k` limbs.
            assert!(
                4 * k + mul::scratch_len(2 * k) <= mul::scratch_len(n),
                "blocks, {n}"
            );
        }
    }

    fn check(a: &[u64], b: &[u64], threshold: usize) {
        let want = mul::tests::reference(a, b);
        let mut got = vec![0; a.len() + b.len()];
        // Exactly the documented scratch size, filled with garbage.
        let mut scratch = vec![0x5A5A_5A5A_5A5A_5A5A; mul::scratch_len(a.len().max(b.len()))];
        mul_with(&mut got, a, b, &mut scratch, threshold);
        assert_eq!(
            got,
            want,
            "{} x {}, threshold {threshold}",
            a.len(),
            b.len()
        );
    }

    #[test]
    fn matches_karatsuba() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(1);
        let (m, t) = (MIN_THRESHOLD, THRESHOLD);
        let lens = (0..=2)
            .chain([
                m - 1,
                m,
                m + 1,
                2 * m - 1,
                2 * m,
                2 * m + 1,
                3 * m - 1,
                3 * m,
                3 * m + 1,
            ])
            .chain([t - 1, t, t + 1, 200, 257, 513]);
        for na in lens.clone() {
            for nb in lens.clone() {
                let a: Vec<u64> = (0..na).map(|_| rng.next_u64()).collect();
                let b: Vec<u64> = (0..nb).map(|_| rng.next_u64()).collect();
                // All-ones limbs maximise every carry and borrow chain.
                let (ones_a, ones_b) = (vec![u64::MAX; na], vec![u64::MAX; nb]);
                // The tuned threshold, and Toom-3 wherever it applies.
                for threshold in [t, m] {
                    check(&a, &b, threshold);
                    check(&ones_a, &ones_b, threshold);
                }
            }
        }
    }

    /// Operands where Toom-3 splits off its shortest top parts, and where `r(-1)` takes
    /// either sign.
    #[test]
    fn toom3_shapes() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(4);
        for nb in [32usize, 33, 34, 35, 99, 100, 101, 300] {
            let k = nb.div_ceil(3);
            for na in [2 * k + 1, 2 * k + 2, nb - 1, nb] {
                for pattern in 0..4 {
                    let mut a: Vec<u64> = (0..na).map(|_| rng.next_u64()).collect();
                    let mut b: Vec<u64> = (0..nb).map(|_| rng.next_u64()).collect();
                    // Zero the middle part to make `x0 - x1 + x2` positive, or the
                    // outer parts to make it negative.
                    for (x, neg) in [(&mut a, pattern & 1 != 0), (&mut b, pattern & 2 != 0)] {
                        if neg {
                            x[..k].fill(0);
                            x[2 * k..].fill(0);
                        } else {
                            x[k..2 * k].fill(0);
                        }
                    }
                    check(&a, &b, MIN_THRESHOLD);
                }
            }
        }
    }

    /// Zero evaluation at -1 and high zero limbs stress interpolation cancellation;
    /// sparse operands also force carries across long zero regions and block splits.
    #[test]
    fn zero_and_cancellation_patterns() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(72);
        for n in [32usize, 33, 34, 127, 128, 129, 257, 513] {
            let k = n.div_ceil(3);
            let mut cancel = vec![0; n];
            for i in 0..k {
                cancel[i] = rng.next_u64();
                cancel[k + i] = cancel[i];
            }
            for nb in [n, n + 1, 2 * n + 1] {
                let b = vec![u64::MAX; nb];
                check(&vec![0; n], &b, MIN_THRESHOLD);
                check(&cancel, &b, MIN_THRESHOLD);
                let sparse: Vec<_> = (0..n)
                    .map(|i| if i % 17 == 0 { u64::MAX } else { 0 })
                    .collect();
                check(&sparse, &b, MIN_THRESHOLD);
            }
        }
    }

    #[test]
    fn div3_exact() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(5);
        for len in 0..20 {
            let random: Vec<u64> = (0..len).map(|_| rng.next_u64()).collect();
            for q in [random, vec![u64::MAX; len]] {
                // x = 3·q, one limb longer.
                let mut x = vec![0; len + 1];
                let mut carry = 0;
                for (o, &l) in x.iter_mut().zip(&q) {
                    (*o, carry) = l.carrying_mul(3, carry);
                }
                x[len] = carry;
                div3(&mut x);
                assert_eq!(x[..len], q);
                assert_eq!(x[len], 0);
            }
        }
    }

    #[test]
    fn div3_carry_boundaries() {
        let third = u64::MAX / 3;
        for q in [
            0,
            1,
            4,
            third,
            third + 1,
            2 * third,
            2 * third + 1,
            u64::MAX,
        ] {
            let product = 3 * q as u128;
            let mut x = [product as u64, (product >> 64) as u64];
            div3(&mut x);
            assert_eq!(x, [q, 0]);
        }
    }

    /// Inputs past the sizes `tests.rs` checks against bs58, where Toom-3 recurses, with
    /// the thresholds for CPUs with and without the IFMA leaf.
    #[test]
    fn decode_matches_karatsuba() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(6);
        let decoders: [Decode; 2] = [
            |s, o| decode_toom3_with(s, o, THRESHOLD),
            |s, o| decode_toom3_with(s, o, IFMA_THRESHOLD),
        ];
        for len in [100_001, 300_000] {
            // 58^len - 1 maximises carries.
            for s in [random_string(&mut rng, len), "z".repeat(len)] {
                let want = decode_vec(crate::dc::decode, &s);
                for f in decoders {
                    assert_eq!(decode_vec(f, &s), want, "{len}");
                }
            }
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    #[kani::unwind(3)]
    #[kani::solver(z3)]
    fn div3_recovers_quotient_with_carry() {
        let q: u64 = kani::any();
        let product = 3 * q as u128;
        let mut x = [product as u64, (product >> 64) as u64];
        div3(&mut x);
        assert_eq!(x[0], q);
        assert_eq!(x[1], 0);
    }
}

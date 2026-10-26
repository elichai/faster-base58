//! Arithmetic on little-endian `u64` limb slices.

#[cfg(target_arch = "x86_64")]
use crate::mul_ifma;

/// Operands shorter than this use schoolbook multiplication.
const KARATSUBA_THRESHOLD: usize = 32;

/// With AVX-512 IFMA, a shorter operand of at least this many limbs, and below
/// [`ifma_threshold`], uses the IFMA schoolbook (`mul_ifma.rs`). On Zen 5 the IFMA leaf
/// is 0.74x scalar schoolbook at 8 limbs, 1.09x at 10 and 1.47x at 12.
const IFMA_MIN: usize = 12;
const _: () = assert!(8 <= IFMA_MIN && IFMA_MIN <= KARATSUBA_THRESHOLD);

#[cfg(target_arch = "x86_64")]
pub(crate) mod ifma {
    use crate::mul_ifma::avx512 as mul_ifma;
    use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};

    /// Tuned on whole decodes on Zen 5.
    static IFMA_THRESHOLD: AtomicUsize = AtomicUsize::new(384);

    /// The IFMA leaf threshold `mul` uses: 0 (none) without AVX-512 IFMA.
    pub fn ifma_threshold() -> usize {
        if mul_ifma::available() {
            return IFMA_THRESHOLD.load(Relaxed);
        }
        0
    }

    /// Sets the IFMA leaf threshold of [`super::mul`], for benchmarks; 0 turns the leaf off.
    #[cfg(any(test, feature = "__internals"))]
    pub fn set_ifma_threshold(t: usize) {
        IFMA_THRESHOLD.store(t, Relaxed);
    }
}

#[cfg(not(target_arch = "x86_64"))]
mod ifma {
    /// The IFMA leaf threshold: this target has no native IFMA leaf.
    pub fn ifma_threshold() -> usize {
        0
    }
}

pub use ifma::ifma_threshold;

/// Scratch limbs `mul` needs when neither operand is longer than `n` limbs.
///
/// By induction: a Karatsuba step on `n` limbs uses at most `2n + 6` limbs, then
/// recurses on at most `n/2 + 3/2` limbs, which fits when `n >= 27`. Cutting blocks of
/// `a <= n/2` limbs uses `2a`, then recurses on `a` limbs: `7a + 64 <= 5n + 64`. The IFMA
/// leaf uses less than `3.9n + 62` (`mul_ifma::scratch_len`).
pub const fn scratch_len(n: usize) -> usize {
    5 * n + 64
}
const _: () = assert!(KARATSUBA_THRESHOLD >= 27);

/// `out = a * b`. Requires `out.len() == a.len() + b.len()` and
/// `scratch.len() >= scratch_len(max(a.len(), b.len()))`. `scratch` may hold anything.
pub fn mul(out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]) {
    mul_with(out, a, b, scratch, ifma_threshold());
}

/// Multiplication with the IFMA leaf for a shorter operand of `IFMA_MIN..t` limbs (at most
/// `mul_ifma::MAX_LEN`); `t = 0` never uses it. Panics if that range is used on an x86-64
/// CPU without AVX-512 IFMA; other targets ignore `t`.
pub fn mul_with(out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64], t: usize) {
    debug_assert_eq!(out.len(), a.len() + b.len());
    let (a, b) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    #[cfg(target_arch = "x86_64")]
    if (IFMA_MIN..t.min(mul_ifma::MAX_LEN + 1)).contains(&a.len()) {
        return mul_ifma::avx512::mul(out, a, b, scratch);
    }
    if a.len() < KARATSUBA_THRESHOLD {
        schoolbook(out, a, b);
    } else if 2 * a.len() <= b.len() {
        mul_blocks(out, a, b, scratch, |o, x, y, s| mul_with(o, x, y, s, t));
    } else {
        karatsuba(out, a, b, scratch, t);
    }
}

/// `out = a * b`, cutting `b` into `a`-sized blocks so that each product `f` makes is
/// balanced. Requires `0 < a.len() <= b.len()`. Uses `2 * a.len()` limbs of `scratch`
/// and passes the rest to `f`.
pub fn mul_blocks(
    out: &mut [u64],
    a: &[u64],
    b: &[u64],
    scratch: &mut [u64],
    mut f: impl FnMut(&mut [u64], &[u64], &[u64], &mut [u64]),
) {
    out.fill(0);
    let (tmp, scratch) = scratch.split_at_mut(2 * a.len());
    for (i, blk) in b.chunks(a.len()).enumerate() {
        let tmp = &mut tmp[..a.len() + blk.len()];
        f(tmp, a, blk, scratch);
        let carry = add_assign(&mut out[i * a.len()..], tmp);
        debug_assert!(!carry);
    }
}

/// Two rows per pass: the second row runs one column behind the first, so the CPU
/// overlaps their carry chains.
fn schoolbook(out: &mut [u64], a: &[u64], b: &[u64]) {
    out.fill(0);
    let n = b.len();
    let Some(&b0) = b.first() else { return };
    let (pairs, last) = a.as_chunks::<2>();
    for (r, &[x0, x1]) in pairs.iter().enumerate() {
        // Earlier rows end below `o[n]`, so its last two limbs are still zero.
        let o = &mut out[2 * r..2 * r + n + 2];
        let (mut c0, mut c1) = (0, 0);
        (o[0], c0) = x0.carrying_mul_add(b0, o[0], c0);
        for j in 1..n {
            let (v, h0) = x0.carrying_mul_add(b[j], o[j], c0);
            (o[j], c1) = x1.carrying_mul_add(b[j - 1], v, c1);
            c0 = h0;
        }
        (o[n], o[n + 1]) = x1.carrying_mul_add(b[n - 1], c0, c1);
    }
    if let &[x] = last {
        let i = a.len() - 1;
        let mut carry = 0;
        for (o, &y) in out[i..].iter_mut().zip(b) {
            (*o, carry) = x.carrying_mul_add(y, *o, carry);
        }
        out[i + n] = carry;
    }
}

/// Requires `a.len() <= b.len() < 2 * a.len()`; `t` as in [`mul_with`].
fn karatsuba(out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64], t: usize) {
    // a = a1·X^h + a0, b = b1·X^h + b0. `h < a.len()`, so `a1` is non-empty.
    let h = b.len() / 2;
    let (a0, a1) = a.split_at(h);
    let (b0, b1) = b.split_at(h);

    // z1 = (a0 + a1)(b0 + b1) - z0 - z2
    let (na, nb) = (a1.len().max(h) + 1, b1.len() + 1);
    let (sa, scratch) = scratch.split_at_mut(na);
    let (sb, scratch) = scratch.split_at_mut(nb);
    let (z1, scratch) = scratch.split_at_mut(na + nb);
    sum(sa, a0, a1);
    sum(sb, b0, b1);
    mul_with(z1, sa, sb, scratch, t);

    let (z0, z2) = out.split_at_mut(2 * h);
    mul_with(z0, a0, b0, scratch, t);
    mul_with(z2, a1, b1, scratch, t);
    let borrow = sub_assign(z1, z0) | sub_assign(z1, z2);
    debug_assert!(!borrow);

    // z1·X^h <= a·b, so z1 fits in `out[h..]` and its excess limbs are zero.
    let (z1, excess) = z1.split_at(z1.len().min(out.len() - h));
    debug_assert!(excess.iter().all(|&x| x == 0));
    let carry = add_assign(&mut out[h..], z1);
    debug_assert!(!carry);
}

/// `out = x + y`. Requires `out` to be longer than `x` and `y`.
fn sum(out: &mut [u64], x: &[u64], y: &[u64]) {
    let (lo, hi) = out.split_at_mut(x.len());
    lo.copy_from_slice(x);
    hi.fill(0);
    let carry = add_assign(out, y);
    debug_assert!(!carry);
}

/// `a += b`, carrying through the rest of `a`. Returns the carry out of `a`.
pub fn add_assign(a: &mut [u64], b: &[u64]) -> bool {
    let (lo, hi) = a.split_at_mut(b.len());
    let mut carry = false;
    for (x, &y) in lo.iter_mut().zip(b) {
        (*x, carry) = x.carrying_add(y, carry);
    }
    for x in hi {
        if !carry {
            break;
        }
        (*x, carry) = x.overflowing_add(1);
    }
    carry
}

/// `out = x mod (β^L - 1)` for `L = out.len() > 0`, as some value in `[0, β^L - 1]`.
pub fn fold_wrapped(out: &mut [u64], x: &[u64]) {
    out.fill(0);
    for c in x.chunks(out.len()) {
        // `β^L ≡ 1`. Both terms are below `β^L`, so after the wrap the sum is at most
        // `β^L - 2` and adding one does not carry again.
        if add_assign(out, c) {
            let carry = add_assign(out, &[1]);
            debug_assert!(!carry);
        }
    }
}

/// `a -= b`, borrowing through the rest of `a`. Returns the borrow out of `a`.
pub fn sub_assign(a: &mut [u64], b: &[u64]) -> bool {
    let (lo, hi) = a.split_at_mut(b.len());
    let mut borrow = false;
    for (x, &y) in lo.iter_mut().zip(b) {
        (*x, borrow) = x.borrowing_sub(y, borrow);
    }
    for x in hi {
        if !borrow {
            break;
        }
        (*x, borrow) = x.overflowing_sub(1);
    }
    borrow
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    /// One row per pass, kept simple as the reference.
    pub(crate) fn reference(a: &[u64], b: &[u64]) -> Vec<u64> {
        let mut out = vec![0; a.len() + b.len()];
        for (i, &x) in a.iter().enumerate() {
            let mut carry = 0;
            for (o, &y) in out[i..].iter_mut().zip(b) {
                (*o, carry) = x.carrying_mul_add(y, *o, carry);
            }
            out[i + b.len()] = carry;
        }
        out
    }

    fn check(a: &[u64], b: &[u64]) {
        let want = reference(a, b);
        // Exactly the documented scratch size, filled with garbage.
        let mut scratch = vec![0x5A5A_5A5A_5A5A_5A5A; scratch_len(a.len().max(b.len()))];
        let mut got = vec![0x5A5A_5A5A_5A5A_5A5A; a.len() + b.len()];
        mul(&mut got, a, b, &mut scratch);
        assert_eq!(got, want, "{} x {}", a.len(), b.len());
        for t in thresholds() {
            got.fill(0x5A5A_5A5A_5A5A_5A5A);
            scratch.fill(0x5A5A_5A5A_5A5A_5A5A);
            mul_with(&mut got, a, b, &mut scratch, t);
            assert_eq!(got, want, "IFMA threshold {t}, {} x {}", a.len(), b.len());
        }
        got.fill(0x5A5A_5A5A_5A5A_5A5A);
        schoolbook(&mut got, a, b);
        assert_eq!(got, want, "schoolbook, {} x {}", a.len(), b.len());
    }

    /// IFMA leaf thresholds for [`mul_with`]: none, and on CPUs with AVX-512 IFMA a small,
    /// the default and one above every test length.
    fn thresholds() -> Vec<usize> {
        let t = ifma_threshold();
        if t == 0 {
            vec![0]
        } else {
            vec![0, IFMA_MIN + 1, t, usize::MAX]
        }
    }

    #[test]
    fn mul_matches_schoolbook() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(1);
        let k = KARATSUBA_THRESHOLD;
        let ifma = [
            7,
            8,
            9,
            IFMA_MIN - 1,
            IFMA_MIN,
            IFMA_MIN + 1,
            71,
            72,
            73,
            100,
        ];
        let lens = (0..=4)
            .chain(k - 4..=k + 4)
            .chain([2 * k - 1, 2 * k, 2 * k + 1, 200, 257, 513])
            .chain(ifma);
        for na in lens.clone() {
            for nb in lens.clone() {
                let a: Vec<u64> = (0..na).map(|_| rng.next_u64()).collect();
                let b: Vec<u64> = (0..nb).map(|_| rng.next_u64()).collect();
                check(&a, &b);
                // All-ones limbs maximise every carry and borrow chain.
                check(&vec![u64::MAX; na], &vec![u64::MAX; nb]);
            }
        }
    }
}

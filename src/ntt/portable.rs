//! The transform kernels in plain Rust, one lane per element, so LLVM vectorizes the
//! stage loops (4-wide `smull` on aarch64). See [`super::Kernels`] for the contracts.
//!
//! Values passed between kernels stay in `(-q, q)`: a sum or difference of two such
//! values is below `2^31`, which [`Prime::reduce`] and [`Prime::mont`] accept. Inside
//! `inverse`, values between stages can reach `1.25 q + 32` (see [`lazy`]).

use super::tables::Prime;
#[cfg(test)]
use super::tables::{PRIMES, Twiddles};

#[cfg(any(
    test,
    feature = "__internals",
    kani,
    not(all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ))
))]
mod kernels;
#[cfg(any(
    test,
    feature = "__internals",
    kani,
    not(all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ))
))]
pub(crate) use kernels::{
    fold_sub, forward, garner, halves, inverse, load, pair, pointwise, square, twist,
};

/// A 32-bit coefficient mod `q`, in `(-q, q)`.
#[inline(always)]
pub(crate) fn load_one(c: u32, p: Prime) -> i32 {
    p.reduce(lift(c, p))
}

/// `c mod q` as an `i32` of magnitude below `2^31`.
#[inline(always)]
fn lift(c: u32, p: Prime) -> i32 {
    // As `i32`, `c >= 2^31` reads `c - 2^32`. Adding `2^32 mod q` restores `c mod q` and
    // stays above `-2^31`.
    let x = c as i32;
    x + ((x >> 31) & p.r32)
}

/// `load`, with coefficient `i` multiplied by the root `w[i]` stands for (see
/// `twist`). Requires `w.len() >= 2 * limbs.len()`.
#[inline(always)]
pub(crate) fn load_twist(dst: &mut [i32], limbs: &[u64], p: Prime, w: &[i32]) {
    let (head, rest) = dst.split_at_mut(2 * limbs.len());
    let w = &w[..head.len()];
    let pairs = head.as_chunks_mut::<2>().0.iter_mut().zip(limbs);
    for ((d, &l), w) in pairs.zip(w.as_chunks::<2>().0) {
        // `|lift| < 2^31` and `|w| <= q / 2`: `mont` gives below `0.75 q`.
        *d = [
            p.mont(lift(l as u32, p), w[0], p.wq(w[0])),
            p.mont(lift((l >> 32) as u32, p), w[1], p.wq(w[1])),
        ];
    }
    rest.fill(0);
}

/// Whether stage `h` of an `n`-point `inverse` skips its reductions: `log2(h)` odd, and
/// not the last stage.
///
/// A reducing stage stores magnitudes of at most `q / 2 + 32`. A lazy stage follows one
/// and adds or subtracts `mont(y, w)`, below `0.75 q`, so it stores less than
/// `1.25 q + 32`. The next stage reduces: `mont` takes its `y` (below `2^31`), and its
/// sums stay below `2 q + 32 < 2^31`. The `h = 1` stage and the last stage reduce, so
/// the input may be anywhere in `(-q, q)` and the output is in it again.
#[inline(always)]
pub(crate) fn lazy(h: usize, n: usize) -> bool {
    h.trailing_zeros() % 2 == 1 && 2 * h < n
}

/// Adds (or, with `sub`, subtracts) the 32-bit halves of `limbs` to `dst`, element by
/// element, reduced. Requires `dst.len() >= 2 * limbs.len()`; the rest of `dst` is kept.
#[inline(always)]
pub(crate) fn load_add(dst: &mut [i32], limbs: &[u64], p: Prime, sub: bool) {
    assert!(dst.len() >= 2 * limbs.len());
    let s = if sub { -1 } else { 1 };
    for (d, &l) in dst.as_chunks_mut::<2>().0.iter_mut().zip(limbs) {
        // `|d| < q` and a loaded value is at most `q / 2 + 32`: the sum is below 2^31.
        d[0] = p.reduce(d[0] + s * load_one(l as u32, p));
        d[1] = p.reduce(d[1] + s * load_one((l >> 32) as u32, p));
    }
}

/// `1 / 2` in Montgomery form, `2^31 mod q`, balanced, and its companion.
pub(crate) fn half(p: Prime) -> (i32, i32) {
    let h = p.to_mont((p.q as u32).div_ceil(2));
    (h, p.wq(h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntt::tables::Tables;
    use crate::ntt::tables::tests::{modq, pow, samples};
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    /// The root of unity of order `n` that [`Tables`] uses, as a plain residue.
    fn root(p: Prime, t: &Twiddles, n: usize) -> u64 {
        let q = p.q as u64;
        if n == 2 {
            return q - 1;
        }
        // fw[n/2 + 1] is w_n in Montgomery form; strip the 2^32.
        modq(t.fw[n / 2 + 1] as i128, p.q) * pow(1 << 32, q - 2, q) % q
    }

    fn bitrev(i: usize, log: u32) -> usize {
        i.reverse_bits() >> (usize::BITS - log)
    }

    #[test]
    fn forward_is_a_dft() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(20);
        let tables = Tables::new(7);
        for (p, tw) in PRIMES.into_iter().zip(&tables.tw) {
            for log in 1..=7 {
                let n = 1 << log;
                let x: Vec<i32> = samples(&mut rng, p.q, n)[..n].to_vec();
                let mut a = x.clone();
                forward(&mut a, p, tw);
                let w = root(p, tw, n);
                let q = p.q as u64;
                for (j, &got) in a.iter().enumerate() {
                    let wk = pow(w, bitrev(j, log) as u64, q);
                    let want = x
                        .iter()
                        .rev()
                        .fold(0, |acc, &v| (acc * wk + modq(v as i128, p.q)) % q);
                    assert_eq!(modq(got as i128, p.q), want, "n {n}, j {j}");
                    assert!(got.abs() < p.q);
                }
            }
        }
    }

    #[test]
    fn inverse_undoes_forward() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(21);
        let tables = Tables::new(12);
        for (p, tw) in PRIMES.into_iter().zip(&tables.tw) {
            for log in 1..=12 {
                let n = 1 << log;
                let x = samples(&mut rng, p.q, n)[..n].to_vec();
                let mut a = x.clone();
                forward(&mut a, p, tw);
                inverse(&mut a, p, tw);
                for (&got, &x) in a.iter().zip(&x) {
                    assert_eq!(
                        modq(got as i128, p.q),
                        modq(x as i128 * n as i128, p.q),
                        "n {n}"
                    );
                    assert!(got.abs() < p.q);
                }
            }
        }
    }

    #[test]
    fn pointwise_scales() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(22);
        for p in PRIMES {
            for n in [8, 64, 1 << 10] {
                let x = samples(&mut rng, p.q, n)[..n].to_vec();
                let y = samples(&mut rng, p.q, n)[..n].to_vec();
                let mut a = x.clone();
                pointwise(&mut a, &y, p);
                let q = p.q as u64;
                let ninv = pow(n as u64, q - 2, q) as i128;
                for ((&got, &x), &y) in a.iter().zip(&x).zip(&y) {
                    assert_eq!(
                        modq(got as i128, p.q),
                        modq(x as i128 * y as i128 * ninv, p.q)
                    );
                    assert!(got.abs() < p.q);
                }
                let (mut a, mut b) = (x.clone(), x.clone());
                square(&mut a, p);
                pointwise(&mut b, &x, p);
                assert_eq!(a, b, "square, n {n}");
            }
        }
    }

    #[test]
    fn load_reduces() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(23);
        let mut limbs = vec![0, u64::MAX, 1 << 63, (1 << 31) | (1 << 63), 0x8000_0000];
        limbs.extend((0..100).map(|_| rng.next_u64()));
        for p in PRIMES {
            let mut dst = vec![0x5A5A_5A5A; 2 * limbs.len() + 7];
            load(&mut dst, &limbs, p);
            for (i, &got) in dst.iter().enumerate() {
                let c = limbs
                    .get(i / 2)
                    .map_or(0, |&l| (l >> (32 * (i % 2))) as u32);
                assert_eq!(modq(got as i128, p.q), c as u64 % p.q as u64, "{i}");
                assert!(got.abs() < p.q);
            }
        }
    }

    #[test]
    fn load_twist_multiplies() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(25);
        let tables = Tables::new(7);
        let mut limbs = vec![0, u64::MAX, 1 << 63, (1 << 31) | (1 << 63), 0x8000_0000];
        limbs.extend((0..27).map(|_| rng.next_u64()));
        for (p, tw) in PRIMES.into_iter().zip(&tables.tw) {
            let (w, q) = (root(p, tw, 128), p.q as u64);
            let mut dst = vec![0x5A5A_5A5A; 2 * limbs.len() + 7];
            load_twist(&mut dst, &limbs, p, &tw.fw[64..]);
            for (i, &got) in dst.iter().enumerate() {
                let c = limbs
                    .get(i / 2)
                    .map_or(0, |&l| (l >> (32 * (i % 2))) as u32);
                let want = c as u64 % q * pow(w, i as u64, q) % q;
                assert_eq!(modq(got as i128, p.q), want, "{i}");
                assert!(got.abs() < p.q);
            }
        }
    }
}

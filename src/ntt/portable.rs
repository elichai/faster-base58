//! The transform kernels in plain Rust, one lane per element, so LLVM vectorizes the
//! stage loops (4-wide `smull` on aarch64). See [`super::Kernels`] for the contracts.
//!
//! Values passed between kernels stay in `(-q, q)`: a sum or difference of two such
//! values is below `2^31`, which [`Prime::reduce`] and [`Prime::mont`] accept. Inside
//! [`inverse`], values between stages can reach `1.25 q + 32` (see [`lazy`]).

use super::crt::GARNER;
use super::tables::{PRIMES, Prime, Twiddles};

#[inline(always)]
pub(crate) fn load(dst: &mut [i32], limbs: &[u64], p: Prime) {
    let (head, rest) = dst.split_at_mut(2 * limbs.len());
    for (d, &l) in head.as_chunks_mut::<2>().0.iter_mut().zip(limbs) {
        *d = [load_one(l as u32, p), load_one((l >> 32) as u32, p)];
    }
    rest.fill(0);
}

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

/// [`load`], with coefficient `i` multiplied by the root `w[i]` stands for (see
/// [`twist`]). Requires `w.len() >= 2 * limbs.len()`.
#[inline(always)]
pub(crate) fn load_twist(dst: &mut [i32], limbs: &[u64], p: Prime, w: &[i32], wq: &[i32]) {
    let (head, rest) = dst.split_at_mut(2 * limbs.len());
    let (w, wq) = (&w[..head.len()], &wq[..head.len()]);
    let pairs = head.as_chunks_mut::<2>().0.iter_mut().zip(limbs);
    for ((d, &l), (w, wq)) in pairs.zip(w.as_chunks::<2>().0.iter().zip(wq.as_chunks::<2>().0)) {
        // `|lift| < 2^31` and `|w| <= q / 2`: `mont` gives below `0.75 q`.
        *d = [p.mont(lift(l as u32, p), w[0], wq[0]), p.mont(lift((l >> 32) as u32, p), w[1], wq[1])];
    }
    rest.fill(0);
}

/// Decimation in frequency: natural order in, bit-reversed order out.
#[inline(always)]
pub(crate) fn forward(a: &mut [i32], p: Prime, t: &Twiddles) {
    let n = a.len();
    debug_assert!(n.is_power_of_two() && n >= 2 && n <= t.fw.len());
    let mut h = n / 2;
    while h >= 2 {
        let (w, wq) = (&t.fw[h..2 * h], &t.fwq[h..2 * h]);
        for blk in a.chunks_exact_mut(2 * h) {
            let (x, y) = blk.split_at_mut(h);
            for (((x, y), &w), &wq) in x.iter_mut().zip(y).zip(w).zip(wq) {
                let (s, d) = (*x + *y, *x - *y);
                *x = p.reduce(s);
                *y = p.mont(d, w, wq);
            }
        }
        h /= 2;
    }
    last_stage(a, p);
}

/// Decimation in time: bit-reversed order in, natural order out, times `a.len()`.
#[inline(always)]
pub(crate) fn inverse(a: &mut [i32], p: Prime, t: &Twiddles) {
    let n = a.len();
    debug_assert!(n.is_power_of_two() && n >= 2 && n <= t.iw.len());
    last_stage(a, p);
    let mut h = 2;
    while h < n {
        if lazy(h, n) {
            inverse_stage::<true>(a, h, p, t);
        } else {
            inverse_stage::<false>(a, h, p, t);
        }
        h *= 2;
    }
}

/// Whether stage `h` of an `n`-point [`inverse`] skips its reductions: `log2(h)` odd, and
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

/// One stage of [`inverse`], on blocks of `2 h`; `LAZY` skips the reductions.
#[inline(always)]
fn inverse_stage<const LAZY: bool>(a: &mut [i32], h: usize, p: Prime, t: &Twiddles) {
    let (w, wq) = (&t.iw[h..2 * h], &t.iwq[h..2 * h]);
    for blk in a.chunks_exact_mut(2 * h) {
        let (x, y) = blk.split_at_mut(h);
        for (((x, y), &w), &wq) in x.iter_mut().zip(y).zip(w).zip(wq) {
            let t = p.mont(*y, w, wq);
            let (s, d) = (*x + t, *x - t);
            (*x, *y) = if LAZY { (s, d) } else { (p.reduce(s), p.reduce(d)) };
        }
    }
}

/// The stage with `h = 1`, whose twiddle is 1; it is the same in both directions.
#[inline(always)]
fn last_stage(a: &mut [i32], p: Prime) {
    for [x, y] in a.as_chunks_mut::<2>().0 {
        (*x, *y) = (p.reduce(*x + *y), p.reduce(*x - *y));
    }
}

#[inline(always)]
pub(crate) fn pointwise(a: &mut [i32], b: &[i32], p: Prime) {
    debug_assert_eq!(a.len(), b.len());
    let (k, kq) = p.scale(a.len());
    for (x, &y) in a.iter_mut().zip(b) {
        *x = p.mont(p.mont(*x, y, p.wq(y)), k, kq);
    }
}

#[inline(always)]
pub(crate) fn square(a: &mut [i32], p: Prime) {
    let (k, kq) = p.scale(a.len());
    for x in a {
        *x = p.mont(p.mont(*x, *x, p.wq(*x)), k, kq);
    }
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

/// `a[i] = a[i] w[i] / 2^32 mod q`: `a[i]` times the root that Montgomery-form `w[i]`
/// stands for. `|w| <= q / 2` (as in [`Twiddles`]), so the result is below `0.75 q`.
#[inline(always)]
pub(crate) fn twist(a: &mut [i32], w: &[i32], wq: &[i32], p: Prime) {
    let (w, wq) = (&w[..a.len()], &wq[..a.len()]);
    for ((x, &w), &wq) in a.iter_mut().zip(w).zip(wq) {
        *x = p.mont(*x, w, wq);
    }
}

/// Subtracts `x^start src` from `dst`, modulo `x^d + 1` (`neg`) or `x^d - 1`, for
/// `d = dst.len()`: `src[t]` goes to `(start + t) mod d`, negated when `neg` and
/// `(start + t) / d` is odd.
#[inline(always)]
pub(crate) fn fold_sub(dst: &mut [i32], src: &[i32], start: usize, neg: bool, p: Prime) {
    let d = dst.len();
    let (mut pos, mut src) = (start, src);
    while !src.is_empty() {
        let at = pos % d;
        let (run, rest) = src.split_at((d - at).min(src.len()));
        let s = if neg && (pos / d) % 2 == 1 { -1 } else { 1 };
        for (x, &y) in dst[at..].iter_mut().zip(run) {
            // Both are below `q` in magnitude.
            *x = p.reduce(*x - s * y);
        }
        (pos, src) = (pos + run.len(), rest);
    }
}

/// `(lo[i], hi[i]) = ((hi[i] + lo[i]) / 2, (hi[i] - lo[i]) / 2)`.
#[inline(always)]
pub(crate) fn halves(lo: &mut [i32], hi: &mut [i32], p: Prime) {
    debug_assert_eq!(lo.len(), hi.len());
    let (h, hq) = half(p);
    for (lo, hi) in lo.iter_mut().zip(hi) {
        // Both are below `q` in magnitude, so the sum and difference are below 2^31.
        (*lo, *hi) = (p.mont(*hi + *lo, h, hq), p.mont(*hi - *lo, h, hq));
    }
}

/// [`super::crt()`] for `r = [c mod (x^m + 1) | c mod (x^n - 1)]`, `n = r.len() - m`,
/// with the first still multiplied by `w^i` at `i` (`iw[i]`, `iwq[i]` undo that), fused
/// with undoing it. Requires `n < m`; then `n` divides `m`.
///
/// With `c = sum_k C_k x^(kn)` in blocks of `n`, `k <= m / n`, the first residue is
/// `C_0 - C_(m/n)` in block 0 and `C_k` in block `k`, and the second is `sum_k C_k`. So
/// `C_0 + C_(m/n)` is the second minus blocks `1..m/n` of the first, and only blocks 0
/// and `m/n` change.
#[inline(always)]
pub(crate) fn pair(r: &mut [i32], m: usize, iw: &[i32], iwq: &[i32], p: Prime) {
    let n = r.len() - m;
    assert!(n < m && m.is_multiple_of(n) && iw.len() >= m && iwq.len() >= m);
    let (e, s) = r.split_at_mut(m);
    let (e0, mid) = e.split_at_mut(n);
    let ws = iw[n..m].chunks_exact(n).zip(iwq[n..m].chunks_exact(n));
    for (blk, (w, wq)) in mid.chunks_exact_mut(n).zip(ws) {
        for (((x, s), &w), &wq) in blk.iter_mut().zip(s.iter_mut()).zip(w).zip(wq) {
            *x = p.mont(*x, w, wq);
            // `|s| < q` and `|x| < 0.75 q`.
            *s = p.reduce(*s - *x);
        }
    }
    let (h, hq) = half(p);
    for (((lo, hi), &w), &wq) in e0.iter_mut().zip(s).zip(&iw[..n]).zip(&iwq[..n]) {
        let x = p.mont(*lo, w, wq);
        (*lo, *hi) = (p.mont(*hi + x, h, hq), p.mont(*hi - x, h, hq));
    }
}

/// `1 / 2` in Montgomery form, `2^31 mod q`, balanced, and its companion.
pub(crate) fn half(p: Prime) -> (i32, i32) {
    let h = p.to_mont((p.q as u32).div_ceil(2));
    (h, p.wq(h))
}

#[inline(always)]
pub(crate) fn garner([r1, r2, r3]: [&mut [i32]; 3]) {
    let ([p1, p2, p3], g) = (PRIMES, GARNER);
    debug_assert!(r1.len() == r2.len() && r2.len() == r3.len());
    for ((x1, x2), x3) in r1.iter_mut().zip(r2.iter_mut()).zip(r3.iter_mut()) {
        let (a, b, c) = (p1.normalize(*x1), p2.normalize(*x2), p3.normalize(*x3));
        // `b - a` and `c - a` are below 2^30 in magnitude; `t - v2` below 1.75 * 2^30.
        let v2 = p2.normalize(p2.mont(b - a, g.c12, g.c12q));
        let t = p3.mont(c - a, g.c13, g.c13q);
        let v3 = p3.normalize(p3.mont(t - v2, g.c23, g.c23q));
        (*x1, *x2, *x3) = (a, v2, v3);
    }
}

/// These kernels compiled for AVX-512 or AVX2, when this CPU has either.
#[cfg(target_arch = "x86_64")]
pub(crate) fn x86() -> Option<super::Kernels> {
    macro_rules! kernels {
        ($feature:literal) => {{
            #[target_feature(enable = $feature)]
            fn tf_load(dst: &mut [i32], limbs: &[u64], p: Prime) {
                load(dst, limbs, p)
            }
            #[target_feature(enable = $feature)]
            fn tf_forward(a: &mut [i32], p: Prime, t: &Twiddles) {
                forward(a, p, t)
            }
            #[target_feature(enable = $feature)]
            fn tf_inverse(a: &mut [i32], p: Prime, t: &Twiddles) {
                inverse(a, p, t)
            }
            #[target_feature(enable = $feature)]
            fn tf_pointwise(a: &mut [i32], b: &[i32], p: Prime) {
                pointwise(a, b, p)
            }
            #[target_feature(enable = $feature)]
            fn tf_square(a: &mut [i32], p: Prime) {
                square(a, p)
            }
            #[target_feature(enable = $feature)]
            fn tf_garner(r: [&mut [i32]; 3]) {
                garner(r)
            }
            #[target_feature(enable = $feature)]
            fn tf_load_twist(dst: &mut [i32], limbs: &[u64], p: Prime, w: &[i32], wq: &[i32]) {
                load_twist(dst, limbs, p, w, wq)
            }
            #[target_feature(enable = $feature)]
            fn tf_load_add(dst: &mut [i32], limbs: &[u64], p: Prime, sub: bool) {
                load_add(dst, limbs, p, sub)
            }
            #[target_feature(enable = $feature)]
            fn tf_twist(a: &mut [i32], w: &[i32], wq: &[i32], p: Prime) {
                twist(a, w, wq, p)
            }
            #[target_feature(enable = $feature)]
            fn tf_fold_sub(dst: &mut [i32], src: &[i32], start: usize, neg: bool, p: Prime) {
                fold_sub(dst, src, start, neg, p)
            }
            #[target_feature(enable = $feature)]
            fn tf_halves(lo: &mut [i32], hi: &mut [i32], p: Prime) {
                halves(lo, hi, p)
            }
            #[target_feature(enable = $feature)]
            fn tf_pair(r: &mut [i32], m: usize, iw: &[i32], iwq: &[i32], p: Prime) {
                pair(r, m, iw, iwq, p)
            }
            // SAFETY (each call): built only after detecting `$feature`.
            super::Kernels {
                load: |d, l, p| unsafe { tf_load(d, l, p) },
                forward: |a, p, t| unsafe { tf_forward(a, p, t) },
                inverse: |a, p, t| unsafe { tf_inverse(a, p, t) },
                pointwise: |a, b, p| unsafe { tf_pointwise(a, b, p) },
                square: |a, p| unsafe { tf_square(a, p) },
                garner: |r| unsafe { tf_garner(r) },
                load_twist: |d, l, p, w, wq| unsafe { tf_load_twist(d, l, p, w, wq) },
                load_add: |d, l, p, sub| unsafe { tf_load_add(d, l, p, sub) },
                twist: |a, w, wq, p| unsafe { tf_twist(a, w, wq, p) },
                fold_sub: |d, s, at, neg, p| unsafe { tf_fold_sub(d, s, at, neg, p) },
                halves: |lo, hi, p| unsafe { tf_halves(lo, hi, p) },
                pair: |r, m, w, wq, p| unsafe { tf_pair(r, m, w, wq, p) },
            }
        }};
    }
    if is_x86_feature_detected!("avx512f") {
        Some(kernels!("avx512f"))
    } else if is_x86_feature_detected!("avx2") {
        Some(kernels!("avx2"))
    } else {
        None
    }
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
                    let want =
                        x.iter().rev().fold(0, |acc, &v| (acc * wk + modq(v as i128, p.q)) % q);
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
                    assert_eq!(modq(got as i128, p.q), modq(x as i128 * n as i128, p.q), "n {n}");
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
                    assert_eq!(modq(got as i128, p.q), modq(x as i128 * y as i128 * ninv, p.q));
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
                let c = limbs.get(i / 2).map_or(0, |&l| (l >> (32 * (i % 2))) as u32);
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
            load_twist(&mut dst, &limbs, p, &tw.fw[64..], &tw.fwq[64..]);
            for (i, &got) in dst.iter().enumerate() {
                let c = limbs.get(i / 2).map_or(0, |&l| (l >> (32 * (i % 2))) as u32);
                let want = c as u64 % q * pow(w, i as u64, q) % q;
                assert_eq!(modq(got as i128, p.q), want, "{i}");
                assert!(got.abs() < p.q);
            }
        }
    }
}

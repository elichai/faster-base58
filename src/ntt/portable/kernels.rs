//! Full plain-Rust kernels, also used as comparisons and by x86 wrappers.

use super::super::crt::GARNER;
use super::super::tables::{PRIMES, Prime, Twiddles};
use super::super::{Kernels, portable};
use super::{half, lazy, load_one};

#[inline(always)]
pub(crate) fn load(dst: &mut [i32], limbs: &[u64], p: Prime) {
    let (head, rest) = dst.split_at_mut(2 * limbs.len());
    for (d, &l) in head.as_chunks_mut::<2>().0.iter_mut().zip(limbs) {
        *d = [load_one(l as u32, p), load_one((l >> 32) as u32, p)];
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
        let w = &t.fw[h..2 * h];
        for blk in a.chunks_exact_mut(2 * h) {
            let (x, y) = blk.split_at_mut(h);
            for ((x, y), &w) in x.iter_mut().zip(y).zip(w) {
                let (s, d) = (*x + *y, *x - *y);
                *x = p.reduce(s);
                *y = p.mont(d, w, p.wq(w));
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

/// One stage of [`inverse`], on blocks of `2 h`; `LAZY` skips the reductions.
#[inline(always)]
fn inverse_stage<const LAZY: bool>(a: &mut [i32], h: usize, p: Prime, t: &Twiddles) {
    let w = &t.iw[h..2 * h];
    for blk in a.chunks_exact_mut(2 * h) {
        let (x, y) = blk.split_at_mut(h);
        for ((x, y), &w) in x.iter_mut().zip(y).zip(w) {
            let t = p.mont(*y, w, p.wq(w));
            let (s, d) = (*x + t, *x - t);
            (*x, *y) = if LAZY {
                (s, d)
            } else {
                (p.reduce(s), p.reduce(d))
            };
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

/// `a[i] = a[i] w[i] / 2^32 mod q`: `a[i]` times the root that Montgomery-form `w[i]`
/// stands for. `|w| <= q / 2` (as in [`Twiddles`]), so the result is below `0.75 q`.
#[inline(always)]
pub(crate) fn twist(a: &mut [i32], w: &[i32], p: Prime) {
    let w = &w[..a.len()];
    for (x, &w) in a.iter_mut().zip(w) {
        *x = p.mont(*x, w, p.wq(w));
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

/// [`super::super::crt()`] for `r = [c mod (x^m + 1) | c mod (x^n - 1)]`, `n = r.len() - m`,
/// with the first still multiplied by `w^i` at `i` (`iw[i]` undoes that), fused
/// with undoing it. Requires `n < m`; then `n` divides `m`.
///
/// With `c = sum_k C_k x^(kn)` in blocks of `n`, `k <= m / n`, the first residue is
/// `C_0 - C_(m/n)` in block 0 and `C_k` in block `k`, and the second is `sum_k C_k`. So
/// `C_0 + C_(m/n)` is the second minus blocks `1..m/n` of the first, and only blocks 0
/// and `m/n` change.
#[inline(always)]
pub(crate) fn pair(r: &mut [i32], m: usize, iw: &[i32], p: Prime) {
    let n = r.len() - m;
    assert!(n < m && m.is_multiple_of(n) && iw.len() >= m);
    let (e, s) = r.split_at_mut(m);
    let (e0, mid) = e.split_at_mut(n);
    for (blk, w) in mid.chunks_exact_mut(n).zip(iw[n..m].chunks_exact(n)) {
        for ((x, s), &w) in blk.iter_mut().zip(s.iter_mut()).zip(w) {
            *x = p.mont(*x, w, p.wq(w));
            // `|s| < q` and `|x| < 0.75 q`.
            *s = p.reduce(*s - *x);
        }
    }
    let (h, hq) = half(p);
    for ((lo, hi), &w) in e0.iter_mut().zip(s).zip(&iw[..n]) {
        let x = p.mont(*lo, w, p.wq(w));
        (*lo, *hi) = (p.mont(*hi + x, h, hq), p.mont(*hi - x, h, hq));
    }
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

impl Kernels {
    pub fn portable() -> Self {
        Kernels {
            load: portable::load,
            forward: portable::forward,
            inverse: portable::inverse,
            pointwise: portable::pointwise,
            square: portable::square,
            garner: portable::garner,
            load_twist: portable::load_twist,
            load_add: portable::load_add,
            twist: portable::twist,
            fold_sub: portable::fold_sub,
            halves: portable::halves,
            pair: portable::pair,
        }
    }
}

//! The transform kernels with Wasm SIMD128, four `i32` lanes per vector.
//!
//! Each computes exactly what its `portable.rs` counterpart computes, bit for bit. Wasm
//! has no 32-bit high multiply, so the products are widened with `extmul` (`smull` on
//! `AArch64` hosts) and the wanted halves picked by one shuffle. The `h = 2` and `h = 1`
//! stages run inside registers.

use core::arch::wasm32::*;

use super::Kernels;
use super::crt::GARNER;
use super::portable;
use super::tables::{PRIMES, Prime, S};

/// The high halves of the four signed products `a[i] b[i]`, given as two `i64x2`.
#[inline(always)]
fn high(lo: v128, hi: v128) -> v128 {
    i32x4_shuffle::<1, 3, 5, 7>(lo, hi)
}

/// [`Prime::mont`]: `a w - m q` is a multiple of `2^32` and fits `i64`, so its high
/// half is the result.
#[inline(always)]
fn mont(a: v128, w: v128, wq: v128, q: v128) -> v128 {
    let m = i32x4_mul(a, wq);
    let lo = i64x2_sub(i64x2_extmul_low_i32x4(a, w), i64x2_extmul_low_i32x4(m, q));
    let hi = i64x2_sub(i64x2_extmul_high_i32x4(a, w), i64x2_extmul_high_i32x4(m, q));
    high(lo, hi)
}

/// [`Prime::reduce`]. With `P = x v`, it takes `t = (floor(P / 2^31) + 2^(S-1)) >> S`;
/// both that and `(floor(P / 2^32) + 2^(S-2)) >> (S-1)` are `floor((P + 2^(S+30)) /
/// 2^(S+31))`, and the second needs only the high halves.
#[inline(always)]
fn reduce(x: v128, q: v128, v: v128) -> v128 {
    let p = high(i64x2_extmul_low_i32x4(x, v), i64x2_extmul_high_i32x4(x, v));
    let t = i32x4_shr(i32x4_add(p, i32x4_splat(1 << (S - 2))), (S - 1) as u32);
    i32x4_sub(x, i32x4_mul(t, q))
}

/// [`Prime::normalize`].
#[inline(always)]
fn normalize(x: v128, q: v128) -> v128 {
    i32x4_add(x, v128_and(i32x4_shr(x, 31), q))
}

/// `portable::lift` of four 32-bit coefficients.
#[inline(always)]
fn lift(x: v128, r32: v128) -> v128 {
    i32x4_add(x, v128_and(i32x4_shr(x, 31), r32))
}

/// Four `i32` at `p`.
///
/// # Safety
///
/// `p` must have four readable `i32`s.
#[inline(always)]
unsafe fn ld<T>(p: *const T) -> v128 {
    // SAFETY: the caller's contract; Wasm loads need no alignment.
    unsafe { v128_load(p.cast()) }
}

/// Stores four `i32` at `p`.
///
/// # Safety
///
/// `p` must have four writable `i32`s.
#[inline(always)]
unsafe fn st(p: *mut i32, x: v128) {
    // SAFETY: the caller's contract.
    unsafe { v128_store(p.cast(), x) }
}

/// `c`, read from memory. Loops call this for the splats that `extmul` multiplies:
/// LLVM otherwise hoists their sign extension out of the loop, and the product becomes
/// an `i64x2.mul`, which V8 expands to about seven instructions on `AArch64`.
#[inline(always)]
fn fresh<const N: usize>(c: &[v128; N]) -> [v128; N] {
    *core::hint::black_box(c)
}

/// The constants of `p`: `q`, `v` and `q^-1`.
#[inline(always)]
fn consts(p: Prime) -> (v128, v128, v128) {
    (i32x4_splat(p.q), i32x4_splat(p.v), i32x4_splat(p.qinv))
}

pub(crate) fn load(dst: &mut [i32], limbs: &[u64], p: Prime) {
    let m = 2 * limbs.len();
    assert!(dst.len() >= m);
    let full = m / 4 * 4;
    let (qv, vv, _) = consts(p);
    let (r32, c) = (i32x4_splat(p.r32), [vv]);
    let src = limbs.as_ptr().cast::<u32>();
    for i in (0..full).step_by(4) {
        let [vv] = fresh(&c);
        // SAFETY: `i + 4 <= full <= m`: the `u32` halves of `limbs` (little-endian, so
        // low half first) and the start of `dst`.
        unsafe {
            st(
                dst.as_mut_ptr().add(i),
                reduce(lift(ld(src.add(i)), r32), qv, vv),
            )
        }
    }
    if full < m {
        let l = limbs[limbs.len() - 1];
        dst[full] = portable::load_one(l as u32, p);
        dst[full + 1] = portable::load_one((l >> 32) as u32, p);
    }
    dst[m..].fill(0);
}

/// `[t[2], t[3], t[2], t[3]]`, the twiddles of the `h = 2` stage for two blocks.
#[inline(always)]
fn h2(t: &[i32]) -> v128 {
    i32x4(t[2], t[3], t[2], t[3])
}

pub(crate) fn forward(a: &mut [i32], p: Prime, fw: &[i32]) {
    let n = a.len();
    assert!(n.is_power_of_two() && n >= 8 && n <= fw.len());
    let (qv, vv, qinv) = consts(p);
    let c = [qv, vv];
    let ptr = a.as_mut_ptr();
    let mut h = n / 2;
    while h >= 4 {
        let w = fw[h..2 * h].as_ptr();
        for s in (0..n).step_by(2 * h) {
            for j in (0..h).step_by(4) {
                let [qv, vv] = fresh(&c);
                // SAFETY: `s + j + h + 4 <= n`, and `j + 4 <= h` twiddles.
                unsafe {
                    let (px, py) = (ptr.add(s + j), ptr.add(s + j + h));
                    let (x, y) = (ld(px), ld(py));
                    let w = ld(w.add(j));
                    let wq = i32x4_mul(w, qinv);
                    st(px, reduce(i32x4_add(x, y), qv, vv));
                    st(py, mont(i32x4_sub(x, y), w, wq, qv));
                }
            }
        }
        h /= 2;
    }
    let w2 = h2(fw);
    let w2q = i32x4_mul(w2, qinv);
    let c = [qv, vv, w2];
    for i in (0..n).step_by(8) {
        let [qv, vv, w2] = fresh(&c);
        // SAFETY: `i + 8 <= n`.
        unsafe {
            let (v0, v1) = (ld(ptr.add(i)), ld(ptr.add(i + 4)));
            // h = 2: pairs (0, 2), (1, 3) of each 4-element block.
            let (x, y) = (i64x2_shuffle::<0, 2>(v0, v1), i64x2_shuffle::<1, 3>(v0, v1));
            let (x, y) = (
                reduce(i32x4_add(x, y), qv, vv),
                mont(i32x4_sub(x, y), w2, w2q, qv),
            );
            let (v0, v1) = (i64x2_shuffle::<0, 2>(x, y), i64x2_shuffle::<1, 3>(x, y));
            // h = 1: pairs (0, 1), twiddle 1.
            let (x, y) = (
                i32x4_shuffle::<0, 2, 4, 6>(v0, v1),
                i32x4_shuffle::<1, 3, 5, 7>(v0, v1),
            );
            let (x, y) = (
                reduce(i32x4_add(x, y), qv, vv),
                reduce(i32x4_sub(x, y), qv, vv),
            );
            st(ptr.add(i), i32x4_shuffle::<0, 4, 1, 5>(x, y));
            st(ptr.add(i + 4), i32x4_shuffle::<2, 6, 3, 7>(x, y));
        }
    }
}

pub(crate) fn inverse(a: &mut [i32], p: Prime, iw: &[i32]) {
    let n = a.len();
    assert!(n.is_power_of_two() && n >= 8 && n <= iw.len());
    let (qv, vv, qinv) = consts(p);
    let ptr = a.as_mut_ptr();
    let w2 = h2(iw);
    let w2q = i32x4_mul(w2, qinv);
    let c = [qv, vv, w2];
    for i in (0..n).step_by(8) {
        let [qv, vv, w2] = fresh(&c);
        // SAFETY: `i + 8 <= n`.
        unsafe {
            let (v0, v1) = (ld(ptr.add(i)), ld(ptr.add(i + 4)));
            // h = 1, twiddle 1, unreduced (`portable::reduces_x`).
            let (x, y) = (
                i32x4_shuffle::<0, 2, 4, 6>(v0, v1),
                i32x4_shuffle::<1, 3, 5, 7>(v0, v1),
            );
            let (x, y) = (i32x4_add(x, y), i32x4_sub(x, y));
            let (v0, v1) = (
                i32x4_shuffle::<0, 4, 1, 5>(x, y),
                i32x4_shuffle::<2, 6, 3, 7>(x, y),
            );
            // h = 2 reduces x: `n >= 8`, so this is not the last stage.
            let (x, y) = (i64x2_shuffle::<0, 2>(v0, v1), i64x2_shuffle::<1, 3>(v0, v1));
            let (x, y) = (reduce(x, qv, vv), mont(y, w2, w2q, qv));
            let (x, y) = (i32x4_add(x, y), i32x4_sub(x, y));
            st(ptr.add(i), i64x2_shuffle::<0, 2>(x, y));
            st(ptr.add(i + 4), i64x2_shuffle::<1, 3>(x, y));
        }
    }
    let mut h = 4;
    while h < n {
        match (portable::reduces_x(h), 2 * h == n) {
            (false, false) => inverse_stage::<false, false>(a, h, p, iw),
            (true, false) => inverse_stage::<true, false>(a, h, p, iw),
            (false, true) => inverse_stage::<false, true>(a, h, p, iw),
            (true, true) => inverse_stage::<true, true>(a, h, p, iw),
        }
        h *= 2;
    }
}

/// One stage of [`inverse`]: `h` is a multiple of 4, `a.len()` a multiple of `2 h` that
/// the twiddles serve. `RX` and `LAST` are as in `portable::inverse_stage`.
#[inline(always)]
fn inverse_stage<const RX: bool, const LAST: bool>(a: &mut [i32], h: usize, p: Prime, iw: &[i32]) {
    let n = a.len();
    assert!(h >= 4 && h.is_multiple_of(4) && n.is_multiple_of(2 * h) && n <= iw.len());
    let (qv, vv, qinv) = consts(p);
    let c = [qv, vv];
    let ptr = a.as_mut_ptr();
    let w = iw[h..2 * h].as_ptr();
    for s in (0..n).step_by(2 * h) {
        for j in (0..h).step_by(4) {
            let [qv, vv] = fresh(&c);
            // SAFETY: `n` is a multiple of `2 h` and `h` of 4, so `s + 2 h <= n` and
            // `j + 4 <= h`: `s + j + h + 4 <= n`, and `j + 4 <= h` twiddles.
            unsafe {
                let (px, py) = (ptr.add(s + j), ptr.add(s + j + h));
                let w = ld(w.add(j));
                let wq = i32x4_mul(w, qinv);
                let x = ld(px);
                let x = if RX { reduce(x, qv, vv) } else { x };
                let y = mont(ld(py), w, wq, qv);
                let (sum, dif) = (i32x4_add(x, y), i32x4_sub(x, y));
                if LAST {
                    st(px, reduce(sum, qv, vv));
                    st(py, reduce(dif, qv, vv));
                } else {
                    st(px, sum);
                    st(py, dif);
                }
            }
        }
    }
}

pub(crate) fn pointwise(a: &mut [i32], b: &[i32], p: Prime) {
    let n = a.len();
    assert!(n == b.len() && n.is_multiple_of(4));
    let (k, kq) = p.scale(n);
    let (qv, _, qinv) = consts(p);
    let (k, kq) = (i32x4_splat(k), i32x4_splat(kq));
    let c = [qv, k];
    for i in (0..n).step_by(4) {
        let [qv, k] = fresh(&c);
        // SAFETY: `i + 4 <= n` for both slices.
        unsafe {
            let x = ld(a.as_ptr().add(i));
            let y = ld(b.as_ptr().add(i));
            let c = mont(x, y, i32x4_mul(y, qinv), qv);
            st(a.as_mut_ptr().add(i), mont(c, k, kq, qv));
        }
    }
}

pub(crate) fn square(a: &mut [i32], p: Prime) {
    let n = a.len();
    assert!(n.is_multiple_of(4));
    let (k, kq) = p.scale(n);
    let (qv, _, qinv) = consts(p);
    let (k, kq) = (i32x4_splat(k), i32x4_splat(kq));
    let c = [qv, k];
    for i in (0..n).step_by(4) {
        let [qv, k] = fresh(&c);
        // SAFETY: `i + 4 <= n`.
        unsafe {
            let x = ld(a.as_ptr().add(i));
            let c = mont(x, x, i32x4_mul(x, qinv), qv);
            st(a.as_mut_ptr().add(i), mont(c, k, kq, qv));
        }
    }
}

pub(crate) fn garner([r1, r2, r3]: [&mut [i32]; 3]) {
    let n = r1.len();
    assert!(n == r2.len() && n == r3.len() && n.is_multiple_of(4));
    let [p1, p2, p3] = PRIMES;
    let (q1, q2, q3) = (i32x4_splat(p1.q), i32x4_splat(p2.q), i32x4_splat(p3.q));
    let g = GARNER;
    let (c12, c12q) = (i32x4_splat(g.c12), i32x4_splat(g.c12q));
    let (c13, c13q) = (i32x4_splat(g.c13), i32x4_splat(g.c13q));
    let (c23, c23q) = (i32x4_splat(g.c23), i32x4_splat(g.c23q));
    let k = [q2, q3, c12, c13, c23];
    for i in (0..n).step_by(4) {
        let [q2, q3, c12, c13, c23] = fresh(&k);
        // SAFETY: `i + 4 <= n` for all three slices.
        unsafe {
            // Only `a` is normalized, as in `portable::garner`.
            let a = normalize(ld(r1.as_ptr().add(i)), q1);
            let (b, c) = (ld(r2.as_ptr().add(i)), ld(r3.as_ptr().add(i)));
            let v2 = normalize(mont(i32x4_sub(b, a), c12, c12q, q2), q2);
            let t = mont(i32x4_sub(c, a), c13, c13q, q3);
            let v3 = normalize(mont(i32x4_sub(t, v2), c23, c23q, q3), q3);
            st(r1.as_mut_ptr().add(i), a);
            st(r2.as_mut_ptr().add(i), v2);
            st(r3.as_mut_ptr().add(i), v3);
        }
    }
}

/// `radix::garner_two_portable` for lengths divisible by four.
pub(super) fn garner_two(r1: &mut [i32], r2: &mut [i32]) {
    let n = r1.len();
    assert!(n == r2.len() && n.is_multiple_of(4));
    let [p1, p2, _] = PRIMES;
    let (q1, q2) = (i32x4_splat(p1.q), i32x4_splat(p2.q));
    let c12q = i32x4_splat(GARNER.c12q);
    let k = [q2, i32x4_splat(GARNER.c12)];
    for i in (0..n).step_by(4) {
        let [q2, c12] = fresh(&k);
        // SAFETY: `i + 4 <= n` for both slices.
        unsafe {
            let a = normalize(ld(r1.as_ptr().add(i)), q1);
            let b = ld(r2.as_ptr().add(i));
            let v2 = normalize(mont(i32x4_sub(b, a), c12, c12q, q2), q2);
            st(r1.as_mut_ptr().add(i), a);
            st(r2.as_mut_ptr().add(i), v2);
        }
    }
}

pub(crate) fn load_twist(dst: &mut [i32], limbs: &[u64], p: Prime, w: &[i32]) {
    let m = 2 * limbs.len();
    assert!(dst.len() >= m && w.len() >= m);
    let full = m / 4 * 4;
    let (qv, _, qinv) = consts(p);
    let (r32, c) = (i32x4_splat(p.r32), [qv]);
    let src = limbs.as_ptr().cast::<u32>();
    for i in (0..full).step_by(4) {
        let [qv] = fresh(&c);
        // SAFETY: `i + 4 <= full <= m`, within the halves of `limbs`, `dst`, `w`.
        unsafe {
            let w = ld(w.as_ptr().add(i));
            let wq = i32x4_mul(w, qinv);
            let x = lift(ld(src.add(i)), r32);
            st(dst.as_mut_ptr().add(i), mont(x, w, wq, qv));
        }
    }
    if full < m {
        let mut tail = [0; 2];
        portable::load_twist(&mut tail, &limbs[limbs.len() - 1..], p, &w[full..]);
        dst[full..m].copy_from_slice(&tail);
    }
    dst[m..].fill(0);
}

pub(crate) fn load_add(dst: &mut [i32], limbs: &[u64], p: Prime, sub: bool) {
    let m = 2 * limbs.len();
    assert!(dst.len() >= m);
    let full = m / 4 * 4;
    let (qv, vv, _) = consts(p);
    let r32 = i32x4_splat(p.r32);
    // `x * s` is `x` or `-x`.
    let s = i32x4_splat(if sub { -1 } else { 1 });
    let c = [vv];
    let src = limbs.as_ptr().cast::<u32>();
    for i in (0..full).step_by(4) {
        let [vv] = fresh(&c);
        // SAFETY: `i + 4 <= full <= m`, within the halves of `limbs` and `dst`.
        unsafe {
            let d = dst.as_mut_ptr().add(i);
            let x = i32x4_mul(reduce(lift(ld(src.add(i)), r32), qv, vv), s);
            st(d, reduce(i32x4_add(ld(d), x), qv, vv));
        }
    }
    if full < m {
        portable::load_add(&mut dst[full..m], &limbs[limbs.len() - 1..], p, sub);
    }
}

pub(crate) fn twist(a: &mut [i32], w: &[i32], p: Prime) {
    let n = a.len();
    assert!(n.is_multiple_of(4) && w.len() >= n);
    let (qv, _, qinv) = consts(p);
    let c = [qv];
    for i in (0..n).step_by(4) {
        let [qv] = fresh(&c);
        // SAFETY: `i + 4 <= n`, within `a` and `w`.
        unsafe {
            let w = ld(w.as_ptr().add(i));
            let wq = i32x4_mul(w, qinv);
            let x = a.as_mut_ptr().add(i);
            st(x, mont(ld(x), w, wq, qv));
        }
    }
}

pub(crate) fn fold_sub(dst: &mut [i32], src: &[i32], start: usize, neg: bool, p: Prime) {
    let d = dst.len();
    assert!(d.is_multiple_of(4) && src.len().is_multiple_of(4) && start.is_multiple_of(4));
    let (qv, vv, _) = consts(p);
    let c = [vv];
    let (mut pos, mut src) = (start, src);
    while !src.is_empty() {
        let at = pos % d;
        let (run, rest) = src.split_at((d - at).min(src.len()));
        let s = i32x4_splat(if neg && (pos / d) % 2 == 1 { -1 } else { 1 });
        let x = dst[at..at + run.len()].as_mut_ptr();
        for i in (0..run.len()).step_by(4) {
            let [vv] = fresh(&c);
            // SAFETY: `i + 4 <= run.len()`, within `run` and `dst[at..]`: lengths are
            // multiples of 4.
            unsafe {
                let y = i32x4_mul(ld(run.as_ptr().add(i)), s);
                st(x.add(i), reduce(i32x4_sub(ld(x.add(i)), y), qv, vv));
            }
        }
        (pos, src) = (pos + run.len(), rest);
    }
}

pub(crate) fn halves(lo: &mut [i32], hi: &mut [i32], p: Prime) {
    let n = lo.len();
    assert!(n == hi.len() && n.is_multiple_of(4));
    let qv = i32x4_splat(p.q);
    let (h, hq) = portable::half(p);
    let (h, hq) = (i32x4_splat(h), i32x4_splat(hq));
    let c = [qv, h];
    for i in (0..n).step_by(4) {
        let [qv, h] = fresh(&c);
        // SAFETY: `i + 4 <= n`, within `lo` and `hi`.
        unsafe {
            let (pl, ph) = (lo.as_mut_ptr().add(i), hi.as_mut_ptr().add(i));
            let (x, y) = (ld(pl), ld(ph));
            st(pl, mont(i32x4_add(y, x), h, hq, qv));
            st(ph, mont(i32x4_sub(y, x), h, hq, qv));
        }
    }
}

pub(crate) fn pair(r: &mut [i32], m: usize, iw: &[i32], p: Prime) {
    let n = r.len() - m;
    assert!(n < m && n.is_multiple_of(4) && m.is_multiple_of(n));
    assert!(iw.len() >= m);
    let (qv, vv, qinv) = consts(p);
    let c = [qv, vv];
    let (r, w) = (r.as_mut_ptr(), iw.as_ptr());
    for b in (n..m).step_by(n) {
        for i in (0..n).step_by(4) {
            let [qv, vv] = fresh(&c);
            // SAFETY: `b + i + 4 <= m` and `m + i + 4 <= m + n`, within `r`, `iw`.
            unsafe {
                let px = r.add(b + i);
                let w = ld(w.add(b + i));
                let x = mont(ld(px), w, i32x4_mul(w, qinv), qv);
                st(px, x);
                let ps = r.add(m + i);
                st(ps, reduce(i32x4_sub(ld(ps), x), qv, vv));
            }
        }
    }
    let (h, hq) = portable::half(p);
    let (h, hq) = (i32x4_splat(h), i32x4_splat(hq));
    let c = [qv, h];
    for i in (0..n).step_by(4) {
        let [qv, h] = fresh(&c);
        // SAFETY: `i + 4 <= n` and `m + i + 4 <= m + n`, within `r`, `iw`.
        unsafe {
            let (plo, phi) = (r.add(i), r.add(m + i));
            let w = ld(w.add(i));
            let x = mont(ld(plo), w, i32x4_mul(w, qinv), qv);
            let s = ld(phi);
            st(plo, mont(i32x4_add(s, x), h, hq, qv));
            st(phi, mont(i32x4_sub(s, x), h, hq, qv));
        }
    }
}

impl Kernels {
    pub fn wasm() -> Self {
        Kernels {
            load,
            forward,
            inverse,
            pointwise,
            square,
            garner,
            load_twist,
            load_add,
            twist,
            fold_sub,
            halves,
            pair,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntt::tables::tests::samples;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::SeedableRng;

    /// `mont` and `reduce` on edge and random lanes against the scalar ones.
    #[test]
    fn arithmetic_matches_scalar() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(41);
        for p in PRIMES {
            let (qv, vv, qinv) = consts(p);
            let wide = samples(&mut rng, i32::MAX, 4000);
            let wide = [&[i32::MIN, i32::MIN + 1][..], &wide].concat();
            let w = samples(&mut rng, p.q / 2 + 1, 4000);
            for (x, w) in wide
                .as_chunks::<4>()
                .0
                .iter()
                .zip(w.as_chunks::<4>().0.iter().cycle())
            {
                let (xv, wv) = (i32x4(x[0], x[1], x[2], x[3]), i32x4(w[0], w[1], w[2], w[3]));
                let got = reduce(xv, qv, vv);
                let m = mont(xv, wv, i32x4_mul(wv, qinv), qv);
                for l in 0..4 {
                    let lane = |v: v128| -> i32 {
                        let mut a = [0i32; 4];
                        // SAFETY: `a` has four `i32`s.
                        unsafe { st(a.as_mut_ptr(), v) };
                        a[l]
                    };
                    assert_eq!(lane(got), p.reduce(x[l]), "reduce {}", x[l]);
                    assert_eq!(lane(m), p.mont(x[l], w[l], p.wq(w[l])), "mont {}", x[l]);
                }
            }
        }
    }

    /// Each kernel gives exactly what the portable one gives.
    #[test]
    fn matches_portable() {
        crate::ntt::tests::check_kernels_match_portable(&Kernels::wasm());
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
        let n = 4096;
        let r1 = samples(&mut rng, PRIMES[0].q, n)[..n].to_vec();
        let r2 = samples(&mut rng, PRIMES[1].q, n)[..n].to_vec();
        let ((mut a1, mut a2), (mut b1, mut b2)) = ((r1.clone(), r2.clone()), (r1, r2));
        garner_two(&mut a1, &mut a2);
        crate::ntt::radix::garner_two_portable(&mut b1, &mut b2);
        assert_eq!((a1, a2), (b1, b2), "garner_two");
    }
}

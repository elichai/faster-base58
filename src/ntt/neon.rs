//! The transform kernels with NEON intrinsics, four `i32` lanes per vector.
//!
//! Each computes exactly what its `portable.rs` counterpart computes, bit for bit:
//! `sqdmulh` + `vhsub` is the same exact Montgomery quotient as [`Prime::mont`], and
//! `sqdmulh` + `srshr` + `mls` is [`Prime::reduce`]. The `h = 2` and `h = 1` stages run
//! inside registers with `zip`/`uzp`.

use core::arch::aarch64::*;

use super::Kernels;
use super::crt::GARNER;
use super::portable;
use super::tables::{PRIMES, Prime, S, Twiddles};

/// [`Prime::mont`]: `sqdmulh(a, w)` is `floor(a w / 2^31)`, and its difference with
/// `floor(m q / 2^31)` is even, because `a w = m q mod 2^32`. `sqdmulh` saturates only
/// for `-2^31 * -2^31`, which `mont`'s bounds exclude.
#[inline]
#[target_feature(enable = "neon")]
fn mont(a: int32x4_t, w: int32x4_t, wq: int32x4_t, q: int32x4_t) -> int32x4_t {
    let m = vmulq_s32(a, wq);
    vhsubq_s32(vqdmulhq_s32(a, w), vqdmulhq_s32(m, q))
}

/// [`Prime::reduce`].
#[inline]
#[target_feature(enable = "neon")]
fn reduce(x: int32x4_t, q: int32x4_t, v: int32x4_t) -> int32x4_t {
    vmlsq_s32(x, vrshrq_n_s32::<S>(vqdmulhq_s32(x, v)), q)
}

/// [`Prime::normalize`].
#[inline]
#[target_feature(enable = "neon")]
fn normalize(x: int32x4_t, q: int32x4_t) -> int32x4_t {
    vaddq_s32(x, vandq_s32(vshrq_n_s32::<31>(x), q))
}

#[inline]
#[target_feature(enable = "neon")]
fn zip1_64(a: int32x4_t, b: int32x4_t) -> int32x4_t {
    vreinterpretq_s32_s64(vzip1q_s64(
        vreinterpretq_s64_s32(a),
        vreinterpretq_s64_s32(b),
    ))
}

#[inline]
#[target_feature(enable = "neon")]
fn zip2_64(a: int32x4_t, b: int32x4_t) -> int32x4_t {
    vreinterpretq_s32_s64(vzip2q_s64(
        vreinterpretq_s64_s32(a),
        vreinterpretq_s64_s32(b),
    ))
}

/// `[t[2], t[3], t[2], t[3]]`, the twiddles of the `h = 2` stage for two blocks.
#[target_feature(enable = "neon")]
fn h2(t: &[i32]) -> int32x4_t {
    let pair = [t[2], t[3], t[2], t[3]];
    // SAFETY: `pair` has four elements.
    unsafe { vld1q_s32(pair.as_ptr()) }
}

// The kernels. This module only builds with `target_feature = "neon"`, so calling the
// `#[target_feature(enable = "neon")]` bodies is sound.

pub(crate) fn load(dst: &mut [i32], limbs: &[u64], p: Prime) {
    // SAFETY: NEON is enabled for this build.
    unsafe { load_neon(dst, limbs, p) }
}

pub(crate) fn forward(a: &mut [i32], p: Prime, t: &Twiddles) {
    // SAFETY: NEON is enabled for this build.
    unsafe { forward_neon(a, p, t) }
}

pub(crate) fn inverse(a: &mut [i32], p: Prime, t: &Twiddles) {
    // SAFETY: NEON is enabled for this build.
    unsafe { inverse_neon(a, p, t) }
}

pub(crate) fn pointwise(a: &mut [i32], b: &[i32], p: Prime) {
    // SAFETY: NEON is enabled for this build.
    unsafe { pointwise_neon(a, b, p) }
}

pub(crate) fn square(a: &mut [i32], p: Prime) {
    // SAFETY: NEON is enabled for this build.
    unsafe { square_neon(a, p) }
}

pub(crate) fn garner(r: [&mut [i32]; 3]) {
    // SAFETY: NEON is enabled for this build.
    unsafe { garner_neon(r) }
}

/// Two-prime Garner preprocessing: replace residues in `(-q1, q1)` and `(-q2, q2)`
/// with `r1` in `[0, q1)` and `v2 = (r2 - r1) / q1 mod q2` in `[0, q2)`.
/// Both slices must have the same length, divisible by four.
pub(super) fn garner_two(r1: &mut [i32], r2: &mut [i32]) {
    // SAFETY: NEON is enabled for this build.
    unsafe { garner_two_neon(r1, r2) }
}

pub(crate) fn load_twist(dst: &mut [i32], limbs: &[u64], p: Prime, w: &[i32], wq: &[i32]) {
    // SAFETY: NEON is enabled for this build.
    unsafe { load_twist_neon(dst, limbs, p, w, wq) }
}

pub(crate) fn load_add(dst: &mut [i32], limbs: &[u64], p: Prime, sub: bool) {
    // SAFETY: NEON is enabled for this build.
    unsafe { load_add_neon(dst, limbs, p, sub) }
}

pub(crate) fn twist(a: &mut [i32], w: &[i32], wq: &[i32], p: Prime) {
    // SAFETY: NEON is enabled for this build.
    unsafe { twist_neon(a, w, wq, p) }
}

pub(crate) fn fold_sub(dst: &mut [i32], src: &[i32], start: usize, neg: bool, p: Prime) {
    // SAFETY: NEON is enabled for this build.
    unsafe { fold_sub_neon(dst, src, start, neg, p) }
}

pub(crate) fn halves(lo: &mut [i32], hi: &mut [i32], p: Prime) {
    // SAFETY: NEON is enabled for this build.
    unsafe { halves_neon(lo, hi, p) }
}

pub(crate) fn pair(r: &mut [i32], m: usize, iw: &[i32], iwq: &[i32], p: Prime) {
    // SAFETY: NEON is enabled for this build.
    unsafe { pair_neon(r, m, iw, iwq, p) }
}

#[target_feature(enable = "neon")]
fn load_neon(dst: &mut [i32], limbs: &[u64], p: Prime) {
    let m = 2 * limbs.len();
    assert!(dst.len() >= m);
    let full = m / 4 * 4;
    let (qv, vv, r32) = (vdupq_n_s32(p.q), vdupq_n_s32(p.v), vdupq_n_s32(p.r32));
    let src = limbs.as_ptr().cast::<u32>();
    for i in (0..full).step_by(4) {
        // SAFETY: `i + 4 <= full <= m`: the `u32` halves of `limbs` (little-endian, so
        // low half first) and the start of `dst`.
        unsafe {
            let x = vreinterpretq_s32_u32(vld1q_u32(src.add(i)));
            let x = vaddq_s32(x, vandq_s32(vshrq_n_s32::<31>(x), r32));
            vst1q_s32(dst.as_mut_ptr().add(i), reduce(x, qv, vv));
        }
    }
    if full < m {
        let l = limbs[limbs.len() - 1];
        dst[full] = portable::load_one(l as u32, p);
        dst[full + 1] = portable::load_one((l >> 32) as u32, p);
    }
    dst[m..].fill(0);
}

#[target_feature(enable = "neon")]
fn forward_neon(a: &mut [i32], p: Prime, t: &Twiddles) {
    let n = a.len();
    assert!(n.is_power_of_two() && n >= 8 && n <= t.fw.len());
    let (qv, vv) = (vdupq_n_s32(p.q), vdupq_n_s32(p.v));
    let ptr = a.as_mut_ptr();
    let mut h = n / 2;
    while h >= 4 {
        let (w, wq) = (t.fw[h..2 * h].as_ptr(), t.fwq[h..2 * h].as_ptr());
        for s in (0..n).step_by(2 * h) {
            for j in (0..h).step_by(4) {
                // SAFETY: `s + j + h + 4 <= n`, and `j + 4 <= h` twiddles.
                unsafe {
                    let (px, py) = (ptr.add(s + j), ptr.add(s + j + h));
                    let (x, y) = (vld1q_s32(px), vld1q_s32(py));
                    let (w, wq) = (vld1q_s32(w.add(j)), vld1q_s32(wq.add(j)));
                    vst1q_s32(px, reduce(vaddq_s32(x, y), qv, vv));
                    vst1q_s32(py, mont(vsubq_s32(x, y), w, wq, qv));
                }
            }
        }
        h /= 2;
    }
    let (w2, w2q) = (h2(&t.fw), h2(&t.fwq));
    for i in (0..n).step_by(8) {
        // SAFETY: `i + 8 <= n`.
        unsafe {
            let (v0, v1) = (vld1q_s32(ptr.add(i)), vld1q_s32(ptr.add(i + 4)));
            // h = 2: pairs (0, 2), (1, 3) of each 4-element block.
            let (x, y) = (zip1_64(v0, v1), zip2_64(v0, v1));
            let (x, y) = (
                reduce(vaddq_s32(x, y), qv, vv),
                mont(vsubq_s32(x, y), w2, w2q, qv),
            );
            let (v0, v1) = (zip1_64(x, y), zip2_64(x, y));
            // h = 1: pairs (0, 1), twiddle 1.
            let (x, y) = (vuzp1q_s32(v0, v1), vuzp2q_s32(v0, v1));
            let (x, y) = (
                reduce(vaddq_s32(x, y), qv, vv),
                reduce(vsubq_s32(x, y), qv, vv),
            );
            vst1q_s32(ptr.add(i), vzip1q_s32(x, y));
            vst1q_s32(ptr.add(i + 4), vzip2q_s32(x, y));
        }
    }
}

#[target_feature(enable = "neon")]
fn inverse_neon(a: &mut [i32], p: Prime, t: &Twiddles) {
    let n = a.len();
    assert!(n.is_power_of_two() && n >= 8 && n <= t.iw.len());
    let (qv, vv) = (vdupq_n_s32(p.q), vdupq_n_s32(p.v));
    let ptr = a.as_mut_ptr();
    let (w2, w2q) = (h2(&t.iw), h2(&t.iwq));
    for i in (0..n).step_by(8) {
        // SAFETY: `i + 8 <= n`.
        unsafe {
            let (v0, v1) = (vld1q_s32(ptr.add(i)), vld1q_s32(ptr.add(i + 4)));
            // h = 1, twiddle 1.
            let (x, y) = (vuzp1q_s32(v0, v1), vuzp2q_s32(v0, v1));
            let (x, y) = (
                reduce(vaddq_s32(x, y), qv, vv),
                reduce(vsubq_s32(x, y), qv, vv),
            );
            let (v0, v1) = (vzip1q_s32(x, y), vzip2q_s32(x, y));
            // h = 2, lazy (`portable::lazy`): `n >= 8`, so this is not the last stage.
            let (x, y) = (zip1_64(v0, v1), zip2_64(v0, v1));
            let y = mont(y, w2, w2q, qv);
            let (x, y) = (vaddq_s32(x, y), vsubq_s32(x, y));
            vst1q_s32(ptr.add(i), zip1_64(x, y));
            vst1q_s32(ptr.add(i + 4), zip2_64(x, y));
        }
    }
    let mut h = 4;
    while h < n {
        if portable::lazy(h, n) {
            inverse_stage::<true>(a, h, p, t);
        } else {
            inverse_stage::<false>(a, h, p, t);
        }
        h *= 2;
    }
}

/// One stage of [`inverse_neon`]: `h` is a multiple of 4, `a.len()` a multiple of `2 h`
/// that the twiddles serve. `LAZY` skips the reductions, as `portable::inverse_stage`
/// does.
#[inline]
#[target_feature(enable = "neon")]
fn inverse_stage<const LAZY: bool>(a: &mut [i32], h: usize, p: Prime, t: &Twiddles) {
    let n = a.len();
    assert!(h >= 4 && h.is_multiple_of(4) && n.is_multiple_of(2 * h) && n <= t.iw.len());
    let (qv, vv) = (vdupq_n_s32(p.q), vdupq_n_s32(p.v));
    let ptr = a.as_mut_ptr();
    let (w, wq) = (t.iw[h..2 * h].as_ptr(), t.iwq[h..2 * h].as_ptr());
    for s in (0..n).step_by(2 * h) {
        for j in (0..h).step_by(4) {
            // SAFETY: `n` is a multiple of `2 h` and `h` of 4, so `s + 2 h <= n` and
            // `j + 4 <= h`: `s + j + h + 4 <= n`, and `j + 4 <= h` twiddles.
            unsafe {
                let (px, py) = (ptr.add(s + j), ptr.add(s + j + h));
                let (w, wq) = (vld1q_s32(w.add(j)), vld1q_s32(wq.add(j)));
                let x = vld1q_s32(px);
                let y = mont(vld1q_s32(py), w, wq, qv);
                let (sum, dif) = (vaddq_s32(x, y), vsubq_s32(x, y));
                if LAZY {
                    vst1q_s32(px, sum);
                    vst1q_s32(py, dif);
                } else {
                    vst1q_s32(px, reduce(sum, qv, vv));
                    vst1q_s32(py, reduce(dif, qv, vv));
                }
            }
        }
    }
}

#[target_feature(enable = "neon")]
fn pointwise_neon(a: &mut [i32], b: &[i32], p: Prime) {
    let n = a.len();
    assert!(n == b.len() && n.is_multiple_of(4));
    let (k, kq) = p.scale(n);
    let (qv, qinv) = (vdupq_n_s32(p.q), vdupq_n_s32(p.qinv));
    let (k, kq) = (vdupq_n_s32(k), vdupq_n_s32(kq));
    for i in (0..n).step_by(4) {
        // SAFETY: `i + 4 <= n` for both slices.
        unsafe {
            let x = vld1q_s32(a.as_ptr().add(i));
            let y = vld1q_s32(b.as_ptr().add(i));
            let c = mont(x, y, vmulq_s32(y, qinv), qv);
            vst1q_s32(a.as_mut_ptr().add(i), mont(c, k, kq, qv));
        }
    }
}

#[target_feature(enable = "neon")]
fn square_neon(a: &mut [i32], p: Prime) {
    let n = a.len();
    assert!(n.is_multiple_of(4));
    let (k, kq) = p.scale(n);
    let (qv, qinv) = (vdupq_n_s32(p.q), vdupq_n_s32(p.qinv));
    let (k, kq) = (vdupq_n_s32(k), vdupq_n_s32(kq));
    for i in (0..n).step_by(4) {
        // SAFETY: `i + 4 <= n`.
        unsafe {
            let x = vld1q_s32(a.as_ptr().add(i));
            let c = mont(x, x, vmulq_s32(x, qinv), qv);
            vst1q_s32(a.as_mut_ptr().add(i), mont(c, k, kq, qv));
        }
    }
}

#[target_feature(enable = "neon")]
fn garner_neon([r1, r2, r3]: [&mut [i32]; 3]) {
    let n = r1.len();
    assert!(n == r2.len() && n == r3.len() && n.is_multiple_of(4));
    let [p1, p2, p3] = PRIMES;
    let (q1, q2, q3) = (vdupq_n_s32(p1.q), vdupq_n_s32(p2.q), vdupq_n_s32(p3.q));
    let g = GARNER;
    let (c12, c12q) = (vdupq_n_s32(g.c12), vdupq_n_s32(g.c12q));
    let (c13, c13q) = (vdupq_n_s32(g.c13), vdupq_n_s32(g.c13q));
    let (c23, c23q) = (vdupq_n_s32(g.c23), vdupq_n_s32(g.c23q));
    for i in (0..n).step_by(4) {
        // SAFETY: `i + 4 <= n` for all three slices.
        unsafe {
            let a = normalize(vld1q_s32(r1.as_ptr().add(i)), q1);
            let b = normalize(vld1q_s32(r2.as_ptr().add(i)), q2);
            let c = normalize(vld1q_s32(r3.as_ptr().add(i)), q3);
            let v2 = normalize(mont(vsubq_s32(b, a), c12, c12q, q2), q2);
            let t = mont(vsubq_s32(c, a), c13, c13q, q3);
            let v3 = normalize(mont(vsubq_s32(t, v2), c23, c23q, q3), q3);
            vst1q_s32(r1.as_mut_ptr().add(i), a);
            vst1q_s32(r2.as_mut_ptr().add(i), v2);
            vst1q_s32(r3.as_mut_ptr().add(i), v3);
        }
    }
}

#[target_feature(enable = "neon")]
fn garner_two_neon(r1: &mut [i32], r2: &mut [i32]) {
    let n = r1.len();
    assert!(n == r2.len() && n.is_multiple_of(4));
    let [p1, p2, _] = PRIMES;
    let (q1, q2) = (vdupq_n_s32(p1.q), vdupq_n_s32(p2.q));
    let (c12, c12q) = (vdupq_n_s32(GARNER.c12), vdupq_n_s32(GARNER.c12q));
    for i in (0..n).step_by(4) {
        // SAFETY: equal lengths divisible by four give `i + 4 <= n` in both slices.
        unsafe {
            let a = normalize(vld1q_s32(r1.as_ptr().add(i)), q1);
            let b = vld1q_s32(r2.as_ptr().add(i));
            // `-(q1 + q2) < b - a < q2`, and `q1 + q2 < 2^31`, so subtraction
            // fits i32. The balanced c12 lets mont take this difference directly:
            // its output has magnitude below 0.75 q2 and needs one normalization.
            let v2 = normalize(mont(vsubq_s32(b, a), c12, c12q, q2), q2);
            vst1q_s32(r1.as_mut_ptr().add(i), a);
            vst1q_s32(r2.as_mut_ptr().add(i), v2);
        }
    }
}

/// The 32-bit halves of limbs `i / 2` to `i / 2 + 1` as `c mod q` with magnitude below
/// `2^31` (`portable::lift`).
///
/// # Safety
///
/// `src` must have `i + 4` readable `u32`s.
#[inline]
#[target_feature(enable = "neon")]
unsafe fn lift4(src: *const u32, i: usize, r32: int32x4_t) -> int32x4_t {
    // SAFETY: the caller's contract.
    let x = vreinterpretq_s32_u32(unsafe { vld1q_u32(src.add(i)) });
    vaddq_s32(x, vandq_s32(vshrq_n_s32::<31>(x), r32))
}

#[target_feature(enable = "neon")]
fn load_twist_neon(dst: &mut [i32], limbs: &[u64], p: Prime, w: &[i32], wq: &[i32]) {
    let m = 2 * limbs.len();
    assert!(dst.len() >= m && w.len() >= m && wq.len() >= m);
    let full = m / 4 * 4;
    let (qv, r32) = (vdupq_n_s32(p.q), vdupq_n_s32(p.r32));
    let src = limbs.as_ptr().cast::<u32>();
    for i in (0..full).step_by(4) {
        // SAFETY: `i + 4 <= full <= m`, within the halves of `limbs`, `dst`, `w`, `wq`.
        unsafe {
            let (w, wq) = (vld1q_s32(w.as_ptr().add(i)), vld1q_s32(wq.as_ptr().add(i)));
            vst1q_s32(dst.as_mut_ptr().add(i), mont(lift4(src, i, r32), w, wq, qv));
        }
    }
    if full < m {
        let mut tail = [0; 2];
        portable::load_twist(
            &mut tail,
            &limbs[limbs.len() - 1..],
            p,
            &w[full..],
            &wq[full..],
        );
        dst[full..m].copy_from_slice(&tail);
    }
    dst[m..].fill(0);
}

#[target_feature(enable = "neon")]
fn load_add_neon(dst: &mut [i32], limbs: &[u64], p: Prime, sub: bool) {
    let m = 2 * limbs.len();
    assert!(dst.len() >= m);
    let full = m / 4 * 4;
    let (qv, vv, r32) = (vdupq_n_s32(p.q), vdupq_n_s32(p.v), vdupq_n_s32(p.r32));
    // `x * s` is `x` or `-x`.
    let s = vdupq_n_s32(if sub { -1 } else { 1 });
    let src = limbs.as_ptr().cast::<u32>();
    for i in (0..full).step_by(4) {
        // SAFETY: `i + 4 <= full <= m`, within the halves of `limbs` and `dst`.
        unsafe {
            let d = dst.as_mut_ptr().add(i);
            let x = vmulq_s32(reduce(lift4(src, i, r32), qv, vv), s);
            vst1q_s32(d, reduce(vaddq_s32(vld1q_s32(d), x), qv, vv));
        }
    }
    if full < m {
        portable::load_add(&mut dst[full..m], &limbs[limbs.len() - 1..], p, sub);
    }
}

#[target_feature(enable = "neon")]
fn twist_neon(a: &mut [i32], w: &[i32], wq: &[i32], p: Prime) {
    let n = a.len();
    assert!(n.is_multiple_of(4) && w.len() >= n && wq.len() >= n);
    let qv = vdupq_n_s32(p.q);
    for i in (0..n).step_by(4) {
        // SAFETY: `i + 4 <= n`, within `a`, `w` and `wq`.
        unsafe {
            let (w, wq) = (vld1q_s32(w.as_ptr().add(i)), vld1q_s32(wq.as_ptr().add(i)));
            let x = a.as_mut_ptr().add(i);
            vst1q_s32(x, mont(vld1q_s32(x), w, wq, qv));
        }
    }
}

#[target_feature(enable = "neon")]
fn fold_sub_neon(dst: &mut [i32], src: &[i32], start: usize, neg: bool, p: Prime) {
    let d = dst.len();
    assert!(d.is_multiple_of(4) && src.len().is_multiple_of(4) && start.is_multiple_of(4));
    let (qv, vv) = (vdupq_n_s32(p.q), vdupq_n_s32(p.v));
    let (mut pos, mut src) = (start, src);
    while !src.is_empty() {
        let at = pos % d;
        let (run, rest) = src.split_at((d - at).min(src.len()));
        let s = vdupq_n_s32(if neg && (pos / d) % 2 == 1 { -1 } else { 1 });
        let x = dst[at..at + run.len()].as_mut_ptr();
        for i in (0..run.len()).step_by(4) {
            // SAFETY: `i + 4 <= run.len()`, within `run` and `dst[at..]`: lengths are
            // multiples of 4.
            unsafe {
                let y = vmulq_s32(vld1q_s32(run.as_ptr().add(i)), s);
                vst1q_s32(x.add(i), reduce(vsubq_s32(vld1q_s32(x.add(i)), y), qv, vv));
            }
        }
        (pos, src) = (pos + run.len(), rest);
    }
}

#[target_feature(enable = "neon")]
fn halves_neon(lo: &mut [i32], hi: &mut [i32], p: Prime) {
    let n = lo.len();
    assert!(n == hi.len() && n.is_multiple_of(4));
    let qv = vdupq_n_s32(p.q);
    let (h, hq) = portable::half(p);
    let (h, hq) = (vdupq_n_s32(h), vdupq_n_s32(hq));
    for i in (0..n).step_by(4) {
        // SAFETY: `i + 4 <= n`, within `lo` and `hi`.
        unsafe {
            let (pl, ph) = (lo.as_mut_ptr().add(i), hi.as_mut_ptr().add(i));
            let (x, y) = (vld1q_s32(pl), vld1q_s32(ph));
            vst1q_s32(pl, mont(vaddq_s32(y, x), h, hq, qv));
            vst1q_s32(ph, mont(vsubq_s32(y, x), h, hq, qv));
        }
    }
}

#[target_feature(enable = "neon")]
fn pair_neon(r: &mut [i32], m: usize, iw: &[i32], iwq: &[i32], p: Prime) {
    let n = r.len() - m;
    assert!(n < m && n.is_multiple_of(4) && m.is_multiple_of(n));
    assert!(iw.len() >= m && iwq.len() >= m);
    let (qv, vv) = (vdupq_n_s32(p.q), vdupq_n_s32(p.v));
    let (r, w, wq) = (r.as_mut_ptr(), iw.as_ptr(), iwq.as_ptr());
    for b in (n..m).step_by(n) {
        for i in (0..n).step_by(4) {
            // SAFETY: `b + i + 4 <= m` and `m + i + 4 <= m + n`, within `r`, `iw`, `iwq`.
            unsafe {
                let px = r.add(b + i);
                let x = mont(
                    vld1q_s32(px),
                    vld1q_s32(w.add(b + i)),
                    vld1q_s32(wq.add(b + i)),
                    qv,
                );
                vst1q_s32(px, x);
                let ps = r.add(m + i);
                vst1q_s32(ps, reduce(vsubq_s32(vld1q_s32(ps), x), qv, vv));
            }
        }
    }
    let (h, hq) = portable::half(p);
    let (h, hq) = (vdupq_n_s32(h), vdupq_n_s32(hq));
    for i in (0..n).step_by(4) {
        // SAFETY: `i + 4 <= n` and `m + i + 4 <= m + n`, within `r`, `iw`, `iwq`.
        unsafe {
            let (plo, phi) = (r.add(i), r.add(m + i));
            let x = mont(
                vld1q_s32(plo),
                vld1q_s32(w.add(i)),
                vld1q_s32(wq.add(i)),
                qv,
            );
            let s = vld1q_s32(phi);
            vst1q_s32(plo, mont(vaddq_s32(s, x), h, hq, qv));
            vst1q_s32(phi, mont(vsubq_s32(s, x), h, hq, qv));
        }
    }
}

impl Kernels {
    pub fn neon() -> Self {
        Kernels {
            load: self::load,
            forward: self::forward,
            inverse: self::inverse,
            pointwise: self::pointwise,
            square: self::square,
            garner: self::garner,
            load_twist: self::load_twist,
            load_add: self::load_add,
            twist: self::twist,
            fold_sub: self::fold_sub,
            halves: self::halves,
            pair: self::pair,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ntt::tables::Tables;
    use crate::ntt::tables::tests::samples;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    /// `inverse_stage` checks its contract in every build, not only with debug assertions:
    /// here `a` is longer than the twiddles serve.
    #[test]
    #[should_panic(expected = "n <= t.iw.len()")]
    fn inverse_stage_checks_its_contract() {
        let t = Tables::new(3);
        // SAFETY: NEON is enabled for this build.
        unsafe { inverse_stage::<false>(&mut [0; 16], 4, PRIMES[0], &t.tw[0]) }
    }

    #[test]
    fn garner_two_matches_scalar() {
        let [p1, p2, _] = PRIMES;
        let edges = |q: i32| [1 - q, -q / 2 - 1, -q / 2, -1, 0, 1, q / 2, q - 1];
        let (mut r1, mut r2) = (Vec::new(), Vec::new());
        for a in edges(p1.q) {
            for b in edges(p2.q) {
                r1.push(a);
                r2.push(b);
            }
        }
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(5806);
        r1.extend_from_slice(&samples(&mut rng, p1.q, 4096)[..4096]);
        r2.extend_from_slice(&samples(&mut rng, p2.q, 4096)[..4096]);
        let (mut want1, mut want2) = (r1.clone(), r2.clone());
        for (a, b) in want1.iter_mut().zip(&mut want2) {
            *a = p1.normalize(*a);
            // Compare with the original scalar expression, including its reduction.
            *b = p2.normalize(p2.mont(p2.reduce(*b - *a), GARNER.c12, GARNER.c12q));
        }
        for offset in 0..4 {
            let n = r1.len();
            let (mut a, mut b) = (vec![i32::MAX; n + 8], vec![i32::MAX; n + 8]);
            a[offset..offset + n].copy_from_slice(&r1);
            b[offset..offset + n].copy_from_slice(&r2);
            garner_two(&mut a[offset..offset + n], &mut b[offset..offset + n]);
            assert_eq!(a[offset..offset + n], want1, "r1, offset {offset}");
            assert_eq!(b[offset..offset + n], want2, "v2, offset {offset}");
            for x in [&a, &b] {
                assert!(
                    x[..offset]
                        .iter()
                        .chain(&x[offset + n..])
                        .all(|&v| v == i32::MAX)
                );
            }
        }
        garner_two(&mut [], &mut []);
    }

    #[test]
    fn garner_two_reconstructs_coefficients() {
        let [p1, p2, _] = PRIMES;
        let modulus = p1.q as u64 * p2.q as u64;
        let mut coefficients = vec![
            0,
            1,
            p1.q as u64,
            p2.q as u64,
            modulus / 2,
            modulus - 2,
            modulus - 1,
            0,
        ];
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(5807);
        coefficients.extend((0..4096).map(|_| rng.next_u64() % modulus));
        let residues = |p: Prime, parity: usize| -> Vec<i32> {
            coefficients
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    let r = (c % p.q as u64) as i32;
                    if r != 0 && i % 2 == parity {
                        r - p.q
                    } else {
                        r
                    }
                })
                .collect()
        };
        let (mut r1, mut v2) = (residues(p1, 0), residues(p2, 1));
        garner_two(&mut r1, &mut v2);
        for ((&a, &b), &coefficient) in r1.iter().zip(&v2).zip(&coefficients) {
            assert!((0..p1.q).contains(&a));
            assert!((0..p2.q).contains(&b));
            assert_eq!(a as u64 + p1.q as u64 * b as u64, coefficient);
        }
    }

    #[test]
    #[should_panic(expected = "n == r2.len() && n.is_multiple_of(4)")]
    fn garner_two_rejects_unequal_lengths() {
        garner_two(&mut [0; 4], &mut [0; 8]);
    }

    #[test]
    #[should_panic(expected = "n == r2.len() && n.is_multiple_of(4)")]
    fn garner_two_rejects_partial_vectors() {
        garner_two(&mut [0; 3], &mut [0; 3]);
    }

    /// Each kernel gives exactly what the portable one gives.
    #[test]
    fn matches_portable() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(40);
        let tables = Tables::new(16);
        for (p, tw) in PRIMES.into_iter().zip(&tables.tw) {
            for log in 3..=16 {
                let n = 1 << log;
                let x = samples(&mut rng, p.q, n)[..n].to_vec();
                let y = samples(&mut rng, p.q, n)[..n].to_vec();
                let (mut a, mut b) = (x.clone(), x.clone());
                forward(&mut a, p, tw);
                portable::forward(&mut b, p, tw);
                assert_eq!(a, b, "forward, n {n}");
                let (mut a, mut b) = (x.clone(), x.clone());
                inverse(&mut a, p, tw);
                portable::inverse(&mut b, p, tw);
                assert_eq!(a, b, "inverse, n {n}");
                let (mut a, mut b) = (x.clone(), x.clone());
                pointwise(&mut a, &y, p);
                portable::pointwise(&mut b, &y, p);
                assert_eq!(a, b, "pointwise, n {n}");
                let (mut a, mut b) = (x.clone(), x.clone());
                square(&mut a, p);
                portable::square(&mut b, p);
                assert_eq!(a, b, "square, n {n}");
            }
            for len in 0..=9 {
                let limbs: Vec<u64> = (0..len).map(|_| rng.next_u64()).chain([u64::MAX]).collect();
                let (mut a, mut b) = (vec![7; 2 * limbs.len() + 5], vec![9; 2 * limbs.len() + 5]);
                load(&mut a, &limbs, p);
                portable::load(&mut b, &limbs, p);
                assert_eq!(a, b, "load, {} limbs", limbs.len());
                let (w, wq) = (&tw.fw[32..64], &tw.fwq[32..64]);
                load_twist(&mut a, &limbs, p, w, wq);
                portable::load_twist(&mut b, &limbs, p, w, wq);
                assert_eq!(a, b, "load_twist, {} limbs", limbs.len());
                let d = samples(&mut rng, p.q, a.len())[..a.len()].to_vec();
                for sub in [false, true] {
                    let (mut a, mut b) = (d.clone(), d.clone());
                    load_add(&mut a, &limbs, p, sub);
                    portable::load_add(&mut b, &limbs, p, sub);
                    assert_eq!(a, b, "load_add, {} limbs, sub {sub}", limbs.len());
                }
            }
            let n = 4096;
            let x = samples(&mut rng, p.q, n)[..n].to_vec();
            let y = samples(&mut rng, p.q, n)[..n].to_vec();
            let (mut a, mut b) = (x.clone(), x.clone());
            twist(&mut a, &tw.iw[n..], &tw.iwq[n..], p);
            portable::twist(&mut b, &tw.iw[n..], &tw.iwq[n..], p);
            assert_eq!(a, b, "twist");
            let ((mut a, mut c), (mut b, mut d)) = ((x.clone(), y.clone()), (x.clone(), y.clone()));
            halves(&mut a, &mut c, p);
            portable::halves(&mut b, &mut d, p);
            assert_eq!((a, c), (b, d), "halves");
            for (m, n) in [(8, 4), (64, 8), (1024, 256), (2048, 8)] {
                let (mut a, mut b) = (x[..m + n].to_vec(), x[..m + n].to_vec());
                let (w, wq) = (&tw.iw[m..2 * m], &tw.iwq[m..2 * m]);
                pair(&mut a, m, w, wq, p);
                portable::pair(&mut b, m, w, wq, p);
                assert_eq!(a, b, "pair, m {m}, n {n}");
            }
            for (d, start, len, neg) in [
                (8, 8, 8, true),
                (64, 72, 2000, true),
                (256, 96, 3000, false),
            ] {
                let (mut a, mut b) = (y[..d].to_vec(), y[..d].to_vec());
                fold_sub(&mut a, &x[..len], start, neg, p);
                portable::fold_sub(&mut b, &x[..len], start, neg, p);
                assert_eq!(a, b, "fold_sub, d {d}, start {start}");
            }
        }
        let n = 4096;
        let r: [Vec<i32>; 3] =
            core::array::from_fn(|i| samples(&mut rng, PRIMES[i].q, n)[..n].to_vec());
        let (mut a, mut b) = (r.clone(), r);
        let [a1, a2, a3] = &mut a;
        garner([a1, a2, a3]);
        let [b1, b2, b3] = &mut b;
        portable::garner([b1, b2, b3]);
        assert_eq!(a, b, "garner");
    }
}

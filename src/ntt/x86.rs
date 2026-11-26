//! Portable NTT kernels compiled for detected x86 target features.

use super::portable::{
    fold_sub, forward, garner, halves, inverse, load, load_add, load_twist, pair, pointwise,
    square, twist,
};
use super::tables::Prime;

/// The auxiliary kernels compiled for `$feature`, paired with caller-supplied transforms
/// and pointwise products.
macro_rules! aux_kernels {
    ($feature:literal, $forward:expr, $inverse:expr, $pointwise:expr, $square:expr) => {{
        #[target_feature(enable = $feature)]
        fn tf_load(dst: &mut [i32], limbs: &[u64], p: Prime) {
            load(dst, limbs, p)
        }
        #[target_feature(enable = $feature)]
        fn tf_garner(r: [&mut [i32]; 3]) {
            garner(r)
        }
        #[target_feature(enable = $feature)]
        fn tf_load_twist(dst: &mut [i32], limbs: &[u64], p: Prime, w: &[i32]) {
            load_twist(dst, limbs, p, w)
        }
        #[target_feature(enable = $feature)]
        fn tf_load_add(dst: &mut [i32], limbs: &[u64], p: Prime, sub: bool) {
            load_add(dst, limbs, p, sub)
        }
        #[target_feature(enable = $feature)]
        fn tf_twist(a: &mut [i32], w: &[i32], p: Prime) {
            twist(a, w, p)
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
        fn tf_pair(r: &mut [i32], m: usize, iw: &[i32], p: Prime) {
            pair(r, m, iw, p)
        }
        // The closures adapt these functions to the table's safe pointers. A table of
        // `unsafe fn` pointers without them measured neutral (Zen 5, aligned builds,
        // within 2%): a kernel call does thousands of coefficients.
        super::Kernels {
            // SAFETY: built only after detecting $feature.
            load: |d, l, p| unsafe { tf_load(d, l, p) },
            forward: $forward,
            inverse: $inverse,
            pointwise: $pointwise,
            square: $square,
            // SAFETY: built only after detecting $feature.
            garner: |r| unsafe { tf_garner(r) },
            // SAFETY: built only after detecting $feature.
            load_twist: |d, l, p, w| unsafe { tf_load_twist(d, l, p, w) },
            // SAFETY: built only after detecting $feature.
            load_add: |d, l, p, sub| unsafe { tf_load_add(d, l, p, sub) },
            // SAFETY: built only after detecting $feature.
            twist: |a, w, p| unsafe { tf_twist(a, w, p) },
            // SAFETY: built only after detecting $feature.
            fold_sub: |d, s, at, neg, p| unsafe { tf_fold_sub(d, s, at, neg, p) },
            // SAFETY: built only after detecting $feature.
            halves: |lo, hi, p| unsafe { tf_halves(lo, hi, p) },
            // SAFETY: built only after detecting $feature.
            pair: |r, m, w, p| unsafe { tf_pair(r, m, w, p) },
        }
    }};
}

/// Auxiliary kernels compiled for AVX-512F, combined with `avx512.rs`'s hand-written
/// transforms so auto-vectorized AVX-512F transforms are not emitted as dead code.
///
/// # Safety
///
/// The CPU must support AVX-512F.
pub(super) unsafe fn avx512_with(
    forward: fn(&mut [i32], Prime, &[i32]),
    inverse: fn(&mut [i32], Prime, &[i32]),
    pointwise: fn(&mut [i32], &[i32], Prime),
    square: fn(&mut [i32], Prime),
) -> super::Kernels {
    aux_kernels!("avx512f", forward, inverse, pointwise, square)
}

/// All portable kernels compiled for AVX2, when this CPU has AVX2.
pub(super) fn kernels() -> Option<super::Kernels> {
    if !crate::cpu::avx2() {
        return None;
    }
    #[target_feature(enable = "avx2")]
    fn tf_forward(a: &mut [i32], p: Prime, fw: &[i32]) {
        forward(a, p, fw)
    }
    #[target_feature(enable = "avx2")]
    fn tf_inverse(a: &mut [i32], p: Prime, iw: &[i32]) {
        inverse(a, p, iw)
    }
    #[target_feature(enable = "avx2")]
    fn tf_pointwise(a: &mut [i32], b: &[i32], p: Prime) {
        pointwise(a, b, p)
    }
    #[target_feature(enable = "avx2")]
    fn tf_square(a: &mut [i32], p: Prime) {
        square(a, p)
    }
    Some(aux_kernels!(
        "avx2",
        // SAFETY: the AVX2 check above succeeded.
        |a, p, t| unsafe { tf_forward(a, p, t) },
        // SAFETY: the AVX2 check above succeeded.
        |a, p, iw| unsafe { tf_inverse(a, p, iw) },
        // SAFETY: the AVX2 check above succeeded.
        |a, b, p| unsafe { tf_pointwise(a, b, p) },
        // SAFETY: the AVX2 check above succeeded.
        |a, p| unsafe { tf_square(a, p) }
    ))
}

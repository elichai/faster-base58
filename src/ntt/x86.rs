//! Portable NTT kernels compiled for detected x86 target features.

use super::tables::{Prime, Twiddles};
use super::portable::{load, forward, inverse, pointwise, square, garner,
    load_twist, load_add, twist, fold_sub, halves, pair};

/// These kernels compiled for AVX-512 or AVX2, when this CPU has either.
pub(super) fn kernels() -> Option<super::Kernels> {
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

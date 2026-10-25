//! AVX-512F/IFMA implementation of the shared radix-52 kernel.

use super::{Kernel, V8, kernel};
use core::arch::x86_64::*;

/// Whether this CPU has AVX-512F and AVX-512 IFMA.
pub fn available() -> bool {
    is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512ifma")
}

/// `out = a * b`. Requires [`available`], `8 <= a.len() <= b.len()`,
/// `a.len() <= MAX_LEN`, `out.len() == a.len() + b.len()` and
/// `scratch.len() >= scratch_len(a.len(), b.len())`. `scratch` may hold anything.
pub fn mul(out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]) {
    assert!(available(), "no AVX-512 IFMA");
    // SAFETY: the CPU has the features.
    unsafe { mul_tf(out, a, b, scratch) }
}

#[target_feature(enable = "avx512f,avx512ifma")]
fn mul_tf(out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]) {
    // SAFETY: the CPU has the features.
    unsafe { super::mul_v::<Z>(out, a, b, scratch) }
}

#[target_feature(enable = "avx512f,avx512ifma")]
unsafe fn kernel_tf<const NW: usize>(c: *mut u64, a: *const u64, groups: usize, b: *const u64) {
    // SAFETY: the caller's.
    unsafe { kernel::<Z, NW>(c, a, groups, b) }
}

/// `finish` on AVX-512, for tests. Safety: [`available`].
#[cfg(test)]
#[target_feature(enable = "avx512f,avx512ifma")]
pub(super) unsafe fn finish_tf(out: &mut [u64], c: &mut [u64]) {
    // SAFETY: the caller's.
    unsafe { super::finish::<Z>(out, c) }
}

fn mask(n: usize) -> __mmask8 {
    ((1u32 << n.min(8)) - 1) as u8
}

#[derive(Clone, Copy)]
struct Z(__m512i);

// SAFETY (each block): a `Z` exists, or the caller promises one may, so the CPU has
// AVX-512F and IFMA; pointer contracts are the caller's.
impl V8 for Z {
    #[inline(always)]
    unsafe fn splat(x: u64) -> Self {
        Z(unsafe { _mm512_set1_epi64(x as i64) })
    }
    #[inline(always)]
    unsafe fn from_array(x: [u64; 8]) -> Self {
        Z(unsafe { core::mem::transmute::<[u64; 8], __m512i>(x) })
    }
    #[inline(always)]
    unsafe fn load(p: *const u64) -> Self {
        Z(unsafe { _mm512_loadu_epi64(p.cast()) })
    }
    #[inline(always)]
    unsafe fn load_n(p: *const u64, n: usize) -> Self {
        Z(unsafe { _mm512_maskz_loadu_epi64(mask(n), p.cast()) })
    }
    #[inline(always)]
    unsafe fn store(self, p: *mut u64) {
        unsafe { _mm512_storeu_epi64(p.cast(), self.0) }
    }
    #[inline(always)]
    unsafe fn store_n(self, p: *mut u64, n: usize) {
        unsafe { _mm512_mask_storeu_epi64(p.cast(), mask(n), self.0) }
    }
    #[inline(always)]
    fn add(self, b: Self) -> Self {
        Z(unsafe { _mm512_add_epi64(self.0, b.0) })
    }
    #[inline(always)]
    fn and(self, b: Self) -> Self {
        Z(unsafe { _mm512_and_si512(self.0, b.0) })
    }
    #[inline(always)]
    fn or(self, b: Self) -> Self {
        Z(unsafe { _mm512_or_si512(self.0, b.0) })
    }
    #[inline(always)]
    fn shr52(self) -> Self {
        Z(unsafe { _mm512_srli_epi64::<52>(self.0) })
    }
    #[inline(always)]
    fn shrv(self, s: Self) -> Self {
        Z(unsafe { _mm512_srlv_epi64(self.0, s.0) })
    }
    #[inline(always)]
    fn shlv(self, s: Self) -> Self {
        Z(unsafe { _mm512_sllv_epi64(self.0, s.0) })
    }
    #[inline(always)]
    fn up(self, prev: Self) -> Self {
        Z(unsafe { _mm512_alignr_epi64::<7>(self.0, prev.0) })
    }
    #[inline(always)]
    fn permute(self, idx: Self) -> Self {
        Z(unsafe { _mm512_permutexvar_epi64(idx.0, self.0) })
    }
    #[inline(always)]
    fn permute2(self, idx: Self, hi: Self) -> Self {
        Z(unsafe { _mm512_permutex2var_epi64(self.0, idx.0, hi.0) })
    }
    #[inline(always)]
    fn madd52lo(self, x: Self, y: Self) -> Self {
        Z(unsafe { _mm512_madd52lo_epu64(self.0, x.0, y.0) })
    }
    #[inline(always)]
    fn madd52hi(self, x: Self, y: Self) -> Self {
        Z(unsafe { _mm512_madd52hi_epu64(self.0, x.0, y.0) })
    }
    #[inline(always)]
    fn bit52(self) -> bool {
        unsafe { _mm512_test_epi64_mask(self.0, _mm512_set1_epi64(1 << 52)) != 0 }
    }
    fn kernel(nw: usize) -> Kernel {
        pick!(kernel_tf, nw)
    }
}

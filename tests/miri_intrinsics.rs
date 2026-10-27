//! Isolated support probes. Run each test separately: an unsupported intrinsic stops Miri.
#![cfg(miri)]

#[cfg(target_arch = "aarch64")]
mod arm {
    use core::arch::aarch64::*;

    #[test]
    fn halving_subtract32() {
        let a = [i32::MIN, i32::MAX, 0, -3];
        let b = [i32::MAX, i32::MIN, 1, 0];
        let result: [i32; 4] = unsafe {
            core::mem::transmute(vhsubq_s32(core::mem::transmute(a), core::mem::transmute(b)))
        };
        assert_eq!(result, [i32::MIN, i32::MAX, -1, -2]);
    }

    #[test]
    fn rounded_shift32() {
        let result: [i32; 4] =
            unsafe { core::mem::transmute(vrshrq_n_s32::<26>(vdupq_n_s32(1 << 25))) };
        assert_eq!(result, [1; 4]);
    }

    #[test]
    fn shift_insert32() {
        let result: [u32; 4] =
            unsafe { core::mem::transmute(vsliq_n_u32::<16>(vdupq_n_u32(1), vdupq_n_u32(2))) };
        assert_eq!(result, [0x20001; 4]);
    }

    #[test]
    fn shift_insert16() {
        let result: [u16; 8] =
            unsafe { core::mem::transmute(vsliq_n_u16::<8>(vdupq_n_u16(1), vdupq_n_u16(2))) };
        assert_eq!(result, [0x201; 8]);
    }

    #[test]
    fn table_extend1() {
        let result: [u8; 16] = unsafe {
            core::mem::transmute(vqtbx1q_u8(vdupq_n_u8(1), vdupq_n_u8(2), vdupq_n_u8(0)))
        };
        assert_eq!(result, [2; 16]);
    }

    #[test]
    fn table_extend4() {
        let result: [u8; 16] = unsafe {
            let table = uint8x16x4_t(vdupq_n_u8(2), vdupq_n_u8(3), vdupq_n_u8(4), vdupq_n_u8(5));
            core::mem::transmute(vqtbx4q_u8(vdupq_n_u8(1), table, vdupq_n_u8(0)))
        };
        assert_eq!(result, [2; 16]);
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
mod x86 {
    use core::arch::x86_64::*;

    #[test]
    fn variable_right64() {
        let result: [u64; 8] = unsafe {
            core::mem::transmute(_mm512_srlv_epi64(
                _mm512_set1_epi64(8),
                _mm512_set1_epi64(1),
            ))
        };
        assert_eq!(result, [4; 8]);
    }

    #[test]
    fn variable_left64() {
        let result: [u64; 8] = unsafe {
            core::mem::transmute(_mm512_sllv_epi64(
                _mm512_set1_epi64(8),
                _mm512_set1_epi64(1),
            ))
        };
        assert_eq!(result, [16; 8]);
    }

    #[cfg(target_feature = "avx512ifma")]
    #[test]
    fn multiply_low52() {
        let result: [u64; 8] = unsafe {
            core::mem::transmute(_mm512_madd52lo_epu64(
                _mm512_set1_epi64(1),
                _mm512_set1_epi64(2),
                _mm512_set1_epi64(3),
            ))
        };
        assert_eq!(result, [7; 8]);
    }

    #[cfg(target_feature = "avx512ifma")]
    #[test]
    fn multiply_high52() {
        let result: [u64; 8] = unsafe {
            core::mem::transmute(_mm512_madd52hi_epu64(
                _mm512_set1_epi64(1),
                _mm512_set1_epi64(1 << 51),
                _mm512_set1_epi64(2),
            ))
        };
        assert_eq!(result, [2; 8]);
    }

    #[test]
    fn permute2_d512() {
        let result: [u32; 16] = unsafe {
            core::mem::transmute(_mm512_permutex2var_epi32(
                _mm512_set1_epi32(2),
                _mm512_set1_epi32(0),
                _mm512_set1_epi32(3),
            ))
        };
        assert_eq!(result, [2; 16]);
    }

    #[test]
    fn permute_q512() {
        let result: [u64; 8] = unsafe {
            core::mem::transmute(_mm512_permutexvar_epi64(
                _mm512_set1_epi64(0),
                _mm512_set1_epi64(2),
            ))
        };
        assert_eq!(result, [2; 8]);
    }

    #[test]
    fn permute2_q512() {
        let result: [u64; 8] = unsafe {
            core::mem::transmute(_mm512_permutex2var_epi64(
                _mm512_set1_epi64(2),
                _mm512_set1_epi64(0),
                _mm512_set1_epi64(3),
            ))
        };
        assert_eq!(result, [2; 8]);
    }

    #[cfg(target_feature = "avx512vbmi")]
    #[test]
    fn permute2_b256() {
        let result: [u8; 32] = unsafe {
            core::mem::transmute(_mm256_permutex2var_epi8(
                _mm256_set1_epi8(2),
                _mm256_set1_epi8(0),
                _mm256_set1_epi8(3),
            ))
        };
        assert_eq!(result, [2; 32]);
    }

    #[cfg(target_feature = "avx512vbmi")]
    #[test]
    fn permute_b256() {
        let result: [u8; 32] = unsafe {
            core::mem::transmute(_mm256_permutexvar_epi8(
                _mm256_set1_epi8(0),
                _mm256_set1_epi8(2),
            ))
        };
        assert_eq!(result, [2; 32]);
    }
}

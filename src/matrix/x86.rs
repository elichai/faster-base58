//! x86_64 panel kernels: AVX2 or SSE4.1, picked at run time. Parsing stays portable.
//! Same contract as `portable::panel`.

use core::arch::x86_64::*;

use super::portable;

/// See `portable::panel`.
pub(super) fn panel(acc: &mut [u64], words: &[u32], d: &[u32; 8]) {
    assert!(acc.len().is_multiple_of(4) && words.len() == 8 * acc.len());
    if crate::cpu::avx2() {
        // SAFETY: the CPU has AVX2.
        unsafe { avx2(acc, words, d) }
    } else if crate::cpu::sse41() {
        // SAFETY: the CPU has SSE4.1.
        unsafe { sse41(acc, words, d) }
    } else {
        portable::panel(acc, words, d)
    }
}

/// [`panel`] with AVX2, without its length check.
#[target_feature(enable = "avx2")]
fn avx2(acc: &mut [u64], words: &[u32], d: &[u32; 8]) {
    for (a, t) in acc
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(words.as_chunks::<32>().0)
    {
        let a = a.as_mut_ptr().cast::<__m256i>();
        let t = t.as_ptr().cast::<__m128i>();
        // SAFETY: `a` points to 32 bytes and `t` to 8 rows of 16 bytes; all accesses are
        // unaligned.
        unsafe {
            let mut s = _mm256_loadu_si256(a);
            for (r, &dr) in d.iter().enumerate() {
                // 4 words zero-extended to u64 lanes, times the group (`vpmuludq` uses the
                // low 32 bits of each lane).
                let w = _mm256_cvtepu32_epi64(_mm_loadu_si128(t.add(r)));
                s = _mm256_add_epi64(s, _mm256_mul_epu32(w, _mm256_set1_epi64x(dr as i64)));
            }
            _mm256_storeu_si256(a, s);
        }
    }
}

/// [`panel`] with SSE4.1, without its length check.
#[target_feature(enable = "sse4.1")]
fn sse41(acc: &mut [u64], words: &[u32], d: &[u32; 8]) {
    for (a, t) in acc
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(words.as_chunks::<32>().0)
    {
        let a = a.as_mut_ptr().cast::<__m128i>();
        let t = t.as_ptr().cast::<__m128i>();
        // SAFETY: as in `avx2`.
        unsafe {
            let mut a0 = _mm_loadu_si128(a);
            let mut a1 = _mm_loadu_si128(a.add(1));
            for (r, &dr) in d.iter().enumerate() {
                let w = _mm_loadu_si128(t.add(r));
                let dr = _mm_set1_epi64x(dr as i64);
                a0 = _mm_add_epi64(a0, _mm_mul_epu32(_mm_cvtepu32_epi64(w), dr));
                let hi = _mm_cvtepu32_epi64(_mm_srli_si128::<8>(w));
                a1 = _mm_add_epi64(a1, _mm_mul_epu32(hi, dr));
            }
            _mm_storeu_si128(a, a0);
            _mm_storeu_si128(a.add(1), a1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    /// Each kernel this CPU has against the portable one.
    #[test]
    fn kernels_match_portable() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(9);
        for nb in [0, 1, 2, 7, 50] {
            let acc: Vec<u64> = (0..4 * nb).map(|_| rng.next_u64() >> 24).collect();
            let words: Vec<u32> = (0..32 * nb).map(|_| rng.next_u32()).collect();
            let d: [u32; 8] = core::array::from_fn(|_| rng.next_u32() % 58u32.pow(4));
            let mut want = acc.clone();
            portable::panel(&mut want, &words, &d);
            let mut got = acc.clone();
            panel(&mut got, &words, &d);
            assert_eq!(got, want, "dispatch, nb {nb}");
            if crate::cpu::sse41() {
                let mut got = acc.clone();
                // SAFETY: the CPU has SSE4.1.
                unsafe { sse41(&mut got, &words, &d) };
                assert_eq!(got, want, "sse4.1, nb {nb}");
            }
            if crate::cpu::avx2() {
                let mut got = acc.clone();
                // SAFETY: the CPU has AVX2.
                unsafe { avx2(&mut got, &words, &d) };
                assert_eq!(got, want, "avx2, nb {nb}");
            } else {
                eprintln!("no AVX2 on this CPU: AVX2 kernel not run");
            }
        }
    }
}

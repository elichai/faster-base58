//! `x86_64` kernels, picked at run time: AVX2 parsing, AVX2 or SSE4.1 panels. Same
//! contracts as `portable.rs`.

use core::arch::x86_64::*;

use super::portable;

/// Low-nibble classes for [`parse_avx2`] (Muła and Lemire's nibble lookups). Bits 0-4
/// mark the low nibbles that are invalid after high nibble 3, 4, 5 or 7, 6, and any
/// other; bits 5 and 6 those whose digit is one lower again after high nibble 4 (past
/// 'I') and 6 (past 'l').
const LO: [u8; 16] = [
    0x1B, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x12, 0x31, 0x35, 0x3D, 0x75, 0x75, 0x77,
];
/// High-nibble classes, matching [`LO`].
const HI: [u8; 16] = [
    0x10, 0x10, 0x10, 0x01, 0x22, 0x04, 0x48, 0x04, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10,
];
/// Byte minus digit, by high nibble, before the corrections in [`LO`].
const OFF: [u8; 16] = [0, 0, 0, 49, 56, 58, 64, 65, 0, 0, 0, 0, 0, 0, 0, 0];

/// See `portable::parse`. With AVX2 it writes all `8 * digits.len().div_ceil(32)` groups.
pub(super) fn parse(digits: &[u8], groups: &mut [u32]) -> bool {
    assert!(groups.len() >= 8 * digits.len().div_ceil(32));
    if crate::cpu::avx2() {
        // SAFETY: the CPU has AVX2.
        unsafe { parse_avx2(digits, groups) }
    } else {
        portable::parse(digits, groups)
    }
}

/// [`parse`] with AVX2, without its length check.
#[target_feature(enable = "avx2")]
fn parse_avx2(digits: &[u8], groups: &mut [u32]) -> bool {
    let n = digits.len();
    let (full, rem) = (n / 32, n % 32);
    // SAFETY: each table has 16 bytes.
    let table =
        |t: &[u8; 16]| unsafe { _mm256_broadcastsi128_si256(_mm_loadu_si128(t.as_ptr().cast())) };
    let (lo_classes, hi_classes, offsets) = (table(&LO), table(&HI), table(&OFF));
    let nibble = _mm256_set1_epi8(0x0F);
    let mut bad = _mm256_setzero_si256();
    let out = groups.as_mut_ptr();
    let mut block = |v: __m256i, k: usize| {
        // Digits: the byte minus its high nibble's offset, minus one more past a gap.
        // Digits of invalid bytes are unspecified; their class bits 0-4 are nonzero.
        let hi = _mm256_and_si256(_mm256_srli_epi16::<4>(v), nibble);
        let m = _mm256_and_si256(
            _mm256_shuffle_epi8(lo_classes, _mm256_and_si256(v, nibble)),
            _mm256_shuffle_epi8(hi_classes, hi),
        );
        bad = _mm256_or_si256(bad, m);
        let one_more = _mm256_min_epu8(
            _mm256_and_si256(m, _mm256_set1_epi8(0x60)),
            _mm256_set1_epi8(1),
        );
        let x = _mm256_sub_epi8(
            _mm256_sub_epi8(v, _mm256_shuffle_epi8(offsets, hi)),
            one_more,
        );
        // Pairs `58 x0 + x1`, then groups `3364 p0 + p1`, most significant first. The
        // permute reverses them: like `portable::parse`, the least significant group
        // comes first in memory.
        let p = _mm256_maddubs_epi16(x, _mm256_set1_epi16(0x013A));
        let g = _mm256_madd_epi16(p, _mm256_set1_epi32(0x0001_0D24));
        let g = _mm256_permutevar8x32_epi32(g, _mm256_setr_epi32(7, 6, 5, 4, 3, 2, 1, 0));
        // SAFETY: block `k < n.div_ceil(32)` stores `groups[8 k..8 k + 8]`, in bounds by
        // `parse`'s assert.
        unsafe { _mm256_storeu_si256(out.add(8 * k).cast(), g) };
    };
    for k in 0..full {
        // SAFETY: block `k < full` loads `digits[n - 32 (k + 1)..n - 32 k]`.
        block(
            unsafe { _mm256_loadu_si256(digits.as_ptr().add(n - 32 * (k + 1)).cast()) },
            k,
        );
    }
    if rem != 0 {
        // '1' is digit 0, so padding on the left keeps the value.
        let mut head = [b'1'; 32];
        head[32 - rem..].copy_from_slice(&digits[..rem]);
        // SAFETY: `head` has 32 bytes.
        block(unsafe { _mm256_loadu_si256(head.as_ptr().cast()) }, full);
    }
    _mm256_testz_si256(bad, _mm256_set1_epi8(0x1F)) != 0
}

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
    use crate::common::DIGITS;
    use crate::tests::random_string;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    /// `parse` against the portable kernel: the flag, and the groups when valid.
    fn check_parse(s: &[u8]) -> bool {
        let len = 8 * s.len().div_ceil(32);
        let (mut a, mut b) = (vec![0; len], vec![0; len]);
        let ok = portable::parse(s, &mut b);
        assert_eq!(parse(s, &mut a), ok, "{s:?}");
        if ok {
            assert_eq!(a, b, "{s:?}");
        }
        ok
    }

    #[test]
    fn parse_matches_portable() {
        if !crate::cpu::avx2() {
            eprintln!("no AVX2 on this CPU: AVX2 parse not run");
        }
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(10);
        let lens: &[usize] = if cfg!(miri) {
            &[1, 31, 32, 33, 65]
        } else {
            &[255, 256, 257, 1000, 5632]
        };
        for n in (0..=if cfg!(miri) { 0 } else { 100 }).chain(lens.iter().copied()) {
            let valid = random_string(&mut rng, n);
            assert!(check_parse(valid.as_bytes()));
            assert!(check_parse("z".repeat(n).as_bytes()));
            if n > 0 {
                let pos = rng.next_u32() as usize % n;
                assert!(!check_parse(
                    format!("{}0{}", &valid[..pos], &valid[pos + 1..]).as_bytes()
                ));
            }
        }
    }

    /// Every byte value at every position of a full block and a padded head.
    #[test]
    fn parse_flags_every_byte() {
        for n in [31, 32, 33, 64] {
            for pos in 0..n {
                for c in 0..=255u8 {
                    let mut s = vec![b'z'; n];
                    s[pos] = c;
                    assert_eq!(check_parse(&s), DIGITS[c as usize] < 58, "{c} at {pos}");
                }
            }
        }
    }

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

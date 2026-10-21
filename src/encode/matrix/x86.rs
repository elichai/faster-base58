//! AVX2 kernels for the fixed size of 64 bytes, which `fixed.rs` picks at run time:
//! column sums, the carry in four segments (`fixed::carry_split`), and digits that stay in
//! registers until three stores write the result. README, "Fixed sizes", has the numbers.

use core::arch::x86_64::*;

use super::fixed::{self, TABLE_64, Table, words};
use crate::BufferTooSmall;
use crate::encode::max_encoded_len;

/// Columns that hold the value at 64 bytes: 88 characters.
const COLS_64: usize = max_encoded_len(64).div_ceil(4);

/// [`encode_matrix`](super::encode_matrix) for 64 bytes.
#[target_feature(enable = "avx2")]
pub(super) fn encode_64(x: &[u8; 64], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    let t = cols(x, &TABLE_64);
    // Vectors, not the array of carried columns: LLVM keeps an array that this path and
    // the fallback both write on the stack, and its vector loads from there do not get the
    // columns forwarded from the stores.
    let (c, zeros) = match fixed::carry_split::<24, 4>(&t, COLS_64) {
        Some(r) => (lanes(&r), zero_digits(&r)),
        None => carry_cold(&t),
    };
    // The 96 digits, top first: columns 23 to 16, 15 to 8, 7 to 0.
    let d = [group(c[0], c[1]), group(c[2], c[3]), group(c[4], c[5])];
    // '1' is the zero digit: drop the leading ones, then add one per leading zero byte.
    let n = fixed::leading_zeros(x) + 96 - zeros;
    let out = output.get_mut(..n).ok_or(BufferTooSmall)?;
    // `64 <= n <= 88` (`fixed::Table::check`), so the stores below are in `out` and the
    // last ends at `n`. The second overwrites the first one's excess.
    let skip = 96 - n;
    let [c0, c1, c2] = d.map(|d| alphabet(d));
    store(out.first_chunk_mut().unwrap(), shift(c0, skip));
    store(out[32 - skip..].first_chunk_mut().unwrap(), c1);
    store(out[64 - skip..].first_chunk_mut().unwrap(), c2);
    Ok(n)
}

/// [`encode_64`]'s columns where `fixed::carry_split` falls back.
#[cold]
#[inline(never)]
#[target_feature(enable = "avx2")]
fn carry_cold(t: &[u64; 24]) -> ([__m256i; 6], usize) {
    let r = fixed::carry_serial(t, COLS_64);
    (lanes(&r), zero_digits(&r))
}

/// The leading zero digits of the 96 digits of the carried columns `r`. Counted here and
/// not from the digit vectors (movemasks): with native flags on Zen 5 that took 66 ns
/// against 53 (README).
#[inline(always)]
fn zero_digits(r: &[u32; 24]) -> usize {
    let Some(j) = r.iter().rposition(|&c| c != 0) else {
        return 96;
    };
    let c = r[j];
    let digits = 1 + (c >= 58) as usize + (c >= 58 * 58) as usize + (c >= 58 * 58 * 58) as usize;
    4 * (24 - j) - digits
}

/// The carried columns `r` in the lanes [`group`] takes, top first.
#[target_feature(enable = "avx2")]
fn lanes(r: &[u32; 24]) -> [__m256i; 6] {
    let v = |a: usize, b, c, d| {
        _mm256_setr_epi64x(r[a] as i64, r[b] as i64, r[c] as i64, r[d] as i64)
    };
    [
        v(23, 22, 19, 18),
        v(21, 20, 17, 16),
        v(15, 14, 11, 10),
        v(13, 12, 9, 8),
        v(7, 6, 3, 2),
        v(5, 4, 1, 0),
    ]
}

#[target_feature(enable = "avx2")]
fn store(out: &mut [u8; 32], v: __m256i) {
    // SAFETY: `out` holds 32 bytes; the store is unaligned.
    unsafe { _mm256_storeu_si256(out.as_mut_ptr().cast(), v) };
}

/// The bytes of `v` from `n` on, `n <= 32`, followed by unspecified bytes.
#[target_feature(enable = "avx2")]
fn shift(v: __m256i, n: usize) -> __m256i {
    // Dword `i` is bytes `n..n + 4` of dwords `i + n / 4` and `i + n / 4 + 1`. Indices
    // past 7 wrap, into the unspecified bytes.
    let i = _mm256_set1_epi32(n as i32 / 4);
    let i = _mm256_add_epi32(_mm256_setr_epi32(0, 1, 2, 3, 4, 5, 6, 7), i);
    let lo = _mm256_permutevar8x32_epi32(v, i);
    let hi = _mm256_permutevar8x32_epi32(v, _mm256_add_epi32(i, _mm256_set1_epi32(1)));
    let bits = _mm256_set1_epi32(8 * (n % 4) as i32);
    // Shifts by 32 bits give zero.
    let hi = _mm256_sllv_epi32(hi, _mm256_sub_epi32(_mm256_set1_epi32(32), bits));
    _mm256_or_si256(_mm256_srlv_epi32(lo, bits), hi)
}

/// `ceil(2^36 / 3364)`: `(c * M) >> 36 = c / 3364` for every `c < 58^4`.
const DIV_3364: i64 = 20427907;
/// `ceil(2^18 / 58)`: `(x * M) >> 18 = x / 58` for every `x < 3364`.
const DIV_58: i16 = 4520;

/// The 32 digits, top first, of eight carried columns `j..j + 8`: `a` holds columns
/// `j + 7, j + 6, j + 3, j + 2` and `b` columns `j + 5, j + 4, j + 1, j`, one per 64-bit
/// lane. So 128-bit lane 0 has columns `j + 7` to `j + 4` and lane 1 the rest, and no step
/// crosses lanes.
#[target_feature(enable = "avx2")]
fn group(a: __m256i, b: __m256i) -> __m256i {
    let (a, b) = (digits(a), digits(b));
    // `b`'s digits to bytes 2, 3, 6 and 7 of each 64-bit lane, beside `a`'s.
    let d = _mm256_or_si256(a, _mm256_slli_epi64::<16>(b));
    #[rustfmt::skip]
    let order = _mm256_setr_epi8(
        0, 1, 4, 5, 8, 9, 12, 13, 2, 3, 6, 7, 10, 11, 14, 15,
        0, 1, 4, 5, 8, 9, 12, 13, 2, 3, 6, 7, 10, 11, 14, 15,
    );
    _mm256_shuffle_epi8(d, order)
}

/// The digits of four columns below 58^4, one per 64-bit lane: bytes 0, 1, 4 and 5 of
/// each lane, top first; the other bytes are zero.
#[target_feature(enable = "avx2")]
fn digits(c: __m256i) -> __m256i {
    let q = _mm256_srli_epi64::<36>(_mm256_mul_epu32(c, _mm256_set1_epi64x(DIV_3364)));
    let r = _mm256_sub_epi64(c, _mm256_mul_epu32(q, _mm256_set1_epi64x(3364)));
    // u16 lanes `q, 0, r, 0` per column, then `x / 58` and `x % 58` of each.
    let x = _mm256_or_si256(q, _mm256_slli_epi64::<32>(r));
    let a = _mm256_srli_epi16::<2>(_mm256_mulhi_epu16(x, _mm256_set1_epi16(DIV_58)));
    let b = _mm256_sub_epi16(x, _mm256_mullo_epi16(a, _mm256_set1_epi16(58)));
    _mm256_or_si256(a, _mm256_slli_epi16::<8>(b))
}

/// The characters of 32 digits below 58: `'1' + d`, plus the alphabet's gaps before
/// 'A' (7), 'J', 'P', 'a' (6) and 'm'.
#[target_feature(enable = "avx2")]
fn alphabet(d: __m256i) -> __m256i {
    // -1 where `d > n`.
    let gt = |n: i8| _mm256_cmpgt_epi8(d, _mm256_set1_epi8(n));
    let wide = _mm256_add_epi8(
        _mm256_and_si256(gt(8), _mm256_set1_epi8(7)),
        _mm256_and_si256(gt(32), _mm256_set1_epi8(6)),
    );
    let narrow = _mm256_add_epi8(_mm256_add_epi8(gt(16), gt(21)), gt(43));
    let base = _mm256_add_epi8(d, _mm256_set1_epi8(b'1' as i8));
    _mm256_sub_epi8(_mm256_add_epi8(base, wide), narrow)
}

/// The column sums of `x`'s words, four columns per block, with the contract of
/// `fixed::cols`. Not inlined, as in the Zen 5 runs that chose [`encode_64`]'s kernels.
#[inline(never)]
#[target_feature(enable = "avx2")]
fn cols<const N: usize, const W: usize, const C: usize>(x: &[u8; N], t: &Table<W, C>) -> [u64; C] {
    let w = words::<N, W>(x);
    let mut out = [0; C];
    for (b, o) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let mut s = _mm256_setzero_si256();
        let first = t.start[4 * b];
        for (&w, row) in w[first..].iter().zip(&t.rows[first..]) {
            let e = &row[4 * b..][..4];
            // SAFETY: `e` holds 16 bytes; the load is unaligned.
            let e = _mm256_cvtepu32_epi64(unsafe { _mm_loadu_si128(e.as_ptr().cast()) });
            // `vpmuludq` multiplies the low 32 bits of each lane.
            s = _mm256_add_epi64(s, _mm256_mul_epu32(e, _mm256_set1_epi64x(w as i64)));
        }
        // SAFETY: `o` holds 32 bytes; the store is unaligned.
        unsafe { _mm256_storeu_si256(o.as_mut_ptr().cast(), s) };
    }
    out
}

#[cfg(test)]
mod tests {
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    use super::super::fixed::TABLE_32;
    use super::super::four_digits;
    use super::super::table::COL_BASE;
    use super::*;
    use crate::common::ALPHABET;

    fn avx2() -> bool {
        let yes = is_x86_feature_detected!("avx2");
        if !yes {
            eprintln!("no AVX2 on this CPU: AVX2 kernels not run");
        }
        yes
    }

    #[test]
    fn cols_match_portable() {
        if !avx2() {
            return;
        }
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(45);
        let mut inputs: Vec<[u8; 64]> = [0, 1, 0x80, 0xFF].map(|b| [b; 64]).to_vec();
        inputs.extend((0..1000).map(|_| std::array::from_fn(|_| rng.next_u32() as u8)));
        for x in &inputs {
            let half: &[u8; 32] = x[..32].try_into().unwrap();
            // SAFETY: the CPU has AVX2.
            unsafe {
                assert_eq!(cols(x, &TABLE_64), fixed::cols(x, &TABLE_64), "{x:?}");
                assert_eq!(cols(half, &TABLE_32), fixed::cols(half, &TABLE_32), "{x:?}");
            }
        }
    }

    /// Every column value below 58^4, in all four lanes, against the scalar kernel.
    #[test]
    fn digits_every_value() {
        if !avx2() {
            return;
        }
        let mut lanes = [0u64; 4];
        for c in 0..COL_BASE {
            lanes[c as usize % 4] = c;
            if c % 4 != 3 && c != COL_BASE - 1 {
                continue;
            }
            // SAFETY: the CPU has AVX2; the loads and stores are unaligned, 32 bytes.
            let got: [[u8; 8]; 4] = unsafe {
                let d = digits(_mm256_loadu_si256(lanes.as_ptr().cast()));
                std::mem::transmute(d)
            };
            for (l, (&c, g)) in lanes.iter().zip(got).enumerate() {
                let digit = |ch| ALPHABET.iter().position(|&a| a == ch).unwrap() as u8;
                let want = four_digits(c).map(digit);
                assert_eq!([g[0], g[1], g[4], g[5]], want, "{c} in lane {l}");
                assert_eq!([g[2], g[3], g[6], g[7]], [0; 4], "{c} in lane {l}");
            }
        }
    }

    /// Every digit through `alphabet`, at every byte.
    #[test]
    fn alphabet_every_digit() {
        if !avx2() {
            return;
        }
        for first in 0..58u8 {
            let d: [u8; 32] = std::array::from_fn(|i| (first + i as u8) % 58);
            // SAFETY: the CPU has AVX2.
            let got: [u8; 32] = unsafe {
                std::mem::transmute(alphabet(std::mem::transmute::<[u8; 32], __m256i>(d)))
            };
            let want = d.map(|d| ALPHABET[d as usize]);
            assert_eq!(got, want);
        }
    }

    #[test]
    fn shift_every_count() {
        if !avx2() {
            return;
        }
        let v: [u8; 32] = std::array::from_fn(|i| i as u8 + 100);
        for n in 0..=32 {
            // SAFETY: the CPU has AVX2.
            let got: [u8; 32] = unsafe {
                std::mem::transmute(shift(std::mem::transmute::<[u8; 32], __m256i>(v), n))
            };
            assert_eq!(got[..32 - n], v[n..], "shift by {n}");
        }
    }

    /// `encode_64` against bs58: random inputs, every count of leading zero bytes, all
    /// 0xFF, inputs that take the serial fallback; into an ample buffer (nothing written
    /// past the result), an exact one and one a byte short (nothing written).
    #[test]
    fn encode_64_matches_bs58() {
        if !avx2() {
            return;
        }
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(46);
        let mut random = || -> [u8; 64] { std::array::from_fn(|_| rng.next_u32() as u8) };
        let mut inputs: Vec<[u8; 64]> = (0..3000).map(|_| random()).collect();
        for zeros in 0..=64 {
            let mut x = random();
            x[..zeros].fill(0);
            if zeros < 64 {
                x[zeros] = x[zeros].max(1);
                inputs.push(x);
                x[zeros] = 1;
                x[zeros + 1..].fill(0);
            }
            inputs.push(x);
        }
        inputs.extend([0x80, 0xFF].map(|b| [b; 64]));
        inputs.extend(fixed::split_fallbacks(&mut rng, 4, 20));
        for x in &inputs {
            let want = bs58::encode(x).into_vec();
            // SAFETY: the CPU has AVX2.
            let f = |o: &mut [u8]| unsafe { encode_64(x, o) };
            let mut out = vec![0xAA; want.len() + 40];
            assert_eq!(f(&mut out), Ok(want.len()), "{x:?}");
            assert!(out[..want.len()] == want[..], "{x:?}");
            let past = out[want.len()..].iter().any(|&b| b != 0xAA);
            assert!(!past, "{x:?}: wrote past the result");
            assert_eq!(f(&mut out[..want.len()]), Ok(want.len()), "{x:?}: exact");
            let mut short = vec![0xAA; want.len() - 1];
            assert_eq!(f(&mut short), Err(BufferTooSmall), "{x:?}: short");
            assert!(short.iter().all(|&b| b == 0xAA), "{x:?}: wrote into a short buffer");
        }
    }
}

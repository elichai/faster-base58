//! AVX2 kernels for the fixed sizes, with the contracts of `fixed::cols` and
//! `fixed::digits`. `fixed.rs` picks them at run time.

use core::arch::x86_64::*;

use super::fixed::{self, Table, words};
use super::table::COL_BASE;
use crate::BufferTooSmall;

/// `fixed::encode` with the AVX2 kernels.
#[target_feature(enable = "avx2")]
pub(super) fn encode_fixed<const N: usize, const W: usize, const C: usize>(
    x: &[u8; N],
    output: &mut [u8],
    t: &Table<W, C>,
) -> Result<usize, BufferTooSmall> {
    fixed::encode(x, output, |x, o| fixed::chars(x, o, |x| cols(x, t), |t, q, o| digits(t, q, o)))
}

/// The column sums of `x`'s words, four columns per block.
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

/// `ceil(2^36 / 3364)`: `(c * M) >> 36 = c / 3364` for every `c < 58^4`.
const DIV_3364: i64 = 20427907;
/// `ceil(2^18 / 58)`: `(x * M) >> 18 = x / 58` for every `x < 3364`.
const DIV_58: i16 = 4520;

/// See `fixed::digits`. Four columns per step; requires a multiple of four.
#[target_feature(enable = "avx2")]
fn digits(t: &[u64], q: &[u32], out: &mut [[u8; 4]]) {
    assert!(t.len() == q.len() && t.len() == out.len() && t.len().is_multiple_of(4));
    // Per 128-bit lane: digits 0, 1, 4 and 5 of the upper column, then of the lower one.
    #[rustfmt::skip]
    let gather = _mm256_setr_epi8(
        8, 9, 12, 13, 0, 1, 4, 5, -1, -1, -1, -1, -1, -1, -1, -1,
        8, 9, 12, 13, 0, 1, 4, 5, -1, -1, -1, -1, -1, -1, -1, -1,
    );
    let groups = out.as_chunks_mut::<4>().0.iter_mut().rev();
    for ((t, q), o) in t.as_chunks::<4>().0.iter().zip(q.as_chunks::<4>().0).zip(groups) {
        // SAFETY: `t` holds 32 bytes and `q` 16; the loads are unaligned.
        let t = unsafe { _mm256_loadu_si256(t.as_ptr().cast()) };
        // SAFETY: as above.
        let q = unsafe { _mm_loadu_si128(q.as_ptr().cast()) };
        // `fixed::column`: right in the low 32 bits of each lane, which is all the steps
        // below read.
        let q = _mm256_mul_epu32(_mm256_cvtepu32_epi64(q), _mm256_set1_epi64x(COL_BASE as i64));
        let c = _mm256_sub_epi64(t, q);
        let q = _mm256_srli_epi64::<36>(_mm256_mul_epu32(c, _mm256_set1_epi64x(DIV_3364)));
        let r = _mm256_sub_epi64(c, _mm256_mul_epu32(q, _mm256_set1_epi64x(3364)));
        // u16 lanes `q, 0, r, 0` per column, then `x / 58` and `x % 58` of each.
        let x = _mm256_or_si256(q, _mm256_slli_epi64::<32>(r));
        let a = _mm256_srli_epi16::<2>(_mm256_mulhi_epu16(x, _mm256_set1_epi16(DIV_58)));
        let b = _mm256_sub_epi16(x, _mm256_mullo_epi16(a, _mm256_set1_epi16(58)));
        // Bytes `x / 58, x % 58`: a column's digits are bytes 0, 1, 4 and 5 of its 8.
        let d = _mm256_or_si256(a, _mm256_slli_epi16::<8>(b));
        // The upper lane's 8 digits, then the lower's: the 4 columns, top first.
        let d = _mm256_permute4x64_epi64::<0b10>(_mm256_shuffle_epi8(d, gather));
        let chars = alphabet(_mm256_castsi256_si128(d));
        // SAFETY: `o` holds 16 bytes; the store is unaligned.
        unsafe { _mm_storeu_si128(o.as_mut_ptr().cast(), chars) };
    }
}

/// The characters of 16 digits below 58: `'1' + d`, plus the alphabet's gaps before
/// 'A' (7), 'J', 'P', 'a' (6) and 'm'.
#[target_feature(enable = "avx2")]
fn alphabet(d: __m128i) -> __m128i {
    // -1 where `d > n`.
    let gt = |n: i8| _mm_cmpgt_epi8(d, _mm_set1_epi8(n));
    let wide = _mm_add_epi8(
        _mm_and_si128(gt(8), _mm_set1_epi8(7)),
        _mm_and_si128(gt(32), _mm_set1_epi8(6)),
    );
    let narrow = _mm_add_epi8(_mm_add_epi8(gt(16), gt(21)), gt(43));
    let base = _mm_add_epi8(d, _mm_set1_epi8(b'1' as i8));
    _mm_sub_epi8(_mm_add_epi8(base, wide), narrow)
}

#[cfg(test)]
mod tests {
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    use super::super::fixed::{TABLE_32, TABLE_64};
    use super::*;

    /// Every column value, in groups of four, against the scalar kernel: as `t` with
    /// `q = 0`, and as `t - 58^4 q` with `t` past 2^32.
    #[test]
    fn digits_every_value() {
        if !is_x86_feature_detected!("avx2") {
            eprintln!("no AVX2 on this CPU: AVX2 digits not run");
            return;
        }
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(44);
        let cols: Vec<u64> = (0..COL_BASE).collect();
        let mut want = vec![[0; 4]; cols.len()];
        super::super::digits(&cols, &mut want);
        let zero = vec![0; cols.len()];
        let q: Vec<u32> = cols.iter().map(|_| rng.next_u32()).collect();
        let t: Vec<u64> = cols.iter().zip(&q).map(|(&c, &q)| c + COL_BASE * q as u64).collect();
        for (t, q) in [(&cols, &zero), (&t, &q)] {
            let mut got = vec![[0; 4]; cols.len()];
            // SAFETY: the CPU has AVX2.
            unsafe { digits(t, q, &mut got) };
            assert!(got == want);
        }
    }

    #[test]
    fn cols_match_portable() {
        if !is_x86_feature_detected!("avx2") {
            eprintln!("no AVX2 on this CPU: AVX2 columns not run");
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
}

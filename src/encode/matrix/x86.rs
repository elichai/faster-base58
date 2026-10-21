//! AVX2 column sums for the fixed sizes, with the contract of `fixed::cols`. `fixed.rs`
//! picks them at run time for 64 bytes.

use core::arch::x86_64::*;

use super::fixed::{self, Table, words};
use crate::BufferTooSmall;

/// `fixed::encode` with AVX2 column sums and the plain-Rust carry and digits, compiled for
/// AVX2. On Zen 5 an AVX2 digit kernel took longer than the plain-Rust one, at both sizes
/// (README, "Fixed sizes").
#[target_feature(enable = "avx2")]
pub(super) fn encode_fixed<const N: usize, const W: usize, const C: usize>(
    x: &[u8; N],
    output: &mut [u8],
    t: &Table<W, C>,
) -> Result<usize, BufferTooSmall> {
    fixed::encode(x, output, |x, o| fixed::chars(x, o, |x| cols(x, t), fixed::digits))
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

#[cfg(test)]
mod tests {
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    use super::super::fixed::{TABLE_32, TABLE_64};
    use super::*;

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

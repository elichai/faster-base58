//! Wasm SIMD128 kernels for 64 bytes: the fixed-size column sums, with the contract of
//! `fixed::portable::cols`, and digits of carried columns.

use core::arch::wasm32::*;

use super::fixed::{self, Table, words};
use crate::BufferTooSmall;
use crate::common::ALPHABET;
use crate::encode::max_encoded_len;

/// Fixed 32-byte encoding uses scalar kernels: with these SIMD column sums and digits it
/// took 1.69x the time on M5 / V8 (2026-11-07).
pub(crate) fn encode_fixed_32(x: &[u8; 32], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    fixed::portable::encode_32_portable(x, output)
}

/// Fixed 64-byte encoding with the carry in four segments.
pub(crate) fn encode_fixed_64(x: &[u8; 64], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    fixed::encode(x, output, |x, o| {
        let (t, r) = (cols(x, &fixed::TABLE_64), max_encoded_len(64).div_ceil(4));
        let c = fixed::carry_split::<24, 4>(&t, r).unwrap_or_else(|| carry_cold(&t, r));
        column_digits(&c, o)
    })
}

/// Serial carry fallback, kept cold and out of line.
#[cold]
#[inline(never)]
fn carry_cold(t: &[u64; 24], r: usize) -> [u32; 24] {
    fixed::carry_serial(t, r)
}

/// The column sums of `x`'s words, four columns per block. A row that is zero in a
/// block's upper two columns adds to the lower two only.
#[inline(always)]
fn cols<const N: usize, const W: usize, const C: usize>(x: &[u8; N], t: &Table<W, C>) -> [u64; C] {
    let w = words::<N, W>(x).map(|w| u32x4_splat(w));
    // Opaque, so that the products stay `extmul`: LLVM would see the splats and the
    // constant table entries through the extensions and emit `i64x2.mul`, which V8
    // expands to about seven instructions on `AArch64`.
    let (w, rows) = (*core::hint::black_box(&w), core::hint::black_box(&t.rows));
    let mut out = [0; C];
    for (b, o) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let (low, full) = (t.start[4 * b], t.start[4 * b + 2]);
        let (mut lo, mut hi) = (u64x2_splat(0), u64x2_splat(0));
        // SAFETY: each load reads 4 table words of a row, `C` being a multiple of 4; the
        // stores write the 4 columns of `o`.
        unsafe {
            let row = |k: usize| v128_load(rows[k][4 * b..][..4].as_ptr().cast());
            for (k, &w) in w.iter().enumerate().take(full).skip(low) {
                lo = i64x2_add(lo, u64x2_extmul_low_u32x4(row(k), w));
            }
            for (k, &w) in w.iter().enumerate().skip(full) {
                let e = row(k);
                lo = i64x2_add(lo, u64x2_extmul_low_u32x4(e, w));
                hi = i64x2_add(hi, u64x2_extmul_high_u32x4(e, w));
            }
            v128_store(o.as_mut_ptr().cast(), lo);
            v128_store(o.as_mut_ptr().add(2).cast(), hi);
        }
    }
    out
}

/// `ceil(2^36 / 3364)`: `(c * M) >> 36 = c / 3364` for every `c < 58^4`.
const DIV_3364: u32 = 20427907;
/// `ceil(2^16 / 58)`: `(x * M) >> 16 = x / 58` for every `x < 3364`.
const DIV_58: u16 = 1130;

/// The alphabet in four 16-byte tables, padded with zeros.
const TABLES: [v128; 4] = {
    let mut t = [[0u8; 16]; 4];
    let mut i = 0;
    while i < ALPHABET.len() {
        t[i / 16][i % 16] = ALPHABET[i];
        i += 1;
    }
    // SAFETY: `[u8; 16]` and `v128` have the same size, and every bit pattern is valid.
    unsafe { core::mem::transmute(t) }
};

/// The characters of four columns below 58^4, given lowest first: the top column's first.
#[inline(always)]
fn chars(c: v128) -> v128 {
    // Opaque, as in `cols`.
    let m = *core::hint::black_box(&u32x4_splat(DIV_3364));
    let q =
        i32x4_shuffle::<1, 3, 5, 7>(u64x2_extmul_low_u32x4(c, m), u64x2_extmul_high_u32x4(c, m));
    let q = u32x4_shr(q, 4);
    let r = i32x4_sub(c, i32x4_mul(q, u32x4_splat(3364)));
    // u16 lanes: the two halves of each column, high half first.
    let x = v128_or(q, i32x4_shl(r, 16));
    let m = u16x8_splat(DIV_58);
    let a = u32x4_extmul_low_u16x8(x, m);
    let a = i16x8_shuffle::<1, 3, 5, 7, 9, 11, 13, 15>(a, u32x4_extmul_high_u16x8(x, m));
    let b = i16x8_sub(x, i16x8_mul(a, u16x8_splat(58)));
    // Bytes: each column's four digits, most significant first; lowest column first.
    let d = v128_or(a, i16x8_shl(b, 8));
    let mut ch = u8x16_swizzle(TABLES[0], d);
    for (i, &t) in TABLES.iter().enumerate().skip(1) {
        ch = v128_or(
            ch,
            u8x16_swizzle(t, u8x16_sub(d, u8x16_splat(16 * i as u8))),
        );
    }
    i32x4_shuffle::<3, 2, 1, 0>(ch, ch)
}

/// The characters of the carried columns `c` (see `fixed::carry_split`), top first:
/// `out[i]` holds those of column `C - 1 - i`. Requires whole groups of four columns.
#[inline(always)]
fn column_digits<const C: usize>(c: &[u32; C], out: &mut [[u8; 4]; C]) {
    const { assert!(C.is_multiple_of(4)) };
    for g in 0..C / 4 {
        // SAFETY: step `g` reads `c[C - 4 (g + 1)..C - 4 g]` and writes `out[4 g..4 g + 4]`.
        unsafe {
            let v = v128_load(c.as_ptr().add(C - 4 * (g + 1)).cast());
            v128_store(out.as_mut_ptr().add(4 * g).cast(), chars(v));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{COL_BASE, four_digits};
    use super::fixed::portable;
    use super::*;
    use crate::encode::tests::random_bytes;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::SeedableRng;

    /// The 64-byte column kernel and whole encodes against the plain-Rust ones, including
    /// inputs on which the split carry falls back.
    #[test]
    fn fixed_64_matches_portable() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
        let fallbacks = fixed::split_fallbacks(&mut rng, 4, 20);
        let inputs = (0..1000)
            .map(|_| random_bytes(&mut rng, 64))
            .chain([vec![0; 64], vec![0xFF; 64]])
            .chain(fallbacks.into_iter().map(Vec::from));
        for x in inputs {
            let x: &[u8; 64] = x[..].try_into().unwrap();
            let t = &fixed::TABLE_64;
            assert_eq!(cols(x, t), portable::cols(x, t), "{x:?}");
            let (mut a, mut b) = ([0; 100], [0; 100]);
            let n = encode_fixed_64(x, &mut a);
            assert_eq!(n, portable::encode_64_portable(x, &mut b), "{x:?}");
            assert_eq!(a, b, "{x:?}");
        }
    }

    /// `chars` of every column value, in groups of four.
    #[test]
    fn every_value() {
        for c in (0..COL_BASE as u32).step_by(4) {
            let mut got = [[0u8; 4]; 4];
            // SAFETY: `got` has 16 bytes.
            unsafe {
                v128_store(
                    got.as_mut_ptr().cast(),
                    chars(u32x4(c, c + 1, c + 2, c + 3)),
                )
            };
            for (i, o) in got.iter().enumerate() {
                let c = c + 3 - i as u32;
                assert_eq!(*o, four_digits(c.into()), "{c}");
            }
        }
    }
}

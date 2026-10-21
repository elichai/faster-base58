//! NEON kernels: digits, with the contract of `super::digits`, and the fixed-size column
//! sums and digits, with those of `fixed::cols` and `fixed::digits`.

use core::arch::aarch64::*;

use super::fixed::{Table, words};
use super::four_digits;
use super::table::COL_BASE;
use crate::common::ALPHABET;

/// The column sums of `x`'s words, four columns per block: `umlal` on the low and high
/// halves of four table entries. A row that is zero in a block's upper two columns adds
/// to the lower two only.
#[inline(always)]
pub(super) fn cols<const N: usize, const W: usize, const C: usize>(
    x: &[u8; N],
    t: &Table<W, C>,
) -> [u64; C] {
    let w = words::<N, W>(x);
    // Opaque, so that the entries are loaded 4 at a time: as constants, each pair of them
    // took its own load.
    let rows = std::hint::black_box(&t.rows);
    let mut out = [0; C];
    for (b, o) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let (low, full) = (t.start[4 * b], t.start[4 * b + 2]);
        // SAFETY: NEON is enabled at compile time (cfg on the module). The loads read 2 and
        // 4 table words, and the stores write the 4 columns of `o`.
        unsafe {
            let (mut lo, mut hi) = (vdupq_n_u64(0), vdupq_n_u64(0));
            // Indexed on purpose: with zipped iterators 64 bytes ran 1.6x slower (README,
            // "Fixed sizes").
            for k in low..full {
                lo = vmlal_n_u32(lo, vld1_u32(rows[k][4 * b..][..2].as_ptr()), w[k]);
            }
            for k in full..W {
                let e = vld1q_u32(rows[k][4 * b..][..4].as_ptr());
                lo = vmlal_n_u32(lo, vget_low_u32(e), w[k]);
                hi = vmlal_high_n_u32(hi, e, w[k]);
            }
            vst1q_u64(o.as_mut_ptr(), lo);
            vst1q_u64(o.as_mut_ptr().add(2), hi);
        }
    }
    out
}

/// `ceil(2^36 / 3364)`: `(c * M) >> 36 = c / 3364` for every `c < 58^4`.
const DIV_3364: i32 = 20427907;
/// `ceil(2^18 / 58)`: `(x * M) >> 18 = x / 58` for every `x < 3364`.
const DIV_58: i16 = 4520;

/// The alphabet padded to 64 bytes for `tbl`.
const TABLE: [u8; 64] = {
    let mut t = [0; 64];
    let mut i = 0;
    while i < ALPHABET.len() {
        t[i] = ALPHABET[i];
        i += 1;
    }
    t
};

/// Reverses the order of four 4-byte groups.
const REVERSE: [u8; 16] = [12, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3];

/// The characters of four columns below 58^4, given lowest first: the top column's first.
#[inline(always)]
fn chars(c: uint32x4_t, table: uint8x16x4_t, reverse: uint8x16_t) -> uint8x16_t {
    // SAFETY: NEON is enabled at compile time (cfg on the module).
    unsafe {
        // `(c * M) >> 31` (no saturation: both are positive), then `>> 5`.
        let q = vreinterpretq_u32_s32(vqdmulhq_s32(
            vreinterpretq_s32_u32(c),
            vdupq_n_s32(DIV_3364),
        ));
        let q = vshrq_n_u32::<5>(q);
        let r = vmlsq_n_u32(c, q, 3364);
        // u16 lanes: the two halves of each column, high half first.
        let x = vreinterpretq_s16_u32(vsliq_n_u32::<16>(q, r));
        let a = vshrq_n_u16::<3>(vreinterpretq_u16_s16(vqdmulhq_n_s16(x, DIV_58)));
        let b = vmlsq_n_u16(vreinterpretq_u16_s16(x), a, 58);
        // Bytes: each column's four digits, most significant first; lowest column first.
        let d = vreinterpretq_u8_u16(vsliq_n_u16::<8>(a, b));
        vqtbl1q_u8(vqtbl4q_u8(table, d), reverse)
    }
}

/// See `super::digits`.
pub(super) fn digits(cols: &[u64], out: &mut [[u8; 4]]) {
    assert_eq!(cols.len(), out.len());
    let n = cols.len();
    let (quads, rest) = (n / 4, n % 4);
    // SAFETY: NEON is enabled at compile time (cfg on the module). Step `g < quads` reads
    // `cols[n - 4 (g + 1)..n - 4 g]` and writes `out[4 g..4 g + 4]`: in bounds as
    // `out.len() == n`.
    unsafe {
        let table = vld1q_u8_x4(TABLE.as_ptr());
        let reverse = vld1q_u8(REVERSE.as_ptr());
        for g in 0..quads {
            let c = cols.as_ptr().add(n - 4 * (g + 1));
            // The low halves of the four columns, which are below 58^4.
            let lo = vreinterpretq_u32_u64(vld1q_u64(c));
            let hi = vreinterpretq_u32_u64(vld1q_u64(c.add(2)));
            let chars = chars(vuzp1q_u32(lo, hi), table, reverse);
            vst1q_u8(out.as_mut_ptr().add(4 * g).cast(), chars);
        }
    }
    for (o, &c) in out[4 * quads..].iter_mut().zip(cols[..rest].iter().rev()) {
        *o = four_digits(c);
    }
}

/// See `fixed::digits`. Requires whole groups of four columns.
#[inline(always)]
pub(super) fn fixed_digits(t: &[u64], q: &[u32], out: &mut [[u8; 4]]) {
    let n = t.len();
    assert!(q.len() == n && out.len() == n && n.is_multiple_of(4));
    // SAFETY: NEON is enabled at compile time (cfg on the module). Step `g` reads `t` and
    // `q` at `n - 4 (g + 1)..n - 4 g` and writes `out[4 g..4 g + 4]`: in bounds by the
    // assert.
    unsafe {
        let table = vld1q_u8_x4(TABLE.as_ptr());
        let reverse = vld1q_u8(REVERSE.as_ptr());
        for g in 0..n / 4 {
            let i = n - 4 * (g + 1);
            let lo = vreinterpretq_u32_u64(vld1q_u64(t.as_ptr().add(i)));
            let hi = vreinterpretq_u32_u64(vld1q_u64(t.as_ptr().add(i + 2)));
            // `fixed::column` of the four, modulo 2^32.
            let q = vld1q_u32(q.as_ptr().add(i));
            let c = vmlsq_n_u32(vuzp1q_u32(lo, hi), q, COL_BASE as u32);
            vst1q_u8(out.as_mut_ptr().add(4 * g).cast(), chars(c, table, reverse));
        }
    }
}

/// The characters of the carried columns `c` (see `fixed::carry_split`), top first:
/// `out[i]` holds those of column `C - 1 - i`. Requires whole groups of four columns.
#[inline(always)]
pub(super) fn column_digits<const C: usize>(c: &[u32; C], out: &mut [[u8; 4]; C]) {
    const { assert!(C.is_multiple_of(4)) };
    // SAFETY: NEON is enabled at compile time (cfg on the module). Step `g` reads
    // `c[C - 4 (g + 1)..C - 4 g]` and writes `out[4 g..4 g + 4]`.
    unsafe {
        let table = vld1q_u8_x4(TABLE.as_ptr());
        let reverse = vld1q_u8(REVERSE.as_ptr());
        for g in 0..C / 4 {
            let v = vld1q_u32(c.as_ptr().add(C - 4 * (g + 1)));
            vst1q_u8(out.as_mut_ptr().add(4 * g).cast(), chars(v, table, reverse));
        }
    }
}

/// Carry variants for 32 and 64 bytes, for one M5 sweep: `benches/encode.rs` times each.
/// Temporary: the winner replaces `fixed::encode_best`'s carry.
#[doc(hidden)]
pub mod sweep {
    use super::super::fixed::{self, TABLE_32, TABLE_64};
    use super::*;
    use crate::BufferTooSmall;

    type Encode = fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall>;

    /// `encode_matrix`'s path: the serial carry keeps the quotients.
    fn quotients_32(d: &[u8], o: &mut [u8]) -> Result<usize, BufferTooSmall> {
        let x: &[u8; 32] = d.try_into().expect("32 bytes");
        fixed::encode(x, o, |x, o| fixed::chars(x, o, |x| cols(x, &TABLE_32), fixed_digits))
    }

    fn quotients_64(d: &[u8], o: &mut [u8]) -> Result<usize, BufferTooSmall> {
        let x: &[u8; 64] = d.try_into().expect("64 bytes");
        fixed::encode(x, o, |x, o| fixed::chars(x, o, |x| cols(x, &TABLE_64), fixed_digits))
    }

    /// The carried columns from `K` segments (1: serial).
    fn split_32<const K: usize>(d: &[u8], o: &mut [u8]) -> Result<usize, BufferTooSmall> {
        let x: &[u8; 32] = d.try_into().expect("32 bytes");
        fixed::encode(x, o, |x, o| {
            fixed::chars_split::<32, 12, K>(x, o, |x| cols(x, &TABLE_32), column_digits)
        })
    }

    fn split_64<const K: usize>(d: &[u8], o: &mut [u8]) -> Result<usize, BufferTooSmall> {
        let x: &[u8; 64] = d.try_into().expect("64 bytes");
        fixed::encode(x, o, |x, o| {
            fixed::chars_split::<64, 24, K>(x, o, |x| cols(x, &TABLE_64), column_digits)
        })
    }

    /// The carried columns `c` in vectors of four, lowest first.
    #[inline(always)]
    fn vectors<const C: usize, const G: usize>(c: &[u32; C]) -> [uint32x4_t; G] {
        const { assert!(C == 4 * G) };
        // SAFETY: NEON is enabled at compile time (cfg on the module).
        std::array::from_fn(|g| unsafe {
            let pair = |i: usize| vcreate_u32(c[i] as u64 | (c[i + 1] as u64) << 32);
            vcombine_u32(pair(4 * g), pair(4 * g + 2))
        })
    }

    /// The fallback, returning vectors so that the hot path's columns need not go through
    /// an array on the stack.
    #[cold]
    #[inline(never)]
    fn vectors_cold<const C: usize, const G: usize>(t: &[u64; C], r: usize) -> [uint32x4_t; G] {
        vectors(&fixed::carry_serial(t, r))
    }

    /// [`split_32`] and [`split_64`] with the carried columns moved into vectors from
    /// general registers.
    #[inline(always)]
    fn regs<const N: usize, const W: usize, const C: usize, const G: usize, const K: usize>(
        d: &[u8],
        o: &mut [u8],
        table: &Table<W, C>,
    ) -> Result<usize, BufferTooSmall> {
        let x: &[u8; N] = d.try_into().expect("N bytes");
        fixed::encode(x, o, |x, o: &mut [[u8; 4]; C]| {
            let t = cols(x, table);
            let r = crate::encode::max_encoded_len(N).div_ceil(4);
            let v: [uint32x4_t; G] = match fixed::carry_split::<C, K>(&t, r) {
                Some(c) => vectors(&c),
                None => vectors_cold(&t, r),
            };
            // SAFETY: NEON is enabled at compile time (cfg on the module). Step `g` writes
            // `o[4 g..4 g + 4]`, and `o` has `C = 4 G` entries.
            unsafe {
                let alphabet = vld1q_u8_x4(TABLE.as_ptr());
                let reverse = vld1q_u8(REVERSE.as_ptr());
                for g in 0..G {
                    let c = chars(v[G - 1 - g], alphabet, reverse);
                    vst1q_u8(o.as_mut_ptr().add(4 * g).cast(), c);
                }
            }
        })
    }

    fn regs_32<const K: usize>(d: &[u8], o: &mut [u8]) -> Result<usize, BufferTooSmall> {
        regs::<32, 8, 12, 3, K>(d, o, &TABLE_32)
    }

    fn regs_64<const K: usize>(d: &[u8], o: &mut [u8]) -> Result<usize, BufferTooSmall> {
        regs::<64, 16, 24, 6, K>(d, o, &TABLE_64)
    }

    /// [`regs_64`] with the digit groups in the other order: a probe of placement.
    fn regs_64b<const K: usize>(d: &[u8], o: &mut [u8]) -> Result<usize, BufferTooSmall> {
        let x: &[u8; 64] = d.try_into().expect("64 bytes");
        fixed::encode(x, o, |x, o: &mut [[u8; 4]; 24]| {
            let t = cols(x, &TABLE_64);
            let r = crate::encode::max_encoded_len(64).div_ceil(4);
            let v: [uint32x4_t; 6] = match fixed::carry_split::<24, K>(&t, r) {
                Some(c) => vectors(&c),
                None => vectors_cold(&t, r),
            };
            // SAFETY: as in `regs`.
            unsafe {
                let alphabet = vld1q_u8_x4(TABLE.as_ptr());
                let reverse = vld1q_u8(REVERSE.as_ptr());
                for g in (0..6).rev() {
                    let c = chars(v[5 - g], alphabet, reverse);
                    vst1q_u8(o.as_mut_ptr().add(4 * g).cast(), c);
                }
            }
        })
    }

    /// Name, input size, encoder.
    pub const VARIANTS: [(&str, usize, Encode); 17] = [
        ("fixed-q", 32, quotients_32),
        ("fixed-serial", 32, split_32::<1>),
        ("fixed-split2", 32, split_32::<2>),
        ("fixed-split3", 32, split_32::<3>),
        ("fixed-split4", 32, split_32::<4>),
        ("fixed-regs-split2", 32, regs_32::<2>),
        ("fixed-regs-split3", 32, regs_32::<3>),
        ("fixed-q", 64, quotients_64),
        ("fixed-serial", 64, split_64::<1>),
        ("fixed-split2", 64, split_64::<2>),
        ("fixed-split3", 64, split_64::<3>),
        ("fixed-split4", 64, split_64::<4>),
        ("fixed-split6", 64, split_64::<6>),
        ("fixed-regs-split3", 64, regs_64::<3>),
        ("fixed-regs-split4", 64, regs_64::<4>),
        ("fixed-regs-split6", 64, regs_64::<6>),
        ("fixed-regs-split4-b", 64, regs_64b::<4>),
    ];
}

#[cfg(test)]
mod tests {
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    use super::super::fixed;
    use super::*;
    use crate::BufferTooSmall;

    /// Each sweep variant against bs58: random inputs, every count of leading zero bytes,
    /// all 0xFF, and inputs that force each split's fallback; into an ample buffer
    /// (nothing written past the result), an exact one and one a byte short.
    #[test]
    fn sweep_matches_bs58() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(48);
        let mut inputs: Vec<Vec<u8>> = vec![];
        for n in [32, 64] {
            for _ in 0..3000 {
                inputs.push((0..n).map(|_| rng.next_u32() as u8).collect());
            }
            for zeros in 0..=n {
                let mut x: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
                x[..zeros].fill(0);
                if zeros < n {
                    x[zeros] = x[zeros].max(1);
                }
                inputs.push(x);
            }
            inputs.push(vec![0xFF; n]);
        }
        for k in [2, 3, 4] {
            inputs.extend(fixed::split_fallbacks::<32>(&mut rng, k, 5).into_iter().map(Vec::from));
        }
        for k in [2, 3, 4, 6] {
            inputs.extend(fixed::split_fallbacks::<64>(&mut rng, k, 5).into_iter().map(Vec::from));
        }
        for x in &inputs {
            let want = bs58::encode(x).into_vec();
            for (name, n, f) in sweep::VARIANTS {
                if n != x.len() {
                    continue;
                }
                let at = format!("{name}, {n} bytes: {x:?}");
                let mut out = vec![0xAA; want.len() + 40];
                assert_eq!(f(x, &mut out), Ok(want.len()), "{at}");
                assert!(out[..want.len()] == want[..], "{at}");
                assert!(out[want.len()..].iter().all(|&b| b == 0xAA), "{at}: wrote past");
                assert_eq!(f(x, &mut out[..want.len()]), Ok(want.len()), "{at}: exact");
                let mut short = vec![0xAA; want.len() - 1];
                assert_eq!(f(x, &mut short), Err(BufferTooSmall), "{at}: short");
                assert!(short.iter().all(|&b| b == 0xAA), "{at}: wrote into a short buffer");
            }
        }
    }

    #[test]
    fn matches_scalar() {
        for n in [0, 1, 3, 4, 5, 8, 9, 100] {
            for start in [0, COL_BASE - 400, 1_000_003] {
                let cols: Vec<u64> = (0..n).map(|i| (start + 3 * i as u64) % COL_BASE).collect();
                let mut want = vec![[0; 4]; n];
                super::super::digits(&cols, &mut want);
                let mut got = vec![[0; 4]; n];
                digits(&cols, &mut got);
                assert_eq!(got, want, "{n} columns from {start}");
            }
        }
    }

    /// Every column value, in groups of four.
    #[test]
    fn every_value() {
        let cols: Vec<u64> = (0..COL_BASE).collect();
        let mut got = vec![[0; 4]; cols.len()];
        digits(&cols, &mut got);
        for (o, &c) in got.iter().zip(cols.iter().rev()) {
            assert_eq!(*o, four_digits(c), "{c}");
        }
    }
}

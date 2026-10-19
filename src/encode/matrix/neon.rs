//! NEON digit kernel. Same contract as `super::digits`.

use core::arch::aarch64::*;

use super::four_digits;
use crate::common::ALPHABET;

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
            let c = vuzp1q_u32(lo, hi);
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
            let chars = vqtbl1q_u8(vqtbl4q_u8(table, d), reverse);
            vst1q_u8(out.as_mut_ptr().add(4 * g).cast(), chars);
        }
    }
    for (o, &c) in out[4 * quads..].iter_mut().zip(cols[..rest].iter().rev()) {
        *o = four_digits(c);
    }
}

#[cfg(test)]
mod tests {
    use super::super::table::COL_BASE;
    use super::*;

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

//! Plain-Rust fixed-size kernels and the reference kernels for tests.

use super::super::four_digits;
use super::{TABLE_32, TABLE_64, Table, chars, column, encode, words};
use crate::BufferTooSmall;

/// Fixed 32-byte encoding with plain-Rust kernels.
///
/// # Errors
/// Returns [`BufferTooSmall`] if `out` cannot hold the encoded string.
pub fn encode_32_portable(x: &[u8; 32], out: &mut [u8]) -> Result<usize, BufferTooSmall> {
    encode(x, out, |x, o| chars(x, o, |x| cols(x, &TABLE_32), digits))
}

/// Fixed 64-byte encoding with plain-Rust kernels.
///
/// # Errors
/// Returns [`BufferTooSmall`] if `out` cannot hold the encoded string.
pub fn encode_64_portable(x: &[u8; 64], out: &mut [u8]) -> Result<usize, BufferTooSmall> {
    encode(x, out, |x, o| chars(x, o, |x| cols(x, &TABLE_64), digits))
}

/// The column sums `Σ w[k] rows[k][j]` of `x`'s words.
#[inline(always)]
pub(in crate::encode::matrix) fn cols<const N: usize, const W: usize, const C: usize>(
    x: &[u8; N],
    t: &Table<W, C>,
) -> [u64; C] {
    let w = words::<N, W>(x);
    core::array::from_fn(|j| {
        (t.start[j]..W)
            .map(|k| w[k] as u64 * t.rows[k][j] as u64)
            .sum()
    })
}

/// The characters of the carried columns (see [`super::carry`]), top first:
/// `out[i]` holds those of column `n - 1 - i`. Requires `t`, `q` and `out` of one length.
#[inline(always)]
pub(in crate::encode::matrix) fn digits(t: &[u64], q: &[u32], out: &mut [[u8; 4]]) {
    assert!(t.len() == q.len() && t.len() == out.len());
    for ((o, &t), &q) in out.iter_mut().zip(t.iter().rev()).zip(q.iter().rev()) {
        *o = four_digits(column(t, q) as u64);
    }
}

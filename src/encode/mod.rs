//! Base58 (Bitcoin alphabet) encoding. See README.md for the design.
//!
//! - [`encode_batched`] (`batched.rs`): quadratic, one base-58^10 limb update per seven
//!   input bytes, in place in the output.
//! - [`encode`] (`dc.rs`): divide and conquer by powers of 58^10, each division Barrett's
//!   with a Newton reciprocal, over any `Multiplier`.
//! - [`encode_matrix`] (`matrix.rs`): a matrix of powers with the decoder's SIMD
//!   multiply-adds, up to 8 KiB, and a fixed-size path for 32 and 64 bytes
//!   (`matrix/fixed.rs`).

mod batched;
mod dc;
mod matrix;
#[cfg(test)]
mod tests;

pub use batched::encode_batched;
pub use dc::{Leaves, encode, encode_karatsuba_with};
pub use matrix::{encode_matrix, encode_matrix_portable, encode_matrix_with};
#[cfg(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon"))]
pub use matrix::fixed_sweep;
pub(crate) use dc::{encode_tuned, encode_with};

use crate::common::{ALPHABET, BASE};

/// `floor(log_58(2) * 2^64)`.
const LOG58_2: u128 = 3148993499475808360;

/// An upper bound on the encoded length of `len` bytes.
pub const fn max_encoded_len(len: usize) -> usize {
    // Leading zero bytes take one character each, fewer than any other byte, so `len`
    // bytes encode to at most the digits of 2^(8 len) - 1.
    ((8 * len as u128 * (LOG58_2 + 1)) >> 64) as usize + 1
}

/// A lower bound on the digits of a value of `bits` significant bits, at most two below
/// the actual count: the value is at least 2^(bits - 1), and below 2^bits, whose digit
/// count is at most one more.
fn min_digits(bits: u64) -> usize {
    // `LOG58_2` rounds down, which loses at most one digit while `bits < 2^64`.
    (((bits - 1) as u128 * LOG58_2) >> 64) as usize + 1
}

/// Significant bits of the big-endian `x`, whose first byte is not zero.
fn bits(x: &[u8]) -> u64 {
    8 * x.len() as u64 - x[0].leading_zeros() as u64
}

/// Splits off the leading zero bytes, each of which encodes to a '1'.
fn split_zeros(input: &[u8]) -> (usize, &[u8]) {
    let zeros = input.iter().take_while(|&&b| b == 0).count();
    (zeros, &input[zeros..])
}

/// The ten characters of a limb below [`BASE`], most significant first.
fn ten_digits(y: u64) -> [u8; 10] {
    debug_assert!(y < BASE);
    const HALF: u64 = 58u64.pow(5);
    let mut d = [0; 10];
    five_digits(&mut d[..5], (y / HALF) as u32);
    five_digits(&mut d[5..], (y % HALF) as u32);
    d
}

/// The five characters of `x < 58^5`.
fn five_digits(d: &mut [u8], mut x: u32) {
    for c in d.iter_mut().rev() {
        *c = ALPHABET[(x % 58) as usize];
        x /= 58;
    }
}

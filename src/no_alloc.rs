//! Base58 conversion without heap allocation, including first use and errors.
//!
//! The caller supplies output storage. Encoding retains the fixed 32/64-byte
//! kernels and their compile-time tables; other sizes use the batched encoder.
//! Decoding uses the tiny or batched decoder. These paths never initialize the
//! general matrix or NTT caches. General conversion takes quadratic time and
//! uses the output buffer as scratch. Timing depends on input values and length.

use crate::BufferTooSmall;
use crate::common::{DecodeError, split_zeros};

/// Encodes Bitcoin Base58 and returns the number of bytes written.
///
/// Leading zero bytes each produce a `'1'`. On success, bytes after the result
/// are untouched. On error, the contents of `output` are unspecified. No heap
/// storage is allocated, even on first use or insufficient output capacity.
///
/// # Errors
/// Returns [`BufferTooSmall`] if `output` cannot hold the encoded string.
#[inline]
pub fn encode(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    match input.len() {
        32 => crate::encode::matrix::fixed::encode_32(input.try_into().unwrap(), output),
        64 => crate::encode::matrix::fixed::encode_64(input.try_into().unwrap(), output),
        _ => crate::encode::encode_batched(input, output),
    }
}

/// Decodes Bitcoin Base58 and returns the number of bytes written.
///
/// Leading `'1'` characters each produce a zero byte. Invalid-character indices
/// are byte offsets. On success, bytes after the result are untouched. On error,
/// the contents of `output` are unspecified. When input is invalid and capacity
/// is insufficient, either error can be reported. No heap storage is allocated,
/// including first use and error paths.
///
/// # Errors
/// Returns [`DecodeError::InvalidCharacter`] for a byte outside the alphabet, or
/// [`DecodeError::BufferTooSmall`] if `output` cannot hold the decoded bytes.
#[inline]
pub(crate) fn decode(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    if input.len() <= 2 {
        crate::batched::decode_short(input, output)
    } else {
        let (zeros, _) = split_zeros(input);
        crate::batched::decode_parts(input, zeros, output)
    }
}

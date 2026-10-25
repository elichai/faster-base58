//! Base58 conversion without heap allocation, including first use and errors.
//!
//! The caller supplies output storage. Encoding retains the fixed 32/64-byte
//! kernels and their compile-time tables; other sizes use the batched encoder.
//! Decoding uses the tiny or batched decoder. These paths never initialize the
//! general matrix or NTT caches. General conversion takes quadratic time and
//! uses the output buffer as scratch. Timing depends on input values and length.

use crate::{BufferTooSmall, Error};

/// Encodes Bitcoin Base58 and returns the number of bytes written.
///
/// Leading zero bytes each produce a `'1'`. On success, bytes after the result
/// are untouched. On error, the contents of `output` are unspecified. No heap
/// storage is allocated, even on first use or insufficient output capacity.
#[inline]
pub fn encode(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    match crate::encode::encode_fixed(input, output) {
        Some(result) => result,
        None => crate::encode::encode_batched(input, output),
    }
}

/// Decodes Bitcoin Base58 and returns the number of bytes written.
///
/// Leading `'1'` characters each produce a zero byte. Invalid-character indices
/// are byte offsets. On success, bytes after the result are untouched. On error,
/// the contents of `output` are unspecified. When input is invalid and capacity
/// is insufficient, either error can be reported. No heap storage is allocated,
/// including first use and error paths.
#[inline]
pub fn decode(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    if input.len() <= 2 {
        crate::batched::decode_short(input, output)
    } else {
        crate::batched::decode_batched(input, output)
    }
}

/// Encodes into an array or slice and borrows the resulting ASCII string.
///
/// Uses [`encode`] with the same allocation and buffer contracts.
/// Borrow arrays and vectors with `&input`; byte slices can be passed directly.
/// Input storage stays with the caller, so this function never drops owned input.
/// `crate::max_encoded_len(input.len())` bytes suffice for an actual input slice.
pub fn encode_into<'a>(input: &[u8], output: &'a mut [u8]) -> Result<&'a str, BufferTooSmall> {
    let len = encode(input, output)?;
    Ok(std::str::from_utf8(&output[..len]).expect("base58 output is ASCII"))
}

/// Decodes into an array or slice and borrows the bytes written.
///
/// Uses [`decode`] with the same allocation and buffer contracts.
pub fn decode_into<'a>(input: &str, output: &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    let len = decode(input, output)?;
    Ok(&mut output[..len])
}

/// Decodes exactly `N` bytes into an array, without adding or removing zeros.
///
/// The result uses `N` bytes of stack storage, with no heap allocation. A shorter
/// result returns [`Error::InvalidLength`]; a result longer than `N` returns
/// [`Error::BufferTooSmall`]. Invalid input has the same error contract as [`decode`].
pub fn decode_array<const N: usize>(input: &str) -> Result<[u8; N], Error> {
    let mut output = [0; N];
    let actual = decode(input, &mut output)?;
    if actual != N {
        return Err(Error::InvalidLength {
            expected: N,
            actual,
        });
    }
    Ok(output)
}

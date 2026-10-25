//! Automatic dispatch and output conveniences. Process-wide caches are opt-in.

use crate::batched::decode_batched;
use crate::{BufferTooSmall, Error};

/// Encodes Bitcoin Base58 and returns the number of bytes written to `output`.
///
/// Without `cache`, fixed 32/64-byte inputs use built-in tables; other inputs use
/// batched or divide-and-conquer encoding with owned NTT tables for large products.
/// With `cache`, general matrix tables and bounded-size NTT tables are retained for
/// the process lifetime; larger NTT tables live for one conversion.
/// On success, the output tail is untouched. On error, its contents are unspecified.
/// Timing depends on the input. Use [`encode_into`] to borrow the resulting string.
#[inline]
pub fn encode(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    if input.len() < 22 {
        return crate::encode::encode_batched(input, output);
    }
    #[cfg(feature = "cache")]
    {
        crate::encode::matrix::entry::encode_matrix(input, output)
    }
    #[cfg(not(feature = "cache"))]
    {
        if let Some(result) = crate::encode::encode_fixed(input, output) {
            return result;
        }
        // Conservative crossover based on historical kernel measurements:
        // Karatsuba/Horner in the medium range, owned NTT above it. This is not
        // a claim that the crossover is optimal on every target.
        if input.len() <= crate::ntt::ENCODE_THRESHOLD {
            crate::encode::dc::karatsuba::encode(input, output)
        } else {
            crate::ntt::encode(input, output)
        }
    }
}

/// Decodes Bitcoin Base58 and returns the number of bytes written to `output`.
///
/// Without `cache`, small inputs use batched decoding, medium inputs use divide and
/// conquer, and large inputs use owned NTT tables. With `cache`, medium inputs use
/// the matrix kernel on selected targets and large inputs use bounded NTT caching.
///
/// Leading `'1'` characters each produce a zero byte. Invalid-character indices are
/// byte offsets. On success, bytes after the returned length are untouched. On error,
/// the contents of `output` are unspecified. The algorithms have data-dependent timing.
/// Process-wide table retention requires `cache`; larger NTT tables always live for
/// one conversion. See the review for the measured feature-enabled routes.
pub fn decode(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    #[cfg(not(feature = "cache"))]
    {
        if input.len() <= 2 {
            return crate::batched::decode_short(input, output);
        }
        if input.len() <= crate::dc::DC_THRESHOLD * crate::common::CHUNK {
            return decode_batched(input, output);
        }
        // Preserve the former medium/large boundary while avoiding matrix setup.
        // The historical measurements motivate this conservative boundary;
        // measurements of the complete dispatch do not establish optimality.
        if input.len() <= 5632 {
            return crate::dc::decode(input, output);
        }
        crate::ntt::decode(input, output)
    }
    #[cfg(feature = "cache")]
    {
        let matrix_min = if cfg!(all(
            target_arch = "aarch64",
            target_endian = "little",
            target_feature = "neon"
        )) || cfg!(target_arch = "wasm32")
        {
            576
        } else {
            1024
        };
        if input.len() < matrix_min {
            if input.len() <= 2 {
                return crate::batched::decode_short(input, output);
            }
            return decode_batched(input, output);
        }
        if input.len() <= 5632 {
            #[cfg(target_arch = "x86_64")]
            if crate::mul::ifma_threshold() != 0 {
                if input.len() > 5120 {
                    return crate::ntt::decode(input, output);
                }
                // Native AVX2 builds make Horner faster than matrix on the measured Zen 5.
                if cfg!(target_feature = "avx2") {
                    return decode_batched(input, output);
                }
            }
            if cfg!(all(
                target_arch = "aarch64",
                target_endian = "little",
                target_feature = "neon"
            )) || cfg!(target_arch = "wasm32")
            {
                return crate::matrix::entry::decode_matrix(input, output);
            }
            #[cfg(target_arch = "x86_64")]
            if std::is_x86_feature_detected!("sse4.1") {
                return crate::matrix::entry::decode_matrix(input, output);
            }
            return decode_batched(input, output);
        }
        crate::ntt::decode(input, output)
    }
}

/// Encodes into an array or slice and borrows the resulting ASCII string.
///
/// `max_encoded_len(input.len())` bytes always suffice for an actual input slice.
/// The output tail is untouched on success; its contents are unspecified on error.
/// This uses the same automatic dispatch and optional caches as [`crate::encode`].
///
/// ```
/// let input = [42; 32];
/// let mut buffer = [0; 44];
/// let text = base58_bench::encode_into(input, &mut buffer)?;
/// assert_eq!(base58_bench::decode_array::<32>(text)?, input);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn encode_into(input: impl AsRef<[u8]>, output: &mut [u8]) -> Result<&str, BufferTooSmall> {
    let len = encode(input.as_ref(), output)?;
    Ok(std::str::from_utf8(&output[..len]).expect("base58 output is ASCII"))
}

/// Encodes an array or byte slice into an owned string.
pub fn encode_string(input: impl AsRef<[u8]>) -> String {
    let input = input.as_ref();
    let mut output = vec![0; crate::max_encoded_len(input.len())];
    let len = encode(input, &mut output).expect("max_encoded_len bounds the result");
    output.truncate(len);
    String::from_utf8(output).expect("base58 output is ASCII")
}

/// Decodes into an array or slice and borrows the bytes written.
///
/// The output tail is untouched on success; its contents are unspecified on error.
pub fn decode_into<'a>(input: &str, output: &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    let len = decode(input, output)?;
    Ok(&mut output[..len])
}

/// Decodes into an owned byte vector, preserving leading zero bytes.
pub fn decode_vec(input: &str) -> Result<Vec<u8>, Error> {
    let mut output = vec![0; input.len()];
    let len = decode(input, &mut output)?;
    output.truncate(len);
    Ok(output)
}

/// Decodes exactly `N` bytes, without adding or removing leading zeros.
///
/// A shorter result returns [`Error::InvalidLength`]; a result longer than `N` returns
/// [`Error::BufferTooSmall`]. Invalid input can report either invalid-character or
/// insufficient-buffer errors when both conditions apply, as with [`decode`].
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

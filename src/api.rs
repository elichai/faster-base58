//! Automatic dispatch and output conveniences. Process-wide caches are opt-in.

use alloc::{string::String, vec::Vec};

use crate::{BufferTooSmall, Error};

/// Encodes Bitcoin Base58 and returns the number of bytes written to `output`.
///
/// Without `cache`, fixed 32/64-byte inputs use built-in tables; other inputs use
/// batched or divide-and-conquer encoding with owned NTT tables for large products.
/// With `cache`, general matrix tables and bounded-size NTT tables are retained for
/// the process lifetime; larger NTT tables live for one conversion.
/// On success, the output tail is untouched. On error, its contents are unspecified.
/// Timing depends on the input. Use [`encode_into`] to borrow the resulting string.
///
/// # Errors
/// Returns [`BufferTooSmall`] if `output` cannot hold the encoded string.
#[inline]
pub fn encode(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    match input.len() {
        32 => return crate::encode::matrix::fixed::encode_32(input.try_into().unwrap(), output),
        64 => return crate::encode::matrix::fixed::encode_64(input.try_into().unwrap(), output),
        0..22 => return crate::encode::encode_batched(input, output),
        _ => {}
    }
    let (zeros, x) = crate::encode::split_zeros(input);
    #[cfg(feature = "cache")]
    {
        use crate::encode::matrix::entry::{IFMA_CAP, MATRIX_MIN, MAX_INPUT};
        if x.len() < MATRIX_MIN {
            return crate::encode::batched::encode_parts(zeros, x, output);
        }
        let cap = if crate::mul::ifma_threshold() != 0 {
            IFMA_CAP
        } else {
            MAX_INPUT
        };
        if x.len() <= cap {
            return crate::encode::matrix::entry::encode(zeros, x, output);
        }
    }
    #[cfg(not(feature = "cache"))]
    {
        // Conservative crossover based on historical kernel measurements:
        // Karatsuba/Horner in the medium range, owned NTT above it. This is not
        // a claim that the crossover is optimal on every target.
        if x.len() <= crate::ntt::ENCODE_THRESHOLD {
            let (threshold, leaf) = crate::encode::dc::karatsuba::params();
            if x.len() > threshold {
                return crate::encode::dc::encode_parts(
                    zeros,
                    x,
                    output,
                    &mut crate::dc::Karatsuba,
                    leaf,
                    crate::encode::Leaves::Horner,
                );
            }
            return crate::encode::batched::encode_parts(zeros, x, output);
        }
    }
    crate::ntt::encode(zeros, x, output)
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
///
/// # Errors
/// Returns [`Error::InvalidCharacter`] for a byte outside the alphabet, or
/// [`Error::BufferTooSmall`] if `output` cannot hold the decoded bytes.
pub fn decode(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    if input.len() <= 2 {
        return crate::batched::decode_short(input, output);
    }
    let (zeros, _) = crate::common::split_zeros(input.as_bytes());
    decode_parts(input, zeros, output)
}

fn decode_parts(input: &str, zeros: usize, output: &mut [u8]) -> Result<usize, Error> {
    let digits = input.len() - zeros;
    #[cfg(not(feature = "cache"))]
    {
        if digits <= crate::dc::DC_THRESHOLD * crate::common::CHUNK {
            return crate::batched::decode_parts(input, zeros, output);
        }
        // Preserve the former medium/large boundary while avoiding matrix setup.
        // The historical measurements motivate this conservative boundary;
        // measurements of the complete dispatch do not establish optimality.
        if digits <= 5632 {
            return crate::dc::decode_parts(
                input,
                zeros,
                output,
                &mut crate::dc::Karatsuba,
                crate::dc::Leaf::Horner,
            );
        }
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
        if digits < matrix_min {
            return crate::batched::decode_parts(input, zeros, output);
        }
        if digits <= 5632 {
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            if crate::mul::ifma_threshold() != 0 {
                if digits > 5120 {
                    return crate::ntt::decode(input, zeros, output);
                }
                // Native AVX2 builds make Horner faster than matrix on the measured Zen 5.
                if cfg!(target_feature = "avx2") {
                    return crate::batched::decode_parts(input, zeros, output);
                }
            }
            if cfg!(all(
                target_arch = "aarch64",
                target_endian = "little",
                target_feature = "neon"
            )) || cfg!(target_arch = "wasm32")
            {
                return crate::matrix::entry::decode(input, zeros, output);
            }
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            if crate::cpu::sse41() {
                return crate::matrix::entry::decode(input, zeros, output);
            }
            return crate::batched::decode_parts(input, zeros, output);
        }
    }
    crate::ntt::decode(input, zeros, output)
}

/// Encodes into an array or slice and borrows the resulting ASCII string.
///
/// `max_encoded_len(input.len())` bytes always suffice for an actual input slice.
/// The output tail is untouched on success; its contents are unspecified on error.
/// This uses the same automatic dispatch and optional caches as [`crate::encode`].
///
/// # Errors
/// Returns [`BufferTooSmall`] if `output` cannot hold the encoded string.
///
/// ```
/// let input = [42; 32];
/// let mut buffer = [0; 44];
/// let text = faster_base58::encode_into(input, &mut buffer)?;
/// assert_eq!(faster_base58::decode_array::<32>(text)?, input);
/// # Ok::<(), Box<dyn core::error::Error>>(())
/// ```
pub fn encode_into(input: impl AsRef<[u8]>, output: &mut [u8]) -> Result<&str, BufferTooSmall> {
    let len = encode(input.as_ref(), output)?;
    Ok(core::str::from_utf8(&output[..len]).expect("base58 output is ASCII"))
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
///
/// # Errors
/// Returns the same invalid-character and insufficient-buffer errors as [`decode`].
pub fn decode_into<'a>(input: &str, output: &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    let len = decode(input, output)?;
    Ok(&mut output[..len])
}

/// Decodes into an owned byte vector, preserving leading zero bytes.
///
/// # Errors
/// Returns [`Error::InvalidCharacter`] for a byte outside the alphabet.
#[inline]
pub fn decode_vec(input: &str) -> Result<Vec<u8>, Error> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    decode_vec_nonempty(input)
}

/// Requires nonempty input. Keep the nonempty decoder's frame off the empty path.
#[inline(never)]
fn decode_vec_nonempty(input: &str) -> Result<Vec<u8>, Error> {
    debug_assert!(!input.is_empty());
    if input.len() <= 2 {
        let mut output = vec![0; input.len()];
        let len = decode(input, &mut output)?;
        output.truncate(len);
        return Ok(output);
    }
    let (zeros, digits) = crate::common::split_zeros(input.as_bytes());
    let mut output = vec![0; zeros + decoded_capacity(digits.len())];
    let len = decode_parts(input, zeros, &mut output)?;
    output.truncate(len);
    Ok(output)
}

fn decoded_capacity(digits: usize) -> usize {
    // 58^64 < 256^47; separate floors keep this above 47*digits/64 without overflow.
    digits - digits / 4 - digits / 64
}

/// Decodes exactly `N` bytes, without adding or removing leading zeros.
///
/// # Errors
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

#[cfg(kani)]
mod capacity_proofs {
    use super::decoded_capacity;

    #[kani::proof]
    fn decoded_capacity_bounds_every_usize() {
        let digits: usize = kani::any();
        let bound = decoded_capacity(digits);
        assert!(bound <= digits);
        assert!(bound as u128 * 64 >= digits as u128 * 47);
    }

    #[kani::proof]
    fn decoded_capacity_is_within_one_byte_of_ceil47() {
        let digits: usize = kani::any();
        let bound = decoded_capacity(digits);
        let ceil = (digits as u128 * 47).div_ceil(64);
        assert!(bound as u128 <= ceil + 1);
    }

    #[kani::proof]
    fn decoded_capacity_prefix_sum_never_overflows() {
        let digits: usize = kani::any();
        let zeros: usize = kani::any();
        kani::assume(zeros <= usize::MAX - digits);
        let capacity = zeros + decoded_capacity(digits);
        assert!(capacity <= zeros + digits);
    }
}

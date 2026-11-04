//! Automatic dispatch and output conveniences. Process-wide caches are opt-in.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "alloc")]
use crate::{BufferTooSmall, Error};

#[inline]
#[cfg(feature = "alloc")]
pub fn encode(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    match input.len() {
        32 => return crate::encode::matrix::fixed::encode_32(input.try_into().unwrap(), output),
        64 => return crate::encode::matrix::fixed::encode_64(input.try_into().unwrap(), output),
        0..22 => return crate::encode::encode_batched(input, output),
        _ => {}
    }
    let (zeros, x) = crate::encode::split_zeros(input);
    match encode_backend(x.len()) {
        EncodeBackend::Batched => crate::encode::batched::encode_parts(zeros, x, output),
        #[cfg(not(feature = "cache"))]
        EncodeBackend::Karatsuba { leaf } => crate::encode::dc::encode_parts(
            zeros,
            x,
            output,
            &mut crate::dc::Karatsuba,
            leaf,
            crate::encode::Leaves::Horner,
        ),
        #[cfg(feature = "cache")]
        EncodeBackend::Matrix => crate::encode::matrix::entry::encode(zeros, x, output),
        EncodeBackend::Ntt => crate::ntt::encode(zeros, x, output),
    }
}

#[cfg(feature = "alloc")]
pub(crate) enum EncodeBackend {
    Batched,
    #[cfg(all(feature = "alloc", not(feature = "cache")))]
    Karatsuba {
        leaf: usize,
    },
    #[cfg(feature = "cache")]
    Matrix,
    #[cfg(feature = "alloc")]
    Ntt,
}

#[cfg(feature = "alloc")]
#[inline(always)]
pub(crate) fn encode_backend(bytes: usize) -> EncodeBackend {
    #[cfg(not(feature = "alloc"))]
    {
        let _ = bytes;
        EncodeBackend::Batched
    }
    #[cfg(feature = "alloc")]
    {
        #[cfg(feature = "cache")]
        {
            use crate::encode::matrix::entry::{IFMA_CAP, MATRIX_MIN, MAX_INPUT};
            if bytes < MATRIX_MIN {
                return EncodeBackend::Batched;
            }
            let cap = if crate::mul::ifma_threshold() != 0 {
                IFMA_CAP
            } else {
                MAX_INPUT
            };
            if bytes <= cap {
                return EncodeBackend::Matrix;
            }
        }
        #[cfg(not(feature = "cache"))]
        if bytes <= crate::ntt::ENCODE_THRESHOLD {
            let (threshold, leaf) = crate::encode::dc::karatsuba::params();
            return if bytes > threshold {
                EncodeBackend::Karatsuba { leaf }
            } else {
                EncodeBackend::Batched
            };
        }
        EncodeBackend::Ntt
    }
}

#[cfg(feature = "alloc")]
pub fn decode(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    if input.len() <= 2 {
        return crate::batched::decode_short(input, output);
    }
    let (zeros, _) = crate::common::split_zeros(input.as_bytes());
    decode_parts(input, zeros, output)
}

#[cfg(feature = "alloc")]
pub(crate) fn decode_parts(input: &str, zeros: usize, output: &mut [u8]) -> Result<usize, Error> {
    let digits = input.len() - zeros;
    if digits <= decode_small_limit() {
        return crate::batched::decode_parts(input, zeros, output);
    }
    match decode_backend(digits) {
        DecodeBackend::Batched => crate::batched::decode_parts(input, zeros, output),
        #[cfg(not(feature = "cache"))]
        DecodeBackend::Karatsuba => crate::dc::decode_parts(
            input,
            zeros,
            output,
            &mut crate::dc::Karatsuba,
            crate::dc::Leaf::Horner,
        ),
        #[cfg(feature = "cache")]
        DecodeBackend::Matrix => crate::matrix::entry::decode(input, zeros, output),
        DecodeBackend::Ntt => crate::ntt::decode(input, zeros, output),
    }
}

#[cfg(feature = "alloc")]
pub(crate) enum DecodeBackend {
    Batched,
    #[cfg(all(feature = "alloc", not(feature = "cache")))]
    Karatsuba,
    #[cfg(feature = "cache")]
    Matrix,
    #[cfg(feature = "alloc")]
    Ntt,
}

#[cfg(feature = "alloc")]
#[inline(always)]
fn decode_small_limit() -> usize {
    #[cfg(not(feature = "alloc"))]
    {
        usize::MAX
    }
    #[cfg(all(feature = "alloc", not(feature = "cache")))]
    {
        crate::dc::DC_THRESHOLD * crate::common::CHUNK
    }
    #[cfg(feature = "cache")]
    {
        if cfg!(all(
            target_arch = "aarch64",
            target_endian = "little",
            target_feature = "neon"
        )) || cfg!(target_arch = "wasm32")
        {
            575
        } else {
            1023
        }
    }
}

#[cfg(feature = "alloc")]
#[inline(always)]
pub(crate) fn decode_backend(digits: usize) -> DecodeBackend {
    if digits <= decode_small_limit() {
        return DecodeBackend::Batched;
    }
    #[cfg(not(feature = "alloc"))]
    {
        let _ = digits;
        DecodeBackend::Batched
    }
    #[cfg(feature = "alloc")]
    {
        #[cfg(not(feature = "cache"))]
        {
            if digits <= 5632 {
                return DecodeBackend::Karatsuba;
            }
            DecodeBackend::Ntt
        }
        #[cfg(feature = "cache")]
        {
            if digits > 5632 {
                return DecodeBackend::Ntt;
            }
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            if crate::mul::ifma_threshold() != 0 {
                if digits > 5120 {
                    return DecodeBackend::Ntt;
                }
                if cfg!(target_feature = "avx2") {
                    return DecodeBackend::Batched;
                }
            }
            if cfg!(all(
                target_arch = "aarch64",
                target_endian = "little",
                target_feature = "neon"
            )) || cfg!(target_arch = "wasm32")
            {
                return DecodeBackend::Matrix;
            }
            #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
            if crate::cpu::sse41() {
                return DecodeBackend::Matrix;
            }
            DecodeBackend::Batched
        }
    }
}

#[cfg(not(feature = "alloc"))]
pub(crate) use crate::no_alloc::{decode, encode};

#[inline]
#[cfg(feature = "alloc")]
pub fn decode_vec(input: &str) -> Result<Vec<u8>, Error> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    decode_vec_nonempty(input)
}

#[inline(never)]
#[cfg(feature = "alloc")]
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
    let len = decode_parts(input, zeros, output.as_mut_slice())?;
    output.truncate(len);
    Ok(output)
}

#[cfg(any(feature = "alloc", kani))]
fn decoded_capacity(digits: usize) -> usize {
    // 58^64 < 256^47; separate floors keep this above 47*digits/64 without overflow.
    digits - digits / 4 - digits / 64
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

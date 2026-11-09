//! Automatic dispatch and output conveniences. Process-wide caches are opt-in.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "alloc")]
use crate::{BufferTooSmall, common::DecodeError};

#[inline]
#[cfg(feature = "alloc")]
pub(crate) fn encode(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
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

/// Backend for `bytes` significant input bytes. Shared by plain and checked encoding.
#[cfg(any(feature = "alloc", feature = "check"))]
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

#[cfg(any(feature = "alloc", feature = "check"))]
#[inline(always)]
pub(crate) fn encode_backend(bytes: usize) -> EncodeBackend {
    core::cfg_select! {
        not(feature = "alloc") => {
            let _ = bytes;
            EncodeBackend::Batched
        }
        feature = "cache" => {
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
                EncodeBackend::Matrix
            } else {
                EncodeBackend::Ntt
            }
        }
        _ => {
            if bytes > crate::ntt::ENCODE_THRESHOLD {
                return EncodeBackend::Ntt;
            }
            let (threshold, leaf) = crate::encode::dc::karatsuba::params();
            if bytes > threshold {
                EncodeBackend::Karatsuba { leaf }
            } else {
                EncodeBackend::Batched
            }
        }
    }
}

#[cfg(feature = "alloc")]
pub(crate) fn decode(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    if input.len() <= 2 {
        return crate::batched::decode_short(input, output);
    }
    let (zeros, _) = crate::common::split_zeros(input);
    decode_parts(input, zeros, output)
}

#[cfg(feature = "alloc")]
pub(crate) fn decode_parts(
    input: &[u8],
    zeros: usize,
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    // One selection, so that the batched decoder has one call site: a second CPU check
    // left two, and LLVM stopped inlining it (8-32 byte decodes 7-16% slower on Zen 5).
    match decode_backend(input.len() - zeros) {
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

/// Backend for `digits` significant characters. Shared by plain and checked decoding.
#[cfg(any(feature = "alloc", feature = "check"))]
pub(crate) enum DecodeBackend {
    Batched,
    #[cfg(all(feature = "alloc", not(feature = "cache")))]
    Karatsuba,
    #[cfg(feature = "cache")]
    Matrix,
    #[cfg(feature = "alloc")]
    Ntt,
}

/// Whether `digits` significant characters use the batched decoder.
#[cfg(any(feature = "alloc", feature = "check"))]
#[inline(always)]
fn decode_is_small(digits: usize) -> bool {
    core::cfg_select! {
        not(feature = "alloc") => {
            let _ = digits;
            true
        }
        feature = "cache" => {
            digits <= if cfg!(all(
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
        // With short leaves (Wasm, IFMA) the tree wins from four leaves: Karatsuba took
        // 1.07x the batched time at 1224 digits and 0.85x at 1311 on Zen 5, and the NTT
        // decoder 1.07x at 2447 digits and 0.94-0.97x at 2622 under V8. On M5 Karatsuba
        // took 1.14-1.22x at 6.6-8.4 thousand digits, and the NTT decoder 1.01x at 7167
        // digits and 0.97x at 7342.
        _ => {
            if cfg!(target_arch = "wasm32") {
                digits <= 2 * crate::dc::min_digits()
            } else if cfg!(all(
                target_arch = "aarch64",
                target_endian = "little",
                target_feature = "neon"
            )) {
                digits <= 7250
            } else {
                // The IFMA limit first, so that shorter inputs skip CPU detection.
                digits <= 2 * crate::dc::IFMA_LEAF * crate::common::CHUNK
                    || (digits <= 5120 && crate::mul::ifma_threshold() == 0)
            }
        }
    }
}

#[cfg(any(feature = "alloc", feature = "check"))]
#[inline(always)]
pub(crate) fn decode_backend(digits: usize) -> DecodeBackend {
    if decode_is_small(digits) {
        return DecodeBackend::Batched;
    }
    core::cfg_select! {
        // Without `alloc` the small limit covers every length.
        not(feature = "alloc") => { DecodeBackend::Batched }
        feature = "cache" => {
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
        // Under V8 with SIMD128 the NTT decoder took 0.56-0.83x Karatsuba's time from 3496
        // digits. Scalar Wasm Karatsuba was up to 7% faster at 3496-4195 digits, the NTT
        // decoder from 4545 (0.95x at 5594).
        _ => {
            if digits <= 5632 && !cfg!(target_arch = "wasm32") {
                DecodeBackend::Karatsuba
            } else {
                DecodeBackend::Ntt
            }
        }
    }
}

#[cfg(not(feature = "alloc"))]
pub(crate) use crate::no_alloc::{decode, encode};

#[inline]
#[cfg(feature = "alloc")]
pub(crate) fn decode_vec(input: &[u8]) -> Result<Vec<u8>, DecodeError> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    decode_vec_nonempty(input)
}

#[cfg(feature = "alloc")]
fn decode_vec_nonempty(input: &[u8]) -> Result<Vec<u8>, DecodeError> {
    debug_assert!(!input.is_empty());
    if input.len() <= 2 {
        let mut output = vec![0; input.len()];
        let len = decode(input, &mut output)?;
        output.truncate(len);
        return Ok(output);
    }
    let (zeros, digits) = crate::common::split_zeros(input);
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

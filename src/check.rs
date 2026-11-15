//! Bitcoin `Base58Check` framing; only compiled with the `check` feature.
//!
//! Conversion backends report [`DecodeError`]; only this layer adds the checksum and
//! version errors of the public [`Error`].

use sha2::{Digest, Sha256};

use crate::api::{DecodeBackend, EncodeBackend};
use crate::common::{DecodeError, split_zeros};
use crate::{BufferTooSmall, Error};

fn checksum(data: &[u8], version: Option<u8>) -> [u8; 4] {
    let mut first = Sha256::new();
    first.update(version.as_slice());
    first.update(data);
    let [a, b, c, d, ..]: [u8; 32] = Sha256::digest(first.finalize()).into();
    [a, b, c, d]
}

#[inline]
pub(crate) fn encode(
    input: &[u8],
    output: &mut [u8],
    version: Option<u8>,
) -> Result<usize, BufferTooSmall> {
    // Each frame byte takes at least one character, so the frame fits in `output`.
    let total = input.len() + 4 + usize::from(version.is_some());
    if output.len() < total {
        return Err(BufferTooSmall);
    }
    let sum = checksum(input, version);
    let parts = [version.as_slice(), input, sum.as_slice()];
    let stage = |output: &mut [u8]| {
        let mut pos = 0;
        for part in parts {
            output[pos..pos + part.len()].copy_from_slice(part);
            pos += part.len();
        }
    };
    // Preserve the fixed-size kernels, copying only their small input on the stack.
    if total == 32 {
        stage(output);
        let bytes: [u8; 32] = *output.first_chunk().unwrap();
        return crate::api::encode(&bytes, output);
    }
    if total == 64 {
        stage(output);
        let bytes: [u8; 64] = *output.first_chunk().unwrap();
        return crate::api::encode(&bytes, output);
    }
    let first = parts
        .iter()
        .flat_map(|part| part.iter())
        .enumerate()
        .find(|&(_, &b)| b != 0);
    let (zeros, bits) = match first {
        Some((zeros, first)) => (
            zeros,
            8 * (total - zeros) as u64 - first.leading_zeros() as u64,
        ),
        None => (total, 0),
    };
    match crate::api::encode_backend(total - zeros) {
        EncodeBackend::Batched => {
            crate::encode::batched::encode_segments(zeros, bits, &parts, output)
        }
        #[cfg(feature = "cache")]
        EncodeBackend::Matrix => {
            stage(output);
            crate::encode::matrix::entry::encode_staged(zeros, total, output)
        }
        #[cfg(all(feature = "alloc", not(feature = "cache")))]
        EncodeBackend::Karatsuba { leaf } => {
            stage(output);
            crate::encode::dc::encode_staged(
                zeros,
                total,
                output,
                &mut crate::dc::Karatsuba,
                leaf,
                crate::encode::Leaves::Horner,
            )
        }
        #[cfg(feature = "alloc")]
        EncodeBackend::Ntt => {
            stage(output);
            crate::ntt::encode_staged(zeros, total, output)
        }
    }
}

/// Splits a decoded frame into its data and checksum, then validates them.
#[inline]
#[cfg(feature = "alloc")]
pub(crate) fn verify(frame: &[u8], version: Option<u8>) -> Result<usize, Error> {
    let (data, sum) = frame.split_last_chunk().ok_or(Error::MissingChecksum)?;
    validate(data, *sum, version)?;
    Ok(data.len())
}

/// Checks the checksum before the version.
fn validate(data: &[u8], sum: [u8; 4], version: Option<u8>) -> Result<(), Error> {
    if checksum(data, None) != sum {
        return Err(Error::InvalidChecksum);
    }
    if let Some(expected) = version {
        let actual = data.first().copied();
        if actual != Some(expected) {
            return Err(Error::InvalidVersion { expected, actual });
        }
    }
    Ok(())
}

#[inline(always)]
pub(crate) fn decode(input: &[u8], output: &mut [u8], version: Option<u8>) -> Result<usize, Error> {
    let (_, digits) = split_zeros(input);
    let (frame, sum) = decode_frame(input, digits, output)?;
    let len = frame.checked_sub(4).ok_or(Error::MissingChecksum)?;
    validate(&output[..len], sum, version)?;
    Ok(len)
}

/// Decodes all but the last four frame bytes of `input`, whose significant characters
/// are its suffix `digits`, into `output`. Returns the frame length
/// and those four bytes, which `output` need not hold. Backends report only
/// [`DecodeError`]; checksum and version errors belong to [`decode`]. Inlined for the
/// reason of `api::decode_parts`.
#[inline(always)]
fn decode_frame(
    input: &[u8],
    digits: &[u8],
    output: &mut [u8],
) -> Result<(usize, [u8; 4]), DecodeError> {
    // Only the tree backends take the zero count.
    #[cfg(feature = "alloc")]
    let zeros = input.len() - digits.len();
    match crate::api::decode_backend(digits.len()) {
        DecodeBackend::Batched => crate::batched::decode_checked_parts(input, digits, output),
        #[cfg(all(
            feature = "alloc",
            not(feature = "cache"),
            not(target_arch = "wasm32"),
            not(all(
                target_arch = "aarch64",
                target_endian = "little",
                target_feature = "neon"
            ))
        ))]
        DecodeBackend::Karatsuba => {
            let limbs = crate::dc::decode_value(
                digits,
                zeros,
                output.len().saturating_add(4),
                &mut crate::dc::Karatsuba,
                crate::dc::Leaf::Horner,
            )?;
            Ok(split(zeros, &limbs, output)?)
        }
        #[cfg(feature = "cache")]
        DecodeBackend::Matrix => crate::matrix::entry::decode_checked(input, zeros, output),
        #[cfg(feature = "alloc")]
        DecodeBackend::Ntt => {
            let limbs = crate::ntt::decode_value(digits, zeros, output.len().saturating_add(4))?;
            Ok(split(zeros, &limbs, output)?)
        }
    }
}

/// Writes `zeros` zero bytes, then the trimmed little-endian `limbs` as big-endian
/// bytes, except the last four. Returns the frame length and those four bytes.
#[cfg(feature = "alloc")]
pub(crate) fn split(
    zeros: usize,
    limbs: &[u64],
    output: &mut [u8],
) -> Result<(usize, [u8; 4]), BufferTooSmall> {
    let numeric = limbs
        .last()
        .map_or(0, |top| 8 * limbs.len() - top.leading_zeros() as usize / 8);
    let frame = zeros + numeric;
    let data = output
        .get_mut(..frame.saturating_sub(4))
        .ok_or(BufferTooSmall)?;
    let high_len = numeric.saturating_sub(4);
    let (leading, high) = data.split_at_mut(data.len() - high_len);
    leading.fill(0);
    // `value >> 32` as little-endian words, stored in full 8-byte chunks.
    let (top, body) = high.as_rchunks_mut::<8>();
    for (chunk, pair) in body.iter_mut().rev().zip(limbs.windows(2)) {
        *chunk = ((pair[0] >> 32) | (pair[1] << 32)).to_be_bytes();
    }
    if !top.is_empty() {
        // `high` has at most `8 limbs.len() - 4` bytes, so `body.len() < limbs.len()`.
        let word = (limbs[body.len()] >> 32) | limbs.get(body.len() + 1).map_or(0, |&l| l << 32);
        // The low `top.len()` (1 to 7) bytes of `word`, most significant first.
        crate::common::store_low_le(top, word.swap_bytes() >> (64 - 8 * top.len()));
    }
    let sum = (limbs.first().copied().unwrap_or(0) as u32).to_be_bytes();
    Ok((frame, sum))
}

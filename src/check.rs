//! Bitcoin `Base58Check` framing; only compiled with the `check` feature.

use sha2::{Digest, Sha256};

use crate::common::DecodeError;
use crate::{BufferTooSmall, Error};

fn checksum(data: &[u8], version: Option<u8>) -> [u8; 4] {
    let mut first = Sha256::new();
    first.update(version.as_slice());
    first.update(data);
    let hash = Sha256::digest(first.finalize());
    hash[..4]
        .try_into()
        .expect("SHA-256 has at least four bytes")
}

#[inline(always)]
pub(crate) fn encode(
    input: &[u8],
    output: &mut [u8],
    version: Option<u8>,
) -> Result<usize, BufferTooSmall> {
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
        let bytes: [u8; 32] = output[..32].try_into().unwrap();
        return crate::api::encode(&bytes, output);
    }
    if total == 64 {
        stage(output);
        let bytes: [u8; 64] = output[..64].try_into().unwrap();
        return crate::api::encode(&bytes, output);
    }
    let zeros = parts
        .iter()
        .flat_map(|part| part.iter())
        .take_while(|&&b| b == 0)
        .count();
    let significant = total - zeros;
    let bits = if significant == 0 {
        0
    } else {
        let first = parts
            .iter()
            .flat_map(|part| part.iter())
            .nth(zeros)
            .unwrap();
        8 * significant as u64 - first.leading_zeros() as u64
    };
    use crate::api::EncodeBackend;
    match crate::api::encode_backend(significant) {
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

#[inline(always)]
#[cfg(feature = "alloc")]
pub(crate) fn verify(data: &[u8], version: Option<u8>) -> Result<usize, Error> {
    let len = data.len().checked_sub(4).ok_or(Error::MissingChecksum)?;
    validate(&data[..len], data[len..].try_into().unwrap(), version)?;
    Ok(len)
}

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
    let (zeros, _) = crate::common::split_zeros(input);
    let (frame, sum) = decode_frame(input, zeros, output)?;
    let len = frame.checked_sub(4).ok_or(Error::MissingChecksum)?;
    validate(&output[..len], sum, version)?;
    Ok(len)
}

/// Decodes all but the last four frame bytes into `output`. Returns the frame length
/// and those four bytes, which `output` need not hold. Backends report only
/// [`DecodeError`]; checksum and version errors belong to [`decode`].
fn decode_frame(
    input: &[u8],
    zeros: usize,
    output: &mut [u8],
) -> Result<(usize, [u8; 4]), DecodeError> {
    use crate::api::DecodeBackend;
    match crate::api::decode_backend(input.len() - zeros) {
        DecodeBackend::Batched => crate::batched::decode_checked_parts(input, zeros, output),
        #[cfg(all(feature = "alloc", not(feature = "cache")))]
        DecodeBackend::Karatsuba => {
            let limbs = crate::dc::decode_value(
                input,
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
            let limbs = crate::ntt::decode_value(input, zeros, output.len().saturating_add(4))?;
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
    let numeric_len = limbs
        .last()
        .map_or(0, |top| 8 * limbs.len() - top.leading_zeros() as usize / 8);
    let frame = zeros + numeric_len;
    let len = frame.saturating_sub(4);
    let data = output.get_mut(..len).ok_or(BufferTooSmall)?;
    let high_len = numeric_len.saturating_sub(4);
    let leading = len - high_len;
    data[..leading].fill(0);
    for (i, byte) in data[leading..].iter_mut().rev().enumerate() {
        let index = i + 4;
        *byte = (limbs[index / 8] >> (8 * (index % 8))) as u8;
    }
    let sum = (limbs.first().copied().unwrap_or(0) as u32).to_be_bytes();
    Ok((frame, sum))
}

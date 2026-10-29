//! Caller-buffer adapters for the separately versioned benchmark competitors.

#![allow(
    dead_code,
    reason = "Each measurement build selects a subset of these adapters"
)]

#[cfg(any(feature = "bs58", feature = "five8"))]
use faster_base58::{BufferTooSmall, Error};

#[cfg(feature = "bs58")]
pub fn encode_bs58(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    bs58::encode(input).onto(output).map_err(|_| BufferTooSmall)
}

#[cfg(feature = "bs58")]
pub fn decode_bs58(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    bs58::decode(input)
        .onto(output)
        .map_err(|error| match error {
            bs58::decode::Error::InvalidCharacter { index, .. }
            | bs58::decode::Error::NonAsciiCharacter { index } => Error::InvalidCharacter { index },
            _ => Error::BufferTooSmall,
        })
}

/// Fixed-width API: input is exactly 32/64 bytes; output needs the full 44/88-byte array.
#[cfg(feature = "five8")]
pub fn encode_five8(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    match input.len() {
        32 => {
            let output = output
                .get_mut(..five8::BASE58_ENCODED_32_MAX_LEN)
                .ok_or(BufferTooSmall)?;
            Ok(five8::encode_32(input.try_into().unwrap(), output.try_into().unwrap()) as usize)
        }
        64 => {
            let output = output
                .get_mut(..five8::BASE58_ENCODED_64_MAX_LEN)
                .ok_or(BufferTooSmall)?;
            Ok(five8::encode_64(input.try_into().unwrap(), output.try_into().unwrap()) as usize)
        }
        _ => Err(BufferTooSmall),
    }
}

/// The exact 32/64-byte output selects five8's width. Non-fixture errors are collapsed.
#[cfg(feature = "five8")]
pub fn decode_five8(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    let size = output.len();
    match size {
        32 => five8::decode_32(input, output.try_into().unwrap()),
        64 => five8::decode_64(input, output.try_into().unwrap()),
        _ => return Err(Error::BufferTooSmall),
    }
    .map_err(|_| Error::BufferTooSmall)?;
    Ok(size)
}

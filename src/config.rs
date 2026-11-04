//! Reusable conversion configuration. Only terminal methods perform conversion.

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

use crate::{BufferTooSmall, Error};

#[derive(Debug, Clone, Copy)]
enum Alphabet {
    Bitcoin,
}

/// Reusable Base58 encoding configuration, without input or conversion state.
///
/// ```
/// use faster_base58::Encoder;
/// const ENCODER: Encoder = Encoder::bitcoin();
/// let mut output = [0; 4];
/// assert_eq!(ENCODER.encode([0, 0, 1]).onto(&mut output)?, "112");
/// # Ok::<(), faster_base58::BufferTooSmall>(())
/// ```
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct Encoder {
    alphabet: Alphabet,
}

impl Encoder {
    /// Selects the Bitcoin alphabet for encoding.
    #[inline(always)]
    pub const fn bitcoin() -> Self {
        Self {
            alphabet: Alphabet::Bitcoin,
        }
    }

    /// Sets input for one conversion. References borrow; owned inputs move in.
    /// Conversion starts at a terminal method such as [`EncodeInput::onto`].
    #[inline(always)]
    pub const fn encode<I: AsRef<[u8]>>(self, input: I) -> EncodeInput<I> {
        EncodeInput {
            encoder: self,
            input,
        }
    }

    /// Bounds encoded length.
    /// Saturates at `usize::MAX` if the bound cannot be represented.
    #[must_use]
    #[inline(always)]
    pub const fn max_encoded_len(self, input_len: usize) -> usize {
        crate::encode::max_encoded_len(input_len)
    }
}

/// An encoder and its input, ready for one conversion.
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct EncodeInput<I> {
    encoder: Encoder,
    input: I,
}

impl<I: AsRef<[u8]>> EncodeInput<I> {
    /// Encodes into an owned ASCII string.
    #[inline(always)]
    #[cfg(feature = "alloc")]
    pub fn into_string(self) -> String {
        let bytes = self.into_vec();
        debug_assert!(bytes.is_ascii(), "encoders write only alphabet bytes");
        // SAFETY: every encoder writes only `ALPHABET` bytes, which are ASCII.
        unsafe { String::from_utf8_unchecked(bytes) }
    }

    /// Encodes into owned ASCII bytes.
    #[inline(always)]
    #[cfg(feature = "alloc")]
    pub fn into_vec(self) -> Vec<u8> {
        let input = self.input.as_ref();
        let mut output = vec![0; self.encoder.max_encoded_len(input.len())];
        let len = match self.encoder.alphabet {
            Alphabet::Bitcoin => crate::api::encode(input, &mut output),
        }
        .expect("max_encoded_len bounds the result");
        output.truncate(len);
        output
    }

    /// Encodes into `output` and borrows the written ASCII string. The unused
    /// tail is untouched on success; contents are unspecified on error.
    ///
    /// # Errors
    /// Returns [`BufferTooSmall`] if `output` cannot hold the encoded result.
    #[inline(always)]
    pub fn onto(self, output: &mut [u8]) -> Result<&str, BufferTooSmall> {
        let len = match self.encoder.alphabet {
            Alphabet::Bitcoin => crate::api::encode(self.input.as_ref(), output),
        }?;
        let text = &output[..len];
        debug_assert!(text.is_ascii(), "encoders write only alphabet bytes");
        // SAFETY: every encoder writes only `ALPHABET` bytes, which are ASCII, to
        // `output[..len]`.
        Ok(unsafe { core::str::from_utf8_unchecked(text) })
    }
}

/// Reusable Base58 decoding configuration, without input or conversion state.
///
/// ```
/// use faster_base58::Decoder;
/// const DECODER: Decoder = Decoder::bitcoin();
/// assert_eq!(DECODER.decode("112").into_array::<3>()?, [0, 0, 1]);
/// # Ok::<(), faster_base58::Error>(())
/// ```
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct Decoder {
    alphabet: Alphabet,
}

impl Decoder {
    /// Selects the Bitcoin alphabet for decoding.
    #[inline(always)]
    pub const fn bitcoin() -> Self {
        Self {
            alphabet: Alphabet::Bitcoin,
        }
    }

    /// Sets Base58 text for one conversion: a string or raw bytes, which need not be
    /// UTF-8. References borrow; owned inputs move in.
    /// Conversion starts at a terminal method such as [`DecodeInput::onto`].
    #[inline(always)]
    pub const fn decode<I: AsRef<[u8]>>(self, input: I) -> DecodeInput<I> {
        DecodeInput {
            decoder: self,
            input,
        }
    }
}

/// A decoder and its input, ready for one conversion.
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct DecodeInput<I> {
    decoder: Decoder,
    input: I,
}

impl<I: AsRef<[u8]>> DecodeInput<I> {
    /// Decodes into an owned byte vector.
    ///
    /// # Errors
    /// Returns [`Error`] for invalid Base58.
    #[inline(always)]
    #[cfg(feature = "alloc")]
    pub fn into_vec(self) -> Result<Vec<u8>, Error> {
        Ok(match self.decoder.alphabet {
            Alphabet::Bitcoin => crate::api::decode_vec(self.input.as_ref()),
        }?)
    }

    /// Decodes into `output` and borrows the data bytes. Exact data capacity
    /// suffices.
    /// The unused tail is untouched on success; contents are unspecified on error.
    ///
    /// # Errors
    /// Returns [`Error`] for invalid Base58 or insufficient capacity.
    #[inline(always)]
    pub fn onto(self, output: &mut [u8]) -> Result<&mut [u8], Error> {
        let len = match self.decoder.alphabet {
            Alphabet::Bitcoin => crate::api::decode(self.input.as_ref(), output),
        }?;
        Ok(&mut output[..len])
    }

    /// Decodes exactly `N` bytes.
    ///
    /// # Errors
    /// Returns [`Error::InvalidLength`] for a shorter result. A longer result returns
    /// [`Error::BufferTooSmall`]: decoding stops when the array is full, so the actual
    /// length is unknown. Also returns the validation errors from [`Self::onto`].
    /// Error precedence is unspecified.
    #[inline(always)]
    pub fn into_array<const N: usize>(self) -> Result<[u8; N], Error> {
        let mut output = [0; N];
        let actual = self.onto(&mut output)?.len();
        if actual != N {
            return Err(Error::InvalidLength {
                expected: N,
                actual,
            });
        }
        Ok(output)
    }
}

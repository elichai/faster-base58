//! Reusable conversion configuration. Only terminal methods perform conversion.

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

use crate::{BufferTooSmall, Error};

#[derive(Debug, Clone, Copy)]
enum Alphabet {
    Bitcoin,
}

#[derive(Debug, Clone, Copy)]
enum Check {
    None,
    #[cfg(feature = "check")]
    Base58Check(Option<u8>),
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
    check: Check,
}

impl Encoder {
    /// Selects the Bitcoin alphabet without a checksum.
    #[inline(always)]
    pub const fn bitcoin() -> Self {
        Self {
            alphabet: Alphabet::Bitcoin,
            check: Check::None,
        }
    }

    /// Appends a `Base58Check` checksum without inserting a version prefix.
    /// Replaces any previous checksum/version setting.
    #[cfg(feature = "check")]
    #[inline(always)]
    pub const fn with_check(mut self) -> Self {
        self.check = Check::Base58Check(None);
        self
    }

    /// Prepends `version` and appends a `Base58Check` checksum covering both the
    /// version and input. Replaces any previous checksum/version setting.
    #[cfg(feature = "check")]
    #[inline(always)]
    pub const fn with_check_version(mut self, version: u8) -> Self {
        self.check = Check::Base58Check(Some(version));
        self
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

    /// Bounds encoded length, including any inserted version and checksum.
    /// Saturates at `usize::MAX` if the bound cannot be represented.
    #[must_use]
    #[inline(always)]
    pub const fn max_encoded_len(self, input_len: usize) -> usize {
        let extra = match self.check {
            Check::None => 0,
            #[cfg(feature = "check")]
            Check::Base58Check(version) => 4 + version.is_some() as usize,
        };
        crate::encode::max_encoded_len(input_len.saturating_add(extra))
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
    /// Encodes into an owned ASCII string. Checked encoding uses the result buffer
    /// for framing, plus the codec's normal workspace.
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
        let len = match (self.encoder.alphabet, self.encoder.check) {
            (Alphabet::Bitcoin, Check::None) => crate::api::encode(input, &mut output),
            #[cfg(feature = "check")]
            (Alphabet::Bitcoin, Check::Base58Check(version)) => {
                crate::check::encode(input, &mut output, version)
            }
        }
        .expect("max_encoded_len bounds the result");
        output.truncate(len);
        output
    }

    /// Encodes into `output` and borrows the written ASCII string. The unused
    /// tail is untouched on success; contents are unspecified on error.
    /// Checked encoding uses the output buffer for framing, plus normal codec workspace.
    ///
    /// # Errors
    /// Returns [`BufferTooSmall`] if `output` cannot hold the encoded result.
    #[inline(always)]
    pub fn onto(self, output: &mut [u8]) -> Result<&str, BufferTooSmall> {
        let len = match (self.encoder.alphabet, self.encoder.check) {
            (Alphabet::Bitcoin, Check::None) => crate::api::encode(self.input.as_ref(), output),
            #[cfg(feature = "check")]
            (Alphabet::Bitcoin, Check::Base58Check(version)) => {
                crate::check::encode(self.input.as_ref(), output, version)
            }
        }?;
        let text = &output[..len];
        debug_assert!(text.is_ascii(), "encoders write only alphabet bytes");
        // SAFETY: every encoder writes only `ALPHABET` bytes, which are ASCII, to
        // `output[..len]`.
        Ok(unsafe { core::str::from_utf8_unchecked(text) })
    }
}

/// Reusable Base58 decoding configuration, without input or conversion state.
/// Checked decoding removes the checksum and retains any version prefix.
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
    check: Check,
}

impl Decoder {
    /// Selects the Bitcoin alphabet without checksum validation.
    #[inline(always)]
    pub const fn bitcoin() -> Self {
        Self {
            alphabet: Alphabet::Bitcoin,
            check: Check::None,
        }
    }

    /// Requires a valid `Base58Check` checksum without requiring a version.
    /// Replaces any previous checksum/version setting.
    #[cfg(feature = "check")]
    #[inline(always)]
    pub const fn with_check(mut self) -> Self {
        self.check = Check::Base58Check(None);
        self
    }

    /// Requires a valid `Base58Check` checksum and the given first data byte.
    /// The version byte is retained in the output. Replaces any previous setting.
    #[cfg(feature = "check")]
    #[inline(always)]
    pub const fn with_check_version(mut self, version: u8) -> Self {
        self.check = Check::Base58Check(Some(version));
        self
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
    /// Decodes into an owned byte vector, removing a checksum when configured.
    ///
    /// # Errors
    /// Returns [`Error`] for invalid Base58 or a missing/invalid checksum or
    /// unexpected version when checking is enabled.
    #[inline(always)]
    #[cfg(feature = "alloc")]
    pub fn into_vec(self) -> Result<Vec<u8>, Error> {
        match (self.decoder.alphabet, self.decoder.check) {
            (Alphabet::Bitcoin, Check::None) => Ok(crate::api::decode_vec(self.input.as_ref())?),
            #[cfg(feature = "check")]
            (Alphabet::Bitcoin, Check::Base58Check(version)) => {
                let mut output = crate::api::decode_vec(self.input.as_ref())?;
                let len = crate::check::verify(&output, version)?;
                output.truncate(len);
                Ok(output)
            }
        }
    }

    /// Decodes into `output` and borrows the data bytes. Exact data capacity
    /// suffices, including any retained version and excluding the checksum.
    /// Checked decoding holds the checksum separately in a four-byte stack value.
    /// The unused tail is untouched on success; contents are unspecified on error.
    ///
    /// # Errors
    /// Returns [`Error`] for invalid Base58, insufficient capacity, or a
    /// missing/invalid checksum or unexpected version when checking is enabled.
    #[inline(always)]
    pub fn onto(self, output: &mut [u8]) -> Result<&mut [u8], Error> {
        let len = match (self.decoder.alphabet, self.decoder.check) {
            (Alphabet::Bitcoin, Check::None) => crate::api::decode(self.input.as_ref(), output)?,
            #[cfg(feature = "check")]
            (Alphabet::Bitcoin, Check::Base58Check(version)) => {
                crate::check::decode(self.input.as_ref(), output, version)?
            }
        };
        Ok(&mut output[..len])
    }

    /// Decodes exactly `N` bytes, including any retained version and excluding
    /// the checksum. Checked decoding reserves four checksum bytes on the stack,
    /// in addition to normal codec workspace.
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

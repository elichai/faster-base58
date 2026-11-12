//! Fast Bitcoin Base58 encoding and decoding with automatic CPU dispatch.
//!
//! [`Encoder`] and [`Decoder`] select conversion algorithms by input size and CPU
//! support. They are reusable, `Copy` configurations that can be defined as constants.
//! Set the input with `.encode(input)` or `.decode(input)`, then choose an output
//! method to perform the conversion. Both accept `AsRef<[u8]>` inputs, including
//! arrays, byte slices, and strings. Decoding raw bytes requires no UTF-8 conversion.
//!
//! The crate supports `no_std`. The default `alloc` feature enables owned results
//! and allocating backends for large inputs. Caller-provided buffers and exact-size
//! arrays work with or without `alloc`. Requires Rust 1.95 or newer.
//!
//! # Quick start
//!
//! With the default features, encode to a string and decode to a byte vector:
//!
//! ```
//! # #[cfg(feature = "alloc")]
//! # {
//! use faster_base58::{Decoder, Encoder};
//!
//! let text = Encoder::bitcoin().encode(b"hello world").into_string();
//! assert_eq!(text, "StV1DL6CwTryKyV");
//!
//! let bytes = Decoder::bitcoin().decode(&text).into_vec()?;
//! assert_eq!(bytes, b"hello world");
//! # }
//! # Ok::<(), faster_base58::Error>(())
//! ```
//!
//! With `alloc`, encoding also provides `.into_vec()` for owned ASCII bytes.
//!
//! # Caller-provided buffers and fixed-size output
//!
//! Use [`EncodeInput::onto`] and [`DecodeInput::onto`] to borrow the written region
//! of a buffer, or [`DecodeInput::into_array`] to require an exact decoded length:
//!
//! ```
//! use faster_base58::{Decoder, Encoder};
//!
//! const ENCODER: Encoder = Encoder::bitcoin();
//! const DECODER: Decoder = Decoder::bitcoin();
//!
//! let data = [0, 0, 1];
//! let mut text_buffer = [0; ENCODER.max_encoded_len(3)];
//! let text = ENCODER.encode(data).onto(&mut text_buffer)?;
//! assert_eq!(text, "112"); // Leading zero bytes become leading '1' characters.
//!
//! let mut byte_buffer = [0; 3];
//! assert_eq!(DECODER.decode(text).onto(&mut byte_buffer)?, data);
//! assert_eq!(DECODER.decode(text).into_array::<3>()?, data);
//! # Ok::<(), faster_base58::Error>(())
//! ```
//!
//! [`Encoder::max_encoded_len`] bounds the required encoding capacity, including
//! any configured checksum and version prefix. An exactly sized result buffer also
//! works. Plain Base58 decoding needs at most one output byte per input text byte.
//! For `.into_array::<N>()`, a shorter result returns [`Error::InvalidLength`]; a
//! longer result returns [`Error::BufferTooSmall`].
//!
//! Supplying output buffers avoids allocating the result, but with `alloc` enabled
//! large conversions can still allocate internal workspace. To disable internal
//! heap allocation, including on first calls and errors, disable default features:
//!
//! ```toml
//! [dependencies]
//! faster-base58 = { version = "0.1.0", default-features = false }
//! ```
//!
//! The buffer example above works with this configuration. General large inputs
//! then use quadratic conversion. Cargo features are additive: `alloc` must remain
//! disabled across the dependency graph. Allocator calls in caller-defined `AsRef`
//! implementations or from dropping owned inputs are outside this guarantee; borrow
//! inputs when counting all allocator operations.
//!
//! # `Base58Check`
//!
//! Enable `check` to add Bitcoin's four-byte double-SHA-256 checksum:
//!
//! ```toml
//! [dependencies]
//! faster-base58 = { version = "0.1.0", features = ["check"] }
//! ```
//!
//! Checking is selected on each configuration. This example uses a one-byte version
//! prefix and works with or without `alloc`:
//!
//! ```
//! # #[cfg(feature = "check")]
//! # {
//! use faster_base58::{Decoder, Encoder};
//!
//! const ENCODER: Encoder = Encoder::bitcoin().with_check_version(0);
//! const DECODER: Decoder = Decoder::bitcoin().with_check_version(0);
//!
//! let payload = [0u8; 20];
//! let mut buffer = [0; ENCODER.max_encoded_len(20)];
//! let text = ENCODER.encode(payload).onto(&mut buffer)?;
//! assert_eq!(text, "1111111111111111111114oLvT2");
//!
//! // Decoding removes the checksum and retains the version byte.
//! let decoded = DECODER.decode(text).into_array::<21>()?;
//! assert_eq!(decoded[0], 0);
//! assert_eq!(decoded[1..], payload);
//! # }
//! # Ok::<(), faster_base58::Error>(())
//! ```
//!
//! Use `.with_check()` on both configurations for checksums without inserting or
//! requiring a version. Multi-byte version prefixes belong in the input payload;
//! use `.with_check()` and inspect the decoded prefix. Checked decoding needs output
//! capacity only for the returned data, including any retained version; the checksum
//! is held separately on the stack. Enabling `check` does not enable checking on
//! plain `Encoder::bitcoin()` or `Decoder::bitcoin()` configurations.
//!
//! # Cargo features
//!
//! | Feature | Enabled by default | Effect |
//! | --- | :---: | --- |
//! | `alloc` | Yes | Owned output methods and allocating backends for faster large-input conversion. Does not require `std`. |
//! | `std` | No | Standard-library support, including runtime CPU detection through `std` on x86-64. Does not enable `alloc`. |
//! | `check` | No | `Base58Check` configuration and checksum errors through the optional `sha2` dependency. Requires neither `alloc` nor `std`. |
//! | `cache` | No | Enables `alloc` and `std`; lazily retains bounded matrix and NTT tables across calls. Changes dispatch and retains heap memory for the process lifetime. |
//! | `compact` | No | Uses fewer x86-64 IFMA kernel widths to reduce linked code, with a size-dependent speed tradeoff. Preserves the API and allocation contracts. |
//! | `__internals` | No | Enables `alloc` and exposes unstable backend controls for this repository's tests and measurements. Not intended for applications. |
//!
//! # Errors and security
//!
//! Encoding into an undersized buffer returns [`BufferTooSmall`]. Decoding returns
//! [`Error`] for invalid characters, insufficient capacity, length mismatches, and
//! checksum or version failures when checking is configured. Invalid-character
//! indices are byte offsets. `Error` is non-exhaustive; matches need a wildcard arm.
//!
//! Successful buffer conversions leave the unused tail untouched. On error, buffer
//! contents and error precedence are unspecified. Empty inputs and leading zero
//! bytes are supported.
//!
//! Conversion time depends on input values and length. These algorithms are not
//! constant-time and are not designed to hide either. A `Base58Check` checksum
//! detects accidental corruption; it does not authenticate data.
//!
//! # Further reading
//!
//! See the [repository README](https://github.com/elichai/faster-base58#readme) for
//! performance results and the [design guide](https://github.com/elichai/faster-base58/blob/main/DESIGN.md)
//! for algorithms, dispatch, and resource costs.

#![cfg_attr(not(any(feature = "std", test)), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
#[cfg(any(feature = "alloc", test, kani))]
extern crate alloc;

#[cfg(all(kani, not(feature = "std")))]
extern crate std;

mod api;
mod batched;
#[cfg(feature = "check")]
mod check;
mod common;
mod config;
#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
mod cpu;
#[cfg(any(feature = "alloc", test, kani))]
mod dc;
mod encode;
#[cfg(test)]
#[path = "../tests/support/malachite.rs"]
mod malachite;
#[cfg(test)]
mod malachite_tests;
#[cfg(any(feature = "alloc", test, kani))]
mod matrix;
#[cfg(any(feature = "alloc", test, kani))]
mod mul;
#[cfg(all(
    any(feature = "alloc", test, kani),
    any(all(target_arch = "x86_64", target_feature = "sse2"), test, kani)
))]
mod mul_ifma;
#[cfg(any(not(feature = "alloc"), test, feature = "__internals"))]
mod no_alloc;
#[cfg(any(feature = "alloc", test, kani))]
mod ntt;
#[cfg(test)]
mod tests;
#[cfg(kani)]
mod verification;

#[cfg(test)]
pub(crate) use api::encode;

/// Automatic decoding with the hooks' `&str` / [`Error`] signature, for unit tests.
#[cfg(test)]
pub(crate) fn decode(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    Ok(api::decode(input.as_bytes(), output)?)
}
pub use config::{DecodeInput, Decoder, EncodeInput, Encoder};

#[cfg(any(test, feature = "__internals"))]
pub mod internals;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_examples {}

#[cfg(test)]
use malachite::{decode_malachite, encode_malachite};

#[cfg(test)]
extern crate self as faster_base58;

/// A decoding error. When the input has an invalid character and the output buffer is
/// also too small for it, which of the two is reported is unspecified.
/// Additional variants may be added; downstream matches need a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The byte at `index` is the first that is not in the alphabet.
    InvalidCharacter { index: usize },
    /// The output buffer cannot hold the decoded bytes.
    BufferTooSmall,
    /// An exact-length array was requested, but fewer bytes were decoded.
    InvalidLength { expected: usize, actual: usize },
    /// Fewer than four checksum bytes were decoded.
    #[cfg(feature = "check")]
    MissingChecksum,
    /// The four-byte double-SHA-256 checksum did not match the data.
    #[cfg(feature = "check")]
    InvalidChecksum,
    /// The validated data has a different version, or no version byte.
    #[cfg(feature = "check")]
    InvalidVersion { expected: u8, actual: Option<u8> },
}

/// An encoding error: the output buffer cannot hold the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferTooSmall;

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidCharacter { index } => {
                write!(f, "invalid base58 character at byte {index}")
            }
            Self::BufferTooSmall => BufferTooSmall.fmt(f),
            Self::InvalidLength { expected, actual } => {
                write!(f, "expected {expected} decoded bytes, got {actual}")
            }
            #[cfg(feature = "check")]
            Self::MissingChecksum => f.write_str("missing Base58Check checksum"),
            #[cfg(feature = "check")]
            Self::InvalidChecksum => f.write_str("invalid Base58Check checksum"),
            #[cfg(feature = "check")]
            Self::InvalidVersion {
                expected,
                actual: Some(actual),
            } => {
                write!(f, "expected version {expected}, got {actual}")
            }
            #[cfg(feature = "check")]
            Self::InvalidVersion {
                expected,
                actual: None,
            } => {
                write!(f, "expected version {expected}, got empty data")
            }
        }
    }
}

impl core::error::Error for Error {}

impl From<BufferTooSmall> for Error {
    #[inline]
    fn from(BufferTooSmall: BufferTooSmall) -> Self {
        Self::BufferTooSmall
    }
}

impl core::fmt::Display for BufferTooSmall {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("output buffer is too small")
    }
}

impl core::error::Error for BufferTooSmall {}

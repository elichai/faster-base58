//! Base58 (Bitcoin alphabet) decoding and encoding on 64-bit limbs. See DESIGN.md for the
//! design.
//!
//! [`Encoder`] and [`Decoder`] provide reusable `const` configuration followed by
//! `.encode(input)` / `.decode(input)` and a terminal conversion method. The optional
//! `check` feature adds Bitcoin `Base58Check` with an optional version prefix.
//! Errors can change a supplied buffer. Conversion is variable-time.
//!
//! The library is `no_std`. Default feature `alloc` enables allocating backends and
//! owned results. Disable default features for conversion without heap allocation,
//! including first calls and errors; large inputs then use quadratic conversion.
//! The optional `cache` feature enables `alloc` and `std` and retains bounded NTT and
//! matrix tables across calls. All named backend selectors require `__internals`;
//! that feature exposes the unstable `internals` module for tests and measurements.

#![cfg_attr(not(any(feature = "std", test)), no_std)]

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
#[cfg(any(test, feature = "__internals", kani))]
mod toom3;
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

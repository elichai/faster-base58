//! Base58 (Bitcoin alphabet) decoding and encoding on 64-bit limbs. See README.md for the
//! design.
//!
//! [`encode`] and [`decode`] select a backend by input size and CPU support. Use
//! [`encode_into`] / [`decode_into`] with arrays or reusable buffers, [`encode_string`]
//! / [`decode_vec`] for owned results, and [`decode_array`] for an exact decoded size.
//! Errors can change the contents of a supplied buffer. These algorithms are variable
//! time and are not designed to hide input values or lengths.
//!
//! [`no_alloc`] guarantees zero internal heap allocation, including first calls.
//! It accepts caller buffers and supports exact arrays. Large inputs use quadratic
//! conversion; Auto can allocate workspace for faster algorithms.
//!
//! The optional `cache` feature retains bounded NTT and matrix tables across calls.
//! It is disabled by default. All named backend selectors require `__internals`;
//! that feature exposes the unstable `internals` module for tests and measurements.

mod api;
mod batched;
mod common;
mod dc;
mod encode;
#[cfg(test)]
#[path = "../tests/support/malachite.rs"]
mod malachite;
#[cfg(test)]
mod malachite_tests;
mod matrix;
mod mul;
#[cfg(any(target_arch = "x86_64", test, kani))]
mod mul_ifma;
mod ntt;
pub mod no_alloc;
#[cfg(test)]
mod tests;
#[cfg(kani)]
mod verification;
#[cfg(any(test, feature = "__internals", kani))]
mod toom3;

pub use api::{decode, decode_array, decode_into, decode_vec, encode, encode_into, encode_string};
pub use encode::max_encoded_len;

#[cfg(any(test, feature = "__internals"))]
pub mod internals;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_examples {}

#[cfg(test)]
use malachite::{decode_malachite, encode_malachite};

#[cfg(test)]
extern crate self as base58_bench;

/// A decoding error. When the input has an invalid character and the output buffer is
/// also too small for it, which of the two is reported is unspecified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The byte at `index` is the first that is not in the alphabet.
    InvalidCharacter { index: usize },
    BufferTooSmall,
    /// An exact-length array was requested, but fewer bytes were decoded.
    InvalidLength { expected: usize, actual: usize },
}

/// An encoding error: the output buffer cannot hold the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferTooSmall;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCharacter { index } => write!(f, "invalid base58 character at byte {index}"),
            Self::BufferTooSmall => BufferTooSmall.fmt(f),
            Self::InvalidLength { expected, actual } => {
                write!(f, "expected {expected} decoded bytes, got {actual}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for BufferTooSmall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("output buffer is too small")
    }
}

impl std::error::Error for BufferTooSmall {}

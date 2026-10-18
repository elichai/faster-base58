//! Base58 (Bitcoin alphabet) decoding on 64-bit limbs. See README.md for the design.
//!
//! Each algorithm has its own module:
//! - [`decode_batched`] (`batched.rs`): quadratic, one limb update per ten characters
//!   and eight output bytes.
//! - [`decode`] (`dc.rs`): divide-and-conquer radix conversion for large inputs, with
//!   Karatsuba multiplication (`mul.rs`).
//! - [`decode_toom3`] (`toom3.rs`): [`decode`] with Toom-3 for large products.
//! - [`decode_ntt`], [`decode_ntt_cached`], [`decode_ntt_portable`] (`ntt/`): [`decode`]
//!   with number-theoretic-transform multiplication and SIMD kernels.
//! - [`decode_malachite`] (`malachite.rs`): [`decode`] with malachite-nz's
//!   multiplication.
//! - [`decode_matrix`], [`decode_matrix_portable`] (`matrix/`): a matrix of powers with
//!   SIMD multiply-adds, for about 0.4 KiB to 4 KiB on aarch64 (NEON).
//!
//! `common.rs` holds what the decoders share; `tests.rs` checks every decoder against
//! `bs58`.

mod batched;
mod common;
mod dc;
mod malachite;
mod matrix;
mod mul;
#[cfg(any(target_arch = "x86_64", test))]
mod mul_ifma;
mod ntt;
#[cfg(test)]
mod tests;
mod toom3;

pub use batched::decode_batched;
pub use dc::decode;
pub use malachite::decode_malachite;
pub use matrix::{decode_matrix, decode_matrix_portable};
pub use ntt::{decode_ntt, decode_ntt_cached, decode_ntt_portable};
pub use toom3::decode_toom3;

/// Hooks for benchmarks and A/B harnesses; not part of the API.
#[doc(hidden)]
pub mod bench {
    #[cfg(target_arch = "x86_64")]
    pub use crate::mul::set_ifma_threshold;
    pub use crate::mul::{ifma_threshold, mul_with, scratch_len};
    #[cfg(target_arch = "x86_64")]
    pub use crate::mul_ifma::{mul as ifma_mul, scratch_len as ifma_scratch_len};
}

/// A decoding error. When the input has an invalid character and the output buffer is
/// also too small for it, which of the two is reported is unspecified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The byte at `index` is the first that is not in the alphabet.
    InvalidCharacter { index: usize },
    BufferTooSmall,
}

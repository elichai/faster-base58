//! Comparison and tuning entry points; the root facade keeps their flattened names.

use super::{CACHED_LEAF, ENCODE_THRESHOLD, Kernels, Ntt};
use crate::dc::{self, Leaf};
use crate::encode::{self, Leaves};
use crate::{BufferTooSmall, Error};

pub(crate) mod cached;
pub(crate) mod owned;

/// NTT conversion crossover with IFMA, retained for explicit backend comparisons.
pub(crate) const ENCODE_IFMA_THRESHOLD: usize = 512;

fn encode_params() -> (usize, usize) {
    match crate::mul::ifma_threshold() {
        0 => (ENCODE_THRESHOLD, CACHED_LEAF),
        _ => (ENCODE_IFMA_THRESHOLD, CACHED_LEAF),
    }
}

impl Ntt {
    /// Own tables even when the `cache` feature is enabled, for cost comparisons.
    pub(crate) fn new_owned(kernels: Kernels) -> Self {
        Self {
            #[cfg(feature = "cache")]
            force_owned: true,
            ..Self::new(kernels)
        }
    }

    /// Select the table policy for an explicit comparison, without changing leaves.
    pub(super) fn with_cache(cached: bool, kernels: Kernels) -> Self {
        if cached {
            Self::new(kernels)
        } else {
            Self::new_owned(kernels)
        }
    }
}

/// [`owned::decode_ntt`], or with `cached` [`cached::decode_ntt_cached`], with
/// `threshold` and `prepared` in place of `THRESHOLD` and `PREPARED_THRESHOLD`.
///
/// # Errors
/// Returns [`Error::InvalidCharacter`] for a byte outside the alphabet, or
/// [`Error::BufferTooSmall`] if `output` cannot hold the decoded bytes.
pub fn decode_ntt_with(
    input: &str,
    output: &mut [u8],
    cached: bool,
    threshold: usize,
    prepared: usize,
) -> Result<usize, Error> {
    let ntt = Ntt::with_cache(cached, Kernels::best());
    let mut ntt = Ntt {
        threshold,
        prepared_threshold: prepared,
        ..ntt
    };
    // The leaves of the decoder it stands in for.
    let leaf = if cached { Leaf::BEST } else { Leaf::Horner };
    dc::decode_with_leaf(input, output, &mut ntt, leaf)
}

/// [`cached::decode_ntt_cached`] with the portable kernels, compared with hand-written SIMD.
/// On aarch64 LLVM vectorizes the portable kernels with NEON itself, so this measures
/// hand-written against compiler-generated NEON, not SIMD against scalar code. The leaves
/// are those of [`cached::decode_ntt_cached`].
///
/// # Errors
/// Returns the same invalid-character and insufficient-buffer errors as
/// [`cached::decode_ntt_cached`].
pub fn decode_ntt_portable(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    dc::decode_with_leaf(
        input,
        output,
        &mut Ntt::new(Kernels::portable()),
        Leaf::BEST,
    )
}

/// [`cached::encode_ntt_cached`] with divide and conquer above `threshold` bytes and
/// leaves of at most `leaf` chunks, converted by `leaves`.
///
/// # Errors
/// Returns [`BufferTooSmall`] if `output` cannot hold the encoded string.
pub fn encode_ntt_with(
    input: &[u8],
    output: &mut [u8],
    threshold: usize,
    leaf: usize,
    leaves: Leaves,
) -> Result<usize, BufferTooSmall> {
    let m = &mut Ntt::new(Kernels::best());
    encode::encode_tuned(input, output, m, threshold, leaf, leaves)
}

/// [`cached::encode_ntt_cached`] with the portable kernels, as [`decode_ntt_portable`] is for
/// decoding: on aarch64 this compares hand-written with compiler-generated NEON.
///
/// # Errors
/// Returns [`BufferTooSmall`] if `output` cannot hold the encoded string.
pub fn encode_ntt_portable(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    let m = &mut Ntt::new(Kernels::portable());
    let (threshold, leaf) = encode_params();
    encode::encode_tuned(input, output, m, threshold, leaf, Leaves::BEST)
}

#[cfg(feature = "__internals")]
pub(crate) mod diagnostics;

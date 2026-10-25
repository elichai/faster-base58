//! Feature-only same-leaf comparisons and retained-memory diagnostics.

use super::*;

/// Compare owned/cached tables with the same leaves and thresholds.
pub fn decode_ntt_same_leaves(
    input: &str, output: &mut [u8], cached: bool,
) -> Result<usize, Error> {
    dc::decode_with_leaf(input, output, &mut Ntt::with_cache(cached, Kernels::best()), Leaf::BEST)
}

/// Compare owned/cached tables with the same leaves and thresholds.
pub fn encode_ntt_same_leaves(
    input: &[u8], output: &mut [u8], cached: bool,
) -> Result<usize, BufferTooSmall> {
    let (threshold, leaf) = encode_params();
    encode::encode_tuned(
        input, output, &mut Ntt::with_cache(cached, Kernels::best()), threshold, leaf, Leaves::BEST,
    )
}

#[cfg(feature = "cache")]
pub use super::super::tables::cache::cache_memory as ntt_cache_memory;

/// Cache metadata without retention when the `cache` feature is disabled.
#[cfg(not(feature = "cache"))]
pub fn ntt_cache_memory() -> (usize, [(u32, bool, usize, usize); super::super::MAX_LOG as usize + 1]) {
    (0, std::array::from_fn(|log| (log as u32, false, 0, 0)))
}

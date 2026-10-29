//! Legacy comparison names for the feature-selected NTT policy.

use super::*;

/// Feature-selected NTT decoding, with the small-input comparison fallback.
///
/// # Errors
/// Returns [`Error::InvalidCharacter`] for a byte outside the alphabet, or
/// [`Error::BufferTooSmall`] if `output` cannot hold the decoded bytes.
pub fn decode_ntt_cached(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    dc::decode_with_leaf(input, output, &mut Ntt::new(Kernels::best()), Leaf::BEST)
}

/// Feature-selected NTT encoding, with the small-input comparison fallback.
///
/// # Errors
/// Returns [`BufferTooSmall`] if `output` cannot hold the encoded string.
pub fn encode_ntt_cached(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    let (threshold, leaf) = encode_params();
    encode::encode_tuned(
        input,
        output,
        &mut Ntt::new(Kernels::best()),
        threshold,
        leaf,
        Leaves::BEST,
    )
}

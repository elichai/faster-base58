//! Entry points that own their twiddle tables.

use super::super::{ENCODE_LEAF, Kernels, Ntt};
use super::encode_params;
use crate::dc;
use crate::encode::{self, Leaves};
use crate::{BufferTooSmall, Error};

/// Divide-and-conquer decoding with NTT multiplication. Builds owned twiddle tables
/// only when a product uses transforms.
///
/// # Errors
/// Returns [`Error::InvalidCharacter`] for a byte outside the alphabet, or
/// [`Error::BufferTooSmall`] if `output` cannot hold the decoded bytes.
pub fn decode_ntt(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    dc::decode_with(input, output, &mut Ntt::new_owned(Kernels::best()))
}

/// Divide-and-conquer encoding with NTT multiplication. Builds owned twiddle tables
/// only when a product uses transforms.
///
/// # Errors
/// Returns [`BufferTooSmall`] if `output` cannot hold the encoded string.
pub fn encode_ntt(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    let m = &mut Ntt::new_owned(Kernels::best());
    encode::encode_tuned(
        input,
        output,
        m,
        encode_params().0,
        ENCODE_LEAF,
        Leaves::Horner,
    )
}

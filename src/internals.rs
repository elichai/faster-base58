//! Unstable backend selectors and measurement hooks, enabled by `__internals`.
//!
//! This is for differential tests, fuzzing and benchmarks. Prefer the crate's
//! automatic API or [`crate::no_alloc`] in applications. These hooks may change
//! without compatibility guarantees; tuning parameters have backend-specific
//! preconditions documented on each function.

/// Matrix cache class: input limit in bytes/characters, initialized, panel count,
/// panel capacity bytes, coefficient count, coefficient capacity bytes.
pub type MatrixCacheClass = (usize, bool, usize, usize, usize, usize);

pub use crate::batched::decode_batched;
pub use crate::dc::{Multiplier, decode as decode_karatsuba, decode_with};
pub use crate::encode::dc::{
    internals::encode_karatsuba_with, karatsuba::encode as encode_karatsuba,
};
pub use crate::encode::matrix::internals::{encode_matrix_portable, encode_matrix_with};
pub use crate::encode::matrix::{encode_whole as encode_matrix_whole, entry::encode_matrix};
pub use crate::encode::{Leaves, encode_batched, encode_tuned};
pub use crate::matrix::{entry::decode_matrix, internals::decode_matrix_portable};
pub use crate::mul::{ifma_threshold, mul_with, scratch_len};
pub use crate::ntt::internals::{
    cached::{decode_ntt_cached, encode_ntt_cached},
    decode_ntt_portable, decode_ntt_with, encode_ntt_portable, encode_ntt_with,
    owned::{decode_ntt, encode_ntt},
};
pub use crate::toom3::{decode_toom3, decode_toom3_with, encode_toom3};
#[cfg(feature = "__internals")]
pub use crate::{
    encode::matrix::internals::feature_only::{
        encode_matrix_cache_memory, fixed_matrix_table_bytes,
    },
    matrix::internals::feature_only::{decode_matrix_cache_memory, decode_matrix_forced},
    ntt::internals::diagnostics::{
        decode_ntt_same_leaves, encode_ntt_same_leaves, ntt_cache_memory,
    },
};
#[cfg(target_arch = "x86_64")]
pub use crate::{
    mul::ifma::set_ifma_threshold,
    mul_ifma::{avx512::mul as ifma_mul, scratch_len as ifma_scratch_len},
};

#![allow(unexpected_cfgs)]
#![allow(dead_code, reason = "each probe selects one backend/direction from shared adapters")]

use std::hint::black_box;

#[cfg(size_external_malachite)]
include!("malachite_adapter.rs");
#[cfg(any(size_backend = "bs58", size_backend = "five8"))]
include!("competitors_adapter.rs");

macro_rules! backend {
    ($name:literal, $encode:path, $decode:path) => {
        #[cfg(all(size_backend = $name, not(size_direction = "decode")))]
        use $encode as encode;
        #[cfg(all(size_backend = $name, not(size_direction = "encode")))]
        use $decode as decode;
    };
}

backend!("auto", base58_bench::encode, base58_bench::decode);
backend!("no-alloc", base58_bench::no_alloc::encode, base58_bench::no_alloc::decode);
backend!("batched", base58_bench::internals::encode_batched, base58_bench::internals::decode_batched);
backend!("karatsuba", base58_bench::internals::encode_karatsuba, base58_bench::internals::decode_karatsuba);
backend!("toom3", base58_bench::internals::encode_toom3, base58_bench::internals::decode_toom3);
backend!("ntt", base58_bench::internals::encode_ntt, base58_bench::internals::decode_ntt);
backend!("ntt-cached", base58_bench::internals::encode_ntt_cached, base58_bench::internals::decode_ntt_cached);
backend!("ntt-portable", base58_bench::internals::encode_ntt_portable, base58_bench::internals::decode_ntt_portable);
backend!("matrix", base58_bench::internals::encode_matrix, base58_bench::internals::decode_matrix);
backend!("matrix-portable", base58_bench::internals::encode_matrix_portable, base58_bench::internals::decode_matrix_portable);
backend!("matrix-forced", base58_bench::internals::encode_matrix_whole, base58_bench::internals::decode_matrix_forced);
backend!("ntt-same-owned", self::encode_ntt_same_leaves, self::decode_ntt_same_leaves);
backend!("ntt-same-cached", self::encode_ntt_same_leaves, self::decode_ntt_same_leaves);
backend!("bs58", competitors_adapter::encode_bs58, competitors_adapter::decode_bs58);
backend!("five8", competitors_adapter::encode_five8, competitors_adapter::decode_five8);
#[cfg(all(size_backend = "malachite", not(size_external_malachite)))]
compile_error!("the Malachite probe requires the external tests/support/malachite.rs adapter");
#[cfg(size_external_malachite)]
backend!("malachite", malachite_adapter::encode_malachite, malachite_adapter::decode_malachite);

#[cfg(all(any(size_backend = "ntt-same-owned", size_backend = "ntt-same-cached"), not(size_direction = "decode")))]
fn encode_ntt_same_leaves(input: &[u8], output: &mut [u8]) -> Result<usize, base58_bench::BufferTooSmall> {
    base58_bench::internals::encode_ntt_same_leaves(input, output, cfg!(size_backend = "ntt-same-cached"))
}

#[cfg(all(any(size_backend = "ntt-same-owned", size_backend = "ntt-same-cached"), not(size_direction = "encode")))]
fn decode_ntt_same_leaves(input: &str, output: &mut [u8]) -> Result<usize, base58_bench::Error> {
    base58_bench::internals::decode_ntt_same_leaves(input, output, cfg!(size_backend = "ntt-same-cached"))
}

// Every artifact exports these two helpers, including the empty baseline.
#[unsafe(no_mangle)]
pub extern "C" fn size_alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8
}

/// `ptr` must have been returned by `size_alloc(len)` and not freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn size_dealloc(ptr: *mut u8, len: usize) {
    unsafe { drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len))) };
}

/// Pointers must be valid, non-overlapping slices for the stated lengths.
/// Returns usize::MAX on any conversion error.
#[cfg(not(size_direction = "decode"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn size_encode(
    input: *const u8, input_len: usize, output: *mut u8, output_len: usize,
) -> usize {
    let input = unsafe { std::slice::from_raw_parts(input, input_len) };
    let output = unsafe { std::slice::from_raw_parts_mut(output, output_len) };
    #[cfg(size_backend = "empty")]
    return black_box((input.len(), output.len())).0;
    #[cfg(not(size_backend = "empty"))]
    encode(input, output).unwrap_or(usize::MAX)
}

/// Same pointer contract as `size_encode`; checks UTF-8 and conversion errors.
#[cfg(not(size_direction = "encode"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn size_decode(
    input: *const u8, input_len: usize, output: *mut u8, output_len: usize,
) -> usize {
    let input = unsafe { std::slice::from_raw_parts(input, input_len) };
    let output = unsafe { std::slice::from_raw_parts_mut(output, output_len) };
    let Ok(input) = std::str::from_utf8(input) else { return usize::MAX; };
    #[cfg(size_backend = "empty")]
    return black_box((input.len(), output.len())).0;
    #[cfg(not(size_backend = "empty"))]
    decode(input, output).unwrap_or(usize::MAX)
}

macro_rules! repeat {
    ($name:ident, $operation:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            input: *const u8, input_len: usize, output: *mut u8, output_len: usize,
            iterations: usize,
        ) -> usize {
            let mut result = 0;
            for _ in 0..iterations {
                result = black_box(unsafe {
                    $operation(black_box(input), black_box(input_len), black_box(output), black_box(output_len))
                });
            }
            result
        }
    };
}

#[cfg(not(size_direction = "decode"))]
repeat!(size_repeat_encode, size_encode);
#[cfg(not(size_direction = "encode"))]
repeat!(size_repeat_decode, size_decode);

/// Repeat decoding without revalidating UTF-8, like a native caller holding `&str`.
///
/// # Safety
/// Pointers must be non-null, aligned for u8, and valid for their stated slice lengths;
/// zero-length slices may use dangling pointers. Nonempty slices must not overlap.
/// The entire input must be UTF-8 and stay unchanged for this call.
#[cfg(not(size_direction = "encode"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn size_repeat_decode_prevalidated(
    input: *const u8, input_len: usize, output: *mut u8, output_len: usize,
    iterations: usize,
) -> usize {
    let input = unsafe { std::slice::from_raw_parts(input, input_len) };
    // SAFETY: UTF-8 and input immutability are part of this export's caller contract.
    let input = unsafe { std::str::from_utf8_unchecked(input) };
    let output = unsafe { std::slice::from_raw_parts_mut(output, output_len) };
    let mut result = 0;
    for _ in 0..iterations {
        #[cfg(size_backend = "empty")]
        { result = black_box((black_box(input).len(), black_box(&mut *output).len())).0; }
        #[cfg(not(size_backend = "empty"))]
        { result = black_box(decode(black_box(input), black_box(&mut *output)).unwrap_or(usize::MAX)); }
    }
    result
}

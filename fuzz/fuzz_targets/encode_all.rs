#![no_main]

#[path = "../../tests/support/mod.rs"]
mod support;

use libfuzzer_sys::fuzz_target;

// Full-width inputs one byte past 64 KiB need 8193 u64 limbs instead of 8192.
const MAX_INPUT_BYTES: usize = 64 * 1024 + 1;
const FIXED_BYTES: usize = 64;
const COL_BASE: u64 = 58_u64.pow(4);
const FIXED_COLUMNS: usize = 22;

unsafe extern "C" {
    fn LLVMFuzzerMutate(data: *mut u8, size: usize, max_size: usize) -> usize;
}

/// Divide a 512-bit big-endian integer by the fixed matrix column base.
fn div_rem_column(number: &mut [u8; FIXED_BYTES]) -> u64 {
    let mut remainder = 0;
    for byte in number {
        let value = remainder * 256 + u64::from(*byte);
        *byte = (value / COL_BASE) as u8;
        remainder = value % COL_BASE;
    }
    remainder
}

/// Multiply a 512-bit big-endian integer by the fixed matrix column base and add `digit`.
fn push_column(number: &mut [u8; FIXED_BYTES], digit: u64) {
    let mut carry = digit;
    for byte in number.iter_mut().rev() {
        let value = u64::from(*byte) * COL_BASE + carry;
        *byte = value as u8;
        carry = value >> 8;
    }
    debug_assert_eq!(carry, 0);
}

/// Build a public 64-byte input for which one split-carry boundary falls back to serial.
fn split_fallback(seed: u32) -> [u8; FIXED_BYTES] {
    let (k, segment) = (
        2 + seed as usize % 5,
        1 + (seed as usize >> 3) % (1 + seed as usize % 5),
    );
    let mut state = u64::from(seed);
    let mut number = [0; FIXED_BYTES];
    for byte in &mut number {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        *byte = (state >> 56) as u8;
    }
    let mut digits = [0; FIXED_COLUMNS];
    for digit in &mut digits {
        *digit = div_rem_column(&mut number);
    }
    let zero = segment * FIXED_COLUMNS / k + 1;
    digits[zero] = 0;
    for digit in digits.iter().rev() {
        push_column(&mut number, *digit);
    }
    #[cfg(debug_assertions)]
    {
        let mut check = number;
        for _ in 0..zero {
            div_rem_column(&mut check);
        }
        debug_assert_eq!(div_rem_column(&mut check), 0);
    }
    number
}

/// Retain libFuzzer's general mutator, but periodically preserve the correlated columns that
/// force the public fixed-width encoder through its serial carry fallback.
///
/// # Safety
/// `data` must be exclusively writable for `max_size` bytes, with `size <= max_size`
/// and its first `size` bytes initialized. libFuzzer must provide `LLVMFuzzerMutate`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LLVMFuzzerCustomMutator(
    data: *mut u8,
    size: usize,
    max_size: usize,
    seed: u32,
) -> usize {
    if max_size < FIXED_BYTES || seed & 7 != 0 {
        // SAFETY: libFuzzer owns `data` for `max_size` bytes and exports this mutator ABI.
        return unsafe { LLVMFuzzerMutate(data, size, max_size) };
    }
    // SAFETY: the branch above proves libFuzzer reserved at least 64 writable bytes.
    unsafe { data.copy_from_nonoverlapping(split_fallback(seed).as_ptr(), FIXED_BYTES) };
    FIXED_BYTES
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let selector = support::scalar::selector(data);
    support::check_encode(data, selector);
    support::check_decode(&bs58::encode(data).into_string(), selector);
});

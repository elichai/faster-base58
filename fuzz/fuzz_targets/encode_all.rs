#![no_main]

#[path = "../../tests/support/mod.rs"]
mod support;

use libfuzzer_sys::fuzz_target;

// Full-width inputs one byte past 64 KiB need 8193 u64 limbs instead of 8192.
const MAX_INPUT_BYTES: usize = 64 * 1024 + 1;
const FIXED_BYTES: usize = 64;
const COL_BASE: u64 = 58_u64.pow(4);
const FIXED_COLUMNS: usize = 22;
// Shorter inputs fit one leaf of the division tree (at least 32 limbs of 10 digits, about
// 234 bytes), so they have no zero leaves or subtrees.
const DIGIT_MIN_BYTES: usize = 256;
// Twice the largest leaf (1,024 limbs).
const MAX_ZERO_RUN: usize = 20_480;
const DIGIT: [u8; 256] = {
    let mut table = [0; 256];
    let mut i = 0;
    while i < 58 {
        table[support::ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    table
};

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

/// Mutate the base-58 digits of `input` instead of its bytes, and return the new bytes if
/// they fit in `max_size`. Byte mutations rewrite every digit below the changed byte, so
/// they almost never make runs of the zero digit `1`, which make whole leaves and subtrees
/// of the encoders' division tree zero. This writes such a run or applies libFuzzer's
/// mutations to the digit values.
fn mutate_digits(input: &[u8], max_size: usize, seed: u32) -> Option<Vec<u8>> {
    let text = faster_base58::Encoder::bitcoin()
        .encode(input)
        .into_string();
    let mut digits: Vec<u8> = text.bytes().map(|c| DIGIT[c as usize]).collect();
    let len = digits.len();
    let mut state = u64::from(seed);
    let mut below = |n: usize| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (state >> 33) as usize % n
    };
    if seed & 8 == 0 {
        let run = 1 + below(MAX_ZERO_RUN.min(len));
        let start = below(len - run + 1);
        digits[start..start + run].fill(0);
    } else {
        let cap = faster_base58::Encoder::bitcoin().max_encoded_len(max_size);
        digits.resize(cap.max(len), 0);
        // SAFETY: `digits` holds `digits.len()` initialized bytes, the first `len` of them
        // the input to mutate.
        let new_len = unsafe { LLVMFuzzerMutate(digits.as_mut_ptr(), len, digits.len()) };
        digits.truncate(new_len);
    }
    let text: Vec<u8> = digits
        .iter()
        .map(|&d| support::ALPHABET[usize::from(d) % 58])
        .collect();
    let bytes = faster_base58::Decoder::bitcoin()
        .decode(&text)
        .into_vec()
        .ok()?;
    (bytes.len() <= max_size).then_some(bytes)
}

/// Retain libFuzzer's general mutator, but one call in eight builds the correlated columns
/// that force the public fixed-width encoder through its serial carry fallback, and two in
/// eight mutate the input's base-58 digits.
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
    match seed & 7 {
        0 if max_size >= FIXED_BYTES => {
            // SAFETY: libFuzzer reserved at least 64 writable bytes.
            unsafe { data.copy_from_nonoverlapping(split_fallback(seed).as_ptr(), FIXED_BYTES) };
            return FIXED_BYTES;
        }
        1 | 2 if size >= DIGIT_MIN_BYTES => {
            // SAFETY: the first `size` bytes of `data` are initialized.
            let input = unsafe { std::slice::from_raw_parts(data, size) };
            if let Some(bytes) = mutate_digits(input, max_size, seed) {
                // SAFETY: `bytes.len() <= max_size`, and `bytes` is a separate allocation.
                unsafe { data.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len()) };
                return bytes.len();
            }
        }
        _ => {}
    }
    // SAFETY: libFuzzer owns `data` for `max_size` bytes and exports this mutator ABI.
    unsafe { LLVMFuzzerMutate(data, size, max_size) }
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let selector = support::scalar::selector(data);
    support::check_round_trip(data, selector);
});

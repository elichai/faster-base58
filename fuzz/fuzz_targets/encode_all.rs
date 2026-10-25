#![no_main]

#[path = "../../tests/support/mod.rs"]
mod support;

use libfuzzer_sys::fuzz_target;

// Full-width inputs one byte past 64 KiB need 8193 u64 limbs instead of 8192.
const MAX_INPUT_BYTES: usize = 64 * 1024 + 1;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let selector = data.first().copied().unwrap_or(0) as usize;
    support::check_encode(data, selector);
    support::check_decode(&bs58::encode(data).into_string(), selector);
});

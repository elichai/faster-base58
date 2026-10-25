#![no_main]

#[path = "../../tests/support/mod.rs"]
mod support;

use libfuzzer_sys::fuzz_target;

// Match the encoded-length budget of encode_all: currently 89,502 raw bytes.
// Lossy UTF-8 conversion and the inserted NUL may produce longer strings.
const MAX_INPUT_BYTES: usize = base58_bench::max_encoded_len(64 * 1024 + 1);

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let selector = data.first().copied().unwrap_or(0) as usize;
    support::check_decode(&String::from_utf8_lossy(data), selector);
    let mut valid: String = data
        .iter()
        .map(|&byte| support::ALPHABET[byte as usize % 58] as char)
        .collect();
    support::check_decode(&valid, selector);
    let at = selector % (valid.len() + 1);
    valid.insert(at, '\0');
    assert_eq!(support::oracle_decode(&valid), Err(at));
    support::check_decode(&valid, selector);
});

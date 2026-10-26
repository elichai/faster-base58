#![no_main]

#[path = "../../tests/support/mod.rs"]
mod support;

use libfuzzer_sys::fuzz_target;

// Match the encoded-length budget of encode_all: currently 89,502 raw bytes.
// Lossy UTF-8 conversion and the inserted NUL may produce longer strings.
const MAX_INPUT_BYTES: usize = faster_base58::max_encoded_len(64 * 1024 + 1);

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let selector = support::scalar::selector(data);
    support::check_decode(&String::from_utf8_lossy(data), selector);
    let mut valid: String = data
        .iter()
        .map(|&byte| support::ALPHABET[byte as usize % 58] as char)
        .collect();
    support::check_decode(&valid, selector);
    let at = selector % (valid.len() + 1);
    // Include multi-byte invalid characters. The expected error is a byte offset,
    // and the position can cover the complete input rather than only 0..255.
    let invalid = ['\0', '0', 'O', 'I', 'l', 'é', '中', '🦀'][selector % 8];
    valid.insert(at, invalid);
    assert_eq!(support::oracle_decode(&valid), Err(at));
    support::check_decode(&valid, selector);
});

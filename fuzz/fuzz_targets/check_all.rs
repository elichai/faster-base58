#![no_main]

#[path = "../../tests/support/mod.rs"]
mod support;

use faster_base58::{BufferTooSmall, Decoder, Encoder, Error};
use libfuzzer_sys::fuzz_target;

const CANARY: u8 = 0xA5;
const GUARD: usize = 8;

// bs58's quadratic oracle bounds useful input sizes.
const MAX_INPUT_BYTES: usize = 8193;

fuzz_target!(|raw: &[u8]| {
    if raw.len() > MAX_INPUT_BYTES {
        return;
    }
    let selector = support::scalar::selector(raw);
    let version = (selector & 1 == 1).then_some((selector >> 1) as u8);
    let (encoder, decoder) = match version {
        Some(v) => (
            Encoder::bitcoin().with_check_version(v),
            Decoder::bitcoin().with_check_version(v),
        ),
        None => (
            Encoder::bitcoin().with_check(),
            Decoder::bitcoin().with_check(),
        ),
    };

    // `raw` as the payload.
    let text = match version {
        Some(v) => bs58::encode(raw).with_check_version(v).into_vec(),
        None => bs58::encode(raw).with_check().into_vec(),
    };
    assert!(text.len() <= encoder.max_encoded_len(raw.len()));
    assert_eq!(encoder.encode(raw).into_vec(), text);
    check_encode(encoder, raw, &text, selector);

    // The valid frame, one changed character, `raw` as text, and `raw` mapped to digits.
    check_decode(decoder, version, &text, selector);
    // A valid checksum must reach the version check, including a missing version.
    let actual = version.or_else(|| raw.first().copied());
    let wrong_version = actual.unwrap_or(0).wrapping_add(1);
    check_decode(
        Decoder::bitcoin().with_check_version(wrong_version),
        Some(wrong_version),
        &text,
        selector,
    );
    let mut changed = text.clone();
    let at = selector % changed.len();
    let digit = support::ALPHABET
        .iter()
        .position(|&c| c == changed[at])
        .unwrap();
    changed[at] = support::ALPHABET[(digit + 1 + selector % 57) % 58];
    check_decode(decoder, version, &changed, selector);
    check_decode(decoder, version, raw, selector);
    let mapped: Vec<u8> = raw
        .iter()
        .map(|&b| support::ALPHABET[b as usize % 58])
        .collect();
    check_decode(decoder, version, &mapped, selector);
});

/// bs58 with checksum but without its version check, then our version rule. bs58
/// compares the version with `output[0]` even for empty data, which is then the first
/// checksum byte; we report a missing version instead.
fn oracle(text: &[u8], version: Option<u8>) -> Result<Vec<u8>, Error> {
    let data = bs58::decode(text)
        .with_check(None)
        .into_vec()
        .map_err(|error| match error {
            bs58::decode::Error::InvalidCharacter { index, .. }
            | bs58::decode::Error::NonAsciiCharacter { index } => Error::InvalidCharacter { index },
            bs58::decode::Error::NoChecksum => Error::MissingChecksum,
            bs58::decode::Error::InvalidChecksum { .. } => Error::InvalidChecksum,
            error => panic!("unexpected bs58 error: {error:?}"),
        })?;
    if let Some(expected) = version {
        let actual = data.first().copied();
        if actual != Some(expected) {
            return Err(Error::InvalidVersion { expected, actual });
        }
    }
    Ok(data)
}

fn guarded(capacity: usize) -> Vec<u8> {
    vec![CANARY; GUARD + capacity + GUARD]
}

fn assert_guards(storage: &[u8], capacity: usize, written: usize, what: &str) {
    assert!(
        storage[..GUARD].iter().all(|&b| b == CANARY),
        "{what}: prefix"
    );
    assert!(
        storage[GUARD + written..].iter().all(|&b| b == CANARY),
        "{what}: wrote past {written} of {capacity}"
    );
}

fn check_encode(encoder: Encoder, data: &[u8], want: &[u8], selector: usize) {
    let n = want.len();
    for capacity in [0, n - 1, n, n + 17, selector % (n + 18)] {
        let mut storage = guarded(capacity);
        let result = encoder
            .encode(data)
            .onto(&mut storage[GUARD..GUARD + capacity])
            .map(str::len);
        if capacity < n {
            assert_eq!(result, Err(BufferTooSmall), "encode into {capacity}");
            assert_guards(&storage, capacity, capacity, "encode error");
        } else {
            assert_eq!(result, Ok(n), "encode into {capacity}");
            assert_eq!(&storage[GUARD..GUARD + n], want, "encoded text");
            assert_guards(&storage, capacity, n, "encode");
        }
    }
}

fn check_decode(decoder: Decoder, version: Option<u8>, text: &[u8], selector: usize) {
    let want = oracle(text, version);
    assert_eq!(decoder.decode(text).into_vec(), want, "owned decode");
    // Data bytes the frame carries, when it is valid Base58 with a checksum.
    let data_len = bs58::decode(text)
        .into_vec()
        .ok()
        .and_then(|f| f.len().checked_sub(4));
    let n = want.as_ref().map_or(text.len(), Vec::len);
    for capacity in [0, n.saturating_sub(1), n, n + 17, selector % (n + 18)] {
        let mut storage = guarded(capacity);
        let result = decoder
            .decode(text)
            .onto(&mut storage[GUARD..GUARD + capacity])
            .map(|bytes| bytes.len());
        match &want {
            Ok(bytes) if capacity >= n => {
                assert_eq!(result, Ok(n), "decode into {capacity}");
                assert_eq!(&storage[GUARD..GUARD + n], bytes, "decoded data");
                assert_guards(&storage, capacity, n, "decode");
            }
            Ok(_) => {
                assert_eq!(result, Err(Error::BufferTooSmall), "short decode");
                assert_guards(&storage, capacity, capacity, "short decode");
            }
            Err(error) => {
                // Capacity is checked before the checksum; for invalid characters the
                // precedence is unspecified.
                let short = match error {
                    Error::InvalidCharacter { .. } => capacity < text.len(),
                    _ => data_len.is_some_and(|len| capacity < len),
                };
                assert!(
                    result == Err(*error) || short && result == Err(Error::BufferTooSmall),
                    "{result:?}, expected {error:?}, capacity {capacity}"
                );
                assert_guards(&storage, capacity, capacity, "decode error");
            }
        }
    }
}

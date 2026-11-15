#![cfg_attr(not(test), no_main)]

#[path = "../../tests/support/mod.rs"]
mod support;

#[cfg(not(test))]
use libfuzzer_sys::fuzz_target;

// Match the encoded-length budget of encode_all: currently 89,502 raw bytes.
// Lossy UTF-8 conversion and the inserted NUL may produce longer strings.
#[cfg(not(test))]
const MAX_INPUT_BYTES: usize = faster_base58::Encoder::bitcoin().max_encoded_len(64 * 1024 + 1);

fn check_decode_array<const N: usize>(input: &[u8], want: &Result<Vec<u8>, usize>) {
    use faster_base58::{Decoder, Error};

    let result = Decoder::bitcoin().decode(input).into_array::<N>();
    match want {
        Ok(bytes) if bytes.len() == N => {
            assert_eq!(
                result.as_ref().map(|array| array.as_slice()),
                Ok(bytes.as_slice()),
                "array output"
            );
        }
        Ok(bytes) if bytes.len() < N => assert_eq!(
            result,
            Err(Error::InvalidLength {
                expected: N,
                actual: bytes.len()
            }),
            "array length"
        ),
        Ok(_) => assert_eq!(result, Err(Error::BufferTooSmall), "array short buffer"),
        Err(index) => {
            let invalid = Err(Error::InvalidCharacter { index: *index });
            // Error precedence is unspecified when the buffer is also too small.
            assert!(
                result == invalid || (N < input.len() && result == Err(Error::BufferTooSmall)),
                "array: {result:?}, expected invalid byte {index}, capacity {N}"
            );
        }
    }
}

fn check_decode_array_prefix(input: &[u8], selector: usize) {
    // Keep the additional oracle work bounded even for the largest fuzz inputs.
    let input = &input[..input.len().min(129)];
    let want = support::oracle_decode_bytes(input);
    match selector % 4 {
        0 => check_decode_array::<0>(input, &want),
        1 => check_decode_array::<1>(input, &want),
        2 => check_decode_array::<32>(input, &want),
        _ => check_decode_array::<64>(input, &want),
    }
}

#[cfg(not(test))]
fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    let selector = support::scalar::selector(data);
    check_decode_array_prefix(data, selector);
    // The public decoder takes raw bytes; backend hooks take lossy `&str`.
    support::check_decode_bytes(data, selector);
    support::check_decode(&String::from_utf8_lossy(data), selector);
    let mut valid: String = data
        .iter()
        .map(|&byte| support::ALPHABET[byte as usize % 58] as char)
        .collect();
    check_decode_array_prefix(valid.as_bytes(), selector);
    support::check_decode(&valid, selector);
    let at = selector % (valid.len() + 1);
    // Include multi-byte invalid characters. The expected error is a byte offset,
    // and the position can cover the complete input rather than only 0..255.
    let invalid = ['\0', '0', 'O', 'I', 'l', 'é', '中', '🦀'][selector % 8];
    valid.insert(at, invalid);
    assert_eq!(support::oracle_decode(&valid), Err(at));
    support::check_decode(&valid, selector);
});

#[cfg(test)]
mod tests {
    use super::{check_decode_array, check_decode_array_prefix};

    #[test]
    fn array_fuzz_assertions_cover_lengths_and_invalid_bytes() {
        check_decode_array::<0>(b"", &Ok(vec![]));
        check_decode_array::<1>(b"2", &Ok(vec![1]));
        check_decode_array::<32>(&[b'1'; 32], &Ok(vec![0; 32]));
        check_decode_array::<64>(&[b'1'; 64], &Ok(vec![0; 64]));
        check_decode_array::<32>(b"2", &Ok(vec![1]));
        check_decode_array::<1>(b"112", &Ok(vec![0, 0, 1]));
        check_decode_array::<0>(b"1", &Ok(vec![0]));
        check_decode_array::<64>(b"12\xff", &Err(2));
        check_decode_array::<1>(b"112\xff", &Err(3));
    }

    #[test]
    fn array_fuzz_prefix_covers_every_capacity_and_stops_before_invalid_suffix() {
        let mut input = vec![b'1'; 129];
        input.push(0xff);
        for selector in 0..4 {
            check_decode_array_prefix(b"", selector);
            check_decode_array_prefix(b"2", selector);
            check_decode_array_prefix(&[b'1'; 32], selector);
            check_decode_array_prefix(&[b'1'; 64], selector);
            check_decode_array_prefix(&input, selector);
            check_decode_array_prefix(b"12\xff", selector);
        }
    }

    #[test]
    #[should_panic(expected = "array output")]
    fn array_fuzz_assertions_reject_wrong_output() {
        check_decode_array::<1>(b"2", &Ok(vec![2]));
    }

    #[test]
    #[should_panic(expected = "invalid byte")]
    fn array_fuzz_assertions_reject_wrong_invalid_index() {
        check_decode_array::<64>(b"12\xff", &Err(1));
    }
}

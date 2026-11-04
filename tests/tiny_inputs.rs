use faster_base58::{BufferTooSmall, Error};

const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn check_encode(input: &[u8]) {
    let expected = bs58::encode(input).into_string();
    for capacity in 0..=expected.len() + 1 {
        let mut buffer = [0xA5; 8];
        let result = faster_base58::Encoder::bitcoin()
            .encode(input)
            .onto(&mut buffer[1..1 + capacity])
            .map(str::len);
        if capacity < expected.len() {
            assert_eq!(result, Err(BufferTooSmall));
        } else {
            assert_eq!(result, Ok(expected.len()));
            assert_eq!(&buffer[1..1 + expected.len()], expected.as_bytes());
            assert!(buffer[1 + expected.len()..].iter().all(|&b| b == 0xA5));
        }
        assert_eq!(buffer[0], 0xA5);
        assert!(buffer[1 + capacity..].iter().all(|&b| b == 0xA5));
    }
}

fn check_decode(input: &str) {
    let expected = bs58::decode(input).into_vec().unwrap();
    for capacity in 0..=expected.len() + 1 {
        let mut buffer = [0xA5; 8];
        let result = faster_base58::Decoder::bitcoin()
            .decode(input)
            .onto(&mut buffer[1..1 + capacity])
            .map(|bytes| bytes.len());
        if capacity < expected.len() {
            assert_eq!(result, Err(Error::BufferTooSmall));
        } else {
            assert_eq!(result, Ok(expected.len()));
            assert_eq!(&buffer[1..1 + expected.len()], expected.as_slice());
            assert!(buffer[1 + expected.len()..].iter().all(|&b| b == 0xA5));
        }
        assert_eq!(buffer[0], 0xA5);
        assert!(buffer[1 + capacity..].iter().all(|&b| b == 0xA5));
    }
}

#[test]
fn every_byte_and_short_text_matches_bs58() {
    check_encode(&[]);
    check_decode("");
    for value in 0..=255 {
        let input = [value];
        check_encode(&input);
        check_decode(&bs58::encode(input).into_string());
    }
    for &first in ALPHABET {
        check_decode(std::str::from_utf8(&[first]).unwrap());
        for &second in ALPHABET {
            check_decode(std::str::from_utf8(&[first, second]).unwrap());
        }
    }
}

#[test]
fn tiny_invalid_inputs_keep_byte_offsets() {
    for (input, index) in [
        ("0", 0),
        ("O", 0),
        ("1I", 1),
        ("l1", 0),
        ("é", 0),
        ("1é", 1),
    ] {
        let mut buffer = [0xA5; 4];
        assert!(
            matches!(faster_base58::Decoder::bitcoin().decode(input).onto(&mut buffer), Err(Error::InvalidCharacter { index: actual, .. }) if actual == index)
        );
    }
}

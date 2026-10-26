//! These properties compile without the internal backend feature.

#[path = "support/scalar.rs"]
mod scalar;

use faster_base58::{BufferTooSmall, Error};
use proptest::prelude::*;

type Encode = fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall>;
type Decode = fn(&str, &mut [u8]) -> Result<usize, Error>;

const ENCODERS: &[Encode] = &[
    faster_base58::encode,
    faster_base58::no_alloc::encode,
    |input, output| faster_base58::encode_into(input, output).map(str::len),
    |input, output| faster_base58::no_alloc::encode_into(input, output).map(str::len),
];
const DECODERS: &[Decode] = &[
    faster_base58::decode,
    faster_base58::no_alloc::decode,
    |input, output| faster_base58::decode_into(input, output).map(|bytes| bytes.len()),
    |input, output| faster_base58::no_alloc::decode_into(input, output).map(|bytes| bytes.len()),
];

fn check(input: &[u8], selector: usize) {
    let want = scalar::encode(input).expect("generated numeric value fits u128");
    let text = std::str::from_utf8(&want).unwrap();
    assert_eq!(bs58::encode(input).into_vec(), want);
    assert_eq!(faster_base58::encode_string(input).as_bytes(), want);
    assert_eq!(faster_base58::decode_vec(text), Ok(input.to_vec()));
    assert_eq!(scalar::decode(text), Some(Ok(input.to_vec())));
    let zeros = input.iter().take_while(|&&byte| byte == 0).count();
    assert_eq!(want.iter().take_while(|&&byte| byte == b'1').count(), zeros);
    assert!(want.len() <= faster_base58::max_encoded_len(input.len()));
    for capacity in [
        0,
        want.len().saturating_sub(1),
        want.len(),
        want.len() + 9,
        selector % (want.len() + 10),
    ] {
        for encode in ENCODERS {
            let mut output = vec![0xA5; capacity + 2];
            let result = encode(input, &mut output[1..1 + capacity]);
            assert_eq!((output[0], output[capacity + 1]), (0xA5, 0xA5));
            if capacity < want.len() {
                assert_eq!(result, Err(BufferTooSmall));
            } else {
                assert_eq!(result, Ok(want.len()));
                assert_eq!(&output[1..1 + want.len()], want);
                assert!(output[1 + want.len()..].iter().all(|&byte| byte == 0xA5));
            }
        }
    }
    for capacity in [
        0,
        input.len().saturating_sub(1),
        input.len(),
        input.len() + 9,
        selector % (input.len() + 10),
    ] {
        for decode in DECODERS {
            let mut output = vec![0xA5; capacity + 2];
            let result = decode(text, &mut output[1..1 + capacity]);
            assert_eq!((output[0], output[capacity + 1]), (0xA5, 0xA5));
            if capacity < input.len() {
                assert_eq!(result, Err(Error::BufferTooSmall));
            } else {
                assert_eq!(result, Ok(input.len()));
                assert_eq!(&output[1..1 + input.len()], input);
                assert!(output[1 + input.len()..].iter().all(|&byte| byte == 0xA5));
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::default())]

    #[test]
    fn independent_scalar_values_preserve_zero_prefix_and_capacities(
        suffix in prop::collection::vec(any::<u8>(), 0..17),
        width in prop::sample::select(vec![0usize, 1, 21, 22, 23, 31, 32, 33, 63, 64, 65, 127, 128, 129, 511, 512, 513, 8191, 8192, 8193]),
        capacity in any::<usize>(),
    ) {
        let mut input = vec![0; width];
        let used = suffix.len().min(width);
        input[width - used..].copy_from_slice(&suffix[..used]);
        check(&input, capacity);
    }

    #[test]
    fn independent_scalar_radix_digits_decode(
        digits in prop::collection::vec(0u8..58, 0..22), zeros in 0usize..96,
        capacity in any::<usize>(),
    ) {
        // 58^21 < 2^128; the zero prefix does not change the numeric value.
        let text: String = std::iter::repeat_n('1', zeros)
            .chain(digits.into_iter().map(|d| scalar::ALPHABET[d as usize] as char)).collect();
        let input = scalar::decode(&text).unwrap().unwrap();
        assert_eq!(scalar::encode(&input).unwrap(), text.as_bytes());
        check(&input, capacity);
    }

    #[test]
    fn unicode_invalid_indices_are_byte_offsets(
        digits in prop::collection::vec(0u8..58, 0..2049), position in any::<usize>(),
        invalid in prop::sample::select(vec!['\0', '0', 'O', 'I', 'l', 'é', '中', '🦀', '\u{7f}', '\u{fffd}']),
    ) {
        let mut text: String = digits.into_iter().map(|d| scalar::ALPHABET[d as usize] as char).collect();
        let index = position % (text.len() + 1);
        text.insert(index, invalid);
        // A later, different invalid UTF-8 character must not change the first index.
        text.push('🦀');
        assert_eq!(scalar::decode(&text), Some(Err(index)));
        for decode in DECODERS {
            let mut output = vec![0xA5; text.len() + 2];
            assert_eq!(decode(&text, &mut output[1..1 + text.len()]), Err(Error::InvalidCharacter { index }));
            assert_eq!((output[0], output[text.len() + 1]), (0xA5, 0xA5));
        }
    }

    #[test]
    fn reused_output_and_dispatch_history_preserve_the_unwritten_tail(
        history in prop::collection::vec((prop::sample::select(vec![22usize, 32, 64, 65, 512, 768, 1504, 8192, 8193]), prop::array::uniform16(any::<u8>())), 1..6),
    ) {
        let mut encoded = vec![0xA5; faster_base58::max_encoded_len(8193) + 9];
        let mut decoded = vec![0x5A; 8193 + 9];
        for (width, pattern) in history {
            let mut input: Vec<u8> = (0..width).map(|i| pattern[i % pattern.len()]).collect();
            // Significant input reaches actual matrix/recursive/NTT setup. The
            // scalar properties separately cover long zero prefixes.
            input[0] |= 1;
            let want = bs58::encode(&input).into_vec();
            let old = encoded.clone();
            assert_eq!(faster_base58::encode(&input, &mut encoded), Ok(want.len()));
            assert_eq!(&encoded[..want.len()], want);
            assert_eq!(&encoded[want.len()..], &old[want.len()..]);
            let old = decoded.clone();
            let text = std::str::from_utf8(&want).unwrap();
            assert_eq!(faster_base58::decode(text, &mut decoded), Ok(width));
            assert_eq!(&decoded[..width], input);
            assert_eq!(&decoded[width..], &old[width..]);
        }
    }
}

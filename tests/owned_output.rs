//! Owned decoding must keep its tighter allocation large enough for every value.

#[path = "support/scalar.rs"]
mod scalar;

use faster_base58::Error;
use proptest::prelude::*;

fn check(input: &str, expected: &[u8]) {
    let output = faster_base58::decode_vec(input).unwrap();
    assert_eq!(output, expected);
    assert!(output.capacity() <= input.len());
    let zeros = input.bytes().take_while(|&byte| byte == b'1').count();
    let digits = input.len() - zeros;
    let bound = zeros + ((digits as u128 * 47).div_ceil(64) as usize) + usize::from(digits != 0);
    assert!(output.capacity() <= bound);
}

#[test]
fn maximum_digit_values_fit_at_dispatch_and_rounding_boundaries() {
    for digits in [
        0, 1, 2, 3, 4, 9, 10, 11, 21, 22, 43, 44, 63, 64, 65, 87, 88, 127, 128, 129, 575, 576, 577,
        999, 1000, 1001, 1023, 1024, 1025, 5119, 5120, 5121, 5631, 5632, 5633, 8191, 8192, 8193,
        16384,
    ] {
        for zeros in [0, 1, 6000] {
            let input = "1".repeat(zeros) + &"z".repeat(digits);
            let expected = bs58::decode(&input).into_vec().unwrap();
            check(&input, &expected);
        }
    }
}

#[test]
fn late_invalid_bytes_are_reported_after_long_zero_and_numeric_prefixes() {
    for significant in [0, 1, 2, 575, 576, 1024, 5632, 5633, 8193] {
        for zeros in [0, 1, 6000] {
            let prefix = "1".repeat(zeros) + &"z".repeat(significant);
            for invalid in ['\0', '0', 'O', 'I', 'l', 'é', '中', '🦀'] {
                let mut input = prefix.clone();
                input.push(invalid);
                input.push('🦀');
                assert_eq!(
                    faster_base58::decode_vec(&input),
                    Err(Error::InvalidCharacter {
                        index: prefix.len()
                    }),
                    "significant={significant}, zeros={zeros}, invalid={invalid:?}"
                );
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::default())]

    #[test]
    fn arbitrary_radix_digits_and_zero_prefixes_fit(
        digits in prop::collection::vec(0u8..58, 0..2049),
        zeros in 0usize..2049,
    ) {
        let input: String = std::iter::repeat_n('1', zeros)
            .chain(digits.into_iter().map(|digit| scalar::ALPHABET[digit as usize] as char))
            .collect();
        let expected = bs58::decode(&input).into_vec().unwrap();
        if let Some(scalar) = scalar::decode(&input) {
            prop_assert_eq!(&expected, &scalar.unwrap());
        }
        check(&input, &expected);
    }

    #[test]
    fn independent_scalar_values_with_long_zero_prefixes_fit(
        suffix in prop::collection::vec(any::<u8>(), 0..17),
        zeros in prop::sample::select(vec![0usize, 1, 575, 576, 1024, 5632, 5633, 8193]),
    ) {
        let mut expected = vec![0; zeros];
        expected.extend_from_slice(&suffix);
        let input = scalar::encode(&expected).unwrap();
        let input = core::str::from_utf8(&input).unwrap();
        check(input, &expected);
    }
}

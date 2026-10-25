use base58_bench::{BufferTooSmall, Error};

#[test]
fn borrowed_and_owned_outputs_agree() {
    for len in [0, 1, 2, 21, 22, 31, 32, 33, 63, 64, 65, 512, 4096, 8193, 32769] {
        let mut data: Vec<u8> = (0..len).map(|i| (i * 73 + 19) as u8).collect();
        let leading = len.min(5);
        data[..leading].fill(0);
        let want = bs58::encode(&data).into_string();
        assert_eq!(base58_bench::encode_string(&data), want);
        let mut encoded = vec![0xA5; base58_bench::max_encoded_len(len) + 7];
        let text = base58_bench::encode_into(&data, &mut encoded).unwrap();
        assert_eq!(text, want);
        assert!(encoded[want.len()..].iter().all(|&b| b == 0xA5));

        assert_eq!(base58_bench::decode_vec(&want), Ok(data.clone()));
        let mut decoded = vec![0xA5; len + 7];
        assert_eq!(base58_bench::decode_into(&want, &mut decoded).unwrap(), data);
        assert!(decoded[len..].iter().all(|&b| b == 0xA5));
    }
}

#[test]
fn arrays_require_exact_decoded_length() {
    for data in [[0; 32], [0xFF; 32], std::array::from_fn(|i| i as u8)] {
        let text = base58_bench::encode_string(data);
        assert_eq!(base58_bench::decode_array::<32>(&text), Ok(data));
    }
    let data = [0xA5; 64];
    let mut buffer = [0; 88];
    let text = base58_bench::encode_into(data, &mut buffer).unwrap();
    assert_eq!(base58_bench::decode_array::<64>(text), Ok(data));
    assert_eq!(base58_bench::decode_array::<0>(""), Ok([]));
    assert_eq!(base58_bench::decode_array::<2>("2"), Err(Error::InvalidLength { expected: 2, actual: 1 }));
    assert_eq!(base58_bench::decode_array::<1>("112"), Err(Error::BufferTooSmall));
    assert_eq!(base58_bench::decode_array::<4>("12O4"), Err(Error::InvalidCharacter { index: 2 }));
}

#[test]
fn errors_are_usable_with_standard_error_types() {
    let errors: [&dyn std::error::Error; 4] = [
        &BufferTooSmall,
        &Error::BufferTooSmall,
        &Error::InvalidCharacter { index: 3 },
        &Error::InvalidLength { expected: 32, actual: 31 },
    ];
    assert_eq!(errors[0].to_string(), "output buffer is too small");
    assert!(errors[2].to_string().contains('3'));
    assert!(errors[3].to_string().contains("32"));
    assert_eq!(base58_bench::encode_into([1, 2, 3], &mut []), Err(BufferTooSmall));
    assert_eq!(base58_bench::decode_vec("é"), Err(Error::InvalidCharacter { index: 0 }));
}

use faster_base58::{BufferTooSmall, Error};

#[test]
fn borrowed_and_owned_outputs_agree() {
    for len in [
        0, 1, 2, 21, 22, 31, 32, 33, 63, 64, 65, 512, 4096, 8193, 32769,
    ] {
        let mut data: Vec<u8> = (0..len).map(|i| (i * 73 + 19) as u8).collect();
        let leading = len.min(5);
        data[..leading].fill(0);
        let want = bs58::encode(&data).into_string();
        assert_eq!(faster_base58::encode_string(&data), want);
        let mut encoded = vec![0xA5; faster_base58::max_encoded_len(len) + 7];
        let text = faster_base58::encode_into(&data, &mut encoded).unwrap();
        assert_eq!(text, want);
        assert!(encoded[want.len()..].iter().all(|&b| b == 0xA5));

        assert_eq!(faster_base58::decode_vec(&want), Ok(data.clone()));
        let mut decoded = vec![0xA5; len + 7];
        assert_eq!(
            faster_base58::decode_into(&want, &mut decoded).unwrap(),
            data
        );
        assert!(decoded[len..].iter().all(|&b| b == 0xA5));
    }
}

#[test]
fn arrays_require_exact_decoded_length() {
    for data in [[0; 32], [0xFF; 32], std::array::from_fn(|i| i as u8)] {
        let text = faster_base58::encode_string(data);
        assert_eq!(faster_base58::decode_array::<32>(&text), Ok(data));
    }
    let data = [0xA5; 64];
    let mut buffer = [0; 88];
    let text = faster_base58::encode_into(data, &mut buffer).unwrap();
    assert_eq!(faster_base58::decode_array::<64>(text), Ok(data));
    assert_eq!(faster_base58::decode_array::<0>(""), Ok([]));
    assert_eq!(
        faster_base58::decode_array::<2>("2"),
        Err(Error::InvalidLength {
            expected: 2,
            actual: 1
        })
    );
    assert_eq!(
        faster_base58::decode_array::<1>("112"),
        Err(Error::BufferTooSmall)
    );
    assert_eq!(
        faster_base58::decode_array::<4>("12O4"),
        Err(Error::InvalidCharacter { index: 2 })
    );
}

#[test]
fn dispatch_boundaries_preserve_zero_prefixes_and_capacities() {
    for significant in [
        0, 1, 21, 22, 23, 31, 32, 33, 63, 64, 65, 511, 512, 513, 3583, 3584, 3585, 4095, 4096,
        4097, 8191, 8192, 8193,
    ] {
        for zeros in [0, 1, 6000] {
            let mut input = vec![0; zeros];
            input.extend((0..significant).map(|i| (i * 73 + 19) as u8));
            let want = bs58::encode(&input).into_vec();
            for capacity in [0, want.len().saturating_sub(1), want.len(), want.len() + 7] {
                let mut output = vec![0xA5; capacity + 2];
                let result = faster_base58::encode(&input, &mut output[1..1 + capacity]);
                if capacity < want.len() {
                    assert_eq!(result, Err(BufferTooSmall));
                } else {
                    assert_eq!(result, Ok(want.len()));
                    assert_eq!(&output[1..1 + want.len()], want);
                    assert!(output[1 + want.len()..].iter().all(|&b| b == 0xA5));
                }
                assert_eq!((output[0], output[capacity + 1]), (0xA5, 0xA5));
            }
        }
    }
    for significant in [
        0, 1, 2, 575, 576, 577, 1023, 1024, 1025, 5119, 5120, 5121, 5631, 5632, 5633,
    ] {
        for zeros in [0, 1, 6000] {
            let mut text = "1".repeat(zeros);
            text.push_str(&"z".repeat(significant));
            let want = bs58::decode(&text).into_vec().unwrap();
            for capacity in [0, want.len().saturating_sub(1), want.len(), want.len() + 7] {
                let mut output = vec![0xA5; capacity + 2];
                let result = faster_base58::decode(&text, &mut output[1..1 + capacity]);
                if capacity < want.len() {
                    assert_eq!(result, Err(Error::BufferTooSmall));
                } else {
                    assert_eq!(result, Ok(want.len()));
                    assert_eq!(&output[1..1 + want.len()], want);
                    assert!(output[1 + want.len()..].iter().all(|&b| b == 0xA5));
                }
                assert_eq!((output[0], output[capacity + 1]), (0xA5, 0xA5));
            }
            let index = zeros + significant / 2;
            text.insert(index, 'é');
            assert_eq!(
                faster_base58::decode(&text, &mut vec![0; text.len()]),
                Err(Error::InvalidCharacter { index })
            );
        }
    }
}

#[test]
fn errors_are_usable_with_standard_error_types() {
    let errors: [&dyn std::error::Error; 4] = [
        &BufferTooSmall,
        &Error::BufferTooSmall,
        &Error::InvalidCharacter { index: 3 },
        &Error::InvalidLength {
            expected: 32,
            actual: 31,
        },
    ];
    assert_eq!(errors[0].to_string(), "output buffer is too small");
    assert!(errors[2].to_string().contains('3'));
    assert!(errors[3].to_string().contains("32"));
    assert_eq!(
        faster_base58::encode_into([1, 2, 3], &mut []),
        Err(BufferTooSmall)
    );
    assert_eq!(
        faster_base58::decode_vec("é"),
        Err(Error::InvalidCharacter { index: 0 })
    );
}

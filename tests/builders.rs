#![cfg(feature = "alloc")]

use faster_base58::{BufferTooSmall, Decoder, Encoder, Error};

const ENCODER: Encoder = Encoder::bitcoin();
const DECODER: Decoder = Decoder::bitcoin();
const INPUT: faster_base58::EncodeInput<&[u8]> = ENCODER.encode(&[0, 1, 2, 3]);

#[test]
fn plain_configuration_is_const_reusable_and_matches_direct_api() {
    let want = faster_base58::Encoder::bitcoin()
        .encode([0, 1, 2, 3])
        .into_string();
    assert_eq!(INPUT.into_string(), want);
    for data in [vec![], vec![0; 32], vec![0xff; 64], vec![42; 512]] {
        let want = faster_base58::Encoder::bitcoin()
            .encode(&data)
            .into_string();
        assert_eq!(ENCODER.encode(&data).into_string(), want);
        assert_eq!(ENCODER.encode(data.clone()).into_vec(), want.as_bytes());
        assert_eq!(DECODER.decode(want.clone()).into_vec(), Ok(data.clone()));
        let mut encoded = vec![0xa5; ENCODER.max_encoded_len(data.len()) + 1];
        let text = ENCODER.encode(&data).onto(&mut encoded).unwrap();
        assert_eq!(text, want);
        let mut decoded = vec![0xa5; data.len() + 1];
        assert_eq!(DECODER.decode(text).onto(&mut decoded).unwrap(), data);
        assert_eq!(encoded[want.len()], 0xa5);
        assert_eq!(decoded[data.len()], 0xa5);
    }
    assert_eq!(DECODER.decode("112").into_array::<3>(), Ok([0, 0, 1]));
    assert_eq!(DECODER.decode("").into_array::<0>(), Ok([]));
    assert_eq!(
        DECODER.decode("112").into_array::<4>(),
        Err(Error::InvalidLength {
            expected: 4,
            actual: 3
        })
    );
    assert_eq!(
        DECODER.decode("112").into_array::<2>(),
        Err(Error::BufferTooSmall)
    );
    assert_eq!(
        DECODER.decode("1é").into_vec(),
        Err(Error::InvalidCharacter { index: 1 })
    );
    assert_eq!(ENCODER.encode([42]).onto(&mut []), Err(BufferTooSmall));
    assert_eq!(ENCODER.max_encoded_len(usize::MAX), usize::MAX);
}

#[test]
fn byte_input_needs_no_utf8_and_reports_byte_offsets() {
    let data = [0, 1, 2, 3];
    let text = ENCODER.encode(data).into_string();
    let mut bytes = text.into_bytes();
    assert_eq!(DECODER.decode(&bytes).into_vec(), Ok(data.to_vec()));
    assert_eq!(DECODER.decode(bytes.as_slice()).into_array::<4>(), Ok(data));
    // Bytes that are not UTF-8, at the start, middle and end.
    for (at, bad) in [(0, 0x80), (2, 0xC3), (bytes.len(), 0xFF)] {
        bytes.insert(at, bad);
        let want = Error::InvalidCharacter { index: at };
        assert_eq!(DECODER.decode(&bytes).into_vec(), Err(want));
        assert_eq!(
            DECODER
                .decode(&bytes)
                .onto(&mut [0; 8])
                .map(|bytes| bytes.len()),
            Err(want)
        );
        bytes.remove(at);
    }
    assert_eq!(Error::from(BufferTooSmall), Error::BufferTooSmall);
}

#[cfg(feature = "check")]
mod checked {
    use super::*;
    use proptest::prelude::*;

    const ENCODER: Encoder = Encoder::bitcoin().with_check();
    const DECODER: Decoder = Decoder::bitcoin().with_check();
    const VERSIONED_ENCODER: Encoder = Encoder::bitcoin().with_check_version(42);
    const VERSIONED_DECODER: Decoder = Decoder::bitcoin().with_check_version(42);

    #[test]
    fn known_answers_cover_empty_data_and_zero_prefixes() {
        for (data, text) in [
            (&b""[..], "3QJmnh"),
            (&[0x2d, 0x31][..], "PWEu9GGN"),
            (&[0; 21][..], "1111111111111111111114oLvT2"),
        ] {
            assert_eq!(ENCODER.encode(data).into_string(), text);
            assert_eq!(ENCODER.encode(data).into_vec(), text.as_bytes());
            assert_eq!(DECODER.decode(text).into_vec().unwrap(), data);
        }
    }

    #[test]
    fn versions_are_prepended_validated_and_retained() {
        for version in [0, 1, 42, 255] {
            for data in [&b""[..], &b"hello"[..], &[0; 32][..]] {
                let encoder = Encoder::bitcoin().with_check_version(version);
                let decoder = Decoder::bitcoin().with_check_version(version);
                let text = encoder.encode(data).into_string();
                assert_eq!(
                    text,
                    bs58::encode(data).with_check_version(version).into_string()
                );
                let mut want = vec![version];
                want.extend_from_slice(data);
                assert_eq!(decoder.decode(&text).into_vec().unwrap(), want);
                assert_eq!(DECODER.decode(&text).into_vec().unwrap(), want);
                assert_eq!(
                    decoder.decode(&text).into_vec().unwrap(),
                    bs58::decode(&text)
                        .with_check(Some(version))
                        .into_vec()
                        .unwrap()
                );
                let wrong = version.wrapping_add(1);
                assert_eq!(
                    Decoder::bitcoin()
                        .with_check_version(wrong)
                        .decode(&text)
                        .into_vec(),
                    Err(Error::InvalidVersion {
                        expected: wrong,
                        actual: Some(version)
                    })
                );
            }
        }
        let text = VERSIONED_ENCODER.encode(b"hello").into_string();
        assert_eq!(
            VERSIONED_DECODER.decode(text).into_array::<6>(),
            Ok(*b"*hello")
        );
        assert_eq!(
            VERSIONED_DECODER.decode("3QJmnh").into_vec(),
            Err(Error::InvalidVersion {
                expected: 42,
                actual: None
            })
        );
        // Setters replace their previous setting, including the optional version.
        assert_eq!(
            VERSIONED_ENCODER.with_check().encode([]).into_string(),
            "3QJmnh"
        );
        assert_eq!(
            VERSIONED_DECODER.with_check().decode("3QJmnh").into_vec(),
            Ok(vec![])
        );
        assert_eq!(
            ENCODER.with_check_version(42).encode([]).into_string(),
            VERSIONED_ENCODER.encode([]).into_string()
        );
    }

    #[test]
    fn large_checked_frames_reject_corruption_before_version_validation() {
        for len in [512, 4096, 8193] {
            let data: Vec<u8> = (0..len).map(|i| (i * 73 + 19) as u8).collect();
            for version in [None, Some(42)] {
                let text = match version {
                    Some(v) => bs58::encode(&data).with_check_version(v).into_string(),
                    None => bs58::encode(&data).with_check().into_string(),
                };
                let decoder = version.map_or(DECODER, |v| Decoder::bitcoin().with_check_version(v));
                let frame = bs58::decode(&text).into_vec().unwrap();
                let mut output = vec![0; frame.len() - 4];
                for index in [0, frame.len() - 4, frame.len() - 1] {
                    let mut corrupt = frame.clone();
                    corrupt[index] ^= 1;
                    let corrupt = bs58::encode(corrupt).into_string();
                    assert_eq!(
                        decoder.decode(&corrupt).onto(&mut output),
                        Err(Error::InvalidChecksum)
                    );
                    assert_eq!(
                        decoder.decode(&corrupt).into_vec(),
                        Err(Error::InvalidChecksum)
                    );
                    assert_eq!(
                        VERSIONED_DECODER
                            .with_check_version(43)
                            .decode(&corrupt)
                            .onto(&mut output),
                        Err(Error::InvalidChecksum)
                    );
                }
                if version.is_some() {
                    assert_eq!(
                        Decoder::bitcoin()
                            .with_check_version(43)
                            .decode(&text)
                            .onto(&mut output),
                        Err(Error::InvalidVersion {
                            expected: 43,
                            actual: Some(42)
                        })
                    );
                }
            }
        }
    }

    #[test]
    fn malformed_data_and_each_checksum_byte_are_rejected() {
        for len in 0..4 {
            assert_eq!(
                DECODER.decode("1".repeat(len)).into_vec(),
                Err(Error::MissingChecksum)
            );
        }
        assert_eq!(
            DECODER.decode("0").into_vec(),
            Err(Error::InvalidCharacter { index: 0 })
        );
        assert_eq!(
            DECODER.decode("1é").into_vec(),
            Err(Error::InvalidCharacter { index: 1 })
        );
        let text = ENCODER.encode([0, 1, 2, 3]).into_string();
        let bytes = faster_base58::Decoder::bitcoin()
            .decode(&text)
            .into_vec()
            .unwrap();
        for index in 0..bytes.len() {
            let mut corrupt = bytes.clone();
            corrupt[index] ^= 1;
            let text = faster_base58::Encoder::bitcoin()
                .encode(corrupt)
                .into_string();
            assert_eq!(
                DECODER.decode(&text).into_vec(),
                Err(Error::InvalidChecksum)
            );
            assert_eq!(
                VERSIONED_DECODER.decode(&text).into_vec(),
                Err(Error::InvalidChecksum)
            );
            assert_eq!(
                DECODER.decode(&text).onto(&mut [0; 8]),
                Err(Error::InvalidChecksum)
            );
            assert_eq!(
                DECODER.decode(&text).into_array::<4>(),
                Err(Error::InvalidChecksum)
            );
        }
        for error in [
            Error::MissingChecksum,
            Error::InvalidChecksum,
            Error::InvalidVersion {
                expected: 42,
                actual: Some(1),
            },
            Error::InvalidVersion {
                expected: 42,
                actual: None,
            },
        ] {
            let standard: &dyn std::error::Error = &error;
            assert!(!standard.to_string().is_empty());
        }
    }

    #[test]
    fn exact_payload_buffers_keep_checksum_separate_and_preserve_tails() {
        for version in [None, Some(0), Some(42)] {
            let encoder = version.map_or(ENCODER, |v| Encoder::bitcoin().with_check_version(v));
            let decoder = version.map_or(DECODER, |v| Decoder::bitcoin().with_check_version(v));
            for data in [&b""[..], &[0, 0, 1, 2, 3][..], &[42; 64][..]] {
                let text = encoder.encode(data).into_string();
                let want = bs58::decode(&text).with_check(version).into_vec().unwrap();
                assert!(text.len() <= encoder.max_encoded_len(data.len()));
                for capacity in [0, text.len() - 1, text.len(), text.len() + 7] {
                    let mut output = vec![0xa5; capacity + 2];
                    let result = encoder.encode(data).onto(&mut output[1..1 + capacity]);
                    if capacity < text.len() {
                        assert_eq!(result, Err(BufferTooSmall));
                    } else {
                        assert_eq!(result.unwrap(), text);
                        assert!(output[1 + text.len()..].iter().all(|&b| b == 0xa5));
                    }
                    assert_eq!((output[0], output[capacity + 1]), (0xa5, 0xa5));
                }
                for capacity in [0, want.len().saturating_sub(1), want.len(), want.len() + 7] {
                    let mut output = vec![0xa5; capacity + 2];
                    let result = decoder.decode(&text).onto(&mut output[1..1 + capacity]);
                    if capacity < want.len() {
                        assert_eq!(result, Err(Error::BufferTooSmall));
                    } else {
                        assert_eq!(result.unwrap(), want);
                        assert!(output[1 + want.len()..].iter().all(|&b| b == 0xa5));
                    }
                    assert_eq!((output[0], output[capacity + 1]), (0xa5, 0xa5));
                }
            }
        }
        let text = ENCODER.encode([0, 1, 2, 3, 4]).into_string();
        assert_eq!(DECODER.decode(&text).into_array::<5>(), Ok([0, 1, 2, 3, 4]));
        assert_eq!(
            DECODER.decode(&text).into_array::<4>(),
            Err(Error::BufferTooSmall)
        );
        assert_eq!(
            DECODER.decode(&text).into_array::<6>(),
            Err(Error::InvalidLength {
                expected: 6,
                actual: 5
            })
        );
        assert_eq!(DECODER.decode("3QJmnh").into_array::<0>(), Ok([]));
        assert_eq!(ENCODER.max_encoded_len(usize::MAX), usize::MAX);
        assert_eq!(
            VERSIONED_ENCODER.max_encoded_len(usize::MAX - 3),
            usize::MAX
        );
    }

    #[test]
    fn dispatch_boundaries_match_bs58_check() {
        for len in [0, 1, 17, 18, 19, 27, 28, 29, 59, 60, 61, 512, 4096, 8193] {
            for zeros in [0, 5] {
                let mut data = vec![0; zeros];
                data.extend((0..len).map(|i| (i * 73 + 19) as u8));
                let want = bs58::encode(&data).with_check().into_string();
                assert_eq!(ENCODER.encode(&data).into_string(), want);
                assert_eq!(DECODER.decode(&want).into_vec().unwrap(), data);
            }
        }
    }

    #[test]
    fn checked_encoding_uses_exact_buffers_across_fast_routes() {
        for version in [None, Some(0), Some(42)] {
            let encoder = version.map_or(ENCODER, |v| Encoder::bitcoin().with_check_version(v));
            for len in [17, 28, 60, 512, 5120, 8188, 8193, 16379] {
                for zeros in [0, len] {
                    let mut data = vec![0; zeros];
                    data.resize(len, 0xa5);
                    let expected = match version {
                        Some(v) => bs58::encode(&data).with_check_version(v).into_string(),
                        None => bs58::encode(&data).with_check().into_string(),
                    };
                    for capacity in [expected.len() - 1, expected.len(), expected.len() + 7] {
                        let mut buffer = vec![0x5a; capacity + 2];
                        let result = encoder.encode(&data).onto(&mut buffer[1..1 + capacity]);
                        if capacity < expected.len() {
                            assert_eq!(result, Err(BufferTooSmall));
                        } else {
                            assert_eq!(result.unwrap(), expected);
                            assert!(buffer[1 + expected.len()..].iter().all(|&b| b == 0x5a));
                        }
                        assert_eq!((buffer[0], buffer[capacity + 1]), (0x5a, 0x5a));
                    }
                }
            }
        }
    }

    #[test]
    fn zero_prefixes_can_extend_into_the_checksum_segment() {
        for len in 0..256 {
            let data = vec![0; len];
            let text = bs58::encode(&data).with_check().into_string();
            let mut output = vec![0xa5; len + 7];
            assert_eq!(
                DECODER.decode(&text).onto(&mut output[..len]).unwrap(),
                data
            );
            assert_eq!(&output[len..], &[0xa5; 7]);
        }
        for len in 0..8 {
            let text = "1".repeat(len);
            let mut output = [0xa5; 8];
            let want = if len < 4 {
                Error::MissingChecksum
            } else {
                Error::InvalidChecksum
            };
            assert_eq!(DECODER.decode(&text).onto(&mut output), Err(want));
        }
    }

    proptest! {
        #[test]
        fn arbitrary_data_matches_bs58(data in proptest::collection::vec(any::<u8>(), 0..1024), version in proptest::option::of(any::<u8>())) {
            let encoder = version.map_or(ENCODER, |v| Encoder::bitcoin().with_check_version(v));
            let decoder = version.map_or(DECODER, |v| Decoder::bitcoin().with_check_version(v));
            let want = match version {
                Some(v) => bs58::encode(&data).with_check_version(v).into_string(),
                None => bs58::encode(&data).with_check().into_string(),
            };
            prop_assert_eq!(encoder.encode(&data).into_string(), want.as_str());
            let expected = bs58::decode(&want).with_check(version).into_vec().unwrap();
            prop_assert_eq!(decoder.decode(&want).into_vec().unwrap(), &expected[..]);
            let mut output = vec![0xa5; expected.len() + 7];
            prop_assert_eq!(decoder.decode(&want).onto(&mut output[..expected.len()]).unwrap(), &expected[..]);
            prop_assert_eq!(&output[expected.len()..], &[0xa5; 7]);
        }

        #[test]
        fn arbitrary_base58_matches_bs58_validation(digits in proptest::collection::vec(0usize..58, 0..256)) {
            let alphabet = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
            let text: String = digits.into_iter().map(|d| alphabet[d] as char).collect();
            let ours = DECODER.decode(&text).into_vec();
            let mut output = vec![0xa5; text.len()];
            prop_assert_eq!(DECODER.decode(&text).onto(&mut output).map(|bytes| bytes.to_vec()), ours.clone());
            match bs58::decode(&text).with_check(None).into_vec() {
                Ok(data) => prop_assert_eq!(ours.unwrap(), data),
                Err(bs58::decode::Error::NoChecksum) => prop_assert_eq!(ours, Err(Error::MissingChecksum)),
                Err(bs58::decode::Error::InvalidChecksum { .. }) => prop_assert_eq!(ours, Err(Error::InvalidChecksum)),
                other => panic!("unexpected oracle result: {other:?}"),
            }
        }
    }
}

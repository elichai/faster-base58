mod support;

use faster_base58::internals::{self, Leaves};
use proptest::prelude::*;
use support::{ALPHABET, check_decode, check_encode};

type TunedEncode =
    fn(&[u8], &mut [u8], usize, usize, Leaves) -> Result<usize, faster_base58::BufferTooSmall>;
const TUNED_ENCODERS: &[(&str, TunedEncode)] = &[
    ("dc-tuned", internals::encode_karatsuba_with),
    ("ntt-tuned", internals::encode_ntt_with),
    ("malachite-tuned", support::malachite::encode_malachite_with),
];

fn lengths() -> impl Strategy<Value = usize> {
    prop_oneof![
        8 => 0usize..513,
        2 => prop::sample::select(vec![
            0, 1, 6, 7, 8, 9, 10, 11, 21, 22, 23, 31, 32, 33, 63, 64, 65,
            511, 512, 513, 767, 768, 769, 1279, 1280, 1281, 1503, 1504, 1505,
            3071, 3072, 3073, 3583, 3584, 3585, 4095, 4096, 4097, 5119, 5120,
            5121, 5631, 5632, 5633, 8191, 8192, 8193,
        ]),
        1 => 0usize..8194,
    ]
}

fn bytes() -> impl Strategy<Value = Vec<u8>> {
    lengths().prop_flat_map(|len| prop::collection::vec(any::<u8>(), len))
}

proptest! {
    #![proptest_config(ProptestConfig::default())]

    #[test]
    fn every_encoder_matches_independent_oracle(mut data in bytes(), zeros in 0usize..40, capacity in any::<usize>()) {
        let leading = zeros.min(data.len());
        data[..leading].fill(0);
        check_encode(&data, capacity);
        check_decode(&bs58::encode(&data).into_string(), capacity);
    }

    #[test]
    fn arbitrary_base58_decodes(digits in lengths().prop_flat_map(|n| prop::collection::vec(0u8..58, n)), capacity in any::<usize>()) {
        let input: String = digits.iter().map(|&d| ALPHABET[d as usize] as char).collect();
        check_decode(&input, capacity);
    }

    #[test]
    fn malformed_utf8_strings_report_first_bad_byte(data in prop::collection::vec(any::<u8>(), 0..2049), capacity in any::<usize>()) {
        // The public backends accept &str. Invalid UTF-8 is outside that API's domain.
        check_decode(&String::from_utf8_lossy(&data), capacity);
    }

    #[test]
    fn injected_invalid_bytes_report_first_index(digits in prop::collection::vec(0u8..58, 0..8194), position in any::<usize>(), invalid in any::<u8>()) {
        let mut input: String = digits.iter().map(|&d| ALPHABET[d as usize] as char).collect();
        let at = position % (input.len() + 1);
        let bad = if ALPHABET.contains(&invalid) { '\0' } else { char::from(invalid) };
        input.insert(at, bad);
        assert_eq!(support::oracle_decode(&input), Err(at));
        check_decode(&input, position);
    }
}

#[test]
fn recursive_and_ntt_boundaries_match_bs58() {
    // 64 KiB crosses the x86 IFMA NTT thresholds too. Adjacent sizes exercise partial limbs.
    for len in [
        16_383usize,
        16_384,
        16_385,
        32_767,
        32_768,
        32_769,
        65_535,
        65_536,
        65_537,
    ] {
        let mut data: Vec<u8> = (0..len)
            .map(|i| (i.wrapping_mul(73) ^ (i >> 7)) as u8)
            .collect();
        data[0] = 0xFF;
        data[len / 3..len / 2].fill(0);
        data[2 * len / 3..].fill(0xFF);
        check_encode(&data, len / 2);
        check_decode(&bs58::encode(data).into_string(), len / 2);
    }
}

#[test]
fn radix_hooks_fall_back_above_the_transform_limit() {
    let input = vec![1; (1 << 24) + 1];
    for encode in [
        internals::encode_radix as support::Encode,
        internals::encode_radix_portable,
    ] {
        assert_eq!(encode(&input, &mut []), Err(faster_base58::BufferTooSmall));
    }
}

#[test]
fn leading_zeros_and_carry_boundaries() {
    for len in [0, 1, 7, 8, 10, 32, 64, 512, 768, 1504, 5121, 8193] {
        for data in [vec![0; len], vec![0xFF; len]] {
            check_encode(&data, len);
            check_decode(&bs58::encode(data).into_string(), len);
        }
    }
    for exponent in [1, 9, 10, 11, 19, 20, 21, 5119, 5120, 5121, 8193] {
        for text in ["z".repeat(exponent), format!("2{}", "1".repeat(exponent))] {
            let data = bs58::decode(&text).into_vec().unwrap();
            check_encode(&data, exponent);
            check_decode(&text, exponent);
        }
    }
}

#[test]
fn tuned_backends_match_bs58() {
    for len in [0, 1, 33, 513, 8193] {
        let data: Vec<u8> = (0..len).map(|i| (i * 73 + 1) as u8).collect();
        let want = bs58::encode(&data).into_vec();
        for leaves in [Leaves::Horner, Leaves::Matrix] {
            let max = if leaves == Leaves::Horner { 256 } else { 1024 };
            for leaf in [1, 32, max] {
                for &(name, encode) in TUNED_ENCODERS {
                    for threshold in [0, usize::MAX] {
                        let mut out = vec![0xA5; want.len() + 17];
                        assert_eq!(
                            encode(&data, &mut out, threshold, leaf, leaves),
                            Ok(want.len()),
                            "{name}: {len} bytes, {leaves:?} leaf {leaf}, threshold {threshold}"
                        );
                        assert_eq!(&out[..want.len()], &want);
                        assert!(out[want.len()..].iter().all(|&x| x == 0xA5));
                    }
                }
            }
        }
        // min=0 sends even empty/all-zero input directly to the matrix conversion.
        for min in [0, 1, usize::MAX] {
            let mut out = vec![0; want.len()];
            assert_eq!(
                internals::encode_matrix_with(&data, &mut out, min),
                Ok(want.len())
            );
            assert_eq!(out, want);
        }
        let text = std::str::from_utf8(&want).unwrap();
        for cached in [false, true] {
            for threshold in [0, 1, usize::MAX] {
                for prepared in [0, 1, usize::MAX] {
                    let mut out = vec![0; data.len()];
                    assert_eq!(
                        internals::decode_ntt_with(text, &mut out, cached, threshold, prepared),
                        Ok(data.len())
                    );
                    assert_eq!(out, data);
                }
            }
        }
        for threshold in [32, 128, usize::MAX] {
            let mut out = vec![0; data.len()];
            assert_eq!(
                internals::decode_toom3_with(text, &mut out, threshold),
                Ok(data.len())
            );
            assert_eq!(out, data);
        }
    }
}

#[test]
fn tuning_preconditions_reject_invalid_values() {
    for &(_, encode) in TUNED_ENCODERS {
        for (leaves, leaf) in [
            (Leaves::Horner, 0),
            (Leaves::Horner, 257),
            (Leaves::Matrix, 0),
            (Leaves::Matrix, 1025),
        ] {
            std::panic::catch_unwind(|| encode(&[], &mut [], 0, leaf, leaves)).unwrap_err();
        }
    }
    for threshold in [0, 1, 31] {
        std::panic::catch_unwind(|| internals::decode_toom3_with("", &mut [], threshold))
            .unwrap_err();
    }
}

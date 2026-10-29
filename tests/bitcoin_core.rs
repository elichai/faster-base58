//! Bitcoin Core known-answer vectors and explicit strict-parser compatibility rules.
//!
//! Vectors copied from bitcoin/bitcoin at 2b5b6fe231ff45a09d9e24411e4420d51ed45cab:
//! `src/test/data/base58_encode_decode.json`. Copyright (c) 2009-2026 The Bitcoin
//! Core developers and Bitcoin Developers. Distributed under the MIT license;
//! see docs/references/bitcoin-core-LICENSE.txt. Runtime code was not copied.

#[cfg(feature = "__internals")]
mod support;

use faster_base58::{BufferTooSmall, Error};

type Encode = fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall>;
type Decode = fn(&str, &mut [u8]) -> Result<usize, Error>;
const ENCODERS: [Encode; 2] = [faster_base58::encode, faster_base58::no_alloc::encode];
const DECODERS: [Decode; 2] = [faster_base58::decode, faster_base58::no_alloc::decode];

const VECTORS: &[(&str, &str)] = &[
    ("", ""),
    ("61", "2g"),
    ("626262", "a3gV"),
    ("636363", "aPEr"),
    (
        "73696d706c792061206c6f6e6720737472696e67",
        "2cFupjhnEsSn59qHXstmK2ffpLv2",
    ),
    (
        "00eb15231dfceb60925886b67d065299925915aeb172c06647",
        "1NS17iag9jJgTHD1VXjvLCEnZuQ3rJDE9L",
    ),
    ("516b6fcd0f", "ABnLTmg"),
    ("bf4f89001e670274dd", "3SEo3LWLoPntC"),
    ("572e4794", "3EFU7m"),
    ("ecac89cad93923c02321", "EJDM8drfXA6uyA"),
    ("10c8511e", "Rt5zm"),
    ("00000000000000000000", "1111111111"),
    (
        "00000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "1111111111111111111111111111111111111111",
    ),
    (
        "00000000000000000000000000000000000000000000000000000000000000000000000000000001",
        "1111111111111111111111111111111111111112",
    ),
    (
        concat!(
            "000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000ec39d04c37e71e5d591881f6",
        ),
        concat!(
            "111111111111111111111111111111111111111111111111111111111111111111111111111111111111",
            "115TYzLYH1udmLdzCLM",
        ),
    ),
    (
        concat!(
            "000111d38e5fc9071ffcd20b4a763cc9ae4f252bb4e48fd66a835e252ada93ff480d6dd43dc62a641155",
            "a5",
        ),
        "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz",
    ),
    (
        concat!(
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272829",
            "2a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f50515253",
            "5455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d",
            "7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7",
            "a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1",
            "d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafb",
            "fcfdfeff",
        ),
        concat!(
            "1cWB5HCBdLjAuqGGReWE3R3CguuwSjw6RHn39s2yuDRTS5NsBgNiFpWgAnEx6VQi8csexkgYw3mdYrMHr8x9",
            "i7aEwP8kZ7vccXWqKDvGv3u1GxFKPuAkn8JCPPGDMf3vMMnbzm6Nh9zh1gcNsMvH3ZNLmP5fSG6DGbbi2tuw",
            "MWPthr4boWwCxf7ewSgNQeacyozhKDDQQ1qL5fQFUW52QKUZDZ5fw3KXNQJMcNTcaB723LchjeKun7MuGW5q",
            "yCBZYzA1KjofN1gYBV3NqyhQJ3Ns746GNuf9N2pQPmHz4xpnSrrfCvy6TVVz5d4PdrjeshsWQwpZsZGzvbdA",
            "dN8MKV5QsBDY",
        ),
    ),
    ("271F359E", "zzzzy"),
    ("271F359F", "zzzzz"),
    ("271F35A0", "211111"),
    ("271F35A1", "211112"),
];

fn hex_bytes(text: &str) -> Vec<u8> {
    let (pairs, rest) = text.as_bytes().as_chunks::<2>();
    assert!(rest.is_empty());
    pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn check_vector(data: &[u8], text: &str) {
    for encode in ENCODERS {
        let mut output = vec![0xA5; text.len() + 7];
        assert_eq!(encode(data, &mut output), Ok(text.len()));
        assert_eq!(&output[..text.len()], text.as_bytes());
        assert!(output[text.len()..].iter().all(|&byte| byte == 0xA5));
        if !text.is_empty() {
            assert_eq!(
                encode(data, &mut vec![0; text.len() - 1]),
                Err(BufferTooSmall)
            );
        }
    }
    for decode in DECODERS {
        let mut output = vec![0xA5; data.len() + 7];
        assert_eq!(decode(text, &mut output), Ok(data.len()));
        assert_eq!(&output[..data.len()], data);
        assert!(output[data.len()..].iter().all(|&byte| byte == 0xA5));
        if !data.is_empty() {
            assert_eq!(
                decode(text, &mut vec![0; data.len() - 1]),
                Err(Error::BufferTooSmall)
            );
        }
    }
    #[cfg(feature = "__internals")]
    {
        assert_eq!(bs58::encode(data).into_string(), text);
        support::check_encode(data, data.len());
        support::check_decode(text, text.len());
    }
}

#[test]
fn bitcoin_core_vectors_and_fixed_width_zero_extensions() {
    for &(hex, text) in VECTORS {
        let data = hex_bytes(hex);
        check_vector(&data, text);
        for width in [32, 64] {
            if data.len() < width {
                let zeros = width - data.len();
                let mut padded = vec![0; zeros];
                padded.extend_from_slice(&data);
                check_vector(&padded, &format!("{}{text}", "1".repeat(zeros)));
            }
        }
    }
}

#[test]
fn strict_decoder_rejects_whitespace_and_embedded_nuls() {
    // Core permits surrounding ASCII whitespace. This API deliberately rejects it.
    for separator in [' ', '\t', '\n', '\r', '\u{b}', '\u{c}', '\0'] {
        for (text, index) in [
            (format!("{separator}good"), 0),
            (format!("good{separator}"), 4),
            (format!("good{separator}bad0IOl"), 4),
        ] {
            for decode in DECODERS {
                assert_eq!(
                    decode(&text, &mut [0; 64]),
                    Err(Error::InvalidCharacter { index })
                );
            }
            #[cfg(feature = "__internals")]
            support::check_decode(&text, index);
        }
    }
}

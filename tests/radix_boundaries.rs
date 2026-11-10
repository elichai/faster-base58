#![cfg(any(
    all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ),
    all(target_arch = "x86_64", target_feature = "sse2"),
    target_arch = "wasm32"
))]

use malachite_base::num::conversion::traits::{Digits, PowerOf2Digits};
use malachite_nz::natural::Natural;
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn check(data: &[u8], short: bool) {
    let zeros = data.iter().take_while(|&&b| b == 0).count();
    let value = Natural::from_power_of_2_digits_desc(8, data[zeros..].iter().copied()).unwrap();
    let digits: Vec<u8> = value.to_digits_desc(&58);
    let want: Vec<u8> = core::iter::repeat_n(b'1', zeros)
        .chain(digits.iter().map(|&d| ALPHABET[d as usize]))
        .collect();
    let mut guarded = vec![0xa5; want.len() + 32];
    let got = faster_base58::Encoder::bitcoin()
        .encode(data)
        .onto(&mut guarded[16..16 + want.len()])
        .map(str::len)
        .unwrap();
    assert_eq!(got, want.len(), "input {}", data.len());
    assert_eq!(&guarded[16..16 + got], want, "input {}", data.len());
    assert!(
        guarded[..16]
            .iter()
            .chain(&guarded[16 + got..])
            .all(|&c| c == 0xa5)
    );
    if short {
        assert_eq!(
            faster_base58::Encoder::bitcoin()
                .encode(data)
                .onto(&mut guarded[16..15 + want.len()])
                .map(str::len),
            Err(faster_base58::BufferTooSmall)
        );
        assert!(
            guarded[..16]
                .iter()
                .chain(&guarded[16 + got..])
                .all(|&c| c == 0xa5)
        );
    }
    eprintln!(
        "oracle passed: {} input bytes, {} output digits",
        data.len(),
        got
    );
}

#[test]
fn public_dispatch_boundaries_match_malachite() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(0x580a_46d4_722d_d38d);
    // Includes the last sizes below 5/8 and 3/4 of the root transform used, and the first
    // sizes reaching them (radix with and without `cache`), each target's minimum and the
    // first sizes reaching 9/16 (scalar Wasm).
    for n in [
        768, 769, 1152, 3072, 3073, 5059, 5060, 8191, 8192, 8193, 10120, 10121, 11245, 11246,
        12288, 13494, 13495, 27648, 27649, 32767, 32768, 32769, 65535, 65536, 65537, 89976, 89977,
        107972, 107973, 262143, 262144, 262145, 1048575, 1048576, 1048577,
    ] {
        let mut data = vec![0; n + 37];
        rng.fill_bytes(&mut data[37..]);
        data[37] |= 1;
        check(&data, true);
    }
}

#[test]
#[ignore = "Exercises 16 MiB transforms; run explicitly for release validation"]
fn full_transform_limit_and_fallback_match_malachite() {
    for n in [(1 << 24) - 1, 1 << 24, (1 << 24) + 1] {
        let mut data = vec![255; n];
        if n == (1 << 24) + 1 {
            data.fill(0);
            data[0] = 1;
            data[n - 1] = 1;
        }
        check(&data, false);
    }
}

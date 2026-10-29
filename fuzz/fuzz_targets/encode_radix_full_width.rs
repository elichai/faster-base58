#![no_main]

#[path = "../../tests/support/mod.rs"]
mod support;

use libfuzzer_sys::fuzz_target;
use malachite_base::num::conversion::traits::{Digits, PowerOf2Digits};
use malachite_nz::natural::Natural;

fuzz_target!(|raw: &[u8]| {
    // Both widths select cached NEON radix; the larger also owns log19 twiddles.
    let width = if raw.first().is_some_and(|b| b & 0x80 != 0) {
        1 << 20
    } else {
        1 << 18
    };
    let mut input = vec![0xA5; width];
    if !raw.is_empty() {
        for chunk in input.chunks_mut(raw.len()) {
            chunk.copy_from_slice(&raw[..chunk.len()]);
        }
    }
    input[0] |= 0x80;
    let value = Natural::from_power_of_2_digits_desc(8, input.iter().copied()).unwrap();
    let digits: Vec<u8> = value.to_digits_desc(&58);
    let want: Vec<u8> = digits
        .iter()
        .map(|&d| support::ALPHABET[d as usize])
        .collect();
    let selector = support::scalar::selector(raw);
    let encoders = [
        ("auto", faster_base58::encode as support::Encode),
        ("radix", faster_base58::internals::encode_radix),
        (
            "radix-portable",
            faster_base58::internals::encode_radix_portable,
        ),
    ];
    let (name, encode) = encoders[raw.first().copied().unwrap_or(0) as usize % encoders.len()];
    // Always convert; short-only mutations must not avoid the multiplication tree.
    support::check_encoder_capacity(
        name,
        encode,
        &input,
        &want,
        want.len() + (selector & 1) * 17,
    );
    if selector & 7 == 0 {
        support::check_encoder_capacity(name, encode, &input, &want, want.len() - 1);
    }
});

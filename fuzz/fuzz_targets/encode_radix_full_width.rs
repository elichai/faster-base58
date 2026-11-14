#![no_main]

#[path = "../full_width.rs"]
mod full_width;
#[path = "../../tests/support/mod.rs"]
mod support;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|raw: &[u8]| {
    // Both widths select cached NEON radix; the larger also owns log19 twiddles.
    let width = if raw.first().is_some_and(|b| b & 0x80 != 0) {
        1 << 20
    } else {
        1 << 18
    };
    let input = full_width::expand_to(raw, width);
    let want = support::reference::encode(&input);
    let selector = support::scalar::selector(raw);
    let encoders = [
        (
            "auto",
            (|input, output| {
                faster_base58::Encoder::bitcoin()
                    .encode(input)
                    .onto(output)
                    .map(str::len)
            }) as support::Encode,
        ),
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

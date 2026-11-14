#![no_main]

#[path = "../full_width.rs"]
mod full_width;
#[path = "../../tests/support/mod.rs"]
mod support;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|raw: &[u8]| {
    let mut text = full_width::decode_input(raw);
    let selector = support::scalar::selector(raw);
    let want = support::oracle_decode_bytes(&text);
    // Quadratic kernels are already checked by the general targets. Each callback
    // selects one subquadratic decoder and always performs a full conversion.
    let decoders: Vec<_> = support::DECODERS
        .iter()
        .filter(|&&(name, _)| !matches!(name, "batched" | "no-alloc"))
        .collect();
    let &&(name, decode) = &decoders[selector % decoders.len()];
    support::check_decoder(
        name,
        decode,
        core::str::from_utf8(&text).unwrap(),
        &want,
        selector,
    );

    let at = selector % text.len();
    text[at] = [0, b'0', 0x80, 0xFF][selector % 4];
    support::check_decoder(
        "public-invalid-bytes",
        |input: &[u8], output: &mut [u8]| {
            faster_base58::Decoder::bitcoin()
                .decode(input)
                .onto(output)
                .map(|bytes| bytes.len())
        },
        &text[..],
        &Err(at),
        selector,
    );
});

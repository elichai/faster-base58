#![no_main]

#[path = "../full_width.rs"]
mod full_width;
#[path = "../../tests/support/mod.rs"]
mod support;

use libfuzzer_sys::fuzz_target;
#[path = "../isa.rs"]
mod isa;

fuzz_target!(init: isa::init(), |raw: &[u8]| {
    let input = full_width::expand(raw);
    let selector = support::scalar::selector(raw);
    support::check_round_trip(&input, selector);
});

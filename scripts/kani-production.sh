#!/bin/sh
# Production tiny paths and the alphabet properties used to specify them.
set -eu
cargo kani --lib --solver cadical \
    --harness one_byte_batched_encoding_matches_radix_and_capacity \
    --harness two_byte_short_decode_preserves_radix_errors_and_canaries \
    --harness alphabet_character_roundtrips \
    --harness valid_digit_table_entries_roundtrip_to_character \
    -Z function-contracts -Z stubbing -Z unstable-options \
    --harness-timeout 60s --output-format terse "$@"

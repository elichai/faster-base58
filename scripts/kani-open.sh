#!/bin/sh
# Retained open obligations. A nonzero exit is expected until they are discharged.
# Run from the repository root; all production checks remain enabled.
set -u
status=0
cargo kani --lib \
    --harness encoded_capacity_is_monotone_and_saturating \
    --harness encoded_capacity_adjacent_monotonicity \
    --harness encoded_capacity_adjacent_monotonicity_composed \
    --harness encode_batched_split_is_exact \
    --harness encode_batched_step_limb_is_exact \
    --harness full_chunk_matches_horner_and_rejects_invalid \
    --harness full_chunk_valid_numeric_identity \
    --harness fixed_scalar_carry_preserves_value \
    --harness fixed_split_carry_matches_serial \
    --harness barrett_divide_one_base_power_is_exact \
    --harness paired_update_matches_sequential \
    --harness paired_limb_update_matches_sequential \
    --harness one_byte_batched_encoding_matches_radix_and_capacity \
    --harness two_digit_batched_decoding_matches_radix_and_capacity \
    -Z function-contracts -Z stubbing -Z unstable-options \
    --harness-timeout 60s --jobs 2 --output-format terse || status=1
# Isolate this harness: current Kani may fail during SIMD-reachable compilation.
cargo kani --lib --harness one_byte_borrowed_api_preserves_exact_array_contracts \
    -Z function-contracts -Z stubbing -Z unstable-options \
    --harness-timeout 60s || status=1
exit "$status"

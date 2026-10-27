#!/bin/sh
# Retained open obligations. A nonzero exit is expected until they are discharged.
# Run from the repository root; all production checks remain enabled.
# Parser numeric/rejection and capacity monotonicity are closed by the required
# verified compositions. Their historical direct harnesses remain in source.
# One-byte encoding and two-digit decoding are required; retain wider queries.
set -u
status=0
cargo kani --lib \
    --harness encode_batched_split_is_exact \
    --harness encode_batched_step_limb_is_exact \
    --harness fixed_scalar_carry_preserves_value \
    --harness fixed_split_carry_matches_serial \
    --harness barrett_divide_one_base_power_is_exact \
    --harness paired_update_matches_sequential \
    --harness paired_limb_update_matches_sequential \
    -Z function-contracts -Z stubbing -Z unstable-options \
    --harness-timeout 60s --jobs 2 --output-format terse || status=1
# Keep this wrapper isolated: its previous retry compiled, then timed out in CBMC.
cargo kani --lib --harness one_byte_borrowed_api_preserves_exact_array_contracts \
    -Z function-contracts -Z stubbing -Z unstable-options \
    --harness-timeout 60s || status=1
exit "$status"

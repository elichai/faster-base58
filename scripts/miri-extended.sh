#!/bin/sh
# Reproduce every known missing shim separately, retaining each failed execution.
set -eu
case "${1-}" in
    arm)
        target=aarch64-unknown-linux-gnu
        export RUSTFLAGS='-C target-cpu=generic'
        set -- arm::rounded_shift32 arm::shift_insert32 arm::shift_insert16 \
            arm::table_extend1 arm::table_extend4
        ;;
    x86)
        target=x86_64-unknown-linux-gnu
        export RUSTFLAGS='-C target-cpu=x86-64 -C target-feature=+avx2,+avx512f,+avx512bw,+avx512vl,+avx512vbmi,+avx512ifma'
        set -- x86::variable_right64 x86::variable_left64 x86::multiply_low52 \
            x86::multiply_high52 x86::permute2_d512 x86::permute2_b256
        ;;
    *)
        echo 'usage: sh scripts/miri-extended.sh arm|x86' >&2
        exit 2
        ;;
esac
export MIRIFLAGS='-Zmiri-strict-provenance'
cargo +nightly-2026-10-23 miri setup --target "$target"
status=0
for probe in "$@"; do
    cargo +nightly-2026-10-23 miri test --target "$target" --locked \
        --test miri_intrinsics "$probe" -- --exact || status=1
done
exit "$status"

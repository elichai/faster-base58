#!/bin/sh
# Run every declared SIMD probe separately and retain nonzero/zero-test failures.
set -eu
BASE58_MIRI_TOOLCHAIN=${BASE58_MIRI_TOOLCHAIN:-nightly-2026-10-26}
case "${1-}" in
    arm)
        target=aarch64-unknown-linux-gnu
        export RUSTFLAGS='-C target-cpu=generic'
        set -- arm::rounded_shift32 arm::shift_insert32 arm::shift_insert16 \
            arm::table_extend1 arm::table_extend4 arm::halving_subtract32
        ;;
    x86)
        target=x86_64-unknown-linux-gnu
        export RUSTFLAGS='-C target-cpu=x86-64 -C target-feature=+avx2,+avx512f,+avx512bw,+avx512vl,+avx512vbmi,+avx512ifma'
        set -- x86::variable_right64 x86::variable_left64 x86::multiply_low52 \
            x86::multiply_high52 x86::permute2_d512 x86::permute_q512 \
            x86::permute2_q512 x86::permute2_b256 x86::permute_b256
        ;;
    *)
        echo 'usage: sh scripts/miri-extended.sh arm|x86' >&2
        exit 2
        ;;
esac
run_miri() {
    if [ -n "${BASE58_CARGO_MIRI-}" ]; then
        "$BASE58_CARGO_MIRI" miri "$@"
    else
        cargo +"$BASE58_MIRI_TOOLCHAIN" miri "$@"
    fi
}
if [ -n "${BASE58_MIRI_LOG_DIR-}" ]; then
    logs=$BASE58_MIRI_LOG_DIR
    mkdir "$logs"
else
    logs=$(mktemp -d "${TMPDIR:-/tmp}/base58-miri-extended.XXXXXX")
fi
export MIRIFLAGS="${MIRIFLAGS:--Zmiri-strict-provenance}"
printf 'Miri probe logs: %s\n' "$logs"
if run_miri setup --target "$target" >"$logs/setup.log" 2>&1; then
    cat "$logs/setup.log"
else
    cat "$logs/setup.log"
    exit 1
fi
status=0
for probe in "$@"; do
    label=$(printf '%s' "$probe" | tr ':' '-')
    log=$logs/$label.log
    code=0
    run_miri test --target "$target" --locked --test miri_intrinsics \
        "$probe" -- --exact >"$log" 2>&1 || code=$?
    cat "$log"
    if [ "$code" -ne 0 ] || ! awk '
        /^test result: ok\. 1 passed; 0 failed; 0 ignored;/ { count++ }
        END { exit count == 1 ? 0 : 1 }
    ' "$log"; then
        status=1
    fi
done
exit "$status"

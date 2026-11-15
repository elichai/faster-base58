#!/usr/bin/env bash
# Run from the repository root. Replay fixed inputs; do not load the stored corpus.
set -euo pipefail

export RUSTUP_TOOLCHAIN="${1:?usage: bash fuzz/smoke-isa.sh NIGHTLY}"
export RUSTFLAGS='-C target-cpu=generic'
unset CARGO_ENCODED_RUSTFLAGS
export RUSTC_WRAPPER="$PWD/fuzz/oracle_rustc.py"

smoke_seeds=target/isa-smoke-seeds
python3 fuzz/seed_corpus.py --output "$smoke_seeds"
mkdir -p target/isa-smoke
smoke_logs=$(mktemp -d target/isa-smoke/logs.XXXXXX)

replay() {
    local smoke_target=$1
    shift
    local smoke_log="$smoke_logs/$FASTER_BASE58_FUZZ_MAX_ISA-$smoke_features-$smoke_target.log"
    local smoke_started=$SECONDS
    local smoke_input
    for smoke_input in "$@"; do
        if [[ ! -f "$smoke_input" ]]; then
            printf 'Missing smoke input: %s\n' "$smoke_input" >&2
            return 1
        fi
    done
    cargo fuzz run --verbose --sanitizer address --no-trace-compares --debug-assertions \
        --codegen-units 16 --target-dir target/isa-smoke --no-default-features \
        --features "$smoke_features" "$smoke_target" "$@" -- \
        -runs=0 -timeout=120 -rss_limit_mb=4096 2>&1 | tee "$smoke_log"
    # A stale or incorrectly configured build must not pass as a capped replay.
    grep -Fq "fuzz max ISA=$FASTER_BASE58_FUZZ_MAX_ISA; effective" "$smoke_log"
    grep -Fq "Running $# inputs 1 time(s) each." "$smoke_log"
    printf 'Smoke completed: isa=%s features=%s target=%s inputs=%s seconds=%s\n' \
        "$FASTER_BASE58_FUZZ_MAX_ISA" "$smoke_features" "$smoke_target" "$#" "$((SECONDS - smoke_started))"
}

for smoke_features in check check,cache; do
    for smoke_isa in avx2 sse41; do
        export FASTER_BASE58_FUZZ_MAX_ISA=$smoke_isa
        replay encode_all \
            "$smoke_seeds/encode_all/pattern-0" \
            "$smoke_seeds/encode_all/pattern-32" \
            "$smoke_seeds/encode_all/pattern-64" \
            "$smoke_seeds/encode_all/carry-64" \
            "$smoke_seeds/encode_all/zeros-64" \
            "$smoke_seeds/encode_all/split-fallback-k2-s1" \
            "$smoke_seeds/encode_all/pattern-512" \
            "$smoke_seeds/encode_all/pattern-8193" \
            "$smoke_seeds/encode_all/pattern-16384" \
            "$smoke_seeds/encode_all/pattern-16385" \
            "$smoke_seeds/encode_all/pattern-32768" \
            "$smoke_seeds/encode_all/pattern-32769" \
            "$smoke_seeds/encode_all/pattern-65537"
        replay decode_all \
            "$smoke_seeds/decode_all/array-exact-32" \
            "$smoke_seeds/decode_all/array-exact-64" \
            "$smoke_seeds/decode_all/carry-512" \
            "$smoke_seeds/decode_all/malformed-64" \
            "$smoke_seeds/decode_all/unicode-four-byte-end-769" \
            "$smoke_seeds/decode_all/ntt-carry-44761" \
            "$smoke_seeds/decode_all/ntt-carry-89502"
        replay check_all \
            "$smoke_seeds/check_all/pattern-0" \
            "$smoke_seeds/check_all/pattern-28" \
            "$smoke_seeds/check_all/pattern-60" \
            "$smoke_seeds/check_all/pattern-8193"
    done
done

printf 'ISA smoke replay passed; logs: %s\n' "$smoke_logs"

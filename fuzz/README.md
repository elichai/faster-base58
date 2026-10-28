# Differential fuzzing

Both targets compare Auto, `no_alloc`, and the backends exposed by `__internals`
with `bs58`. They check empty inputs, leading zeros, exact/short/oversized output
slices, result suffixes, and canaries around each slice. Decoding checks the first
invalid byte index. The backends take
`&str`, so malformed UTF-8 is tested after lossy conversion; arbitrary raw invalid
UTF-8 cannot be passed to those APIs.

`encode_all` accepts up to 65,537 bytes: one byte past 64 KiB. Full-width inputs
cross from 8,192 to 8,193 `u64` limbs. `decode_all` derives its raw-input budget from
`max_encoded_len(65_537)`, currently 89,502 bytes. It checks both lossy strings and
a valid alphabet mapping, then inserts an invalid ASCII or multibyte character.
This is a raw-byte budget; lossy UTF-8 can expand it by up to 3 times, and the
inserted character adds 1–4 UTF-8 bytes.

The generator includes all-`z` inputs at 89,499..89,502 characters. Such an input represents
`58^n - 1`: 89,499 characters fit 8,192 limbs, while 89,500 require 8,193. The final
89,502-character seed can decode to 65,538 bytes, so the decode budget does not
impose the encode target's binary-length limit.

These budgets reach divide-and-conquer, Toom-3, matrix and NTT paths, including
the larger IFMA multiplication thresholds on supported CPUs. They do not cover
all supported NTT sizes or force unsupported instructions. For nonzero valid
inputs near either decode cap, the current split/power bounds prepare log14
tables; moving the cap does not cross that boundary. The next prepared-table
transition is at 96,641 characters. Actual transform use also depends on operand
lengths and the selected multiplication thresholds.

Install `cargo-fuzz` 0.13.2 and nightly-2026-10-26 with `rust-src`. Use a C++
compiler compatible with that toolchain's LLVM 23 runtime. Run from the repository:

```sh
python3 fuzz/test_seed_corpus.py
python3 fuzz/seed_corpus.py
cargo +nightly-2026-10-26 fuzz run --careful --sanitizer address encode_all fuzz/corpus/encode_small -- -max_len=2048 -max_total_time=60 -timeout=120 -rss_limit_mb=4096
cargo +nightly-2026-10-26 fuzz run --careful --sanitizer address decode_all fuzz/corpus/decode_small -- -max_len=2048 -max_total_time=60 -timeout=120 -rss_limit_mb=4096
cargo +nightly-2026-10-26 fuzz run --careful --sanitizer address encode_all -- -runs=0 -max_len=65537 -timeout=120 -rss_limit_mb=4096
cargo +nightly-2026-10-26 fuzz run --careful --sanitizer address decode_all -- -runs=0 -max_len=89502 -timeout=120 -rss_limit_mb=4096
```

`--careful` rebuilds the standard library with extra initialization and UB checks;
debug assertions and integer overflow checks remain enabled. Repeat with
`--features cache`; use `--sanitizer leak` for the separate LeakSanitizer campaign.
The short-input campaigns perform mutations quickly; replaying
the full boundary corpus exercises larger kernels without spending the entire
mutation budget on the quadratic oracle. For sustained large-input fuzzing,
replace `-runs=0` with `-max_total_time=3600`.

`encode_full_width` adds structured large-input mutations. Each raw input repeats
to exactly 65,537 bytes; its first byte has the high bit set. Empty input uses
`0xA5`. Thus shrinking raw inputs cannot avoid the native IFMA host's NTT paths.
It reuses the same encoding and generated-text decoding oracles, capacities,
canaries and owned-helper checks. This periodic structured domain does not cover
every 65,537-byte value. Unsupported CPU instructions are never forced.
Threshold retuning must recheck the size needed to reach actual transforms.

The fuzz-only `full-width` feature enables this slower target. CI runs it for
180 seconds with careful ASan in the existing default/cache rows; general
targets and complete boundary replays remain separate. The raw mutation cap is
8,193 bytes. `-print_corpus_stats=1` supplies each base input's
`InputInfo::NumExecutedMutations` from libFuzzer bundled in `libfuzzer-sys`
0.4.13; its sum counts
post-initialization mutation attempts. It excludes corpus initialization and
leak-check repeats, and does not count distinct inputs. `check_mutations.py`
requires a nonzero sum and retains callback counts separately. Campaign logs,
statistics and corpora are uploaded with the existing fuzz artifacts.

The generator preserves existing seeds and fails if a named seed was edited,
including under `python3 -O`. New
coverage seeds and crash artifacts are ignored by Git. Replay any crash with
`cargo +nightly-2026-10-26 fuzz run --careful TARGET PATH_TO_ARTIFACT`.

See [CORRECTNESS.md](../CORRECTNESS.md) for run counts, sanitizer limits,
proof domains and missing Miri shims. CI builds, mutates and replays the targets
in separate steps and retains corpus/crash artifacts for 14 days.

# Differential fuzzing

Both targets compare Auto, `no_alloc`, and the backends exposed by `__internals`
with `bs58`. They check empty inputs, leading zeros, exact/short/oversized output
slices, result suffixes, and canaries around each slice. Decoding checks the first
invalid byte index. The backends take
`&str`, so malformed UTF-8 is tested after lossy conversion; arbitrary raw invalid
UTF-8 cannot be passed to those APIs.

The forced `radix` and `radix-portable` encoders bypass production CPU/size
selection and participate in the shared differential checks on every target.
Thus x86 fuzz CI reaches the multiplication tree even though production selects
it only on little-endian AArch64 with NEON. Native ARM runs exercise NEON;
x86 runs do not establish NEON coverage. Both hooks fall back to the division
tree above the radix transform limit. The general targets' 65,537-byte binary
cap stays below the 262,144-byte cached production threshold; public dispatch and the 16 MiB limit
are covered separately by `radix_boundaries` tests.

Radix seeds include binary powers and sparse values around leaf/tree boundaries,
so zero subtrees and sparse carry paths do not depend on random mutations.

`encode_radix_full_width` repeats each raw input to 256 KiB or 1 MiB, selected
by its first byte's high bit, then sets the expanded first byte's high bit.
Each callback selects automatic, forced radix or portable radix
encoding and checks it against Malachite's independent integer conversion, with
an exact/oversized buffer and occasional short-buffer checks. Every callback
performs a full conversion; native/portable seeds initialize both forced hooks.
Avoiding the quadratic bs58 oracle makes large mutations practical.
Shrinking cannot avoid the cached production route on NEON; 1 MiB cases also
reach owned twiddles above log18. The `full-width` feature enables it; the optional
manual Verification fuzz dispatch runs 180-second careful ASan campaigns for default/cache and
requires actual mutations. This periodic
input domain excludes leading zeros and does not cover every value at either width.

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

The fuzz-only `full-width` feature enables this slower target. The optional manual
Verification fuzz dispatch runs it for 180 seconds with careful ASan in default/cache rows; general
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

`base58.dict` is versioned on `main` and is passed to the mutating decode lane.
It supplies invalid ASCII and UTF-8 boundaries; structured 64-byte
`split-fallback-*` seeds force the correlated base-58^4 condition that makes
the fixed encoder's split carry use its serial fallback. These inputs belong in
the deterministic generator because the relationship is arithmetic, not a
useful byte token for a dictionary.

The shared corpus is not committed to `main`. The protected orphan
`fuzz-corpus` branch stores content-addressed inputs under
`corpora/<target>/<sha256>` and per-run metadata under `runs/`. Main/PR CI
only validates and replays a pinned revision. The optional Verification dispatch
with `run_fuzz` runs bounded campaigns on its selected ref but never publishes
the corpus. Nightly workers upload artifacts; the separate trusted publisher
validates, replays, and additively merges them.
Use `python3 fuzz/corpus_store.py validate CORPUS_ROOT` before any replay.

See [CORRECTNESS.md](../CORRECTNESS.md) for run counts, sanitizer limits,
proof domains and missing Miri shims. PR CI replays the targets; the optional
Verification dispatch builds, mutates, and retains corpus/crash artifacts for 14 days.

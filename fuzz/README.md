# Differential fuzzing

`encode_all` and `decode_all` compare configured Auto, the internal heap-free kernels, and the backends exposed by `__internals`
with an independent oracle: `bs58` up to 2,048 bytes, and above that
[Malachite's subquadratic radix conversion](../tests/support/reference.rs), which
calls nothing from this crate. `bs58` is quadratic and takes seconds per 64 KiB
input. A property test checks that the two oracles agree on both sides of the
switch. A [linear-time residue check](../tests/support/residue.rs) confirms each
successful oracle result: equal zero prefixes and equal values modulo three
fixed 61-bit primes. They check empty inputs, leading zeros, exact/short/oversized output
slices, result suffixes, and canaries around each slice. Results longer than 8,193
bytes or characters check exact or oversized slices, alternating by input, since
each costs a whole conversion. The quadratic `batched` and `no-alloc` hooks run only
on inputs up to 8,193 bytes or characters; unit tests check them at 200,001 bytes
and 300,001 characters. Decoding checks the first
invalid byte index. The public decoder takes raw bytes, so decode also passes the
fuzz input unchanged, including invalid UTF-8. The backend hooks take `&str`, so
they see malformed UTF-8 only after lossy conversion. The encode targets then
decode the expected text with every decoder and expect the original input, so
they need no decode oracle.

The forced `radix` and `radix-portable` encoders bypass production CPU/size
selection and participate in the shared differential checks on every target.
Thus x86 fuzz CI reaches the multiplication tree even though production selects
it only on little-endian AArch64 with NEON. Native ARM runs exercise NEON;
x86 runs do not establish NEON coverage. Both hooks fall back to the division
tree above the radix transform limit. The general targets' 65,537-byte binary
cap stays below the 262,144-byte cached production threshold; public dispatch and the 16 MiB limit
are covered separately by `radix_boundaries` tests.

`check_all` (feature `check`) compares Base58Check encoding and decoding with
bs58, with and without a version byte selected from the input. It encodes the
input as a payload, then decodes that frame, the frame with one changed character,
the raw input and the input mapped to digits. Each case checks exact, short and
oversized output slices, untouched tails and canaries; capacity errors may precede
checksum errors only when the data does not fit. The oracle takes bs58's checksum
result and applies the version rule itself: bs58 0.5.1 compares the version with the
first checksum byte when the data is empty, while this crate reports a missing
version. Inputs are capped at 8,193 bytes for the quadratic oracle.
Valid frames are also decoded with a different expected version, including
missing-version data. Payloads of 28 and 60 bytes seed complete 32- and 64-byte frames.
It has deterministic payload seeds, a stored corpus, nightly mutation and merge
lanes, and ASan source-coverage and candidate replays with `check` enabled.

Radix seeds include binary powers and sparse values around leaf/tree boundaries,
so zero subtrees and sparse carry paths do not depend on random mutations.
`zero-run-*` seeds are `encode_all` inputs whose text contains 2,000–30,000-digit
runs of `1`, so that zero leaves and subtrees of the division tree are covered from
the start.

`encode_radix_full_width` expands each raw input to 256 KiB or 1 MiB, selected
by its first byte's high bit, then sets the expanded first byte's high bit.
Each callback selects automatic, forced radix or portable radix
encoding and checks it against Malachite's independent integer conversion, with
an exact/oversized buffer and occasional short-buffer checks. Every callback
performs a full conversion; native/portable seeds initialize both forced hooks.
Avoiding the quadratic bs58 oracle makes large mutations practical.
Shrinking cannot avoid the cached production route on NEON; 1 MiB cases also
reach owned twiddles above log18. The `full-width` feature enables it; the optional
manual Verification fuzz dispatch runs 180-second careful ASan campaigns for default/cache and
requires actual mutations. This structured
input domain excludes leading zeros and does not cover every value at either width.

The three full-width targets share a deterministic expander. One-byte inputs and
multi-byte inputs with bit 6 clear repeat their raw pattern. Other multi-byte
inputs seed a SplitMix64 stream from all raw bytes, then overlay the raw bytes at
the start. Both modes remain in the seeds. This adds nonperiodic large values;
it does not add entropy to the seed or cover every value at the expanded width.

`decode_full_width` maps the expansion to valid Base58 at 128 KiB or 2 MiB of
text, selected by bit 7. Bit 5 adds a quarter-width run of leading `1`s; the
remaining suffix starts with `z`, so shrinking cannot avoid a large conversion.
Each callback selects one subquadratic decoder and compares it with Malachite's
independent conversion and the residue check, using the shared capacity and
canary checks. It then inserts an invalid ASCII or non-UTF-8 byte and checks the
public decoder's first-invalid offset. These targets use the `full-width`
feature and an 8,193-byte raw mutation cap.

`encode_all` accepts up to 65,537 bytes: one byte past 64 KiB. Full-width inputs
cross from 8,192 to 8,193 `u64` limbs. `decode_all` derives its raw-input budget from
`max_encoded_len(65_537)`, currently 89,502 bytes. It checks both lossy strings and
a valid alphabet mapping, then inserts an invalid ASCII or multibyte character.
This is a raw-byte budget; lossy UTF-8 can expand it by up to 3 times, and the
inserted character adds 1–4 UTF-8 bytes.
The target also checks `into_array` at capacities 0, 1, 32 and 64, using bounded
129-byte raw and mapped prefixes. It checks output bytes, exact length errors,
short buffers and invalid-byte offsets with the documented error precedence.

The generator includes all-`z` inputs at 89,499..89,502 characters. Such an input represents
`58^n - 1`: 89,499 characters fit 8,192 limbs, while 89,500 require 8,193. The final
89,502-character seed can decode to 65,538 bytes, so the decode budget does not
impose the encode target's binary-length limit.

These budgets reach divide-and-conquer, matrix and NTT paths, including
the larger IFMA multiplication thresholds on supported CPUs. They do not cover
all supported NTT sizes or force unsupported instructions. For nonzero valid
inputs near either decode cap, the current split/power bounds prepare log14
tables; moving the cap does not cross that boundary. The next prepared-table
transition is at 96,641 characters. Actual transform use also depends on operand
lengths and the selected multiplication thresholds.

Install `cargo-fuzz` 0.13.2 and the nightly in
[ci/nightly/rust-toolchain.toml](../ci/nightly/rust-toolchain.toml) with `rust-src`.
Use a C++ compiler compatible with that toolchain's LLVM runtime. Run from the
repository:

```sh
nightly=$(sed -n 's/^channel = "\(.*\)"$/\1/p' ci/nightly/rust-toolchain.toml)
python3 fuzz/test_seed_corpus.py
python3 fuzz/seed_corpus.py
cargo +"$nightly" fuzz run --careful --sanitizer address encode_all fuzz/corpus/encode_small -- -max_len=2048 -max_total_time=60 -timeout=120 -rss_limit_mb=4096
cargo +"$nightly" fuzz run --careful --sanitizer address decode_all fuzz/corpus/decode_small -- -max_len=2048 -max_total_time=60 -timeout=120 -rss_limit_mb=4096
cargo +"$nightly" fuzz run --careful --sanitizer address encode_all -- -runs=0 -max_len=65537 -timeout=120 -rss_limit_mb=4096
cargo +"$nightly" fuzz run --careful --sanitizer address decode_all -- -runs=0 -max_len=89502 -timeout=120 -rss_limit_mb=4096
```

Linux CI installs `clang-23` and `libclang-rt-23-dev` from the signed
`apt.llvm.org` LLVM 23 repository. The package revision may advance within LLVM
23, but the compiler major remains matched to Rust's LLVM runtime.

`--careful` rebuilds the standard library with extra initialization and UB checks;
debug assertions and integer overflow checks remain enabled. Repeat with
`--features cache`; use `--sanitizer leak` for the separate LeakSanitizer campaign.
The short-input campaigns perform mutations quickly; replaying
the full boundary corpus exercises larger kernels without spending the entire
mutation budget on large inputs. For sustained large-input fuzzing,
replace `-runs=0` with `-max_total_time=3600`.

`encode_full_width` adds structured large-input mutations. Each raw input expands
to exactly 65,537 bytes; its first byte has the high bit set. Empty input uses
`0xA5`. Thus shrinking raw inputs cannot avoid the native IFMA host's NTT paths.
It runs the same round-trip checks as `encode_all`, with the Malachite oracle,
capacities, canaries and owned-helper checks. This structured domain does not cover
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
`cargo +"$nightly" fuzz run --careful TARGET PATH_TO_ARTIFACT`.

`base58.dict` is versioned on `main` and is passed to the mutating decode lane.
It supplies invalid ASCII and UTF-8 boundaries; structured 64-byte
`split-fallback-*` seeds force the correlated base-58^4 condition that makes
the fixed encoder's split carry use its serial fallback. These inputs belong in
the deterministic generator because the relationship is arithmetic, not a
useful byte token for a dictionary.

`encode_all` has a custom mutator. One call in eight builds such a split-carry
input. Two in eight, for inputs of at least 256 bytes, mutate the input's base-58
digits instead of its bytes: they write a run of up to 20,480 zero digits, or apply
libFuzzer's mutations to the digit values, and decode the result back to bytes.
A changed byte rewrites every digit below it, so byte mutations almost never make
the zero-digit runs that zero whole leaves and subtrees of the division tree. The
other calls use libFuzzer's mutations. Replay, merge and coverage do not run the
mutator; the corpus keeps its byte results.

The shared corpus is not committed to `main`. The protected orphan
`fuzz-corpus` branch stores content-addressed inputs under
`corpora/<target>/<sha256>` and per-run metadata under `runs/`. Main/PR CI
only validates and replays a pinned revision. The optional Verification dispatch
with `run_fuzz` runs bounded campaigns on its selected ref but never publishes
the corpus. Nightly workers start from all deterministic seeds and a reproducible
sample of at most 16 stored inputs per target, and require actual mutations.
Their oracle wrapper keeps ASan but omits oracle fuzz instrumentation and debug
checks. Workers upload artifacts; every replay build adds the
artifacts that bring new coverage to the pinned corpus by libFuzzer merge, and
after an ASan replay of the combined result the separate trusted publisher
replaces the stored corpus with it. CI never removes a stored input
([details](../docs/fuzzing-corpus.md)).
Use `python3 fuzz/corpus_store.py validate CORPUS_ROOT` before any replay.

Initialization consumes `max_total_time`, but libFuzzer checks that limit only
afterward. Full-store replays and merges remain separate from mutation sampling.

To shrink the store, merge it with any new campaigns from the seeds alone, as in
[CI.md](../CI.md#fuzz-corpus-workflow), in every build that selects different code:
`-C target-cpu=generic` and `native`, cache off and on (`full-width` for the three
full-width targets and `check` for `check_all`), on AArch64 and on an x86-64 CPU
with AVX-512 IFMA and VBMI.
Store the union of the results only after an ASan replay of it on the same hosts.

See [CORRECTNESS.md](../CORRECTNESS.md) for run counts, sanitizer limits,
proof domains and missing Miri shims. PR CI replays the targets; the optional
Verification dispatch builds, mutates, and retains corpus/crash artifacts for 14 days.

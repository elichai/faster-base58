# Correctness and verification

Verification combines independent conversion oracles, property tests, arithmetic
tests, native sanitizers, Miri and scoped Kani proofs. Each checks different
properties. Passing these checks does not prove arbitrary-length conversion or
the semantics of every SIMD instruction. The implementation map is in
[DESIGN.md](DESIGN.md); repeatable jobs are in
[the workflow](.github/workflows/verification.yml).

[Latest local verification](docs/verification/local-ci-current.md) records the
workflow rows, source changes, exact commands and platform gaps. Earlier
dispatcher checks remain in the [verification index](docs/verification/README.md).

## Public contract

- The alphabet is Bitcoin Base58. Leading zero bytes become leading `'1'`s.
- Exact output capacity suffices. Successful calls leave the unused tail intact.
  Buffers may change on error. Invalid-input versus insufficient-capacity error
  precedence is unspecified when both apply.
- Invalid-character indices are UTF-8 **byte** offsets. `decode_array::<N>`
  requires exactly `N` bytes; shorter results return `InvalidLength`, longer
  ones `BufferTooSmall`.
- `no_alloc` performs no allocator operations, including first calls and errors.
  Auto may allocate workspace even with caller-provided output. Allocation
  failure can abort; conversion errors do not represent allocation failure.
- Work and dispatch depend on values and lengths. The crate is not constant-time.
  Transform and fuzz bounds are not maximum input lengths.

## Tests and independent references

| Check | What it exercises |
| --- | --- |
| [Public properties](tests/public_properties.rs) | Independent integer/radix conversion, zero prefixes, exact/short/oversized buffers, Unicode error indices, repeated output and dispatch history |
| [Bitcoin Core vectors](tests/bitcoin_core.rs) | 21 pinned known-answer vectors, zero extensions to 32/64 bytes, short output, preserved tails, and explicit strict whitespace/NUL rules; internal wrappers run when enabled |
| [Owned decoding](tests/owned_output.rs) | Maximum digits at capacity and dispatch boundaries, arbitrary digits, long zero prefixes, independent scalar values and byte offsets of late invalid characters |
| [Backend properties](tests/properties.rs) | Every registered encoder/decoder, portable controls, tuned leaves and thresholds, malformed text, recursive and NTT boundaries |
| [Scalar oracle](tests/support/scalar.rs) | Simple `u128` conversion for up to 16 significant binary bytes, with arbitrary leading zeros; independent of production limb arithmetic |
| [Large radix oracle](src/encode/tests.rs) | Independent Malachite `Natural` conversion plus bs58 comparisons; the [multiplier adapter](tests/support/malachite.rs) shares our conversion tree and is not an independent end-to-end oracle |
| [Allocation contract](tests/no_alloc.rs) and [preflight](tests/resource_preflight.rs) | Allocation, zeroed-allocation, reallocation and deallocation counters; cold/success/error paths and impossible-buffer rejection before table construction |
| Native kernel tests | Scalar/vector agreement, carry bounds, table coefficients, prepared/wrapped products, exact scratch, offsets and canaries, ISA guards |
| Concurrency tests | Cold Matrix initialization, shared tables and eight-thread NTT cache/history reuse |

Property strategies cover arbitrary binary data, valid Base58 strings and valid
UTF-8 containing malformed Base58. The API accepts `&str`, so raw invalid UTF-8
cannot enter it. bs58 0.5.1 supplies an additional oracle, including first-invalid
indices. Canary checks surround the passed output slice; they do not assert
unspecified error-buffer contents. Unit tests also cover MiB conversions, all-one
carry patterns, powers, small exhaustive inputs and all legal NTT scale lengths.

CI selects 512 cases per property in debug and release, cache off/on, across Linux,
macOS and Windows. Compact builds have additional Linux and macOS rows. Other
jobs cover 32-bit Linux, Malachite's 32-bit limb
mode, bare-metal no_std builds, Wasm/AArch64 builds, Rust 1.95, API namespace
boundaries, examples, Clippy, formatting and package contents. Build checks alone
do not execute kernels. The maximum `2^23`-point transform test runs separately;
two expensive Matrix sweeps execute with `cache` rather than rebuilding the
same owned tables for every size.

```sh
PROPTEST_CASES=512 cargo test --locked --features __internals --lib --tests
PROPTEST_CASES=512 cargo test --locked --features __internals,cache --lib --tests
cargo test --locked --release --features __internals --lib --tests
cargo test --locked --release --lib ntt::tests::largest_transforms -- --ignored --exact
cargo test --locked --doc
```

## Fuzzing and sanitizers

The [two general differential targets](fuzz/README.md) compare all backend
wrappers, Auto and `no_alloc`: values, capacities, suffixes, canaries, owned
helpers and malformed input. Decode also maps bytes to valid digits and inserts
invalid ASCII or multibyte characters, including late positions. Small mutation
inputs keep quadratic oracles useful; named boundary replay covers larger
recursive kernels.

General target budgets are 65,537 binary bytes and
`max_encoded_len(65_537) = 89,502` decode bytes. Lossy UTF-8 and inserted
characters can expand text. These budgets are not format limits or coverage of
every transform size. The 8,193-byte native mutation cap does not reach real NTT
thresholds on the IFMA host; large boundary replay, MSan and Miri do exercise NTT.

The optional `encode_full_width` target expands every raw mutation into a
periodic 65,537-byte value with its first bit set. It checks encoding and
generated-text decoding against independent oracles. The pinned-source geometry
audit places those values above the native NTT thresholds. The two added
native careful-ASan campaigns remain **unverified**. The cloud provider
preempted the VM during the first build. Independent retrieval confirmed
five helper passes and no completed build or campaign records. Recovery
was prepared on the same VM, then automated screening ended the delegated
agent turn before any recovery remote call, reporting "possible cybersecurity
risk". No tool explicitly rejected a command; no ASan campaign result was
produced. The recovered VM was later confirmed idle and stopped. Local
helper tests and target builds are separate qualification; compilation is
not campaign execution. This structured domain does not include every value
at that width.

On the frozen implementation, existing careful-ASan campaigns passed default,
cache, compact and cache+compact modes; LSan passed default/cache. Each target
used 180 seconds followed by full named-boundary replay. Callbacks include
initial corpus processing and possible repeats; they are not distinct mutations
or seed-file counts. The new full-width stages additionally require a positive
post-initialization mutation-attempt count from libFuzzer's corpus statistics.

Four native Rust/std MSan variants (cache off/on × compact off/on) passed 20
covered tests each, with IFMA required and zero ISA skips. Native Linux TSan
passed five concurrency tests. The exact maximum transform passed at `2^23`
points. Windows runtime checks remain unavailable.

Both fresh named-boundary source-coverage rows replayed 169 encode and 201 decode
files per cache policy. Per-target library line counts are 2608/3669 encode and
1468/3669 decode without cache, and 2699/3777 encode and 1513/3777 decode with
cache. The target denominators overlap and are not added. These are unsanitized,
translated-x86 observations without branch instrumentation or native IFMA
coverage. [Current local verification](docs/verification/local-ci-current.md)
records source scopes, commands, tool hashes and counters. Earlier sanitizer and
coverage records remain at their original sources in the
[verification index](docs/verification/README.md).

A historical full MSan/libFuzzer configuration failed in file-discovery startup
before the Base58 callback. A later full campaign was rejected by automatic
execution review, which stated "possible cybersecurity risk"; that campaign was
not run. Neither is a passing campaign or an identified crate defect. Passing
Rust/std MSan avoids that C++ engine and does not close the gap. No sanitizer
runtime was patched to obtain the passes. TSan does not fully model atomic
fences or inline assembly; its results supplement the other checks.

## Miri

Supported tests use strict provenance; selected paths also run with Tree Borrows
and a different seed. They cover portable recursive/prepared/wrapped arithmetic,
fixed encoders, exact scratch and supported SSE4.1/AVX2/NEON panel operations.
Miri executes MIR: it does not verify LLVM auto-vectorization or emitted native
instructions. Native sanitizer and oracle tests remain necessary.

Twelve LLVM intrinsics lack shims in stock `nightly-2026-10-26`. A direct run of
all 15 probes confirmed these failures; the other three permutation probes pass.

| Target | Missing intrinsic |
| --- | --- |
| AArch64 | `llvm.aarch64.neon.srshl.v4i32` |
| AArch64 | `llvm.aarch64.neon.vsli.v4i32` |
| AArch64 | `llvm.aarch64.neon.vsli.v8i16` |
| AArch64 | `llvm.aarch64.neon.tbx1.v16i8` |
| AArch64 | `llvm.aarch64.neon.tbx4.v16i8` |
| AArch64 | `llvm.aarch64.neon.shsub.v4i32` |
| x86 | `llvm.x86.avx512.psrlv.q.512` |
| x86 | `llvm.x86.avx512.psllv.q.512` |
| x86 | `llvm.x86.avx512.vpmadd52l.uq.512` |
| x86 | `llvm.x86.avx512.vpmadd52h.uq.512` |
| x86 | `llvm.x86.avx512.vpermi2var.d.512` |
| x86 | `llvm.x86.avx512.vpermi2var.qi.256` |

[Minimal probes](tests/miri_intrinsics.rs) isolate them;
[the runner](scripts/miri-extended.sh) retains every probe. The workflow's
`extended_miri` input intentionally exposes those failures rather than marking
unsupported instructions as passes. It is off for normal CI.

The [pinned development interpreter](ci/miri/README.md) supplies these shims
without changing library dependencies. Its reviewed patch preserves the
maintainer's shift, permutation, table-lookup and IFMA implementations, and adds
signed halving subtraction for NEON NTT. Signed extrema and negative odd
differences agree with native ARM execution. Custom-interpreter results remain
separate from stock results. Its runner checks exact test selection and splits
all 14 original buffer cases across processes to bound interpreter memory.

The final frozen implementation passed **186 custom-interpreter test
executions** plus eight discovery commands, and **16 supported-stock executions**.
Tree Borrows covers full ARM/x86 suites under both cache policies; Stacked
Borrows covers kernels. Compact x86 kernels run under both models and policies.
The disjoint union reuses 72 exactly matched kernel invocations once. All 14
original buffer cases run separately; no ISA-skip output is accepted.

[Latest local verification](docs/verification/local-ci-current.md) binds those
selections and the audited reuse of the pinned Linux cold setup. These current
tests are M5-host interpretations of Linux targets. The
[historical Miri component](docs/verification/miri-current.md) retains its earlier
150 custom executions, 16 stock executions, 15 setup probes and original source
bridges. SIMD multiplication uses owned tables; native oracle and cache-history
tests supply broader shared-transform coverage. These are concrete executions,
not native x86 execution or complete-domain proofs.

## Kani proofs

The frozen implementation passed **50 distinct required harnesses** plus three
overlapping compact executions with Kani 0.68.0 and CBMC 6.11.0 on translated
x86 Linux (64-bit `usize`). The run produced 18,744 default checks and 8,096 compact
checks. Repeated bodies are not additional proofs. Earlier M5 components retain
their own source maps. Memory, overflow and unwinding checks remain enabled;
finite loops have explicit bounds. The local scheduling-only `--jobs 1`
adaptation preserves domains, solvers and the original 60/180/360-second budgets.

| Obligation family | Proven domain or property |
| --- | --- |
| Tiny production conversion | Every one-byte encoder value and capacity 0–3; valid UTF-8 text of exactly two bytes and capacity 0–3; alphabet and lookup round trips |
| Generic batched decoding | All `58^2` valid two-digit strings and capacities 0–3; numeric value, leading zeros, errors and output canaries |
| Parsing | All 256 lookup entries; validity through four bytes, zero split through eight, partial chunks through nine; full ten-byte numeric value and invalid rejection by verified composition |
| Limb arithmetic | Full-width two-limb carry/borrow; one-limb multiplication; single radix updates; three limbs folded into one modulo `2^64 - 1`; exact Toom division by three |
| NTT scalar operations | Reduction for every `i32`, normalization in `(-q,q)`, loading for every `u32`, all three primes; plan bounds and scale at every power-of-two length 1 through `2^23` |
| Matrix / IFMA models | Two-column carries; eight-column radix-52 carry; 13 limbs ↔ 16 radix-52 digits through a scalar vector model |
| Capacity | Actual encoded-bound safety, coverage of input count, endpoint saturation and full adjacent monotonicity; owned decoded-bound arithmetic and safe prefix addition for every 64-bit `usize` |
| Paired update safety | Full-width stored limb and nonzero top, two spare limb slots, both digits below `58^10`: interleaved and sequential updates succeed; numeric equality remains open |

The parser composition checks the unchanged production parser. Its digit-range
and split/Horner contracts are separately proved. Adjacent capacity monotonicity
uses separately proved affine, clamp and actual-function product contracts.
The logarithmic argument connecting either capacity bound to actual conversion
length remains outside Kani's scope. Scalar models do not prove emitted SIMD,
arbitrary-length conversions or concurrent-cache correctness.

[Historical per-target components](docs/verification/proofs-current.json) list all 50 names,
domains, dependencies, portable command recipes, input hashes and raw-log hashes.
[Earlier proof records](docs/verification/README.md) remain historical evidence.
The production script and workflow select the required obligations. The
[latest local receipt](https://github.com/elichai/faster-base58/blob/main/docs/verification/data/local-ci-current.json) binds the fresh Linux component.

For the two-digit query, reducing unwind 12 to 6 removed excessive symbolic
iterator expansion. All 14 unwinding assertions then passed, so the original
input domain remains covered. CaDiCaL avoided a CBMC SMT-conversion failure.
It proved 1,352 checks, including 54 unreachable, on each target. M5 completed in
147 seconds and the earlier Linux run in 254 seconds. The fresh final-source
command completed in 316.85 seconds within the same 360-second budget.

A separate optional encoder-split query kept its original full domain and
production body, using opt0 Z3/polynomial normalization. It reached solving
and timed out at 90 seconds, without a theorem or counterexample. This is
distinct from paired-query lowering errors. All eight selectors remain
open; the [latest receipt](https://github.com/elichai/faster-base58/blob/main/docs/verification/data/local-ci-current.json) retains the scoped attempt index.

Wider paired numeric equality, encoder split/step and fixed carries, actual
Barrett division and the public borrowed-array wrapper remain unresolved.
Recent paired queries timed out or hit a CBMC
translation error; Barrett timed out during symbolic execution. These are
neither counterexamples nor proofs. [The open runner](scripts/kani-open.sh)
retains eight distinct unresolved queries. Historical direct parser/capacity
formulations remain in source and archives; their properties are now closed by
verified composition and are no longer classified as open.

## Remaining work

1. Upstream the twelve missing Miri shims. Keep stock and development-interpreter
   results separate, including strict provenance and both alias models.
2. Discharge the wider Kani obligations through further decomposition. There is
   no arbitrary-length end-to-end proof of each backend.
3. Complete the two unverified full-width native ASan stages. Repair or
   replace the libFuzzer/MSan integration before claiming MSan fuzzing.
   Continue longer campaigns and inspect coverage gaps; all completed campaigns
   are finite. Run the native Windows tests when a host is available.
4. Repeat deployment-specific performance, peak memory and linked-size checks.
   Historical benchmark source generations are labelled in
   [the data index](docs/benchmarks/README.md); they are not fresh release numbers.
5. Resolve any confirmed source-adaptation notice obligations described in
   [REFERENCES.md](REFERENCES.md), and make an explicit release decision after CI.

The repository has not been published to crates.io. The current report describes
local checks; hosted results remain tied to their earlier commits. A passing job
establishes only the checks and platforms it actually executes.

# Correctness and verification

Verification combines independent conversion oracles, property tests, arithmetic
tests, native sanitizers, Miri and scoped Kani proofs. Each checks different
properties. Passing these checks does not prove arbitrary-length conversion or
the semantics of every SIMD instruction. The implementation map is in
[DESIGN.md](DESIGN.md); repeatable jobs are in
[the workflow](.github/workflows/verification.yml).

## Recorded check scope

As of 2026-10-28, the recorded local checks cover runtime resource source
`304dc4c5dbf1`, with original test executions under source `7dcfc511`.
Later workflow and full-width fuzz-target changes have separate qualification.
These source-scoped local results do not establish that every current CI row
passed.

| Check | Recorded result and scope |
| --- | --- |
| Tests | Native AMD EPYC 9B45 Linux and Apple M5 ARM, debug/release and cache off/on; compact variants; 512 cases per selected property |
| ASan / LSan | Existing native careful-ASan targets passed default/cache/compact/cache+compact; LSan passed default/cache; 180 seconds per target followed by named-boundary replay. Two added full-width ASan stages remain unverified. |
| MSan / TSan | Four native Rust/std MSan variants passed 20 covered tests each, requiring IFMA with no ISA skips; native Linux TSan passed five concurrency tests. Full libFuzzer/MSan campaigns remain incomplete. |
| Maximum transform | Native x86 Linux execution passed the exact `2^23`-point test. |
| Miri | M5-host interpretation of Linux targets: 186 custom-interpreter test executions plus eight discovery commands; 16 supported-stock executions. ARM/x86, cache policies, compact kernels and Tree/Stacked models have distinct selections. |
| Kani | Translated x86 Linux, 64-bit `usize`: 50 distinct required harnesses plus three overlapping compact executions; 18,744 default and 8,096 compact checks. |
| Source coverage | Translated x86 Linux, unsanitized: 169 encode and 201 decode boundary files per cache policy. Encode/decode library lines: 2608/3669 and 1468/3669 without cache; 2699/3777 and 1513/3777 with cache. No branch or native IFMA coverage; overlapping denominators are not added. |
| Platform gaps | Four native Windows runtime rows were unavailable. Cross-builds and translated execution do not establish native execution. |

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
| [Radix boundaries](tests/radix_boundaries.rs) | Malachite public-API oracle, leading zeros, exact/short buffers and guards; normally ignored 16 MiB−1 / 16 MiB / 16 MiB+1 cases run explicitly in ARM release CI |

The radix allocation regression failed on the parent and passes with one reused
digit workspace. Its 1/4 MiB cases now also bound allocation requests after
pooling power digits and spectra, and guard both ends of the output.
Product tests check poisoned output and the live low-child
prefix consumed by fused carry. Forced best/portable radix hooks participate in
unit, property and fuzz backend lists; their out-of-domain fallback is checked.

The whole-limb formatter's `ten_digits_matches_division` test compares serial
division at both domain ends, radix boundaries and 10,000 seeded values.
The 2026-10-29 extracted-formatter check also tested all 656,356,768 five-digit
halves with the other half zero, pair boundaries and one million complete limbs.
That is bounded enumeration, not exhaustive coverage of the `58^10` domain.

Property strategies cover arbitrary binary data, valid Base58 strings and valid
UTF-8 containing malformed Base58. The API accepts `&str`, so raw invalid UTF-8
cannot enter it. bs58 0.5.1 supplies an additional oracle, including first-invalid
indices. Canary checks surround the passed output slice; they do not assert
unspecified error-buffer contents. Unit tests also cover MiB conversions, all-one
carry patterns, powers, small exhaustive inputs and all legal NTT scale lengths.

CI selects 512 cases per property in debug and release, cache off/on, across Linux,
macOS and Windows. Compact builds have additional Linux and macOS rows. Big-endian
PowerPC64 tests run under cross/QEMU in optimized debug builds, cache off/on,
including unit, integration and documentation tests, with 512 cases per property.
The cross revision and container image digest are pinned. Other
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

`encode_radix_full_width` uses an independent Malachite oracle on periodic
256 KiB / 1 MiB values, reaching owned large twiddles. Native ARM careful-ASan
smoke campaigns for pooled-scratch snapshot `11d0c16` completed on 2026-10-29,
before the table-alignment and lint integration:
19 default and 8 cache post-initialization mutation attempts over 65/66 seconds,
without crashes. These short runs are not a refreshed coverage percentage or a
proof. General targets cover leading zeros, and boundary tests cover larger
sizes. Arbitrary large values remain outside this structured fuzzing domain.

A local source-coverage replay on 2026-10-29 used integrated snapshot `fbe6043`,
native M5/AArch64, generic target CPU, nightly `2026-10-26`, and cargo-fuzz
`0.13.2`, with `--cfg careful`, debug assertions and no sanitizer. Sixteen
`encode_radix_full_width` corpus files reached 235/261 lines (90.04%), 334/377
regions (88.59%) and 21/23 functions in `src/ntt/radix.rs` with default features.
There is no branch-coverage result. This fixed corpus replay is not a mutation
campaign, cache-policy coverage, or coverage of all input values.

The optional `encode_full_width` target expands every raw mutation into a
periodic 65,537-byte value with its first bit set. It checks encoding and
generated-text decoding against independent oracles. The pinned-source geometry
audit places those values above the native NTT thresholds. Its two added native
careful-ASan stages remain **unverified**; helper tests and target compilation
are not campaign execution. This structured domain does not include every value
at that width.

Campaign callback counts include initial corpus processing and possible repeats;
they are not distinct mutations or seed-file counts. Full-width stages require
a positive post-initialization mutation-attempt count from libFuzzer's corpus
statistics. Keep bounded mutation campaigns separate from large boundary replay.

Passing Rust/std MSan tests does not close the full libFuzzer/MSan campaign gap.
No sanitizer runtime was patched to obtain the passes. TSan does not fully model
atomic fences or inline assembly; its results supplement the other checks.

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

In the recorded runs, Tree Borrows covers full ARM/x86 suites under both cache
policies; Stacked Borrows covers kernels. Compact x86 kernels run under both
models and policies. The execution total counts 72 exactly matched reused kernel
invocations once. All 14 buffer cases run separately; no ISA-skip output is
accepted. SIMD multiplication uses owned tables; native oracle and cache-history
tests supply broader shared-transform coverage. These are concrete executions,
not native x86 execution or complete-domain proofs.

## Kani proofs

The recorded proofs use Kani 0.68.0 and CBMC 6.11.0. Repeated bodies are not
additional proofs. Memory, overflow and unwinding checks remain enabled; finite
loops have explicit bounds. Required harnesses use 60/180/360-second budgets.

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

Wider paired numeric equality, encoder split/step and fixed carries, actual
Barrett division and the public borrowed-array wrapper remain unresolved.
Timed-out or untranslated queries are neither counterexamples nor proofs.
[The open runner](scripts/kani-open.sh) selects eight distinct unresolved queries.
Direct parser/capacity formulations also remain in source; their properties are
closed by verified composition and are no longer classified as open.

## Reproduce and update checks

Use the feature/target matrix and pinned tools in
[the verification workflow](.github/workflows/verification.yml). The commands
above cover ordinary local tests; compact, native sanitizer, Miri and proof rows
need their own selections and hosts. [Fuzzing](fuzz/README.md) documents campaign
and boundary replay commands. [Development Miri](ci/miri/README.md) describes
the pinned interpreter; [the extended runner](scripts/miri-extended.sh) isolates
stock unsupported-intrinsic diagnostics. The workflow selects the full required
Kani set; [kani-production.sh](scripts/kani-production.sh) runs the tiny public
conversion and alphabet obligations only.

When code changes, record the tested source, tool versions, features, target,
actual execution host and selected tests. Distinguish compiled, executed,
unsupported, skipped and incomplete checks. Require a nonzero test selection,
and require ISA execution where a row claims SIMD coverage. Preserve proof
domains, overflow checks and unwinding assertions when changing solver settings.
Update the dated scope above after fresh checks; documentation edits alone do
not refresh a prior result. For performance and memory work, use the separate
[measurement workflow](docs/performance.md).

## Remaining work

1. Upstream the twelve missing Miri shims. Keep stock and development-interpreter
   results separate, including strict provenance and both alias models.
2. Discharge the wider Kani obligations through further decomposition. There is
   no arbitrary-length end-to-end proof of each backend.
3. Complete the two unverified full-width native ASan stages. Repair or
   replace the libFuzzer/MSan integration before claiming MSan fuzzing.
   Continue longer campaigns and inspect coverage gaps; all completed campaigns
   are finite. Run the native Windows tests when a host is available.
4. Repeat deployment-specific performance, peak memory and linked-size checks
   using [the measurement guide](docs/performance.md).
5. Resolve any confirmed source-adaptation notice obligations described in
   [REFERENCES.md](REFERENCES.md), and make an explicit release decision after CI.

A passing job establishes only the checks and platforms it actually executes.

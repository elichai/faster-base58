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

[Two differential targets](fuzz/README.md) compare all backend wrappers, Auto and
`no_alloc`, checking values, capacities, suffixes, canaries and malformed input.
Owned string/vector helpers share those oracles. Decode also maps bytes to valid
digits and injects invalid ASCII or multibyte characters. The selector can
choose late insertion positions. Smaller mutation
corpora keep the quadratic oracles useful;
separate boundary replays exercise larger recursive kernels.

The raw budgets are 65,537 binary bytes and `max_encoded_len(65_537) = 89,502`
decode bytes. They cross 8192/8193-limb boundaries. Lossy UTF-8 and inserted
characters can expand the decoded text. These are campaign budgets, not format
limits; they do not exercise every transform size.

The final owned-decoder source completed **59,718 AddressSanitizer** and
**53,574 LeakSanitizer runs in mutation campaigns**, plus **1,508 runs in
boundary replays**. These are libFuzzer's reported execution counters, including
initial corpus processing and any repeated executions. They are not counts of
distinct mutations or named seed files.
ASan used 600 seconds per target/cache policy; LSan used 180. Both used
`--careful`, rebuilt std, debug assertions and overflow checks. The later
Kani-only unwind change leaves production conversion code unchanged.
[Current receipts](docs/verification/local-ci-current.md) preserve that source distinction.

Additional native x86 ASan campaigns completed 52,466 reported runs during
mutation campaigns and 745 during boundary replays on the AMD EPYC 9B45 VM.
Each target/cache policy used 180 seconds, with the same careful checks and
full boundary inputs. The native and translated campaigns retain separate
source, tool, corpus and execution counts.

Separate unsanitized coverage builds replayed 5,008 accumulated corpus cases per
cache policy against that source. They observed 2,628/3,669 library lines without
cache and 2,721/3,777 with cache on translated x86 Linux. These are fuzz-only
observations, including unexecuted AVX-512/IFMA code and convenience wrappers
tested separately by public properties. No branch coverage was collected.
CI also replays all 169 encode and 201 decode named boundary files per policy
for source coverage; it retains profiles and reports.

Rust MSan passed 19 covered tests per cache policy on translated Linux, with an
explicit IFMA skip. Native GCP MSan passed 20 tests per policy, requiring IFMA
and accepting no ISA skips. TSan passed five concurrency tests on each native
macOS ARM and Linux x86 host. The Rosetta-only TSan startup failure remains
recorded separately. The maximum-transform test also passed natively at
`2^23` points with portable and selected vector kernels. Windows runtime
checks remain unavailable.

The historical 2026-10-26 careful AddressSanitizer campaign completed **450,058 executions in mutation campaigns**
across default/cache encode/decode, plus full and supplemental boundary replays.
Linux LeakSanitizer completed another **249,334 executions**. Both retain debug
assertions and overflow checks. ThreadSanitizer passed five nonzero concurrency
test invocations on each of Apple Silicon and Linux x86-64. Native Rust
MemorySanitizer passed six guarded Matrix/NTT/IFMA kernel commands and the public
API command on the AMD EPYC 9B45 VM, with rebuilt standard library/dependencies
and no suppressions. Source identities and tool versions are retained in the
[sanitizer receipt](docs/verification/sanitizers.json).

The historical libFuzzer/MSan configuration failed in the fuzzer's file-discovery startup
before the Base58 callback. It is not a successful fuzz campaign or an identified
crate defect. Native Rust MSan avoids that C++ engine; it does not close that
tooling gap. A current full MSan/libFuzzer campaign remains unverified; the
current report records the execution-tool limitation. No sanitizer runtime was
patched to obtain the passing results.
ThreadSanitizer does not fully model synchronization through atomic fences or
inline assembly; its results supplement the other checks.

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

[Current Miri evidence](docs/verification/miri-current.md) records 150 passing
custom-interpreter executions across both targets, cache settings and alias
models; 16 supported-stock executions; and 15 separate cold-Linux-driver probes.
The receipt includes exact frozen inputs and the later Kani-only source change.
SIMD multiplication uses owned tables; shared-table initialization and reuse
have separate policy checks. These are concrete interpreter runs, not native
x86 execution or complete-domain proofs.

The owned-decoder wrapper then passed 28 focused custom-Miri executions across
both targets, cache settings and alias models. Those later runs and their source
bridge are separate from the original 150 in the current local receipt.

## Kani proofs

The selected suite has passing records for **50 distinct harnesses** with
Kani 0.68.0 and CBMC 6.11.0 on aarch64 macOS and translated x86 Linux, both with
64-bit `usize`. The original 49 and the additional decoder proof have distinct
compiled input manifests. Repeated capacity checks are not additional harnesses.
Memory, overflow and unwinding checks remain enabled. Default unwind is one;
finite loops have explicit bounds. The receipts retain per-target counts and
solver versions; these are local executions.

| Obligation family | Proven domain or property |
| --- | --- |
| Tiny production conversion | Every one-byte encoder value and capacity 0–3; valid UTF-8 text of exactly two bytes and capacity 0–3; alphabet and lookup round trips |
| Generic batched decoding | All `58^2` valid two-digit strings and capacities 0–3; numeric value, leading zeros, errors and output canaries |
| Parsing | All 256 lookup entries; validity through four bytes, zero split through eight, partial chunks through nine; full ten-byte numeric value and invalid rejection by verified composition |
| Limb arithmetic | Full-width two-limb carry/borrow; one-limb multiplication; single radix updates; three limbs folded into one modulo `2^64 - 1`; exact Toom division by three |
| NTT scalar operations | Reduction for every `i32`, normalization in `(-q,q)`, loading for every `u32`, all three primes; plan bounds and scale at every power-of-two length 1 through `2^23` |
| Matrix / IFMA models | Two-column carries; eight-column radix-52 carry; 13 limbs ↔ 16 radix-52 digits through a scalar vector model |
| Capacity | Actual encoded-bound safety, coverage of input count, endpoint saturation and full adjacent monotonicity; owned decoded-bound arithmetic and safe prefix addition for every `usize64` |
| Paired update safety | Full-width stored limb and nonzero top, two spare limb slots, both digits below `58^10`: interleaved and sequential updates succeed; numeric equality remains open |

The parser composition checks the unchanged production parser. Its digit-range
and split/Horner contracts are separately proved. Adjacent capacity monotonicity
uses separately proved affine, clamp and actual-function product contracts.
The logarithmic argument connecting either capacity bound to actual conversion
length remains outside Kani's scope. Scalar models do not prove emitted SIMD,
arbitrary-length conversions or concurrent-cache correctness.

[The current receipt](docs/verification/proofs-current.json) lists all 50 names,
domains, dependencies, portable command recipes, input hashes and raw-log hashes.
[Earlier proof records](docs/verification/README.md) remain historical evidence.
The production script and workflow select the required obligations.

For the two-digit query, reducing unwind 12 to 6 removed excessive symbolic
iterator expansion. All 14 unwinding assertions then passed, so the original
input domain remains covered. CaDiCaL avoided a CBMC SMT-conversion failure.
It proved 1,352 checks, including 54 unreachable, on each target. M5 completed in
147 seconds; translated Linux needed 254 seconds and a 360-second budget.

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
3. Repair or replace the libFuzzer/MSan integration before claiming MSan fuzzing.
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

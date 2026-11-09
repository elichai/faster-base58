# Correctness and verification

Verification combines independent conversion oracles, property tests, arithmetic
tests, native sanitizers, Miri and scoped Kani proofs. Each checks different
properties. Passing these checks does not prove arbitrary-length conversion or
the semantics of every SIMD instruction. The implementation map is in
[DESIGN.md](DESIGN.md); repeatable jobs are in
[the workflow](.github/workflows/verification.yml), and [CI.md](CI.md) gives
their local commands.

## Recorded check scope

[CI](.github/workflows/verification.yml) is the record for each commit; a passing
job establishes only the checks and platforms it executes. This revision passes, on the M5, the debug and
release `__internals` suites with and without `cache` (512 cases per property), the
`check` and no-allocation lanes, documentation tests, the 16 MiB radix boundaries
(both policies) and the `2^23`-point transform test. On a Zen 5 host with AVX-512
IFMA and VBMI it passes the `__internals` suites with and without `cache` with
`BASE58_REQUIRE_IFMA=1`, so the native AVX-512 kernel tests run instead of skipping.

## Public contract

The default-on `alloc` feature enables allocating backends and owned results.
`cache` and `__internals` imply `alloc`. Allocation-free public tests run with
`--no-default-features --features check`.
The downstream API checker also checks owned/check method availability and
rejects the removed free functions and `no_alloc` module.

- The alphabet is Bitcoin Base58. Leading zero bytes become leading `'1'`s.
- Exact output capacity suffices. Successful calls leave the unused tail intact.
  Buffers may change on error. Invalid-input versus insufficient-capacity error
  precedence is unspecified when both apply.
- Decoding accepts strings or raw bytes; bytes need not be UTF-8. Invalid-character
  indices are **byte** offsets. `.into_array::<N>()` requires exactly `N` bytes;
  shorter results return `InvalidLength`, longer ones `BufferTooSmall`.
- Without `alloc`, conversion performs no allocator operations, including first
  calls and errors. Borrow input to exclude allocator calls from dropping owned
  input. With `alloc`, conversion may allocate workspace even with caller-provided output. Allocation
  failure can abort; conversion errors do not represent allocation failure.
- Work and dispatch depend on values and lengths. The crate is not constant-time.
  Transform and fuzz bounds are not maximum input lengths.
- `Encoder` / `Decoder` configurations are reusable constants. They convert only
  at terminal methods. The optional `check` feature adds Base58Check: first four
  bytes of double SHA-256 over data including any version prefix. Decoding checks
  the checksum before the requested version, removes only the checksum, and retains
  the version. Valid empty data is accepted without a version requirement.
- Checked borrowed output requires only data capacity. A separate four-byte
  checksum segment is held internally, leaving the entire unused output tail
  untouched on success. Checked arrays use the same split destination with a
  plain stack array and report shorter data as `InvalidLength`, longer data
  as `BufferTooSmall`. `Error` is non-exhaustive; downstream matches must retain
  a wildcard arm, including when enabling feature-gated checksum/version errors.

## Tests and independent references

| Check | What it exercises |
| --- | --- |
| [Configurable codec](tests/builders.rs) | Const configuration, owned/borrowed inputs, plain codec agreement, known Base58Check answers, bs58 differential properties, version insertion/validation/retention, empty/truncated/corrupted frames, large-frame checksum-before-version errors across shared backends, exact data capacity with a separate checksum segment, and untouched guards/tails |
| [Public properties](tests/public_properties.rs) | Independent integer/radix conversion, zero prefixes, exact/short/oversized buffers, Unicode error indices, repeated output and dispatch history |
| [Bitcoin Core vectors](tests/bitcoin_core.rs) | 21 pinned known-answer vectors, zero extensions to 32/64 bytes, short output, preserved tails, and explicit strict whitespace/NUL rules; internal wrappers run when enabled |
| [Owned decoding](tests/owned_output.rs) | Maximum digits at capacity and dispatch boundaries, arbitrary digits, long zero prefixes, independent scalar values and byte offsets of late invalid characters |
| [Backend properties](tests/properties.rs) | Every registered encoder/decoder, portable controls, tuned leaves and thresholds, malformed text, recursive and NTT boundaries |
| [Scalar oracle](tests/support/scalar.rs) | Simple `u128` conversion for up to 16 significant binary bytes, with arbitrary leading zeros; independent of production limb arithmetic |
| [Shared oracle](tests/support/mod.rs) | bs58 up to 2,048 bytes, then Malachite's [subquadratic conversion](tests/support/reference.rs), which calls nothing from this crate; a property checks that they agree up to 8,193 bytes, including invalid and non-ASCII bytes. Round trips expect the original bytes back, without a decode oracle |
| [Residue oracle](tests/support/residue.rs) | Confirms every successful oracle result in linear time: equal zero prefixes, then equal values modulo three 61-bit safe primes unrelated to the NTT moduli. A wrong value passes only if its error is a multiple of their product (above 2^182); one-digit and one-byte changes never pass. A property checks that it accepts the bs58 encoding and rejects one-element insertions, removals and replacements of text or bytes |
| [Large radix oracle](src/encode/tests.rs) | Independent Malachite `Natural` conversion plus bs58 comparisons; the [multiplier adapter](tests/support/malachite.rs) shares our conversion tree and is not an independent end-to-end oracle |
| [Allocation contract](tests/no_alloc.rs) and [preflight](tests/resource_preflight.rs) | Allocation, zeroed-allocation, reallocation and deallocation counters; cold/success/error paths, checked array storage, no extra check-frame allocations versus ordinary codec workspace, and impossible-buffer rejection before table construction |
| Native kernel tests | Scalar/vector agreement, carry bounds, table coefficients, prepared/wrapped products, exact scratch, offsets and canaries, ISA guards |
| Concurrency tests | Cold Matrix initialization, shared tables and eight-thread NTT cache/history reuse |
| [Radix boundaries](tests/radix_boundaries.rs) | Malachite public-API oracle, leading zeros, exact/short buffers and guards on AArch64 NEON and x86-64 (the radix route needs AVX-512F there); normally ignored 16 MiB−1 / 16 MiB / 16 MiB+1 cases run explicitly in ARM release CI |
| [Public API](tests/api.rs) and [tiny inputs](tests/tiny_inputs.rs) | Borrowed/owned agreement, exact arrays, zero prefixes and capacities at dispatch boundaries, standard error traits; every byte and short text against bs58 with byte offsets |
| [Cold Matrix tables](tests/matrix_cold.rs) and [Miri entry points](tests/miri.rs) | First decodes from several threads in a fresh process; buffer cases split for the custom Miri runner |

Radix allocation tests bound allocation requests and peak requested live bytes at
1/4 MiB and guard both ends of the output. A forced owned-NTT case checks one allocation
per prime plus the packed conversion arena, without prepared spectra or reallocations.
Scratch tests enumerate every plan through the transform limit and retain the
running workspace maximum, including alignment and recursion requirements.
Product tests check poisoned output and the live low-child
prefix consumed by fused carry. Forced best/portable radix hooks participate in
unit, property and fuzz backend lists; their out-of-domain fallback is checked.

The whole-limb formatter's `ten_digits_matches_division` test compares serial
division at both domain ends, radix boundaries and 10,000 seeded values; that is
not exhaustive coverage of the `58^10` domain.

Property strategies cover arbitrary binary data, valid Base58 strings and valid
UTF-8 containing malformed Base58. The public decoder also takes raw bytes; the
builder tests, heap-free tests and `decode_all` fuzzer pass invalid UTF-8 to it. bs58 0.5.1 supplies an additional oracle up to 2,048 bytes, including
first-invalid indices; Malachite replaces it above that length. Canary checks surround the passed output slice; they do not assert
unspecified error-buffer contents. Unit tests also cover MiB conversions, all-one
carry patterns, powers, small exhaustive inputs, all legal NTT scale lengths and the
partial-word store at every length from 0 to 8 bytes, with guard bytes.

CI selects 512 cases per property in debug and release, cache off/on, on x86-64
and arm64 Linux. Compact builds have additional x86-64 Linux rows. Big-endian
PowerPC64 tests run under cross/QEMU in optimized debug builds, cache off/on,
including unit, integration and documentation tests, with 512 cases per property.
In the `tests` job, the x86-64 Linux debug rows (cache off and on) run the unit,
API and property suite with `cargo test`, so the tests of one binary share a
process. All other unit and integration lists in `tests`, `compact`,
`alternate_limb_width`, `msrv` and `pointer_width_32` use Nextest, including the
arm64 Linux debug rows. Documentation examples and the two ignored boundary tests
(16 MiB radix and `2^23`-point transform) use `cargo test`. The cross revision
and container image digest are pinned, and so are the toolchains: stable in
[rust-toolchain.toml](rust-toolchain.toml), which also applies to local builds in
the repository, and the nightly in
[ci/nightly/rust-toolchain.toml](ci/nightly/rust-toolchain.toml). Dependabot
proposes toolchain updates. Other jobs cover 32-bit Linux, Malachite's 32-bit limb
mode, bare-metal no_std builds, Wasm/AArch64 builds, Rust 1.95, API namespace
boundaries, Clippy, formatting and package contents. Build checks alone
do not execute kernels. The maximum `2^23`-point transform test runs separately;
two expensive Matrix sweeps execute with `cache` rather than rebuilding the
same owned tables for every size. [CI.md](CI.md#test-jobs) gives the local
commands for these rows.

The SIMD inlining job builds the Auto consumer library with generic CPU flags for
x86-64 and arm64 Linux, each default, `cache` and `compact`, on one x86-64 runner
(arm64 is cross-compiled). [check_simd_inlining.py](scripts/check_simd_inlining.py)
compares every out-of-line function whose name mentions the crate or a `core::arch`
intrinsic with [check_simd_inlining.txt](scripts/check_simd_inlining.txt), per
configuration. It fails on an unlisted function and on a listed function that is no
longer out of line, so the list records LLVM's inlining decisions for the pinned
toolchain; a code or toolchain change that moves a function updates the list. The
list comments the SIMD functions kept apart on purpose: dispatch-table entries,
target-feature entry points, kernels selected by pointer and cold paths. The check
reads symbols only: it does not see slow code inside an inlined body. `--build`
needs x86-64 or arm64 Linux. On an Apple silicon Mac, run it in an arm64 container,
which cross-compiles x86-64:

```sh
docker run --rm -v "$PWD":/src:ro -w /src ubuntu:26.04 sh -euc 'apt-get update -qq; apt-get install -y -qq rustup gcc gcc-x86-64-linux-gnu python3 ca-certificates git > /dev/null; rustup target add x86_64-unknown-linux-gnu; python3 -B scripts/check_simd_inlining.py --build /tmp/simd'
```

The x86 AVX2 Matrix parser has unit tests against the portable parser: random,
all-`'z'` and one-invalid-byte strings at lengths 0–100 and around the 32-byte
block and head, and every byte value at every position of 31, 32, 33 and 64
characters (validity flag against the lookup table, groups when valid). They print
a skip line on CPUs without AVX2.

Wasm SIMD128 kernels build only with `+simd128` and have unit tests against the
portable kernels: exact NTT kernels and lane arithmetic, Matrix parsing (every byte
value at every block position) and panels, 64-byte column sums and whole encodes
(including split-carry fallbacks), and character packing of every column value.
They run under a WASI engine together with the rest of the library unit tests. The
`wasm_simd128` CI job runs them with wasmtime 50.0.1 and `__internals,cache`
features; default features were also run locally. WASI preview 1 cannot spawn threads or
processes, so those tests are skipped, and `should_panic` tests are ignored there
because Wasm panics abort. [CI.md](CI.md#test-jobs) gives the local commands.

## Fuzzing and sanitizers

The [two general differential targets](fuzz/README.md) compare all backend
wrappers and both allocation policies: values, capacities, suffixes, canaries, owned
helpers and malformed input. Decode also maps bytes to valid digits and inserts
invalid ASCII or multibyte characters, including late positions. The shared
oracle switches from bs58 to Malachite above 2,048 bytes, so large inputs do not
pay for a quadratic oracle; the quadratic `batched` encoder under test still
dominates their time. Named boundary replay covers larger recursive kernels.

`check_all` (feature `check`) compares Base58Check encoding and decoding with
bs58's checksum validation plus an independent version rule, with and without a
version, for valid, changed and arbitrary text. It checks exact/short/oversized
slices, tails and canaries. Payloads up to 8,193 bytes encode to about 11,200
characters, so decoding the encoded frame can reach every checked encode and decode
backend; changed frames and arbitrary text share that budget. CI runs it for default and cache policies in the manual fuzz dispatch.

General target budgets are 65,537 binary bytes and
`max_encoded_len(65_537) = 89,502` decode bytes. Lossy UTF-8 and inserted
characters can expand text. These budgets are not format limits or coverage of
every transform size. The 8,193-byte native mutation cap does not reach real NTT
thresholds on the IFMA host; large boundary replay, MSan and Miri do exercise NTT.

`encode_radix_full_width` uses an independent Malachite oracle on periodic
256 KiB / 1 MiB values, reaching owned large twiddles. Arbitrary large values
remain outside this structured domain.

The optional `encode_full_width` target expands every raw mutation into a
periodic 65,537-byte value with its first bit set. It checks encoding and
generated-text decoding against independent oracles. The pinned-source geometry
audit places those values above the native NTT thresholds. Its default-policy
native careful-ASan stage is **unverified**. It uses the Malachite oracle; the
quadratic `batched` encoder under test still takes most of each input's time.
This structured domain does not include every value at that width.

Campaign callback counts include initial corpus processing and possible repeats;
they are not distinct mutations or seed-file counts. Full-width stages require
a positive post-initialization mutation-attempt count from libFuzzer's corpus
statistics. Keep bounded mutation campaigns separate from large boundary replay.

The shared corpus on the `fuzz-corpus` branch is minimized by libFuzzer merge
in each of the eight replay builds without ASan, then combined
([corpus guide](docs/fuzzing-corpus.md)). It preserves each build's
edge-counter coverage of the inputs it saw, not each input. Value-profile
features, ASan-only behavior and inputs that reach the same coverage with
different values are not preserved; removed inputs remain only in Git history.

Passing Rust/std MSan tests does not close the full libFuzzer/MSan campaign gap.
No sanitizer runtime was patched to obtain the passes. TSan does not fully model
atomic fences or inline assembly; its results supplement the other checks.

## Miri

Supported tests use strict provenance; selected paths also run with Tree Borrows
and a different seed. They cover portable recursive/prepared/wrapped arithmetic,
fixed encoders, exact scratch, supported SSE4.1/AVX2/NEON panel operations and
AVX2 Matrix parsing.
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

In the custom-interpreter jobs, Tree Borrows covers full ARM/x86 suites under both cache
policies; Stacked Borrows covers kernels. Compact x86 kernels run under both
models and policies. All 14 buffer cases run separately; no ISA-skip output is
accepted. SIMD multiplication uses owned tables; native oracle and cache-history
tests supply broader shared-transform coverage. These are concrete executions,
not native x86 execution or complete-domain proofs.

## Kani proofs

The proofs use Kani 0.68.0 and CBMC 6.11.0. Repeated bodies are not
additional proofs. Memory, overflow and unwinding checks remain enabled; finite
loops have explicit bounds. Required harnesses use 60/180/360-second budgets.

| Obligation family | Proven domain or property |
| --- | --- |
| Tiny production conversion | Every one-byte encoder value and capacity 0–3; every two-byte input (including non-UTF-8) and capacity 0–3; alphabet and lookup round trips |
| Generic batched decoding | All `58^2` valid two-digit strings and capacities 0–3; numeric value, leading zeros, errors and output canaries |
| Parsing | All 256 lookup entries; validity through four bytes, zero split through eight, partial chunks through nine; full ten-byte numeric value and invalid rejection by verified composition |
| Limb arithmetic | Full-width two-limb carry/borrow; one-limb multiplication; single radix updates; three limbs folded into one modulo `2^64 - 1` |
| NTT scalar operations | Reduction for every `i32`, normalization in `(-q,q)`, loading for every `u32`, all three primes; plan bounds and scale at every power-of-two length 1 through `2^23`; both inverse butterfly kinds for every input in their ranges and every balanced twiddle (outputs below `1.25 q` and `1.9 q`) |
| Matrix / IFMA models | Two-column carries; eight-column radix-52 carry; 13 limbs ↔ 16 radix-52 digits through a scalar vector model |
| Capacity | Actual encoded-bound safety, coverage of input count, endpoint saturation and full adjacent monotonicity; owned decoded-bound arithmetic and safe prefix addition for every 64-bit `usize` |
| Paired update safety | Full-width stored limb and nonzero top, two spare limb slots, both digits below `58^10`: interleaved and sequential updates succeed; numeric equality remains open. The harnesses use one or two limbs, so the four-limb loop body is covered only by tests (lengths 0–9 against sequential passes) |

The parser composition checks the unchanged production parser. Its digit-range
and split/Horner contracts are separately proved. Adjacent capacity monotonicity
uses separately proved affine, clamp and actual-function product contracts.
The logarithmic argument connecting either capacity bound to actual conversion
length remains outside Kani's scope. Scalar models do not prove emitted SIMD,
arbitrary-length conversions or concurrent-cache correctness. The inverse stage
schedule that chains the proved butterflies is checked by tests and by debug
assertions in the portable kernel, not by Kani.

Wider paired numeric equality, encoder split/step and fixed carries, actual
Barrett division and the public borrowed-array wrapper remain unresolved.
Timed-out or untranslated queries are neither counterexamples nor proofs.
[The open runner](scripts/kani-open.sh) selects eight distinct unresolved queries.
Direct parser/capacity formulations also remain in source; their properties are
closed by verified composition.

## Reproduce and update checks

[CI.md](CI.md) lists every CI job and repository script with its trigger,
platforms, prerequisites, host limits and local commands.

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
3. Complete the default full-width native ASan stages. Repair or
   replace the libFuzzer/MSan integration before claiming MSan fuzzing.
   Continue longer campaigns and inspect coverage gaps; all completed campaigns
   are finite.
4. Run the native AVX-512 kernel tests on AVX-512 hardware in every CI run.
   `mul_ifma::tests::avx512_matches_reference` needs AVX-512F and IFMA;
   `ntt::avx512::tests::avx512_matches_portable` needs AVX-512F. Without the
   feature, each prints `skipped:` and passes; with `BASE58_REQUIRE_IFMA` set, each
   fails instead. Both compile only for x86-64, so CI can run them only in the
   x86-64 rows of `tests`, `compact` and `alternate_limb_width`; the
   `native_sanitizers` memory rows also run `avx512_matches_reference`. No CI job
   sets the variable: `native_sanitizers` does not pass `--require-ifma`, and the
   custom Miri runner sets it but `ci/miri/coverage.json` selects neither test.
   The kernels therefore execute in CI only when a hosted runner has the features,
   and a skip passes. The IFMA-only dispatch arms also run only on such a runner.
5. Repeat deployment-specific performance, peak memory and linked-size checks
   using [the measurement guide](docs/performance.md).
6. Resolve any confirmed source-adaptation notice obligations described in
   [REFERENCES.md](REFERENCES.md), and make an explicit release decision after CI.

A passing job establishes only the checks and platforms it actually executes.

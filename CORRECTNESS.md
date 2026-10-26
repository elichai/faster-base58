# Correctness and verification

Verification combines independent conversion oracles, property tests, arithmetic
tests, native sanitizers, Miri and scoped Kani proofs. Each checks different
properties. Passing these checks does not prove arbitrary-length conversion or
the semantics of every SIMD instruction. The implementation map is in
[DESIGN.md](DESIGN.md); repeatable jobs are in
[the workflow](.github/workflows/verification.yml).

[Local verification of the normalized dispatcher](docs/verification/local-ci.md)
records the full matrix, outlined-source checks and later portable-target
correction, with exact input hashes, platform adaptations and remaining gaps.

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

CI runs 512 cases per property in debug and release, cache off/on, across Linux,
macOS and Windows. Additional jobs cover 32-bit Linux, Malachite's 32-bit limb
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
Decode also maps bytes to valid digits and injects invalid ASCII or multibyte
characters. The selector consumes the full input so late insertion positions
remain reachable. Smaller mutation corpora keep the quadratic oracles useful;
separate boundary replays exercise larger recursive kernels.

The raw budgets are 65,537 binary bytes and `max_encoded_len(65_537) = 89,502`
decode bytes. They cross 8192/8193-limb boundaries. Lossy UTF-8 and inserted
characters can expand the decoded text. These are campaign budgets, not format
limits; they do not exercise every transform size.

The historical 2026-10-26 careful AddressSanitizer campaign completed **450,058 executions in mutation campaigns**
across default/cache encode/decode, plus full and supplemental boundary replays.
Linux LeakSanitizer completed another **249,334 executions**. Both retain debug
assertions and overflow checks. ThreadSanitizer passed five nonzero concurrency
test invocations on each of Apple Silicon and Linux x86-64. Native Rust
MemorySanitizer passed six guarded Matrix/NTT/IFMA kernel commands and the public
API command on the AMD EPYC 9B45 VM, with rebuilt standard library/dependencies
and no suppressions. Source identities and tool versions are retained in the
[sanitizer receipt](docs/verification/sanitizers.json).

The libFuzzer/MSan configuration failed in the fuzzer's file-discovery startup
before the Base58 callback. It is not a successful fuzz campaign or an identified
crate defect. Native Rust MSan avoids that C++ engine; it does not close that
tooling gap. No sanitizer runtime was patched to obtain the passing results.
ThreadSanitizer does not fully model synchronization through atomic fences or
inline assembly; its results supplement the other checks.

## Miri

Supported tests use strict provenance; selected paths also run with Tree Borrows
and a different seed. They cover portable recursive/prepared/wrapped arithmetic,
fixed encoders, exact scratch and supported SSE4.1/AVX2/NEON panel operations.
Miri executes MIR: it does not verify LLVM auto-vectorization or emitted native
instructions. Native sanitizer and oracle tests remain necessary.

Eleven LLVM intrinsics still lack Miri shims in the pinned 2026-10-26 toolchain:

| Target | Missing intrinsic |
| --- | --- |
| AArch64 | `llvm.aarch64.neon.srshl.v4i32` |
| AArch64 | `llvm.aarch64.neon.vsli.v4i32` |
| AArch64 | `llvm.aarch64.neon.vsli.v8i16` |
| AArch64 | `llvm.aarch64.neon.tbx1.v16i8` |
| AArch64 | `llvm.aarch64.neon.tbx4.v16i8` |
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

## Kani proofs

The selected suite passes **40 distinct harnesses** with Kani 0.68.0 / CBMC 6.11.0,
its bundled compiler and 64-bit
`usize`. Memory, overflow and unwinding checks remain enabled. The default unwind
is one; finite loops need explicit bounds. Scalar proof models do not establish
SIMD instruction semantics or concurrent-cache correctness.

| Obligation family | Proven domain or property |
| --- | --- |
| Tiny production conversion | Every one-byte encoder value and capacity 0–3; valid UTF-8 decoder text of at most two bytes and capacities 0–3; alphabet and lookup round trips |
| Parsing | All 256 lookup entries; validity through four bytes, leading-zero split through eight, partial chunks through nine; invalid full-chunk rejection |
| Limb arithmetic | Full-width two-limb carry/borrow; one-limb multiplication; single radix updates; three limbs folded into one modulo `2^64 - 1`; exact Toom division by three |
| NTT scalar operations | Reduction for every `i32`, normalization for residues in `(-q,q)`, loading for every `u32`; all three primes; transform-plan bounds; inverse scale at every power-of-two length 1 through `2^23` |
| Matrix / IFMA models | Two-column carry bounds; eight-column radix-52 carry; 13 limbs ↔ 16 radix-52 digits via a scalar vector model |
| Capacity | Actual function panic/overflow safety, `max_encoded_len(n) >= n`, saturation endpoint; three supporting formula/step lemmas |

The inverse-scale tests additionally check all 72 prime/length combinations and
3528 Montgomery edge compositions against independent wide-integer expressions.
The capacity proofs do not establish that the bound covers every encoded output;
the logarithmic output-length argument is outside their Kani scope.
[The production proof script](scripts/kani-production.sh) and the workflow specify
exact harness selections. Proof receipts are in
[docs/verification](docs/verification/README.md).

Proof work uses decomposition, tighter invariants, verified contracts and solver
selection without shrinking the stated input domains or disabling safety checks.
Splitting valid/invalid parsing discharged invalid rejection; splitting capacity
conjuncts discharged coverage and saturation. Full adjacent monotonicity,
the all-valid ten-digit numeric pipeline, wider batched/paired updates and
Barrett split obligations remain open. Some runs timed out; others encountered
solver translation errors. Neither outcome is a counterexample or a proof.
[The open runner](scripts/kani-open.sh) preserves those obligations separately.

## Remaining work

1. Add the eleven upstream Miri shims, then run the currently unsupported SIMD
   paths with strict provenance and both alias models.
2. Discharge the wider Kani obligations through further decomposition. There is
   no arbitrary-length end-to-end proof of each backend.
3. Repair or replace the libFuzzer/MSan integration before claiming MSan fuzzing.
   Extend sustained fuzz campaigns and assess coverage; smoke runs are finite.
4. Repeat deployment-specific performance, peak memory and linked-size checks.
   Historical benchmark source generations are labelled in
   [the data index](docs/benchmarks/README.md); they are not fresh release numbers.
5. Resolve any confirmed source-adaptation notice obligations described in
   [REFERENCES.md](REFERENCES.md), and make an explicit release decision after CI.

The repository has not been published to crates.io. Hosted CI status is available
on the tested commit. A green workflow establishes only the checks and platforms
it actually executes; it does not imply the open obligations above are complete.

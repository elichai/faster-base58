# Design and resource costs

This crate converts the Bitcoin Base58 alphabet. The root API selects kernels by
size and CPU; `no_alloc` provides conversion without internal heap allocation.
Generated-table retention is an opt-in `cache` feature, off by default.
The library requires `std` and has no normal dependencies.

This map complements [the user guide](README.md), [historical derivations](docs/historical-readme.md) and
[the tradeoff evidence](docs/tradeoffs/README.md). The
[public API report](docs/tradeoffs/2026-10-25/public-api/README.md) records current feature choices;
the earlier [resource/dispatch follow-up](docs/tradeoffs/2026-10-24/resource-dispatch/) preserves their precursors.

## Contract and representation

Leading zero bytes become leading `'1'` characters and are preserved on decode.
Decoders accept `&str`; invalid-character positions are UTF-8 **byte** offsets.
An exactly sized output buffer suffices. Successful calls leave its remaining
tail untouched. Error-buffer contents and error precedence when both invalid
input and insufficient capacity apply are unspecified.

`encode_into` / `decode_into` borrow caller storage; `encode_string` /
`decode_vec` allocate the result. `decode_array::<N>` requires exactly `N` decoded
bytes. Auto can allocate internal workspace despite caller-supplied storage.
`no_alloc` has the length/borrowed/array helpers, without owned-result helpers;
its cold and warm calls allocate no internal heap, including with `cache` enabled.
Allocation failure is not represented by these conversion error types.
All algorithms have input-dependent timing; they do not hide values or lengths.

Internal integers use little-endian `u64` limbs; ten text digits fit in one limb.
Powers omit low zero limbs using `58^k = 2^k * 29^k`, shortening products.
Matrix and transform kernels use narrower coefficients with bounded accumulation.

## Entry points and dispatch

Named backend selectors require `__internals`; they are hidden, unstable test and
measurement hooks. Their kernels also serve normal automatic dispatch. The hooks
select wrappers, whose small/large fallbacks contribute to their linked cost.
`__internals` does not enable `cache`; all library `OnceLock` caches require it.

| Family | Main use | Boundary behavior |
| --- | --- | --- |
| Batched | Small inputs and compact consumers | Quadratic at large sizes; no internal heap allocation |
| Karatsuba | General divide-and-conquer conversion | Batched leaves; shared schoolbook/IFMA multiplication below its recursive threshold |
| Toom-3 | Faster large products than Karatsuba on some hosts | Same conversion tree; falls back to Karatsuba for smaller products |
| NTT | Large inputs | Karatsuba/IFMA below transform thresholds; owned or cached twiddles |
| Matrix | Fixed and medium inputs | Batched below its minimum; encoder falls back to NTT, decoder to Karatsuba above its cap |
| Portable variants | Kernel comparisons and differential controls | Plain-Rust matrix/NTT kernels; wrapper fallbacks still apply |
| Malachite, dev only | Multiplication comparison | Shared conversion tree; operand copies into reused `Natural` staging, with product and internal scratch allocations |

Without `cache`, Auto encoding uses batched conversion below 22 input bytes and
fixed matrices at exactly 32/64 bytes. Other sizes through 4096 bytes use the
Karatsuba encoder (including its batched fallback); larger sizes use owned NTT.
Decoding uses the tiny helper through two text bytes, batched through 5120,
Karatsuba through 5632, then owned NTT. Recursive leaves use Horner conversion.

With `cache`, encoding instead uses general matrix conversion through 8192
significant bytes, reduced to 3584 with IFMA, then NTT; tiny/fixed routes remain.
Decoding starts with batched conversion below 576 text bytes on little-endian
AArch64 with NEON or Wasm, or 1024 elsewhere. Medium inputs use matrix on those
targets and x86 with SSE4.1, then NTT above 5632 text bytes.
IFMA hosts switch above 5120; IFMA hosts compiled with AVX2 keep the remaining
medium range on batched decoding. Other targets keep batched decoding there.
The matrix wrapper itself applies its limits after stripping leading `'1'`s.
These conservative crossovers are not format limits or optimality guarantees.
`no_alloc` encoding uses the fixed 32/64-byte matrices and batched conversion
elsewhere; decoding uses tiny/batched conversion. It never builds general tables
or recursive workspace. Its general-size work remains quadratic.

## Conversion families

**Batched Horner conversion** updates a whole limb for each
[ten text digits](src/batched.rs) or [seven binary bytes](src/encode/batched.rs).
Two updates overlap carry chains. The encoder uses reciprocal splitting by
`58^10` and stores limbs inside the eventual output; the decoder keeps its top
limb separate for exact buffers. Work is quadratic, with no heap workspace.
Shared batched encoding bypasses Horner for empty/one-byte inputs. Auto and
`no_alloc` decoding use the internal at-most-two-byte text helper; named backend
hooks keep the Batched core. Earlier tiny/medium dispatch experiments and their
costs remain in the [historical report](docs/tradeoffs/2026-10-24/resource-dispatch/performance.md).

**Divide and conquer** uses balanced subproblems. [Decoding](src/dc.rs) combines
`high * (58^10)^e + low`; [encoding](src/encode/dc.rs) divides by those powers,
then recursively emits the quotient and padded remainder. Barrett estimates use
reciprocals refined by Newton steps; powers and reciprocals are reused within one call.
Karatsuba, Toom-3 and NTT share this tree. See [radix-conversion background](https://gmplib.org/manual/Binary-to-Radix).
The [Malachite adapter](tests/support/malachite.rs) also shares it; the independent
`Natural` radix oracle uses a separate conversion implementation.

**Matrix conversion** replaces a long carry chain with independent multiply-adds
against powers of the source radix. [Encoding](src/encode/matrix.rs) maps 32-bit
words into base `58^4`; [decoding](src/matrix/mod.rs) maps four-character groups
into base `2^32`. Tables store eight-row, four-column panels suited to the same
SIMD multiply-add kernels. Scheduled carry
passes keep sums within `u64`; checked constant bounds constrain that schedule.
Tables cover bounded size classes because storage and work grow quadratically.

Fixed 32/64-byte encoders use compile-time tables, local sums and no general-table
setup. [Firedancer's Rust port, five8](https://docs.rs/five8/1.0.0/five8/) uses a related approach;
here columns use `58^4`. Optimized 64-byte kernels split carry into segments;
correction/fallback preserves exact results at carry boundaries.

## Multiplication and CPU-specific work

[Schoolbook multiplication](src/mul.rs) overlaps two rows' carry chains.
Karatsuba replaces four half-size products with three; [Toom-3](src/toom3.rs)
replaces nine third-size products with five and exact small divisions. Unbalanced
operands are cut into blocks. Both reuse scratch rather than allocate recursively.
Background: [Karatsuba](https://gmplib.org/manual/Karatsuba-Multiplication) and
[Toom-3](https://gmplib.org/manual/Toom-3_002dWay-Multiplication) in the GMP manual.

[NTT multiplication](src/ntt/mod.rs) splits each limb into two 32-bit coefficients, transforms
modulo three primes below `2^30`, and reconstructs exact coefficients with CRT
before carrying into limbs. Montgomery products and bounded lazy reductions
reduce division/reduction overhead. Forward/inverse orderings avoid a separate
bit-reversal permutation. Squares and prepared radix-power transforms save
forward transforms. Cyclic/negacyclic pieces reduce padding near transform-size
boundaries; wrapped products also shorten some reciprocal/division work.
The largest transform has `2^23` points. Larger products split; this is not a
maximum accepted input length. The coefficient/CRT bounds depend on this cap.

| Target | Hand-written or selected work |
| --- | --- |
| Little-endian AArch64 + NEON | Matrix parsing/panels and digit packing; fixed encoders; NTT butterflies, pointwise products and CRT |
| x86-64 | Runtime AVX2/SSE4.1 matrix panels; AVX2 or AVX-512 VBMI fixed 64-byte encoding; AVX-512 NTT plus target-feature plain-Rust kernels |
| x86-64 + AVX-512 IFMA | Radix-`2^52` schoolbook leaves with exact bounded column sums; changes whole-conversion crossover thresholds |
| Other targets, including default Wasm | Plain-Rust kernels and shared fallbacks |

Runtime feature checks guard x86 instructions. Compile-time target features also
affect dispatch and LLVM optimization, so a generic x86 build may still select
SIMD at runtime. The recorded native flag sets are not interchangeable controls.
[Hoisting the relaxed atomic IFMA threshold lookup](docs/review/threshold-snapshot.md) once per conversion was tested and rejected;
the existing lookup remains, and emitted loads depend on compiler folding.

`*_portable` selectors require `__internals` and are hidden from API docs. They
measure hand-written kernels against plain Rust and provide differential controls.
The kernels remain available internally for normal target fallbacks. This does
**not** promise scalar machine code:
[LLVM can vectorize loops and scalar operations](https://llvm.org/docs/Vectorizers.html).
The selector replaces its named kernel family. Portable NTT keeps shared wrapper
leaves and multiplication fallbacks, which can use hand-written SIMD.
Portable matrix can fall back to normal NTT encoding or Karatsuba decoding.
They are unstable test/measurement hooks, outside the normal public API.

## Heap, stack and cache costs

Batched calls use only caller output and small local state. Fixed encoding uses
small local arrays and constant tables: their two reference layouts total 2208
bytes on 64-bit targets, not necessarily 2208 emitted binary bytes.
General matrix calls use roughly 30 KiB of encoder stack or 14 KiB of decoder
stack; recursive encoder matrix leaves add 8 KiB of bytes, about 38 KiB total.
Only used prefixes are initialized; encoder scratch is 64-byte aligned. Out-of-line
paths isolate these frames. These are source estimates, not stack high-water tests.
`decode_array::<N>` also constructs an `N`-byte local array.

Without `cache`, explicit general-matrix hooks own one table per invocation;
recursive matrix leaves do not share those tables. Auto avoids these paths.
With `cache`, first use builds shared `OnceLock` tables;
warm matrix calls need no table allocation. Initialized classes remain for the
process lifetime; larger tables can serve smaller calls, so history matters.
Known coefficient counts are reserved once, avoiding geometric spare capacity.
The historical v011 64-bit capacity snapshot has largest tables of 11,510,016
encoder bytes / 2,437,248 decoder bytes including panel metadata. Class capacities
sum to 15,364,736 / 3,253,024 bytes, not a simultaneous active-peak measurement.
Matrix entry paths reject provably insufficient output before table setup using
conservative lower bounds after leading-zero handling. They do not require a
worst-case output capacity; an exact buffer still succeeds. See the
[resource-hardening evidence](docs/tradeoffs/2026-10-24/resource-dispatch/resource-hardening.md).

Recursive conversion allocates powers, reciprocal/prepared-transform data and
workspace. The decoder packs its main result/power/scratch regions into one
allocation; encoding needs several owned buffers. Karatsuba scratch is bounded
by `5*n + 64` limbs, NTT by `16*n + 64`, for the operand bound used by those
interfaces. These are scratch formulas, not total conversion-memory bounds.

An NTT log-`l` table has `48 * 2^l` twiddle bytes across three primes with four arrays each.
Without `cache`, all generated twiddle tables are owned. With `cache`, the log-18
policy retains at most 25,164,288 bytes across production logs 5–18 (~24 MiB);
larger tables stay owned. The arithmetic log-23 limit is unchanged. This bounds
opt-in idle twiddles, not total active memory. Both feature choices can replicate
large owned twiddles under concurrency: 4 MiB of significant encoder input can
need 96 MiB per call, where the older unbounded global cache could share one.
Workspace, prepared powers and matrix caches add further costs. See the
[historical retention trials](docs/tradeoffs/2026-10-24/ntt-retention-policy.md).

In the current M5 raw Auto calls, cached 1 KiB encoding retains 185,856 table bytes.
Default 4 MiB encoding retains zero but reaches 284,269,528 extra requested live
bytes during the call. The [current resource data](docs/tradeoffs/2026-10-25/public-api/README.md)
exclude allocator overhead/arenas, RSS and stack. Wasm page high-water includes
stack/static reservation, caller buffers, allocator state and tables: a different metric.

## Linked size and selecting a smaller consumer

Compare linked executables/modules with the same compiler, source, flags,
direction and scaffold. `.rlib` size is not deployment cost. Fat LTO and dead
stripping already remove unreachable implementations; deleting an unused public
function does not recover its source/archive size from a consumer.

Current matched encode+decode consumers, source snapshot `90e71a1a…`:

| Entry | M5 executable | GCP generic | GCP native | Wasm module | Wasm gzip |
| --- | ---: | ---: | ---: | ---: | ---: |
| bs58 0.5.1 | 302,768 | 291,160 | 292,504 | 20,058 | 8,073 |
| no_alloc | 302,864 | 325,032 | 315,112 | 40,091 | 19,525 |
| Auto, default | 369,024 | 616,440 | 596,736 | 105,752 | 45,208 |
| Auto + cache | 385,584 | 613,968 | 585,104 | 108,412 | 46,370 |

Bytes, stripped, fat LTO. M5/Wasm use Rust 1.99.1; GCP 1.99.0, all LLVM 23.1.1.
CPU flags are M5 `apple-m4`, GCP `x86-64` / `native`, Wasm default features.
The [full comparison](docs/tradeoffs/2026-10-25/public-api/README.md) has both feature
scaffolds, sections and exact bs58 ratios. M5 `no_alloc` adds 112 file bytes but
13,033 initialized-section bytes. Cache-off is not always smaller: its GCP native
Auto file exceeds cache-on. Cross-host compiler, ABI and link formats differ.
The earlier [v011 backend comparison](docs/tradeoffs/2026-10-24/resource-dispatch/linked-size-final.md)
is historical; its Batched probe is not today's `no_alloc` with fixed encoders.
five8 supports only 32/64-byte values, unlike dynamic-length consumers.

## Evidence and remaining limits

Use the earlier [29-size baseline grids](docs/tradeoffs/2026-10-24/benchmark-results/README.md)
for warmed throughput and support limits before these resource changes. Matrix/NTT wrapper wins at large sizes
often measure the same fallback. Cold setup, retained bytes and binary size need
their own comparisons. Cache-only claims require matched leaves and thresholds;
internal owned/cached NTT selectors can also differ in leaf policy.

[Verification](docs/review/verification.md) combines differential/property tests,
independent large radix conversion, arithmetic references, sanitizer fuzzing,
supported Miri paths and scoped Kani obligations. It is not a proof of complete
backends or SIMD instruction semantics. Open solver obligations and missing Miri
intrinsics remain; short fuzz campaigns do not establish exhaustive coverage.
The maximum-transform test is separate from ordinary tests. Public API examples
and downstream compiler fixtures are tested. Hosted CI, application-specific
performance and release validation remain distinct completion gates.

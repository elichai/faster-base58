# Design and resource costs

This crate converts the Bitcoin Base58 alphabet through several caller-buffer
backends. Auto favors throughput on the measured M5 and Zen 5 systems; explicit
backends expose size, setup and memory tradeoffs. None wins every workload.
The library requires `std` and has no external dependencies.

This map complements [the derivations](README.md) and
[the tradeoff evidence](docs/tradeoffs/README.md). The
[resource/dispatch follow-up](docs/tradeoffs/2026-10-24/resource-dispatch/) records current policy decisions.

## Contract and representation

Leading zero bytes become leading `'1'` characters and are preserved on decode.
Decoders accept `&str`; invalid-character positions are UTF-8 **byte** offsets.
An exactly sized output buffer suffices. Successful calls leave its remaining
tail untouched. Error-buffer contents and error precedence when both invalid
input and insufficient capacity apply are unspecified.

`encode_into` / `decode_into` borrow caller storage; `encode_string` /
`decode_vec` allocate the result. `decode_array::<N>` requires exactly `N` decoded
bytes. Supplying output storage does not prevent internal workspace allocation.
Allocation failure is not represented by these conversion error types.
All algorithms have input-dependent timing; they do not hide values or lengths.

Internal integers use little-endian `u64` limbs; ten text digits fit in one limb.
Powers omit low zero limbs using `58^k = 2^k * 29^k`, shortening products.
Matrix and transform kernels use narrower coefficients with bounded accumulation.

## Entry points and dispatch

Public backend names select wrappers, not an unconditional arithmetic kernel.
Their small-input and large-input fallbacks are part of their linked cost.

| Family | Main use | Boundary behavior |
| --- | --- | --- |
| Batched | Small inputs and compact consumers | Quadratic at large sizes; no internal heap allocation |
| Karatsuba | General divide-and-conquer conversion | Batched leaves; shared schoolbook/IFMA multiplication below its recursive threshold |
| Toom-3 | Faster large products than Karatsuba on some hosts | Same conversion tree; falls back to Karatsuba for smaller products |
| NTT | Large inputs | Karatsuba/IFMA below transform thresholds; owned or cached twiddles |
| Matrix | Fixed and medium inputs | Batched below its minimum; encoder falls back to cached NTT, decoder to Karatsuba above its cap |
| Portable variants | Kernel comparisons and differential controls | Plain-Rust matrix/NTT kernels; wrapper fallbacks still apply |
| Malachite, dev only | Multiplication comparison | Shared conversion tree; operand copies into reused `Natural` staging, with product and internal scratch allocations |

Automatic encoding uses batched conversion below 22 input bytes. Exactly 32/64
bytes use fixed matrix encoders. General matrix conversion handles up to 8192
significant bytes, reduced to 3584 when the IFMA leaf is available; larger inputs
use cached NTT. Leading zeros can move a general call to a smaller route.

Automatic decoding starts with batched conversion: below 576 text bytes on
little-endian AArch64 with NEON or Wasm, or 1024 elsewhere. Medium inputs use
matrix on those targets and x86 with SSE4.1, then cached NTT above 5632 text bytes.
IFMA hosts switch above 5120; IFMA hosts compiled with AVX2 keep the remaining
medium range on batched decoding. Other targets keep batched decoding there.
The matrix wrapper itself applies its limits after stripping leading `'1'`s.
Thresholds are tuning choices, not format limits or universal CPU guarantees.
Earlier v007 Wasm dispatch trials cut M5/V8 warm elapsed time 47–62%; first calls take 4–8× baseline time.
That v007 change adds cached matrix heap and 4579 Wasm bytes; reservations reduce cold setup ([evidence](docs/tradeoffs/2026-10-24/resource-dispatch/performance.md)).

## Conversion families

**Batched Horner conversion** updates a whole limb for each
[ten text digits](src/batched.rs) or [seven binary bytes](src/encode/batched.rs).
Two updates overlap carry chains. The encoder uses reciprocal splitting by
`58^10` and stores limbs inside the eventual output; the decoder keeps its top
limb separate for exact buffers. Work is quadratic, with no heap workspace.
Shared batched encoding bypasses Horner for empty/one-byte inputs. Only Auto
decoding uses the internal at-most-two-byte text helper; explicit backends keep
the Batched core. For empty/one-byte payloads, v011/v008 Auto decode time falls
11–19% on native CPU and 33–47% in V8 warm elapsed; GCP native 22-byte CPU time falls 18.1%. M5 Auto 32-byte CPU costs 3.54%
(~0.59 ns between side medians); explicit backends restore their earlier tiny costs.

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

`*_portable` exists to measure hand-written kernels against plain Rust and to
provide differential controls. It does **not** promise scalar machine code:
[LLVM can vectorize loops and scalar operations](https://llvm.org/docs/Vectorizers.html).
The selector replaces its named kernel family. Portable NTT keeps cached-backend
leaves and shared multiplication fallbacks, which can use hand-written SIMD.
Portable matrix can fall back to normal cached NTT encoding or Karatsuba decoding.
These public selectors are retained while API/release work is deferred.

## Heap, stack and cache costs

Batched calls use only caller output and small local state. Fixed encoding uses
small local arrays and constant tables: their two reference layouts total 2208
bytes on 64-bit targets, not necessarily 2208 emitted binary bytes.
General matrix calls use roughly 30 KiB of encoder stack or 14 KiB of decoder
stack; recursive encoder matrix leaves add 8 KiB of bytes, about 38 KiB total.
Only used prefixes are initialized; encoder scratch is 64-byte aligned. Out-of-line
paths isolate these frames. These are source estimates, not stack high-water tests.

Warm general matrix calls allocate no heap. First use builds a shared `OnceLock`
table; initialized size classes remain for the process lifetime. Larger tables
can serve smaller calls, so retained memory depends on call history. Known final
coefficient counts are reserved once, avoiding geometric spare capacity. Largest
classes retain 11,510,016 encoder bytes or 2,437,248 decoder bytes including panel
metadata on 64-bit targets. Individual class capacities sum to 15,364,736 /
3,253,024 bytes; this is not a simultaneous active-peak measurement.
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
Tables are lazy. The selected log-18 policy retains at most 25,164,288 bytes
across production logs 5–18 (~24 MiB); cap 16 would retain ~6 MiB.
In cache-only trials, cap 16 costs 4–5% encode CPU at 256 KiB and 10–11% decode at 1 MiB on GCP,
where cap 18 is near baseline. Above-cap calls have observed 4–9% warm CPU costs
at selected large GCP cells and twelve extra allocations. Larger tables belong to each
conversion; the arithmetic log-23 limit is unchanged. Idle retention is bounded,
but active memory can grow with concurrency: each 4 MiB encoder owns 96 MiB of
twiddles where the old cache could share one. Workspace, prepared powers and
matrix caches add further costs. [Policy](docs/tradeoffs/2026-10-24/ntt-retention-policy.md), [measurements](docs/tradeoffs/2026-10-24/resource-dispatch/performance.md).

The [resource measurements](docs/tradeoffs/2026-10-24/resource-dispatch/performance.md) record requested Rust allocation
layouts and actual vector capacities. It excludes allocator overhead/arenas,
RSS and stack. Wasm linear-memory high-water includes stack/static reservation,
caller buffers, allocator state and tables; it is a different metric.

## Linked size and selecting a smaller consumer

Compare linked executables/modules with the same compiler, source, flags,
direction and scaffold. `.rlib` size is not deployment cost. Fat LTO and dead
stripping already remove unreachable implementations; deleting an unused public
function does not recover its source/archive size from a consumer.

Matched encode+decode sizes for final source `v011`:

| Entry | M5 executable | GCP generic | GCP native | Wasm module | Wasm gzip |
| --- | ---: | ---: | ---: | ---: | ---: |
| Empty scaffold | 302,752 | 288,280 | 289,032 | 16,452 | 6,480 |
| bs58 0.5.1 | 302,768 | 291,176 | 292,520 | 20,058 | 8,073 |
| Batched | 302,864 | 296,784 | 298,912 | 26,345 | 10,576 |
| Karatsuba | 335,936 | 493,944 | 503,808 | 65,093 | 29,548 |
| Cached NTT | 369,040 | 587,992 | 575,488 | 95,218 | 41,158 |
| Auto | 385,568 | 614,688 | 585,680 | 108,552 | 46,254 |

Bytes, stripped, fat LTO. M5/Wasm use Rust 1.99.1; GCP 1.99.0, all LLVM 23.1.1.
CPU flags are M5 `apple-m4`, GCP `x86-64` / `native`, Wasm default features.
Within each configuration, source/compiler/directions/scaffold match.
The [final comparison](docs/tradeoffs/2026-10-24/resource-dispatch/linked-size-final.md)
has all 13 entries, empty-scaffold deltas, sections and exact bs58 ratios.
Native padding/runtime data make sections essential: on M5, bs58 adds 16 file bytes but
1814 code/read-only/data bytes. Cross-host compiler, ABI and link formats differ.

An explicit batched consumer is small but quadratic. Explicit Karatsuba cuts
Wasm size substantially and is much slower at multi-MiB sizes. Automatic helpers
still select auto; calling them can bring its reachable paths into a compact
link. Malachite is development support, not a normal dependency: its reachable
adapter costs about 1.1 MiB of Wasm in this snapshot. five8 only supports
32/64-byte values and is not a dynamic-length replacement.

## Evidence and remaining limits

Use the earlier [29-size baseline grids](docs/tradeoffs/2026-10-24/benchmark-results/README.md)
for warmed throughput and support limits before these resource changes. Matrix/NTT wrapper wins at large sizes
often measure the same fallback. Cold setup, retained bytes and binary size need
their own comparisons. Cache-only claims require matched leaves and thresholds;
public owned/cached NTT selectors can also differ in leaf policy.

[Verification](docs/review/verification.md) combines differential/property tests,
independent large radix conversion, arithmetic references, sanitizer fuzzing,
supported Miri paths and scoped Kani obligations. It is not a proof of complete
backends or SIMD instruction semantics. Open solver obligations and missing Miri
intrinsics remain; short fuzz campaigns do not establish exhaustive coverage.
The maximum-transform test is separate from ordinary tests. Hosted CI and
downstream ergonomic-API/release validation remain distinct completion gates.

# Algorithms and resource costs

The public API has two modes: automatic selection and `no_alloc`. Named backends,
portable controls and tuning hooks require `__internals` and are unstable.
The library uses `core` and `alloc` by default. `cache` enables `std` and retains
generated tables. See [correctness and verification](CORRECTNESS.md),
[measurements](docs/performance.md), and [implementation references](REFERENCES.md).

## Conversion families

| Family | Method and role | Main cost |
| --- | --- | --- |
| Batched | Horner conversion, seven binary bytes or ten Base58 digits per step; paired updates overlap carry chains | Quadratic work, no heap allocation |
| Karatsuba | Balanced radix conversion with Karatsuba multiplication and batched leaves | Per-call powers, reciprocals and scratch |
| Matrix | Independent multiply-adds against a power table, followed by bounded carry passes | Quadratic table size and work; setup can dominate small calls |
| NTT | Balanced radix conversion with transform multiplication and prepared powers | Subquadratic large-input work, substantial scratch and twiddle storage |
| Toom-3 | Experimental multiplier using five third-size products | Test/benchmark/proof builds only; Auto never selects it |
| Malachite | Development-only multiplier adapter and separate radix oracle | Not a normal dependency; its multiplier shares our conversion tree |

Integers use little-endian `u64` limbs. A radix chunk is `58^10`. Powers omit low
zero limbs using `58^k = 2^k * 29^k`, reducing multiplication work.

[Batched decoding](src/batched.rs) keeps its top limb separate so an exactly
sized output buffer suffices. [Encoding](src/encode/batched.rs) stores temporary
limbs inside the eventual output and uses reciprocal splitting by `58^10`.
The public paths handle empty and tiny inputs before the general loops.

[Divide-and-conquer decoding](src/dc.rs) combines `high * (58^10)^e + low`.
[Encoding](src/encode/dc.rs) divides by the same powers, then emits quotient and
padded remainder recursively. Barrett estimates use reciprocals refined by Newton
steps; each power and reciprocal is reused within one conversion. Karatsuba,
Toom-3 and NTT share this conversion tree. Unbalanced products use blocks.

[Matrix encoding](src/encode/matrix.rs) maps 32-bit words to base `58^4`;
[decoding](src/matrix/mod.rs) maps four-character groups to base `2^32`.
Eight-row, four-column panels share SIMD multiply-add kernels. Scheduled carry
passes and compile-time bounds keep accumulations within `u64`.

Fixed 32/64-byte encoders use the same matrix idea with compile-time tables and
local sums. Fixed dimensions remove table construction and dimension handling;
64-byte kernels also segment the carry chain. Auto and `no_alloc` select these
specializations directly. The internal Matrix comparison hooks use the general
matrix kernel. Their short carry schedule needs
fixed-size bounds and cannot replace the general schedule directly. Padding
nearby lengths into these kernels was [rejected](docs/benchmarks/decisions/README.md):
zero-heavy inputs regressed despite some ordinary-input gains.
Portable fixed decoding was also [measured and rejected](docs/benchmarks/fixed-decode/README.md):
the general decoder was faster on M5, and the prototypes added code and tables.

[NTT multiplication](src/ntt/mod.rs) uses two 32-bit coefficients per limb,
three primes below `2^30`, Montgomery products and exact CRT reconstruction.
Forward/inverse orderings avoid a separate bit-reversal pass. Squares and
prepared radix powers save transforms; cyclic/negacyclic pieces reduce padding.
Wrapped products shorten some reciprocal work. The `2^23`-point transform cap
supports the coefficient bounds; larger products split. It is not an input limit.

## Automatic selection

These are tuned crossovers, not format limits or guarantees of optimality for
every input. After the tiny and fixed routes, Auto strips leading zeros once and
selects by significant length in [`src/api.rs`](src/api.rs). The selected kernels
accept that normalized input and do not dispatch to another backend. Their size
preconditions are checked; small inputs never enter the large stack frames or
construct multipliers. Internal comparison/tuning wrappers still accept every
length so differential tests can exercise their fallbacks.

Normalized selection changes some zero-prefixed inputs' backend and allocation
behavior. With `cache`, a Matrix table can now be retained where the earlier
route used batched conversion. The [dispatch report](docs/benchmarks/dispatch/README.md)
records that cost, timing variation and the NTT outlining needed to keep large
stack frames out of tiny encode entries.

| Mode | Encode | Decode |
| --- | --- | --- |
| Auto, default | Fixed matrices at exactly 32/64 bytes; otherwise batched through 4096 significant bytes, with IFMA Karatsuba at 513–4096; then owned NTT | Tiny through two text bytes; batched through 5120 significant digits; Karatsuba through 5632; then owned NTT |
| Auto + `cache` | Tiny/fixed routes first; batched below 22 significant bytes; general Matrix through 8192, or 3584 with IFMA; then NTT | Batched below 576 significant digits on NEON/Wasm, 1024 elsewhere; supported medium-size Matrix; then NTT |
| `no_alloc` | Fixed 32/64-byte matrices; batched elsewhere | Tiny/batched |

Cached decoding uses Matrix only on NEON, Wasm or x86 with SSE4.1. Its upper
boundary is 5632 text bytes, reduced to 5120 with IFMA. IFMA hosts compiled with
AVX2 keep the remaining medium range on batched decoding. Other targets retain
batched decoding there. Recursive leaves also have independent thresholds.

The `cache` feature is an implementation policy, not a runtime API parameter.
`Ntt::new` owns optional per-call tables and initializes them through mutable
access once the transform size is known. Only cache-enabled builds contain the
shared `OnceLock` stores. `__internals` does not enable them.

## Architecture-specific work

| Target | Selected operations | Source layout |
| --- | --- | --- |
| Little-endian AArch64 + NEON | Matrix parsing/panels and character packing; fixed encoders; NTT butterflies, pointwise products and CRT | `matrix/neon.rs`, `encode/matrix/neon.rs`, `ntt/neon.rs` |
| x86-64 SSE4.1 / AVX2 | Matrix panels; AVX2 fixed 64-byte encoding; target-feature Rust NTT wrappers | `matrix/x86.rs`, `encode/matrix/x86.rs`, `ntt/x86.rs` |
| x86-64 AVX-512 | Native NTT; VBMI fixed 64-byte encoding; IFMA radix-`2^52` schoolbook leaves | `ntt/avx512.rs`, `encode/matrix/x86.rs`, `mul_ifma/avx512.rs` |
| Wasm and other targets | Portable kernels; LLVM may vectorize them when the target permits it | Shared portable modules |

On x86, runtime checks guard the required ISA combinations. With `std`, detection
uses `std::is_x86_feature_detected!`; otherwise it uses `cpufeatures`. The latter
caches five atomic-byte detection results without heap allocation. Cargo still
resolves that x86 dependency with `std`, but those detector expansions are excluded.
Native x86 kernels also require baseline SSE2 in the target. Soft-float targets
without it use portable kernels; runtime CPUID cannot opt them into SIMD.
Selectors use `core::cfg_select!`; the MSRV is Rust 1.95.

SIMD applies beyond fixed lengths. Fixed 32-byte x86 encoding deliberately uses
plain Rust; fixed 64-byte encoding has explicit vector kernels. AVX2 NTT wrappers
allow LLVM vectorization, while AVX-512 kernels are hand-written. `*_portable`
hooks replace a kernel family, not every downstream fallback, and do not promise
scalar assembly. [SIMD results](docs/simd.md) include the mixed Wasm `+simd128`
tradeoff. [Assembly review](docs/assembly.md) covers callbacks, detector loads,
panic paths and size. Ordinary builds use a constant IFMA threshold; only tests
and `__internals` retain its tuning atomic. Earlier ordinary artifacts already
folded that atomic away, so the source cleanup is not a demonstrated speedup.
Generic x86 builds still read cached CPU feature flags in the NTT short-product
fallback. [Broader state plumbing](docs/benchmarks/decisions/README.md) has no
established overall benefit; narrower reuse of the constructor's choice still
needs controlled measurements.

## Heap, stack and cache costs

`no_alloc` makes no allocator calls on cold, warm, success or error paths. Its
general-size conversion remains quadratic. Fixed tables have a combined
reference layout of 2208 bytes on 64-bit targets; emitted binary bytes can differ.
`decode_array::<N>` constructs an `N`-byte local result. General Matrix uses about
30 KiB encoder / 14 KiB decoder stack; recursive encoder leaves add about 8 KiB.
These are source estimates, not measured stack high-water marks.

`encode_string` reserves `max_encoded_len(input.len())` bytes and truncates the
length after conversion; it retains that capacity, including for zero-heavy input.
`decode_vec` reserves one byte per leading `'1'`, then `d - d/4 - d/64` bytes
for the remaining `d` text bytes. The integer bound follows from `58^64 < 256^47`
and cannot overflow. It asymptotically saves 26.6% of the significant-text output
capacity against allocating the full text length; backend scratch is additional. The
prefix scan is shared with dispatch, and truncation does not reallocate. Tiny
inputs of at most two bytes retain the short route. Empty input returns before
entering the nonempty decoder. The [owned-result measurements](docs/benchmarks/2026-10-27/README.md)
retain size-specific timings and unchanged-binary controls that limit attribution.

Matrix construction reuses eight scratch rows. One table has three allocations:
temporary rows, retained panel metadata and retained coefficients. Combining all
three into one retained owner would keep dead scratch alive. Final coefficient
counts are reserved once. Provably insufficient buffers are rejected before
table setup without requiring worst-case output capacity.

The divide-and-conquer decoder packs result, powers and scratch into one arena.
Encoder divisors borrow one pre-sized payload for normalized powers and
reciprocals; construction scratch and original powers are dropped before
prepared transforms and conversion scratch. Each prepared NTT operand owns one
vector containing all three prime transforms. A shared prepared-power arena
saved little peak memory and had no established overall benefit, so it was not retained.
Karatsuba scratch is `5*n + 64` limbs; NTT scratch is `16*n + 64` for the operand
bound of those interfaces. Neither formula bounds total conversion memory.

| Retained with `cache` | Heap payload capacities | Static headers, 64-bit layout |
| --- | ---: | ---: |
| All five encoder + four decoder Matrix classes | 18,617,760 bytes | 504 bytes |
| Production NTT logs 5–18 | 25,164,288 bytes | 5776 bytes |
| Combined maximum after using every production class | **43,782,048 bytes (41.8 MiB)** | **6280 bytes** |

These stores are absent by default. Initialized classes remain for the process
lifetime, and history affects reuse. The largest Matrix tables retain 11,510,016
encoder / 2,437,248 decoder bytes. NTT log `l` needs `48 * 2^l` twiddle bytes;
logs above 18 remain owned even with `cache`. The internal cache helper also
accepts logs 1–4; filling those would raise its NTT bound to 25,165,728 bytes.

Retained capacities exclude allocator overhead, construction scratch, prepared
powers, stack and concurrent conversions. A 4 MiB significant encode can require
96 MiB of owned twiddles per call. The allocation-layout measurement reduced
its extra requested live heap from 284,269,528 to 201,582,112 bytes, with zero
retention in the cache-off build. Those counts exclude allocator-internal
reallocation overlap and RSS. [Current process peaks](docs/benchmarks/2026-10-27/README.md)
and Wasm page high-water are separate metrics. [Allocation data](docs/benchmarks/allocation-layout/README.md)
retain exact source and measurement identities.

Precomputing every general cache table would embed roughly 41.8 MiB before dead
stripping, including classes a process may never use. That is unsuitable as the
default for small/Wasm consumers. The small fixed tables are already compile-time.

## Linked code size

Measure a stripped consumer with matched compiler, target features, direction,
scaffold and LTO. An `.rlib` is not deployment size. Dead stripping removes
unreachable implementations, so deleting an unused backend does not recover its
source size from a consumer. Auto links more code than `no_alloc`, which supplies
a smaller general path. Fixed-only five8 has a narrower contract.

The opt-in `compact` feature reduces the maximum x86 IFMA window from twelve to
eight vector groups. It removes four large unrolled kernel bodies and increases
the chunk count for some operand shapes. A matched prototype saved about 93 KiB
of linked code; that comparison is separate from the older platform size tables.
It preserves format, API and allocation contracts. Speed depends on operand shape;
the default retains all twelve widths. Other production architectures do not use
these IFMA kernels.

[Per-platform backend sizes](docs/benchmarks/backend-resources/README.md),
[public-mode sizes](docs/benchmarks/public-modes/README.md) and
[later allocation-layout sizes](docs/benchmarks/allocation-layout/README.md)
preserve distinct source generations. Use their section deltas as well as total
file bytes. The [README](README.md#linked-size) has the short bs58 comparison.
The rejected loop and explicit Wasm-panel experiments do not justify further size
features. See [verification limits](CORRECTNESS.md#remaining-work) before making
claims about all inputs or architectures.

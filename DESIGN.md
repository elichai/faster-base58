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
Each ten-character chunk is formatted as two five-character groups, reusing the
two-character lookup table to avoid dividing by 58 for each digit.
The public paths handle empty and tiny inputs before the general loops.

[Divide-and-conquer decoding](src/dc.rs) combines `high * (58^10)^e + low`.
[Encoding](src/encode/dc.rs) divides by the same powers, then emits quotient and
padded remainder recursively. Barrett estimates use reciprocals refined by Newton
steps; each power and reciprocal is reused within one conversion. Karatsuba,
Toom-3 and the fallback NTT encoder share this conversion tree. Unbalanced
products use blocks.

On little-endian AArch64 with NEON, large encoding can instead use a
[multiplication tree in base `58^3`](src/ntt/radix.rs). It splits at byte
boundaries and combines `hi * 256^k + lo`, avoiding reciprocals and division at
each node. Two NTT primes recover the digit products; vectorized reconstruction
and one carry pass also add `lo`. Powers and prepared transforms remain local to
the conversion. Owned twiddles cover only those two primes; eligible conversions
borrow the existing shared three-prime tables. Recursive calls reuse one digit
workspace, holding the high child in scratch and the low child in the output
until the carry pass consumes it. Power digits share one backing vector; lower
prepared spectra share another. The top power is transformed in arithmetic
scratch when used at the root, so it needs no stored spectrum. Seed bytes stay
on the stack; schoolbook sums reserve their usual capacity once and can grow
for sparse, unbalanced products. All owners are dropped on return.
This route requires
at least 8192 significant bytes and at least 7/8 utilization of its full transform.
With `cache`, it requires 262144 bytes and at least two input bytes per transform
point, preserving the cheaper division route just below binary power-of-two
boundaries. Heavily padded sizes, significant inputs above 16 MiB, and other
targets retain the division tree. These conservative selection rules follow the
[large-input measurements](docs/performance.md#historical-paired-radix-allocation-comparison).

[Matrix encoding](src/encode/matrix.rs) maps 32-bit words to base `58^4`;
[decoding](src/matrix/mod.rs) maps four-character groups to base `2^32`.
Eight-row, four-column panels share SIMD multiply-add kernels. Scheduled carry
passes and compile-time bounds keep accumulations within `u64`.

Fixed 32/64-byte encoders use the same matrix idea with compile-time tables and
local sums. Their table type keeps rows first and 16-byte aligned for NEON
column loads, avoiding sensitivity to incidental constant placement.
Fixed dimensions remove table construction and dimension handling;
64-byte kernels also segment the carry chain. Auto and `no_alloc` select these
specializations directly. The internal Matrix comparison hooks use the general
matrix kernel. Their short carry schedule needs fixed-size bounds and cannot
replace the general schedule directly. Padding nearby lengths into these kernels
regressed zero-heavy inputs. Portable fixed decoder prototypes were slower than
the general decoder on M5 and added code and tables.

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

Leading zeros affect the selected backend and allocation behavior. With `cache`,
a short significant suffix can select Matrix even when the total input is large.
NTT entry points are outlined to keep large stack frames out of tiny encode calls.

| Mode | Encode | Decode |
| --- | --- | --- |
| Auto, default | Fixed matrices at exactly 32/64 bytes; otherwise batched through 4096 significant bytes, with IFMA Karatsuba at 513–4096; then owned NTT | Tiny through two text bytes; batched through 5120 significant digits; Karatsuba through 5632; then owned NTT |
| Auto + `cache` | Tiny/fixed routes first; batched below 22 significant bytes; general Matrix through 8192, or 3584 with IFMA; then NTT | Batched below 576 significant digits on NEON/Wasm, 1024 elsewhere; supported medium-size Matrix; then NTT |
| `no_alloc` | Fixed 32/64-byte matrices; batched elsewhere | Tiny/batched |

Cached decoding uses Matrix only on NEON, Wasm or x86 with SSE4.1. Its upper
boundary is 5632 significant digits, reduced to 5120 with IFMA. IFMA hosts compiled
with AVX2 use batched decoding from the minimum Matrix size through 5120, then
NTT above 5120. Targets without a supported Matrix kernel use batched decoding
through 5632, then NTT. Recursive leaves also have independent thresholds.

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
scalar assembly. Wasm `+simd128` requires a SIMD-capable engine and has no scalar
runtime fallback within that module. Measurements show mixed tradeoffs; see
[performance](docs/performance.md).

Ordinary builds use a constant IFMA threshold; only tests and `__internals`
retain its tuning atomic. Generic x86 builds still read cached CPU feature flags
in the NTT short-product fallback. Passing threshold state through multipliers
had no established overall benefit; narrower reuse of the constructor's choice
needs controlled measurements. Resolve final linked call targets before treating
source-level function pointers as runtime dispatch costs.

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
entering the nonempty decoder. See [performance](docs/performance.md) for measured
owned-result and resource tradeoffs.

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
Karatsuba scratch is `5*n + 64` limbs; NTT scratch is `12*n + 128` for the operand
bound of those interfaces. Neither formula bounds total conversion memory.

| Retained with `cache` | Heap payload capacities | Static headers, 64-bit layout |
| --- | ---: | ---: |
| All five encoder + four decoder Matrix classes | 18,617,760 bytes | 504 bytes |
| Production NTT logs 5–18 | 12,582,144 bytes | 3040 bytes |
| Combined maximum after using every production class | **31,199,904 bytes (29.8 MiB)** | **3544 bytes** |

These stores are absent by default. Initialized classes remain for the process
lifetime, and history affects reuse. The largest Matrix tables retain 11,510,016
encoder / 2,437,248 decoder bytes. NTT log `l` needs `24 * 2^l` twiddle bytes;
only forward and inverse roots are stored. Their Montgomery companions are
derived with a wrapping 32-bit multiply when loaded, including SIMD loads.
Logs above 18 remain owned even with `cache`. The internal cache helper also
accepts logs 1–4; filling those would raise its NTT bound to 12,582,864 bytes.

Retained capacities exclude allocator overhead, construction scratch, prepared
powers, stack and concurrent conversions. The division-tree route can require
48 MiB of owned twiddles for a 4 MiB significant encode. The selected AArch64
radix route uses two prime tables instead of three and pools its digit and
prepared-spectrum scratch. A 2026-10-29 comparison against `7962809` reduced
4 MiB AArch64 encoding from 131,965,668 to 98,411,236 bytes of extra requested
live heap, with no retained heap under either cache policy. It saves 32 MiB
by halving the two-prime twiddle payload. The three-prime decoder also benefits
from tighter scratch, falling from 109,997,816 to 75,657,464 bytes by default.
These counts exclude caller buffers, allocator-internal
reallocation overlap and RSS. Other routes and sizes differ; process peaks and
Wasm page high-water are separate metrics. See [measurement scopes](docs/performance.md).

Precomputing every general cache table would embed roughly 29.8 MiB before dead
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
the chunk count for some operand shapes. The 2026-10-28 measurements of runtime
source `304dc4c5dbf1` show about 93.2 KiB less executable code, 33–37% of the matched
x86 Auto delta above empty. On one EPYC 9B45 VM, compact's cache-off encoding is
about 2–9% faster at 4 KiB–4 MiB; large decoding and cache-mode conversions mostly
cost 1–3%.
These whole-consumer observations include compiler/call-context effects;
identical-content controls also varied. M5 and scalar Wasm have no section saving.
Format, API and the no-allocation contract are unchanged; wide12 remains the
default. Other production architectures do not use these IFMA kernels.

Use linked-section deltas as well as total file bytes, and keep source generations
separate. [Performance](docs/performance.md) explains the measurements and update
workflow; the [README](README.md#linked-size) has the short bs58 comparison.
The rejected loop and explicit Wasm-panel experiments do not justify further size
features. See [verification limits](CORRECTNESS.md#remaining-work) before making
claims about all inputs or architectures.

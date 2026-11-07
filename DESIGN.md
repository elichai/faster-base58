# Algorithms and resource costs

The public API uses `Encoder` / `Decoder`. Default feature `alloc` selects fast
allocating backends; disabling it selects heap-free conversion. Named backends,
portable controls and tuning hooks require `__internals` and are unstable.
The library uses `core` and `alloc` by default. `cache` enables `alloc` and `std` and retains
generated tables. See [correctness and verification](CORRECTNESS.md),
[measurements](docs/performance.md), and [implementation references](REFERENCES.md).

Automatic backend policy lives in `src/api.rs`. Its encode/decode selectors are
shared by plain and checked buffer calls. Plain decoding tests the same small
cutoff directly before matching the large backend, keeping enum classification
off the small path. Fixed-size encoding and tiny decoding run before general
selection; their wrappers preserve the specialized input contracts.

Decoders take raw bytes and return a crate-private two-variant error; only the
public terminal methods convert it to `Error`. Kernels that can fail only on
capacity return the zero-sized `BufferTooSmall`. This keeps hot-loop result
layouts independent of public, feature-gated error variants. Encoder output is
alphabet bytes only, so `.onto()` and `.into_string()` skip UTF-8 validation
(a debug assertion checks it).

Base58Check uses the same conversion families; input loading and final
serialization sit outside the arithmetic bodies, so checking adds no arithmetic
monomorphization. Encoding feeds small frames as borrowed segments and stages
larger frames inside the output. Checked decoding writes two logical segments,
the data in the caller's buffer and the four-byte checksum on the stack, and
verifies the checksum after conversion, before the version.

## Conversion families

| Family | Method and role | Main cost |
| --- | --- | --- |
| Batched | Horner conversion, seven binary bytes or ten Base58 digits per step; paired updates overlap carry chains | Quadratic work, no heap allocation |
| Karatsuba | Balanced radix conversion with Karatsuba multiplication and batched leaves | Per-call powers, reciprocals and scratch |
| Matrix | Independent multiply-adds against a power table, followed by bounded carry passes | Quadratic table size and work; setup can dominate small calls |
| NTT | Balanced radix conversion with transform multiplication and prepared powers | Subquadratic large-input work, substantial scratch and twiddle storage |
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
steps; each power and reciprocal is reused within one conversion. Karatsuba
and the fallback NTT encoder share this conversion tree. Unbalanced products
use blocks.

On little-endian AArch64 with NEON and on x86-64 with AVX-512F, large encoding can
instead use a
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
more than 4096 significant bytes (on x86, 48 KiB) and at least 5/8 utilization
of its root transform, or 3/4 with `cache`. Utilization grows linearly with the
input from 0.48 just above each transform-length step (`17 * 2^k + 1` bytes) to 0.97.
With `cache`, cached Matrix leaves speed up only the division tree, so its crossover
is higher. On Zen 5 the division tree with
IFMA Karatsuba products is faster up to the 32 KiB sizes. Heavily padded sizes,
significant inputs above 16 MiB, x86 CPUs without AVX-512F (not measured) and
other targets keep the division tree. The carry pass runs four
interleaved chains over quarters of the digits, then adds each quarter's carry
into the next.

[Matrix encoding](src/encode/matrix.rs) maps 32-bit words to base `58^4`;
[decoding](src/matrix/mod.rs) maps four-character groups to base `2^32`.
Eight-row, four-column panels share SIMD multiply-add kernels. Scheduled carry
passes and compile-time bounds keep accumulations within `u64`.

Fixed 32/64-byte encoders use the same matrix idea with compile-time tables and
local sums. Their table type keeps rows first and 16-byte aligned for NEON
column loads, avoiding sensitivity to incidental constant placement.
Fixed dimensions remove table construction and dimension handling;
64-byte kernels also segment the carry chain. Both allocation policies select these
specializations directly. The internal Matrix comparison hooks use the general
matrix kernel. Their short carry schedule needs fixed-size bounds and cannot
replace the general schedule directly. Nearby lengths are not padded into these
kernels, which would slow zero-heavy inputs, and there is no fixed-size decoder: the
general decoder is faster on M5.

[NTT multiplication](src/ntt/mod.rs) uses two 32-bit coefficients per limb,
three primes below `2^30`, Montgomery products and exact CRT reconstruction.
Forward/inverse orderings avoid a separate bit-reversal pass. Kernels pass values
in `(-q, q)`. The forward transform reduces every butterfly sum. Inside the inverse,
values stay below `2 q`: every other stage reduces only its `x` input, and only the
last stage reduces its outputs ([`reduces_x`](src/ntt/portable.rs)). Garner's CRT
step normalizes only the first residue. Squares and
prepared radix powers save transforms; cyclic/negacyclic pieces reduce padding.
Wrapped products shorten some reciprocal work. The `2^23`-point transform cap
supports the coefficient bounds; larger products split. It is not an input limit.

On x86-64 CPUs with AVX-512 IFMA, the same products, pieces, prepared powers and
wrapped products [use two primes below `2^50`](src/ntt/ifma.rs) in `u64` lanes:
Harvey's lazy butterflies with Shoup twiddles (`vpmadd52*`), then a two-prime
Garner step. A product coefficient sums at most `2^22` products of 32-bit
coefficients, below `2^86` (`2^87` for wrapped products), and the primes' product
exceeds `2^99.99`. Values stay below `4p < 2^52`, the width IFMA multiplies. Only
forward roots are stored; inverse stages read them backwards, since
`w^-j = -w^(h-j)`. Both backends share the plan, folding and polynomial CRT code.
The base-`58^3` radix encoder keeps its two 30-bit primes.

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
| Without `alloc` | Fixed 32/64-byte matrices; batched elsewhere | Tiny/batched |

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
| x86-64 AVX-512 | Native NTT and the base-`58^3` encoder's two-prime CRT; VBMI fixed 64-byte encoding; IFMA radix-`2^52` schoolbook leaves and two-prime NTT products | `ntt/avx512.rs`, `encode/matrix/x86.rs`, `mul_ifma/avx512.rs`, `ntt/ifma.rs` |
| Wasm with `simd128` | Matrix parsing/panels; fixed 64-byte column sums and characters; NTT butterflies, pointwise products and CRT | `matrix/wasm.rs`, `encode/matrix/wasm.rs`, `ntt/wasm.rs` |
| Scalar Wasm and other targets | Portable kernels; LLVM may vectorize them when the target permits it | Shared portable modules |

On x86, runtime checks guard the required ISA combinations. With `std`, detection
uses `std::is_x86_feature_detected!`; otherwise it uses `cpufeatures`. The latter
caches five atomic-byte detection results without heap allocation. Cargo still
resolves that x86 dependency with `std`, but those detector expansions are excluded.
Native x86 kernels also require baseline SSE2 in the target. Soft-float targets
without it use portable kernels; runtime CPUID cannot opt them into SIMD.
Selectors use `core::cfg_select!`; the MSRV is Rust 1.95. A Matrix conversion checks
for AVX2 once and runs its whole panel loop in one AVX2 function; without AVX2, each
panel selects the SSE4.1 or portable kernel.

SIMD applies beyond fixed lengths. Fixed 32-byte x86 encoding deliberately uses
plain Rust; fixed 64-byte encoding has explicit vector kernels. AVX2 NTT wrappers
allow LLVM vectorization, while AVX-512 kernels are hand-written. `*_portable`
hooks replace a kernel family, not every downstream fallback, and do not promise
scalar assembly. Wasm `+simd128` requires a SIMD-capable engine and has no scalar
runtime fallback within that module; its kernels are selected at compile time.
Wasm has no 32-bit high multiply, so they widen products with `extmul` and pick the
needed halves with one shuffle. Splat operands of those products are read through
`black_box` inside each loop: otherwise LLVM hoists their extension and emits
`i64x2.mul`, which V8 expands to several instructions. Fixed 32-byte encoding and
general Matrix character packing stay scalar on Wasm. Measurements show mixed
tradeoffs; see [performance](docs/performance.md).

Dispatch calls one `#[target_feature]` function per kernel. The vector code it runs
is inlined into it, or kept in functions that have those features themselves (cold
paths and kernels selected by pointer). Code on vectors therefore uses
`#[inline(always)]` functions and explicit arrays, not `core` array and iterator
adapters, nor closures outside `#[target_feature]` functions: those compile without
the caller's target features, so a copy that LLVM keeps out of line turns each
intrinsic into a call. CI checks every out-of-line crate function against
`scripts/check_simd_inlining.txt`
([CORRECTNESS.md](CORRECTNESS.md#tests-and-independent-references)).

The IFMA leaf threshold is a constant. Generic x86 builds still read cached CPU
feature flags in the NTT short-product fallback; passing threshold state through
multipliers had no established overall benefit.

## Heap, stack and cache costs

Without `alloc`, conversion makes no allocator calls on cold, warm, success or error paths. Its
general-size conversion remains quadratic. Fixed tables have a combined
reference layout of 2208 bytes on 64-bit targets; emitted binary bytes can differ.
`.into_array::<N>()` constructs an `N`-byte local result. General Matrix uses about
30 KiB encoder / 14 KiB decoder stack; recursive encoder leaves add about 8 KiB.
These are source estimates, not measured stack high-water marks.

Encoding `.into_string()` reserves `max_encoded_len(input.len())` bytes and truncates the
length after conversion; it retains that capacity, including for zero-heavy input.
Decoding `.into_vec()` reserves one byte per leading `'1'`, then `d - d/4 - d/64` bytes
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
reciprocals; original powers and their construction scratch share one buffer, dropped before
prepared transforms and conversion scratch. Each prepared NTT operand owns one
vector containing all three prime transforms (12 bytes per point; the two-prime
IFMA backend stores 16); decoding squares each prepared
power from those transforms, without a forward transform. Prepared powers do not
share one arena, which would save little peak memory.
Skipped prepared powers do not grow metadata, but still clear an existing slot.
Karatsuba scratch is `5*n + 64` limbs. Within the transform domain, NTT scratch
uses the nondecreasing workspace envelope of every plan through a product of
`2*n` limbs, plus 128 spare limbs and clipped at `12*n + 128`. It lies between
`8*n + 128` and `12*n + 128`; larger inputs keep the linear bound. The envelope
retains the previous one-piece maximum when a two-piece plan needs less space.
It is never below Karatsuba and preserves the DC, block and split inequalities.
The two-prime backend's residues take 8 bytes instead of 4, so it asks 1.5 times
that bound, or Karatsuba's alone for operands below both transform thresholds.
These are operand bounds in `u64` limbs, not total conversion memory.

The two-prime radix encoder batches spectra and child workspace in one `i32`
arena. If the root needs `P` transform points and spectra need `S` elements, the
arena holds `max(S + 3*P/2, 3*P)` elements instead of `S + 3*P`. Every child and
power-building square fits `P/2` points: for the digit count of `256^b` in base
`58^3`, `d(2*b)` is `2*d(b)` or `2*d(b)-1`, and parity removes the apparent
extra point at the boundary. After both root children finish, the power metadata
and its spectrum borrows are dropped. The entire arena then serves the root's
`2*P` residues and `P` temporary elements. The scalar sums buffer is preserved.
This never increases the arena payload, including small inputs with no spectra.

| Retained with `cache` | Heap payload capacities | Static headers, 64-bit layout |
| --- | ---: | ---: |
| All five encoder + four decoder Matrix classes | 18,617,760 bytes | 504 bytes |
| Production NTT logs 5–18 | 9,436,416 bytes | 1672 bytes |
| Two-prime IFMA NTT logs 5–18 (x86-64 with IFMA) | 16,786,944 bytes | 1216 bytes |
| Combined maximum after using every production class, without IFMA | **28,054,176 bytes (26.8 MiB)** | **3392 bytes** |
| Combined maximum with IFMA | **44,841,120 bytes (42.8 MiB)** | **3392 bytes** |

These stores are absent by default. Initialized classes remain for the process
lifetime, and history affects reuse. The largest Matrix tables retain 11,510,016
encoder / 2,437,248 decoder bytes. NTT log `l` needs
`12 * (2^l + min(2^l, 2^16))` twiddle bytes. Forward roots and a bounded inverse
prefix share one allocation per prime. Larger inverse transforms generate the remaining stages in
the product temporary after pointwise multiplication has consumed it; negacyclic
untwisting reuses that same temporary. Their Montgomery companions are derived
with a wrapping 32-bit multiply when loaded, including SIMD loads.
Logs above 18 remain owned even with `cache`. The internal cache helper also
accepts logs 1–4; filling those would raise its NTT bound to 9,437,136 bytes.
With IFMA, decoders and the division tree fill the two-prime store instead:
log `l` needs `32 * 2^l + 768` bytes in one allocation per prime (forward roots,
their Shoup companions, eight padding entries each, and the short inverse
stages); the radix encoder still fills the three-prime store, so both can be
retained.

Retained capacities exclude allocator overhead, construction scratch, prepared
powers, stack and concurrent conversions. The division-tree route can require
24.75 MiB of owned twiddles for a 4 MiB significant encode, 64 MiB with the
two-prime IFMA tables (log 21). The selected AArch64
and x86 radix route uses two prime tables instead of three, pools its digit and
prepared-spectrum scratch, and retains nothing under either cache policy. Measured
4 MiB peaks are in the [performance guide](docs/performance.md#cache-and-memory).

Precomputing every general cache table would embed roughly 26.8 MiB before dead
stripping, including classes a process may never use. That is unsuitable as the
default for small/Wasm consumers. The small fixed tables are already compile-time.

## Linked code size

Measure a stripped consumer with matched compiler, target features, direction,
scaffold and LTO. An `.rlib` is not deployment size. Dead stripping removes
unreachable implementations, so deleting an unused backend does not recover its
source size from a consumer. Auto links more code than conversion without `alloc`, which supplies
a smaller general path. Fixed-only five8 has a narrower contract.

The opt-in `compact` feature reduces the maximum x86 IFMA window from twelve to
eight vector groups. It removes four large unrolled kernel bodies (about 93 KiB of
code) and increases the chunk count for some operand shapes; the
[performance guide](docs/performance.md#linked-size-and-compact) has the measured
tradeoff. Format, API and the no-allocation contract are unchanged; wide12 remains
the default. Other production architectures do not use these IFMA kernels.

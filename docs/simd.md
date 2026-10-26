# SIMD coverage and Wasm tradeoffs

The existing kernels were retained. Shared Rust already emits useful Wasm SIMD;
two explicit matrix-panel prototypes added code without a consistent benefit.
Enabling Wasm SIMD remains a workload and engine decision.

These results use baseline `7c6ffa325164`, resource identity `17ab5f264ed4`.
The [receipt](benchmarks/simd/receipts.json) binds full source maps, compiler,
modules, fixtures, runtimes and private raw archives. The production dispatcher,
algorithm and cache policy were unchanged. [DESIGN.md](../DESIGN.md) gives the
current size-dependent routes.

## Existing coverage

| Operation | Native coverage | Wasm coverage |
| --- | --- | --- |
| General matrix panel, both directions | x86 AVX2 → SSE4.1 → portable; AArch64 NEON | Shared Rust vectorizes with simd128. |
| General digit parsing and character output | NEON on AArch64; shared Rust on x86 | Shared Rust, subject to compiler vectorization. |
| Fixed 32-byte encode | AArch64 NEON; plain Rust on x86 | Plain Rust. |
| Fixed 64-byte encode | AArch64 NEON; x86 VBMI → AVX2 → plain Rust | Plain Rust. |
| NTT | AArch64 NEON; AVX2/AVX512F-targeted portable functions and handwritten AVX512 kernels | Portable functions emit vector instructions with simd128. |
| Schoolbook/Karatsuba/experimental Toom-3 | Scalar limb arithmetic; separate AVX512F+IFMA multiplication path | Scalar limb arithmetic. |

Plain Rust does not imply scalar machine code. There is no separate SSE4.1 NTT
wrapper. Fixed64 VBMI requires AVX2 and AVX512F/BW/VL/VBMI together. Native
selectors retain OS-aware feature guards; see [assembly findings](assembly.md).

## Compiler SIMD flag

The same Wasm modules were built once on M5 with Rust 1.99.1 / LLVM 23.1.1,
opt-level 3, fat LTO and 16 codegen units, then run on M5 Pro and the AMD EPYC
9B45 VM on GCP. Both use Node 26.12.1 / V8 14.6.202.34-node.40. Each cell has
three fresh-process pairs and three warm batches per process, each at least
3 ms elapsed. The change is the median paired time ratio; positive means slower.
It can differ from the ratio of side medians. Ranges in the
[complete CSV](benchmarks/simd/timings.csv) are descriptive, not confidence intervals.

| Mode / operation / decoded size | M5 paired time change, +SIMD vs −SIMD | GCP paired time change |
| --- | ---: | ---: |
| Auto default encode, 8 KiB | -28.35% | -15.14% |
| Auto default decode, 8 KiB | -20.72% | -15.82% |
| Auto+cache encode, 4 KiB | +36.67% | +3.01% |
| Auto+cache decode, 4 KiB | +31.44% | +3.06% |

The measured larger NTT routes benefit; cached matrix routes can regress.
Small fixed/batched cases are mixed. A SIMD module requires an engine with SIMD
support and has no scalar runtime fallback inside that module. Disabling SIMD
does not disable every other default Wasm target feature.

These timings cover conversion into reusable caller buffers. They exclude
caller allocation/copying, owned String/Vec helpers, prevalidation of repeated
decode input, and module compilation/instantiation. Cold and second conversion
times, including cache setup, remain separate in the CSV.

| Both-direction linked module | −SIMD stripped B | +SIMD stripped B | Change B |
| --- | ---: | ---: | ---: |
| Auto default | 103,628 | 113,054 | +9,426 |
| Auto+cache | 105,781 | 114,469 | +8,688 |
| no_alloc, cache-linked control | 40,035 | 40,396 | +361 |
| Empty scaffold | 16,452 | 16,876 | +424 |

[All 30 modules](benchmarks/simd/linked-sizes.csv) retain CODE/DATA sections,
gzip sizes, initial memory, empty-scaffold deltas and artifact hashes. These are
linked cdylibs with both directions reachable, not rlibs or V8 JIT-code memory.
Wasm pages are not requested heap or RSS.

## Rejected explicit panel kernels

The first prototype kept four accumulators and eight rows, using unsigned
32×32→64 widening intrinsics. The second kept that panel out of line. Layout,
parsing, carry bounds, native kernels and no_alloc routes were unchanged.
LLVM hoisted extended digit vectors and emitted general `i64x2.mul` in both
versions; the intended unsigned `extmul` instruction did not survive.

Against the existing +SIMD Auto+cache module, the inline-eligible version added
586 stripped bytes; the out-of-line version added 86. On M5 the first version's
4 KiB encode improved 8.91%, but 1 KiB decode worsened 12.36%. Outlining worsened
4 KiB encode 13.35% and decode 12.50%. The corresponding GCP panel cells were
mostly near-flat. Unchanged-route controls also varied, with some M5 process
ranges spanning 20–50%. No cause was established for that variation. The added
code and inconsistent results did not support either prototype. All cases,
including zero/tail inputs and wide ranges, remain in the CSV.

The panel source adds no heap allocation or large Rust stack array. Host JIT
spills were not measured. Cache storage and bounds were unchanged; this is not
a new allocation policy.

## Validation and reproduction limits

On each host, five actual Wasm consumers passed 2,929 conversion cases and 192
exact error-index cases each. SIMD consumers also passed 400 bounded panel cases
against a u128 oracle and five malformed-shape traps. Cases include unaligned
buffers, unsigned high bits, leading zeros, exact/short/surplus output buffers,
sentinels and unchanged inputs. This is Node execution, not a Miri verification
of Wasm SIMD. [CORRECTNESS.md](../CORRECTNESS.md) separates supported checks from
interpreter limitations.

Across both hosts, 1,008 fresh processes supplied 3,024 accepted warm batches.
All summary CSVs replayed byte-for-byte from the raw workers using the original
statistical exporter. This is reproducibility, not an independent statistical
audit. Laptop power was sampled; no thermal or frequency claim follows.

For new measurements, build separate −simd128/+simd128 modules from one frozen
source, retain exact target flags and inspect the linked code before timing.
`scripts/wasm_tradeoffs.mjs` validates metadata-bound modules under the recorded
Node flags. `scripts/code_size.py` controls RUSTFLAGS itself and does not expose
an arbitrary SIMD-flag override; setting that environment variable alone does
not reproduce this experiment. Exact historical panel rebuild/replay needs the
archival inputs identified in the receipt. A Rust 1.95 Wasm compilation was not
performed for these rejected prototypes.

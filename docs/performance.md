# Performance and resource tradeoffs

The [README tables](../README.md#performance) show the current public-API timings
and linked sizes. Auto trades workspace for large-input speed; disabling `alloc`
avoids internal allocation, and its general large conversions remain quadratic.
[DESIGN.md](../DESIGN.md) describes dispatch and algorithms;
[CORRECTNESS.md](../CORRECTNESS.md) describes validation and remaining gaps.

## Current results

Measured **2026-11-05** with ordinary public consumers: `code_size.py` cdylibs that
call `Encoder::bitcoin().encode(..).onto(..)` and `Decoder::bitcoin().decode(..).onto(..)`,
with no `__internals`. Release opt-level 3, fat LTO, 16 codegen units, generic
target flags (`RUSTFLAGS=""`). Default and `cache` are separate builds; no-allocation
uses `default-features = false`; bs58 0.5.1 and five8 1.0.0 use their defaults.

| Platform | Library source | Compiler / runtime | Clock, processes |
| --- | --- | --- | --- |
| Apple M5 Pro, macOS 26.7.2, AC power | `8bab387` | Rust 1.99.1 / LLVM 23.1.1 | process CPU, 5 |
| GCP AMD EPYC 9B45 (Zen 5, 4 vCPU), Linux, CPU 0 pinned | `18b2eb8` | Rust 1.99.0 / LLVM 23.1.1 | process CPU, 3 |
| Scalar Wasm on the M5 | `18b2eb8` | Rust 1.99.1; Node 26.13.0 / V8 14.6.202.34 | V8 elapsed, 3 |

`18b2eb8` and `8bab387` differ only in the radix route rule, which is compiled for
little-endian AArch64 with NEON only; the Wasm Auto module is byte-identical
between them. Each fresh process times every library, interleaved, in three
batches of at least 10 ms after calibration. A cell is the median of the process
medians; figure bars span the process medians. Fixtures are random bytes without
leading zeros. Outputs and round trips are checked outside timing, decode input
is prevalidated UTF-8, and caller buffers are reused (decode output is exact).
Workspace allocation is inside the timing. No-allocation and bs58 stop at 64 KiB;
five8 covers 32/64 B.

Ranges for 4 MiB Auto (median [min, max] of process medians):

| Platform / policy | Encode | Decode |
| --- | ---: | ---: |
| M5 / default | 215.5 ms [214.7, 217.4] | 138.5 ms [137.5, 141.5] |
| M5 / cache | 216.6 ms [216.3, 217.1] | 133.4 ms [133.2, 136.1] |
| x86-64 / default | 334.1 ms [332.8, 334.6] | 149.6 ms [147.7, 150.0] |
| x86-64 / cache | 311.0 ms [310.5, 311.6] | 148.4 ms [148.3, 149.0] |
| Wasm / default | 1.77 s [1.71, 1.92] | 880.5 ms [835.8, 922.7] |
| Wasm / cache | 1.80 s [1.70, 1.90] | 879.7 ms [839.9, 918.7] |

Default Auto encoding around the fixed 32/64-byte routes:

| Payload | M5 | x86-64 | Wasm |
| --- | ---: | ---: | ---: |
| 16 B | 13.9 ns | 25.7 ns | 22.8 ns |
| 21 B | 14.8 ns | 27.8 ns | 29.5 ns |
| 22 B | 16.6 ns | 31.2 ns | 33.4 ns |
| 31 B | 24.1 ns | 43.5 ns | 54.2 ns |
| 32 B | 11.7 ns | 30.0 ns | 26.2 ns |
| 33 B | 23.8 ns | 43.2 ns | 52.3 ns |
| 63 B | 57.5 ns | 108 ns | 148 ns |
| 64 B | 23.4 ns | 31.1 ns | 105 ns |
| 65 B | 56.8 ns | 108 ns | 165 ns |
| 128 B | 149 ns | 319 ns | 545 ns |

Not measured in this cohort: cold calls, counted allocations, owned-output
helpers, checked conversion, native CPU flags and other compilers. The sections
below say when their results are older.

## Measure and update

Run from the repository root on an idle host (laptops on AC power); keep output
under ignored `target/measurements/`. Build every compared artifact before timing
and never time while compiling.

Public consumers (the README tables and figures):

```sh
python3 -B scripts/code_size.py --source . --output target/measurements/m5-default --target aarch64-apple-darwin --cpu generic --backends empty,auto,no-alloc,bs58,five8 --directions both --kinds cdylib
python3 -B scripts/code_size.py --source . --output target/measurements/m5-cache --target aarch64-apple-darwin --cpu generic --backends empty,auto --directions both --kinds cdylib --cache
python3 -B scripts/public_bench.py --platform m5 --output target/measurements/m5.csv auto=target/measurements/m5-default/auto-both-cdylib.dylib auto+cache=target/measurements/m5-cache/auto-both-cdylib.dylib no-alloc=target/measurements/m5-default/no-alloc-both-cdylib.dylib bs58=target/measurements/m5-default/bs58-both-cdylib.dylib five8=target/measurements/m5-default/five8-both-cdylib.dylib
```

On Linux add `--strip-tool strip` to `code_size.py` (it expects `llvm-strip`) and
`--cpu 0` to `public_bench.py` to pin workers. For Wasm, build with `--target
wasm32-unknown-unknown` and time with `public_bench.py --wasm`; it runs Node with
`--no-liftoff --no-wasm-lazy-compilation --no-wasm-async-compilation`. Render the
README figures and tables from the CSVs and each cohort's `sizes.csv`:

```sh
python3 -B assets/graphs/render_current.py --data m5.csv gcp.csv wasm.csv --sizes m5=m5-default/sizes.csv m5+cache=m5-cache/sizes.csv ... --output NEW_DIRECTORY
```

Rendering needs the pinned Matplotlib in `assets/graphs/requirements.txt`.
`python3 -B assets/graphs/render.py --check` validates links and figure syntax
only; it does not reproduce numbers.

Paired comparisons: build both variants' Auto probes, then pass both to
`public_bench.py` with `--baseline LABEL` (more `--processes` and `--batches` help
for small effects). It prints each cell's median and range of per-process
ratios against the baseline with the same cache policy.

Internal backends and allocations: `scripts/resource_report.py build` / `run`
times internal workers (`__internals`, not ordinary public consumers) and counts
allocations with `--count-allocations` in separate, untimed builds;
`scripts/wasm_allocations.mjs` counts Wasm allocations. The Criterion benches
(`benches/encode.rs`, `decode.rs`, `check.rs`) compare backends and comparators in
one process with wall time. `benches/encode_tune.rs` and `encode_matrix_tune.rs`
tune the encoder leaves, `MATRIX_MIN` and `IFMA_CAP`.

When updating results, change each table, figure, date, source and build scope
together, and label results that the code has since changed as historical.

## Timing limits

Code placement moves single cells by more than many source changes do. On Zen 5,
unchanged 512 B paths moved by up to ±20% between builds of unrelated changes, and
the same library loaded at another address moved a 1 KiB encode by 12%. On the
M5, default 512 B decoding is sensitive to loop alignment (a `-C
llvm-args=-align-loops=64` build removed a 10–20% difference that came and went
with unrelated changes). Earlier, byte-identical bs58/five8 modules measured in
separate cohorts differed by 0.74–1.03×. Therefore:

- compare variants only within one paired run (`--baseline`), never across cohorts;
- judge a change by the cells its code path reaches, and treat movement in
  untouched cells as the noise floor;
- for effects below about 5%, add an aligned build
  (`-C llvm-args=-align-all-functions=6 -C llvm-args=-align-loops=64`) as a
  diagnostic; it is not the shipped configuration.

Background load on a laptop can make one process of a cell slow (a bimodal
spread); rerun with more processes rather than dropping observations.

## Generic and native x86-64

**Historical** internal-worker observations from 2026-10-28 (Rust 1.99.1, before
#7, #15, #16 and #18), with the median of nine batches. Generic builds keep
runtime CPU selection; native builds use `-C target-cpu=native` on the EPYC VM
(AVX2, AVX-512 IFMA and VBMI). Each cell is **encode / decode**:

| Build / policy | 4 KiB | 64 KiB | 4 MiB |
| --- | ---: | ---: | ---: |
| Generic / default | 51.0 µs / 33.1 µs | 3.56 ms / 1.36 ms | 351 ms / 165 ms |
| Generic / cache | 41.3 µs / 33.3 µs | 2.78 ms / 1.30 ms | 326 ms / 163 ms |
| Native / default | 48.8 µs / 21.9 µs | 3.56 ms / 1.24 ms | 340 ms / 156 ms |
| Native / cache | 38.4 µs / 22.2 µs | 2.77 ms / 1.17 ms | 313 ms / 154 ms |

Native code generation mostly helps decoding. It is not a portable deployment
setting.

## basexx comparison

`basexx` 0.2.0 is a benchmark comparator: its `Base58` is much faster than bs58,
and Auto's margin over it is much smaller than over bs58. **Historical**
Criterion medians from 2026-10-29 (M5 Pro, `-C target-cpu=apple-m4`, fat LTO,
`__internals`, cache off; 30 samples, wall time). basexx returns an owned result
that the adapter copies and drops inside the loop; bs58 and Auto reuse caller
output.

| Operation / payload | Auto | bs58 0.5.1 | basexx Base58 | basexx Base58B |
| --- | ---: | ---: | ---: | ---: |
| Encode / 1 KiB | 12.5 µs | 1.14 ms | 28.0 µs | 968 µs |
| Encode / 4 KiB | 109 µs | 17.2 ms | 188 µs | 13.7 ms |
| Decode / 1 KiB | 2.77 µs | 333 µs | 5.45 µs | 315 µs |
| Decode / 4 KiB | 49.9 µs | 5.34 ms | 73.0 µs | 5.38 ms |

Reproduce with `cargo bench --bench encode` / `--bench decode` (feature
`__internals`), filtering `(bs58|basexx|basexx-b|auto)/(1024|4096)$`.

## Cache and memory

Enabling `cache` changes dispatch and leaf policy as well as retention. Maximum
retained payloads are in [DESIGN.md](../DESIGN.md#heap-stack-and-cache-costs):
Matrix tables 18,617,760 B and NTT twiddles 9,436,416 B (26.8 MiB combined,
depending on call history), plus 2,176 B of static headers. Per-call scratch,
larger owned tables, allocator overhead and concurrent calls add to that; it is
not an active-heap bound. Wasm linear-memory pages need not return to the host.

Warm peaks for a 4 MiB payload, measured 2026-11-01 with counted `__internals`
Auto workers after #7 (caller buffers and previously retained cache excluded):

| Host | Encode peak / requests | Decode peak / requests |
| --- | ---: | ---: |
| M5 Pro, default and cache | 69,575,396 B / 7 | 56,783,096 B / 18 |
| EPYC 9B45, default | 109,639,584 B / 23 | 56,592,440 B / 12 |
| EPYC 9B45, cache | 108,099,416 B / 23 | 56,592,440 B / 12 |

Requests count allocations plus reallocations; peaks are requested Rust layouts,
not RSS, allocator usable sizes or stack high water. Before #7 the same M5 calls
peaked at 131,965,668 B (encode) and 109,997,816 B (decode). Later changes keep
these peaks: the radix carry pass and prepared-power squares use existing
workspace. The unified radix rule makes cached M5 encoding at 16 KiB–1 MiB use the
radix route's owned tables, as default builds already did, instead of retained NTT
tables; those sizes were not recounted.

**Historical** first and second calls at 4 KiB (2026-10-29, `06e0815`), single
operations including timer overhead and cold code/data effects, **encode / decode**:

| Platform / policy | First call | Second call | Warm |
| --- | ---: | ---: | ---: |
| M5 / default | 124 µs / 56 µs | 111 µs / 40 µs | 109 µs / 40.6 µs |
| M5 / cache | 1.34 ms / 808 µs | 31 µs / 31 µs | 29 µs / 25.9 µs |
| x86-64 / default | 122 µs / 50.6 µs | 54.4 µs / 29.4 µs | 49.7 µs / 27.5 µs |
| x86-64 / cache | 254 µs / 66.5 µs | 51.9 µs / 29.2 µs | 40.8 µs / 27.3 µs |
| Wasm / default | 486 µs / 226 µs | 474 µs / 223 µs | 466 µs / 218 µs |
| Wasm / cache | 1.86 ms / 1.07 ms | 150 µs / 129 µs | 148 µs / 127 µs |

First cached calls build tables; by estimate, the 4 KiB Matrix classes are 0.7–1 ms
of serial carry chains. Whether reuse repays that depends on the workload.

## Linked size and compact

The [README size table](../README.md#linked-size) subtracts each cohort's empty
consumer (same scaffold, no conversion call) from cdylibs with both directions
reachable. Native columns are code and readonly sections; Wasm columns are CODE
and DATA sections. Initialized data, BSS, file size and linear-memory pages are
different costs. `CHAR_PAIRS` (3,364 two-byte entries, 6,728 B) is in readonly
data whenever a reachable path uses it.

With only plain calls retained, enabling `check` added 64 B (default) and 320 B
(`cache`) of M5 code in the 2026-11-05 review build; also retaining checked and
versioned calls added 14–15 KiB.

On x86-64, `compact` keeps eight IFMA kernel widths instead of twelve.
**Historical** (2026-10-28): it saved 95,440 B (about 93 KiB) of generic Auto code
in both policies; 4 KiB–4 MiB default encoding took 0.91–0.98× the wide build's
time, and the other cells 1.00–1.03×. Other targets do not use these kernels. Wide is the default.

## Base58Check cost

`cargo bench --features check --bench check` compares checked buffer conversion
with plain conversion of the same complete frame, with and without separate
hashing and validation; repeat with `check,cache` or `--no-default-features
--features check`. On 2026-11-04 (M5, default) a 21-byte payload took 14 ns to
convert and about 53 ns checked: double SHA-256 dominates small checked calls.

Enabling `check` does not change plain conversions measurably. The 2026-11-04/05
paired public-consumer review (M5 and Zen 5, unpinned and aligned builds) put
plain calls with `check` enabled at 0.98–1.01× on aligned Zen 5 decode; the
unpinned ±20% swings at 4–32 KiB appeared with `check` on or off, so they are
placement. Plain encode machine code is identical with and without `check`.

## Optimization log

Adopted (paired runs; ratios are after/before time):

| Date | Change | Effect |
| --- | --- | --- |
| 2026-10-29 | Whole-limb formatter (#1) | 16–128 B encode 0.83–0.94 (M5), 0.91–0.95 (EPYC native) |
| 2026-10-29 | Radix-`58^3` tree on NEON (#2), reused scratch | encode 8 KiB–4 MiB 0.94–0.98; 1 MiB allocations 65,590 → 52 |
| 2026-10-29 | Pooled radix scratch | requests 54 → 14 at 1 MiB, peak −11%; timing neutral |
| 2026-10-29 | 16-byte-aligned fixed tables | cached 32 B encode 38.2 → 12.0 ns (M5) |
| 2026-11-01 | Bounded inverse roots, smaller scratch (#7) | 4 MiB peak encode 132 → 70 MB, decode 110 → 57 MB (M5); EPYC 4 MiB time 0.93–0.97 |
| 2026-11-05 | Fixed-width batched tail stores (#18) | 21–82 B decode 0.90–0.99 (M5) |
| 2026-11-05 | Radix carry as four interleaved chains | encode ≥ 8 KiB 0.84–0.89 (M5) |
| 2026-11-05 | Prepared decode powers square from stored transforms | decode ≥ 8 KiB 0.95–0.97 (M5); cached ≥ 256 KiB 0.98 (Zen 5) |
| 2026-11-05 | One radix rule with and without `cache` | cached encode 16 KiB–1 MiB 0.79–0.88; default 4097/8191 B 0.78/0.74 (M5) |

Tried and not adopted:

- One overlapping 8-byte tail store instead of fixed-width stores: 0.93–0.95 at
  21/25 B but 1.02–1.04 at 32–82 B on Zen 5, 0.98–1.03 on M5.
- 64-byte-aligned Matrix decoder scratch: 1.00–1.02 at 512 B–4 KiB cached decode (M5).
- Eight carry chains instead of four: 1.00–1.01 (M5).
- Padding nearby lengths into the fixed 32/64-byte kernels regressed zero-heavy
  inputs; portable fixed decoders were slower than the general decoder (M5).
- The paired AVX-512 inverse NTT kernel: 2–5% per butterfly, nothing end to end.

## Open levers

Unmeasured candidates from the 2026-11-05 review, largest expected effect first:

- M5: start the radix route below 4 KiB. Batched encoding takes 112 µs at 4096 B;
  the radix route takes 78 µs at 4097 B.
- Radix tree: fuse the first forward stage, whose upper half is always zero
  (about 2–3% of encode); build sparse schoolbook products column by column
  (sparse 1 MiB inputs need 6% more heap); share the digit scratch with the output
  (about 11% of the 4 MiB peak).
- NTT: clip the scratch envelope at the prepared length (−3.5 MiB at a 4 MiB
  decode) and reserve the cyclic sums only when used (−2 MiB); quantize cached
  twiddle classes (−49% worst-case retention, larger first builds).
- Division encoder: 5 allocations per call down to 1–2; reuse the dead quotient
  buffer during the remainder product (−10–14% node scratch); non-power-of-two
  wrapped remainders; a scaled remainder tree; Karp–Markstein at the top node.
- Matrix: build table panels as a wavefront (3–6× faster first cached calls);
  vector digit conversion in the general x86 encoder; retune the 8192-byte cached
  encode cap on AVX2-only x86.

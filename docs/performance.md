# Performance and resource tradeoffs

The [README tables](../README.md#performance) show the public-API timings and
linked sizes. Auto trades workspace for large-input speed; disabling `alloc`
avoids internal allocation, and its general large conversions remain quadratic.
[DESIGN.md](../DESIGN.md) describes dispatch and algorithms;
[CORRECTNESS.md](../CORRECTNESS.md) describes validation and remaining gaps.

## Results

Measured **2026-11-07** on this revision with ordinary public consumers:
`code_size.py` cdylibs that call `Encoder::bitcoin().encode(..).onto(..)` and
`Decoder::bitcoin().decode(..).onto(..)`, with no `__internals`. Release opt-level
3, fat LTO, 16 codegen units, generic target flags (`RUSTFLAGS=""`). Default and
`cache` are separate builds; no-allocation uses `default-features = false`; bs58
0.5.1 and five8 1.0.0 use their defaults.

| Platform | Compiler / runtime | Clock, sampling |
| --- | --- | --- |
| Apple M5 Pro, macOS 26.7.2, AC power | Rust 1.99.1 / LLVM 23.1.1 | process CPU; 5 processes, one file per library |
| GCP AMD EPYC 9B45 (Zen 5, 4 vCPU), Linux, CPU 0 pinned | Rust 1.99.0 / LLVM 23.1.1 | process CPU; 3 processes, 4 fresh copies of each library file |
| Scalar Wasm on the M5 | Rust 1.99.1; Node 26.13.0 / V8 14.6.202.34 | V8 elapsed; 3 processes |

Each fresh process times every library, interleaved, in three batches of at least
10 ms after calibration. A cell is the median of the process medians; figure bars
span the process medians. On Zen 5 every process times the four copies of each
library as separate libraries; a cell is the median over copies of each copy's
process median, and figure bars span the process medians of all copies ([Timing
limits](#timing-limits) explains why). Zen 5 timing was two runs back to back, each
with its own copies: up to 64 KiB with all libraries, then 128 KiB–4 MiB with Auto
only. Fixtures are random bytes without leading zeros. Outputs and round trips are
checked outside timing, decode input is prevalidated UTF-8, and caller buffers are
reused (decode output is exact). Workspace allocation is inside the timing.
No-allocation and bs58 stop at 64 KiB; five8 covers 32/64 B. The M5 runs had a
security agent using about one core in the background.

Zen 5 cells whose four copies differ by more than 5% (median [fastest, slowest copy
median]). All are default Auto, and in each the median lies within 1.9% of the
fastest copy, so one slow copy makes the spread. Which copy was slow was not
recorded. Every other Zen 5 cell's copies agree within 4.6%:

| Payload | Encode | Decode |
| --- | ---: | ---: |
| 1 KiB | 7.72 µs [7.58, 7.97] | — |
| 2 KiB | 18.8 µs [18.5, 19.7] | — |
| 4 KiB | 49.8 µs [49.6, 56.0] | 22.6 µs [22.4, 29.6] |
| 8 KiB | 146 µs [144, 156] | 56.9 µs [56.4, 77.2] |
| 16 KiB | 437 µs [433, 466] | 156 µs [155, 206] |
| 32 KiB | 1.29 ms [1.27, 1.35] | 449 µs [447, 572] |
| 64 KiB | — | 1.20 ms [1.20, 1.47] |
| 128 KiB | 7.21 ms [7.20, 9.08] | — |
| 256 KiB | 15.3 ms [15.2, 18.8] | — |
| 512 KiB | 32.5 ms [32.5, 39.2] | — |
| 1 MiB | 71.3 ms [71.1, 84.2] | — |
| 2 MiB | 155 ms [154, 180] | — |
| 4 MiB | 333 ms [333, 383] | — |

Ranges for 4 MiB Auto (median [min, max] of process medians; on Zen 5 over all
copies):

| Platform / policy | Encode | Decode |
| --- | ---: | ---: |
| M5 / default | 219.9 ms [218.5, 222.7] | 136.6 ms [135.6, 138.7] |
| M5 / cache | 218.5 ms [217.3, 218.9] | 134.7 ms [133.8, 136.1] |
| x86-64 / default | 333.1 ms [332.0, 384.0] | 144.1 ms [143.4, 144.7] |
| x86-64 / cache | 309.8 ms [309.1, 311.1] | 143.7 ms [143.3, 144.7] |
| Wasm / default | 1.720 s [1.711, 1.729] | 842.3 ms [841.5, 845.2] |
| Wasm / cache | 1.720 s [1.715, 1.723] | 838.2 ms [837.9, 847.3] |

Default Auto encoding around the fixed 32/64-byte routes:

| Payload | M5 | x86-64 | Wasm |
| --- | ---: | ---: | ---: |
| 16 B | 13.8 ns | 25.9 ns | 22.1 ns |
| 21 B | 14.9 ns | 27.9 ns | 27.2 ns |
| 22 B | 16.9 ns | 31.1 ns | 30.7 ns |
| 31 B | 24.9 ns | 43.4 ns | 49.3 ns |
| 32 B | 11.9 ns | 30 ns | 25.6 ns |
| 33 B | 24.7 ns | 43.3 ns | 49.8 ns |
| 63 B | 57.2 ns | 103 ns | 143 ns |
| 64 B | 23.9 ns | 31.4 ns | 108 ns |
| 65 B | 58.8 ns | 108 ns | 158 ns |
| 128 B | 152 ns | 323 ns | 523 ns |

Not measured: cold calls, owned-output helpers, checked conversion, native CPU
flags, other compilers and `.text` layout shifts ([Timing limits](#timing-limits)).

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
`--cpu 0` to `public_bench.py` to pin workers. On Zen 5 also add `--copies 4`:
`public_bench.py` then copies each library file four times into its temporary
directory and times the copies, interleaved like separate libraries, instead of the
file itself. A copy's value is its median over processes; a cell is the median over
copies, the CSV adds `copy_min_ns` and `copy_max_ns`, and the run prints each cell
whose copies differ by more than 5%. Before a host times one file per library,
check with `--copies 3` that copies of the Auto probes agree ([Timing
limits](#timing-limits)). For Wasm, build with `--target
wasm32-unknown-unknown` and time with `public_bench.py --wasm`; it runs Node with
`--no-liftoff --no-wasm-lazy-compilation --no-wasm-async-compilation` and refuses
allocation-counting modules. Render the README figures and tables from the CSVs and
each cohort's `sizes.csv`:

```sh
python3 -B assets/graphs/render_current.py --data m5.csv gcp.csv wasm.csv --sizes m5=m5-default/sizes.csv m5+cache=m5-cache/sizes.csv ... --output NEW_DIRECTORY
```

Rendering needs the pinned Matplotlib in `assets/graphs/requirements.txt`.
`python3 -B assets/graphs/render.py --check` validates links and figure syntax
only; it does not reproduce numbers.

Paired comparisons: build both variants' Auto probes, then pass both to
`public_bench.py` with `--baseline LABEL` (more `--processes` and `--batches` help
for small effects). It prints each cell's median and range of per-process
ratios against the baseline with the same cache policy; with `--copies`, each
process's time for a library is its median over copies. To see what a change
inlined or moved out of line, compare the two unstripped probes:
`python3 -B scripts/check_simd_inlining.py --compare OLD NEW` (sizes run to the
next symbol, so they include alignment padding).

Internal backends and allocations: `scripts/resource_report.py build` / `run`
times internal workers (`__internals`, not ordinary public consumers) and counts
allocations with `--count-allocations` in separate, untimed builds;
`scripts/wasm_allocations.mjs` counts Wasm allocations. The Criterion benches
(`benches/encode.rs`, `decode.rs`, `check.rs`, `dispatch.rs`) compare backends and
comparators in one process with wall time. `benches/encode_tune.rs` and
`encode_matrix_tune.rs` tune the encoder leaves, `MATRIX_MIN` and `IFMA_CAP`.

When the code changes, remeasure the affected tables, figures and scope together,
or withdraw them.

## Timing limits

Code placement can move a cell more than many source changes do. On Zen 5 (GCP EPYC
9B45, Auto probes):

- byte-identical copies of one library file can run at different speeds: one file
  took 1.87× as long as each of three copies on a 32 KiB decode, and 1.2× on an
  8 KiB encode. Which file is slow changes between files and VM sessions (page
  placement; the cause is not known). The Zen 5 results are therefore medians over
  four fresh copies of each file; the file that `code_size.py` wrote is not timed.
  In them, one copy of default Auto took up to 1.37× as long as the fastest
  ([Results](#results));
- a loop whose closing branch straddles a 64-byte line ran 1.2–1.3× slower. The
  paired multiply-add loops take four limbs per iteration for this reason; across
  16-byte shifts of all code (0–112 bytes in front of `.text`), their decode cells
  still moved by up to 5% (4–8 KiB). The published builds have one layout: layout
  shifts were not sampled, because `code_size.py` has no option to pad `.text`, so
  that residual remains in the Zen 5 cells;
- loading the same library at another address can move a 1 KiB encode by 12%.

On the M5, three fresh copies of each Auto probe agreed within 1.5% from 512 B to
64 KiB (2026-11-07, five processes), so the M5 results time one file per library.
LLVM function and loop alignment options moved M5 cells by up to 7%. Wasm modules
run code that V8 compiles at load time, so file copies do not apply to them.
Therefore:

- compare variants only within one paired run (`--baseline`); on Zen 5 also pass
  `--copies` and compare medians over copies;
- judge a change by the cells its code reaches, and treat movement in untouched
  cells as the noise floor;
- for effects below about 5%, add an aligned build
  (`-C llvm-args=-align-all-functions=6 -C llvm-args=-align-loops=64`) as a
  diagnostic; it is not the shipped configuration.

Background load on a laptop can make one process of a cell slow (a bimodal spread);
rerun with more processes rather than dropping observations.

## basexx comparison

`basexx` 0.2.0 is a benchmark comparator: its `Base58` is much faster than bs58.
Criterion medians (wall time, one run, default Criterion settings) on the M5,
2026-11-07, generic flags, fat LTO, `__internals`, cache off. basexx returns an
owned result that the adapter copies and drops inside the loop; bs58 and Auto reuse
caller output.

| Operation / payload | Auto | bs58 0.5.1 | basexx Base58 | basexx Base58B |
| --- | ---: | ---: | ---: | ---: |
| Encode / 1 KiB | 7.12 µs | 1.00 ms | 22.4 µs | 788 µs |
| Encode / 4 KiB | 109 µs | 15.3 ms | 185 µs | 12.7 ms |
| Decode / 1 KiB | 1.86 µs | 317 µs | 5.22 µs | 301 µs |
| Decode / 4 KiB | 35.3 µs | 5.13 ms | 68.0 µs | 4.87 ms |

Reproduce with `cargo bench --features __internals --bench encode` (or `decode`),
filtering `(bs58|basexx|basexx-b|auto)/(1024|4096)$`.

## Cache and memory

Enabling `cache` changes dispatch and leaf policy as well as retention. Maximum
retained payloads are in [DESIGN.md](../DESIGN.md#heap-stack-and-cache-costs):
Matrix tables 18,617,760 B and NTT twiddles 9,436,416 B (26.8 MiB combined,
depending on call history), plus 2,176 B of static headers. Per-call scratch,
larger owned tables, allocator overhead and concurrent calls add to that; it is not
an active-heap bound. Wasm linear-memory pages need not return to the host. First
cached calls build the tables they need; whether reuse repays that depends on the
workload.

Warm peaks on the M5 (2026-11-07, counted `__internals` Auto workers, caller buffers
and previously retained tables excluded; three repeats agree exactly):

| Payload | Policy | Operation | Warm requests | Warm peak | First call retains |
| --- | --- | --- | ---: | ---: | ---: |
| 1 MiB | default | encode | 7 | 17,788,292 B | 0 B |
| 1 MiB | default | decode | 16 | 14,781,952 B | 0 B |
| 1 MiB | cache | encode | 7 | 17,788,292 B | 0 B |
| 1 MiB | cache | decode | 13 | 10,849,792 B | 4,090,048 B |
| 4 MiB | default | encode | 7 | 69,575,396 B | 0 B |
| 4 MiB | default | decode | 18 | 56,783,096 B | 0 B |
| 4 MiB | cache | encode | 7 | 69,575,396 B | 0 B |
| 4 MiB | cache | decode | 18 | 56,783,096 B | 616,704 B |

Requests count allocations plus reallocations; peaks are requested Rust layouts,
not RSS, allocator usable sizes or stack high water.

## Linked size and compact

The [README size table](../README.md#linked-size) subtracts each cohort's empty
consumer (same scaffold, no conversion call) from cdylibs with both directions
reachable. Native columns are code and readonly sections; Wasm columns are CODE and
DATA sections. Initialized data, BSS, file size and linear-memory pages are
different costs. `CHAR_PAIRS` (3,364 two-byte entries, 6,728 B) is in readonly data
whenever a reachable path uses it.

On x86-64, `compact` keeps eight IFMA kernel widths instead of twelve: less code,
more chunks for some multiplications. Other targets do not use these kernels; wide
is the default.

## Base58Check cost

`cargo bench --features check --bench check` compares checked buffer conversion with
plain conversion of the same complete frame, with and without separate hashing and
validation; repeat with `check,cache` or `--no-default-features --features check`.
Double SHA-256 adds a fixed cost to every checked call.

## Open levers

Unmeasured candidates, largest expected effect first:

- M5: start the radix route below 4 KiB, where batched encoding still runs.
- M5: default Auto decoding at 4 KiB takes 1.32× as long as the no-allocation
  (batched) decoder in the [results](../README.md#apple-m5-pro); at 8 KiB Auto is
  faster. Retune the divide-and-conquer decoding cutoff there.
- Radix tree: fuse the first forward stage, whose upper half is always zero
  (about 2–3% of encode); build sparse schoolbook products column by column
  (sparse inputs need more heap); share the digit scratch with the output (about
  11% of the 4 MiB peak).
- NTT: clip the scratch envelope at the prepared length (−3.5 MiB at a 4 MiB
  decode) and reserve the cyclic sums only when used (−2 MiB); quantize cached
  twiddle classes (−49% worst-case retention, larger first builds).
- Division encoder: 5 allocations per call down to 1–2; reuse the dead quotient
  buffer during the remainder product (−10–14% node scratch); non-power-of-two
  wrapped remainders; a scaled remainder tree; Karp–Markstein at the top node.
- Matrix: build table panels as a wavefront (faster first cached calls); vector
  digit conversion in the general x86 encoder; retune the 8192-byte cached encode
  cap on AVX2-only x86.

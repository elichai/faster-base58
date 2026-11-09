# Performance and resource tradeoffs

The [README tables](../README.md#performance) show the public-API timings and
linked sizes. Auto trades workspace for large-input speed; disabling `alloc`
avoids internal allocation, and its general large conversions remain quadratic.
[DESIGN.md](../DESIGN.md) describes dispatch and algorithms;
[CORRECTNESS.md](../CORRECTNESS.md) describes validation and remaining gaps.

## Results

Measured **2026-11-09** with ordinary public consumers:
`code_size.py` cdylibs that call `Encoder::bitcoin().encode(..).onto(..)` and
`Decoder::bitcoin().decode(..).onto(..)`, with no `__internals`. Release opt-level
3, fat LTO, 16 codegen units, generic target flags (`RUSTFLAGS=""`). Default and
`cache` are separate builds; no-allocation uses `default-features = false`; bs58
0.5.1 and five8 1.0.0 use their defaults.

| Platform | Compiler / runtime | Clock, processes |
| --- | --- | --- |
| Apple M5 Pro, macOS 26.7.2, AC power | Rust 1.99.1 / LLVM 23.1.1 | process CPU, 5 |
| GCP AMD EPYC 9B45 (Zen 5, 4 vCPU), Linux 6.1, vCPU 1 pinned, [protocol](#measure-and-update) | Rust 1.99.0 / LLVM 23.1.1 | process CPU, 3 × 4 copies |
| Scalar Wasm on the M5 | Rust 1.99.1; Node 26.13.0 / V8 14.6.202.34 | V8 elapsed, 5 |

Each fresh process times every library, interleaved, in three batches of at least
10 ms after calibration. A cell is the median of the process medians (on Zen 5, the
median over copies of each copy's process median); figure bars span the process
medians. Fixtures are random bytes without leading zeros. Outputs and round trips
are checked outside timing, decode input is prevalidated UTF-8, and caller buffers
are reused (decode output is exact). Workspace allocation is inside the timing.
No-allocation and bs58 stop at 64 KiB; five8 covers 32/64 B.

On Zen 5 the copies of one Auto file spread by more than 5% in default decoding from
1 KiB to 32 KiB (copy max/min 1.05–1.16, largest at 2 KiB): code placement moves the
IFMA Karatsuba decoder.

Ranges for 4 MiB Auto (median [min, max] of process medians):

| Platform / policy | Encode | Decode |
| --- | ---: | ---: |
| M5 / default | 212 ms [210 ms, 215 ms] | 129 ms [128 ms, 134 ms] |
| M5 / cache | 210 ms [208 ms, 213 ms] | 129 ms [127 ms, 133 ms] |
| x86-64 / default | 227 ms [226 ms, 228 ms] | 119 ms [119 ms, 122 ms] |
| x86-64 / cache | 227 ms [226 ms, 228 ms] | 116 ms [116 ms, 117 ms] |
| Wasm / default | 1.61 s [1.59 s, 1.63 s] | 773 ms [765 ms, 792 ms] |
| Wasm / cache | 1.61 s [1.59 s, 1.65 s] | 768 ms [766 ms, 774 ms] |

Default Auto encoding around the fixed 32/64-byte routes:

| Payload | M5 | x86-64 | Wasm |
| --- | ---: | ---: | ---: |
| 16 B | 14 ns | 24.8 ns | 21.3 ns |
| 21 B | 15.4 ns | 28.1 ns | 27.6 ns |
| 22 B | 17 ns | 29.9 ns | 30.3 ns |
| 31 B | 24.4 ns | 42.3 ns | 50.9 ns |
| 32 B | 12 ns | 30 ns | 26.1 ns |
| 33 B | 23.7 ns | 42 ns | 50.6 ns |
| 63 B | 59.2 ns | 102 ns | 146 ns |
| 64 B | 24.1 ns | 31.2 ns | 101 ns |
| 65 B | 58.2 ns | 105 ns | 160 ns |
| 128 B | 152 ns | 313 ns | 526 ns |

Not measured: cold calls, owned-output helpers, checked conversion, native CPU
flags and other compilers.

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
`--cpu N` to `public_bench.py` to pin workers. Native workers place every input and
output buffer at a 64-byte boundary.

On the GCP Zen 5 VM (`c4d-highcpu-4`: vCPUs 0/2 and 1/3 are SMT siblings), time on
vCPU 1 and keep vCPU 3 idle:

- before and after each run, the metadata server's `instance/maintenance-event`
  must read `NONE`;
- run a 10-second unmeasured busy loop on vCPU 1 first;
- start `public_bench.py --cpu 1 --copies 4` under `sudo chrt -f 99`, `setarch -R`
  (no address randomization) and `prctl(PR_SET_THP_DISABLE)`; the workers inherit
  all three, and no system setting changes;
- sample `vmstat -t 1` on vCPU 0 and discard a run with any steal above 0.

The guest has no `cpufreq` driver (the host sets the frequency) and already uses the
`tsc` clocksource. SCHED_FIFO tasks are still throttled to 950 ms per second. Report
a cell whose process or copy medians spread by more than 3% with its range instead
of dropping it: on Zen 5 such spread usually comes from code placement, which is
real variation between builds.

For Wasm, build with `--target
wasm32-unknown-unknown` (add `--wasm-simd128` for the SIMD cohort) and time with
`public_bench.py --wasm`; it runs Node with
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
ratios against the baseline with the same cache policy. To see what a change
inlined or moved out of line, compare the two unstripped probes:
`python3 -B scripts/check_simd_inlining.py --compare OLD NEW` (sizes run to the
next symbol, so they include alignment padding).

Internal backends and allocations: `scripts/resource_report.py build` / `run`
times internal workers (`__internals`, not ordinary public consumers) and counts
allocations with `--count-allocations` in separate, untimed builds;
`scripts/wasm_allocations.mjs` counts Wasm allocations. The Criterion benches
(`benches/encode.rs`, `decode.rs`, `check.rs`, `dispatch.rs`) compare backends and
comparators in one process with wall time. `benches/encode_tune.rs` and
`encode_matrix_tune.rs` tune the encoder leaves, `MATRIX_MIN` and `IFMA_CAP`;
`benches/radix_tune.rs` times the forced radix and division trees in one binary
(with a division A/A arm) for the [radix utilization rule](#radix-route-utilization).
`benches/threshold_tune.rs` sweeps the dispatch crossovers and the NTT and IFMA
product thresholds in one binary; its internal arms fall back below their own
minimums (divide-and-conquer decoding needs two leaves). Constants without a runtime
parameter, such as leaf sizes and the cutoffs in `src/api.rs`, are compared as rebuilt
Auto probes with `public_bench.py --baseline`; on Zen 5 the rebuild alone moved
untouched cells by up to 7%, so judge only the cells the change reaches.

When the code changes, remeasure the affected tables, figures and scope together,
or withdraw them.

## Timing limits

Code placement can move a cell more than many source changes do. On Zen 5 (GCP EPYC
9B45, 2026-11-07, Auto probes):

- a loop whose closing branch straddles a 64-byte line ran 1.2–1.3× slower, so a
  16-byte shift of unrelated code moved such cells by that much;
- the placement of a library file's pages matters as well: in one run the original
  probe file took 1.87× as long as each of three byte-identical copies on a 32 KiB
  decode, and 1.2× on an 8 KiB encode, so the same binary can change speed between
  files and VM sessions;
- loading the same library at another address can move a 1 KiB encode by 12%.

On the M5, LLVM function and loop alignment options moved cells by up to 7%.
Therefore:

- compare variants only within one paired run (`--baseline`); on Zen 5 also pass
  `--copies 4`, which times fresh copies of each library file and compares
  medians over copies;
- judge a change by the cells its code reaches, and treat movement in untouched
  cells as the noise floor;
- for effects below about 5%, add an aligned build
  (`-C llvm-args=-align-all-functions=6 -C llvm-args=-align-loops=64`) as a
  diagnostic; it is not the shipped configuration.

Background load on a laptop can make one process of a cell slow (a bimodal spread);
rerun with more processes rather than dropping observations.

## Wasm SIMD128

The published Wasm cohort is scalar. Builds with `-C target-feature=+simd128` are a
separate cohort that needs a SIMD-capable engine. Measured **2026-11-07** on the M5
Pro (AC power), Node 26.13.0 / V8 14.6.202.34, Rust 1.99.1: `code_size.py
--wasm-simd128` Auto probes (generic CPU, fat LTO, default and `cache` builds),
`public_bench.py --wasm`, five processes, three batches, V8 elapsed time. Ratios are
medians [range] of per-process ratios against the same build without the Wasm
kernels (portable kernels, autovectorized):

| Payload | Default encode | Default decode | Cache encode | Cache decode |
| --- | ---: | ---: | ---: | ---: |
| 64 B | 0.56 [0.55, 0.57] | 0.96 [0.86, 1.04] | 0.58 [0.57, 0.59] | 0.99 [0.91, 1.06] |
| 128 B | 1.00 [0.99, 1.02] | 1.00 [0.92, 1.02] | 0.50 [0.50, 0.51] | 1.00 [0.93, 1.08] |
| 1 KiB | 0.99 [0.98, 1.01] | 0.97 [0.84, 1.01] | 0.29 [0.28, 0.29] | 0.33 [0.32, 0.33] |
| 4 KiB | 0.99 [0.97, 1.01] | withdrawn | 0.26 [0.26, 0.27] | 0.27 [0.27, 0.28] |
| 8 KiB | withdrawn | withdrawn | withdrawn | withdrawn |
| 64 KiB | withdrawn | withdrawn | withdrawn | withdrawn |
| 4 MiB | withdrawn | withdrawn | withdrawn | withdrawn |

The NTT rows and the 4 MiB SIMD128 times are withdrawn until remeasured: the NTT's
inverse transform and Garner step changed in both arms. The 4 KiB default decode
cell is withdrawn too: Wasm now decodes it with the NTT decoder instead of Karatsuba.
Cells that reach no Wasm kernel (batched sizes, fixed 32 bytes) stay within
0.95–1.05. The stripped Auto probe grows by 414 bytes (default) and 1,447 bytes
(cache). In the same run, `+simd128` without the explicit kernels took 1.24–1.30
times scalar time for cached Matrix sizes of 512 B–8 KiB; its NTT-size ratio is
withdrawn with the rows above.

## basexx comparison

`basexx` 0.2.0 is a benchmark comparator: its `Base58` is much faster than bs58.
Criterion medians (wall time) on the M5, 2026-11-09, generic flags, fat LTO,
`__internals`, cache off. basexx returns an owned result that the adapter copies
and drops inside the loop; bs58 and Auto reuse caller output.

| Operation / payload | Auto | bs58 0.5.1 | basexx Base58 | basexx Base58B |
| --- | ---: | ---: | ---: | ---: |
| Encode / 1 KiB | 7.24 µs | 1.02 ms | 23 µs | 843 µs |
| Encode / 4 KiB | 74.4 µs | 16.1 ms | 180 µs | 13.4 ms |
| Decode / 1 KiB | 1.91 µs | 336 µs | 5.51 µs | 314 µs |
| Decode / 4 KiB | 28.8 µs | 5.42 ms | 73.4 µs | 5.22 ms |

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

Warm peaks on the M5 (2026-11-09, counted `__internals` Auto workers, caller buffers
and previously retained tables excluded):

| Payload | Policy | Operation | Warm requests | Warm peak | First call retains |
| --- | --- | --- | ---: | ---: | ---: |
| 1 MiB | default | encode | 7 | 17,787,780 B | 0 B |
| 1 MiB | default | decode | 16 | 14,781,952 B | 0 B |
| 1 MiB | cache | encode | 7 | 17,787,780 B | 0 B |
| 1 MiB | cache | decode | 13 | 10,849,792 B | 4,090,048 B |
| 4 MiB | default | encode | 7 | 69,574,884 B | 0 B |
| 4 MiB | default | decode | 18 | 56,783,096 B | 0 B |
| 4 MiB | cache | encode | 7 | 69,574,884 B | 0 B |
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

## Radix route utilization

The radix route needs a used fraction of its root transform (base-`58^3` digit
bound over transform points) of 5/8, or 3/4 with `cache`. The fraction rises
linearly with the input from 0.48 just above each transform-length step
(`17 * 2^k + 1` bytes) to 0.97 below the next one.

Measured **2026-11-07** with `benches/radix_tune.rs`: one `__internals` binary per
feature set times the forced radix tree, the forced division tree and a division
A/A arm, in 15 rounds of interleaved blocks of at least 10 ms. A cell is the median
of the per-round radix/division ratios. Generic flags (`RUSTFLAGS=""`), fat LTO,
default and `cache` builds. M5 Pro: AC power, Rust 1.99.1, two runs, which agree
within 0.02. Zen 5 (EPYC 9B45, 4 vCPU): Rust 1.99.0, pinned to CPU 1, one run.
Process samplers showed no other load. A/A medians stay within 0.97–1.02 (M5) and
0.99–1.01 (Zen 5). Ranges cover the size medians, 34 KiB–4 MiB on M5 and
40 KiB–4 MiB on Zen 5:

| Used | M5 default | M5 cache | Zen 5 default | Zen 5 cache |
| ---: | ---: | ---: | ---: | ---: |
| 0.48 | 1.02–1.12 | 1.17–1.22 | 1.07–1.16 | 1.13–1.24 |
| 0.56 | 0.93–0.95 | 1.05–1.13 | 1.04–1.25 | 1.10–1.24 |
| 0.62 | 0.91–0.94 | 0.96–1.03 | 0.95–1.07 | 0.97–1.05 |
| 0.68 | 0.84–0.87 | 0.99–1.06 | 0.90–0.94 | 0.97–1.04 |
| 0.75 | 0.80–0.81 | 0.89–0.95 | 0.83–0.86 | 0.84–0.90 |
| 0.81 | 0.77–0.78 | 0.83–0.91 | 0.84–0.89 | 0.82–0.89 |
| 0.88 | 0.77 | 0.81–0.89 | 0.83–0.85 | 0.84–0.88 |
| 0.94 | 0.67–0.69 | 0.73–0.81 | 0.74–0.77 | 0.76–0.79 |

The Zen 5 cells above 1 at 0.62 are 44 KiB (below the x86 minimum) and 349 KiB
(1.02). On M5 at 8.5–34 KiB, default reaches 0.95–0.99 at 0.56 used and `cache`
0.98–1.03 at 0.75. At 4.25–8.5 KiB, default takes 1.01–1.02 at 0.62 and 0.88–0.89
at 0.68. With `cache`, cached Matrix leaves speed up only the division tree, which
moves its crossover from about 0.55–0.62 to about 0.7.

End to end (`public_bench.py --baseline`, Auto probes of d317ee4 and this rule,
same flags, five processes; Zen 5 `--cpu 0`): cells that change route take
0.78–0.87 (M5 default), 0.86–0.94 (M5 cache, except 1.03 at 13,495 B),
0.84–0.93 (Zen 5 default) and 0.87 (Zen 5 cache) of the earlier time. Cells that
keep their route, and all decode cells, stay within 0.978–1.014. Power-of-two sizes,
which the README tables use, keep their route.

## Open levers

Unmeasured candidates, largest expected effect first:

- Radix tree: fuse the first forward stage, whose upper half is always zero
  (about 2–3% of encode); build sparse schoolbook products column by column
  (sparse inputs need more heap); share the digit scratch with the output (about
  11% of the 4 MiB peak).
- NTT: clip the scratch envelope at the prepared length (−3.5 MiB at a 4 MiB
  decode) and reserve the cyclic sums only when used (−2 MiB); quantize cached
  twiddle classes (−49% worst-case retention, larger first builds); a forward
  transform with Cooley–Tukey butterflies and one twiddle per block, which could
  use the inverse's reduction schedule (bit-reversed twiddles for the small stages).
- Division encoder: 5 allocations per call down to 1–2; reuse the dead quotient
  buffer during the remainder product (−10–14% node scratch); non-power-of-two
  wrapped remainders; a scaled remainder tree; Karp–Markstein at the top node.
- Matrix: build table panels as a wavefront (faster first cached calls); vector
  digit conversion in the general x86 encoder; retune the 8192-byte cached encode
  cap on AVX2-only x86; retune the 1024-digit cached x86 decode start for
  AVX2 Matrix parsing.

# Performance and resource tradeoffs

The [README](../README.md#performance) shows the measured competing-crate
comparison near the top. [DESIGN.md](../DESIGN.md) describes algorithms,
dispatch and memory ownership; [CORRECTNESS.md](../CORRECTNESS.md) describes
validation and its limits.

## Public conversion results

This comparison cohort was measured **2026-11-09**, source
[`8cf24a72ad66`](https://github.com/elichai/faster-base58/commit/8cf24a72ad66435367b226f89a6a7c30ae496567).
The library has changed since this run. These are results for the recorded source;
fresh timings and linked sizes are being collected on M5 and GCP. The tables and
figures use the same complete cohort, including the competing crates.

Ordinary public consumers use
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
No-allocation and bs58 stop at 64 KiB; five8 covers 32/64 B. Figure bars show
observed ranges, not confidence intervals. Warm cache timings exclude first-use
table setup.

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

This cohort does not measure cold calls, owned-output helpers, Base58Check,
native CPU flags or other compilers.

![Public conversion times and competing crates on Apple M5 Pro](../assets/graphs/m5-public.svg)

![Public conversion times and competing crates on AMD EPYC 9B45](../assets/graphs/gcp-public.svg)

![Scalar Wasm conversion times and competing crates on Apple M5 Pro](../assets/graphs/wasm-public.svg)

The [README tables](../README.md#performance) list selected absolute encode/decode
medians from these figures. Native and Wasm clocks are separate measurement
scopes; their times do not establish a native/Wasm speed ratio.

## Linked size and compact

The [linked-size table](../README.md#linked-size) uses the same November 9 builds
as the figures. Sizes need to be refreshed for the latest source.

The [README size table](../README.md#linked-size) subtracts each cohort's empty
consumer (same scaffold, no conversion call) from cdylibs with both directions
reachable. Native columns are code and readonly sections; Wasm columns are CODE and
DATA sections. Initialized data, BSS, file size and linear-memory pages are
different costs. `CHAR_PAIRS` (3,364 two-byte entries, 6,728 B) is in readonly data
whenever a reachable path uses it.

On x86-64, `compact` keeps eight IFMA kernel widths instead of twelve: less code,
more chunks for some multiplications. Other targets do not use these kernels; wide
is the default.

## Cache and memory

`cache` changes dispatch and leaf policy as well as retention. First calls can
build tables. Code-derived capacities and ownership are in
[DESIGN.md](../DESIGN.md#heap-stack-and-cache-costs). Per-call scratch, owned tables,
allocator overhead and concurrent calls add to retention. Retained capacity is
not an active-heap bound. Wasm linear-memory pages need not return to the host.

Count allocations in separate, untimed builds with
`scripts/resource_report.py build --count-allocations`, then `run`. These internal
workers differ from ordinary public consumers. Requests count allocations plus
reallocations; requested-layout peaks are not RSS, allocator usable sizes or stack
high water. No counted-allocation results accompany this timing cohort.

## Measure and update

Use an idle host, AC power on laptops, and ignored `target/measurements/` output.
Build all artifacts before timing. On the shared VM, use `/mnt/bench`, wait for its
shared lock and pin the timing worker. Do not overlap our timing with builds/tests.
Record source, target, compiler, CPU flags, features, effective LTO, clock, fixture,
buffer ownership/alignment, warm/cold scope and sampling with every published run.

For example, build an ordinary public consumer, selecting the host's target:

```sh
CARGO_PROFILE_RELEASE_LTO=fat python3 -B scripts/code_size.py --source . --output target/measurements/native-default --target aarch64-apple-darwin --cpu generic --backends empty,auto,no-alloc,bs58,five8 --directions both --kinds cdylib
CARGO_PROFILE_RELEASE_LTO=fat python3 -B scripts/code_size.py --source . --output target/measurements/native-cache --target aarch64-apple-darwin --cpu generic --backends empty,auto --directions both --kinds cdylib --cache
```

On Linux use `x86_64-unknown-linux-gnu` and `--strip-tool strip`. Time with
`scripts/public_bench.py`, passing `LABEL=PATH`, `--platform`, `--source-commit`
and a new `--output` CSV. `auto` and `auto+cache` identify the two policies; on
Linux add `--cpu N`. Choose copies/processes/batches explicitly and retain their
ranges. Comparator results need matched fixtures, output ownership, build settings
and sampling; missing competitors must not acquire legend entries or estimates.

The generic renderer consumes public timing CSVs and matching size CSVs, and writes
only to a new directory. Install pinned `assets/graphs/requirements.txt`, then use:

```sh
python3 -B assets/graphs/render_current.py --data TIMINGS.csv --sizes m5=DEFAULT_SIZES.csv m5+cache=CACHE_SIZES.csv --output NEW_FIGURES
python3 -B assets/graphs/render.py --check
python3 -B -m unittest discover -s assets/graphs -p test_render.py
```

Generated CSV/JSON, logs, artifacts and one-off analysis stay outside commits.
Selected tables, SVGs and reusable rendering tools belong in the repository.
The link/SVG check does not verify numeric values or reproduce pixels. Check
published numbers against raw inputs and inspect rendered titles, axes, legends
and captions. README figures and this guide present standalone results; branch
comparisons and merge decisions belong in PR discussions.

The November 9 GCP protocol used the following controls. Apply and record
them explicitly when reproducing that cohort.

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

## Other measurement scopes

Scalar Wasm is included above. Wasm SIMD128, internal backends, dispatch thresholds,
allocation counting, Base58Check and compact configurations require separate,
source-bound cohorts. No current timings are claimed for those scopes here.
Wasm timing uses Node/V8 elapsed time, not native process CPU time. Build with
`--target wasm32-unknown-unknown`, optionally `--wasm-simd128`, and use
`scripts/public_bench.py --wasm`. Allocation-counting modules are excluded from
ordinary timing. Criterion uses wall time; some comparator adapters own their
results while Auto reuses caller buffers, so report that distinction.

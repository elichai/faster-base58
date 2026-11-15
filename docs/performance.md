# Performance and resource tradeoffs

The [README](../README.md#performance) shows a concise performance overview.
[DESIGN.md](../DESIGN.md) describes algorithms, dispatch and memory ownership;
[CORRECTNESS.md](../CORRECTNESS.md) describes validation and its limits.

## Native results

Latest recorded measurements: **2026-11-12**, source
[`3fcfbaff62f3257ddc6de357f4856cf6225eabf8`](https://github.com/elichai/faster-base58/commit/3fcfbaff62f3257ddc6de357f4856cf6225eabf8).
These tables and figures show absolute results for that measured library.
Subsequent encoder changes have not been remeasured. A current-source timing and
linked-size refresh is pending; these values must not be read as a new run of the
latest source.

| Host | Target / ISA | OS | Compiler |
| --- | --- | --- | --- |
| Apple M5 Pro | aarch64-apple-darwin / AArch64 | macOS 26.7.3 | Rust 1.99.1 (58cf8f9c3), LLVM 23.1.1 |
| GCP AMD EPYC 9B45, 4 vCPU | x86_64-unknown-linux-gnu / generic x86-64 with runtime SIMD dispatch | Linux | Rust 1.99.0 (b940084d7), LLVM 23.1.1 |

Release opt-level 3, 16 codegen units, empty RUSTFLAGS, default `alloc` or `cache`,
fat LTO or LTO off. No `check`, `__internals`, `compact`, allocation counter or
target-cpu=native flag is enabled. Public Auto encode and decode are both reachable.
Effective LTO comes from `CARGO_PROFILE_RELEASE_LTO=fat` or `off`; the generated
probe manifest text alone says fat and does not capture the environment override.

GCP uses a shared VM lock and vCPU 1. M5 cohorts start and finish on AC with no
reported thermal warning. Our timing phases contain no builds/tests. Background
VM load was not independently measured. The README figures show fat LTO; tables
below retain both LTO configurations separately.

### Warm timing method

Each result summarizes **5 fresh processes × 3 fresh file copies of the library
× 3 interleaved batches**, calibrated to at least 5 ms per batch. The recorded run
interleaves three binary groups; only the measured library's group is reported
here. Loading, calibration and call order are partially balanced, not independently
counterbalanced. Code/file placement can affect small timings.

Fixtures are deterministic random bytes with a nonzero leading byte. Input and
caller output buffers are 64-byte aligned and reused. The recorded runner checks
each label's encode/decode output before timing and poisons the expected output
region before each untimed check. Internal workspace allocation is timed. Loading,
fixtures, caller buffers and output checks are outside the timer.

The clock is process CPU time around repeated public calls inside the cdylib;
the Python/FFI boundary occurs once per batch. A process/copy cell is the median of
three batches. The reported absolute time is the median over copies of each copy's
median over processes. Figure bars span the minimum and maximum retained cell
medians over all processes and copies. They are observations, **not confidence
intervals**. Individual timed-batch values are not retained. No observations were
dropped or adjusted by a control ratio.

Warm repeat-decode uses an already validated UTF-8 fixture, so UTF-8 checking is
outside that timer. Base58 digit validation stays in the decoder. This run does
not measure invalid-input latency, owned outputs or Base58Check. Warm cache results
exclude first-use table setup. A warm median is not a cold-start prediction.

![Base58 encode and decode times on Apple M5 Pro](../assets/graphs/m5-public.svg)

![Base58 encode and decode times on AMD EPYC 9B45](../assets/graphs/gcp-public.svg)

### Complete warm timings

Each cell is an absolute **encode / decode** median. The tables retain all 27
decoded sizes, including zero and one byte; logarithmic figures omit zero.

<details>
<summary>Apple M5 Pro / fat LTO: all warm timings</summary>

| Payload | Default encode / decode | Cache encode / decode |
| --- | ---: | ---: |
| 0 B | 1.4 ns / 1.41 ns | 1.39 ns / 1.41 ns |
| 1 B | 1.75 ns / 1.71 ns | 1.75 ns / 1.69 ns |
| 32 B | 11.8 ns / 13.7 ns | 11.9 ns / 13.6 ns |
| 52 B | 46.3 ns / 22.9 ns | 47.1 ns / 23 ns |
| 64 B | 24.1 ns / 26 ns | 23.9 ns / 25.8 ns |
| 65 B | 59.4 ns / 26.8 ns | 66.7 ns / 26.4 ns |
| 128 B | 150 ns / 63.4 ns | 111 ns / 64 ns |
| 256 B | 485 ns / 160 ns | 220 ns / 160 ns |
| 512 B | 1.83 µs / 519 ns | 582 ns / 505 ns |
| 1 KiB | 6.89 µs / 1.8 µs | 1.94 µs / 1.65 µs |
| 1,504 B | 14.5 µs / 3.71 µs | 3.97 µs / 3.43 µs |
| 1,875 B | 22.3 µs / 5.5 µs | 5.95 µs / 5.1 µs |
| 2 KiB | 26.4 µs / 6.75 µs | 6.92 µs / 5.98 µs |
| 4 KiB | 73.9 µs / 26.1 µs | 29.1 µs / 24.6 µs |
| 8 KiB | 169 µs / 74.9 µs | 120 µs / 69.8 µs |
| 16 KiB | 379 µs / 180 µs | 370 µs / 171 µs |
| 24 KiB | 745 µs / 311 µs | 682 µs / 292 µs |
| 28 KiB | 792 µs / 376 µs | 777 µs / 353 µs |
| 32 KiB | 841 µs / 424 µs | 826 µs / 408 µs |
| 36 KiB | 1.39 ms / 533 µs | 1.14 ms / 506 µs |
| 48 KiB | 1.68 ms / 724 µs | 1.58 ms / 690 µs |
| 64 KiB | 1.87 ms / 982 µs | 1.83 ms / 951 µs |
| 128 KiB | 4.13 ms / 2.25 ms | 4.06 ms / 2.19 ms |
| 144 KiB | 6.79 ms / 2.78 ms | 5.89 ms / 2.68 ms |
| 256 KiB | 9.09 ms / 5.13 ms | 8.96 ms / 4.99 ms |
| 1 MiB | 43.6 ms / 25.8 ms | 43.5 ms / 25.3 ms |
| 4 MiB | 206 ms / 126 ms | 206 ms / 126 ms |

</details>

<details>
<summary>Apple M5 Pro / off LTO: all warm timings</summary>

| Payload | Default encode / decode | Cache encode / decode |
| --- | ---: | ---: |
| 0 B | 1.52 ns / 1.83 ns | 1.63 ns / 1.91 ns |
| 1 B | 1.58 ns / 2.23 ns | 1.86 ns / 2.27 ns |
| 32 B | 23.3 ns / 38.6 ns | 24.1 ns / 39.4 ns |
| 52 B | 104 ns / 66.7 ns | 48.6 ns / 67.4 ns |
| 64 B | 24.8 ns / 72 ns | 25.1 ns / 73.2 ns |
| 65 B | 121 ns / 72.4 ns | 72.4 ns / 74 ns |
| 128 B | 286 ns / 156 ns | 125 ns / 160 ns |
| 256 B | 735 ns / 354 ns | 248 ns / 362 ns |
| 512 B | 2.3 µs / 901 ns | 584 ns / 522 ns |
| 1 KiB | 7.76 µs / 2.51 µs | 1.95 µs / 1.67 µs |
| 1,504 B | 15.8 µs / 4.7 µs | 4.03 µs / 3.44 µs |
| 1,875 B | 23.9 µs / 6.73 µs | 5.98 µs / 5.11 µs |
| 2 KiB | 27.8 µs / 7.88 µs | 6.96 µs / 5.94 µs |
| 4 KiB | 78.2 µs / 27.6 µs | 29.3 µs / 25.1 µs |
| 8 KiB | 178 µs / 78.9 µs | 121 µs / 71.3 µs |
| 16 KiB | 392 µs / 188 µs | 397 µs / 173 µs |
| 24 KiB | 770 µs / 322 µs | 703 µs / 299 µs |
| 28 KiB | 819 µs / 390 µs | 823 µs / 363 µs |
| 32 KiB | 868 µs / 440 µs | 882 µs / 411 µs |
| 36 KiB | 1.43 ms / 553 µs | 1.17 ms / 516 µs |
| 48 KiB | 1.72 ms / 749 µs | 1.63 ms / 700 µs |
| 64 KiB | 1.92 ms / 1.02 ms | 1.95 ms / 962 µs |
| 128 KiB | 4.24 ms / 2.32 ms | 4.31 ms / 2.21 ms |
| 144 KiB | 6.93 ms / 2.86 ms | 5.98 ms / 2.73 ms |
| 256 KiB | 9.31 ms / 5.25 ms | 9.47 ms / 5.05 ms |
| 1 MiB | 44.6 ms / 26.1 ms | 45.7 ms / 25.6 ms |
| 4 MiB | 210 ms / 128 ms | 215 ms / 128 ms |

</details>

<details>
<summary>AMD EPYC 9B45 (GCP) / fat LTO: all warm timings</summary>

| Payload | Default encode / decode | Cache encode / decode |
| --- | ---: | ---: |
| 0 B | 2.65 ns / 1.82 ns | 2.66 ns / 1.94 ns |
| 1 B | 2.37 ns / 2.18 ns | 2.33 ns / 2.17 ns |
| 32 B | 30.1 ns / 24.4 ns | 30 ns / 23.2 ns |
| 52 B | 83.1 ns / 40.4 ns | 76.8 ns / 40.7 ns |
| 64 B | 31.1 ns / 44.8 ns | 31.5 ns / 45.1 ns |
| 65 B | 106 ns / 46.1 ns | 101 ns / 46.6 ns |
| 128 B | 311 ns / 107 ns | 164 ns / 107 ns |
| 256 B | 1.05 µs / 287 ns | 389 ns / 288 ns |
| 512 B | 3.18 µs / 854 ns | 1.08 µs / 857 ns |
| 1 KiB | 7.26 µs / 2.49 µs | 3.33 µs / 2.37 µs |
| 1,504 B | 12.1 µs / 4.33 µs | 6.79 µs / 5.09 µs |
| 1,875 B | 15.6 µs / 5.42 µs | 10.1 µs / 7.7 µs |
| 2 KiB | 17.8 µs / 6.04 µs | 11.7 µs / 8.97 µs |
| 4 KiB | 47.5 µs / 15.7 µs | 39.6 µs / 15.8 µs |
| 8 KiB | 142 µs / 44.9 µs | 119 µs / 44.9 µs |
| 16 KiB | 428 µs / 134 µs | 365 µs / 135 µs |
| 24 KiB | 813 µs / 262 µs | 693 µs / 266 µs |
| 28 KiB | 943 µs / 349 µs | 857 µs / 339 µs |
| 32 KiB | 1.01 ms / 407 µs | 920 µs / 380 µs |
| 36 KiB | 1.4 ms / 520 µs | 1.36 ms / 490 µs |
| 48 KiB | 1.97 ms / 786 µs | 2.07 ms / 736 µs |
| 64 KiB | 2.21 ms / 1.02 ms | 2.1 ms / 820 µs |
| 128 KiB | 4.82 ms / 2.13 ms | 4.43 ms / 1.92 ms |
| 144 KiB | 7.47 ms / 3.04 ms | 6.83 ms / 2.41 ms |
| 256 KiB | 10.3 ms / 5.23 ms | 9.67 ms / 4.43 ms |
| 1 MiB | 47.5 ms / 24 ms | 47.6 ms / 22.6 ms |
| 4 MiB | 221 ms / 120 ms | 220 ms / 116 ms |

</details>

<details>
<summary>AMD EPYC 9B45 (GCP) / off LTO: all warm timings</summary>

| Payload | Default encode / decode | Cache encode / decode |
| --- | ---: | ---: |
| 0 B | 2.67 ns / 2.68 ns | 2.43 ns / 2.67 ns |
| 1 B | 2.92 ns / 2.91 ns | 2.92 ns / 2.79 ns |
| 32 B | 34.9 ns / 71.2 ns | 34.7 ns / 70.5 ns |
| 52 B | 157 ns / 116 ns | 85.2 ns / 114 ns |
| 64 B | 44.6 ns / 134 ns | 45 ns / 132 ns |
| 65 B | 193 ns / 136 ns | 111 ns / 134 ns |
| 128 B | 486 ns / 277 ns | 174 ns / 276 ns |
| 256 B | 1.4 µs / 618 ns | 406 ns / 612 ns |
| 512 B | 4.4 µs / 1.49 µs | 1.11 µs / 1.49 µs |
| 1 KiB | 9.64 µs / 4.01 µs | 3.41 µs / 2.43 µs |
| 1,504 B | 15.5 µs / 6.62 µs | 6.98 µs / 5.26 µs |
| 1,875 B | 20 µs / 8.31 µs | 10.5 µs / 7.95 µs |
| 2 KiB | 22.6 µs / 9.11 µs | 12.1 µs / 9.27 µs |
| 4 KiB | 57.3 µs / 21.9 µs | 41.8 µs / 22 µs |
| 8 KiB | 159 µs / 57.8 µs | 124 µs / 58 µs |
| 16 KiB | 461 µs / 160 µs | 370 µs / 160 µs |
| 24 KiB | 863 µs / 299 µs | 707 µs / 305 µs |
| 28 KiB | 983 µs / 394 µs | 900 µs / 386 µs |
| 32 KiB | 1.07 ms / 460 µs | 970 µs / 430 µs |
| 36 KiB | 1.47 ms / 575 µs | 1.4 ms / 550 µs |
| 48 KiB | 2.04 ms / 862 µs | 2.1 ms / 813 µs |
| 64 KiB | 2.31 ms / 1.13 ms | 2.2 ms / 927 µs |
| 128 KiB | 5.01 ms / 2.34 ms | 4.63 ms / 2.14 ms |
| 144 KiB | 7.78 ms / 3.28 ms | 7 ms / 2.65 ms |
| 256 KiB | 10.7 ms / 5.63 ms | 10.1 ms / 4.88 ms |
| 1 MiB | 49.1 ms / 25.7 ms | 49.4 ms / 24.3 ms |
| 4 MiB | 226 ms / 126 ms | 227 ms / 124 ms |

</details>

## First-call decode

Separate fresh-process public decode calls use 15 processes and 2 copies of the
library at 1,408, 2,816, 5,632 and 32,768 Base58 characters. Input is `"2"` repeated
that many times. An independent Python bigint oracle checks exact returned length
and bytes. Outputs are poisoned before their clock starts. Loading, oracle
construction, caller buffers and poison writes are outside timing. First codec
setup, internal allocation and the single-call export's UTF-8 check stay inside.
Call order rotates. Absolute time is the median over copies of each copy's median
over processes. Short calls have coarse clock resolution and wide variation.

| Host / LTO / policy | Input chars | First decode |
| --- | ---: | ---: |
| m5 / fat / default | 1,408 | 5.5 µs |
| m5 / fat / default | 2,816 | 10.5 µs |
| m5 / fat / default | 5,632 | 30 µs |
| m5 / fat / default | 32,768 | 322 µs |
| m5 / fat / cache | 1,408 | 57.5 µs |
| m5 / fat / cache | 2,816 | 203 µs |
| m5 / fat / cache | 5,632 | 786 µs |
| m5 / fat / cache | 32,768 | 522 µs |
| m5 / off / default | 1,408 | 8.5 µs |
| m5 / off / default | 2,816 | 15 µs |
| m5 / off / default | 5,632 | 35 µs |
| m5 / off / default | 32,768 | 341 µs |
| m5 / off / cache | 1,408 | 64.5 µs |
| m5 / off / cache | 2,816 | 220 µs |
| m5 / off / cache | 5,632 | 886 µs |
| m5 / off / cache | 32,768 | 549 µs |
| gcp / fat / default | 1,408 | 28.3 µs |
| gcp / fat / default | 2,816 | 48.2 µs |
| gcp / fat / default | 5,632 | 66.3 µs |
| gcp / fat / default | 32,768 | 321 µs |
| gcp / fat / cache | 1,408 | 93.2 µs |
| gcp / fat / cache | 2,816 | 318 µs |
| gcp / fat / cache | 5,632 | 69.3 µs |
| gcp / fat / cache | 32,768 | 326 µs |
| gcp / off / default | 1,408 | 36.3 µs |
| gcp / off / default | 2,816 | 54.5 µs |
| gcp / off / default | 5,632 | 76.8 µs |
| gcp / off / default | 32,768 | 366 µs |
| gcp / off / cache | 1,408 | 106 µs |
| gcp / off / cache | 2,816 | 326 µs |
| gcp / off / cache | 5,632 | 78.6 µs |
| gcp / off / cache | 32,768 | 360 µs |

## Linked size and compact

The following absolute accounting uses the same artifacts as the timing results.
Every value is **Auto-both minus its matching empty-both consumer**, which removes
the probe scaffold.
Both directions are reachable. Code, readonly, initialized data and BSS are section
totals; stripped-file bytes include layout and metadata. Mach-O and ELF categories
are not directly interchangeable. Signed BSS values can be negative after empty
consumer subtraction. BSS movement is not retained-heap measurement; `.rlib` size
is not deployment size. Subsequent encoder changes require these sizes to be
refreshed before they describe the latest build.

| Host / LTO / policy | Code B | Readonly B | Initialized data B | BSS B | Stripped file B |
| --- | ---: | ---: | ---: | ---: | ---: |
| m5 / fat / default | 67,760 | 24,950 | 16 | 0 | 82,784 |
| m5 / fat / cache | 76,232 | 27,768 | 2,192 | 0 | 99,328 |
| m5 / off / default | 80,728 | 50,006 | 8 | 0 | 132,432 |
| m5 / off / cache | 84,760 | 58,019 | 2,184 | 0 | 132,464 |
| gcp / fat / default | 308,157 | 38,444 | 6,248 | -608 | 352,872 |
| gcp / fat / cache | 303,789 | 39,280 | 9,136 | -1,890 | 352,240 |
| gcp / off / default | 332,787 | 75,964 | 10,328 | -1,288 | 419,080 |
| gcp / off / cache | 324,707 | 72,072 | 11,464 | -542 | 408,248 |

`compact` selects eight instead of twelve x86 IFMA widths, trading linked code
for more chunks on some multiplication shapes. This cohort measures the default
twelve-width policy. Compact, no-allocation and Wasm sizes were not measured here.

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
CARGO_PROFILE_RELEASE_LTO=fat python3 -B scripts/code_size.py --source . --output target/measurements/native-default --target aarch64-apple-darwin --cpu generic --backends empty,auto --directions both --kinds cdylib
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

## Other measurement scopes

Wasm scalar/SIMD128, comparators, internal backends, dispatch thresholds,
allocation counting, Base58Check and compact configurations require separate,
source-bound cohorts. No current timings are claimed for those scopes here.
Wasm timing uses Node/V8 elapsed time, not native process CPU time. Build with
`--target wasm32-unknown-unknown`, optionally `--wasm-simd128`, and use
`scripts/public_bench.py --wasm`. Allocation-counting modules are excluded from
ordinary timing. Criterion uses wall time; some comparator adapters own their
results while Auto reuses caller buffers, so report that distinction.

# Performance measurements

The README figures and timing table show the public conversion modes from the
**2026-10-25 source snapshot**, before the later allocation changes, `no_std`
support and package rename. They do not measure the current renamed package.
`Auto` means the root encoding/decoding API; `Auto + cache` enables table retention
and its associated dispatch choices. `no_alloc` uses its separate public module.

## Timing scope

The input is the frozen [public-mode timing CSV](tradeoffs/2026-10-25/ac-power/timings.csv).
M5 native and Wasm samples come from the later laptop run; GCP samples retain the
earlier matched public-API cohorts. The detailed [timing report](tradeoffs/2026-10-25/ac-power/README.md)
preserves first-call measurements, every process range, fixtures and power records.

Each point is the median of three fresh-process medians. Bars show their observed
minimum and maximum, not confidence intervals. The six displayed payload sizes
are 32, 64, 256, 1,024, 4,096 and 65,536 bytes. Decoding sizes refer to decoded
payload bytes. Both axes are logarithmic. Lower time is better; there is no
significance test or fastest-on-every-input claim.

Measurements call raw `encode`/`decode` into reusable output buffers. They exclude
owned `String`/`Vec` allocations and the extra UTF-8 check in `encode_into`.
They use one deterministic fixture per size. Warm cached results exclude table
construction; first calls can be slower and retain heap memory.

| Host | Compiler/target | Clock |
| --- | --- | --- |
| Apple M5 Pro | Rust 1.99.1, `apple-m4` | Process CPU |
| AMD EPYC 9B45 VM on GCP | Rust 1.99.0, generic `x86-64` or `target-cpu=native` | Process CPU |
| Wasm on Apple M5 Pro | Node 26.12.1 / V8 14.6 | Elapsed |

The GCP instance was `c4d-highcpu-4`, with 4 vCPUs and 7 GiB RAM. Native timing
workers also link internal backends and an independent fixture oracle; standalone
consumer layout/inlining can differ. Wasm selects one public mode per module.
Native CPU and Wasm elapsed times are different measurements, not cross-host
speed ratios.

Cache-on/off are whole-configuration comparisons. They include dispatch and
compiled-artifact differences. Unchanged-implementation controls shift by up to
24.7% across artifacts in these cohorts. Compiler/layout effects or measurement
variation may contribute; source equality does not establish identical machine
code. Small differences need the recorded ranges. The slower cached GCP generic
4 KiB decode cell is retained in the README.

`no_alloc` and bs58 stop at 64 KiB by measurement policy, not an API or format
limit. The full report measures Auto through 4 MiB. Fixed-size five8 comparisons
remain in the [historical backend grids](tradeoffs/2026-10-24/benchmark-results/README.md).

## Linked size and later changes

The README size table uses final `v011` rows from the newer
[allocation linked-size CSV](tradeoffs/2026-10-25/allocation-layout/sizes.csv).
Those builds also predate `no_std` and the rename. Values are stripped file bytes
divided by 1,024, rounded to one decimal. Native rows are executables; Wasm rows
are `cdylib` modules. Each includes its runtime and scaffold. Comparisons within
a target preserve compiler, flags, artifact kind and harness. Padding can conceal
section changes; the full report keeps CODE/read-only/DATA/BSS, Wasm sections,
gzip, empty-scaffold deltas and artifact hashes.

The [allocation report](tradeoffs/2026-10-25/allocation-layout/README.md) records
the subsequent storage changes and separate paired public-consumer comparisons
on M5, both GCP profiles and Wasm. It is an incremental comparison, with mixed
timing changes and lower requested allocation/peak-heap costs on the recorded
instrumented paths. Its measurements are not substituted into the competitor
curves above.

## Figure and table checks

[render.py](../assets/graphs/render.py) uses only the Python standard library.
It selects 192 timing cells and 16 linked-size cells from the two frozen CSVs.
[data.json](../assets/graphs/data.json) retains full values and source row numbers;
[receipt.json](../assets/graphs/receipt.json) binds input, renderer and output
hashes. The basic check validates source cells, process ranges, all 192 plotted
points, generated SVGs and the README's table values:

```sh
rtk proxy python3 -B assets/graphs/render.py --check
```

This check renders no new measurements. The original raw-data audits, archived
artifacts and their scoped limits remain in the linked reports.

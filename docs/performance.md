# Performance and resource tradeoffs

The retained results describe specific source snapshots and workloads. They do
not establish one fastest backend for every size, input distribution or build.
[DESIGN.md](../DESIGN.md) describes the current dispatcher and backend costs;
[CORRECTNESS.md](../CORRECTNESS.md) describes verification coverage.

## Datasets and source generations

| Dataset | Coverage and interpretation |
| --- | --- |
| [Backend grid](benchmarks/backend-grid/README.md) | 2026-10-24 source: 29 decoded sizes from empty through 4 MiB; M5 native, GCP generic/native and M5 Wasm. Includes backend wrappers, forced-kernel controls, bs58 and five8. This predates the cache-feature split, no_std and rename. |
| [Backend resources](benchmarks/backend-resources/README.md) | Earlier 2026-10-23 timing, allocation and linked-size associations across five runtime configurations, including Wasm on GCP. Its cache policy was unbounded; its retained heap is historical. |
| [Public modes](benchmarks/public-modes/README.md) | 2026-10-25 Auto, Auto+cache, no_alloc and bs58 comparison. These exact timing inputs feed the README graphs. M5 timings were rerun on sampled AC power; cloud timings retain their matched earlier cohorts. |
| [Allocation layout](benchmarks/allocation-layout/README.md) | Storage refactor v000→v011, before no_std and rename. Includes normal-consumer public timings, separate instrumented allocation measurements and the final linked-size inputs used in the README. |
| [Migration controls](benchmarks/production/README.md) | Earlier no_std/rename comparison and source-identical rebuild/load-path controls. These do not isolate a source-level performance cause. |
| [Assembly](assembly.md), [SIMD](simd.md) | Newer baseline `7c6ffa325164`, resource identity `17ab5f264ed4`. Each experiment retains its own candidate, compiler and feature bindings. |

The backend grid has 3,480 result-or-skip cells and retains all 9,252 process
medians. Its readable tables bold the lowest observed median, including exact
ties. Bold type is not a statistical significance claim. Native workers link
the backends through `__internals`; historical names can share kernels or take
fallback routes. Direct backend rows therefore do not describe separate public
APIs. [Metadata](benchmarks/backend-grid/metadata.json) and
[explicit skips](benchmarks/backend-grid/skipped.csv) define each cell's scope.

The native grid uses Rust 1.99.x / LLVM 23.1.1. The GCP host is an AMD EPYC 9B45
VM, `c4d-highcpu-4`; generic builds retain runtime feature selection, while
native builds use `-C target-cpu=native`. The M5 grid uses
`-C target-cpu=apple-m4`. Wasm timings use Node/V8 and an elapsed clock.
Exact compiler versions, flags and runtime identities remain in each dataset.

Quadratic entries, including bs58 and the measured no_alloc mode, stop at
64 KiB by benchmark policy. This is not a format or API limit. Auto includes
4 MiB. five8 supports the measured 32/64-byte cases only; forced matrix controls
have a separate valid domain. Unsupported or policy-limited cells remain
explicit rather than being extrapolated.

## Read timing, heap and binary size separately

Native primary timings measure process CPU; Wasm primary timings measure elapsed
time. Cold, second-call and warm results are separate. The repeated conversion
loops use reusable caller buffers and exclude fixture construction, caller
allocation and owned String/Vec helpers. Prevalidated decode exports also
exclude the preceding UTF-8 scan. Three fresh-process medians and their ranges
describe the sampled variation; they are not confidence intervals. Primary
grids use one deterministic fixture per size. Independent native oracle checks
cover the large fixtures; the historical Wasm BigInt oracle covers sizes only
through 1 KiB. Error, zero and tail distributions need separate tests or controls.
Historical empty native decode used an exact-length `Vec`. On GCP its timing
depended strongly on that zero-length buffer's address; those rows do not
characterize general empty-input API latency.

Allocator probes count requested Rust layouts. A request is `alloc_calls +
realloc_calls`; `zeroed_calls` is a subset of `alloc_calls`. Requested peak bytes
exclude allocator overhead, RSS and stack. Reallocation hooks cannot observe an
allocator's internal overlap between old and new blocks. Caller input/output
allocations are [reported separately](benchmarks/allocation-layout/public-allocations/io-separate.csv).
The public allocation aliases are inside instrumented `__internals` workers;
they are not the standalone cdylibs used for the allocation-layout public timings.

For example, these are cold 4 MiB Auto encodes on the local allocation worker,
v000→v011. They show the accepted storage benefit, not current-process RSS:

| Feature | Requests before → after | Requested extra peak B before → after | Retained delta B after |
| --- | ---: | ---: | ---: |
| Default | 88 → 44 | 284,269,528 → 201,582,112 | 0 |
| cache | 3,168 → 46 | 294,775,616 → 214,372,960 | 14,401,152 |

[All sizes, directions and hosts](benchmarks/allocation-layout/public-allocations/comparisons.csv)
retain cold/warm distinctions. Cache capacity and per-call retained delta are
different columns. Cache headers have static storage; their backing tables are
heap allocations. The opt-in cache policy and its bounds are specified in
[DESIGN.md](../DESIGN.md); the earlier unbounded cache dataset must not be used
as its memory estimate.

Native code, read-only data, initialized data and BSS are separate from stripped
file size. Wasm CODE/DATA sections and 64 KiB linear-memory pages are different
again. A linked probe includes its runtime and scaffold; subtracting an empty
probe is useful but does not produce exact crate-only instruction bytes.
[Linked sizes](benchmarks/allocation-layout/sizes.csv) distinguish Auto,
Auto+cache, no_alloc and bs58. The
[all-backend size grid](benchmarks/backend-resources/linked-sizes.csv) retains
the earlier complete comparison. Reachable APIs, LTO, compiler and feature
unification can change an application's result.

## Decisions supported by the measurements

Matrix/Divisor storage pooling and earlier temporary drops reduced allocation
requests and some peaks. Their paired timings were mixed, so the accepted
benefit is resource behavior. Exact 32/64-byte paths remain distinct from the
rejected idea of padding arbitrary nearby lengths into fixed tables. Toom-3
remains experimental. Broader threshold-state plumbing, six NTT loop rewrites
and explicit Wasm panel prototypes were rejected; their measured costs and
controls are retained in the [decision ledger](benchmarks/decisions/decisions.json).

Large apparent migration slowdowns were not attributable to no_std, the rename
or a constant table: an unchanged ELF copied to a different inode/path also ran
faster. The [literal-copy controls](benchmarks/production/literal-copy-controls.csv)
preserve that result. Loader, page and heap context remain unresolved; no PMU
measurement established a cause. Small differences need unchanged-route and
literal-artifact controls before they justify a runtime change.

## Reproduction

Use a new output directory and freeze the source before building. The checked-in
tools record source, worker/artifact hashes, compiler, features, flags and clocks.
Build all compared variants before timing, then run them serially on an idle
host. Keep allocation instrumentation out of timing workers.

For a new native backend survey, `scripts/resource_report.py build` produces
worker metadata; `run --mode timing` uses an uncounted build, and
`run --mode allocations` needs a separate `build --count-allocations` result.
The default 29-size grid includes 4 MiB and retains policy skips. Set compiler
flags before the build; use `--cache` in a separate cohort. These are internal
resource workers, not ordinary public-consumer binaries.

`scripts/public_api_campaign.py` has separate `build`, `measure` and
`allocations` phases for Auto/cache/no_alloc/bs58. Its timing phase also uses
internal workers. `scripts/code_size.py` builds standalone reachable-API probes;
its `--cpu generic` clears target-CPU overrides, and other values set
`-C target-cpu=...`. `--opt-level z` provides a separate size-oriented cohort.
`scripts/wasm_tradeoffs.mjs` runs previously built, metadata-bound Wasm modules;
the newer SIMD experiment's exact build settings are in its
[receipt](benchmarks/simd/receipts.json).

The published plots and validation helpers can be checked without benchmarks:

```text
python3 -B assets/graphs/render.py --check
python3 scripts/verify_power.py --self-test
python3 scripts/backend_summary.py --self-test
```

These checks do not recreate historical timings. The retained tables and small
receipts are sufficient to inspect selected results and bindings. Full raw
replays and exact rejected-prototype rebuilds require the recorded archival
inputs; their hashes remain in the receipts. `backend_summary.py` requires
explicit archived raw inputs for a full historical rebuild.

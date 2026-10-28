# GCP backend and resource grid

Measured **2026-10-28 UTC**, 04:15:30–05:03:44, on an AMD EPYC 9B45 / `c4d-highcpu-4` VM with four guest vCPUs. Generic and actual `target-cpu=native` builds use Rust 1.99.1 / LLVM 23.1.1. The native checks confirmed AVX2, AVX-512 IFMA and VBMI support. This complete boot contains 12 timing units and four counted units; it does not pool the earlier interrupted boot.

All 16 backend/control names, 29 payload sizes and both directions are accounted for: **3,268 measured comparison cells + 444 explicit skips**, plus 464 separate temporal Auto controls. Timing used 11,196 fresh processes, each with three warm batches. Allocation measurement used 3,268 fresh cases with cold and warm windows, giving 6,536 layout rows. [Receipts](receipts.json) bind every unit, source, worker and raw evidence hash; [semantic validation](analysis.json) also checks the exact domain/order, phase/sample uniqueness, finite clocks/layouts and independent fixture fingerprints.

The source is frozen resource identity `4e5ac883…1e760`. Each CPU/cache cohort uses one immutable `__internals` executable with runtime-selected backend function pointers. These are **internal backend comparisons**, separate from the ordinary public README curves and [final-source304dc compact study](../final-compact-304dc/README.md). Their clocks, batching, buffer/call context and source epochs must not be silently combined.

## Graphs and complete tables

| CPU build | Full graph | All algorithm rows | Kernel and fallback controls | Heap/code table |
|---|---|---|---|---|
| Generic | [default/cache × encode/decode](../graphs/gcp-generic-backends.svg) | [default](backend-tables/gcp-generic-default-algorithms.md), [cache](backend-tables/gcp-generic-cache-algorithms.md) | [default](backend-tables/gcp-generic-default-kernel-controls.md), [cache](backend-tables/gcp-generic-cache-kernel-controls.md) | [default](resources/gcp-generic-default.md), [cache](resources/gcp-generic-cache.md) |
| Native | [default/cache × encode/decode](../graphs/gcp-native-backends.svg) | [default](backend-tables/gcp-native-default-algorithms.md), [cache](backend-tables/gcp-native-cache-algorithms.md) | [default](backend-tables/gcp-native-default-kernel-controls.md), [cache](backend-tables/gcp-native-cache-kernel-controls.md) | [default](resources/gcp-native-default.md), [cache](resources/gcp-native-cache.md) |

[timings.csv](timings.csv) retains every measured/skipped cell, cold/second/warm values and source-based `route_inference`. Graphs show positive sizes on logarithmic axes; zero-byte rows remain in every CSV/table. Bold marks the lowest observed median within each table, without a significance claim.

Names do not imply independent algorithms. Small recursive wrappers use batched conversion; large Matrix wrappers can share NTT encoding or Karatsuba decoding. `ntt-cached` is a legacy hook name: with `cache` off, its twiddles are owned per conversion. `*_portable` replaces one kernel family, with compiler vectorization permitted; an NTT portable wrapper can still use IFMA short-product fallback, and Matrix portable can route above its domain to normal NTT kernels. Same-leaf controls change twiddle policy while preserving leaf policy. These routes are inferred from the bound source/feature contracts, not sampled traces.

Quadratic no-allocation/batched/bs58 rows stop at 64 KiB by measurement policy. Their APIs are not limited to that size. Five8 has only 32/64 B entries. Forced Matrix has a separate checked kernel domain, including actual encoded-character limits. No missing 4 MiB quadratic value is extrapolated.

## Representative Auto observations

These are warm process-CPU medians of nine batches, with caller/oracle preparation outside the clock. All backend rows remain in the linked tables.

| Build / policy | Payload | Encode | Decode |
|---|---:|---:|---:|
| Generic / cache off |4 KiB|50.963 µs|33.094 µs|
| Generic / cache off |64 KiB|3.558 ms|1.363 ms|
| Generic / cache off |4 MiB|350.689 ms|165.306 ms|
| Generic / cache on |4 KiB|41.331 µs|33.293 µs|
| Generic / cache on |64 KiB|2.776 ms|1.295 ms|
| Generic / cache on |4 MiB|325.575 ms|163.279 ms|
| Native / cache off |4 KiB|48.796 µs|21.894 µs|
| Native / cache off |64 KiB|3.557 ms|1.236 ms|
| Native / cache off |4 MiB|339.745 ms|156.087 ms|
| Native / cache on |4 KiB|38.430 µs|22.210 µs|
| Native / cache on |64 KiB|2.770 ms|1.169 ms|
| Native / cache on |4 MiB|313.082 ms|154.469 ms|

Across [all 464 temporal controls](temporal-auto-controls.csv), later Auto/public-Auto median ratios range **0.895129–1.068490**. [Every group extreme](temporal-controls.md) and every cell remain visible. The same immutable implementation is being remeasured, so these differences are not backend speedups. For example, the native 32 B encoder's apparently large Auto/no-allocation gap sits alongside an unchanged-Auto temporal ratio of 0.895. Close winners and tiny call/context differences must not be treated as established algorithm improvements.

No other team compiler, test, fuzzer, proof or large probe overlapped timing. The original protocol remains unpinned, with guest CPUs 0–3 available and a 3 ms process-CPU batch floor. That does not fix physical CPU frequency, host contention, temperature or input-buffer placement. Each size uses one deterministic independent Natural fixture; this is not a benchmark of every value distribution. Full golden output is checked after batches. The frozen worker does not add the newer observer's poisoned-output/canary checks, and this report does not claim otherwise.

## Heap and linked deployment cost

[allocations.csv](allocations.csv) records exact requested `GlobalAlloc` layouts per backend/size/direction, separately from uncounted timing. `alloc_calls` already includes its `zeroed_calls` subset; the resource tables count allocations plus resizes once. Caller input/output and oracle storage are outside each operation window. Requested layouts exclude allocator metadata/slack, resident pages, realloc-internal copy overlap and stack.

At 4 MiB, both CPU builds have the same observed Auto layout:

| Policy | Encode peak extra heap | Decode peak extra heap | Retained encode / decode heap | Encode / decode allocation or resize calls |
|---|---:|---:|---:|---:|
| Cache off |201,207,328 B|109,807,352 B|0 / 0 B|35 / 22|
| Cache on |199,813,368 B|109,807,352 B|233,984 / 0 B|41 / 22|

The retained encoder bytes are the 512/1,024 Matrix classes, including their panel capacities. This fresh 4 MiB conversion retains no NTT twiddles; it does not prove that mixed-size histories retain none. The feature changes leaf/table policy as well as reuse. The full-family 64-bit cache header inventory is 6,280 B in static storage, while an ordinary consumer can link a subset. Heap caches, caller output, static headers, ELF DATA/RELRO/BSS and stack/RSS are different quantities. See the earlier [cache histories and RSS/stack scope](../README.md#heap-cache-rss-and-stack) for their separate measurements.

[linked-artifacts.csv](linked-artifacts.csv) retains 224 actual GCP ELF rows: 204 frozen-source backend artifacts and 20 separately labeled a479 ordinary controls. Matching empty probes preserve CPU flags, policy, direction and consumer scope. Their section deltas are deployment costs, not crate-only bytes or `.rlib` sizes. [Contracts](linked-contracts.json) retain raw source/compiler/profile/command hashes and parsed feature arguments. The actual linker is **LLD 23.1.1**; installed GNU ld 2.40 is tool inventory. GNU strip 2.40 is a separately recorded file-size step. Legacy per-artifact Cargo arguments are available; those older archives do not retain the final compact study's per-artifact compiler feature-fingerprint JSON.

The [worker assembly view](worker-assembly.json) confirms the mutable internal IFMA threshold folds away in all four fat-LTO timing workers. Actual generic CPU-feature-cache reads remain. This grid must not attribute an internal/public performance difference to a threshold atomic that is absent from its emitted worker. The [ordinary assembly review](../assembly.md) and [detector-hoist proposal](../detector-hoist.md) retain the narrower production concern and unimplemented follow-up.

These results support the existing backend roles: batched/fixed for small and no-allocation work, Karatsuba for recursive/short-product fallbacks, NTT for large conversions, Matrix for useful cache-enabled medium/leaves. Toom-3 stays experimental and Malachite is a development reference. Shared wrapper fallbacks do not justify dropping an underlying family. The final compact feature's code/speed decision is [measured separately](../final-compact-304dc/README.md).

## Reproduce without retargeting the snapshot

Restore frozen4e5 in a separate source directory using the [304dc→07ec patch](../final-compact-304dc/source-304dc-to-07ec.patch), earlier proof patch and [API reverse patch](../source-0915-to-4e5.patch). Check the exact resource fingerprint before builds. Source304dc/current main is a separate epoch. Use Rust 1.99.1, Python optimization level 0, fresh targets/output paths and separate counted/uncounted workers. Generic uses no CPU override; native uses actual `RUSTFLAGS='-C target-cpu=native'`. Preserve all observed or failed cases, the recorded case order and all temporal controls.

For example, in the restored source:

```sh
python3 scripts/resource_report.py fingerprint
python3 scripts/resource_report.py build --target-dir /tmp/b58-target --output /tmp/b58-build.json --binary-output /tmp/b58-worker
python3 scripts/resource_report.py run --build-metadata /tmp/b58-build.json --output /tmp/b58-public --mode timing --backends auto,no-alloc,bs58,five8 --repeats 3 --samples 3 --timeout 1800
```

Use `--cache` for cache builds and `--count-allocations` for the separate allocator worker. The three timing backend lists are public `auto,no-alloc,bs58,five8`; NTT `auto,batched,ntt,ntt-cached,ntt-portable,matrix-forced,ntt-same-owned,ntt-same-cached`; development `auto,dc,toom3,malachite,matrix,matrix-portable`. Counted runs use all registered backends with one fresh repeat. [Receipts](receipts.json) preserve all 16 unit contracts and exact counts. Run one unit at a time in an otherwise idle measurement environment.

Figures can be regenerated from the committed CSV without running a benchmark: create a separate Python environment from [the pinned plotting versions](tools/requirements-plot.txt), then use [render_graphs.py](tools/render_graphs.py) with `--data timings.csv --output NEW_DIR`. [render_tables.py](tools/render_tables.py) similarly accepts new data/output paths. Matplotlib 3.11.2, Python 3.14.10, all 12 dependency versions/manifests and font hash are in [plot-runtime.json](plot-runtime.json). The [title bridge](graph-title-bridge.json) verifies the only renderer revision adds the exact VM type to two titles. Large binaries, archives and raw process logs remain private; their hashes remain bound in these receipts.

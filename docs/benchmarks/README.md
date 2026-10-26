# Benchmark data

These datasets have different source generations and measurement contracts.
Compare within a matched dataset. Historical curves are not current-crate
measurements, and CPU, elapsed time, requested heap and Wasm pages are
different quantities. No cells are extrapolated.

| Dataset | Purpose and source generation |
| --- | --- |
| [Complete backend timing grid](backend-grid/README.md) | Historical 2026-10-24 source; all 29 decoded payload sizes through 4 MiB, four environments. This precedes the cache feature split, package rename and no_std. |
| [Complete earlier backend resource tradeoffs](backend-resources/README.md) | Historical 2026-10-23 source. This is the complete earlier allocation/code-size association for five runtime configurations, including Wasm on GCP. It is not current heap usage. |
| [Published public-mode comparison](public-modes/README.md) | 2026-10-25 snapshot before allocation-layout changes, no_std and rename. These exact inputs feed the README timing graphs. |
| [Accepted allocation-layout refactor](allocation-layout/README.md) | v000→v011 before no_std and rename. Sizes.csv final v011 rows feed the README linked-size table. |
| [Source-migration and load-context controls](production/README.md) | Earlier production no_std/rename source comparison and follow-up controls; distinct from current17ab assembly/SIMD snapshots. |
| [Rejected optimization decisions](decisions/README.md) | Retained small results for rejected IFMA threshold-state plumbing and nearby-length fixed-table reuse. Full raw experiments remain only in the local backup. |
| [Current-source assembly experiment](assembly/README.md) | Frozen baseline7c6ffa3 / resource17ab and its rejected six-loop candidate. Current-stable local/GCP A/B data are separated from the initial MSRV inventory. |
| [Current-source Wasm SIMD experiment](simd/README.md) | Frozen7c/17ab modules and bounded panel prototypes; identical modules were run on M5 and GCP with recorded Node/runtime identities. |

The published README SVGs retain their exact plotted numbers. Their timing
input is [public-modes/timings.csv](public-modes/timings.csv); the linked-size
input is [allocation-layout/sizes.csv](allocation-layout/sizes.csv). The
renderer and receipt must be updated together when moving those paths.

[The retention manifest](retention.json) maps original paths and hashes to these
files. The [history map](history-map.json) maps recorded Git commit IDs to the
rewritten history. Removed archives and development records remain only in the
local backup; they cannot be recovered from the rewritten repository.
The selected benchmark datasets are maintained in the repository; they are excluded from
the distributable crate. Packaged copies of the narrative documentation should
be read alongside [the online data index](https://github.com/elichai/faster-base58/blob/main/docs/benchmarks/README.md).

Large raw archives, binary payloads, agent plans and chronological run logs
remain only in the local backup. They are absent from the rewritten repository
and this data tree. This directory retains the complete selected grids, process
medians, meaningful controls and source bindings.

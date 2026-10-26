# Complete earlier backend resource tradeoffs

Historical 2026-10-23 source. This is the complete earlier allocation/code-size association for five runtime configurations, including Wasm on GCP. It is not current heap usage.

Native allocation metrics are requested Rust layout bytes, excluding caller buffers and allocator overhead. zeroed_calls/bytes are subsets of alloc_calls/bytes; requests are alloc_calls + realloc_calls. Wasm pages include stack/static/caller/allocator space and are not live heap. The historical unbounded table cache was later replaced. Do not combine its timing values with the corrected backend grid.

Original CSV files are copied byte for byte. Files ending `.original.json.gz`
are lossless, timestamp-free compression of the original metadata. Their
old paths are historical locators, not active documentation links. Source
and destination hashes are in the [retention manifest](../retention.json).

| Data | Role |
| --- | --- |
| [observations.csv](observations.csv) | Complete earlier per-backend allocation, cache, Wasm-page and section/file-size tradeoffs |
| [linked-sizes.csv](linked-sizes.csv) | Complete earlier per-backend allocation, cache, Wasm-page and section/file-size tradeoffs |
| [summary-sources.csv](summary-sources.csv) | Complete earlier per-backend allocation, cache, Wasm-page and section/file-size tradeoffs |
| [provenance.original.json](provenance.original.json) | Original source/hash/metric-source index; some locators now name archived files |

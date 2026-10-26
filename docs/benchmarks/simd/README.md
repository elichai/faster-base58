# Current-source Wasm SIMD experiment

Frozen7c/17ab modules and bounded panel prototypes; identical modules were run on M5 and GCP with recorded Node/runtime identities.

Times are elapsed, with separate warm/cold/second metrics. Compare matched module/control rows; enabling simd128 changes compilation and JIT behavior. The receipt binds exact source, profiles, module bytes and raw replay counts. Full binaries/raw events remain private. No new default target-feature policy is implied.

Original CSV files are copied byte for byte. Files ending `.original.json.gz`
are lossless, timestamp-free compression of the original metadata. Their
old paths are historical locators, not active documentation links. Source
and destination hashes are in the [retention manifest](../retention.json).

| Data | Role |
| --- | --- |
| [timings.csv](timings.csv) | Frozen latest SIMD numeric scope and exact source/module bindings |
| [linked-sizes.csv](linked-sizes.csv) | Frozen latest SIMD numeric scope and exact source/module bindings |
| [receipts.json](receipts.json) | Frozen latest SIMD numeric scope and exact source/module bindings |

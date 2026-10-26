# Accepted allocation-layout refactor

v000→v011 before no_std and rename. Sizes.csv final v011 rows feed the README linked-size table.

Public timing uses normal standalone consumer cdylibs. Allocation measurements use separate __internals instrumented workers with their oracle/hook layout. Requests are alloc_calls + realloc_calls; zeroed is a subset. Peaks are requested-layout peaks, not RSS. Caller input/output/fixture allocation is separate in public-allocations/io-separate.csv. Realloc peak accounting cannot see allocator-internal simultaneous old/new blocks. Native sections and Wasm linear memory are separate quantities.

Original CSV files are copied byte for byte. Files ending `.original.json.gz`
are lossless, timestamp-free compression of the original metadata. Their
old paths are historical locators, not active documentation links. Source
and destination hashes are in the [retention manifest](../retention.json).

| Data | Role |
| --- | --- |
| [sizes.csv](sizes.csv) | Accepted resource changes, paired public timing and matched v011 linked-size graph inputs |
| [source-identities.csv](source-identities.csv) | Accepted resource changes, paired public timing and matched v011 linked-size graph inputs |
| [resources.csv](resources.csv) | Accepted resource changes, paired public timing and matched v011 linked-size graph inputs |
| [public-timings.csv](public-timings.csv) | Accepted resource changes, paired public timing and matched v011 linked-size graph inputs |
| [public-processes.csv](public-processes.csv) | Accepted resource changes, paired public timing and matched v011 linked-size graph inputs |
| [public-pairs.csv](public-pairs.csv) | Accepted resource changes, paired public timing and matched v011 linked-size graph inputs |
| [report.original.json.gz](report.original.json.gz) | Original output/input hash chain; archived absolute locators remain historical |
| [inputs.original.json.gz](inputs.original.json.gz) | Original output/input hash chain; archived absolute locators remain historical |
| [public-allocations/measurements.csv](public-allocations/measurements.csv) | Exact measured public aliases: requests/peak/retained; caller I/O separated |
| [public-allocations/comparisons.csv](public-allocations/comparisons.csv) | Exact measured public aliases: requests/peak/retained; caller I/O separated |
| [public-allocations/io-separate.csv](public-allocations/io-separate.csv) | Exact measured public aliases: requests/peak/retained; caller I/O separated |
| [public-allocations/validation.original.json.gz](public-allocations/validation.original.json.gz) | Lossless full allocator/source/binary/raw-summary audit; zeroed is a subset |

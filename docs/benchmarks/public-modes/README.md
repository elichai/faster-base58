# Published public-mode comparison

2026-10-25 snapshot before allocation-layout changes, no_std and rename. These exact inputs feed the README timing graphs.

M5 native and Wasm timing were rerun with sampled AC power and low-power mode off; GCP timing and allocation inputs retain their earlier matched cohorts. Native process CPU and Wasm elapsed clocks remain distinct. Times exclude caller allocation and owned String/Vec helpers. no_alloc and bs58 stop at 64 KiB by measurement policy; Auto includes 4 MiB. Native public aliases are measured inside an __internals worker, not a standalone consumer.

Original CSV files are copied byte for byte. Files ending `.original.json.gz`
are lossless, timestamp-free compression of the original metadata. Their
old paths are historical locators, not active documentation links. Source
and destination hashes are in the [retention manifest](../retention.json).

| Data | Role |
| --- | --- |
| [timings.csv](timings.csv) | Published public-mode comparison; graph source / requested heap / page and unchanged controls |
| [allocations.csv](allocations.csv) | Published public-mode comparison; graph source / requested heap / page and unchanged controls |
| [wasm-pages.csv](wasm-pages.csv) | Published public-mode comparison; graph source / requested heap / page and unchanged controls |
| [controls.csv](controls.csv) | Published public-mode comparison; graph source / requested heap / page and unchanged controls |
| [manifest.original.json.gz](manifest.original.json.gz) | Lossless original source/feature/runtime/AC-sampling audit and hashes |
| [validation.original.json.gz](validation.original.json.gz) | Lossless original source/feature/runtime/AC-sampling audit and hashes |

[Exact 32/64-byte fixed-path comparisons](fixed-paths.csv) retain the specialized-versus-generic measurements separately from the public-mode curves.

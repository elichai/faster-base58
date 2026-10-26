# Complete backend timing grid

Historical 2026-10-24 source; all 29 decoded payload sizes through 4 MiB, four environments. This precedes the cache feature split, package rename and no_std.

Native uses process CPU; Wasm uses elapsed time. There are three process medians per measured cell. The native worker links all backends through __internals; Wasm selects one entry per module. Native fixtures use the independent Natural oracle at all sizes. The large Wasm fixture generator is not independent. FNV fixture fingerprints are consistency checks, not cryptographic proofs.

Original CSV and `.original.json` files retain the original bytes. Their
embedded paths, commit IDs and archive references are historical identifiers,
not active documentation links. Original and current paths and hashes are
recorded in the [retention manifest](../retention.json).

| Data | Role |
| --- | --- |
| [observations.csv](observations.csv) | Complete 29-size / four-environment backend grid and support limits |
| [processes.csv](processes.csv) | Complete 29-size / four-environment backend grid and support limits |
| [skipped.csv](skipped.csv) | Complete 29-size / four-environment backend grid and support limits |
| [provenance.original.json](provenance.original.json) | Lossless source/build/runtime/fixture and input-hash provenance |
| [completion.original.json](completion.original.json) | Original rendered-grid completion and output hashes |

## Readable timing tables

Tables show medians only; exact observed process ranges and route/support
information remain in `observations.csv`. Bold is the exact lowest
observed median within one environment/operation/size/group, including
ties. It is not statistical significance. Entry names describe frozen
benchmark wrappers; they are not current public API names.

- m5-native: [encode](m5-native-encode.md), [decode](m5-native-decode.md).
- gcp-default: [encode](gcp-default-encode.md), [decode](gcp-default-decode.md).
- gcp-native: [encode](gcp-native-encode.md), [decode](gcp-native-decode.md).
- m5-wasm: [encode](m5-wasm-encode.md), [decode](m5-wasm-decode.md).

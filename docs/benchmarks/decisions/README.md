# Rejected optimization decisions

Retained small results for rejected IFMA threshold-state plumbing and nearby-length fixed-table reuse. Full raw experiments remain in private backup and Git history.

The historical threshold snapshot had different internal-worker and normal-consumer no-LTO outcomes. The ordinary consumer already folded the tuning atomic and did not establish a broad gain. Fixed tables remain useful at their exact supported sizes; broad nearby-size padding was rejected. These historical decisions do not establish current-source benchmark values.

Original CSV files are copied byte for byte. Files ending `.original.json.gz`
are lossless, timestamp-free compression of the original metadata. Their
old paths are historical locators, not active documentation links. Source
and destination hashes are in the [retention manifest](../retention.json).

| Data | Role |
| --- | --- |
| [threshold-snapshot/observations.csv](threshold-snapshot/observations.csv) | Rejected state-plumbing experiment: all observed summaries and linked-size deltas |
| [threshold-snapshot/size-deltas.csv](threshold-snapshot/size-deltas.csv) | Rejected state-plumbing experiment: all observed summaries and linked-size deltas |
| [threshold-snapshot/manifest.original.json.gz](threshold-snapshot/manifest.original.json.gz) | Original source/archive hashes and full input manifest |
| [threshold-snapshot/validation.original.json.gz](threshold-snapshot/validation.original.json.gz) | Original source/archive hashes and full input manifest |
| [threshold-snapshot/ordinary-no-lto-summary.csv](threshold-snapshot/ordinary-no-lto-summary.csv) | Distinct default-consumer attribution check |
| [threshold-snapshot/ordinary-no-lto-metadata.original.json.gz](threshold-snapshot/ordinary-no-lto-metadata.original.json.gz) | Original ordinary-consumer compiler/profile/source and artifact identities |
| [fixed-table-reuse-controls.csv](fixed-table-reuse-controls.csv) | Anchors for rejected nearby-length padding: fixed and shared-route controls |
| [fixed-table-reuse-size-deltas.csv](fixed-table-reuse-size-deltas.csv) | Rejected adapter linked-size deltas |

[Decision ledger](decisions.json) records accepted, rejected, experimental and unresolved choices without preserving chronological run narratives.

The newer [inverse-scale formula screen](scale.md) retains local size results for
two rejected NTT arithmetic expressions. Its [CSV](scale-sizes.csv) is separate
from the older timing campaigns; neither timing nor GCP validation is claimed.

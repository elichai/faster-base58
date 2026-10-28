# Verification records

[CORRECTNESS.md](../../CORRECTNESS.md) states the contracts, test coverage, proof
domains and open work. These compact records bind completed runs to source and
tools; they do not replace the source or imply coverage of later changes.

| Record | Scope |
| --- | --- |
| [Latest local checks](local-ci-current.md) / [receipt](https://github.com/elichai/faster-base58/blob/main/docs/verification/data/local-ci-current.json) | Workflow rows, owned-decoder checks, fuzz/coverage observations, source bridges and unavailable or failed platform checks |
| [Earlier per-target proofs](proofs-current.json) | Historical 50-body macOS/Linux components; the latest local receipt adds fresh Linux 50 + compact 3 results; M5 retains its source and eight queries remain open |
| [Guard source bridge](guard-source-bridge.json) / [patch](guard-cancellation.patch) | Reconstructs the prior verification tools; only cancellation ownership changed, with all Rust proof inputs preserved |
| [Historical Miri component](miri-current.md) / [receipt](miri-current.json) | Earlier 150 custom executions, 16 stock and 15 Linux setup probes; latest local verification records fresh 186 custom + 16 stock on its frozen source |
| [Local dispatcher verification](local-ci.md) / [derived receipt](local-ci.json) | Earlier full matrix, outlined-source proofs/sanitizers and the later cfg-only portability fix; exact inputs, commands and remaining platform limits |
| [Historical proofs](proofs.json) | 40 distinct selected Kani harnesses: 4 tiny-production, 31 parser/arithmetic, 5 capacity/lemma; commands, solver/compiler identities and raw-log hashes |
| [Historical harness results](harnesses.csv) | Individual proof names and results; the production code is checked under its stated scalar domains |
| [Historical sanitizers](sanitizers.json) | Careful ASan/LSan fuzz execution and replay, native TSan concurrency and native Rust MSan; frozen source/corpus/tool hashes and explicit excluded setup results |
| [Historical source and verification status](status.json) | Local closure checks and distinct earlier Miri/detector records |

The historical proof snapshot includes the inverse-scale contract check and independent
tests. Later packaging and documentation-only differences are recorded rather
than presented as new proof runs. The sanitizer snapshot includes the three
prepared-transform/divisor assertions; it predates the added inverse-scale
assertion and proof tests. The receipts preserve that distinction. These older records
do not verify the later normalized-dispatch refactor.

Full source archives, models, instrumented binaries, corpora and raw local logs
are kept outside the active documentation tree. Recorded hashes support archival
comparison; hashes alone do not demonstrate a successful run. Hosted checks and
their source commits are available in
[GitHub Actions](https://github.com/elichai/faster-base58/actions/workflows/verification.yml).
The separate manual open-proof and unsupported-Miri jobs are expected to expose
the limits listed in CORRECTNESS.md.

The crate package excludes benchmark datasets and bulk verification artifacts.
For those, use the repository's
[online documentation](https://github.com/elichai/faster-base58/tree/main/docs).

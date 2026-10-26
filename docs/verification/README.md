# Verification records

[CORRECTNESS.md](../../CORRECTNESS.md) states the contracts, test coverage, proof
domains and open work. These compact records bind completed runs to source and
tools; they do not replace the source or imply coverage of later changes.

| Record | Scope |
| --- | --- |
| [Proofs](proofs.json) | 40 distinct selected Kani harnesses: 4 tiny-production, 31 parser/arithmetic, 5 capacity/lemma; commands, solver/compiler identities and raw-log hashes |
| [Harness results](harnesses.csv) | Individual proof names and results; the production code is checked under its stated scalar domains |
| [Sanitizers](sanitizers.json) | Careful ASan/LSan fuzz execution and replay, native TSan concurrency and native Rust MSan; frozen source/corpus/tool hashes and explicit excluded setup results |
| [Source and verification status](status.json) | Local closure checks and distinct earlier Miri/detector records |

The proof snapshot includes the inverse-scale contract check and independent
tests. Later packaging and documentation-only differences are recorded rather
than presented as new proof runs. The sanitizer snapshot includes the three
prepared-transform/divisor assertions; it predates the added inverse-scale
assertion and proof tests. The receipts preserve that distinction. Release
arithmetic and dispatch were not changed by these additions.

Full source archives, models, instrumented binaries, corpora and raw local logs
are kept outside the active documentation tree. Recorded hashes support archival
comparison; hashes alone do not demonstrate a successful run. Hosted checks and
their source commits are available in
[GitHub Actions](https://github.com/elichai/faster-base58/actions/workflows/verification.yml).
The separate manual open-proof and unsupported-Miri jobs are expected to expose
the limits listed in CORRECTNESS.md.

The published crate excludes benchmark datasets and bulk verification artifacts.
For those, use the repository's
[online documentation](https://github.com/elichai/faster-base58/tree/main/docs).

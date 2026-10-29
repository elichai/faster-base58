# Repository instructions

Keep changes small and complete. Read the affected code and its callers before
editing. Review current diffs first, preserve manual edits, and stage only the
changes intended for the task. Review new files individually; never stage
unrelated work.

## What belongs in commits

- Library code, tests, proof harnesses, reusable development tools and their tests.
- Build/CI configuration, lockfiles and operational inputs such as pinned Miri
  settings or compiler package lists. JSON is appropriate for these inputs.
- Small regression fixtures with a clear test purpose, including minimized
  reproductions. Keep large generated fuzz corpora in the configured corpus store.
- Living documentation, selected result tables, useful published SVGs, references
  and required third-party licenses/notices.

Do not commit generated benchmark CSV/JSON, run receipts, raw logs, coverage
exports, compiled binaries, source snapshots, bulk corpora, intermediate experiment
reports, or agent plans/review transcripts. Do not recreate dated benchmark
directories. Put local measurement output under ignored `target/measurements/`;
use temporary directories, CI artifacts or the configured storage for other
generated output. Retaining raw output is not a documentation requirement.

Do not ignore all CSV/JSON or binary files globally: deliberately maintained
configuration and regression fixtures are different from generated run output.

## Documentation to update with code

Update the relevant guide in the same change when its description becomes wrong.
Keep the wiki-style [documentation index](docs/README.md); consolidate useful
conclusions into the guides instead of adding another chronological report.

| Change | Documentation to review/update |
| --- | --- |
| Public APIs, error/buffer contracts, features, MSRV or usage | Rustdoc and [README.md](README.md); [CORRECTNESS.md](CORRECTNESS.md) for contract changes |
| Algorithms, arithmetic invariants, dispatch thresholds, CPU selection or cache ownership | [DESIGN.md](DESIGN.md) |
| Latency, cold/warm behavior, allocations, retained memory, linked size or benchmark methods | [docs/performance.md](docs/performance.md); affected README summaries/figures |
| Tests, fuzzing, sanitizers, Miri, proofs, coverage or verification gaps | [CORRECTNESS.md](CORRECTNESS.md) and the affected operational guide |
| Measurement or verification tooling and commands | The performance/correctness guide and nearby tool instructions |
| Consulted/adapted implementations, dependencies or attribution | [REFERENCES.md](REFERENCES.md) and applicable notices |
| Documentation paths or structure | [docs/README.md](docs/README.md) and incoming links |

Keep instructions near their tools where useful, especially
[fuzzing](fuzz/README.md), [custom Miri](ci/miri/README.md) and
[linked-size probes](tools/code-size/README.md). Do not duplicate their procedures
across several reports.

## Results and claims

Published results need their measurement date, source, host/ISA, compiler, flags,
features, harness, clock and sampling scope. Keep material limitations beside
the result. When replacing a measurement cohort, update its tables, figures and
scope together. Code or documentation edits do not refresh old measurements;
label affected results as historical or withdraw them until they are remeasured.

Keep ordinary public consumers separate from internal backend workers, timing
separate from counted allocations, and native execution separate from translated
execution or cross-builds. Never report an unsupported, skipped, incomplete or
compiled-only check as a passing runtime test or proof. Preserve unresolved
verification gaps when shortening documentation.

The README prioritizes M5, generic x86-64 and Wasm results. Keep
`-C target-cpu=native` comparisons in the performance guide.

## Before committing

Run checks appropriate to the change, following [CORRECTNESS.md](CORRECTNESS.md)
and the [verification workflow](.github/workflows/verification.yml). Add a
focused regression test for a behavioral fix. For documentation/figure changes,
run `python3 -B assets/graphs/render.py --check`; for graph tooling, also run
`python3 -B -m unittest discover -s assets/graphs -p test_render.py`. README Rust
examples are included in rustdoc, so changed examples need `cargo test --locked --doc`.
Package-content changes also need the checks in `scripts/check_package.py`.

Use short, reviewable commands and prefix shell commands with `rtk` as configured
for this repository. Never run `rm` without asking the user first. Check the final
diff and staged paths, and report what actually ran and any remaining limits.

# Repository instructions

Read the affected code and its callers before editing. Review current diffs,
preserve manual edits, and stage only work intended for the task.

## What belongs in commits

- Library code, tests, proof harnesses, reusable development tools and their tests.
- Build/CI configuration, lockfiles and operational inputs such as pinned Miri
  settings or compiler package lists. JSON is appropriate for these inputs.
- Small regression fixtures with a clear test purpose, including minimized
  reproductions. Keep large generated fuzz corpora in the configured corpus store.
- Living documentation, selected result tables, useful published SVGs, references
  and required third-party licenses/notices.

Keep generated measurements, logs, coverage, binaries, source snapshots and agent
reports out of commits. Use ignored `target/measurements/`, temporary directories
or CI artifacts. Do not ignore CSV/JSON or binary files globally; configuration
and regression fixtures may use those formats.

## Documentation to update with code

Update the relevant guide in the same change when its description becomes wrong.
Keep the wiki-style [documentation index](docs/README.md). The guides describe
only the current code and current results: no history, changelogs, comparisons
with earlier revisions or "historical" tables. Record measured alternatives that
were not adopted in a short comment next to the code they concern.

| Change | Documentation to review/update |
| --- | --- |
| Public APIs, error/buffer contracts, features, MSRV or usage | Rustdoc and [README.md](README.md); [CORRECTNESS.md](CORRECTNESS.md) for contract changes |
| Algorithms, arithmetic invariants, dispatch thresholds, CPU selection or cache ownership | [DESIGN.md](DESIGN.md) |
| Latency, cold/warm behavior, allocations, retained memory, linked size or benchmark methods | [docs/performance.md](docs/performance.md); affected README summaries/figures |
| Tests, fuzzing, sanitizers, Miri, proofs, coverage or verification gaps | [CORRECTNESS.md](CORRECTNESS.md) and the affected operational guide |
| CI jobs, repository scripts, measurement or verification tooling and commands | [CI.md](CI.md), the performance/correctness guide and nearby tool instructions |
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
withdraw affected results until they are remeasured.

Separate public-consumer and internal-worker results, timing and allocation
counting, and native and translated execution. Builds, skips and incomplete runs
are not passing tests. Preserve unresolved verification gaps in the guides.

The README prioritizes M5, generic x86-64 and Wasm results. Keep
`-C target-cpu=native` comparisons in the performance guide.

## Before committing

Run checks appropriate to the change, following [CI.md](CI.md),
[CORRECTNESS.md](CORRECTNESS.md) and the
[verification workflow](.github/workflows/verification.yml). Add a
focused regression test for a behavioral fix. For documentation/figure changes,
run `python3 -B assets/graphs/render.py --check`; for graph tooling, also run
`python3 -B -m unittest discover -s assets/graphs -p test_render.py`. README Rust
examples are included in rustdoc, so changed examples need `cargo test --locked --doc`.
Package-content changes also need the checks in `scripts/check_package.py`.

Use short commands prefixed with `rtk`. Never run `rm` without asking the user.
Review the final diff and staged paths. Report checks run and remaining limits.

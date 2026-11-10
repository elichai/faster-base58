# Technical documentation

| Document | Scope |
| --- | --- |
| [Design](../DESIGN.md) | Backends, dispatch, architecture-specific code and resource costs. |
| [Correctness](../CORRECTNESS.md) | Contracts, tests, formal proofs and remaining verification limits. |
| [CI and scripts](../CI.md) | Every CI job and repository script: trigger, platforms, prerequisites, host limits and local commands. |
| [Performance](performance.md) | Benchmark coverage, allocation and binary-size accounting, and reproduction. |
| [References](../REFERENCES.md) | Algorithm references and implementation provenance. |
| [Fuzzing](../fuzz/README.md) and [corpus store](fuzzing-corpus.md) | Fuzz targets, campaigns, boundary replay and the shared corpus branch. |
| [Development Miri](../ci/miri/README.md) | The pinned interpreter with the missing SIMD shims. |
| [Linked size probe](../tools/code-size/README.md) | The consumer probes behind the README timing and size tables. |

Measurements are tied to their source, compiler, features, target and harness.
The performance guide separates ordinary public consumers from internal backend
workers and gives commands for measuring a new revision. Results from different
sources or measurement contracts must not be pooled.

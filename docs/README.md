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
| [Consumer probes](../tools/code-size/README.md) | The public consumers behind the README timing, heap and linked-size tables. |

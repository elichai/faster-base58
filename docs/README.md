# Technical documentation

| Document | Scope |
| --- | --- |
| [Design](../DESIGN.md) | Backends, dispatch, architecture-specific code and resource costs. |
| [Correctness](../CORRECTNESS.md) | Contracts, tests, formal proofs and remaining verification limits. |
| [Performance](performance.md) | Benchmark coverage, allocation and binary-size accounting, and reproduction. |
| [Assembly](assembly.md) | Linked-code findings and the rejected NTT loop experiment. |
| [SIMD](simd.md) | Existing vector coverage and measured Wasm tradeoffs. |
| [References](../REFERENCES.md) | Algorithm references and implementation provenance. |
| [Benchmark datasets](benchmarks/README.md) | Complete selected tables, process ranges, controls and source bindings. |

Measurements are tied to their source, compiler, features, target and harness.
The dataset index separates historical backend comparisons from newer assembly
and SIMD experiments. Results from different generations must not be pooled.

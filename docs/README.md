# Technical documentation

| Document | Scope |
| --- | --- |
| [Design](../DESIGN.md) | Backends, dispatch, architecture-specific code and resource costs. |
| [Correctness](../CORRECTNESS.md) | Contracts, tests, formal proofs and remaining verification limits. |
| [Performance](performance.md) | Benchmark coverage, allocation and binary-size accounting, and reproduction. |
| [References](../REFERENCES.md) | Algorithm references and implementation provenance. |

Measurements are tied to their source, compiler, features, target and harness.
The performance guide separates ordinary public consumers from internal backend
workers and gives commands for measuring a new revision. Results from different
sources or measurement contracts must not be pooled.

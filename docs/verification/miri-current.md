# Historical Miri component

This record preserves earlier executions and their original source bridges.
[Latest local verification](local-ci-current.md) records separate final-source
186 custom and 16 supported-stock executions.

All executions used frozen Cargo manifests and `src/`/`tests/` inputs from
`a113f210f55734c8c5eb177ef76d6d6ba16c0755833474109871be2df16bd9b0`.
The JSON includes their complete per-file source map. A separate byte comparison
bridges these inputs to historical source map
`a7160e6940abeeaa12a72dd4f8e301232c53b5193e4e36698af66904eb8972b7`:
its sole difference is the additional
`paired_update_all_return_success` proof inside `#[cfg(kani)] mod proofs` in
`src/batched.rs`. Removing exactly that item reproduces the frozen file; every
other checked file matches. This proof is excluded from Miri compilation.
The original run source hashes and counts remain unchanged.

`miri-current.json` also binds interpreter, compiler, Cargo, sysroot, coverage
and raw-log hashes. Historical runs on `0986262` are excluded. This receipt
establishes no native x86 execution; consult separate CI records.

The historical custom matrix has **150/150 concrete test executions complete**.
All rows use strict provenance. Counts exclude setup and test discovery.

| Target | Cache | Tree Borrows, seed 1 (full) | Stacked Borrows, seed 7 (kernels) |
|---|---|---:|---:|
| aarch64 | off | 27/27 | 7/7 |
| aarch64 | on | 27/27 | 7/7 |
| x86_64 | off | 32/32 | 9/9 |
| x86_64 | on | 32/32 | 9/9 |

Full rows include the target's isolated probes, actual SIMD kernel and
integration tests, and all fourteen original buffer cases per architecture.
There are fifteen distinct probes across the two targets.
Each buffer case runs separately with all existing backend, short-buffer and
canary checks. The stock suite also executed the original sequential test. The exact lists
are in `ci/miri/coverage.json`. Zero tests, ignored selections and known ISA
omission messages fail the custom runner.

The actual NTT multiplication tests use owned tables. The additional policy
test checks shared table initialization, source and reuse, without transforming
a shared operand. Native oracle and cache-history/concurrency tests supply the
broader shared-transform coverage. These are finite concrete executions, not
complete-domain proofs or a native x86 CPU claim.

Stock `nightly-2026-10-26` ran all fifteen minimal probes: twelve retained
unsupported-intrinsic failures and three passing extra permutation probes.
The exact twelve names and real exit codes are recorded separately. Supported
stock refresh: 7/7 required commands, 16/16 test executions complete.
Custom passes do not relabel stock failures as supported.

The actual Linux cold bootstrap passed eleven commands, including compiler
acquisition and interpreter construction. Execution was local Linux/Rosetta on
M5, with a clean local pinned source bundle. Public fork/revision availability
was checked separately. Driver qualification has passed
15/15 probes with fresh matching sysroots;
it is separate from the matrix count above.
The first ARM harness build lacked its declared cross compiler. Installing
that dependency let a new six-probe run pass; the original failure is retained.

The interpreter pins fork `52a3791566`, compiler `df87fa660`, Cargo `cad07cf8e`
and the licensed reviewed patch. The existing maintainer shim changes retain separate attribution; the
new signed halving subtraction is identified independently. Optional native-lib
is disabled; no FFI or full-upstream-Miri-suite claim is made. No user Miri/Rust
checkout or global toolchain was modified.

Runner-only hardening is bridged by retained source/tool/flag identities and a
final raw-log audit. Earlier receipts keep their original coverage hashes; the
one newly selected policy test has separate supplemental receipts. The final
nine synthetic wrapper regressions check bookkeeping, not Rust correctness.

The interrupted historical broad ARM run remains interrupted. All fourteen
original cases subsequently passed as separate processes on the recorded source.

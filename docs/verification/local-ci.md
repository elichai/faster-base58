# Local verification of the normalized dispatcher

Local runs on 2026-10-26 cover the normalized dispatcher, NTT outlining and a later portable-target correction.
[The derived receipt](local-ci.json) preserves source scopes, original receipt hashes,
commands and compiled-input hashes.
These are local workflow adaptations; no hosted CI result is claimed.

| Source identity | Value |
| --- | --- |
| Base commit | `f1a710f4e774f946e59a433c0b210bd8ae6a90db` |
| Earlier full-matrix patch | `558df06e19f00c9d7c735fad303a775a9119f227563d728aa6688151eb81aeea` |
| Outlined-source patch | `1fb39ea576d1f6d1f5ebdb377685880ef431e98cd55ddf2e84f4d0b8f98d4648` |
| Later cfg-only patch | `ce811a2854d24de199af074d2dbb7128513e5e60071fe1013b39b4fd12e04139` |
| Resource fingerprint after that correction | `61c0ba893a019322a6186efcd1656b4d50faea7ecd37cfa148ba08572d6c7e2b` |

The outlining bridge adds only two `#[inline(never)]` attributes. The earlier **102/102**
workflow commands retain their original source identity. Full debug/release, i686 runtime,
alternate-limb, maximum-transform and 18-module Wasm ABI coverage stays on that source.

## Outlined-source checks

**84/86 fresh commands passed; two object builds failed.** Two additional
exact-workflow metadata checks passed separately. The later correction is below.

| Coverage on the outlined source | Passed / commands |
| --- | ---: |
| macOS ARM / Linux x86-64 release public/internal properties, no_alloc, tiny and NTT groups; cache off/on | 12 / 12 |
| Counted/uncounted resource tests and self-checks on both hosts | 16 / 16 |
| Formatting, feature Clippy and API documentation | 15 / 15 |
| Benchmark metadata checks, both hosts and profiles, cache off/on | 8 / 8 |
| Portable/MSRV/Wasm/AArch64, thumb/i686 and MSVC library objects, plus bare-metal attempts | 31 / 33 |
| Package build and content policy before these documentation additions | 2 / 2 |

Property suites retained **512 cases**. Focused runtime used release; full debug
and i686 runtime, maximum-transform and Wasm ABI campaigns were not repeated.
Packaging used `--allow-dirty` for the uncommitted checkout. The two failures were
`x86_64-unknown-none` object builds, with no features and with `__internals`, at
LLVM's “Do not know how to split the result of this operator!” error. Both also
reproduce before outlining. Their passing metadata checks did not test codegen.

## Later portability correction

The correction requires baseline SSE2 at **28 existing x86 cfg predicates in
12 files**. x86-64 targets without it select existing portable kernels; reversing those
predicates restores every changed file byte for byte. Stable builds passed on
macOS and Linux, and MSRV 1.95 builds passed on Linux: **8 builds per compiler
configuration**, x86-64 none / Thumbv7em none × debug/release × no features/`__internals`.
Two further x86-64 none builds at opt-level 0 and two strict Clippy checks passed.
No firmware was linked or executed. The workflow now builds both profiles.

For the corrected source, a separate **transcribed tool-result summary** records
13 public release tests per cache policy, with 512 property cases and no failures
or ignores. It has no separate raw stdout record and is not a backend-suite rerun.

All **19 frontend expansion pairs** are byte-identical: 12 ordinary libraries,
1 nightly library with `miri` but no `test`, and 6 Kani AArch64 frontend cells
with `kani` and all valid feature sets. These stop before typechecking/codegen.
**24 cfg queries** additionally cover Kani target models and Miri's `miri`/`test`
configurations. Predicate truth values match. This scoped source bridge is
**not a new proof, interpreter, sanitizer or fuzz run**, or a binary-layout claim.

## Proofs and supported Miri

On the **outlined source**, fresh runs verified **40 required scalar Kani
harnesses**: 4 tiny-production, 31 general and 5 capacity/lemma harnesses. Kani
0.68.0 / CBMC 6.11.0 ran natively on macOS ARM. Miri passed **16 selected test
executions in 7 commands, zero ignored**, interpreting x86-64 and AArch64 Linux.
Strict provenance remained on; selected library paths used Tree Borrows/seed 1.

Earlier optional probes reproduce 11 unsupported Miri shims. Of 15 open Kani harnesses,
1 verified, 13 timed out at 60 seconds and 1 ended with CBMC status 6; remaining obligations stay open.

## Fuzzing and native sanitizers

The earlier campaign recorded **72,826 ASan + 58,594 LSan executions in mutation cohorts**.
All 8 cohorts completed 180-second budgets, each reporting 181 seconds. They retain their
earlier source; no cohort ran after outlining or the cfg fix. Mutation caps stayed at 8,193 bytes.

Fresh **outlined-source** checks passed four careful builds, eight valid-seed bootstraps
and eight full boundary replays. The unchanged corpus has 169 encode / 201 decode files,
including empty duplicates. Campaign replay caps remain 65,537 binary / 89,502 raw decode bytes.

Outlined-source native macOS TSan passed 5 concurrency tests. Native Rust MSan on Linux/Rosetta
passed 7 commands: 9 covered tests and 1 IFMA skip among 10 harness tests. Its API guard
required/passed 4 tests. Instrumented std/dependency caches reused identical compiler/sanitizer
identities; changed crates rebuilt. No suppressions or runtime modifications were used.

## Limits and provenance

macOS ran natively on Apple ARM; Linux used Debian trixie x86-64/i686 emulation.
Linux TSan was not repeated: its earlier Rosetta layout failure occurred before tests,
even after an isolated per-process ASLR request succeeded. No host-global setting changed.
Native Windows and SDK-dependent development checks remain open; library builds do not replace them.
AVX-512/IFMA was unavailable, including QEMU 10.0.13 `-cpu max`; no native IFMA execution is claimed.

Miri does not validate emitted SIMD instructions. The scalar Kani obligations do
not prove arbitrary-length conversion. Native Rust MSan does not close the
libFuzzer/MSan integration gap. Finite fuzzing does not prove memory safety.
Raw logs, binaries, corpora and full receipts remain private. The derived receipt is not an original byte record.
Earlier [verification records](README.md) retain their historical identities.

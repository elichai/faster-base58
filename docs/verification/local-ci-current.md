# Current local and native verification

**The current feature build is still being checked.** Implementation `9e67f52` has a 55-row workflow and adds compact IFMA kernels. The completed 47-row snapshot below predates those changes; it does not certify that implementation. Its source identities and results remain unchanged. Final package validation is pending.

**Completed snapshot:** all 47 workflow rows are accounted for. Native x86 Linux ordinary tests, TSan, focused Rust MSan and careful ASan passed on GCP for their recorded inputs. Four native Windows rows remain unavailable, full MSan/libFuzzer remains tooling-blocked, and eight wider proof selectors remain open. This is not an all-green hosted-CI claim.

The [structured receipt](https://github.com/elichai/faster-base58/blob/main/docs/verification/data/local-ci-current.json) binds workflow `4775440951b379994e7ddd72dc81c83a754a9014f1a6f05124f53ddf0e30a556`, exact commands/environments, source epochs, tool versions and raw-receipt/log hashes. The existing named dictionaries are self-described and lossless. Receipt assembly itself ran no project commands.

The current resource identity is `07ec12777e6065bd644baa75af4fd2cd0fe38fc39674957dc1eb220dc220f3b1`. Its only Rust change from 0915 is the cfg(kani) unwind attribute 12→6; ordinary production code is unchanged. The new two-digit proof keeps its `0ea0837b6582b2229e10320a9ba07f38a89318736eee7fbd272af1e71f11f4be` map. The later current-tool map `cf2230169a925aaab42e9f4812830e00682573a5d108a9873bb3b066d183fdf1` changes only kani_guard.py and process_supervisor.py. Earlier 4e5/0915/Miri/proof/public/native executions retain their recorded maps.

Native macOS ARM, local Debian/Trixie amd64 under Rosetta, QEMU i686 and remote native Debian 12 x86_64 are separate scopes. The native supplement ran on AMD EPYC 9B45 with OS-aware AVX-512IFMA availability; it supplements Ubuntu-labelled workflow rows. Cross-built MSVC libraries do not establish Windows linking or execution. No new hosted Actions results are included.

| Workflow row | Status | Actual execution epoch / scope |
| --- | --- | --- |
| `tests/0` · os=ubuntu-latest, profile=debug, caches=off | pass native | remote native Debian 12 x86_64 on AMD EPYC 9B45 (GCP); Ubuntu-labelled workflow substitute · source=native_x86 |
| `tests/1` · os=ubuntu-latest, profile=debug, caches=on | pass native | remote native Debian 12 x86_64 on AMD EPYC 9B45 (GCP); Ubuntu-labelled workflow substitute · source=native_x86 |
| `tests/2` · os=ubuntu-latest, profile=release, caches=off | pass native | remote native Debian 12 x86_64 on AMD EPYC 9B45 (GCP); Ubuntu-labelled workflow substitute · source=native_x86 |
| `tests/3` · os=ubuntu-latest, profile=release, caches=on | pass native | remote native Debian 12 x86_64 on AMD EPYC 9B45 (GCP); Ubuntu-labelled workflow substitute · source=native_x86 |
| `tests/4` · os=macos-latest, profile=debug, caches=off | pass | native macOS ARM · source=ordinary |
| `tests/5` · os=macos-latest, profile=debug, caches=on | pass | native macOS ARM · source=ordinary |
| `tests/6` · os=macos-latest, profile=release, caches=off | pass | native macOS ARM · source=ordinary |
| `tests/7` · os=macos-latest, profile=release, caches=on | pass | native macOS ARM · source=ordinary |
| `tests/8` · os=windows-latest, profile=debug, caches=off | unavailable | Unrun · source=unrun |
| `tests/9` · os=windows-latest, profile=debug, caches=on | unavailable | Unrun · source=unrun |
| `tests/10` · os=windows-latest, profile=release, caches=off | unavailable | Unrun · source=unrun |
| `tests/11` · os=windows-latest, profile=release, caches=on | unavailable | Unrun · source=unrun |
| `alternate_limb_width/0` · os=ubuntu-latest | pass | local Debian/Trixie x86_64 translated by Rosetta on M5 · source=final |
| `alternate_limb_width/1` · os=macos-latest | pass | native macOS ARM · source=final |
| `portable_builds/0` · target=wasm32-unknown-unknown, profile=debug | pass | cross-object builds in local Debian/Trixie under Rosetta · source=library |
| `portable_builds/1` · target=wasm32-unknown-unknown, profile=release | pass | cross-object builds in local Debian/Trixie under Rosetta · source=library |
| `portable_builds/2` · target=aarch64-unknown-linux-gnu, profile=debug | pass | cross-object builds in local Debian/Trixie under Rosetta · source=library |
| `portable_builds/3` · target=aarch64-unknown-linux-gnu, profile=release | pass | cross-object builds in local Debian/Trixie under Rosetta · source=library |
| `maximum_transform/0` | pass native | remote native Debian 12 x86_64 on AMD EPYC 9B45 (GCP); Ubuntu-labelled workflow substitute · source=native_x86 |
| `no_std/0` · target=thumbv7em-none-eabi | pass | cross-object builds in local Debian/Trixie under Rosetta · source=library |
| `no_std/1` · target=x86_64-unknown-none | pass | cross-object builds in local Debian/Trixie under Rosetta · source=library |
| `msrv/0` | pass | local Debian/Trixie x86_64 translated by Rosetta on M5 · source=ordinary |
| `package/0` | pending | Pending · source=unrun |
| `pointer_width_32/0` | pass emulated | i686 binaries through qemu-i386 inside local x86_64 Linux/Rosetta · source=owned_final |
| `lint/0` | pass | fresh final-public source compilation/checks in local Debian/Rosetta · source=public_final |
| `measurement_tools/0` · cache_args= | pass | fresh final-public source compilation/checks in local Debian/Rosetta · source=public_final |
| `measurement_tools/1` · cache_args=--features cache | pass | fresh final-public source compilation/checks in local Debian/Rosetta · source=public_final |
| `wasm_measurement_abi/0` | pass | Wasm compiled in local translated Linux and executed by Node/V8 there · source=final |
| `miri_supported/0` | pass host adaptation | stock interpreter on native macOS ARM host, interpreting Linux target · source=miri |
| `miri_arm_panel/0` | pass host adaptation | stock interpreter on native macOS ARM host, interpreting Linux target · source=miri |
| `miri_custom/0` · arch=arm, cache=off | pass host adaptation | reviewed custom interpreter on native macOS ARM host, interpreting Linux target · source=miri |
| `miri_custom/1` · arch=arm, cache=on | pass host adaptation | reviewed custom interpreter on native macOS ARM host, interpreting Linux target · source=miri |
| `miri_custom/2` · arch=x86, cache=off | pass host adaptation | reviewed custom interpreter on native macOS ARM host, interpreting Linux target · source=miri |
| `miri_custom/3` · arch=x86, cache=on | pass host adaptation | reviewed custom interpreter on native macOS ARM host, interpreting Linux target · source=miri |
| `miri_unsupported_simd/0` · arch=arm | diagnostic evidence only | individual stock interpreter probes on macOS ARM host · source=miri |
| `miri_unsupported_simd/1` · arch=x86 | diagnostic evidence only | individual stock interpreter probes on macOS ARM host · source=miri |
| `fuzz/0` · sanitizer=address, cache_args= | pass native | remote native Debian 12 x86_64 on AMD EPYC 9B45 (GCP); Ubuntu-labelled workflow substitute · source=native_x86 |
| `fuzz/1` · sanitizer=address, cache_args=--features cache | pass native | remote native Debian 12 x86_64 on AMD EPYC 9B45 (GCP); Ubuntu-labelled workflow substitute · source=native_x86 |
| `fuzz/2` · sanitizer=leak, cache_args= | pass | local Debian/Trixie x86_64 under Rosetta · source=owned_final |
| `fuzz/3` · sanitizer=leak, cache_args=--features cache | pass | local Debian/Trixie x86_64 under Rosetta · source=owned_final |
| `fuzz_coverage/0` · cache_args= | pass observation | exact named-boundary unsanitized coverage under local Debian/Rosetta · source=coverage_final |
| `fuzz_coverage/1` · cache_args=--features cache | pass observation | exact named-boundary unsanitized coverage under local Debian/Rosetta · source=coverage_final |
| `native_sanitizers/0` · sanitizer=thread, cache=on | pass native | remote native Debian 12 x86_64 on AMD EPYC 9B45 (GCP); Ubuntu-labelled workflow substitute · source=native_x86 |
| `native_sanitizers/1` · sanitizer=memory, cache=off | pass native | remote native Debian 12 x86_64 on AMD EPYC 9B45 (GCP); Ubuntu-labelled workflow substitute · source=native_x86 |
| `native_sanitizers/2` · sanitizer=memory, cache=on | pass native | remote native Debian 12 x86_64 on AMD EPYC 9B45 (GCP); Ubuntu-labelled workflow substitute · source=native_x86 |
| `proofs/0` | pass multi epoch | original Linux49 plus newly qualified Linux two-digit proof; local Debian/Rosetta · source=proof_components |
| `proofs_open/0` | pending optional | Eight open selectors; bounded M5 attempts are recorded separately. · source=unrun |

The native 07ec supplement passed all four ordinary 512-case rows, including public integrations, documentation examples and benchmark compilation. Combined unit/integration counts by debug-off/debug-on/release-off/release-on are 177/183/175/181 passed, with 3/1/3/1 ignored; each separate public integration command passed 21 and each doctest command passed 3. Native TSan passed 5 concurrency tests. Focused Rust/std MSan passed 20 tests per cache policy with IFMA required and no skips. Exact maximum-transform passed 1/0/0. Its 640,768 KiB maximum descendant RSS includes compilation/test children; it is not backend heap size.

Native careful ASan used 180 seconds per target/policy and passed both policies plus boundary replays. Its timed stages reported 52,466 callback invocations and its replay stages 745. The retained 0915 campaigns separately reported ASan 59,718 and LSan 53,574 timed-stage invocations, plus 1,508 replay invocations; the ASan budgets were 600 seconds and LSan 180. Historical 4e5 counters remain 109,770/1,508. These are libFuzzer DONE/TotalNumberOfRuns counters: they include initial corpus execution and possible leak-check repeats. They are not distinct mutations or named file counts. All eight native target/stage counter sets, initialization counts, boundaries and source-derived counter semantics are retained. No cross-epoch sum or throughput claim is made.

Both exact CI coverage rows passed 169 encode and 201 decode named files, with processed/executed counts, unchanged corpus and profile checks. These are unsanitized source-coverage observation builds with prebuilt std and no branch instrumentation. They remain the 041/0915 source epoch under Rosetta, separate from native sanitizer execution.

Separate manual coverage observations replayed 1,656 unique encode inputs plus 3,352 decode inputs per policy. The corpora came from the earlier 180-second 4e5 campaigns, replayed against 0915; they are not the new 600-second corpora. Cache-off own-source coverage is 2,628/3,669 lines, 350/517 functions and 4,171/5,990 regions. Cache-on is 2,721/3,777 lines, 370/535 functions and 4,294/6,114 regions. All gaps remain visible. These Rosetta observations did not execute native IFMA/AVX-512 paths; ergonomic wrappers absent from the fuzz harness also remain outside these observations, even where separate public tests exercise them. Later native passes do not rewrite these coverage denominators.

Earlier 0915 Rust MSan observations remain 19 covered tests plus one IFMA skip per policy. Earlier macOS ARM TSan passed 5 tests. The earlier translated Linux TSan startup-list exited 66 with zero tests; its offline read-only seccomp-unconfined/setarch-R diagnostic also failed on the Rosetta layout. These failures remain historical evidence after the later native TSan pass. No host settings changed in that diagnostic.

Both full 512-case i686 policies passed: default 168 tests/3 ignored and cache 175/1 ignored, each across 12 binaries. Eight MSVC library builds passed; debug representative members are COFF, while release fat-LTO members are LLVM bitcode with separate representative-module LLVM 23.1.1 COFF emission. Native Windows linking/runtime/docs/benchmarks remain unrun.

The b61 public-asset checks retain 40 measurement commands, 13 lint commands and default/cache doctests 3+3. Four Rust tool test commands passed 5 tests each and 4 profiler self-checks passed. Those earlier Python guards passed 4+4+9. Later tool-only qualification passed 10 supervisor, 6 Kani-guard and 9 custom-Miri tests on each recorded platform, while four deterministic old-code regressions failed as intended. On macOS, the first six-test guard run hit restricted signalling EPERM; the same unchanged six tests then passed with normal child-signalling permission. The restricted failure and retry remain separate. Linux qualification used no adaptation. These are synthetic controller tests, not new crate execution or proof bodies. The [guard source bridge](guard-source-bridge.json) and hash-bound reverse patch reconstruct the earlier 54-input guard epoch in an isolated copy.

The current graph metadata `6468d1adab06f86caf355e144494d4fdd72ba5581baf98bcfd565a413562a2bb` removes the workspace-specific renderer reference while retaining its original provenance hash and the portable generator. A later graph-only render.py --check passed 192 timing points, 16 linked-size cells and 1,160 current cells across 2 figures and 3 README tables, with unchanged input/output hashes. It is a new numeric/hash/SVG observation, not a retarget of all b61 tool commands or pixel regeneration.

**50 distinct required proof bodies remain closed on both recorded targets.** Original 49 components and the additional two-digit proof retain their own source maps. The latter covers all 58² valid digit pairs and capacities 0..3 with the original value/error/canary contract; unwind 6 passed all 14 unwinding assertions plus normal memory/overflow checks on each host, without new stubs or assumptions. Each target produced 1,352 checks including 54 unreachable. M5 verification took 147.43s within 180s; translated Linux took 254.06s within 360s. Earlier failed attempts remain. The integrated proofs-current.json SHA is `cda29eb4d90d067bd1c846202853c101863bb9411ebaff6f1cd55abdfc53cbf4`. The archived `f14b350a0ba980040b89c62044e1841b73b38304b17f0d92ed3cc62f817dd022` receipt identifies only the earlier 49 component.

Per-target component sums remain 9,775 checks/269 unreachable on M5 and 18,744/5,190 on translated Linux. These combine explicit epochs and are not one frozen run. The original target-count difference remains confined to `verification::one_limb_production_multiply_matches_u128`: 618/40 on M5 versus 9,587/4,961 on Linux; other 48 count sets match. Capacity refreshes overlap existing bodies and add no distinct proofs. Historical M5-only limits stay scoped to that component.

Two optional one-byte wrapper attempts compiled, then timed out at 180 seconds: direct and a private unwind 6 candidate. They establish no theorem or counterexample, changed no main attribute and did not add a Linux wrapper proof. A repeated full-domain capacity subset passed 10 checks on the final guards. Its saved-formula diagnostic explains its own solver statuses: query 1 is SAT/model/exit 0; query 2 is UNSAT, then 85 get-value requests at lines 364–448 cause 85 model-not-available errors/exit 1. Direct and guarded statuses match and stderr is empty. The original no-tap formulas were deleted, so that older 0/1 pattern is corroborated rather than byte-compared. No status was rewritten; selected 50/open 8 is unchanged.

The original Miri report retains 150 custom executions, 16 supported stock executions and 15 cold-Linux probe qualifications. The focused 0915 wrapper supplement remains 63 public tests, 28 Miri executions and three capacity bodies/36 checks per proof host. These records are not retargeted to later guards. Finite interpreter coverage is not arbitrary-length or emitted-native-instruction proof.

The earlier independent read-only API/proof review remains in its stated scope. It found no runtime defect and ran no builds, tests, proofs or receipt-hash audit. It prompted two documentation corrections: the tiny proof input is valid UTF-8 of exactly two bytes; the selector reads the first four and last four bytes and can choose late insertion positions.

Full MSan/libFuzzer remains unverified. Its initialized/uninitialized external-runtime controls passed only their stated control scope. The dedicated full worker was stopped twice by an agent safety check; partial state was preserved and no owned orphan remained at the recorded inspection. Native Rust MSan does not close this full-campaign gap. This report starts no replacement attempt.

Only package validation remains a required pending execution among the available feasible rows; four native Windows rows remain unavailable. The full structured receipt is repository-only data and is omitted from crate archives; read it online in the source repository. The packaged narrative links to data/local-ci-current.json. The actual package policy is bound by SHA in the receipt; final archive contents still await validation. Performance work is outside this CI table. Raw logs, corpora, archives and binaries stay private. Historical reports and every earlier private draft are preserved.

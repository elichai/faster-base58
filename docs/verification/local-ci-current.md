# Current local and native verification

The 55-row workflow has 52 required rows. Of the non-package rows, 45 passed,
two added full-width stages are unverified, and four Windows rows are
unavailable. The independent package row's result and archive identity are
recorded in the [structured receipt](https://github.com/elichai/faster-base58/blob/main/docs/verification/data/local-ci-current.json).
Three manual diagnostic rows are separate. These results do not establish
an all-green workflow.

The runtime is frozen at resource source `304dc4c5dbf1`. Original executions keep
their `7dcfc511` source and `3a6f3f7a` workflow records. A timeout-only bridge raises
custom-Miri budgets to 90 minutes. Commit `9d67a5d` adds one conditional fuzz
stage in two rows under a separate `47a00929` source / `9db6d584` workflow epoch.
All 71 library/manifests/test/benchmark inputs are unchanged. Exact maps,
commands, tools and hashes are in the receipt.
Native Linux uses AMD EPYC 9B45 (`c4d-highcpu-4`, Debian 12) on GCP. Other Linux
checks run under Rosetta on M5; Miri interprets Linux layouts on M5. These are
local replicas of CI jobs. No hosted Actions result is claimed.

Documentation commits `94a42e6` and `acb38c8` update DESIGN, performance
reports, graphs, indexes and whitespace attributes. The byte bridges check
all 469 tracked files at `acb38c8`; the 71 compiled inputs, including the
README, remain unchanged. These bridges preserve the original execution
source maps.

| Row | Configuration | Result / actual platform |
| --- | --- | --- |
| `tests/0` | profile=debug, caches=off | pass · native x86 Linux on GCP |
| `tests/1` | profile=debug, caches=on | pass · native x86 Linux on GCP |
| `tests/2` | profile=release, caches=off | pass · native x86 Linux on GCP |
| `tests/3` | profile=release, caches=on | pass · native x86 Linux on GCP |
| `tests/4` | profile=debug, caches=off | pass · native ARM on M5 |
| `tests/5` | profile=debug, caches=on | pass · native ARM on M5 |
| `tests/6` | profile=release, caches=off | pass · native ARM on M5 |
| `tests/7` | profile=release, caches=on | pass · native ARM on M5 |
| `tests/8` | profile=debug, caches=off | unavailable · native Windows host unavailable |
| `tests/9` | profile=debug, caches=on | unavailable · native Windows host unavailable |
| `tests/10` | profile=release, caches=off | unavailable · native Windows host unavailable |
| `tests/11` | profile=release, caches=on | unavailable · native Windows host unavailable |
| `compact/0` | caches=off | pass · native x86 Linux on GCP |
| `compact/1` | caches=on | pass · native x86 Linux on GCP |
| `compact/2` | caches=off | pass · native ARM on M5 |
| `compact/3` | caches=on | pass · native ARM on M5 |
| `alternate_limb_width/0` | — | pass · Rosetta / Linux x86 |
| `alternate_limb_width/1` | — | pass · native ARM on M5 |
| `portable_builds/0` | target=wasm32-unknown-unknown, profile=debug | pass · cross build |
| `portable_builds/1` | target=wasm32-unknown-unknown, profile=release | pass · cross build |
| `portable_builds/2` | target=aarch64-unknown-linux-gnu, profile=debug | pass · cross build |
| `portable_builds/3` | target=aarch64-unknown-linux-gnu, profile=release | pass · cross build |
| `maximum_transform/0` | — | pass · native x86 Linux on GCP |
| `no_std/0` | target=thumbv7em-none-eabi | pass · cross build |
| `no_std/1` | target=x86_64-unknown-none | pass · cross build |
| `msrv/0` | — | pass · Rosetta / Linux x86 |
| `package/0` | — | [see receipt](https://github.com/elichai/faster-base58/blob/main/docs/verification/data/local-ci-current.json) · final included-document commit |
| `pointer_width_32/0` | — | pass · QEMU / i686 |
| `lint/0` | — | pass · Rosetta / Linux x86 |
| `measurement_tools/0` | cache_args= | pass · Rosetta / Linux x86 |
| `measurement_tools/1` | cache_args=--features cache | pass · Rosetta / Linux x86 |
| `wasm_measurement_abi/0` | — | pass · Node/V8 / Wasm |
| `miri_supported/0` | — | pass · M5 interpreter / Linux target |
| `miri_arm_panel/0` | — | pass · M5 interpreter / Linux target |
| `miri_custom/0` | arch=arm, cache=off | pass · M5 interpreter / Linux target |
| `miri_custom/1` | arch=arm, cache=on | pass · M5 interpreter / Linux target |
| `miri_custom/2` | arch=x86, cache=off | pass · M5 interpreter / Linux target |
| `miri_custom/3` | arch=x86, cache=on | pass · M5 interpreter / Linux target |
| `miri_unsupported_simd/0` | arch=arm | optional not rerun · missing-shim diagnostics |
| `miri_unsupported_simd/1` | arch=x86 | optional not rerun · missing-shim diagnostics |
| `fuzz/0` | address / default | unverified extension · original native steps passed; added stage unverified |
| `fuzz/1` | address / cache | unverified extension · original native steps passed; added stage unverified |
| `fuzz/2` | leak / default | pass · native x86 Linux on GCP |
| `fuzz/3` | leak / cache | pass · native x86 Linux on GCP |
| `fuzz/4` | address / compact | pass · native x86 Linux on GCP |
| `fuzz/5` | address / cache-compact | pass · native x86 Linux on GCP |
| `fuzz_coverage/0` | cache_args= | pass · Rosetta / Linux x86 |
| `fuzz_coverage/1` | cache_args=--features cache | pass · Rosetta / Linux x86 |
| `native_sanitizers/0` | thread-on | pass · native x86 Linux on GCP |
| `native_sanitizers/1` | memory-off | pass · native x86 Linux on GCP |
| `native_sanitizers/2` | memory-on | pass · native x86 Linux on GCP |
| `native_sanitizers/3` | memory-off-compact | pass · native x86 Linux on GCP |
| `native_sanitizers/4` | memory-on-compact | pass · native x86 Linux on GCP |
| `proofs/0` | — | pass · Rosetta / Kani scalar domains |
| `proofs_open/0` | — | optional not rerun · open proof queries |

Ordinary and compact rows keep internal tests separate from normal-dependency
public API tests, with a 512-case property setting, doctests and benchmark
compilation. The setting is derived from the executed runner and row/step
environments. It is not a total count of generated examples.

The custom Miri union passed **186 test executions**, plus eight discovery
commands, across ARM/x86, cache policies and Tree/Stacked models. It includes
compact x86 kernels and all 14 buffer shards. Seventy-two exactly matched kernel
invocations are reused once. Supported stock Miri passed **16 executions**.
The pinned Linux cold interpreter setup was audited for compatible reuse; these
current tests remain M5-host interpretations, separate from native ISA execution.

Kani passed **50 distinct required bodies plus three compact overlaps** on
translated Linux: 18,744 default checks and 8,096 compact checks. The local
scheduler ran one harness job at a time; solver identities, domains and
60/180/360-second budgets are unchanged.
Eight wider selectors remain open. Earlier M5 proofs retain their own sources.
A separate current-source encoder-split trial reached Z3 solving, then
timed out at 90 seconds. It produced no theorem or counterexample; the
required count remains 50. The receipt includes its technique and raw hashes.

Four native Rust/std MSan variants passed **20 covered tests each**, requiring
IFMA with zero ISA skips. TSan passed five concurrency tests. The exact maximum
transform passed at `2^23` points. Careful ASan passed default, cache, compact and
cache+compact campaigns; LSan passed default/cache. Each existing target used
180 seconds followed by full named-boundary replay. Callback counters include
initialization/repeats and are not distinct mutations or named file counts.

The small 8,193-byte mutation inputs do not reach native IFMA-host NTT thresholds.
Large boundary replay, MSan and Miri do exercise NTT. The new `encode_full_width`
target expands mutations to periodic 65,537-byte values with a set leading bit,
and checks encoding plus generated-text decoding against independent oracles.
Its two native careful-ASan campaigns are unverified. The cloud provider
preempted the VM during the first build. Independent read-only retrieval
verified 20 metadata/log files, five helper passes and a partial std build
log; completed build, binary and campaign records were absent. Recovery
used the same VM/policy and a fresh offline source/target plan. Automated
screening ended the delegated agent turn before any recovery remote call,
with reported reason "possible cybersecurity risk". No tool explicitly
rejected a command; this is not an ASan failure or an identified crate
defect. The recovered VM was confirmed idle and then stopped. Local
helper/build qualification and the tiny existing-target parser control
remain separate. A prior local Cargo setup timed out before engine
execution; its failed record is retained. Compilation does not establish
a campaign pass. No mutation/callback counts or new source-coverage
denominator are claimed for the two added steps.

Both current source-coverage rows replayed **169 encode and 201 decode files**
per cache policy. These are unsanitized translated-x86 observations without
branch coverage. Per-target denominators overlap and must not be added.

| Cache / target | Library lines | Functions | Regions |
| --- | ---: | ---: | ---: |
| off / encode_all | 2608/3669 | 345/517 | 4138/5990 |
| off / decode_all | 1468/3669 | 215/517 | 2382/5990 |
| on / encode_all | 2699/3777 | 365/535 | 4257/6114 |
| on / decode_all | 1513/3777 | 226/535 | 2435/6114 |

Twelve MSVC library/object cross-builds passed in six feature configurations and
both profiles. They do not establish Windows linking, tests, docs or benchmark
execution. Automatic execution review rejected the full MSan/libFuzzer campaign
with the reason "possible cybersecurity risk". That campaign was not run; it is
separate from the 80 passing native Rust/std MSan test executions. Finite tests and bounded proofs do
not establish correctness for every possible input or emitted instruction.

The [previous report](https://github.com/elichai/faster-base58/blob/d283ed3e90ca4c5de6a3ae3f5cd768ef6bb10393/docs/verification/local-ci-current.md)
and [receipt](https://github.com/elichai/faster-base58/blob/d283ed3e90ca4c5de6a3ae3f5cd768ef6bb10393/docs/verification/data/local-ci-current.json)
retain their own source epochs. Historical timing, coverage, M5 proofs and Miri
records are not retargeted. Raw logs and archives remain private; the structured
receipt binds their verified hashes. No Wiki, release or publication is included.

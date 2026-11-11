# Fuzz corpus and coverage

`main` owns fuzz targets, deterministic seeds, dictionaries, and the corpus
validator. The orphan `fuzz-corpus` branch owns only validated, content-addressed
inputs under `corpora/<target>/<sha256>` and run provenance under `runs/`.

The four supported targets are `encode_all`, `decode_all`,
`encode_full_width`, and `encode_radix_full_width`. A corpus item is shared by
input format, not by machine.
Its provenance records the architecture, target CPU, feature set, toolchain,
sanitizer, duration, and source commit that found it. CI replays one pinned
corpus commit without mutation across the supported architecture/feature/CPU
matrix. Profile data is never merged across those configurations.

The branch holds a coverage-selected corpus, not every input that a campaign
found. Nightly fuzzing writes artifacts only; its campaigns check debug
assertions and overflow, and the cache rows add libFuzzer's value profile. Each
of the four `minimize` builds validates names, file type, size and SHA-256, forms
the union of the campaign artifacts that are not stored yet, and runs
libFuzzer's `-merge=1` with the seeds and the stored corpus as the initial
corpus, which the merge keeps whole. A new input stays when it adds a coverage
feature, an edge-counter bucket, that the initial corpus and smaller inputs do
not reach in that build. The nightly and `minimize` builds are x86-64 and
AArch64, generic CPU, cache off and on; the two full-width targets use the
`full-width` feature. Inputs are architecture-independent bytes, so the
candidate is the union of the four results. `candidate_smoke` replays all of it
with ASan in the three `fuzz_coverage` configurations: x86-64 with cache off and
on, and AArch64 with cache. SIMD paths are chosen at run time, and the code that
differs without cache does not depend on the architecture.

CI never drops a stored input. Coverage depends on the CPU that runs the build:
runtime dispatch selects AVX-512, IFMA or NEON kernels and thresholds, and a
generic build on a hosted runner without them would find inputs that need them
redundant. Shrinking the store is a manual merge of the whole store and the new
campaigns, with only the seeds as the initial corpus, on hosts that together
cover those CPUs and the native builds, followed by an ASan replay of the union;
[fuzz/README.md](../fuzz/README.md) lists the builds.

Merge builds use `--sanitizer none --no-trace-compares`: cargo-fuzz still
enables the coverage counters, and the replay is faster. Compare tracing only
guides mutation, and merges do not use the value profile, so merge and replay
builds omit it; nightly campaigns keep it. libFuzzer's merge skips an input that
crashes, times out or exceeds the memory limit and then retries; a retry fails
the job instead of dropping that input.

The trusted publisher replaces `corpora/` with the candidate union, which holds
every stored input, so a nightly publication only adds inputs. It commits on the
pinned snapshot and rebases onto any later publication, with a retry.
Publishing requires every `fuzz_coverage`, `minimize` and `candidate_smoke` row
to pass; `fuzz_coverage` replays the pinned corpus with ASan, debug assertions
and overflow checks. The deterministic seeds stay on `main` and are not stored
on the branch. Pull requests cannot publish a corpus.

Coverage reports exclude tests, fuzz harnesses, dependencies, and diagnostic
helpers. In the [fuzz corpus workflow](../.github/workflows/fuzz-corpus.yml),
`fuzz_coverage` runs one job per configuration and target. Each replays the seeds
and the pinned corpus with ASan, without mutation, and exports LCOV, so the same
replay checks correctness and measures coverage. The Malachite and bs58 oracles
are built without coverage or debug instrumentation. In the
[verification workflow](../.github/workflows/verification.yml), each
`proptest_coverage` configuration runs the property suites and exports LCOV.
They upload to Codecov under the shared `fuzz` or `tests` flag; see
`codecov.yml`.

## One-time branch bootstrap

Create `fuzz-corpus` from a disposable clone. It is intentionally an empty
orphan commit: CI creates the empty directories when it checks out the branch.
Do this before enabling the workflow, then protect the branch against deletion,
force-pushes, and direct human writes.

```sh
git clone --no-checkout git@github.com:elichai/faster-base58.git fuzz-corpus-bootstrap
cd fuzz-corpus-bootstrap
git switch --orphan fuzz-corpus
git read-tree --empty
git commit --allow-empty -m "Initialize fuzz corpus"
git push origin fuzz-corpus
```

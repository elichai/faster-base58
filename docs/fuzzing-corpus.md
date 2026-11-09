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

The branch holds a coverage-minimized corpus, not every input that a campaign
found. Nightly fuzzing writes artifacts only. Each of the four `minimize` builds
validates names, file type, size and SHA-256, forms the union of the pinned
corpus and all campaign artifacts without the deterministic seeds, and runs
libFuzzer's `-merge=1` with the seeds as the initial corpus. An input stays when
it adds a coverage feature, an edge-counter bucket, that the seeds and smaller
inputs do not reach in that build. The nightly and `minimize` builds are x86-64
and AArch64, generic CPU, cache off and on; the two full-width targets use the
`full-width` feature. Inputs are architecture-independent bytes, so the
candidate is the union of the four minimized sets. `candidate_smoke` replays all
of it with ASan in the `smoke` configurations, which add x86-64 native CPU with
cache.

Merge builds use `--sanitizer none --no-trace-compares`: cargo-fuzz still
enables the coverage counters, and the replay is faster. Compare tracing only
guides mutation, because no job sets `-use_value_profile`, so merge and replay
builds omit it; nightly campaigns keep it. libFuzzer's merge skips an input that
crashes, times out or exceeds the memory limit and then retries; a retry fails
the job instead of dropping that input.

The trusted publisher replaces `corpora/` with the candidate union, so inputs
that no longer add coverage leave the branch; Git history keeps them. It commits
on the pinned snapshot and rebases onto any later publication, with a retry.
Publishing requires every `smoke`, `minimize` and `candidate_smoke` row to pass;
`smoke` replays the pinned corpus with ASan, so an input that fails on current
code is not removed silently. The deterministic seeds stay on `main` and are not
stored on the branch. Pull requests cannot publish a corpus.

Coverage reports exclude tests, fuzz harnesses, dependencies, and diagnostic
helpers. In the [verification workflow](../.github/workflows/verification.yml),
each of the five `fuzz_coverage` configurations replays the seeds and the pinned
corpus for `encode_all`, `decode_all` and `encode_radix_full_width` and exports
LCOV. Each `proptest_coverage` configuration runs the property suites and
exports LCOV. They upload to Codecov under the shared `fuzz` or `tests` flag; see
`codecov.yml`.
The `smoke` job of the [fuzz corpus workflow](../.github/workflows/fuzz-corpus.yml)
exports no coverage: it replays the seeds and the pinned corpus through all four
targets with ASan, without mutation.

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

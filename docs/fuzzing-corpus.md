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

Nightly fuzzing writes artifacts only. A separate, trusted publisher validates
names, file type, size, and SHA-256; replays additions; and publishes an
additive union with a rebase retry. Pull requests cannot publish a corpus.

Coverage reports exclude tests, fuzz harnesses, dependencies, and diagnostic
helpers. In the [verification workflow](../.github/workflows/verification.yml),
each of the eight `fuzz_coverage` configurations replays the seeds and the pinned
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

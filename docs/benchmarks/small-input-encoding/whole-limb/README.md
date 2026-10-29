# Whole-limb pair formatting

This follow-up compares PR #1 at `053f00810de3f39b03f2891a2771348e67bfa4e6`
with source `47974045e02f7b015c4b5cccc0fca7f96f04e2c5`.
Each ten-digit limb is emitted as five two-character table entries. The four
quotients for the lower pairs are independent of the top-pair quotient. The
table already exists; public APIs, dispatch, allocation, and unsafe code do not
change. The obsolete five-digit helper is removed.

For `P = 58²`, pair `k` is `(y / P^k) % P`. The most significant pair needs no
remainder because `y < P^5` implies `y / P^4 < P`. All other table indices use
an explicit remainder. LLVM unrolls the four-iteration loop. Inspection of the
complete formatter on AArch64 and x86 shows scalar reciprocal multiplies and
two-byte table loads, rather than SIMD division.

## Final canonical measurements

Times are nanoseconds per complete public-API encoding into a caller-provided
buffer. Parentheses give less time. Each value is the median of three rounds
of nine alternating batches of at least 10 ms process CPU time. Both immutable
libraries run in one process. Outputs and decoded round trips are checked before
timing. Inputs repeat with warm caches and seeded nonzero random bytes.

Both versions use identical physical source and probe-manifest paths, compiler,
features, flags, and fat LTO. On x86, each build starts with a fresh target
directory at the same physical path; old outputs are renamed and preserved.
The candidate dependency is explicitly recompiled after fresh-mtime source
replacement. On ARM, the final build reuses the original PR #1 probe while
preserving the original published artifact files. Source hashes bind the final
builds to the source commit above.

### Apple M5 Pro, AArch64 macOS, generic CPU, Rust 1.99.1 / LLVM 23.1.1

| Input bytes | Automatic API | `no_alloc` API |
| --- | ---: | ---: |
| 16 | 20.98 → 17.31 (17.5%) | 16.87 → 15.83 (6.2%) |
| 22 | 30.08 → 25.65 (14.7%) | 27.85 → 26.51 (4.8%) |
| 31 | 30.70 → 27.17 (11.5%) | 30.90 → 27.03 (12.5%) |
| 33 | 32.50 → 28.03 (13.8%) | 32.10 → 26.80 (16.5%) |
| 65 | 77.33 → 72.32 (6.5%) | 75.51 → 72.21 (4.4%) |
| 128 | 200.62 → 189.16 (5.7%) | 207.34 → 198.97 (4.0%) |
| 512 | 2086.67 → 2044.07 (2.0%) | 2088.38 → 2034.79 (2.6%) |

### AMD EPYC 9B45, native CPU, Rust 1.99.0 / LLVM 23.1.1

The Linux VM exposes four CPUs; timing is pinned to one allowed CPU.

| Input bytes | Automatic API | `no_alloc` API |
| --- | ---: | ---: |
| 16 | 33.45 → 30.49 (8.8%) | 33.17 → 29.79 (10.2%) |
| 22 | 46.75 → 43.00 (8.0%) | 45.96 → 42.18 (8.2%) |
| 31 | 52.50 → 47.92 (8.7%) | 51.25 → 46.82 (8.7%) |
| 33 | 52.70 → 47.69 (9.5%) | 51.28 → 47.05 (8.2%) |
| 65 | 121.74 → 113.83 (6.5%) | 117.36 → 109.36 (6.8%) |
| 128 | 329.19 → 313.61 (4.7%) | 325.14 → 314.15 (3.4%) |
| 512 | 3939.64 → 3918.09 (0.5%) | 3901.48 → 3868.14 (0.9%) |

The benefit becomes small by 512 bytes. Cold-cache latency and other CPUs are
not measured. Fixed 32/64-byte encoders, decoding, and all-zero scans are controls;
their variations are not attributed to this formatter. One final native run
reported a 17.2% fixed-64 automatic-API slowdown and a 10.5% leading-zero control
slowdown. Earlier canonical runs were flat at fixed 64 bytes. The final native
library's entire `.text` section is byte-identical to that earlier candidate in
both API fixtures. That equality alone does not identify the cause of the
candidate/reference control difference. The raw samples retain these outliers.
Six fresh processes per fixture, with both library load orders, put the fixed-64
candidate/reference median ratio at 0.999 for automatic and 1.000 for `no_alloc`.
The 31-byte gains in those processes remain 9.1% and 8.5%, respectively.
See the [fresh-process control samples](fresh-controls.json).

Separate-path exploratory builds also produced a native decoder outlier, which
disappeared with identical source/probe identities. Those runs are excluded from
the tables. A rejected prototype moved an otherwise identical zero-scan loop
across a 64-byte boundary and doubled its latency. Broken compare/branch fusion
is a plausible explanation, not a measured hardware-counter conclusion.

Native linked code shrinks by 112 bytes for the automatic fixture and 96 bytes
for `no_alloc`; read-only bytes are unchanged. ELF BSS padding changes by the
opposite amount. See the [reference](x86-reference-sizes.csv) and
[candidate](x86-candidate-sizes.csv) section sizes.

## Correctness and evidence

`ten_digits_matches_division` checks both ends of the ten-digit domain, every
single-digit radix boundary and its neighbors, and 10,000 seeded values against
serial division. Extracted formatters also match PR #1 for each of the
656,356,768 possible five-digit halves with the other half zero, all two-digit
radix boundaries, and 1,000,000 seeded complete limbs. These are large bounded
checks, not exhaustive coverage of all `58^10` limbs.

Final ARM full suites pass: default 164 tests (3 ignored), cache plus internals
186 tests (1 ignored). The native first-formatter suites pass 163/184 tests;
native final release `.text` is identical to that tested formatter. Native
rustfmt/rustdoc are absent; ARM formatting and documentation tests pass.
Final Rust 1.95 all-feature and Wasm all-feature checks pass, as does Clippy for
the library/tests with all features. An optional all-target Clippy run reports
pre-existing benchmark warnings in `encode_tune`, `dispatch`, `encode_matrix_tune`,
and `encode`; those benchmark files are unchanged.

Raw final samples: [ARM automatic](arm-auto.json), [ARM no-alloc](arm-no-alloc.json),
[native automatic](x86-auto.json), and [native no-alloc](x86-no-alloc.json).
The [summary](summary.csv) includes all measured controls.
[ARM source/artifact hashes](arm-identity.json) and
[native source/code hashes](x86-identity.json) identify the final source.
The [original ARM build identity](arm-build-identity.json),
[native canonical identity](x86-build-identity.json), and
[final native metadata](x86-final-metadata.json) record compiler commands.
Linked native formatter code is saved for the
[reference](x86-native-reference-encode_parts.s) and
[candidate](x86-native-candidate-encode_parts.s).
Run `python3 docs/benchmarks/small-input-encoding/whole-limb/validate-evidence.py`
from the repository root to verify source binding, summaries, and evidence links.

The accompanying build/check scripts retain exact workspace paths used in this
run. They require the reference/candidate snapshots at those paths. The shared
[`compare.py`](../compare.py) accepts any two fixtures built with identical
physical source/probe paths and flags. Reusing only an artifact filename while
changing the Cargo package path can change LTO emission order and invalidate
unchanged-path controls.

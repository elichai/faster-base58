# References and provenance

Sources consulted during development, algorithm references, and measurement
comparators have different roles. The source review covered reachable history,
unmerged branches, related worktrees and the original development records.
The [consultation receipt](docs/references/consultations.json) pins recovered
revisions and file hashes. No wholesale upstream file copy was identified;
the review does not establish independent authorship of every routine.
The consulted [five8 license](docs/references/five8-LICENSE.txt) and Firedancer
[license](docs/references/firedancer-LICENSE.txt) /
[notice](docs/references/firedancer-NOTICE.txt) are retained with these credits.

## Implementation and arithmetic references

- **Firedancer Base58:** the matrix conversion design explicitly generalizes
  Firedancer's fixed-width method. Our implementation uses base `58^4`,
  generated tables, variable-length panels and different carry handling.
  The AVX2 alphabet map also uses the same digit-gap comparison technique.
  The original consultation used revision `486a8abb…`:
  [Base58 template](https://github.com/firedancer-io/firedancer/blob/486a8abb17f8b4b7528e0fa4062f7aab8a5b6055/src/ballet/base58/fd_base58_tmpl.c),
  [AVX helpers](https://github.com/firedancer-io/firedancer/blob/486a8abb17f8b4b7528e0fa4062f7aab8a5b6055/src/ballet/base58/fd_base58_avx.h),
  [Apache-2.0 license](https://github.com/firedancer-io/firedancer/blob/486a8abb17f8b4b7528e0fa4062f7aab8a5b6055/LICENSE),
  [NOTICE](https://github.com/firedancer-io/firedancer/blob/486a8abb17f8b4b7528e0fa4062f7aab8a5b6055/NOTICE).
  Copyright 2022 Firedancer Contributors.
- **five8, Kevin Heavey:** a Rust port of Firedancer, consulted for the related
  fixed 32/64-byte approach and also benchmarked. Its base-`58^5` tables and
  carry schedule differ from ours. Version **1.0.0**, revision `311279fb…`:
  [project and attribution](https://github.com/kevinheavey/five8/blob/311279fb81461df36e730458a0463a6a322042d7/README.md),
  [MIT license](https://github.com/kevinheavey/five8/blob/311279fb81461df36e730458a0463a6a322042d7/LICENSE).
  The original encoder and fixed-path development records confirm reads of
  this release's encoder and AVX helpers before the corresponding local writes.
- **Richard Brent and Paul Zimmermann, _Modern Computer Arithmetic_
  (Cambridge University Press, 2010):** `encode/dc.rs` implements Algorithm
  3.5, `ApproximateReciprocal`, with local storage and multiplier interfaces.
  [Authors' page](https://members.loria.fr/PZimmermann/mca/pub226.html),
  [electronic version 0.5.2](https://maths-people.anu.edu.au/~brent/pd/mca-0.5.2.pdf).
  The electronic book's front matter specifies **CC BY-NC-ND 3.0**, not MIT/Apache.
  The original design cites this version and Algorithm 3.5; no direct PDF read
  was recovered. The document is not included in the repository or package.
- **Donald Knuth, _The Art of Computer Programming_, volume 2:** Algorithm D
  is the named method in `encode/dc.rs::div_schoolbook`. The retained record
  identifies the algorithm, not the exact edition or source consulted.
- **GNU MP manual:** background for
  [binary-to-radix conversion](https://gmplib.org/manual/Binary-to-Radix),
  [Karatsuba](https://gmplib.org/manual/Karatsuba-Multiplication) and
  [Toom-3](https://gmplib.org/manual/Toom-3_002dWay-Multiplication).
  GMP is not a runtime dependency. Its library has
  [LGPLv3-or-later / GPLv2-or-later terms](https://gmplib.org/manual/Copying);
  a manual link does not establish copied GMP code.

## Dependencies, oracles and comparators

Exact release revisions below come from checksum-verified published crates
and were checked against their upstream Git repositories.

| Project | Role here | Verified license and revision |
|---|---|---|
| [cpufeatures 0.3.1](https://github.com/RustCrypto/utils/tree/e3ac92bfd33051e146025baa2d0ce79891b7fc73/cpufeatures) | Normal x86 dependency; used for CPU detection without `std` | [MIT](https://github.com/RustCrypto/utils/blob/e3ac92bfd33051e146025baa2d0ce79891b7fc73/cpufeatures/LICENSE-MIT) OR [Apache-2.0](https://github.com/RustCrypto/utils/blob/e3ac92bfd33051e146025baa2d0ce79891b7fc73/cpufeatures/LICENSE-APACHE), `e3ac92bf…` |
| [bs58 0.5.1](https://github.com/Nullus157/bs58-rs/tree/7d3c9282d2595612e5474df93dd0e017db9b684f) | Differential oracle, error-index comparison and benchmark comparator | [MIT](https://github.com/Nullus157/bs58-rs/blob/7d3c9282d2595612e5474df93dd0e017db9b684f/LICENSE-MIT) OR [Apache-2.0](https://github.com/Nullus157/bs58-rs/blob/7d3c9282d2595612e5474df93dd0e017db9b684f/LICENSE-APACHE), `7d3c9282…` |
| [basexx 0.2.0](https://github.com/aki-akaguma/basexx/tree/0dd4b49a84dc505eceff23fa7c12c345d48c3e40) | Benchmark comparator | [MIT](https://github.com/aki-akaguma/basexx/blob/0dd4b49a84dc505eceff23fa7c12c345d48c3e40/LICENSE-MIT) OR [Apache-2.0](https://github.com/aki-akaguma/basexx/blob/0dd4b49a84dc505eceff23fa7c12c345d48c3e40/LICENSE-APACHE), `0dd4b49a…` |
| [Malachite 0.14.0](https://github.com/mhogrefe/malachite/tree/734f546826d20ddfe3678ab0d0d4739b8df75e03) | Development-only `Natural` multiplier adapter, independent radix oracle and comparator; historical conversion-design reference | [LGPL text](https://github.com/mhogrefe/malachite/blob/734f546826d20ddfe3678ab0d0d4739b8df75e03/LICENSE), `734f5468…`. Manifest says **LGPL-3.0-only**; inspected source headers say v3-or-later. Neither is MIT/Apache. |

The Malachite adapter calls its public API. Original development records also
confirm consultation of its divide-and-conquer conversion and preinverted
division in `general_digits.rs` before writing the local reciprocal code.
That establishes design consultation, not whether a routine was translated.
The inspected Malachite source itself credits GMP code and FLINT techniques.
Development and archived benchmark binaries can link these libraries; the
crate's MIT OR Apache-2.0 declaration does not replace upstream terms.
Other test/build tools are listed in the manifests; this is not a full
transitive dependency-license inventory.

## Open provenance questions

The original SIMD research consulted
[Gealber/zig-base58 at `48f48026…`](https://github.com/Gealber/zig-base58/tree/48f48026cccd6aece04afc5e2c5ace7b2b20f6b4)
as a design comparison. Its owner, revision and source reads are now confirmed;
**no license grant was found** in that tree. No explicit statement of copying or
translation was recovered. It is not treated as an MIT/Apache-licensed code source.

The initial commit already contains substantial implementation work. The record
cannot reconstruct every source consultation or settle the provenance of every
small routine, particularly the SIMD alphabet map and reciprocal division.
Any confirmed adaptation must retain the applicable upstream notices; this
reference list does not replace them. The exact Knuth edition remains unknown.

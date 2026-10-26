# NTT inverse-scale formula

The original formula is retained. For the three configured primes and legal
length `n = 2^log`, `log <= 23`, the required coefficient is the balanced residue
of `2^(64-log)` modulo the prime. Two equivalent expressions were screened on
M5 with Rust 1.99.1 / LLVM 23.1.1: one checked the private shape contract in release,
the other used a debug check and `ilog2` while preserving the zero-length panic.

[Twelve paired linked-size rows](scale-sizes.csv) bind baseline resource
`17ab5f264ed4` and candidate hashes to code, read-only data and artifact hashes.
Neither candidate reduced both code and read-only data. The second added
704 code bytes in the cached matched-owned-NTT consumer. Stripped file sizes
stayed equal because padding/layout can hide section changes. No-allocation
control text remained identical.

No timing or GCP result is claimed for these candidates. The size screen did
not justify adopting either expression; it does not establish their CPU cost.
The useful result was independent verification of the existing formula: all
72 prime/length combinations, 3528 Montgomery edge compositions and three
scalar Kani harnesses. Those checks are retained in
[the scalar tests](../../../src/ntt/tables.rs) and
[proofs](../../../src/ntt/proofs.rs).

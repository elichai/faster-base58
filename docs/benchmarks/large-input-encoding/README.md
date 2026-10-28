# Multiplication-tree encoding for large Base58 inputs

This change independently targets `main`; it does not require the small-input
pair formatter. It is enabled only for little-endian AArch64 with NEON.

The old encoder repeatedly divides binary integers by Base58 powers. The new
route splits at byte boundaries and combines `high * 256^k + low` directly in
base `58^3`, eliminating reciprocal construction and quotient correction.
Two NTT primes suffice for the bounded digit products. Powers and prepared
spectra are reused within a call, while twiddles retain the existing cache policy.

Profiling of an initial prototype exposed scalar reconstruction and a separate
lower-half addition as substantial costs. NEON two-prime reconstruction and a
fused multiply/add carry pass address them. In the final 1 MiB profile, transforms
account for about 60% of collapsed library samples and multiply/add including
carry for 30%; startup and independent-oracle stacks are excluded.

## Selection and compatibility

Full transforms lose at poorly padded sizes. The default route requires at least
8192 significant input bytes and 7/8 transform utilization. With `cache`, it
requires 262144 bytes and two input bytes per transform point: the old division
tree remains cheaper just below binary power-of-two boundaries. Other sizes,
inputs above 16 MiB, and other architectures retain the existing encoder.
The public API, errors, leading zeros and output-buffer contract are unchanged.
Decoding and `no_alloc` are unchanged.

## Measurements

Standalone default-feature public-API comparison against `09b2df62580cfd88dd436d3c1b4c536fd45a687d`:

| Input bytes | Main | Large-input branch | Less time |
| --- | ---: | ---: | ---: |
| 8 KiB | 0.246 ms | 0.222 ms | 9.7% |
| 32 KiB | 1.261 ms | 1.094 ms | 13.2% |
| 64 KiB | 2.902 ms | 2.519 ms | 13.2% |
| 256 KiB | 14.050 ms | 12.084 ms | 14.0% |
| 1 MiB | 65.068 ms | 55.610 ms | 14.5% |
| 4 MiB | 305.560 ms | 259.563 ms | 15.1% |

Raw measurements: [standalone-public.json](standalone-public.json).

Both libraries are measured in one process: three rounds of nine alternating
batches of at least 10 ms CPU time per batch, reporting the median of the three
round medians. Outputs and decoded round trips are checked before timing.
Host: AArch64 macOS, Rust 1.97.1 / LLVM 22.1.6, generic CPU, release optimization
and fat LTO. The exact CPU model was not verified. Other architectures,
cold-cache latency and concurrent allocator contention were not benchmarked.

Before separating the branches, cache-enabled measurements showed 2.6%, 5.7%
and 6.9% less encoding time at 256 KiB, 1 MiB and 4 MiB. Both sides of those
comparisons included the independent pair formatter; they are supporting
measurements, not the standalone-branch results above.

## Allocation tradeoff

At 1 MiB, peak temporary live memory falls from 50,383,888 to 45,574,053 bytes
(9.5%), but allocation calls rise from 37 to 65,590, and total allocation requests
rise from 62,161,312 to 72,510,009 bytes. Default builds retain no allocated bytes
after return. Recursive vectors cause this traffic; the latency measurements
include its single-thread cost. These counters are unchanged by the pair formatter.

An experimental scratch arena reduced calls to 57, but its incremental encoder
gain was 2.76%, below the optimization workflow's 3% retention threshold, so it
is not part of this change. This tradeoff is relevant for allocation-sensitive
or concurrent workloads. Raw counters are in
[baseline allocations](large-baseline-allocations.jsonl) and
[new allocations](large-final-allocations.jsonl).

## Correctness and reproduction

New tests compare with bs58 and Malachite through 1 MiB, covering maximum digits,
sparse values, zero runs, leading zeros, exact and short buffers, and unwritten
tail guards. Product tests exercise prepared-spectrum prefixes and fused-add
boundaries. SIMD tests cover extreme residues and coefficients, randomized scalar
parity, unaligned slices, guard regions and invalid lengths. Compile-time bounds
establish exact two-prime reconstruction and prevent carry overflow.

Build fixtures with `scripts/code_size.py --backends auto --directions both
--kinds cdylib --cpu generic --strip-tool /path/to/llvm-strip`, using a separate
checkout and output directory per revision. Then run
`python3 docs/benchmarks/large-input-encoding/compare.py OLD.dylib NEW.dylib results.json SIZES...`.
Add `--cache` to both builds when measuring cached behavior. Use
`tools/resource-profile` with `count-allocations` for counters, without it for timing.

The multiplication-tree algorithm was independently implemented after studying
[FLINT's destination-radix conversion](https://raw.githubusercontent.com/flintlib/flint/main/src/radix/set_mpn.c).
No third-party implementation was copied into the library.

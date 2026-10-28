# Pair-table formatting for small Base58 inputs

The encoder previously extracted each five-character group with five serial
division/remainder steps. It now emits the leading digit and reuses the existing
`matrix::four_digits` pair table for the remaining four. This preserves the public
API and output, adds no heap allocation or unsafe code, and does not change dispatch.

## Measurements

Public-API measurements on AArch64 macOS, Rust 1.97.1 / LLVM 22.1.6, generic CPU,
release optimization and fat LTO, compared with `09b2df62580cfd88dd436d3c1b4c536fd45a687d`:

| Input bytes | Original | Pair formatter | Less time |
| --- | ---: | ---: | ---: |
| 16 | 20.63 ns | 16.60 ns | 19.6% |
| 22 | 33.34 ns | 27.51 ns | 17.5% |
| 31 | 35.37 ns | 26.06 ns | 26.3% |
| 33 | 37.47 ns | 28.83 ns | 23.1% |
| 65 | 82.42 ns | 71.13 ns | 13.7% |
| 128 | 209.82 ns | 190.69 ns | 9.1% |
| 512 | 2172.12 ns | 2083.98 ns | 4.1% |

These are complete conversions into caller-provided buffers. Each result is the
median of three rounds, each containing nine alternating batches of at least
10 ms CPU time, with both immutable shared libraries in the same process.
Outputs and decoded round trips are checked before timing. The exact CPU model
was not verified; other architectures and cold-cache latency were not measured.

The 32-byte fixed-path result in the raw data has a large unexplained layout
effect despite unchanged implementation. It is excluded from attributed gains.
The benefit tapers at larger sizes: 4096 bytes improve only 0.8% in this run.

The `no_alloc` fixture retains 6712 additional read-only bytes because the linker
previously discarded the pair table. Its code shrinks by 296 bytes; the automatic
fixture's code shrinks by 572 bytes. Mach-O padding keeps stripped file sizes
unchanged. See [original sizes](size-baseline.csv) and [new sizes](size-pairs.csv).

## Correctness and reproduction

`five_digits_matches_division` exhausts all three-digit suffixes at both ends of
the five-digit range, every single-digit radix boundary and its neighbors, and
10,000 seeded random values against an independent serial-division reference.
Existing suites cover leading zeros, exact and undersized buffers, round trips,
independent Base58 implementations and the no-allocation contract.

Build original and candidate fixtures in separate checkouts using
`scripts/code_size.py --backends auto,no-alloc --directions both --kinds cdylib
--cpu generic --strip-tool /path/to/llvm-strip`. Run
`python3 docs/benchmarks/small-input-encoding/compare.py OLD.dylib NEW.dylib results.json`.
Use the same compiler and flags for both. Full measurements are in
[pairs-public.json](pairs-public.json).

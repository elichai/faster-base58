# Performance and resource tradeoffs

The [README](../README.md#performance) shows the measured competing-crate
comparison near the top. [DESIGN.md](../DESIGN.md) describes algorithms,
dispatch and memory ownership; [CORRECTNESS.md](../CORRECTNESS.md) describes
validation and its limits.

## Public conversion results

Measured **2026-11-15**, source
[`8f7e439dcee94ca4e31ae46c5ea1e28c6c81d5d7`](https://github.com/elichai/faster-base58/commit/8f7e439dcee94ca4e31ae46c5ea1e28c6c81d5d7).
Ordinary public consumers call `Encoder::bitcoin().encode(..).onto(..)` and
`Decoder::bitcoin().decode(..).onto(..)` from `code_size.py` cdylibs/modules.
Release opt-level 3, 16 codegen units, generic CPU flags (`RUSTFLAGS=""`),
fat LTO and LTO off for native, fat LTO for scalar Wasm. Default and `cache`
are separate builds. No-allocation disables default features. No `__internals`,
`check`, `compact`, allocation counting or `target-cpu=native` is enabled.
Comparators use bs58 0.5.1 and five8 1.0.0 with default features.

Effective LTO is selected with `CARGO_PROFILE_RELEASE_LTO=fat` or `off`. The
generated probe manifest says fat; its text does not capture that environment
override.

| Platform | Compiler / runtime | Clock / sampling |
| --- | --- | --- |
| Apple M5 Pro, macOS 26.7.3, AArch64, AC power | Rust 1.99.1 / LLVM 23.1.1 | Process CPU; 5 processes × 3 file copies |
| GCP AMD EPYC 9B45, Linux, generic x86-64, CPU 1 pinned | Rust 1.99.0 / LLVM 23.1.1 | Process CPU; 3 processes × 4 file copies |
| Scalar Wasm on M5 | Rust 1.99.1; Node 26.13.1 / V8 14.6.202.34 | V8 elapsed; 5 processes |

All libraries are interleaved within each process in three timed batches,
calibrated to at least 10 ms. Each retained process/copy cell is its median batch
time. Absolute medians first take each copy's median over processes, then the
median over copies. Ranges span all retained process/copy cells. Individual batch
values are not retained. All observations from each completed cohort are retained;
none are adjusted by control ratios.
Loader and timed order rotate partially; their effects are not independently balanced.

Fixtures are deterministic random bytes with a nonzero first byte. Caller buffers
are outside timing. Native buffers are 64-byte aligned and reused; native untimed
checks poison the expected output before each label. Wasm allocates and copies
caller buffers outside each timed batch and reuses them within the repeated-call
interval. Both workers check each library's output outside timing. Internal codec
workspace allocation is timed. Decode uses prevalidated UTF-8 while Base58 digit
validation remains inside the decoder. Cache timings exclude first-use table setup.

All compared artifacts are built before timing each cohort. GCP uses the shared
VM lock and CPU 1, disables transparent huge pages for the warm cohort and records
`vmstat`.
An unmeasured 10-second busy loop primes CPU 1 before the GCP warm run.
The runner applies no realtime-scheduler or ASLR overrides. M5 power and
thermal receipts bracket the complete run; background desktop activity is not
isolated. Wasm uses `--no-liftoff --no-wasm-lazy-compilation
--no-wasm-async-compilation`. Native CPU clocks and V8 elapsed time are separate
measurement scopes.

![Public conversion times on Apple M5 Pro](../assets/graphs/m5-public.svg)

![Public conversion times on AMD EPYC 9B45 (GCP, generic x86-64)](../assets/graphs/gcp-public.svg)

![Public conversion times on Scalar WebAssembly on Apple M5 Pro (Node/V8)](../assets/graphs/wasm-public.svg)

### Complete warm timings

Each cell is **encode / decode**. A range is shown when the observed process/copy
maximum exceeds the minimum by more than 3%; it is not a confidence interval.
Logarithmic figures omit zero bytes.

<details>
<summary>Apple M5 Pro / fat LTO</summary>

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 B | 1.43 ns [1.33 ns, 1.83 ns] / 1.45 ns [1.41 ns, 1.76 ns] | 1.45 ns [1.32 ns, 1.8 ns] / 1.46 ns [1.4 ns, 1.71 ns] | 1.88 ns [1.76 ns, 1.95 ns] / 1.06 ns [0.937 ns, 1.29 ns] | 1.03 ns [0.979 ns, 1.17 ns] / 0.807 ns [0.669 ns, 1.07 ns] | — / — |
| 1 B | 1.74 ns [1.64 ns, 2.17 ns] / 1.71 ns [1.65 ns, 1.95 ns] | 1.76 ns [1.67 ns, 2.18 ns] / 1.72 ns [1.65 ns, 1.97 ns] | 1.91 ns [1.82 ns, 2.25 ns] / 1.19 ns [1.13 ns, 1.52 ns] | 2.22 ns [1.98 ns, 2.72 ns] / 2.07 ns [1.85 ns, 2.21 ns] | — / — |
| 8 B | 9.86 ns [9.46 ns, 15 ns] / 6.36 ns [6.18 ns, 8.66 ns] | 9.79 ns [9.39 ns, 14.8 ns] / 6.39 ns [6.16 ns, 9.66 ns] | 8.92 ns [8.67 ns, 15.4 ns] / 6.65 ns [6.36 ns, 9.96 ns] | 59 ns [56.4 ns, 74.4 ns] / 24.8 ns [23.5 ns, 34.6 ns] | — / — |
| 16 B | 13.8 ns [13.2 ns, 21.1 ns] / 7.81 ns [7.51 ns, 11.6 ns] | 13.9 ns [13.4 ns, 21.2 ns] / 7.84 ns [7.61 ns, 11.1 ns] | 13.1 ns [12.6 ns, 20.2 ns] / 7.76 ns [7.58 ns, 11.9 ns] | 215 ns [201 ns, 243 ns] / 89 ns [83.3 ns, 115 ns] | — / — |
| 21 B | 15.6 ns [14.7 ns, 22.4 ns] / 10.1 ns [9.88 ns, 20.6 ns] | 15.6 ns [15 ns, 22.9 ns] / 10.1 ns [9.92 ns, 19.4 ns] | 14.7 ns [13.9 ns, 23.1 ns] / 10.2 ns [9.91 ns, 19.9 ns] | 356 ns [343 ns, 362 ns] / 135 ns [126 ns, 172 ns] | — / — |
| 22 B | 17.3 ns [15.9 ns, 24.2 ns] / 10.9 ns [10.6 ns, 16.6 ns] | 16.6 ns [16.1 ns, 24.2 ns] / 11.2 ns [10.7 ns, 16.7 ns] | 16.4 ns [15.5 ns, 24.5 ns] / 11 ns [10.6 ns, 16.4 ns] | 427 ns [406 ns, 456 ns] / 159 ns [149 ns, 198 ns] | — / — |
| 23 B | 18.9 ns [18.1 ns, 29.6 ns] / 11.7 ns [11.3 ns, 16.8 ns] | 18.6 ns [17.9 ns, 28.6 ns] / 11.5 ns [11.1 ns, 16.3 ns] | 18.2 ns [17.5 ns, 29.7 ns] / 11.8 ns [11.3 ns, 15.4 ns] | 442 ns [424 ns, 456 ns] / 171 ns [162 ns, 217 ns] | — / — |
| 31 B | 24.4 ns [22.5 ns, 36.7 ns] / 17.5 ns [16.7 ns, 24.3 ns] | 24.2 ns [22.7 ns, 36.7 ns] / 17.6 ns [16.8 ns, 23.6 ns] | 23.6 ns [21.8 ns, 37.1 ns] / 17.4 ns [16.6 ns, 25 ns] | 810 ns [732 ns, 843 ns] / 335 ns [310 ns, 358 ns] | — / — |
| 32 B | 11.9 ns [11.3 ns, 15.4 ns] / 13.8 ns [13.3 ns, 17.8 ns] | 12.1 ns [11.6 ns, 16.2 ns] / 13.8 ns [13.4 ns, 23.9 ns] | 9.93 ns [9.58 ns, 13.9 ns] / 13.9 ns [13.4 ns, 26.1 ns] | 836 ns [780 ns, 873 ns] / 342 ns [323 ns, 352 ns] | 34.1 ns [30.1 ns, 48 ns] / 32.7 ns [30.5 ns, 53.5 ns] |
| 33 B | 23.9 ns [23.1 ns, 28.3 ns] / 14.2 ns [13.9 ns, 22.2 ns] | 23.9 ns [22.9 ns, 28.4 ns] / 14.2 ns [13.9 ns, 22.7 ns] | 23.5 ns [22.3 ns, 28.6 ns] / 14.2 ns [13.9 ns, 23.6 ns] | 869 ns [818 ns, 920 ns] / 370 ns [347 ns, 386 ns] | — / — |
| 52 B | 45.6 ns [42.9 ns, 59.3 ns] / 23.1 ns [22.3 ns, 32.6 ns] | 48.5 ns [45.6 ns, 64.8 ns] / 23.4 ns [22.7 ns, 33.4 ns] | 46 ns [43.3 ns, 61.3 ns] / 23.2 ns [22.5 ns, 32 ns] | 2.07 µs [1.93 µs, 2.47 µs] / 1 µs [910 ns, 1.11 µs] | — / — |
| 63 B | 59.3 ns [54.3 ns, 79.5 ns] / 29.7 ns [29.2 ns, 44.7 ns] | 55.8 ns [51.9 ns, 73.8 ns] / 29.9 ns [29.6 ns, 45.3 ns] | 60.3 ns [55.7 ns, 75.1 ns] / 29.6 ns [29 ns, 44.3 ns] | 3.1 µs [2.9 µs, 3.77 µs] / 1.35 µs [1.28 µs, 1.45 µs] | — / — |
| 64 B | 24.4 ns [23 ns, 31.6 ns] / 26.3 ns [25.4 ns, 36.6 ns] | 24.3 ns [23 ns, 33.5 ns] / 26.4 ns [25.4 ns, 36.7 ns] | 24.2 ns [23 ns, 31.9 ns] / 25.9 ns [25.1 ns, 36.3 ns] | 3.17 µs [2.92 µs, 3.82 µs] / 1.38 µs [1.25 µs, 1.48 µs] | 80.7 ns [72.4 ns, 99.8 ns] / 96.8 ns [91.4 ns, 139 ns] |
| 65 B | 59.2 ns [55 ns, 74.1 ns] / 27 ns [26.3 ns, 43.1 ns] | 68.3 ns [63.6 ns, 76.5 ns] / 26.7 ns [26 ns, 42.2 ns] | 59.9 ns [55.2 ns, 79.1 ns] / 26.7 ns [26.1 ns, 41.1 ns] | 3.35 µs [3.11 µs, 3.82 µs] / 1.31 µs [1.25 µs, 1.5 µs] | — / — |
| 128 B | 150 ns [140 ns, 177 ns] / 64.1 ns [61.9 ns, 91 ns] | 114 ns [107 ns, 123 ns] / 64.8 ns [62.9 ns, 88 ns] | 155 ns [147 ns, 187 ns] / 64.5 ns [62.7 ns, 92.4 ns] | 14.4 µs [13.4 µs, 15.2 µs] / 4.94 µs [4.65 µs, 5.73 µs] | — / — |
| 256 B | 493 ns [479 ns, 676 ns] / 166 ns [160 ns, 230 ns] | 226 ns [222 ns, 305 ns] / 168 ns [161 ns, 221 ns] | 514 ns [498 ns, 770 ns] / 165 ns [160 ns, 219 ns] | 60.7 µs [56.8 µs, 66.2 µs] / 20.6 µs [19.6 µs, 22 µs] | — / — |
| 512 B | 1.84 µs [1.77 µs, 2.5 µs] / 529 ns [502 ns, 624 ns] | 587 ns [568 ns, 791 ns] / 518 ns [498 ns, 579 ns] | 1.95 µs [1.87 µs, 2.65 µs] / 529 ns [507 ns, 617 ns] | 248 µs [234 µs, 287 µs] / 82.5 µs [76.5 µs, 85.9 µs] | — / — |
| 1 KiB | 7.03 µs [6.54 µs, 11.1 µs] / 1.81 µs [1.72 µs, 2.42 µs] | 1.97 µs [1.9 µs, 2.74 µs] / 1.68 µs [1.65 µs, 2.18 µs] | 7.54 µs [7.08 µs, 9.68 µs] / 1.81 µs [1.75 µs, 2.44 µs] | 1.01 ms [938 µs, 1.04 ms] / 333 µs [319 µs, 350 µs] | — / — |
| 1,504 B | 14.6 µs [14 µs, 21.1 µs] / 3.74 µs [3.6 µs, 5.17 µs] | 4.03 µs [3.93 µs, 5.57 µs] / 3.46 µs [3.4 µs, 4.68 µs] | 15.7 µs [15.2 µs, 23 µs] / 3.66 µs [3.56 µs, 5.16 µs] | 2.18 ms [2.04 ms, 2.25 ms] / 725 µs [690 µs, 783 µs] | — / — |
| 1,875 B | 22.5 µs [21.7 µs, 36.4 µs] / 5.77 µs [5.33 µs, 7.4 µs] | 6.01 µs [5.93 µs, 8.46 µs] / 5.58 µs [5.03 µs, 6.85 µs] | 24.3 µs [23.7 µs, 38.5 µs] / 5.94 µs [5.41 µs, 7.82 µs] | 3.4 ms [3.17 ms, 3.62 ms] / 1.16 ms [1.05 ms, 1.18 ms] | — / — |
| 2 KiB | 28.3 µs [26.2 µs, 41.6 µs] / 6.96 µs [6.66 µs, 9.57 µs] | 7.42 µs [6.85 µs, 9.83 µs] / 6.2 µs [5.91 µs, 8.18 µs] | 29.4 µs [28.6 µs, 35 µs] / 6.68 µs [6.43 µs, 9.48 µs] | 4.13 ms [3.81 ms, 4.64 ms] / 1.34 ms [1.25 ms, 1.41 ms] | — / — |
| 4 KiB | 73.6 µs [71.1 µs, 97.5 µs] / 26.2 µs [25 µs, 33.3 µs] | 29.5 µs [28.8 µs, 36.9 µs] / 26.2 µs [24.4 µs, 30.3 µs] | 108 µs [104 µs, 174 µs] / 26.9 µs [25.7 µs, 32.8 µs] | 16.1 ms [15.2 ms, 16.9 ms] / 5.44 ms [5.06 ms, 5.54 ms] | — / — |
| 8 KiB | 170 µs [166 µs, 223 µs] / 76.7 µs [73 µs, 95.7 µs] | 135 µs [120 µs, 176 µs] / 71.3 µs [69.2 µs, 90.1 µs] | 432 µs [400 µs, 458 µs] / 100 µs [95.2 µs, 127 µs] | 65.2 ms [61.1 ms, 67 ms] / 21.8 ms [20.4 ms, 22.2 ms] | — / — |
| 16 KiB | 379 µs [366 µs, 498 µs] / 181 µs [175 µs, 236 µs] | 374 µs [364 µs, 487 µs] / 173 µs [170 µs, 234 µs] | 1.64 ms [1.57 ms, 2.45 ms] / 391 µs [377 µs, 491 µs] | 260 ms [245 ms, 268 ms] / 86.5 ms [81.3 ms, 90.3 ms] | — / — |
| 24 KiB | 756 µs [729 µs, 905 µs] / 314 µs [300 µs, 380 µs] | 720 µs [679 µs, 899 µs] / 308 µs [292 µs, 388 µs] | 3.74 ms [3.54 ms, 4.81 ms] / 874 µs [835 µs, 1.08 ms] | 588 ms [551 ms, 603 ms] / 195 ms [183 ms, 201 ms] | — / — |
| 28 KiB | 798 µs [788 µs, 1.03 ms] / 378 µs [369 µs, 503 µs] | 789 µs [772 µs, 1 ms] / 364 µs [354 µs, 492 µs] | 5.03 ms [4.86 ms, 6.19 ms] / 1.19 ms [1.15 ms, 1.64 ms] | 798 ms [766 ms, 820 ms] / 265 ms [253 ms, 274 ms] | — / — |
| 32 KiB | 916 µs [838 µs, 1.02 ms] / 431 µs [423 µs, 588 µs] | 833 µs [818 µs, 1.09 ms] / 421 µs [408 µs, 548 µs] | 6.52 ms [6.33 ms, 8.79 ms] / 1.6 ms [1.53 ms, 2.08 ms] | 1.04 s [987 ms, 1.1 s] / 356 ms [339 ms, 404 ms] | — / — |
| 36 KiB | 1.41 ms [1.37 ms, 1.94 ms] / 537 µs [527 µs, 762 µs] | 1.21 ms [1.16 ms, 1.57 ms] / 516 µs [507 µs, 673 µs] | 8.35 ms [8.14 ms, 12.2 ms] / 1.99 ms [1.9 ms, 2.58 ms] | 1.32 s [1.27 s, 1.41 s] / 438 ms [415 ms, 512 ms] | — / — |
| 48 KiB | 1.68 ms [1.64 ms, 1.83 ms] / 734 µs [720 µs, 909 µs] | 1.65 ms [1.61 ms, 1.83 ms] / 699 µs [691 µs, 885 µs] | 14.8 ms [14.3 ms, 15.2 ms] / 3.46 ms [3.39 ms, 4.56 ms] | 2.25 s [2.2 s, 2.36 s] / 755 ms [733 ms, 812 ms] | — / — |
| 64 KiB | 1.88 ms [1.84 ms, 2.5 ms] / 994 µs [968 µs, 1.42 ms] | 1.86 ms [1.84 ms, 2.41 ms] / 969 µs [952 µs, 1.37 ms] | 25.9 ms [25.7 ms, 41.1 ms] / 6.33 ms [5.99 ms, 8.61 ms] | 4 s [3.92 s, 4.38 s] / 1.36 s [1.3 s, 1.5 s] | — / — |
| 128 KiB | 4.13 ms [4.07 ms, 5.63 ms] / 2.26 ms [2.23 ms, 3.17 ms] | 4.11 ms [4.04 ms, 5.56 ms] / 2.22 ms [2.19 ms, 3.05 ms] | — / — | — / — | — / — |
| 144 KiB | 6.95 ms [6.67 ms, 9.38 ms] / 2.85 ms [2.75 ms, 4.17 ms] | 6.16 ms [5.87 ms, 8.21 ms] / 2.82 ms [2.68 ms, 4.09 ms] | — / — | — / — | — / — |
| 256 KiB | 9.36 ms [8.96 ms, 13.3 ms] / 5.21 ms [5.06 ms, 7.43 ms] | 9.17 ms [8.91 ms, 13.2 ms] / 5.15 ms [5 ms, 7.29 ms] | — / — | — / — | — / — |
| 512 KiB | 21.5 ms [19.7 ms, 27.7 ms] / 11.7 ms [11.4 ms, 16.8 ms] | 21.1 ms [19.6 ms, 27.7 ms] / 11.5 ms [11.3 ms, 16.6 ms] | — / — | — / — | — / — |
| 1 MiB | 44.4 ms [43.3 ms, 60 ms] / 26.2 ms [25.6 ms, 36.1 ms] | 44.8 ms [43.8 ms, 60.8 ms] / 26 ms [25.4 ms, 34.2 ms] | — / — | — / — | — / — |
| 2 MiB | 96.7 ms [94.2 ms, 133 ms] / 57.6 ms [57 ms, 80.5 ms] | 96.6 ms [94.8 ms, 133 ms] / 57.8 ms [57.3 ms, 80.7 ms] | — / — | — / — | — / — |
| 4 MiB | 208 ms [206 ms, 293 ms] / 127 ms [125 ms, 176 ms] | 210 ms [206 ms, 292 ms] / 128 ms [126 ms, 178 ms] | — / — | — / — | — / — |

</details>

<details>
<summary>Apple M5 Pro / off LTO</summary>

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 B | 1.48 ns [1.33 ns, 1.8 ns] / 1.85 ns | 1.53 ns [1.34 ns, 1.81 ns] / 1.87 ns | 1.83 ns / 1 ns [0.961 ns, 1.13 ns] | 1.22 ns [1.17 ns, 1.38 ns] / 1.83 ns | — / — |
| 1 B | 1.75 ns [1.59 ns, 1.92 ns] / 2.27 ns [2.25 ns, 2.33 ns] | 1.86 ns [1.64 ns, 1.94 ns] / 2.26 ns [2.21 ns, 2.28 ns] | 1.9 ns [1.86 ns, 1.93 ns] / 1.27 ns [1.24 ns, 1.39 ns] | 2.54 ns [2.4 ns, 2.57 ns] / 2.68 ns [2.32 ns, 2.8 ns] | — / — |
| 8 B | 23.4 ns [22.5 ns, 25.7 ns] / 16.1 ns [15.7 ns, 16.4 ns] | 23.5 ns [22.8 ns, 25.2 ns] / 16.2 ns [15.9 ns, 16.4 ns] | 24.4 ns [22.6 ns, 25.3 ns] / 15.2 ns [14.7 ns, 16.2 ns] | 78.7 ns [71.7 ns, 83.6 ns] / 26.1 ns [25.7 ns, 26.8 ns] | — / — |
| 16 B | 37.6 ns [34.5 ns, 38.8 ns] / 22.4 ns [21.8 ns, 22.7 ns] | 35.1 ns [34 ns, 38.6 ns] / 22.3 ns | 37 ns [35 ns, 38.9 ns] / 22.2 ns [21.8 ns, 22.7 ns] | 212 ns [208 ns, 215 ns] / 83.3 ns | — / — |
| 21 B | 38 ns [35.7 ns, 39.6 ns] / 26.5 ns [26.3 ns, 27.4 ns] | 36.7 ns [34.6 ns, 39.6 ns] / 26.5 ns | 38.9 ns [36.1 ns, 40.9 ns] / 26.1 ns [25.8 ns, 26.9 ns] | 339 ns / 128 ns [127 ns, 132 ns] | — / — |
| 22 B | 40.2 ns [36.8 ns, 42.2 ns] / 32.6 ns [32.1 ns, 33.4 ns] | 38.7 ns [36.7 ns, 43.7 ns] / 32.8 ns [32.3 ns, 33.5 ns] | 40.9 ns [37.9 ns, 43.2 ns] / 32.7 ns | 406 ns [404 ns, 432 ns] / 152 ns | — / — |
| 23 B | 44.5 ns [42 ns, 49.4 ns] / 35 ns [34.2 ns, 35.5 ns] | 45.1 ns [43.5 ns, 49.7 ns] / 34.9 ns [34.4 ns, 35.8 ns] | 46.9 ns [45.7 ns, 51.7 ns] / 33.3 ns [32.3 ns, 38.3 ns] | 440 ns [427 ns, 448 ns] / 165 ns [162 ns, 170 ns] | — / — |
| 31 B | 62.2 ns [58.7 ns, 65.6 ns] / 44.6 ns [43.6 ns, 45.6 ns] | 62.4 ns [58.8 ns, 65.7 ns] / 44.8 ns [43.9 ns, 46.2 ns] | 60.9 ns [56.8 ns, 64 ns] / 45.3 ns [43.7 ns, 49.9 ns] | 750 ns / 320 ns [309 ns, 335 ns] | — / — |
| 32 B | 23.6 ns [22.9 ns, 24.5 ns] / 39.3 ns [38.7 ns, 40.1 ns] | 23.3 ns [23.2 ns, 24.2 ns] / 39.6 ns [39.1 ns, 40.9 ns] | 22.9 ns [22.6 ns, 23.9 ns] / 38.7 ns [37.8 ns, 42.2 ns] | 839 ns [810 ns, 855 ns] / 331 ns [328 ns, 341 ns] | 33.2 ns [31.5 ns, 34.9 ns] / 32.5 ns |
| 33 B | 62.7 ns [59.6 ns, 66.8 ns] / 40.1 ns | 62.5 ns [59.6 ns, 67.3 ns] / 40.3 ns | 61.2 ns [58.3 ns, 65.4 ns] / 39.7 ns [39 ns, 42.6 ns] | 815 ns [806 ns, 837 ns] / 352 ns [349 ns, 364 ns] | — / — |
| 52 B | 102 ns [99.7 ns, 108 ns] / 67.1 ns [66.4 ns, 69.1 ns] | 48.2 ns [47.3 ns, 49.1 ns] / 66.8 ns | 104 ns [102 ns, 110 ns] / 66.8 ns [65.9 ns, 72 ns] | 2.04 µs [1.98 µs, 2.24 µs] / 874 ns [846 ns, 898 ns] | — / — |
| 63 B | 123 ns [121 ns, 135 ns] / 78 ns [76.2 ns, 79.3 ns] | 54.9 ns [51.4 ns, 56 ns] / 78.8 ns | 128 ns [121 ns, 134 ns] / 78 ns [76.6 ns, 87 ns] | 3 µs [2.96 µs, 3.07 µs] / 1.18 µs [1.14 µs, 1.24 µs] | — / — |
| 64 B | 25.1 ns / 72.8 ns [71.7 ns, 73.9 ns] | 24.9 ns / 73.2 ns | 24.9 ns / 73 ns | 3.14 µs [3.03 µs, 3.24 µs] / 1.18 µs [1.14 µs, 1.22 µs] | 74.5 ns [73.5 ns, 82.2 ns] / 97.1 ns |
| 65 B | 128 ns [119 ns, 134 ns] / 73 ns | 71.2 ns [70.4 ns, 74.8 ns] / 73.8 ns | 125 ns [122 ns, 136 ns] / 73.7 ns [73 ns, 85.9 ns] | 3.29 µs [3.19 µs, 3.42 µs] / 1.23 µs [1.21 µs, 1.26 µs] | — / — |
| 128 B | 287 ns [278 ns, 306 ns] / 158 ns | 122 ns [120 ns, 124 ns] / 161 ns | 300 ns [284 ns, 317 ns] / 157 ns | 13.6 µs [13.4 µs, 14 µs] / 4.73 µs [4.67 µs, 4.93 µs] | — / — |
| 256 B | 754 ns [719 ns, 782 ns] / 359 ns | 241 ns [238 ns, 249 ns] / 358 ns [348 ns, 366 ns] | 764 ns [745 ns, 807 ns] / 358 ns [355 ns, 400 ns] | 56.8 µs / 18.9 µs [18.8 µs, 19.4 µs] | — / — |
| 512 B | 2.32 µs [2.29 µs, 2.41 µs] / 910 ns [870 ns, 931 ns] | 580 ns / 522 ns [516 ns, 534 ns] | 2.4 µs [2.35 µs, 2.47 µs] / 906 ns [873 ns, 1.01 µs] | 232 µs [231 µs, 242 µs] / 78.4 µs [77.1 µs, 81.6 µs] | — / — |
| 1 KiB | 7.9 µs [7.68 µs, 8.12 µs] / 2.56 µs [2.48 µs, 2.58 µs] | 1.94 µs / 1.66 µs | 8.05 µs [7.97 µs, 8.48 µs] / 2.54 µs [2.46 µs, 2.63 µs] | 939 µs [935 µs, 1 ms] / 314 µs [312 µs, 322 µs] | — / — |
| 1,504 B | 15.9 µs [15.7 µs, 16.4 µs] / 4.79 µs [4.7 µs, 4.88 µs] | 4.02 µs / 3.44 µs | 16.4 µs [16.1 µs, 16.8 µs] / 4.78 µs [4.69 µs, 4.94 µs] | 2.03 ms / 685 µs [675 µs, 710 µs] | — / — |
| 1,875 B | 24.1 µs [23.6 µs, 24.6 µs] / 6.84 µs | 5.95 µs [5.85 µs, 6.05 µs] / 5.09 µs | 25 µs [24.7 µs, 25.5 µs] / 6.81 µs [6.72 µs, 7.08 µs] | 3.18 ms [3.14 ms, 3.36 ms] / 1.06 ms [1.05 ms, 1.11 ms] | — / — |
| 2 KiB | 28.2 µs [27.8 µs, 28.7 µs] / 8.02 µs | 6.88 µs [6.84 µs, 7.06 µs] / 5.93 µs | 29.3 µs / 8.03 µs [7.93 µs, 8.3 µs] | 3.77 ms / 1.27 ms [1.25 ms, 1.32 ms] | — / — |
| 4 KiB | 79.3 µs [77.6 µs, 80.4 µs] / 27.9 µs [27.6 µs, 28.6 µs] | 29 µs [28.8 µs, 30.2 µs] / 24.7 µs [24.3 µs, 25.4 µs] | 112 µs / 27.8 µs [27.6 µs, 28.6 µs] | 15.2 ms [15.1 ms, 15.6 ms] / 5.1 ms | — / — |
| 8 KiB | 178 µs [175 µs, 183 µs] / 80.4 µs [78.2 µs, 82.9 µs] | 119 µs [117 µs, 126 µs] / 72.6 µs | 432 µs / 102 µs | 60.8 ms / 20.4 ms [20.1 ms, 20.9 ms] | — / — |
| 16 KiB | 397 µs [389 µs, 403 µs] / 189 µs [188 µs, 194 µs] | 393 µs / 172 µs | 1.71 ms / 388 µs | 244 ms / 81.3 ms | — / — |
| 24 KiB | 776 µs [765 µs, 800 µs] / 322 µs [317 µs, 330 µs] | 702 µs [692 µs, 725 µs] / 297 µs | 3.83 ms [3.78 ms, 3.95 ms] / 862 µs | 552 ms [546 ms, 580 ms] / 184 ms [182 ms, 188 ms] | — / — |
| 28 KiB | 834 µs [815 µs, 854 µs] / 391 µs [387 µs, 419 µs] | 821 µs / 362 µs [359 µs, 382 µs] | 5.18 ms [5.13 ms, 5.35 ms] / 1.16 ms [1.16 ms, 1.23 ms] | 752 ms [748 ms, 771 ms] / 250 ms [248 ms, 275 ms] | — / — |
| 32 KiB | 879 µs [866 µs, 894 µs] / 442 µs | 875 µs / 410 µs [407 µs, 424 µs] | 6.75 ms / 1.51 ms [1.5 ms, 1.61 ms] | 981 ms / 327 ms [324 ms, 334 ms] | — / — |
| 36 KiB | 1.42 ms / 553 µs | 1.15 ms / 514 µs | 8.48 ms / 1.91 ms | 1.24 s / 413 ms | — / — |
| 48 KiB | 1.74 ms [1.71 ms, 1.76 ms] / 750 µs [740 µs, 807 µs] | 1.62 ms [1.6 ms, 1.66 ms] / 698 µs [690 µs, 737 µs] | 15.1 ms [14.9 ms, 15.7 ms] / 3.38 ms [3.33 ms, 3.71 ms] | 2.21 s [2.19 s, 2.27 s] / 733 ms [728 ms, 800 ms] | — / — |
| 64 KiB | 1.95 ms [1.91 ms, 2.05 ms] / 1.03 ms [1.01 ms, 1.05 ms] | 1.94 ms [1.91 ms, 2.1 ms] / 969 µs | 27.1 ms [26.7 ms, 27.6 ms] / 6.1 ms [5.94 ms, 6.19 ms] | 3.93 s [3.93 s, 4.11 s] / 1.31 s [1.3 s, 1.36 s] | — / — |
| 128 KiB | 4.3 ms [4.28 ms, 4.44 ms] / 2.35 ms | 4.31 ms [4.25 ms, 4.74 ms] / 2.21 ms | — / — | — / — | — / — |
| 144 KiB | 6.98 ms [6.93 ms, 7.18 ms] / 2.88 ms | 5.94 ms / 2.73 ms | — / — | — / — | — / — |
| 256 KiB | 9.41 ms / 5.29 ms [5.21 ms, 5.42 ms] | 9.36 ms / 5.03 ms [5.01 ms, 5.21 ms] | — / — | — / — | — / — |
| 512 KiB | 20.6 ms [20.4 ms, 21.2 ms] / 11.9 ms | 20.5 ms [20.4 ms, 21.5 ms] / 11.4 ms | — / — | — / — | — / — |
| 1 MiB | 45.1 ms / 26.4 ms | 45.6 ms / 25.6 ms | — / — | — / — | — / — |
| 2 MiB | 98.7 ms [97.8 ms, 106 ms] / 58.6 ms [58.3 ms, 60.4 ms] | 99.2 ms [98.2 ms, 111 ms] / 57.8 ms [57.4 ms, 59.8 ms] | — / — | — / — | — / — |
| 4 MiB | 213 ms / 130 ms | 214 ms / 128 ms | — / — | — / — | — / — |

</details>

<details>
<summary>AMD EPYC 9B45 (GCP, generic x86-64) / fat LTO</summary>

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 B | 2.56 ns [2.52 ns, 2.62 ns] / 1.85 ns [1.82 ns, 1.88 ns] | 2.64 ns [2.62 ns, 2.71 ns] / 1.98 ns | 1.73 ns [1.7 ns, 1.8 ns] / 1.72 ns [1.7 ns, 1.76 ns] | 1.59 ns [1.58 ns, 1.63 ns] / 1.86 ns [1.82 ns, 1.88 ns] | — / — |
| 1 B | 2.39 ns [2.31 ns, 2.51 ns] / 2.2 ns | 2.35 ns [2.32 ns, 2.43 ns] / 2.21 ns | 2 ns [1.96 ns, 2.07 ns] / 1.72 ns | 3.05 ns [2.95 ns, 3.17 ns] / 2.74 ns [2.68 ns, 2.89 ns] | — / — |
| 8 B | 19.4 ns / 10.5 ns [10.3 ns, 10.6 ns] | 19.3 ns / 10.5 ns [10.2 ns, 10.6 ns] | 19.2 ns / 10.5 ns [10.3 ns, 11.3 ns] | 60 ns / 37.3 ns [36.3 ns, 37.5 ns] | — / — |
| 16 B | 25.9 ns / 14.4 ns [14.2 ns, 16 ns] | 25.7 ns / 14.3 ns [14 ns, 15.9 ns] | 28 ns / 16.1 ns [14.5 ns, 16.6 ns] | 214 ns / 98.4 ns | — / — |
| 21 B | 28.3 ns / 18.5 ns | 28.2 ns / 19.2 ns | 30.9 ns / 18.9 ns [18.6 ns, 20.5 ns] | 351 ns / 147 ns | — / — |
| 22 B | 30.9 ns / 19.1 ns | 30.9 ns / 19.2 ns | 34.8 ns / 19.3 ns [19.2 ns, 21.8 ns] | 411 ns / 166 ns | — / — |
| 23 B | 37 ns / 20.9 ns | 36.9 ns / 20.9 ns | 40.5 ns / 20.9 ns [20.9 ns, 22.1 ns] | 434 ns / 175 ns | — / — |
| 31 B | 43.8 ns / 25.9 ns | 43.9 ns / 25.7 ns | 46.4 ns / 25.9 ns [25.6 ns, 27.3 ns] | 852 ns / 313 ns | — / — |
| 32 B | 30.3 ns / 24.4 ns | 30.2 ns / 23.8 ns | 30.2 ns / 23.9 ns [23.7 ns, 24.8 ns] | 911 ns / 327 ns | 55.8 ns / 49.4 ns [48.8 ns, 50.7 ns] |
| 33 B | 43.6 ns / 25 ns | 43.6 ns / 24.9 ns | 46.5 ns / 25.1 ns | 952 ns / 339 ns | — / — |
| 52 B | 83.5 ns / 40.5 ns | 77.3 ns / 41 ns | 93.1 ns / 41 ns [41 ns, 42.8 ns] | 2.54 µs / 832 ns | — / — |
| 63 B | 104 ns / 48.5 ns | 87.5 ns [87 ns, 90.2 ns] / 48.7 ns | 111 ns / 49.4 ns | 3.81 µs / 1.23 µs | — / — |
| 64 B | 31.7 ns [31.3 ns, 37.9 ns] / 45.1 ns | 31.8 ns [31.5 ns, 36.6 ns] / 45.2 ns | 31.7 ns / 45.5 ns | 3.9 µs / 1.25 µs | 116 ns [116 ns, 120 ns] / 129 ns |
| 65 B | 107 ns [106 ns, 111 ns] / 46.6 ns | 101 ns [100 ns, 107 ns] / 47.6 ns | 117 ns / 48.1 ns | 4.07 µs / 1.31 µs | — / — |
| 128 B | 312 ns / 109 ns [107 ns, 110 ns] | 166 ns / 108 ns | 328 ns / 108 ns | 16.8 µs / 5.27 µs | — / — |
| 256 B | 1.06 µs [1.05 µs, 1.09 µs] / 279 ns [278 ns, 288 ns] | 395 ns / 277 ns [276 ns, 287 ns] | 1.11 µs / 280 ns [276 ns, 289 ns] | 70 µs / 22.2 µs [22.1 µs, 22.9 µs] | — / — |
| 512 B | 3.24 µs [3.22 µs, 3.36 µs] / 856 ns [851 ns, 894 ns] | 1.09 µs [1.08 µs, 1.12 µs] / 864 ns [856 ns, 899 ns] | 4.08 µs [4.05 µs, 4.2 µs] / 866 ns [856 ns, 898 ns] | 283 µs / 93.3 µs [92.7 µs, 95.9 µs] | — / — |
| 1 KiB | 7.35 µs [7.21 µs, 7.52 µs] / 2.56 µs [2.5 µs, 2.61 µs] | 3.34 µs [3.33 µs, 3.46 µs] / 2.39 µs [2.36 µs, 2.44 µs] | 15.8 µs [15.7 µs, 16.3 µs] / 2.94 µs [2.91 µs, 3 µs] | 1.14 ms [1.13 ms, 1.17 ms] / 375 µs [371 µs, 384 µs] | — / — |
| 1,504 B | 12.3 µs / 4.4 µs [4.36 µs, 4.52 µs] | 6.84 µs / 5.1 µs | 33.6 µs / 5.96 µs | 2.45 ms / 799 µs | — / — |
| 1,875 B | 15.8 µs / 5.51 µs | 10.2 µs / 7.71 µs | 51.9 µs / 9 µs [8.95 µs, 9.36 µs] | 3.81 ms / 1.24 ms | — / — |
| 2 KiB | 17.9 µs [17.6 µs, 18.4 µs] / 6.15 µs [6.06 µs, 6.42 µs] | 11.8 µs [11.7 µs, 12.1 µs] / 8.98 µs [8.9 µs, 9.37 µs] | 61.5 µs [61.5 µs, 63.7 µs] / 10.7 µs [10.6 µs, 11.1 µs] | 4.53 ms [4.53 ms, 4.71 ms] / 1.47 ms [1.47 ms, 1.53 ms] | — / — |
| 4 KiB | 48.2 µs [47.6 µs, 49.8 µs] / 16 µs [15.7 µs, 16.9 µs] | 40 µs [39.7 µs, 41.9 µs] / 15.7 µs [15.6 µs, 16.9 µs] | 244 µs / 41.5 µs [41.4 µs, 43.4 µs] | 18.2 ms / 5.86 ms [5.83 ms, 6.1 ms] | — / — |
| 8 KiB | 142 µs [142 µs, 146 µs] / 46 µs | 119 µs [119 µs, 123 µs] / 45.1 µs [44.7 µs, 47.3 µs] | 964 µs / 162 µs [161 µs, 167 µs] | 72.6 ms / 23.4 ms | — / — |
| 16 KiB | 430 µs / 136 µs | 363 µs / 134 µs [133 µs, 138 µs] | 3.83 ms / 636 µs | 290 ms / 93.5 ms | — / — |
| 24 KiB | 793 µs [785 µs, 809 µs] / 264 µs | 700 µs [697 µs, 721 µs] / 268 µs | 8.63 ms / 1.41 ms | 654 ms / 210 ms | — / — |
| 28 KiB | 955 µs [943 µs, 977 µs] / 353 µs | 864 µs / 343 µs | 11.8 ms / 1.92 ms | 891 ms / 285 ms | — / — |
| 32 KiB | 1.01 ms / 412 µs | 923 µs / 381 µs [378 µs, 393 µs] | 15.3 ms / 2.5 ms | 1.16 s / 372 ms | — / — |
| 36 KiB | 1.44 ms [1.44 ms, 1.49 ms] / 528 µs [522 µs, 544 µs] | 1.39 ms / 492 µs [490 µs, 506 µs] | 19.1 ms / 3.17 ms | 1.47 s / 477 ms | — / — |
| 48 KiB | 1.98 ms / 792 µs | 2.07 ms / 737 µs | 34.4 ms / 5.6 ms | 2.61 s / 838 ms | — / — |
| 64 KiB | 2.23 ms / 1.03 ms | 2.11 ms / 829 µs | 61.4 ms / 9.96 ms | 4.64 s / 1.49 s | — / — |
| 128 KiB | 4.84 ms / 2.34 ms | 4.43 ms / 1.93 ms [1.92 ms, 1.98 ms] | — / — | — / — | — / — |
| 144 KiB | 7.48 ms / 3.04 ms | 6.83 ms / 2.4 ms | — / — | — / — | — / — |
| 256 KiB | 10.3 ms / 4.84 ms | 9.59 ms / 4.43 ms | — / — | — / — | — / — |
| 512 KiB | 22.4 ms [21.9 ms, 22.8 ms] / 10.9 ms | 21 ms [20.9 ms, 21.8 ms] / 10.2 ms | — / — | — / — | — / — |
| 1 MiB | 48.5 ms [48.3 ms, 49.9 ms] / 24.3 ms | 48.8 ms / 22.9 ms | — / — | — / — | — / — |
| 2 MiB | 107 ms [104 ms, 108 ms] / 54.3 ms [53.4 ms, 55.6 ms] | 106 ms [104 ms, 108 ms] / 52.9 ms [51.8 ms, 53.9 ms] | — / — | — / — | — / — |
| 4 MiB | 230 ms [224 ms, 235 ms] / 119 ms [118 ms, 122 ms] | 230 ms [224 ms, 236 ms] / 116 ms [116 ms, 119 ms] | — / — | — / — | — / — |

</details>

<details>
<summary>AMD EPYC 9B45 (GCP, generic x86-64) / off LTO</summary>

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 B | 2.44 ns [2.43 ns, 2.67 ns] / 2.56 ns | 2.43 ns / 2.46 ns | 2.72 ns / 1.95 ns | 2.19 ns / 2.82 ns | — / — |
| 1 B | 2.92 ns / 2.8 ns | 2.92 ns / 2.79 ns | 2.57 ns [2.42 ns, 2.63 ns] / 1.7 ns | 3.7 ns / 3.87 ns [3.8 ns, 4.14 ns] | — / — |
| 8 B | 33.5 ns / 28.4 ns [28.2 ns, 29.9 ns] | 32.6 ns / 28.3 ns [28.1 ns, 30 ns] | 35.2 ns / 27.2 ns | 60.4 ns / 36.8 ns [36.7 ns, 38.1 ns] | — / — |
| 16 B | 53.5 ns / 42.5 ns [41 ns, 47.8 ns] | 48 ns / 41.8 ns [41 ns, 42.4 ns] | 54.4 ns / 39.9 ns | 213 ns / 99.3 ns [99 ns, 102 ns] | — / — |
| 21 B | 53 ns / 49.9 ns [49.4 ns, 54.2 ns] | 49.5 ns / 49.3 ns [49.1 ns, 51.6 ns] | 58 ns / 48.3 ns | 349 ns / 148 ns | — / — |
| 22 B | 57 ns / 61.2 ns [55.2 ns, 65.5 ns] | 51.4 ns / 57 ns [56.5 ns, 59.4 ns] | 61.1 ns / 59.9 ns [57 ns, 60.2 ns] | 408 ns / 168 ns | — / — |
| 23 B | 67.7 ns / 57.6 ns [57.2 ns, 63 ns] | 63.3 ns [63.1 ns, 66.2 ns] / 57.4 ns [56.9 ns, 59.3 ns] | 76.1 ns / 56 ns | 429 ns / 176 ns | — / — |
| 31 B | 89.2 ns / 72.2 ns [71.6 ns, 79.1 ns] | 82.5 ns / 72.1 ns | 94.5 ns / 70.5 ns | 842 ns / 310 ns | — / — |
| 32 B | 35.5 ns [34.7 ns, 36.6 ns] / 70.4 ns [69.8 ns, 76.2 ns] | 36.3 ns / 70.3 ns | 35.2 ns [33.9 ns, 35.7 ns] / 68.7 ns | 902 ns / 323 ns | 55.1 ns / 47.4 ns |
| 33 B | 87.8 ns / 71.6 ns [70.7 ns, 74.6 ns] | 82.7 ns / 71.2 ns | 93.3 ns / 69.4 ns | 944 ns / 337 ns | — / — |
| 52 B | 158 ns [157 ns, 163 ns] / 115 ns [113 ns, 122 ns] | 85.4 ns / 115 ns | 172 ns / 112 ns | 2.53 µs / 827 ns | — / — |
| 63 B | 190 ns / 134 ns [134 ns, 146 ns] | 95.4 ns / 135 ns | 203 ns / 133 ns [132 ns, 146 ns] | 3.79 µs / 1.22 µs [1.22 µs, 1.28 µs] | — / — |
| 64 B | 44.9 ns / 132 ns [131 ns, 143 ns] | 45 ns / 132 ns | 44.5 ns / 130 ns | 3.88 µs / 1.25 µs | 116 ns / 121 ns |
| 65 B | 193 ns / 135 ns [134 ns, 145 ns] | 112 ns / 134 ns [133 ns, 148 ns] | 205 ns / 131 ns | 4.04 µs / 1.3 µs | — / — |
| 128 B | 488 ns / 277 ns [275 ns, 284 ns] | 177 ns [176 ns, 187 ns] / 276 ns | 511 ns / 274 ns | 16.8 µs / 5.27 µs | — / — |
| 256 B | 1.4 µs / 616 ns [604 ns, 624 ns] | 409 ns [405 ns, 426 ns] / 608 ns | 1.48 µs / 616 ns [604 ns, 622 ns] | 70.4 µs / 22 µs | — / — |
| 512 B | 4.47 µs [4.43 µs, 4.56 µs] / 1.5 µs | 1.11 µs / 1.5 µs | 4.77 µs / 1.5 µs | 283 µs / 92.2 µs | — / — |
| 1 KiB | 9.65 µs / 4.03 µs | 3.42 µs / 2.43 µs | 17.1 µs / 4.11 µs | 1.14 ms / 372 µs | — / — |
| 1,504 B | 15.6 µs [15.4 µs, 16.1 µs] / 6.64 µs | 7 µs / 5.26 µs | 35.4 µs / 8.03 µs | 2.45 ms / 799 µs | — / — |
| 1,875 B | 20 µs / 8.36 µs | 10.5 µs / 7.93 µs | 54.2 µs / 11.7 µs | 3.8 ms / 1.24 ms | — / — |
| 2 KiB | 22.4 µs / 9.14 µs | 12.1 µs / 9.26 µs | 63.9 µs / 13.7 µs | 4.53 ms / 1.48 ms | — / — |
| 4 KiB | 57.1 µs / 21.9 µs | 42.3 µs / 22 µs | 246 µs / 47.5 µs | 18.1 ms / 5.88 ms | — / — |
| 8 KiB | 159 µs / 57.8 µs | 126 µs / 57.4 µs | 968 µs / 170 µs | 72.5 ms / 23.4 ms | — / — |
| 16 KiB | 462 µs / 161 µs | 378 µs [372 µs, 393 µs] / 160 µs | 3.83 ms / 629 µs | 290 ms / 93.4 ms | — / — |
| 24 KiB | 837 µs / 300 µs | 716 µs / 304 µs | 8.59 ms / 1.37 ms | 652 ms / 210 ms | — / — |
| 28 KiB | 989 µs / 396 µs | 900 µs / 386 µs | 11.7 ms / 1.86 ms [1.85 ms, 1.93 ms] | 888 ms / 286 ms | — / — |
| 32 KiB | 1.06 ms / 462 µs | 972 µs / 438 µs | 15.3 ms / 2.4 ms | 1.16 s / 373 ms | — / — |
| 36 KiB | 1.52 ms / 578 µs | 1.41 ms / 549 µs | 19.1 ms / 3.03 ms | 1.47 s / 473 ms | — / — |
| 48 KiB | 2.05 ms / 864 µs | 2.11 ms / 812 µs | 34.2 ms / 5.33 ms | 2.61 s / 840 ms | — / — |
| 64 KiB | 2.33 ms / 1.13 ms | 2.2 ms / 929 µs | 61 ms / 9.43 ms | 4.64 s / 1.49 s | — / — |
| 128 KiB | 5.09 ms / 2.55 ms | 4.65 ms / 2.15 ms | — / — | — / — | — / — |
| 144 KiB | 7.98 ms / 3.32 ms | 7.14 ms [6.99 ms, 7.21 ms] / 2.65 ms | — / — | — / — | — / — |
| 256 KiB | 10.8 ms [10.7 ms, 11 ms] / 5.29 ms | 10.1 ms [10.1 ms, 10.5 ms] / 4.89 ms [4.87 ms, 5.17 ms] | — / — | — / — | — / — |
| 512 KiB | 23.6 ms [23 ms, 23.9 ms] / 11.8 ms | 21.9 ms [21.8 ms, 22.7 ms] / 11.1 ms | — / — | — / — | — / — |
| 1 MiB | 51.3 ms / 26.3 ms [25.6 ms, 27.1 ms] | 51.2 ms / 24.8 ms [24.2 ms, 25 ms] | — / — | — / — | — / — |
| 2 MiB | 108 ms [107 ms, 111 ms] / 58 ms | 107 ms [107 ms, 111 ms] / 56.5 ms | — / — | — / — | — / — |
| 4 MiB | 237 ms / 126 ms [125 ms, 130 ms] | 237 ms / 122 ms [122 ms, 127 ms] | — / — | — / — | — / — |

</details>

<details>
<summary>Scalar WebAssembly on Apple M5 Pro (Node/V8) / fat LTO</summary>

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 B | 1.85 ns / 2.09 ns [2.04 ns, 2.2 ns] | 1.83 ns [1.78 ns, 1.87 ns] / 2.08 ns [2.06 ns, 2.23 ns] | 2.17 ns [2.16 ns, 2.29 ns] / 0.848 ns [0.803 ns, 0.897 ns] | 1.84 ns [1.79 ns, 1.98 ns] / 1.89 ns [1.85 ns, 1.96 ns] | — / — |
| 1 B | 2.18 ns [2.12 ns, 2.2 ns] / 2.53 ns [2.4 ns, 2.58 ns] | 2.13 ns [2.07 ns, 2.22 ns] / 2.51 ns [2.39 ns, 2.67 ns] | 2.73 ns [2.61 ns, 3.01 ns] / 1.16 ns [1.14 ns, 1.22 ns] | 2.77 ns / 2.37 ns [2.3 ns, 2.4 ns] | — / — |
| 8 B | 13.7 ns [13.3 ns, 13.9 ns] / 14.9 ns | 13.3 ns [13.2 ns, 13.7 ns] / 13.1 ns | 18.6 ns [18.1 ns, 19.5 ns] / 14.6 ns [14.4 ns, 15.4 ns] | 42.9 ns [42.1 ns, 45.7 ns] / 22.4 ns [21.1 ns, 23.6 ns] | — / — |
| 16 B | 20.1 ns [19.8 ns, 20.4 ns] / 23.1 ns | 20.1 ns [19.3 ns, 21 ns] / 20.8 ns | 28.2 ns [26.8 ns, 29.1 ns] / 23.1 ns | 149 ns [137 ns, 153 ns] / 67 ns | — / — |
| 21 B | 25.9 ns [25.2 ns, 27.1 ns] / 25.6 ns | 26.8 ns [26.1 ns, 28.5 ns] / 23.4 ns | 35 ns [34.1 ns, 36.4 ns] / 25.5 ns | 226 ns / 101 ns | — / — |
| 22 B | 29.9 ns [29.1 ns, 30.4 ns] / 26 ns | 30.5 ns [30.3 ns, 32.2 ns] / 24.6 ns [24 ns, 25.3 ns] | 39.8 ns [38 ns, 40.2 ns] / 26.1 ns | 268 ns [264 ns, 275 ns] / 115 ns [112 ns, 116 ns] | — / — |
| 23 B | 33.9 ns / 32.9 ns | 34.4 ns / 29.9 ns | 43.7 ns [41.3 ns, 44.4 ns] / 32.5 ns | 272 ns / 119 ns | — / — |
| 31 B | 50.6 ns [50 ns, 52.7 ns] / 49.5 ns [48.1 ns, 49.8 ns] | 50.3 ns [48.8 ns, 53.2 ns] / 47.9 ns [46.9 ns, 48.5 ns] | 58.8 ns [57.5 ns, 60 ns] / 50.6 ns | 611 ns [578 ns, 612 ns] / 205 ns | — / — |
| 32 B | 25.1 ns / 51.9 ns | 25.2 ns [24.8 ns, 25.7 ns] / 50 ns | 25.3 ns / 52.5 ns | 598 ns [594 ns, 639 ns] / 216 ns | 44.5 ns [43.3 ns, 45 ns] / 40.8 ns |
| 33 B | 49.9 ns [48.8 ns, 52.6 ns] / 52.7 ns [51.5 ns, 53.3 ns] | 49.4 ns [47.8 ns, 53.6 ns] / 51.1 ns | 58.2 ns / 53.3 ns | 625 ns / 229 ns | — / — |
| 52 B | 111 ns [111 ns, 116 ns] / 97 ns [95.5 ns, 100 ns] | 114 ns / 93.8 ns [90.5 ns, 100 ns] | 126 ns / 98.5 ns [96.9 ns, 101 ns] | 1.79 µs [1.75 µs, 1.84 µs] / 539 ns | — / — |
| 63 B | 148 ns [140 ns, 152 ns] / 133 ns [131 ns, 135 ns] | 121 ns / 127 ns [125 ns, 131 ns] | 159 ns [155 ns, 162 ns] / 134 ns [131 ns, 136 ns] | 2.71 µs / 791 ns | — / — |
| 64 B | 98.8 ns / 136 ns [134 ns, 140 ns] | 99.3 ns / 131 ns [127 ns, 134 ns] | 100 ns / 136 ns [134 ns, 143 ns] | 2.8 µs [2.73 µs, 2.83 µs] / 819 ns [816 ns, 855 ns] | 91.5 ns / 124 ns [123 ns, 127 ns] |
| 65 B | 155 ns [151 ns, 162 ns] / 135 ns | 178 ns [175 ns, 182 ns] / 130 ns [128 ns, 137 ns] | 167 ns [165 ns, 171 ns] / 140 ns [135 ns, 144 ns] | 2.91 µs [2.85 µs, 3.08 µs] / 849 ns [838 ns, 869 ns] | — / — |
| 128 B | 512 ns [499 ns, 534 ns] / 441 ns [429 ns, 443 ns] | 294 ns / 428 ns | 524 ns [513 ns, 537 ns] / 432 ns [429 ns, 442 ns] | 12.8 µs / 3.48 µs | — / — |
| 256 B | 1.87 µs [1.84 µs, 2 µs] / 1.62 µs [1.49 µs, 1.64 µs] | 855 ns / 1.58 µs [1.49 µs, 1.62 µs] | 1.96 µs [1.94 µs, 2.06 µs] / 1.62 µs [1.49 µs, 1.66 µs] | 55.4 µs [54.7 µs, 57.3 µs] / 16.2 µs | — / — |
| 512 B | 7.15 µs [7.05 µs, 7.68 µs] / 6.06 µs [5.49 µs, 6.19 µs] | 2.77 µs / 2.49 µs | 7.36 µs [7.23 µs, 7.92 µs] / 5.87 µs [5.51 µs, 6.04 µs] | 232 µs / 73 µs | — / — |
| 1 KiB | 27.6 µs [27 µs, 30.1 µs] / 23.3 µs [21 µs, 23.4 µs] | 10.1 µs / 8.8 µs [8.65 µs, 9.13 µs] | 28.3 µs [27.9 µs, 31 µs] / 23.1 µs [20.9 µs, 25 µs] | 940 µs [933 µs, 976 µs] / 310 µs [306 µs, 328 µs] | — / — |
| 1,504 B | 58 µs [57.6 µs, 64 µs] / 49.9 µs [44.5 µs, 50.4 µs] | 20.8 µs / 18.2 µs | 61 µs [58.3 µs, 65.3 µs] / 48.7 µs [44.1 µs, 49.9 µs] | 2.04 ms / 682 µs [675 µs, 696 µs] | — / — |
| 1,875 B | 91.2 µs [88.6 µs, 101 µs] / 64.2 µs [62.5 µs, 67.2 µs] | 32.2 µs / 27.6 µs | 92.5 µs [91.6 µs, 104 µs] / 69.6 µs [67.4 µs, 73.9 µs] | 3.17 ms [3.15 ms, 3.32 ms] / 1.08 ms [1.05 ms, 1.1 ms] | — / — |
| 2 KiB | 109 µs [105 µs, 114 µs] / 68 µs [66.7 µs, 69.6 µs] | 37.8 µs / 32.4 µs | 112 µs [107 µs, 121 µs] / 89.6 µs [81.1 µs, 93.1 µs] | 3.81 ms [3.76 ms, 3.9 ms] / 1.28 ms | — / — |
| 4 KiB | 286 µs / 189 µs | 146 µs / 122 µs | 445 µs [430 µs, 464 µs] / 336 µs [321 µs, 368 µs] | 15.3 ms [15 ms, 15.6 ms] / 5.18 ms [5.1 ms, 5.25 ms] | — / — |
| 8 KiB | 686 µs / 465 µs | 577 µs / 456 µs | 1.73 ms / 1.27 ms [1.24 ms, 1.37 ms] | 60.9 ms / 20.5 ms | — / — |
| 16 KiB | 1.61 ms / 1.11 ms | 1.6 ms / 1.09 ms [1.08 ms, 1.11 ms] | 6.96 ms [6.77 ms, 7.76 ms] / 5.18 ms [4.81 ms, 5.4 ms] | 243 ms / 83 ms [81.4 ms, 83.9 ms] | — / — |
| 24 KiB | 3.62 ms / 1.99 ms [1.96 ms, 2.02 ms] | 3.59 ms / 1.94 ms | 15.6 ms [15.5 ms, 17.5 ms] / 12.1 ms [11.5 ms, 13 ms] | 552 ms / 184 ms | — / — |
| 28 KiB | 3.68 ms / 2.42 ms [2.37 ms, 2.46 ms] | 3.67 ms / 2.38 ms [2.35 ms, 2.43 ms] | 21.3 ms [20.9 ms, 21.6 ms] / 16 ms [15.3 ms, 17.4 ms] | 752 ms / 254 ms [250 ms, 257 ms] | — / — |
| 32 KiB | 3.74 ms / 2.59 ms [2.58 ms, 2.7 ms] | 3.71 ms [3.69 ms, 4.53 ms] / 2.58 ms [2.53 ms, 2.68 ms] | 28.9 ms [27.6 ms, 35 ms] / 20.3 ms [19.3 ms, 22.2 ms] | 984 ms / 330 ms [328 ms, 358 ms] | — / — |
| 36 KiB | 7.19 ms / 3.21 ms | 7.14 ms / 3.14 ms | 35 ms [34.3 ms, 37.3 ms] / 26 ms [25.1 ms, 26.4 ms] | 1.24 s / 420 ms [416 ms, 431 ms] | — / — |
| 48 KiB | 8.32 ms / 4.58 ms | 8.27 ms / 4.51 ms | 61.9 ms [61.6 ms, 66.5 ms] / 44.8 ms [44.3 ms, 46.6 ms] | 2.21 s / 740 ms | — / — |
| 64 KiB | 8.58 ms / 5.98 ms | 8.51 ms / 5.93 ms | 111 ms [109 ms, 117 ms] / 81.3 ms [78.4 ms, 82.2 ms] | 3.93 s / 1.33 s [1.31 s, 1.38 s] | — / — |
| 128 KiB | 19.6 ms / 13.7 ms | 19.6 ms / 13.7 ms | — / — | — / — | — / — |
| 144 KiB | 36.3 ms / 16.6 ms | 36 ms / 16.4 ms | — / — | — / — | — / — |
| 256 KiB | 44 ms [43.6 ms, 45 ms] / 31.1 ms | 44 ms [43.3 ms, 45.1 ms] / 30.8 ms | — / — | — / — | — / — |
| 512 KiB | 98.1 ms / 70.3 ms [69.1 ms, 72.2 ms] | 97.7 ms / 69.2 ms | — / — | — / — | — / — |
| 1 MiB | 220 ms / 155 ms | 220 ms / 155 ms | — / — | — / — | — / — |
| 2 MiB | 486 ms / 346 ms | 484 ms / 345 ms | — / — | — / — | — / — |
| 4 MiB | 1.07 s / 766 ms | 1.07 s / 765 ms | — / — | — / — | — / — |

</details>

## Measurement limits

Warm native measurements reuse 64-byte-aligned input and caller-output buffers.
The timer surrounds repeated public calls inside the cdylib, crossing Python/FFI
once per batch. Warm decode uses prevalidated UTF-8; Base58 digit validation stays
inside the public decoder. Loading, fixture construction and caller-buffer setup
are outside timing. Invalid inputs, zero-prefixed workloads and owned outputs
need their own measurements.

First-call costs require separate fresh processes. Time the single-call export
before any conversion initializes its tables, and include codec setup and internal
allocation. Record whether UTF-8 checking is inside that timer. Construct and
independently verify the expected bytes outside timing; poison output before each
call and check the returned length and bytes afterwards. Single-call CPU clocks
can have coarse resolution, so retain process/copy ranges. Warm medians do not predict first-call latency.

For small source changes, use a paired run with byte-identical controls and fresh
library copies. Retain observed variation and report incomplete loader/call-order
balancing. Code alignment, dynamic mapping, file-page placement and background
load can move small cells. A shifted identical control prevents attributing the
whole observed change to source. Keep repeats separate from the primary cohort;
do not discard observations or silently normalize them by a control ratio.

Inspect default/cache and fat/off LTO independently. Compare reached hot loops as
well as symbol lists with `scripts/check_simd_inlining.py --compare OLD NEW`.
Iterator forms can remove bounds checks while retaining constructor calls or
changing vectorization, spills and stack frames. Check copies, reloads, helper
calls and loop layout in the linked consumer. Assembly establishes emitted work;
latency claims still require measurement. The decoder's validation invariant and
its test coverage remain documented in [DESIGN.md](../DESIGN.md) and
[CORRECTNESS.md](../CORRECTNESS.md).

## First-call decode

Separate native measurements use **16 fresh processes × 2 file copies** for each
LTO/policy/input size. Input is `"2"` repeated 1,408, 2,816, 5,632 or 32,768 Base58
characters. An independent Python bigint oracle checks exact length and bytes;
output is poisoned before timing. Loading, oracle construction and caller buffers
are outside timing. Codec initialization, internal allocation and the single-call
export's UTF-8 check remain inside. Calls rotate over both copy positions.
GCP pins CPU 1 under the shared VM lock. The median is over copies of each copy's
process median; brackets give the full observed process/copy range. Short calls
have coarse CPU-clock resolution.

| Host / LTO / policy | Input characters | First decode [range] |
| --- | ---: | ---: |
| m5 / fat / default | 1,408 | 7.25 µs [4 µs, 59 µs] |
| m5 / fat / default | 2,816 | 12.8 µs [9 µs, 18 µs] |
| m5 / fat / default | 5,632 | 32 µs [28 µs, 37 µs] |
| m5 / fat / default | 32,768 | 331 µs [305 µs, 391 µs] |
| m5 / fat / cache | 1,408 | 58.5 µs [53 µs, 148 µs] |
| m5 / fat / cache | 2,816 | 202 µs [182 µs, 263 µs] |
| m5 / fat / cache | 5,632 | 795 µs [742 µs, 869 µs] |
| m5 / fat / cache | 32,768 | 520 µs [485 µs, 545 µs] |
| m5 / off / default | 1,408 | 10.5 µs [8 µs, 155 µs] |
| m5 / off / default | 2,816 | 16.8 µs [14 µs, 22 µs] |
| m5 / off / default | 5,632 | 36.2 µs [32 µs, 43 µs] |
| m5 / off / default | 32,768 | 349 µs [332 µs, 403 µs] |
| m5 / off / cache | 1,408 | 64.5 µs [60 µs, 372 µs] |
| m5 / off / cache | 2,816 | 216 µs [202 µs, 274 µs] |
| m5 / off / cache | 5,632 | 903 µs [855 µs, 1.05 ms] |
| m5 / off / cache | 32,768 | 566 µs [533 µs, 622 µs] |
| gcp / fat / default | 1,408 | 16.6 µs [12.2 µs, 26.8 µs] |
| gcp / fat / default | 2,816 | 24 µs [19.8 µs, 36.5 µs] |
| gcp / fat / default | 5,632 | 44.6 µs [37.3 µs, 69.9 µs] |
| gcp / fat / default | 32,768 | 305 µs [291 µs, 333 µs] |
| gcp / fat / cache | 1,408 | 82.2 µs [68.5 µs, 111 µs] |
| gcp / fat / cache | 2,816 | 300 µs [297 µs, 316 µs] |
| gcp / fat / cache | 5,632 | 44.6 µs [37.7 µs, 76.2 µs] |
| gcp / fat / cache | 32,768 | 311 µs [290 µs, 338 µs] |
| gcp / off / default | 1,408 | 23.1 µs [18.8 µs, 34.7 µs] |
| gcp / off / default | 2,816 | 31 µs [27.3 µs, 39.5 µs] |
| gcp / off / default | 5,632 | 47 µs [43.4 µs, 60.1 µs] |
| gcp / off / default | 32,768 | 352 µs [329 µs, 381 µs] |
| gcp / off / cache | 1,408 | 98 µs [80.7 µs, 114 µs] |
| gcp / off / cache | 2,816 | 337 µs [328 µs, 354 µs] |
| gcp / off / cache | 5,632 | 89 µs [84.2 µs, 96.7 µs] |
| gcp / off / cache | 32,768 | 392 µs [371 µs, 424 µs] |

## Linked size and compact

The same timing artifacts supply these section and stripped-file byte counts,
after subtracting each configuration's matching empty consumer. Both conversion
directions are reachable. Unstripped consumers are timed; size accounting uses
their stripped copies. Code, readonly, initialized data and BSS are different
categories; stripped-file size includes layout and metadata. Mach-O and ELF
categories are not directly interchangeable. Signed BSS can be negative after
scaffold subtraction; it does not measure retained heap. `.rlib` size does not
measure deployment size.

The encoding `CHAR_PAIRS` constant contains 3,364 two-byte entries (6,728 B),
included in readonly data when its path is reachable.

| Host / LTO / consumer | Code B | Readonly B | Initialized data B | BSS B | Stripped file B |
| --- | ---: | ---: | ---: | ---: | ---: |
| m5 / fat / Auto | 67,824 | 25,004 | 16 | 0 | 82,784 |
| m5 / fat / No allocation | 10,672 | 11,174 | 16 | 0 | 16,640 |
| m5 / fat / bs58 | 1,408 | 950 | 0 | 0 | 0 |
| m5 / fat / five8 | 12,032 | 2,022 | 0 | 0 | 16,528 |
| m5 / fat / Auto + cache | 76,360 | 27,831 | 2,192 | 0 | 99,328 |
| m5 / off / Auto | 80,792 | 50,074 | 8 | 0 | 132,432 |
| m5 / off / No allocation | 12,376 | 12,570 | 8 | 0 | 33,120 |
| m5 / off / bs58 | 1,600 | 1,006 | 0 | 0 | 0 |
| m5 / off / five8 | 11,904 | 2,058 | 0 | 0 | 16,528 |
| m5 / off / Auto + cache | 84,760 | 58,082 | 2,184 | 0 | 132,464 |
| gcp / fat / Auto | 308,109 | 38,636 | 6,248 | -752 | 353,016 |
| gcp / fat / No allocation | 21,808 | 14,808 | 520 | -320 | 37,192 |
| gcp / fat / bs58 | 2,080 | 924 | 144 | -3,168 | 3,168 |
| gcp / fat / five8 | 12,080 | 1,768 | 48 | -1,616 | 13,904 |
| gcp / fat / Auto + cache | 304,013 | 39,504 | 9,136 | -2,338 | 352,688 |
| gcp / off / Auto | 332,803 | 76,212 | 10,328 | -1,544 | 419,336 |
| gcp / off / No allocation | 23,728 | 21,872 | 824 | -1,368 | 46,424 |
| gcp / off / bs58 | 2,256 | 1,012 | 152 | 664 | 3,432 |
| gcp / off / five8 | 12,288 | 1,792 | 48 | -1,840 | 14,128 |
| gcp / off / Auto + cache | 324,723 | 72,328 | 11,464 | -814 | 408,520 |

Wasm CODE and DATA are section payloads above the matching empty module. Initial
linear memory is the module’s absolute declared minimum, including its scaffold;
it is not a warm high-water mark or a retained-heap measurement.

| Wasm ISA / consumer / fat LTO | CODE B | DATA B | Stripped file B | Initial linear memory B |
| --- | ---: | ---: | ---: | ---: |
| Scalar / Auto | 86,172 | 16,225 | 102,605 | 1,114,112 |
| Scalar / No allocation | 15,134 | 10,136 | 25,308 | 1,114,112 |
| Scalar / bs58 | 2,814 | 776 | 3,606 | 1,114,112 |
| Scalar / five8 | 10,242 | 1,664 | 11,921 | 1,114,112 |
| Scalar / Auto + cache | 92,047 | 17,097 | 109,365 | 1,114,112 |
| SIMD128 / Auto | 94,749 | 17,325 | 112,282 | 1,114,112 |
| SIMD128 / No allocation | 16,827 | 10,264 | 27,130 | 1,114,112 |
| SIMD128 / bs58 | 2,881 | 776 | 3,673 | 1,114,112 |
| SIMD128 / five8 | 12,174 | 512 | 12,701 | 1,114,112 |
| SIMD128 / Auto + cache | 101,621 | 18,257 | 120,093 | 1,114,112 |

`compact` selects eight instead of twelve x86 IFMA widths, trading linked code
for more chunks on some multiplication shapes. These measurements use the default
twelve-width policy.

## Wasm SIMD128

Separate fat-LTO builds enable `-C target-feature=+simd128` for the library and
all comparator probes. These require a SIMD-capable engine. The source, M5 host,
compiler, Node/V8 runtime, fixtures, clock and five-process/three-batch sampling
match the scalar protocol; measurements run in a separate cohort.

Each cell is an absolute **encode / decode** median. These rows are independent
of the scalar README figure; linked sizes are listed by ISA above. Brackets show
observed process ranges when the spread exceeds 3%.

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 32 B | 24.7 ns / 46.7 ns [45.9 ns, 47.5 ns] | 25 ns / 44.1 ns | 25.3 ns [25.2 ns, 26 ns] / 47.4 ns [46.1 ns, 47.7 ns] | 597 ns [585 ns, 622 ns] / 208 ns [205 ns, 211 ns] | 44.3 ns [43.3 ns, 44.9 ns] / 39.8 ns |
| 64 B | 60.6 ns [59.1 ns, 61.6 ns] / 127 ns | 62 ns / 121 ns [119 ns, 123 ns] | 59.3 ns / 128 ns [126 ns, 131 ns] | 2.8 µs [2.71 µs, 2.9 µs] / 787 ns | 91.8 ns [89.9 ns, 93.5 ns] / 104 ns |
| 512 B | 7.34 µs [7.14 µs, 8 µs] / 6.02 µs [5.54 µs, 6.08 µs] | 1.12 µs [1.11 µs, 1.23 µs] / 968 ns [950 ns, 1.06 µs] | 7.77 µs [7.29 µs, 7.81 µs] / 6.03 µs [5.53 µs, 6.31 µs] | 237 µs [231 µs, 261 µs] / 74.3 µs [72.5 µs, 82.1 µs] | — / — |
| 4 KiB | 107 µs / 118 µs [115 µs, 120 µs] | 52 µs [51.1 µs, 52.7 µs] / 40.1 µs [39.7 µs, 41 µs] | 433 µs [423 µs, 462 µs] / 348 µs [324 µs, 366 µs] | 15.3 ms [15.1 ms, 16.1 ms] / 5.39 ms [5.12 ms, 5.59 ms] | — / — |
| 64 KiB | 3.03 ms [2.96 ms, 3.06 ms] / 2.65 ms [2.59 ms, 2.71 ms] | 2.85 ms / 2.63 ms [2.56 ms, 2.66 ms] | 111 ms [108 ms, 119 ms] / 78.3 ms [77.3 ms, 82.4 ms] | 3.93 s [3.88 s, 4.14 s] / 1.32 s [1.31 s, 1.4 s] | — / — |
| 1 MiB | 72.8 ms / 61 ms [60.1 ms, 62 ms] | 70.5 ms / 60.5 ms | — / — | — / — | — / — |
| 4 MiB | 350 ms / 287 ms | 328 ms [324 ms, 346 ms] / 285 ms | — / — | — / — | — / — |

## Cache and memory

`cache` changes dispatch and leaf policy as well as retention. First calls can
build tables. Code-derived capacities and ownership are in
[DESIGN.md](../DESIGN.md#heap-stack-and-cache-costs). Per-call scratch, owned tables,
allocator overhead and concurrent calls add to retention. Retained capacity is
not an active-heap bound. Wasm linear-memory pages need not return to the host.

Count allocations in separate, untimed builds with
`scripts/resource_report.py build --count-allocations`, then `run`. These internal
workers differ from ordinary public consumers. Requests count allocations plus
reallocations; requested-layout peaks are not RSS, allocator usable sizes or stack
high water. No counted-allocation results accompany this timing cohort.

## Measure and update

Use an idle host, AC power on laptops, and ignored `target/measurements/` output.
Build all artifacts before timing. On the shared VM, use `/mnt/bench`, wait for its
shared lock and pin the timing worker. Do not overlap our timing with builds/tests.
Record source, target, compiler, CPU flags, features, effective LTO, clock, fixture,
buffer ownership/alignment, warm/cold scope and sampling with every published run.

For example, build an ordinary public consumer, selecting the host's target:

```sh
CARGO_PROFILE_RELEASE_LTO=fat python3 -B scripts/code_size.py --source . --output target/measurements/native-default --target aarch64-apple-darwin --cpu generic --backends empty,auto,no-alloc,bs58,five8 --directions both --kinds cdylib
CARGO_PROFILE_RELEASE_LTO=fat python3 -B scripts/code_size.py --source . --output target/measurements/native-cache --target aarch64-apple-darwin --cpu generic --backends empty,auto --directions both --kinds cdylib --cache
```

On Linux use `x86_64-unknown-linux-gnu` and `--strip-tool strip`. Time with
`scripts/public_bench.py`, passing `LABEL=PATH`, `--platform`, `--source-commit`
and a new `--output` CSV. `auto` and `auto+cache` identify the two policies; on
Linux add `--cpu N`. Choose copies/processes/batches explicitly and retain their
ranges. Comparator results need matched fixtures, output ownership, build settings
and sampling; missing competitors must not acquire legend entries or estimates.

The generic renderer consumes public timing CSVs and matching size CSVs, and writes
only to a new directory. Normalized CSVs can carry `lto` and `wasm_isa` fields;
the renderer requires consistent values within each platform and includes them
in figure captions. Install pinned `assets/graphs/requirements.txt`, then use:

```sh
python3 -B assets/graphs/render_current.py --data TIMINGS.csv --sizes m5=DEFAULT_SIZES.csv m5+cache=CACHE_SIZES.csv --output NEW_FIGURES
python3 -B assets/graphs/render.py --check
python3 -B -m unittest discover -s assets/graphs -p test_render.py
```

Generated CSV/JSON, logs, artifacts and one-off analysis stay outside commits.
Selected tables, SVGs and reusable rendering tools belong in the repository.
The link/SVG check does not verify numeric values or reproduce pixels. Check
published numbers against raw inputs and inspect rendered titles, axes, legends
and captions. README figures and this guide present standalone results; branch
comparisons and merge decisions belong in PR discussions.

## Other measurement scopes

Scalar and SIMD128 Wasm are measured separately. Internal backends, dispatch thresholds,
allocation counting, Base58Check and compact configurations require separate,
source-bound cohorts. Those scopes are outside these timing cohorts.
Wasm timing uses Node/V8 elapsed time, not native process CPU time. Build with
`--target wasm32-unknown-unknown`, optionally `--wasm-simd128`, and use
`scripts/public_bench.py --wasm`. Allocation-counting modules are excluded from
ordinary timing. Criterion uses wall time; some comparator adapters own their
results while Auto reuses caller buffers, so report that distinction.

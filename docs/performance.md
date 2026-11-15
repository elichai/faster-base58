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

Each conversion cell labels **Encode** and **Decode**. A range is shown when the
observed process/copy maximum exceeds the minimum by more than 3%; it is not a confidence interval.
Logarithmic figures omit zero bytes.

In fat-LTO tables, Auto cells show **peak internal heap** beside each direction's
timing. These are separate, untimed public-consumer measurements at
the same decoded byte counts, with input and output storage excluded. Warm peaks
include cached tables retained by the first call. The [heap protocol](#cache-and-memory)
defines their fixtures and cache state. LTO-off tables contain timings only.

<details>
<summary>Apple M5 Pro / fat LTO</summary>

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 B | Encode: 1.43 ns [1.33 ns, 1.83 ns]; heap 0 B<br>Decode: 1.45 ns [1.41 ns, 1.76 ns]; heap 0 B | Encode: 1.45 ns [1.32 ns, 1.8 ns]; heap 0 B<br>Decode: 1.46 ns [1.4 ns, 1.71 ns]; heap 0 B | Encode: 1.88 ns [1.76 ns, 1.95 ns]<br>Decode: 1.06 ns [0.937 ns, 1.29 ns] | Encode: 1.03 ns [0.979 ns, 1.17 ns]<br>Decode: 0.807 ns [0.669 ns, 1.07 ns] | Encode: —<br>Decode: — |
| 1 B | Encode: 1.74 ns [1.64 ns, 2.17 ns]; heap 0 B<br>Decode: 1.71 ns [1.65 ns, 1.95 ns]; heap 0 B | Encode: 1.76 ns [1.67 ns, 2.18 ns]; heap 0 B<br>Decode: 1.72 ns [1.65 ns, 1.97 ns]; heap 0 B | Encode: 1.91 ns [1.82 ns, 2.25 ns]<br>Decode: 1.19 ns [1.13 ns, 1.52 ns] | Encode: 2.22 ns [1.98 ns, 2.72 ns]<br>Decode: 2.07 ns [1.85 ns, 2.21 ns] | Encode: —<br>Decode: — |
| 8 B | Encode: 9.86 ns [9.46 ns, 15 ns]; heap 0 B<br>Decode: 6.36 ns [6.18 ns, 8.66 ns]; heap 0 B | Encode: 9.79 ns [9.39 ns, 14.8 ns]; heap 0 B<br>Decode: 6.39 ns [6.16 ns, 9.66 ns]; heap 0 B | Encode: 8.92 ns [8.67 ns, 15.4 ns]<br>Decode: 6.65 ns [6.36 ns, 9.96 ns] | Encode: 59 ns [56.4 ns, 74.4 ns]<br>Decode: 24.8 ns [23.5 ns, 34.6 ns] | Encode: —<br>Decode: — |
| 16 B | Encode: 13.8 ns [13.2 ns, 21.1 ns]; heap 0 B<br>Decode: 7.81 ns [7.51 ns, 11.6 ns]; heap 0 B | Encode: 13.9 ns [13.4 ns, 21.2 ns]; heap 0 B<br>Decode: 7.84 ns [7.61 ns, 11.1 ns]; heap 0 B | Encode: 13.1 ns [12.6 ns, 20.2 ns]<br>Decode: 7.76 ns [7.58 ns, 11.9 ns] | Encode: 215 ns [201 ns, 243 ns]<br>Decode: 89 ns [83.3 ns, 115 ns] | Encode: —<br>Decode: — |
| 21 B | Encode: 15.6 ns [14.7 ns, 22.4 ns]; heap 0 B<br>Decode: 10.1 ns [9.88 ns, 20.6 ns]; heap 0 B | Encode: 15.6 ns [15 ns, 22.9 ns]; heap 0 B<br>Decode: 10.1 ns [9.92 ns, 19.4 ns]; heap 0 B | Encode: 14.7 ns [13.9 ns, 23.1 ns]<br>Decode: 10.2 ns [9.91 ns, 19.9 ns] | Encode: 356 ns [343 ns, 362 ns]<br>Decode: 135 ns [126 ns, 172 ns] | Encode: —<br>Decode: — |
| 22 B | Encode: 17.3 ns [15.9 ns, 24.2 ns]; heap 0 B<br>Decode: 10.9 ns [10.6 ns, 16.6 ns]; heap 0 B | Encode: 16.6 ns [16.1 ns, 24.2 ns]; heap 0 B<br>Decode: 11.2 ns [10.7 ns, 16.7 ns]; heap 0 B | Encode: 16.4 ns [15.5 ns, 24.5 ns]<br>Decode: 11 ns [10.6 ns, 16.4 ns] | Encode: 427 ns [406 ns, 456 ns]<br>Decode: 159 ns [149 ns, 198 ns] | Encode: —<br>Decode: — |
| 23 B | Encode: 18.9 ns [18.1 ns, 29.6 ns]; heap 0 B<br>Decode: 11.7 ns [11.3 ns, 16.8 ns]; heap 0 B | Encode: 18.6 ns [17.9 ns, 28.6 ns]; heap 0 B<br>Decode: 11.5 ns [11.1 ns, 16.3 ns]; heap 0 B | Encode: 18.2 ns [17.5 ns, 29.7 ns]<br>Decode: 11.8 ns [11.3 ns, 15.4 ns] | Encode: 442 ns [424 ns, 456 ns]<br>Decode: 171 ns [162 ns, 217 ns] | Encode: —<br>Decode: — |
| 31 B | Encode: 24.4 ns [22.5 ns, 36.7 ns]; heap 0 B<br>Decode: 17.5 ns [16.7 ns, 24.3 ns]; heap 0 B | Encode: 24.2 ns [22.7 ns, 36.7 ns]; heap 0 B<br>Decode: 17.6 ns [16.8 ns, 23.6 ns]; heap 0 B | Encode: 23.6 ns [21.8 ns, 37.1 ns]<br>Decode: 17.4 ns [16.6 ns, 25 ns] | Encode: 810 ns [732 ns, 843 ns]<br>Decode: 335 ns [310 ns, 358 ns] | Encode: —<br>Decode: — |
| 32 B | Encode: 11.9 ns [11.3 ns, 15.4 ns]; heap 0 B<br>Decode: 13.8 ns [13.3 ns, 17.8 ns]; heap 0 B | Encode: 12.1 ns [11.6 ns, 16.2 ns]; heap 0 B<br>Decode: 13.8 ns [13.4 ns, 23.9 ns]; heap 0 B | Encode: 9.93 ns [9.58 ns, 13.9 ns]<br>Decode: 13.9 ns [13.4 ns, 26.1 ns] | Encode: 836 ns [780 ns, 873 ns]<br>Decode: 342 ns [323 ns, 352 ns] | Encode: 34.1 ns [30.1 ns, 48 ns]<br>Decode: 32.7 ns [30.5 ns, 53.5 ns] |
| 33 B | Encode: 23.9 ns [23.1 ns, 28.3 ns]; heap 0 B<br>Decode: 14.2 ns [13.9 ns, 22.2 ns]; heap 0 B | Encode: 23.9 ns [22.9 ns, 28.4 ns]; heap 0 B<br>Decode: 14.2 ns [13.9 ns, 22.7 ns]; heap 0 B | Encode: 23.5 ns [22.3 ns, 28.6 ns]<br>Decode: 14.2 ns [13.9 ns, 23.6 ns] | Encode: 869 ns [818 ns, 920 ns]<br>Decode: 370 ns [347 ns, 386 ns] | Encode: —<br>Decode: — |
| 52 B | Encode: 45.6 ns [42.9 ns, 59.3 ns]; heap 0 B<br>Decode: 23.1 ns [22.3 ns, 32.6 ns]; heap 0 B | Encode: 48.5 ns [45.6 ns, 64.8 ns]; heap 47 KiB<br>Decode: 23.4 ns [22.7 ns, 33.4 ns]; heap 0 B | Encode: 46 ns [43.3 ns, 61.3 ns]<br>Decode: 23.2 ns [22.5 ns, 32 ns] | Encode: 2.07 µs [1.93 µs, 2.47 µs]<br>Decode: 1 µs [910 ns, 1.11 µs] | Encode: —<br>Decode: — |
| 63 B | Encode: 59.3 ns [54.3 ns, 79.5 ns]; heap 0 B<br>Decode: 29.7 ns [29.2 ns, 44.7 ns]; heap 0 B | Encode: 55.8 ns [51.9 ns, 73.8 ns]; heap 47 KiB<br>Decode: 29.9 ns [29.6 ns, 45.3 ns]; heap 0 B | Encode: 60.3 ns [55.7 ns, 75.1 ns]<br>Decode: 29.6 ns [29 ns, 44.3 ns] | Encode: 3.1 µs [2.9 µs, 3.77 µs]<br>Decode: 1.35 µs [1.28 µs, 1.45 µs] | Encode: —<br>Decode: — |
| 64 B | Encode: 24.4 ns [23 ns, 31.6 ns]; heap 0 B<br>Decode: 26.3 ns [25.4 ns, 36.6 ns]; heap 0 B | Encode: 24.3 ns [23 ns, 33.5 ns]; heap 0 B<br>Decode: 26.4 ns [25.4 ns, 36.7 ns]; heap 0 B | Encode: 24.2 ns [23 ns, 31.9 ns]<br>Decode: 25.9 ns [25.1 ns, 36.3 ns] | Encode: 3.17 µs [2.92 µs, 3.82 µs]<br>Decode: 1.38 µs [1.25 µs, 1.48 µs] | Encode: 80.7 ns [72.4 ns, 99.8 ns]<br>Decode: 96.8 ns [91.4 ns, 139 ns] |
| 65 B | Encode: 59.2 ns [55 ns, 74.1 ns]; heap 0 B<br>Decode: 27 ns [26.3 ns, 43.1 ns]; heap 0 B | Encode: 68.3 ns [63.6 ns, 76.5 ns]; heap 47 KiB<br>Decode: 26.7 ns [26 ns, 42.2 ns]; heap 0 B | Encode: 59.9 ns [55.2 ns, 79.1 ns]<br>Decode: 26.7 ns [26.1 ns, 41.1 ns] | Encode: 3.35 µs [3.11 µs, 3.82 µs]<br>Decode: 1.31 µs [1.25 µs, 1.5 µs] | Encode: —<br>Decode: — |
| 128 B | Encode: 150 ns [140 ns, 177 ns]; heap 0 B<br>Decode: 64.1 ns [61.9 ns, 91 ns]; heap 0 B | Encode: 114 ns [107 ns, 123 ns]; heap 47 KiB<br>Decode: 64.8 ns [62.9 ns, 88 ns]; heap 0 B | Encode: 155 ns [147 ns, 187 ns]<br>Decode: 64.5 ns [62.7 ns, 92.4 ns] | Encode: 14.4 µs [13.4 µs, 15.2 µs]<br>Decode: 4.94 µs [4.65 µs, 5.73 µs] | Encode: —<br>Decode: — |
| 256 B | Encode: 493 ns [479 ns, 676 ns]; heap 0 B<br>Decode: 166 ns [160 ns, 230 ns]; heap 0 B | Encode: 226 ns [222 ns, 305 ns]; heap 47 KiB<br>Decode: 168 ns [161 ns, 221 ns]; heap 0 B | Encode: 514 ns [498 ns, 770 ns]<br>Decode: 165 ns [160 ns, 219 ns] | Encode: 60.7 µs [56.8 µs, 66.2 µs]<br>Decode: 20.6 µs [19.6 µs, 22 µs] | Encode: —<br>Decode: — |
| 512 B | Encode: 1.84 µs [1.77 µs, 2.5 µs]; heap 0 B<br>Decode: 529 ns [502 ns, 624 ns]; heap 0 B | Encode: 587 ns [568 ns, 791 ns]; heap 47 KiB<br>Decode: 518 ns [498 ns, 579 ns]; heap 40.2 KiB | Encode: 1.95 µs [1.87 µs, 2.65 µs]<br>Decode: 529 ns [507 ns, 617 ns] | Encode: 248 µs [234 µs, 287 µs]<br>Decode: 82.5 µs [76.5 µs, 85.9 µs] | Encode: —<br>Decode: — |
| 1 KiB | Encode: 7.03 µs [6.54 µs, 11.1 µs]; heap 0 B<br>Decode: 1.81 µs [1.72 µs, 2.42 µs]; heap 0 B | Encode: 1.97 µs [1.9 µs, 2.74 µs]; heap 182 KiB<br>Decode: 1.68 µs [1.65 µs, 2.18 µs]; heap 154 KiB | Encode: 7.54 µs [7.08 µs, 9.68 µs]<br>Decode: 1.81 µs [1.75 µs, 2.44 µs] | Encode: 1.01 ms [938 µs, 1.04 ms]<br>Decode: 333 µs [319 µs, 350 µs] | Encode: —<br>Decode: — |
| 1,504 B | Encode: 14.6 µs [14 µs, 21.1 µs]; heap 0 B<br>Decode: 3.74 µs [3.6 µs, 5.17 µs]; heap 0 B | Encode: 4.03 µs [3.93 µs, 5.57 µs]; heap 712 KiB<br>Decode: 3.46 µs [3.4 µs, 4.68 µs]; heap 602 KiB | Encode: 15.7 µs [15.2 µs, 23 µs]<br>Decode: 3.66 µs [3.56 µs, 5.16 µs] | Encode: 2.18 ms [2.04 ms, 2.25 ms]<br>Decode: 725 µs [690 µs, 783 µs] | Encode: —<br>Decode: — |
| 1,875 B | Encode: 22.5 µs [21.7 µs, 36.4 µs]; heap 0 B<br>Decode: 5.77 µs [5.33 µs, 7.4 µs]; heap 0 B | Encode: 6.01 µs [5.93 µs, 8.46 µs]; heap 712 KiB<br>Decode: 5.58 µs [5.03 µs, 6.85 µs]; heap 602 KiB | Encode: 24.3 µs [23.7 µs, 38.5 µs]<br>Decode: 5.94 µs [5.41 µs, 7.82 µs] | Encode: 3.4 ms [3.17 ms, 3.62 ms]<br>Decode: 1.16 ms [1.05 ms, 1.18 ms] | Encode: —<br>Decode: — |
| 2 KiB | Encode: 28.3 µs [26.2 µs, 41.6 µs]; heap 0 B<br>Decode: 6.96 µs [6.66 µs, 9.57 µs]; heap 0 B | Encode: 7.42 µs [6.85 µs, 9.83 µs]; heap 712 KiB<br>Decode: 6.2 µs [5.91 µs, 8.18 µs]; heap 602 KiB | Encode: 29.4 µs [28.6 µs, 35 µs]<br>Decode: 6.68 µs [6.43 µs, 9.48 µs] | Encode: 4.13 ms [3.81 ms, 4.64 ms]<br>Decode: 1.34 ms [1.25 ms, 1.41 ms] | Encode: —<br>Decode: — |
| 4 KiB | Encode: 73.6 µs [71.1 µs, 97.5 µs]; heap 82.5 KiB<br>Decode: 26.2 µs [25 µs, 33.3 µs]; heap 0 B | Encode: 29.5 µs [28.8 µs, 36.9 µs]; heap 2.76 MiB<br>Decode: 26.2 µs [24.4 µs, 30.3 µs]; heap 2.32 MiB | Encode: 108 µs [104 µs, 174 µs]<br>Decode: 26.9 µs [25.7 µs, 32.8 µs] | Encode: 16.1 ms [15.2 ms, 16.9 ms]<br>Decode: 5.44 ms [5.06 ms, 5.54 ms] | Encode: —<br>Decode: — |
| 8 KiB | Encode: 170 µs [166 µs, 223 µs]; heap 164 KiB<br>Decode: 76.7 µs [73 µs, 95.7 µs]; heap 118 KiB | Encode: 135 µs [120 µs, 176 µs]; heap 11 MiB<br>Decode: 71.3 µs [69.2 µs, 90.1 µs]; heap 280 KiB | Encode: 432 µs [400 µs, 458 µs]<br>Decode: 100 µs [95.2 µs, 127 µs] | Encode: 65.2 ms [61.1 ms, 67 ms]<br>Decode: 21.8 ms [20.4 ms, 22.2 ms] | Encode: —<br>Decode: — |
| 16 KiB | Encode: 379 µs [366 µs, 498 µs]; heap 328 KiB<br>Decode: 181 µs [175 µs, 236 µs]; heap 242 KiB | Encode: 374 µs [364 µs, 487 µs]; heap 392 KiB<br>Decode: 173 µs [170 µs, 234 µs]; heap 410 KiB | Encode: 1.64 ms [1.57 ms, 2.45 ms]<br>Decode: 391 µs [377 µs, 491 µs] | Encode: 260 ms [245 ms, 268 ms]<br>Decode: 86.5 ms [81.3 ms, 90.3 ms] | Encode: —<br>Decode: — |
| 24 KiB | Encode: 756 µs [729 µs, 905 µs]; heap 612 KiB<br>Decode: 314 µs [300 µs, 380 µs]; heap 456 KiB | Encode: 720 µs [679 µs, 899 µs]; heap 3.35 MiB<br>Decode: 308 µs [292 µs, 388 µs]; heap 1.03 MiB | Encode: 3.74 ms [3.54 ms, 4.81 ms]<br>Decode: 874 µs [835 µs, 1.08 ms] | Encode: 588 ms [551 ms, 603 ms]<br>Decode: 195 ms [183 ms, 201 ms] | Encode: —<br>Decode: — |
| 28 KiB | Encode: 798 µs [788 µs, 1.03 ms]; heap 634 KiB<br>Decode: 378 µs [369 µs, 503 µs]; heap 474 KiB | Encode: 789 µs [772 µs, 1 ms]; heap 762 KiB<br>Decode: 364 µs [354 µs, 492 µs]; heap 1.05 MiB | Encode: 5.03 ms [4.86 ms, 6.19 ms]<br>Decode: 1.19 ms [1.15 ms, 1.64 ms] | Encode: 798 ms [766 ms, 820 ms]<br>Decode: 265 ms [253 ms, 274 ms] | Encode: —<br>Decode: — |
| 32 KiB | Encode: 916 µs [838 µs, 1.02 ms]; heap 656 KiB<br>Decode: 431 µs [423 µs, 588 µs]; heap 490 KiB | Encode: 833 µs [818 µs, 1.09 ms]; heap 784 KiB<br>Decode: 421 µs [408 µs, 548 µs]; heap 672 KiB | Encode: 6.52 ms [6.33 ms, 8.79 ms]<br>Decode: 1.6 ms [1.53 ms, 2.08 ms] | Encode: 1.04 s [987 ms, 1.1 s]<br>Decode: 356 ms [339 ms, 404 ms] | Encode: —<br>Decode: — |
| 36 KiB | Encode: 1.41 ms [1.37 ms, 1.94 ms]; heap 1.15 MiB<br>Decode: 537 µs [527 µs, 762 µs]; heap 838 KiB | Encode: 1.21 ms [1.16 ms, 1.57 ms]; heap 12 MiB<br>Decode: 516 µs [507 µs, 673 µs]; heap 1.56 MiB | Encode: 8.35 ms [8.14 ms, 12.2 ms]<br>Decode: 1.99 ms [1.9 ms, 2.58 ms] | Encode: 1.32 s [1.27 s, 1.41 s]<br>Decode: 438 ms [415 ms, 512 ms] | Encode: —<br>Decode: — |
| 48 KiB | Encode: 1.68 ms [1.64 ms, 1.83 ms]; heap 1.19 MiB<br>Decode: 734 µs [720 µs, 909 µs]; heap 923 KiB | Encode: 1.65 ms [1.61 ms, 1.83 ms]; heap 4 MiB<br>Decode: 699 µs [691 µs, 885 µs]; heap 1.49 MiB | Encode: 14.8 ms [14.3 ms, 15.2 ms]<br>Decode: 3.46 ms [3.39 ms, 4.56 ms] | Encode: 2.25 s [2.2 s, 2.36 s]<br>Decode: 755 ms [733 ms, 812 ms] | Encode: —<br>Decode: — |
| 64 KiB | Encode: 1.88 ms [1.84 ms, 2.5 ms]; heap 1.28 MiB<br>Decode: 994 µs [968 µs, 1.42 ms]; heap 985 KiB | Encode: 1.86 ms [1.84 ms, 2.41 ms]; heap 1.53 MiB<br>Decode: 969 µs [952 µs, 1.37 ms]; heap 1.17 MiB | Encode: 25.9 ms [25.7 ms, 41.1 ms]<br>Decode: 6.33 ms [5.99 ms, 8.61 ms] | Encode: 4 s [3.92 s, 4.38 s]<br>Decode: 1.36 s [1.3 s, 1.5 s] | Encode: —<br>Decode: — |
| 128 KiB | Encode: 4.13 ms [4.07 ms, 5.63 ms]; heap 2.56 MiB<br>Decode: 2.26 ms [2.23 ms, 3.17 ms]; heap 1.93 MiB | Encode: 4.11 ms [4.04 ms, 5.56 ms]; heap 3.06 MiB<br>Decode: 2.22 ms [2.19 ms, 3.05 ms]; heap 2.19 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 144 KiB | Encode: 6.95 ms [6.67 ms, 9.38 ms]; heap 4.65 MiB<br>Decode: 2.85 ms [2.75 ms, 4.17 ms]; heap 3.31 MiB | Encode: 6.16 ms [5.87 ms, 8.21 ms]; heap 15.5 MiB<br>Decode: 2.82 ms [2.68 ms, 4.09 ms]; heap 3.89 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 256 KiB | Encode: 9.36 ms [8.96 ms, 13.3 ms]; heap 4.62 MiB<br>Decode: 5.21 ms [5.06 ms, 7.43 ms]; heap 3.86 MiB | Encode: 9.17 ms [8.91 ms, 13.2 ms]; heap 5.37 MiB<br>Decode: 5.15 ms [5 ms, 7.29 ms]; heap 4.67 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 512 KiB | Encode: 21.5 ms [19.7 ms, 27.7 ms]; heap 8.73 MiB<br>Decode: 11.7 ms [11.4 ms, 16.8 ms]; heap 6.98 MiB | Encode: 21.1 ms [19.6 ms, 27.7 ms]; heap 9.98 MiB<br>Decode: 11.5 ms [11.3 ms, 16.6 ms]; heap 8.01 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 1 MiB | Encode: 44.4 ms [43.3 ms, 60 ms]; heap 17 MiB<br>Decode: 26.2 ms [25.6 ms, 36.1 ms]; heap 13.2 MiB | Encode: 44.8 ms [43.8 ms, 60.8 ms]; heap 17 MiB<br>Decode: 26 ms [25.4 ms, 34.2 ms]; heap 13.4 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 2 MiB | Encode: 96.7 ms [94.2 ms, 133 ms]; heap 33.4 MiB<br>Decode: 57.6 ms [57 ms, 80.5 ms]; heap 25.7 MiB | Encode: 96.6 ms [94.8 ms, 133 ms]; heap 33.4 MiB<br>Decode: 57.8 ms [57.3 ms, 80.7 ms]; heap 26.3 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 4 MiB | Encode: 208 ms [206 ms, 293 ms]; heap 66.4 MiB<br>Decode: 127 ms [125 ms, 176 ms]; heap 50.7 MiB | Encode: 210 ms [206 ms, 292 ms]; heap 66.4 MiB<br>Decode: 128 ms [126 ms, 178 ms]; heap 51.2 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |

</details>

<details>
<summary>Apple M5 Pro / off LTO</summary>

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 B | Encode: 1.48 ns [1.33 ns, 1.8 ns]<br>Decode: 1.85 ns | Encode: 1.53 ns [1.34 ns, 1.81 ns]<br>Decode: 1.87 ns | Encode: 1.83 ns<br>Decode: 1 ns [0.961 ns, 1.13 ns] | Encode: 1.22 ns [1.17 ns, 1.38 ns]<br>Decode: 1.83 ns | Encode: —<br>Decode: — |
| 1 B | Encode: 1.75 ns [1.59 ns, 1.92 ns]<br>Decode: 2.27 ns [2.25 ns, 2.33 ns] | Encode: 1.86 ns [1.64 ns, 1.94 ns]<br>Decode: 2.26 ns [2.21 ns, 2.28 ns] | Encode: 1.9 ns [1.86 ns, 1.93 ns]<br>Decode: 1.27 ns [1.24 ns, 1.39 ns] | Encode: 2.54 ns [2.4 ns, 2.57 ns]<br>Decode: 2.68 ns [2.32 ns, 2.8 ns] | Encode: —<br>Decode: — |
| 8 B | Encode: 23.4 ns [22.5 ns, 25.7 ns]<br>Decode: 16.1 ns [15.7 ns, 16.4 ns] | Encode: 23.5 ns [22.8 ns, 25.2 ns]<br>Decode: 16.2 ns [15.9 ns, 16.4 ns] | Encode: 24.4 ns [22.6 ns, 25.3 ns]<br>Decode: 15.2 ns [14.7 ns, 16.2 ns] | Encode: 78.7 ns [71.7 ns, 83.6 ns]<br>Decode: 26.1 ns [25.7 ns, 26.8 ns] | Encode: —<br>Decode: — |
| 16 B | Encode: 37.6 ns [34.5 ns, 38.8 ns]<br>Decode: 22.4 ns [21.8 ns, 22.7 ns] | Encode: 35.1 ns [34 ns, 38.6 ns]<br>Decode: 22.3 ns | Encode: 37 ns [35 ns, 38.9 ns]<br>Decode: 22.2 ns [21.8 ns, 22.7 ns] | Encode: 212 ns [208 ns, 215 ns]<br>Decode: 83.3 ns | Encode: —<br>Decode: — |
| 21 B | Encode: 38 ns [35.7 ns, 39.6 ns]<br>Decode: 26.5 ns [26.3 ns, 27.4 ns] | Encode: 36.7 ns [34.6 ns, 39.6 ns]<br>Decode: 26.5 ns | Encode: 38.9 ns [36.1 ns, 40.9 ns]<br>Decode: 26.1 ns [25.8 ns, 26.9 ns] | Encode: 339 ns<br>Decode: 128 ns [127 ns, 132 ns] | Encode: —<br>Decode: — |
| 22 B | Encode: 40.2 ns [36.8 ns, 42.2 ns]<br>Decode: 32.6 ns [32.1 ns, 33.4 ns] | Encode: 38.7 ns [36.7 ns, 43.7 ns]<br>Decode: 32.8 ns [32.3 ns, 33.5 ns] | Encode: 40.9 ns [37.9 ns, 43.2 ns]<br>Decode: 32.7 ns | Encode: 406 ns [404 ns, 432 ns]<br>Decode: 152 ns | Encode: —<br>Decode: — |
| 23 B | Encode: 44.5 ns [42 ns, 49.4 ns]<br>Decode: 35 ns [34.2 ns, 35.5 ns] | Encode: 45.1 ns [43.5 ns, 49.7 ns]<br>Decode: 34.9 ns [34.4 ns, 35.8 ns] | Encode: 46.9 ns [45.7 ns, 51.7 ns]<br>Decode: 33.3 ns [32.3 ns, 38.3 ns] | Encode: 440 ns [427 ns, 448 ns]<br>Decode: 165 ns [162 ns, 170 ns] | Encode: —<br>Decode: — |
| 31 B | Encode: 62.2 ns [58.7 ns, 65.6 ns]<br>Decode: 44.6 ns [43.6 ns, 45.6 ns] | Encode: 62.4 ns [58.8 ns, 65.7 ns]<br>Decode: 44.8 ns [43.9 ns, 46.2 ns] | Encode: 60.9 ns [56.8 ns, 64 ns]<br>Decode: 45.3 ns [43.7 ns, 49.9 ns] | Encode: 750 ns<br>Decode: 320 ns [309 ns, 335 ns] | Encode: —<br>Decode: — |
| 32 B | Encode: 23.6 ns [22.9 ns, 24.5 ns]<br>Decode: 39.3 ns [38.7 ns, 40.1 ns] | Encode: 23.3 ns [23.2 ns, 24.2 ns]<br>Decode: 39.6 ns [39.1 ns, 40.9 ns] | Encode: 22.9 ns [22.6 ns, 23.9 ns]<br>Decode: 38.7 ns [37.8 ns, 42.2 ns] | Encode: 839 ns [810 ns, 855 ns]<br>Decode: 331 ns [328 ns, 341 ns] | Encode: 33.2 ns [31.5 ns, 34.9 ns]<br>Decode: 32.5 ns |
| 33 B | Encode: 62.7 ns [59.6 ns, 66.8 ns]<br>Decode: 40.1 ns | Encode: 62.5 ns [59.6 ns, 67.3 ns]<br>Decode: 40.3 ns | Encode: 61.2 ns [58.3 ns, 65.4 ns]<br>Decode: 39.7 ns [39 ns, 42.6 ns] | Encode: 815 ns [806 ns, 837 ns]<br>Decode: 352 ns [349 ns, 364 ns] | Encode: —<br>Decode: — |
| 52 B | Encode: 102 ns [99.7 ns, 108 ns]<br>Decode: 67.1 ns [66.4 ns, 69.1 ns] | Encode: 48.2 ns [47.3 ns, 49.1 ns]<br>Decode: 66.8 ns | Encode: 104 ns [102 ns, 110 ns]<br>Decode: 66.8 ns [65.9 ns, 72 ns] | Encode: 2.04 µs [1.98 µs, 2.24 µs]<br>Decode: 874 ns [846 ns, 898 ns] | Encode: —<br>Decode: — |
| 63 B | Encode: 123 ns [121 ns, 135 ns]<br>Decode: 78 ns [76.2 ns, 79.3 ns] | Encode: 54.9 ns [51.4 ns, 56 ns]<br>Decode: 78.8 ns | Encode: 128 ns [121 ns, 134 ns]<br>Decode: 78 ns [76.6 ns, 87 ns] | Encode: 3 µs [2.96 µs, 3.07 µs]<br>Decode: 1.18 µs [1.14 µs, 1.24 µs] | Encode: —<br>Decode: — |
| 64 B | Encode: 25.1 ns<br>Decode: 72.8 ns [71.7 ns, 73.9 ns] | Encode: 24.9 ns<br>Decode: 73.2 ns | Encode: 24.9 ns<br>Decode: 73 ns | Encode: 3.14 µs [3.03 µs, 3.24 µs]<br>Decode: 1.18 µs [1.14 µs, 1.22 µs] | Encode: 74.5 ns [73.5 ns, 82.2 ns]<br>Decode: 97.1 ns |
| 65 B | Encode: 128 ns [119 ns, 134 ns]<br>Decode: 73 ns | Encode: 71.2 ns [70.4 ns, 74.8 ns]<br>Decode: 73.8 ns | Encode: 125 ns [122 ns, 136 ns]<br>Decode: 73.7 ns [73 ns, 85.9 ns] | Encode: 3.29 µs [3.19 µs, 3.42 µs]<br>Decode: 1.23 µs [1.21 µs, 1.26 µs] | Encode: —<br>Decode: — |
| 128 B | Encode: 287 ns [278 ns, 306 ns]<br>Decode: 158 ns | Encode: 122 ns [120 ns, 124 ns]<br>Decode: 161 ns | Encode: 300 ns [284 ns, 317 ns]<br>Decode: 157 ns | Encode: 13.6 µs [13.4 µs, 14 µs]<br>Decode: 4.73 µs [4.67 µs, 4.93 µs] | Encode: —<br>Decode: — |
| 256 B | Encode: 754 ns [719 ns, 782 ns]<br>Decode: 359 ns | Encode: 241 ns [238 ns, 249 ns]<br>Decode: 358 ns [348 ns, 366 ns] | Encode: 764 ns [745 ns, 807 ns]<br>Decode: 358 ns [355 ns, 400 ns] | Encode: 56.8 µs<br>Decode: 18.9 µs [18.8 µs, 19.4 µs] | Encode: —<br>Decode: — |
| 512 B | Encode: 2.32 µs [2.29 µs, 2.41 µs]<br>Decode: 910 ns [870 ns, 931 ns] | Encode: 580 ns<br>Decode: 522 ns [516 ns, 534 ns] | Encode: 2.4 µs [2.35 µs, 2.47 µs]<br>Decode: 906 ns [873 ns, 1.01 µs] | Encode: 232 µs [231 µs, 242 µs]<br>Decode: 78.4 µs [77.1 µs, 81.6 µs] | Encode: —<br>Decode: — |
| 1 KiB | Encode: 7.9 µs [7.68 µs, 8.12 µs]<br>Decode: 2.56 µs [2.48 µs, 2.58 µs] | Encode: 1.94 µs<br>Decode: 1.66 µs | Encode: 8.05 µs [7.97 µs, 8.48 µs]<br>Decode: 2.54 µs [2.46 µs, 2.63 µs] | Encode: 939 µs [935 µs, 1 ms]<br>Decode: 314 µs [312 µs, 322 µs] | Encode: —<br>Decode: — |
| 1,504 B | Encode: 15.9 µs [15.7 µs, 16.4 µs]<br>Decode: 4.79 µs [4.7 µs, 4.88 µs] | Encode: 4.02 µs<br>Decode: 3.44 µs | Encode: 16.4 µs [16.1 µs, 16.8 µs]<br>Decode: 4.78 µs [4.69 µs, 4.94 µs] | Encode: 2.03 ms<br>Decode: 685 µs [675 µs, 710 µs] | Encode: —<br>Decode: — |
| 1,875 B | Encode: 24.1 µs [23.6 µs, 24.6 µs]<br>Decode: 6.84 µs | Encode: 5.95 µs [5.85 µs, 6.05 µs]<br>Decode: 5.09 µs | Encode: 25 µs [24.7 µs, 25.5 µs]<br>Decode: 6.81 µs [6.72 µs, 7.08 µs] | Encode: 3.18 ms [3.14 ms, 3.36 ms]<br>Decode: 1.06 ms [1.05 ms, 1.11 ms] | Encode: —<br>Decode: — |
| 2 KiB | Encode: 28.2 µs [27.8 µs, 28.7 µs]<br>Decode: 8.02 µs | Encode: 6.88 µs [6.84 µs, 7.06 µs]<br>Decode: 5.93 µs | Encode: 29.3 µs<br>Decode: 8.03 µs [7.93 µs, 8.3 µs] | Encode: 3.77 ms<br>Decode: 1.27 ms [1.25 ms, 1.32 ms] | Encode: —<br>Decode: — |
| 4 KiB | Encode: 79.3 µs [77.6 µs, 80.4 µs]<br>Decode: 27.9 µs [27.6 µs, 28.6 µs] | Encode: 29 µs [28.8 µs, 30.2 µs]<br>Decode: 24.7 µs [24.3 µs, 25.4 µs] | Encode: 112 µs<br>Decode: 27.8 µs [27.6 µs, 28.6 µs] | Encode: 15.2 ms [15.1 ms, 15.6 ms]<br>Decode: 5.1 ms | Encode: —<br>Decode: — |
| 8 KiB | Encode: 178 µs [175 µs, 183 µs]<br>Decode: 80.4 µs [78.2 µs, 82.9 µs] | Encode: 119 µs [117 µs, 126 µs]<br>Decode: 72.6 µs | Encode: 432 µs<br>Decode: 102 µs | Encode: 60.8 ms<br>Decode: 20.4 ms [20.1 ms, 20.9 ms] | Encode: —<br>Decode: — |
| 16 KiB | Encode: 397 µs [389 µs, 403 µs]<br>Decode: 189 µs [188 µs, 194 µs] | Encode: 393 µs<br>Decode: 172 µs | Encode: 1.71 ms<br>Decode: 388 µs | Encode: 244 ms<br>Decode: 81.3 ms | Encode: —<br>Decode: — |
| 24 KiB | Encode: 776 µs [765 µs, 800 µs]<br>Decode: 322 µs [317 µs, 330 µs] | Encode: 702 µs [692 µs, 725 µs]<br>Decode: 297 µs | Encode: 3.83 ms [3.78 ms, 3.95 ms]<br>Decode: 862 µs | Encode: 552 ms [546 ms, 580 ms]<br>Decode: 184 ms [182 ms, 188 ms] | Encode: —<br>Decode: — |
| 28 KiB | Encode: 834 µs [815 µs, 854 µs]<br>Decode: 391 µs [387 µs, 419 µs] | Encode: 821 µs<br>Decode: 362 µs [359 µs, 382 µs] | Encode: 5.18 ms [5.13 ms, 5.35 ms]<br>Decode: 1.16 ms [1.16 ms, 1.23 ms] | Encode: 752 ms [748 ms, 771 ms]<br>Decode: 250 ms [248 ms, 275 ms] | Encode: —<br>Decode: — |
| 32 KiB | Encode: 879 µs [866 µs, 894 µs]<br>Decode: 442 µs | Encode: 875 µs<br>Decode: 410 µs [407 µs, 424 µs] | Encode: 6.75 ms<br>Decode: 1.51 ms [1.5 ms, 1.61 ms] | Encode: 981 ms<br>Decode: 327 ms [324 ms, 334 ms] | Encode: —<br>Decode: — |
| 36 KiB | Encode: 1.42 ms<br>Decode: 553 µs | Encode: 1.15 ms<br>Decode: 514 µs | Encode: 8.48 ms<br>Decode: 1.91 ms | Encode: 1.24 s<br>Decode: 413 ms | Encode: —<br>Decode: — |
| 48 KiB | Encode: 1.74 ms [1.71 ms, 1.76 ms]<br>Decode: 750 µs [740 µs, 807 µs] | Encode: 1.62 ms [1.6 ms, 1.66 ms]<br>Decode: 698 µs [690 µs, 737 µs] | Encode: 15.1 ms [14.9 ms, 15.7 ms]<br>Decode: 3.38 ms [3.33 ms, 3.71 ms] | Encode: 2.21 s [2.19 s, 2.27 s]<br>Decode: 733 ms [728 ms, 800 ms] | Encode: —<br>Decode: — |
| 64 KiB | Encode: 1.95 ms [1.91 ms, 2.05 ms]<br>Decode: 1.03 ms [1.01 ms, 1.05 ms] | Encode: 1.94 ms [1.91 ms, 2.1 ms]<br>Decode: 969 µs | Encode: 27.1 ms [26.7 ms, 27.6 ms]<br>Decode: 6.1 ms [5.94 ms, 6.19 ms] | Encode: 3.93 s [3.93 s, 4.11 s]<br>Decode: 1.31 s [1.3 s, 1.36 s] | Encode: —<br>Decode: — |
| 128 KiB | Encode: 4.3 ms [4.28 ms, 4.44 ms]<br>Decode: 2.35 ms | Encode: 4.31 ms [4.25 ms, 4.74 ms]<br>Decode: 2.21 ms | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 144 KiB | Encode: 6.98 ms [6.93 ms, 7.18 ms]<br>Decode: 2.88 ms | Encode: 5.94 ms<br>Decode: 2.73 ms | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 256 KiB | Encode: 9.41 ms<br>Decode: 5.29 ms [5.21 ms, 5.42 ms] | Encode: 9.36 ms<br>Decode: 5.03 ms [5.01 ms, 5.21 ms] | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 512 KiB | Encode: 20.6 ms [20.4 ms, 21.2 ms]<br>Decode: 11.9 ms | Encode: 20.5 ms [20.4 ms, 21.5 ms]<br>Decode: 11.4 ms | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 1 MiB | Encode: 45.1 ms<br>Decode: 26.4 ms | Encode: 45.6 ms<br>Decode: 25.6 ms | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 2 MiB | Encode: 98.7 ms [97.8 ms, 106 ms]<br>Decode: 58.6 ms [58.3 ms, 60.4 ms] | Encode: 99.2 ms [98.2 ms, 111 ms]<br>Decode: 57.8 ms [57.4 ms, 59.8 ms] | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 4 MiB | Encode: 213 ms<br>Decode: 130 ms | Encode: 214 ms<br>Decode: 128 ms | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |

</details>

<details>
<summary>AMD EPYC 9B45 (GCP, generic x86-64) / fat LTO</summary>

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 B | Encode: 2.56 ns [2.52 ns, 2.62 ns]; heap 0 B<br>Decode: 1.85 ns [1.82 ns, 1.88 ns]; heap 0 B | Encode: 2.64 ns [2.62 ns, 2.71 ns]; heap 0 B<br>Decode: 1.98 ns; heap 0 B | Encode: 1.73 ns [1.7 ns, 1.8 ns]<br>Decode: 1.72 ns [1.7 ns, 1.76 ns] | Encode: 1.59 ns [1.58 ns, 1.63 ns]<br>Decode: 1.86 ns [1.82 ns, 1.88 ns] | Encode: —<br>Decode: — |
| 1 B | Encode: 2.39 ns [2.31 ns, 2.51 ns]; heap 0 B<br>Decode: 2.2 ns; heap 0 B | Encode: 2.35 ns [2.32 ns, 2.43 ns]; heap 0 B<br>Decode: 2.21 ns; heap 0 B | Encode: 2 ns [1.96 ns, 2.07 ns]<br>Decode: 1.72 ns | Encode: 3.05 ns [2.95 ns, 3.17 ns]<br>Decode: 2.74 ns [2.68 ns, 2.89 ns] | Encode: —<br>Decode: — |
| 8 B | Encode: 19.4 ns; heap 0 B<br>Decode: 10.5 ns [10.3 ns, 10.6 ns]; heap 0 B | Encode: 19.3 ns; heap 0 B<br>Decode: 10.5 ns [10.2 ns, 10.6 ns]; heap 0 B | Encode: 19.2 ns<br>Decode: 10.5 ns [10.3 ns, 11.3 ns] | Encode: 60 ns<br>Decode: 37.3 ns [36.3 ns, 37.5 ns] | Encode: —<br>Decode: — |
| 16 B | Encode: 25.9 ns; heap 0 B<br>Decode: 14.4 ns [14.2 ns, 16 ns]; heap 0 B | Encode: 25.7 ns; heap 0 B<br>Decode: 14.3 ns [14 ns, 15.9 ns]; heap 0 B | Encode: 28 ns<br>Decode: 16.1 ns [14.5 ns, 16.6 ns] | Encode: 214 ns<br>Decode: 98.4 ns | Encode: —<br>Decode: — |
| 21 B | Encode: 28.3 ns; heap 0 B<br>Decode: 18.5 ns; heap 0 B | Encode: 28.2 ns; heap 0 B<br>Decode: 19.2 ns; heap 0 B | Encode: 30.9 ns<br>Decode: 18.9 ns [18.6 ns, 20.5 ns] | Encode: 351 ns<br>Decode: 147 ns | Encode: —<br>Decode: — |
| 22 B | Encode: 30.9 ns; heap 0 B<br>Decode: 19.1 ns; heap 0 B | Encode: 30.9 ns; heap 0 B<br>Decode: 19.2 ns; heap 0 B | Encode: 34.8 ns<br>Decode: 19.3 ns [19.2 ns, 21.8 ns] | Encode: 411 ns<br>Decode: 166 ns | Encode: —<br>Decode: — |
| 23 B | Encode: 37 ns; heap 0 B<br>Decode: 20.9 ns; heap 0 B | Encode: 36.9 ns; heap 0 B<br>Decode: 20.9 ns; heap 0 B | Encode: 40.5 ns<br>Decode: 20.9 ns [20.9 ns, 22.1 ns] | Encode: 434 ns<br>Decode: 175 ns | Encode: —<br>Decode: — |
| 31 B | Encode: 43.8 ns; heap 0 B<br>Decode: 25.9 ns; heap 0 B | Encode: 43.9 ns; heap 0 B<br>Decode: 25.7 ns; heap 0 B | Encode: 46.4 ns<br>Decode: 25.9 ns [25.6 ns, 27.3 ns] | Encode: 852 ns<br>Decode: 313 ns | Encode: —<br>Decode: — |
| 32 B | Encode: 30.3 ns; heap 0 B<br>Decode: 24.4 ns; heap 0 B | Encode: 30.2 ns; heap 0 B<br>Decode: 23.8 ns; heap 0 B | Encode: 30.2 ns<br>Decode: 23.9 ns [23.7 ns, 24.8 ns] | Encode: 911 ns<br>Decode: 327 ns | Encode: 55.8 ns<br>Decode: 49.4 ns [48.8 ns, 50.7 ns] |
| 33 B | Encode: 43.6 ns; heap 0 B<br>Decode: 25 ns; heap 0 B | Encode: 43.6 ns; heap 0 B<br>Decode: 24.9 ns; heap 0 B | Encode: 46.5 ns<br>Decode: 25.1 ns | Encode: 952 ns<br>Decode: 339 ns | Encode: —<br>Decode: — |
| 52 B | Encode: 83.5 ns; heap 0 B<br>Decode: 40.5 ns; heap 0 B | Encode: 77.3 ns; heap 47 KiB<br>Decode: 41 ns; heap 0 B | Encode: 93.1 ns<br>Decode: 41 ns [41 ns, 42.8 ns] | Encode: 2.54 µs<br>Decode: 832 ns | Encode: —<br>Decode: — |
| 63 B | Encode: 104 ns; heap 0 B<br>Decode: 48.5 ns; heap 0 B | Encode: 87.5 ns [87 ns, 90.2 ns]; heap 47 KiB<br>Decode: 48.7 ns; heap 0 B | Encode: 111 ns<br>Decode: 49.4 ns | Encode: 3.81 µs<br>Decode: 1.23 µs | Encode: —<br>Decode: — |
| 64 B | Encode: 31.7 ns [31.3 ns, 37.9 ns]; heap 0 B<br>Decode: 45.1 ns; heap 0 B | Encode: 31.8 ns [31.5 ns, 36.6 ns]; heap 0 B<br>Decode: 45.2 ns; heap 0 B | Encode: 31.7 ns<br>Decode: 45.5 ns | Encode: 3.9 µs<br>Decode: 1.25 µs | Encode: 116 ns [116 ns, 120 ns]<br>Decode: 129 ns |
| 65 B | Encode: 107 ns [106 ns, 111 ns]; heap 0 B<br>Decode: 46.6 ns; heap 0 B | Encode: 101 ns [100 ns, 107 ns]; heap 47 KiB<br>Decode: 47.6 ns; heap 0 B | Encode: 117 ns<br>Decode: 48.1 ns | Encode: 4.07 µs<br>Decode: 1.31 µs | Encode: —<br>Decode: — |
| 128 B | Encode: 312 ns; heap 0 B<br>Decode: 109 ns [107 ns, 110 ns]; heap 0 B | Encode: 166 ns; heap 47 KiB<br>Decode: 108 ns; heap 0 B | Encode: 328 ns<br>Decode: 108 ns | Encode: 16.8 µs<br>Decode: 5.27 µs | Encode: —<br>Decode: — |
| 256 B | Encode: 1.06 µs [1.05 µs, 1.09 µs]; heap 0 B<br>Decode: 279 ns [278 ns, 288 ns]; heap 0 B | Encode: 395 ns; heap 47 KiB<br>Decode: 277 ns [276 ns, 287 ns]; heap 0 B | Encode: 1.11 µs<br>Decode: 280 ns [276 ns, 289 ns] | Encode: 70 µs<br>Decode: 22.2 µs [22.1 µs, 22.9 µs] | Encode: —<br>Decode: — |
| 512 B | Encode: 3.24 µs [3.22 µs, 3.36 µs]; heap 5.2 KiB<br>Decode: 856 ns [851 ns, 894 ns]; heap 0 B | Encode: 1.09 µs [1.08 µs, 1.12 µs]; heap 47 KiB<br>Decode: 864 ns [856 ns, 899 ns]; heap 0 B | Encode: 4.08 µs [4.05 µs, 4.2 µs]<br>Decode: 866 ns [856 ns, 898 ns] | Encode: 283 µs<br>Decode: 93.3 µs [92.7 µs, 95.9 µs] | Encode: —<br>Decode: — |
| 1 KiB | Encode: 7.35 µs [7.21 µs, 7.52 µs]; heap 9.97 KiB<br>Decode: 2.56 µs [2.5 µs, 2.61 µs]; heap 5.59 KiB | Encode: 3.34 µs [3.33 µs, 3.46 µs]; heap 182 KiB<br>Decode: 2.39 µs [2.36 µs, 2.44 µs]; heap 154 KiB | Encode: 15.8 µs [15.7 µs, 16.3 µs]<br>Decode: 2.94 µs [2.91 µs, 3 µs] | Encode: 1.14 ms [1.13 ms, 1.17 ms]<br>Decode: 375 µs [371 µs, 384 µs] | Encode: —<br>Decode: — |
| 1,504 B | Encode: 12.3 µs; heap 14.1 KiB<br>Decode: 4.4 µs [4.36 µs, 4.52 µs]; heap 8.02 KiB | Encode: 6.84 µs; heap 712 KiB<br>Decode: 5.1 µs; heap 602 KiB | Encode: 33.6 µs<br>Decode: 5.96 µs | Encode: 2.45 ms<br>Decode: 799 µs | Encode: —<br>Decode: — |
| 1,875 B | Encode: 15.8 µs; heap 18.4 KiB<br>Decode: 5.51 µs; heap 10.2 KiB | Encode: 10.2 µs; heap 712 KiB<br>Decode: 7.71 µs; heap 602 KiB | Encode: 51.9 µs<br>Decode: 9 µs [8.95 µs, 9.36 µs] | Encode: 3.81 ms<br>Decode: 1.24 ms | Encode: —<br>Decode: — |
| 2 KiB | Encode: 17.9 µs [17.6 µs, 18.4 µs]; heap 19.5 KiB<br>Decode: 6.15 µs [6.06 µs, 6.42 µs]; heap 10.8 KiB | Encode: 11.8 µs [11.7 µs, 12.1 µs]; heap 712 KiB<br>Decode: 8.98 µs [8.9 µs, 9.37 µs]; heap 602 KiB | Encode: 61.5 µs [61.5 µs, 63.7 µs]<br>Decode: 10.7 µs [10.6 µs, 11.1 µs] | Encode: 4.53 ms [4.53 ms, 4.71 ms]<br>Decode: 1.47 ms [1.47 ms, 1.53 ms] | Encode: —<br>Decode: — |
| 4 KiB | Encode: 48.2 µs [47.6 µs, 49.8 µs]; heap 38.4 KiB<br>Decode: 16 µs [15.7 µs, 16.9 µs]; heap 21.3 KiB | Encode: 40 µs [39.7 µs, 41.9 µs]; heap 91.8 KiB<br>Decode: 15.7 µs [15.6 µs, 16.9 µs]; heap 30.3 KiB | Encode: 244 µs<br>Decode: 41.5 µs [41.4 µs, 43.4 µs] | Encode: 18.2 ms<br>Decode: 5.86 ms [5.83 ms, 6.1 ms] | Encode: —<br>Decode: — |
| 8 KiB | Encode: 142 µs [142 µs, 146 µs]; heap 91.4 KiB<br>Decode: 46 µs; heap 52.8 KiB | Encode: 119 µs [119 µs, 123 µs]; heap 135 KiB<br>Decode: 45.1 µs [44.7 µs, 47.3 µs]; heap 59.8 KiB | Encode: 964 µs<br>Decode: 162 µs [161 µs, 167 µs] | Encode: 72.6 ms<br>Decode: 23.4 ms | Encode: —<br>Decode: — |
| 16 KiB | Encode: 430 µs; heap 178 KiB<br>Decode: 136 µs; heap 105 KiB | Encode: 363 µs; heap 220 KiB<br>Decode: 134 µs [133 µs, 138 µs]; heap 119 KiB | Encode: 3.83 ms<br>Decode: 636 µs | Encode: 290 ms<br>Decode: 93.5 ms | Encode: —<br>Decode: — |
| 24 KiB | Encode: 793 µs [785 µs, 809 µs]; heap 285 KiB<br>Decode: 264 µs; heap 181 KiB | Encode: 700 µs [697 µs, 721 µs]; heap 752 KiB<br>Decode: 268 µs; heap 421 KiB | Encode: 8.63 ms<br>Decode: 1.41 ms | Encode: 654 ms<br>Decode: 210 ms | Encode: —<br>Decode: — |
| 28 KiB | Encode: 955 µs [943 µs, 977 µs]; heap 634 KiB<br>Decode: 353 µs; heap 199 KiB | Encode: 864 µs; heap 762 KiB<br>Decode: 343 µs; heap 439 KiB | Encode: 11.8 ms<br>Decode: 1.92 ms | Encode: 891 ms<br>Decode: 285 ms | Encode: —<br>Decode: — |
| 32 KiB | Encode: 1.01 ms; heap 656 KiB<br>Decode: 412 µs; heap 208 KiB | Encode: 923 µs; heap 784 KiB<br>Decode: 381 µs [378 µs, 393 µs]; heap 477 KiB | Encode: 15.3 ms<br>Decode: 2.5 ms | Encode: 1.16 s<br>Decode: 372 ms | Encode: —<br>Decode: — |
| 36 KiB | Encode: 1.44 ms [1.44 ms, 1.49 ms]; heap 429 KiB<br>Decode: 528 µs [522 µs, 544 µs]; heap 275 KiB | Encode: 1.39 ms; heap 1.2 MiB<br>Decode: 492 µs [490 µs, 506 µs]; heap 755 KiB | Encode: 19.1 ms<br>Decode: 3.17 ms | Encode: 1.47 s<br>Decode: 477 ms | Encode: —<br>Decode: — |
| 48 KiB | Encode: 1.98 ms; heap 1.19 MiB<br>Decode: 792 µs; heap 841 KiB | Encode: 2.07 ms; heap 1.25 MiB<br>Decode: 737 µs; heap 889 KiB | Encode: 34.4 ms<br>Decode: 5.6 ms | Encode: 2.61 s<br>Decode: 838 ms | Encode: —<br>Decode: — |
| 64 KiB | Encode: 2.23 ms; heap 1.28 MiB<br>Decode: 1.03 ms; heap 896 KiB | Encode: 2.11 ms; heap 1.53 MiB<br>Decode: 829 µs; heap 1000 KiB | Encode: 61.4 ms<br>Decode: 9.96 ms | Encode: 4.64 s<br>Decode: 1.49 s | Encode: —<br>Decode: — |
| 128 KiB | Encode: 4.84 ms; heap 2.56 MiB<br>Decode: 2.34 ms; heap 1.84 MiB | Encode: 4.43 ms; heap 3.06 MiB<br>Decode: 1.93 ms [1.92 ms, 1.98 ms]; heap 2 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 144 KiB | Encode: 7.48 ms; heap 4.3 MiB<br>Decode: 3.04 ms; heap 3.13 MiB | Encode: 6.83 ms; heap 4.65 MiB<br>Decode: 2.4 ms; heap 3.23 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 256 KiB | Encode: 10.3 ms; heap 4.62 MiB<br>Decode: 4.84 ms; heap 3.78 MiB | Encode: 9.59 ms; heap 5.37 MiB<br>Decode: 4.43 ms; heap 4.04 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 512 KiB | Encode: 22.4 ms [21.9 ms, 22.8 ms]; heap 8.73 MiB<br>Decode: 10.9 ms; heap 6.9 MiB | Encode: 21 ms [20.9 ms, 21.8 ms]; heap 9.98 MiB<br>Decode: 10.2 ms; heap 7.38 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 1 MiB | Encode: 48.5 ms [48.3 ms, 49.9 ms]; heap 17 MiB<br>Decode: 24.3 ms; heap 13.1 MiB | Encode: 48.8 ms; heap 17 MiB<br>Decode: 22.9 ms; heap 13.2 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 2 MiB | Encode: 107 ms [104 ms, 108 ms]; heap 33.4 MiB<br>Decode: 54.3 ms [53.4 ms, 55.6 ms]; heap 25.6 MiB | Encode: 106 ms [104 ms, 108 ms]; heap 33.4 MiB<br>Decode: 52.9 ms [51.8 ms, 53.9 ms]; heap 25.7 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 4 MiB | Encode: 230 ms [224 ms, 235 ms]; heap 66.4 MiB<br>Decode: 119 ms [118 ms, 122 ms]; heap 50.6 MiB | Encode: 230 ms [224 ms, 236 ms]; heap 66.4 MiB<br>Decode: 116 ms [116 ms, 119 ms]; heap 50.6 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |

</details>

<details>
<summary>AMD EPYC 9B45 (GCP, generic x86-64) / off LTO</summary>

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 B | Encode: 2.44 ns [2.43 ns, 2.67 ns]<br>Decode: 2.56 ns | Encode: 2.43 ns<br>Decode: 2.46 ns | Encode: 2.72 ns<br>Decode: 1.95 ns | Encode: 2.19 ns<br>Decode: 2.82 ns | Encode: —<br>Decode: — |
| 1 B | Encode: 2.92 ns<br>Decode: 2.8 ns | Encode: 2.92 ns<br>Decode: 2.79 ns | Encode: 2.57 ns [2.42 ns, 2.63 ns]<br>Decode: 1.7 ns | Encode: 3.7 ns<br>Decode: 3.87 ns [3.8 ns, 4.14 ns] | Encode: —<br>Decode: — |
| 8 B | Encode: 33.5 ns<br>Decode: 28.4 ns [28.2 ns, 29.9 ns] | Encode: 32.6 ns<br>Decode: 28.3 ns [28.1 ns, 30 ns] | Encode: 35.2 ns<br>Decode: 27.2 ns | Encode: 60.4 ns<br>Decode: 36.8 ns [36.7 ns, 38.1 ns] | Encode: —<br>Decode: — |
| 16 B | Encode: 53.5 ns<br>Decode: 42.5 ns [41 ns, 47.8 ns] | Encode: 48 ns<br>Decode: 41.8 ns [41 ns, 42.4 ns] | Encode: 54.4 ns<br>Decode: 39.9 ns | Encode: 213 ns<br>Decode: 99.3 ns [99 ns, 102 ns] | Encode: —<br>Decode: — |
| 21 B | Encode: 53 ns<br>Decode: 49.9 ns [49.4 ns, 54.2 ns] | Encode: 49.5 ns<br>Decode: 49.3 ns [49.1 ns, 51.6 ns] | Encode: 58 ns<br>Decode: 48.3 ns | Encode: 349 ns<br>Decode: 148 ns | Encode: —<br>Decode: — |
| 22 B | Encode: 57 ns<br>Decode: 61.2 ns [55.2 ns, 65.5 ns] | Encode: 51.4 ns<br>Decode: 57 ns [56.5 ns, 59.4 ns] | Encode: 61.1 ns<br>Decode: 59.9 ns [57 ns, 60.2 ns] | Encode: 408 ns<br>Decode: 168 ns | Encode: —<br>Decode: — |
| 23 B | Encode: 67.7 ns<br>Decode: 57.6 ns [57.2 ns, 63 ns] | Encode: 63.3 ns [63.1 ns, 66.2 ns]<br>Decode: 57.4 ns [56.9 ns, 59.3 ns] | Encode: 76.1 ns<br>Decode: 56 ns | Encode: 429 ns<br>Decode: 176 ns | Encode: —<br>Decode: — |
| 31 B | Encode: 89.2 ns<br>Decode: 72.2 ns [71.6 ns, 79.1 ns] | Encode: 82.5 ns<br>Decode: 72.1 ns | Encode: 94.5 ns<br>Decode: 70.5 ns | Encode: 842 ns<br>Decode: 310 ns | Encode: —<br>Decode: — |
| 32 B | Encode: 35.5 ns [34.7 ns, 36.6 ns]<br>Decode: 70.4 ns [69.8 ns, 76.2 ns] | Encode: 36.3 ns<br>Decode: 70.3 ns | Encode: 35.2 ns [33.9 ns, 35.7 ns]<br>Decode: 68.7 ns | Encode: 902 ns<br>Decode: 323 ns | Encode: 55.1 ns<br>Decode: 47.4 ns |
| 33 B | Encode: 87.8 ns<br>Decode: 71.6 ns [70.7 ns, 74.6 ns] | Encode: 82.7 ns<br>Decode: 71.2 ns | Encode: 93.3 ns<br>Decode: 69.4 ns | Encode: 944 ns<br>Decode: 337 ns | Encode: —<br>Decode: — |
| 52 B | Encode: 158 ns [157 ns, 163 ns]<br>Decode: 115 ns [113 ns, 122 ns] | Encode: 85.4 ns<br>Decode: 115 ns | Encode: 172 ns<br>Decode: 112 ns | Encode: 2.53 µs<br>Decode: 827 ns | Encode: —<br>Decode: — |
| 63 B | Encode: 190 ns<br>Decode: 134 ns [134 ns, 146 ns] | Encode: 95.4 ns<br>Decode: 135 ns | Encode: 203 ns<br>Decode: 133 ns [132 ns, 146 ns] | Encode: 3.79 µs<br>Decode: 1.22 µs [1.22 µs, 1.28 µs] | Encode: —<br>Decode: — |
| 64 B | Encode: 44.9 ns<br>Decode: 132 ns [131 ns, 143 ns] | Encode: 45 ns<br>Decode: 132 ns | Encode: 44.5 ns<br>Decode: 130 ns | Encode: 3.88 µs<br>Decode: 1.25 µs | Encode: 116 ns<br>Decode: 121 ns |
| 65 B | Encode: 193 ns<br>Decode: 135 ns [134 ns, 145 ns] | Encode: 112 ns<br>Decode: 134 ns [133 ns, 148 ns] | Encode: 205 ns<br>Decode: 131 ns | Encode: 4.04 µs<br>Decode: 1.3 µs | Encode: —<br>Decode: — |
| 128 B | Encode: 488 ns<br>Decode: 277 ns [275 ns, 284 ns] | Encode: 177 ns [176 ns, 187 ns]<br>Decode: 276 ns | Encode: 511 ns<br>Decode: 274 ns | Encode: 16.8 µs<br>Decode: 5.27 µs | Encode: —<br>Decode: — |
| 256 B | Encode: 1.4 µs<br>Decode: 616 ns [604 ns, 624 ns] | Encode: 409 ns [405 ns, 426 ns]<br>Decode: 608 ns | Encode: 1.48 µs<br>Decode: 616 ns [604 ns, 622 ns] | Encode: 70.4 µs<br>Decode: 22 µs | Encode: —<br>Decode: — |
| 512 B | Encode: 4.47 µs [4.43 µs, 4.56 µs]<br>Decode: 1.5 µs | Encode: 1.11 µs<br>Decode: 1.5 µs | Encode: 4.77 µs<br>Decode: 1.5 µs | Encode: 283 µs<br>Decode: 92.2 µs | Encode: —<br>Decode: — |
| 1 KiB | Encode: 9.65 µs<br>Decode: 4.03 µs | Encode: 3.42 µs<br>Decode: 2.43 µs | Encode: 17.1 µs<br>Decode: 4.11 µs | Encode: 1.14 ms<br>Decode: 372 µs | Encode: —<br>Decode: — |
| 1,504 B | Encode: 15.6 µs [15.4 µs, 16.1 µs]<br>Decode: 6.64 µs | Encode: 7 µs<br>Decode: 5.26 µs | Encode: 35.4 µs<br>Decode: 8.03 µs | Encode: 2.45 ms<br>Decode: 799 µs | Encode: —<br>Decode: — |
| 1,875 B | Encode: 20 µs<br>Decode: 8.36 µs | Encode: 10.5 µs<br>Decode: 7.93 µs | Encode: 54.2 µs<br>Decode: 11.7 µs | Encode: 3.8 ms<br>Decode: 1.24 ms | Encode: —<br>Decode: — |
| 2 KiB | Encode: 22.4 µs<br>Decode: 9.14 µs | Encode: 12.1 µs<br>Decode: 9.26 µs | Encode: 63.9 µs<br>Decode: 13.7 µs | Encode: 4.53 ms<br>Decode: 1.48 ms | Encode: —<br>Decode: — |
| 4 KiB | Encode: 57.1 µs<br>Decode: 21.9 µs | Encode: 42.3 µs<br>Decode: 22 µs | Encode: 246 µs<br>Decode: 47.5 µs | Encode: 18.1 ms<br>Decode: 5.88 ms | Encode: —<br>Decode: — |
| 8 KiB | Encode: 159 µs<br>Decode: 57.8 µs | Encode: 126 µs<br>Decode: 57.4 µs | Encode: 968 µs<br>Decode: 170 µs | Encode: 72.5 ms<br>Decode: 23.4 ms | Encode: —<br>Decode: — |
| 16 KiB | Encode: 462 µs<br>Decode: 161 µs | Encode: 378 µs [372 µs, 393 µs]<br>Decode: 160 µs | Encode: 3.83 ms<br>Decode: 629 µs | Encode: 290 ms<br>Decode: 93.4 ms | Encode: —<br>Decode: — |
| 24 KiB | Encode: 837 µs<br>Decode: 300 µs | Encode: 716 µs<br>Decode: 304 µs | Encode: 8.59 ms<br>Decode: 1.37 ms | Encode: 652 ms<br>Decode: 210 ms | Encode: —<br>Decode: — |
| 28 KiB | Encode: 989 µs<br>Decode: 396 µs | Encode: 900 µs<br>Decode: 386 µs | Encode: 11.7 ms<br>Decode: 1.86 ms [1.85 ms, 1.93 ms] | Encode: 888 ms<br>Decode: 286 ms | Encode: —<br>Decode: — |
| 32 KiB | Encode: 1.06 ms<br>Decode: 462 µs | Encode: 972 µs<br>Decode: 438 µs | Encode: 15.3 ms<br>Decode: 2.4 ms | Encode: 1.16 s<br>Decode: 373 ms | Encode: —<br>Decode: — |
| 36 KiB | Encode: 1.52 ms<br>Decode: 578 µs | Encode: 1.41 ms<br>Decode: 549 µs | Encode: 19.1 ms<br>Decode: 3.03 ms | Encode: 1.47 s<br>Decode: 473 ms | Encode: —<br>Decode: — |
| 48 KiB | Encode: 2.05 ms<br>Decode: 864 µs | Encode: 2.11 ms<br>Decode: 812 µs | Encode: 34.2 ms<br>Decode: 5.33 ms | Encode: 2.61 s<br>Decode: 840 ms | Encode: —<br>Decode: — |
| 64 KiB | Encode: 2.33 ms<br>Decode: 1.13 ms | Encode: 2.2 ms<br>Decode: 929 µs | Encode: 61 ms<br>Decode: 9.43 ms | Encode: 4.64 s<br>Decode: 1.49 s | Encode: —<br>Decode: — |
| 128 KiB | Encode: 5.09 ms<br>Decode: 2.55 ms | Encode: 4.65 ms<br>Decode: 2.15 ms | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 144 KiB | Encode: 7.98 ms<br>Decode: 3.32 ms | Encode: 7.14 ms [6.99 ms, 7.21 ms]<br>Decode: 2.65 ms | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 256 KiB | Encode: 10.8 ms [10.7 ms, 11 ms]<br>Decode: 5.29 ms | Encode: 10.1 ms [10.1 ms, 10.5 ms]<br>Decode: 4.89 ms [4.87 ms, 5.17 ms] | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 512 KiB | Encode: 23.6 ms [23 ms, 23.9 ms]<br>Decode: 11.8 ms | Encode: 21.9 ms [21.8 ms, 22.7 ms]<br>Decode: 11.1 ms | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 1 MiB | Encode: 51.3 ms<br>Decode: 26.3 ms [25.6 ms, 27.1 ms] | Encode: 51.2 ms<br>Decode: 24.8 ms [24.2 ms, 25 ms] | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 2 MiB | Encode: 108 ms [107 ms, 111 ms]<br>Decode: 58 ms | Encode: 107 ms [107 ms, 111 ms]<br>Decode: 56.5 ms | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 4 MiB | Encode: 237 ms<br>Decode: 126 ms [125 ms, 130 ms] | Encode: 237 ms<br>Decode: 122 ms [122 ms, 127 ms] | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |

</details>

<details>
<summary>Scalar WebAssembly on Apple M5 Pro (Node/V8) / fat LTO</summary>

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 B | Encode: 1.85 ns; heap 0 B<br>Decode: 2.09 ns [2.04 ns, 2.2 ns]; heap 0 B | Encode: 1.83 ns [1.78 ns, 1.87 ns]; heap 0 B<br>Decode: 2.08 ns [2.06 ns, 2.23 ns]; heap 0 B | Encode: 2.17 ns [2.16 ns, 2.29 ns]<br>Decode: 0.848 ns [0.803 ns, 0.897 ns] | Encode: 1.84 ns [1.79 ns, 1.98 ns]<br>Decode: 1.89 ns [1.85 ns, 1.96 ns] | Encode: —<br>Decode: — |
| 1 B | Encode: 2.18 ns [2.12 ns, 2.2 ns]; heap 0 B<br>Decode: 2.53 ns [2.4 ns, 2.58 ns]; heap 0 B | Encode: 2.13 ns [2.07 ns, 2.22 ns]; heap 0 B<br>Decode: 2.51 ns [2.39 ns, 2.67 ns]; heap 0 B | Encode: 2.73 ns [2.61 ns, 3.01 ns]<br>Decode: 1.16 ns [1.14 ns, 1.22 ns] | Encode: 2.77 ns<br>Decode: 2.37 ns [2.3 ns, 2.4 ns] | Encode: —<br>Decode: — |
| 8 B | Encode: 13.7 ns [13.3 ns, 13.9 ns]; heap 0 B<br>Decode: 14.9 ns; heap 0 B | Encode: 13.3 ns [13.2 ns, 13.7 ns]; heap 0 B<br>Decode: 13.1 ns; heap 0 B | Encode: 18.6 ns [18.1 ns, 19.5 ns]<br>Decode: 14.6 ns [14.4 ns, 15.4 ns] | Encode: 42.9 ns [42.1 ns, 45.7 ns]<br>Decode: 22.4 ns [21.1 ns, 23.6 ns] | Encode: —<br>Decode: — |
| 16 B | Encode: 20.1 ns [19.8 ns, 20.4 ns]; heap 0 B<br>Decode: 23.1 ns; heap 0 B | Encode: 20.1 ns [19.3 ns, 21 ns]; heap 0 B<br>Decode: 20.8 ns; heap 0 B | Encode: 28.2 ns [26.8 ns, 29.1 ns]<br>Decode: 23.1 ns | Encode: 149 ns [137 ns, 153 ns]<br>Decode: 67 ns | Encode: —<br>Decode: — |
| 21 B | Encode: 25.9 ns [25.2 ns, 27.1 ns]; heap 0 B<br>Decode: 25.6 ns; heap 0 B | Encode: 26.8 ns [26.1 ns, 28.5 ns]; heap 0 B<br>Decode: 23.4 ns; heap 0 B | Encode: 35 ns [34.1 ns, 36.4 ns]<br>Decode: 25.5 ns | Encode: 226 ns<br>Decode: 101 ns | Encode: —<br>Decode: — |
| 22 B | Encode: 29.9 ns [29.1 ns, 30.4 ns]; heap 0 B<br>Decode: 26 ns; heap 0 B | Encode: 30.5 ns [30.3 ns, 32.2 ns]; heap 0 B<br>Decode: 24.6 ns [24 ns, 25.3 ns]; heap 0 B | Encode: 39.8 ns [38 ns, 40.2 ns]<br>Decode: 26.1 ns | Encode: 268 ns [264 ns, 275 ns]<br>Decode: 115 ns [112 ns, 116 ns] | Encode: —<br>Decode: — |
| 23 B | Encode: 33.9 ns; heap 0 B<br>Decode: 32.9 ns; heap 0 B | Encode: 34.4 ns; heap 0 B<br>Decode: 29.9 ns; heap 0 B | Encode: 43.7 ns [41.3 ns, 44.4 ns]<br>Decode: 32.5 ns | Encode: 272 ns<br>Decode: 119 ns | Encode: —<br>Decode: — |
| 31 B | Encode: 50.6 ns [50 ns, 52.7 ns]; heap 0 B<br>Decode: 49.5 ns [48.1 ns, 49.8 ns]; heap 0 B | Encode: 50.3 ns [48.8 ns, 53.2 ns]; heap 0 B<br>Decode: 47.9 ns [46.9 ns, 48.5 ns]; heap 0 B | Encode: 58.8 ns [57.5 ns, 60 ns]<br>Decode: 50.6 ns | Encode: 611 ns [578 ns, 612 ns]<br>Decode: 205 ns | Encode: —<br>Decode: — |
| 32 B | Encode: 25.1 ns; heap 0 B<br>Decode: 51.9 ns; heap 0 B | Encode: 25.2 ns [24.8 ns, 25.7 ns]; heap 0 B<br>Decode: 50 ns; heap 0 B | Encode: 25.3 ns<br>Decode: 52.5 ns | Encode: 598 ns [594 ns, 639 ns]<br>Decode: 216 ns | Encode: 44.5 ns [43.3 ns, 45 ns]<br>Decode: 40.8 ns |
| 33 B | Encode: 49.9 ns [48.8 ns, 52.6 ns]; heap 0 B<br>Decode: 52.7 ns [51.5 ns, 53.3 ns]; heap 0 B | Encode: 49.4 ns [47.8 ns, 53.6 ns]; heap 0 B<br>Decode: 51.1 ns; heap 0 B | Encode: 58.2 ns<br>Decode: 53.3 ns | Encode: 625 ns<br>Decode: 229 ns | Encode: —<br>Decode: — |
| 52 B | Encode: 111 ns [111 ns, 116 ns]; heap 0 B<br>Decode: 97 ns [95.5 ns, 100 ns]; heap 0 B | Encode: 114 ns; heap 46.9 KiB<br>Decode: 93.8 ns [90.5 ns, 100 ns]; heap 0 B | Encode: 126 ns<br>Decode: 98.5 ns [96.9 ns, 101 ns] | Encode: 1.79 µs [1.75 µs, 1.84 µs]<br>Decode: 539 ns | Encode: —<br>Decode: — |
| 63 B | Encode: 148 ns [140 ns, 152 ns]; heap 0 B<br>Decode: 133 ns [131 ns, 135 ns]; heap 0 B | Encode: 121 ns; heap 46.9 KiB<br>Decode: 127 ns [125 ns, 131 ns]; heap 0 B | Encode: 159 ns [155 ns, 162 ns]<br>Decode: 134 ns [131 ns, 136 ns] | Encode: 2.71 µs<br>Decode: 791 ns | Encode: —<br>Decode: — |
| 64 B | Encode: 98.8 ns; heap 0 B<br>Decode: 136 ns [134 ns, 140 ns]; heap 0 B | Encode: 99.3 ns; heap 0 B<br>Decode: 131 ns [127 ns, 134 ns]; heap 0 B | Encode: 100 ns<br>Decode: 136 ns [134 ns, 143 ns] | Encode: 2.8 µs [2.73 µs, 2.83 µs]<br>Decode: 819 ns [816 ns, 855 ns] | Encode: 91.5 ns<br>Decode: 124 ns [123 ns, 127 ns] |
| 65 B | Encode: 155 ns [151 ns, 162 ns]; heap 0 B<br>Decode: 135 ns; heap 0 B | Encode: 178 ns [175 ns, 182 ns]; heap 46.9 KiB<br>Decode: 130 ns [128 ns, 137 ns]; heap 0 B | Encode: 167 ns [165 ns, 171 ns]<br>Decode: 140 ns [135 ns, 144 ns] | Encode: 2.91 µs [2.85 µs, 3.08 µs]<br>Decode: 849 ns [838 ns, 869 ns] | Encode: —<br>Decode: — |
| 128 B | Encode: 512 ns [499 ns, 534 ns]; heap 0 B<br>Decode: 441 ns [429 ns, 443 ns]; heap 0 B | Encode: 294 ns; heap 46.9 KiB<br>Decode: 428 ns; heap 0 B | Encode: 524 ns [513 ns, 537 ns]<br>Decode: 432 ns [429 ns, 442 ns] | Encode: 12.8 µs<br>Decode: 3.48 µs | Encode: —<br>Decode: — |
| 256 B | Encode: 1.87 µs [1.84 µs, 2 µs]; heap 0 B<br>Decode: 1.62 µs [1.49 µs, 1.64 µs]; heap 0 B | Encode: 855 ns; heap 46.9 KiB<br>Decode: 1.58 µs [1.49 µs, 1.62 µs]; heap 0 B | Encode: 1.96 µs [1.94 µs, 2.06 µs]<br>Decode: 1.62 µs [1.49 µs, 1.66 µs] | Encode: 55.4 µs [54.7 µs, 57.3 µs]<br>Decode: 16.2 µs | Encode: —<br>Decode: — |
| 512 B | Encode: 7.15 µs [7.05 µs, 7.68 µs]; heap 0 B<br>Decode: 6.06 µs [5.49 µs, 6.19 µs]; heap 0 B | Encode: 2.77 µs; heap 46.9 KiB<br>Decode: 2.49 µs; heap 40 KiB | Encode: 7.36 µs [7.23 µs, 7.92 µs]<br>Decode: 5.87 µs [5.51 µs, 6.04 µs] | Encode: 232 µs<br>Decode: 73 µs | Encode: —<br>Decode: — |
| 1 KiB | Encode: 27.6 µs [27 µs, 30.1 µs]; heap 0 B<br>Decode: 23.3 µs [21 µs, 23.4 µs]; heap 0 B | Encode: 10.1 µs; heap 181 KiB<br>Decode: 8.8 µs [8.65 µs, 9.13 µs]; heap 154 KiB | Encode: 28.3 µs [27.9 µs, 31 µs]<br>Decode: 23.1 µs [20.9 µs, 25 µs] | Encode: 940 µs [933 µs, 976 µs]<br>Decode: 310 µs [306 µs, 328 µs] | Encode: —<br>Decode: — |
| 1,504 B | Encode: 58 µs [57.6 µs, 64 µs]; heap 0 B<br>Decode: 49.9 µs [44.5 µs, 50.4 µs]; heap 0 B | Encode: 20.8 µs; heap 712 KiB<br>Decode: 18.2 µs; heap 602 KiB | Encode: 61 µs [58.3 µs, 65.3 µs]<br>Decode: 48.7 µs [44.1 µs, 49.9 µs] | Encode: 2.04 ms<br>Decode: 682 µs [675 µs, 696 µs] | Encode: —<br>Decode: — |
| 1,875 B | Encode: 91.2 µs [88.6 µs, 101 µs]; heap 0 B<br>Decode: 64.2 µs [62.5 µs, 67.2 µs]; heap 13.3 KiB | Encode: 32.2 µs; heap 712 KiB<br>Decode: 27.6 µs; heap 602 KiB | Encode: 92.5 µs [91.6 µs, 104 µs]<br>Decode: 69.6 µs [67.4 µs, 73.9 µs] | Encode: 3.17 ms [3.15 ms, 3.32 ms]<br>Decode: 1.08 ms [1.05 ms, 1.1 ms] | Encode: —<br>Decode: — |
| 2 KiB | Encode: 109 µs [105 µs, 114 µs]; heap 0 B<br>Decode: 68 µs [66.7 µs, 69.6 µs]; heap 13.7 KiB | Encode: 37.8 µs; heap 712 KiB<br>Decode: 32.4 µs; heap 602 KiB | Encode: 112 µs [107 µs, 121 µs]<br>Decode: 89.6 µs [81.1 µs, 93.1 µs] | Encode: 3.81 ms [3.76 ms, 3.9 ms]<br>Decode: 1.28 ms | Encode: —<br>Decode: — |
| 4 KiB | Encode: 286 µs; heap 82.3 KiB<br>Decode: 189 µs; heap 56.8 KiB | Encode: 146 µs; heap 2.76 MiB<br>Decode: 122 µs; heap 2.32 MiB | Encode: 445 µs [430 µs, 464 µs]<br>Decode: 336 µs [321 µs, 368 µs] | Encode: 15.3 ms [15 ms, 15.6 ms]<br>Decode: 5.18 ms [5.1 ms, 5.25 ms] | Encode: —<br>Decode: — |
| 8 KiB | Encode: 686 µs; heap 164 KiB<br>Decode: 465 µs; heap 119 KiB | Encode: 577 µs; heap 11 MiB<br>Decode: 456 µs; heap 126 KiB | Encode: 1.73 ms<br>Decode: 1.27 ms [1.24 ms, 1.37 ms] | Encode: 60.9 ms<br>Decode: 20.5 ms | Encode: —<br>Decode: — |
| 16 KiB | Encode: 1.61 ms; heap 328 KiB<br>Decode: 1.11 ms; heap 243 KiB | Encode: 1.6 ms; heap 392 KiB<br>Decode: 1.09 ms [1.08 ms, 1.11 ms]; heap 257 KiB | Encode: 6.96 ms [6.77 ms, 7.76 ms]<br>Decode: 5.18 ms [4.81 ms, 5.4 ms] | Encode: 243 ms<br>Decode: 83 ms [81.4 ms, 83.9 ms] | Encode: —<br>Decode: — |
| 24 KiB | Encode: 3.62 ms; heap 612 KiB<br>Decode: 1.99 ms [1.96 ms, 2.02 ms]; heap 456 KiB | Encode: 3.59 ms; heap 740 KiB<br>Decode: 1.94 ms; heap 456 KiB | Encode: 15.6 ms [15.5 ms, 17.5 ms]<br>Decode: 12.1 ms [11.5 ms, 13 ms] | Encode: 552 ms<br>Decode: 184 ms | Encode: —<br>Decode: — |
| 28 KiB | Encode: 3.68 ms; heap 633 KiB<br>Decode: 2.42 ms [2.37 ms, 2.46 ms]; heap 474 KiB | Encode: 3.67 ms; heap 761 KiB<br>Decode: 2.38 ms [2.35 ms, 2.43 ms]; heap 474 KiB | Encode: 21.3 ms [20.9 ms, 21.6 ms]<br>Decode: 16 ms [15.3 ms, 17.4 ms] | Encode: 752 ms<br>Decode: 254 ms [250 ms, 257 ms] | Encode: —<br>Decode: — |
| 32 KiB | Encode: 3.74 ms; heap 655 KiB<br>Decode: 2.59 ms [2.58 ms, 2.7 ms]; heap 490 KiB | Encode: 3.71 ms [3.69 ms, 4.53 ms]; heap 783 KiB<br>Decode: 2.58 ms [2.53 ms, 2.68 ms]; heap 518 KiB | Encode: 28.9 ms [27.6 ms, 35 ms]<br>Decode: 20.3 ms [19.3 ms, 22.2 ms] | Encode: 984 ms<br>Decode: 330 ms [328 ms, 358 ms] | Encode: —<br>Decode: — |
| 36 KiB | Encode: 7.19 ms; heap 1.15 MiB<br>Decode: 3.21 ms; heap 838 KiB | Encode: 7.14 ms; heap 1.15 MiB<br>Decode: 3.14 ms; heap 838 KiB | Encode: 35 ms [34.3 ms, 37.3 ms]<br>Decode: 26 ms [25.1 ms, 26.4 ms] | Encode: 1.24 s<br>Decode: 420 ms [416 ms, 431 ms] | Encode: —<br>Decode: — |
| 48 KiB | Encode: 8.32 ms; heap 1.19 MiB<br>Decode: 4.58 ms; heap 924 KiB | Encode: 8.27 ms; heap 1.44 MiB<br>Decode: 4.51 ms; heap 924 KiB | Encode: 61.9 ms [61.6 ms, 66.5 ms]<br>Decode: 44.8 ms [44.3 ms, 46.6 ms] | Encode: 2.21 s<br>Decode: 740 ms | Encode: —<br>Decode: — |
| 64 KiB | Encode: 8.58 ms; heap 1.28 MiB<br>Decode: 5.98 ms; heap 986 KiB | Encode: 8.51 ms; heap 1.53 MiB<br>Decode: 5.93 ms; heap 1.02 MiB | Encode: 111 ms [109 ms, 117 ms]<br>Decode: 81.3 ms [78.4 ms, 82.2 ms] | Encode: 3.93 s<br>Decode: 1.33 s [1.31 s, 1.38 s] | Encode: —<br>Decode: — |
| 128 KiB | Encode: 19.6 ms; heap 2.56 MiB<br>Decode: 13.7 ms; heap 1.93 MiB | Encode: 19.6 ms; heap 3.06 MiB<br>Decode: 13.7 ms; heap 2.04 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 144 KiB | Encode: 36.3 ms; heap 4.65 MiB<br>Decode: 16.6 ms; heap 3.31 MiB | Encode: 36 ms; heap 4.65 MiB<br>Decode: 16.4 ms; heap 3.31 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 256 KiB | Encode: 44 ms [43.6 ms, 45 ms]; heap 4.62 MiB<br>Decode: 31.1 ms; heap 3.86 MiB | Encode: 44 ms [43.3 ms, 45.1 ms]; heap 5.37 MiB<br>Decode: 30.8 ms; heap 4.08 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 512 KiB | Encode: 98.1 ms; heap 8.73 MiB<br>Decode: 70.3 ms [69.1 ms, 72.2 ms]; heap 6.98 MiB | Encode: 97.7 ms; heap 9.98 MiB<br>Decode: 69.2 ms; heap 7.42 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 1 MiB | Encode: 220 ms; heap 17 MiB<br>Decode: 155 ms; heap 13.2 MiB | Encode: 220 ms; heap 17 MiB<br>Decode: 155 ms; heap 13.2 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 2 MiB | Encode: 486 ms; heap 33.4 MiB<br>Decode: 346 ms; heap 25.7 MiB | Encode: 484 ms; heap 33.4 MiB<br>Decode: 345 ms; heap 25.7 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 4 MiB | Encode: 1.07 s; heap 66.4 MiB<br>Decode: 766 ms; heap 50.7 MiB | Encode: 1.07 s; heap 66.4 MiB<br>Decode: 765 ms; heap 50.7 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |

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

Each cell labels absolute **Encode** and **Decode** medians. These rows are independent
of the scalar README figure; linked sizes are listed by ISA above. Brackets show
observed process ranges when the spread exceeds 3%.
Auto cells give peak internal heap beside each direction under the same
convention as the scalar tables.

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 32 B | Encode: 24.7 ns; heap 0 B<br>Decode: 46.7 ns [45.9 ns, 47.5 ns]; heap 0 B | Encode: 25 ns; heap 0 B<br>Decode: 44.1 ns; heap 0 B | Encode: 25.3 ns [25.2 ns, 26 ns]<br>Decode: 47.4 ns [46.1 ns, 47.7 ns] | Encode: 597 ns [585 ns, 622 ns]<br>Decode: 208 ns [205 ns, 211 ns] | Encode: 44.3 ns [43.3 ns, 44.9 ns]<br>Decode: 39.8 ns |
| 64 B | Encode: 60.6 ns [59.1 ns, 61.6 ns]; heap 0 B<br>Decode: 127 ns; heap 0 B | Encode: 62 ns; heap 0 B<br>Decode: 121 ns [119 ns, 123 ns]; heap 0 B | Encode: 59.3 ns<br>Decode: 128 ns [126 ns, 131 ns] | Encode: 2.8 µs [2.71 µs, 2.9 µs]<br>Decode: 787 ns | Encode: 91.8 ns [89.9 ns, 93.5 ns]<br>Decode: 104 ns |
| 512 B | Encode: 7.34 µs [7.14 µs, 8 µs]; heap 0 B<br>Decode: 6.02 µs [5.54 µs, 6.08 µs]; heap 0 B | Encode: 1.12 µs [1.11 µs, 1.23 µs]; heap 46.9 KiB<br>Decode: 968 ns [950 ns, 1.06 µs]; heap 40 KiB | Encode: 7.77 µs [7.29 µs, 7.81 µs]<br>Decode: 6.03 µs [5.53 µs, 6.31 µs] | Encode: 237 µs [231 µs, 261 µs]<br>Decode: 74.3 µs [72.5 µs, 82.1 µs] | Encode: —<br>Decode: — |
| 4 KiB | Encode: 107 µs; heap 82.3 KiB<br>Decode: 118 µs [115 µs, 120 µs]; heap 56.8 KiB | Encode: 52 µs [51.1 µs, 52.7 µs]; heap 2.76 MiB<br>Decode: 40.1 µs [39.7 µs, 41 µs]; heap 2.32 MiB | Encode: 433 µs [423 µs, 462 µs]<br>Decode: 348 µs [324 µs, 366 µs] | Encode: 15.3 ms [15.1 ms, 16.1 ms]<br>Decode: 5.39 ms [5.12 ms, 5.59 ms] | Encode: —<br>Decode: — |
| 64 KiB | Encode: 3.03 ms [2.96 ms, 3.06 ms]; heap 1.28 MiB<br>Decode: 2.65 ms [2.59 ms, 2.71 ms]; heap 986 KiB | Encode: 2.85 ms; heap 1.53 MiB<br>Decode: 2.63 ms [2.56 ms, 2.66 ms]; heap 1.02 MiB | Encode: 111 ms [108 ms, 119 ms]<br>Decode: 78.3 ms [77.3 ms, 82.4 ms] | Encode: 3.93 s [3.88 s, 4.14 s]<br>Decode: 1.32 s [1.31 s, 1.4 s] | Encode: —<br>Decode: — |
| 1 MiB | Encode: 72.8 ms; heap 17 MiB<br>Decode: 61 ms [60.1 ms, 62 ms]; heap 13.2 MiB | Encode: 70.5 ms; heap 17 MiB<br>Decode: 60.5 ms; heap 13.2 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |
| 4 MiB | Encode: 350 ms; heap 66.4 MiB<br>Decode: 287 ms; heap 50.7 MiB | Encode: 328 ms [324 ms, 346 ms]; heap 66.4 MiB<br>Decode: 285 ms; heap 50.7 MiB | Encode: —<br>Decode: — | Encode: —<br>Decode: — | Encode: —<br>Decode: — |

## Cache and memory

`cache` changes dispatch and leaf policy as well as retention. First calls can
build tables. Code-derived capacities and ownership are in
[DESIGN.md](../DESIGN.md#heap-stack-and-cache-costs). Per-call scratch, owned tables,
allocator overhead and concurrent calls add to retention. Retained capacity is
not an active-heap bound. Wasm linear-memory pages need not return to the host.

Heap measurements dated **2026-11-15** use source
[`d94a8567385f`](https://github.com/elichai/faster-base58/commit/d94a8567385fc20feb1cf8896289704856e2d878).
The M5, GCP and scalar/SIMD128 Wasm hosts and compilers match the platform
descriptions above: Rust 1.99.1 on M5/Wasm, Rust 1.99.0 on GCP and Node 26.13.1 /
V8 14.6.202.34 for Wasm. Builds use generic CPU flags, opt-level 3, fat LTO and
16 codegen units. Default, `cache` and no-default-feature consumers are separate
builds. None enables `__internals`, `check` or `compact`.

`scripts/resource_report.py public-heap` runs counted `code_size.py` public consumers. Each
operation, feature policy and size starts a fresh native process or Wasm instance.
One first call and one warm call are counted, with three fresh-process repeats.
The independent integer oracle constructs a significant integer of exactly the
reported byte length as `(58**digits - 1) // 57`; its Bitcoin Base58 encoding is
`digits` copies of `2`. These fixtures contain no leading zeros and differ from
the timing fixtures. Exact output, unchanged inputs, output tails and buffer guards
are checked outside each counting window. Empty consumers and no-allocation
consumers provide zero-allocation controls through 64 KiB.

**Only library-internal heap appears in the tables.** Caller input, output and
guard buffers are allocated before counting and subtracted from live storage.
Owned result storage is excluded; `.into_string()` / `.into_vec()` need their
output capacity in addition to these borrowed-conversion costs. The warm peak
includes both per-call workspace and tables retained by this case's first call.
It is not just newly allocated bytes. Encode and decode run in independent
processes; their retained tables are not combined. A mixed workload can retain
more classes, up to the code-derived cache capacities in DESIGN.md.

Requests count allocations plus reallocations. Peaks track live Rust requested
layouts, with the net live-size change on reallocation. They exclude stack,
allocator overhead, temporary storage inside the allocator, Wasm linear-memory
page slack and RSS. Input values, leading zeros, CPU dispatch and call history can
change heap use; these measurements are not upper bounds. No-allocation controls
make zero requests. All three repeats agree exactly for every published cell.

The tables below separate first-call peak, retained storage and request counts.
Each cell labels encode and decode; cached request cells also label cold and warm calls.
Default first-call and warm peaks and request counts match for these fixtures.

<details>
<summary>Apple M5 Pro: first calls and retained heap</summary>

| Payload | Default requests | Cached first-call peak | Cached retained heap | Cached requests |
| --- | ---: | ---: | ---: | ---: |
| 0 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 32 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 64 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 512 B | Encode: 0<br>Decode: 0 | Encode: 52.6 KiB<br>Decode: 44.3 KiB | Encode: 47 KiB<br>Decode: 40.2 KiB | Cold: encode 3, decode 3<br>Warm: encode 0, decode 0 |
| 4 KiB | Encode: 7<br>Decode: 0 | Encode: 2.8 MiB<br>Decode: 2.36 MiB | Encode: 2.76 MiB<br>Decode: 2.32 MiB | Cold: encode 3, decode 3<br>Warm: encode 0, decode 0 |
| 64 KiB | Encode: 7<br>Decode: 11 | Encode: 1.53 MiB<br>Decode: 1.18 MiB | Encode: 768 KiB<br>Decode: 538 KiB | Cold: encode 8, decode 14<br>Warm: encode 5, decode 8 |
| 1 MiB | Encode: 7<br>Decode: 16 | Encode: 17 MiB<br>Decode: 13.4 MiB | Encode: 0 B<br>Decode: 3.9 MiB | Cold: encode 7, decode 19<br>Warm: encode 7, decode 13 |
| 4 MiB | Encode: 7<br>Decode: 18 | Encode: 66.4 MiB<br>Decode: 51.3 MiB | Encode: 0 B<br>Decode: 602 KiB | Cold: encode 7, decode 21<br>Warm: encode 7, decode 18 |

</details>

<details>
<summary>AMD EPYC 9B45 (generic x86-64): first calls and retained heap</summary>

| Payload | Default requests | Cached first-call peak | Cached retained heap | Cached requests |
| --- | ---: | ---: | ---: | ---: |
| 0 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 32 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 64 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 512 B | Encode: 5<br>Decode: 0 | Encode: 52.6 KiB<br>Decode: 0 B | Encode: 47 KiB<br>Decode: 0 B | Cold: encode 3, decode 0<br>Warm: encode 0, decode 0 |
| 4 KiB | Encode: 5<br>Decode: 1 | Encode: 97.4 KiB<br>Decode: 30.3 KiB | Encode: 47 KiB<br>Decode: 0 B | Cold: encode 8, decode 1<br>Warm: encode 5, decode 1 |
| 64 KiB | Encode: 7<br>Decode: 6 | Encode: 1.53 MiB<br>Decode: 1000 KiB | Encode: 768 KiB<br>Decode: 384 KiB | Cold: encode 8, decode 8<br>Warm: encode 5, decode 5 |
| 1 MiB | Encode: 7<br>Decode: 11 | Encode: 17 MiB<br>Decode: 13.2 MiB | Encode: 0 B<br>Decode: 3.75 MiB | Cold: encode 7, decode 12<br>Warm: encode 7, decode 9 |
| 4 MiB | Encode: 7<br>Decode: 13 | Encode: 66.4 MiB<br>Decode: 50.6 MiB | Encode: 0 B<br>Decode: 0 B | Cold: encode 7, decode 15<br>Warm: encode 7, decode 15 |

</details>

<details>
<summary>Scalar Wasm on M5: first calls and retained heap</summary>

| Payload | Default requests | Cached first-call peak | Cached retained heap | Cached requests |
| --- | ---: | ---: | ---: | ---: |
| 0 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 32 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 64 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 512 B | Encode: 0<br>Decode: 0 | Encode: 52.5 KiB<br>Decode: 44.2 KiB | Encode: 46.9 KiB<br>Decode: 40 KiB | Cold: encode 3, decode 3<br>Warm: encode 0, decode 0 |
| 4 KiB | Encode: 7<br>Decode: 6 | Encode: 2.8 MiB<br>Decode: 2.36 MiB | Encode: 2.76 MiB<br>Decode: 2.32 MiB | Cold: encode 3, decode 3<br>Warm: encode 0, decode 0 |
| 64 KiB | Encode: 7<br>Decode: 11 | Encode: 1.53 MiB<br>Decode: 1.02 MiB | Encode: 768 KiB<br>Decode: 384 KiB | Cold: encode 8, decode 11<br>Warm: encode 5, decode 8 |
| 1 MiB | Encode: 7<br>Decode: 16 | Encode: 17 MiB<br>Decode: 13.2 MiB | Encode: 0 B<br>Decode: 3.75 MiB | Cold: encode 7, decode 16<br>Warm: encode 7, decode 13 |
| 4 MiB | Encode: 7<br>Decode: 18 | Encode: 66.4 MiB<br>Decode: 50.7 MiB | Encode: 0 B<br>Decode: 0 B | Cold: encode 7, decode 18<br>Warm: encode 7, decode 18 |

</details>

<details>
<summary>SIMD128 Wasm on M5: first calls and retained heap</summary>

| Payload | Default requests | Cached first-call peak | Cached retained heap | Cached requests |
| --- | ---: | ---: | ---: | ---: |
| 0 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 32 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 64 B | Encode: 0<br>Decode: 0 | Encode: 0 B<br>Decode: 0 B | Encode: 0 B<br>Decode: 0 B | Cold: encode 0, decode 0<br>Warm: encode 0, decode 0 |
| 512 B | Encode: 0<br>Decode: 0 | Encode: 52.5 KiB<br>Decode: 44.2 KiB | Encode: 46.9 KiB<br>Decode: 40 KiB | Cold: encode 3, decode 3<br>Warm: encode 0, decode 0 |
| 4 KiB | Encode: 7<br>Decode: 6 | Encode: 2.8 MiB<br>Decode: 2.36 MiB | Encode: 2.76 MiB<br>Decode: 2.32 MiB | Cold: encode 3, decode 3<br>Warm: encode 0, decode 0 |
| 64 KiB | Encode: 7<br>Decode: 11 | Encode: 1.53 MiB<br>Decode: 1.02 MiB | Encode: 768 KiB<br>Decode: 384 KiB | Cold: encode 8, decode 11<br>Warm: encode 5, decode 8 |
| 1 MiB | Encode: 7<br>Decode: 16 | Encode: 17 MiB<br>Decode: 13.2 MiB | Encode: 0 B<br>Decode: 3.75 MiB | Cold: encode 7, decode 16<br>Warm: encode 7, decode 13 |
| 4 MiB | Encode: 7<br>Decode: 18 | Encode: 66.4 MiB<br>Decode: 50.7 MiB | Encode: 0 B<br>Decode: 0 B | Cold: encode 7, decode 18<br>Warm: encode 7, decode 18 |

</details>

For internal backend studies, `scripts/resource_report.py build --count-allocations`
and `run` remain available. Those workers enable `__internals` and are separate
from the public-consumer cohort here.

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

Scalar and SIMD128 Wasm are measured separately. Public heap counts have their
own [untimed cohort](#cache-and-memory). Internal backends, dispatch thresholds,
Base58Check and compact configurations require separate, source-bound cohorts.
Those scopes are outside these timing cohorts.
Wasm timing uses Node/V8 elapsed time, not native process CPU time. Build with
`--target wasm32-unknown-unknown`, optionally `--wasm-simd128`, and use
`scripts/public_bench.py --wasm`. Allocation-counting modules are excluded from
ordinary timing. Criterion uses wall time; some comparator adapters own their
results while Auto reuses caller buffers, so report that distinction.

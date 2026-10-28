# wasm / cache / kernel-controls

Frozen `4e5ac883…` backend comparison. Native M5 uses an all-backend `__internals` worker; Wasm links each backend separately and selectively enables internal hooks. These are not the later ordinary native public API curves.

Time is the median of nine warm batch observations from three fresh processes × three batches. Native uses process CPU; Wasm uses elapsed time. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **1.87 ns** | 2.47 ns | 9.19 ns | 9.17 ns | 9.2 ns | 9.1 ns | 9.1 ns | — | 4.73 ns |
| 1 B | **2.6 ns** | 2.9 ns | 16.9 ns | 17.1 ns | 16.6 ns | 16.6 ns | 16.8 ns | 41.1 ns | 13.5 ns |
| 8 B | **22.5 ns** | 28 ns | 28.1 ns | 29.2 ns | 27.3 ns | 28 ns | 26.9 ns | 43.8 ns | 23 ns |
| 16 B | **37.7 ns** | 44.3 ns | 44.4 ns | 43.4 ns | 43.3 ns | 43.8 ns | 43.7 ns | 49.3 ns | 39.1 ns |
| 21 B | **40.7 ns** | 45.4 ns | 46.1 ns | 48 ns | 45.5 ns | 46.5 ns | 45.7 ns | 54.1 ns | 43 ns |
| 22 B | **51.7 ns** | 54.3 ns | 53.6 ns | 54.2 ns | 54.6 ns | 54.1 ns | 54.8 ns | 54.7 ns | 56.1 ns |
| 23 B | **52.4 ns** | 59.1 ns | 59.4 ns | 58.9 ns | 61.3 ns | 58.8 ns | 59.2 ns | 54.7 ns | 56 ns |
| 31 B | **58.4 ns** | 81.3 ns | 80.2 ns | 81 ns | 81.6 ns | 81.4 ns | 81 ns | 60.1 ns | 61.9 ns |
| 32 B | **25.6 ns** | 80.5 ns | 82 ns | 80.4 ns | 81.3 ns | 81.7 ns | 81.4 ns | 61.4 ns | 61.5 ns |
| 33 B | 102 ns | 82.5 ns | 82.3 ns | 81.3 ns | 81.9 ns | **81.1 ns** | 83.8 ns | 103 ns | 114 ns |
| 63 B | 123 ns | 178 ns | 181 ns | 182 ns | 180 ns | 181 ns | 181 ns | **122 ns** | 130 ns |
| 64 B | **99.6 ns** | 194 ns | 191 ns | 188 ns | 190 ns | 209 ns | 189 ns | 124 ns | 135 ns |
| 65 B | **177 ns** | 198 ns | 198 ns | 198 ns | 197 ns | 198 ns | 208 ns | 180 ns | 181 ns |
| 128 B | 296 ns | 574 ns | 592 ns | 587 ns | 584 ns | 587 ns | 576 ns | **296 ns** | 297 ns |
| 256 B | **860 ns** | 2.08 µs | 2.12 µs | 2.1 µs | 2.08 µs | 2.14 µs | 2.24 µs | 861 ns | 884 ns |
| 512 B | 2.85 µs | 7.84 µs | 7.74 µs | 7.71 µs | 7.79 µs | 7.77 µs | 7.81 µs | **2.78 µs** | 2.82 µs |
| 1 KiB | 10 µs | 30.5 µs | 32.2 µs | 30.3 µs | 30.3 µs | 30.1 µs | 29.8 µs | **10 µs** | 10.3 µs |
| 2 KiB | **37.6 µs** | 117 µs | 118 µs | 118 µs | 121 µs | 117 µs | 117 µs | 37.8 µs | 38.1 µs |
| 4 KiB | 147 µs | 466 µs | 464 µs | 469 µs | 467 µs | 469 µs | 460 µs | **147 µs** | 149 µs |
| 8 KiB | 588 µs | 1.95 ms | 1.16 ms | 1.14 ms | 1.16 ms | 1.17 ms | 1.15 ms | **584 µs** | 598 µs |
| 16 KiB | **2.68 ms** | 7.28 ms | 2.74 ms | 2.82 ms | 2.72 ms | 2.74 ms | 2.72 ms | — | 2.94 ms |
| 32 KiB | 6.26 ms | 29.4 ms | 6.39 ms | 6.23 ms | **6.22 ms** | 6.31 ms | 6.26 ms | — | 6.27 ms |
| 64 KiB | **14 ms** | 117 ms | 14.3 ms | 14.1 ms | 14.1 ms | 14.2 ms | 14.1 ms | — | 14.6 ms |
| 128 KiB | **31.3 ms** | — | 31.6 ms | 31.7 ms | 31.4 ms | 31.7 ms | 31.7 ms | — | 31.9 ms |
| 256 KiB | **69.3 ms** | — | 70.3 ms | 69.7 ms | 69.7 ms | 70.7 ms | 70.8 ms | — | 71.1 ms |
| 512 KiB | **154 ms** | — | 156 ms | 154 ms | 155 ms | 155 ms | 154 ms | — | 157 ms |
| 1 MiB | **339 ms** | — | 342 ms | 340 ms | 340 ms | 340 ms | 341 ms | — | 344 ms |
| 2 MiB | **740 ms** | — | 749 ms | 745 ms | 745 ms | 753 ms | 746 ms | — | 742 ms |
| 4 MiB | **1.62 s** | — | 1.63 s | 1.62 s | 1.63 s | 1.64 s | 1.63 s | — | 1.64 s |

## Decode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **2.08 ns** | 9.25 ns | 12.1 ns | 12.6 ns | 12.2 ns | 12 ns | 12.1 ns | — | 9.52 ns |
| 1 B | **2.87 ns** | 11.4 ns | 15 ns | 15.2 ns | 15.4 ns | 15.3 ns | 15.1 ns | 35 ns | 11.6 ns |
| 8 B | 17.7 ns | 17.8 ns | 21.2 ns | 21.3 ns | 20.9 ns | 21.1 ns | 21.2 ns | 39 ns | **17.5 ns** |
| 16 B | **25.5 ns** | 25.6 ns | 28.9 ns | 29.2 ns | 28.6 ns | 29.3 ns | 30.2 ns | 45.9 ns | 26.2 ns |
| 21 B | 30.7 ns | **29.5 ns** | 33.2 ns | 33.4 ns | 32.9 ns | 35.2 ns | 33.9 ns | 51.8 ns | 32.1 ns |
| 22 B | 31.8 ns | **31.1 ns** | 35.3 ns | 34.2 ns | 34.6 ns | 34.8 ns | 34.7 ns | 52.8 ns | 33 ns |
| 23 B | 39.2 ns | **38.5 ns** | 42.4 ns | 42 ns | 41 ns | 42.9 ns | 43.1 ns | 51.9 ns | 40 ns |
| 31 B | 53.2 ns | **52 ns** | 55.9 ns | 54.4 ns | 55.3 ns | 54.8 ns | 55.4 ns | 81.8 ns | 52.4 ns |
| 32 B | 53.8 ns | **52.2 ns** | 54.3 ns | 54.7 ns | 54.5 ns | 55.7 ns | 55.1 ns | 79.5 ns | 55.6 ns |
| 33 B | **52.9 ns** | 53.1 ns | 54.7 ns | 55.4 ns | 55.1 ns | 56.6 ns | 56.8 ns | 83.3 ns | 54 ns |
| 63 B | 129 ns | 126 ns | **124 ns** | 126 ns | 136 ns | 133 ns | 137 ns | 141 ns | 132 ns |
| 64 B | 133 ns | **130 ns** | 130 ns | 136 ns | 132 ns | 135 ns | 133 ns | 137 ns | 135 ns |
| 65 B | 134 ns | 131 ns | 132 ns | **130 ns** | 137 ns | 135 ns | 134 ns | 147 ns | 136 ns |
| 128 B | 418 ns | 427 ns | 424 ns | 424 ns | 422 ns | 426 ns | 424 ns | **342 ns** | 422 ns |
| 256 B | 1.38 µs | 1.36 µs | 1.41 µs | 1.38 µs | 1.38 µs | 1.39 µs | 1.39 µs | **845 ns** | 1.39 µs |
| 512 B | 2.65 µs | 4.87 µs | 5 µs | 4.93 µs | 5.08 µs | 4.9 µs | 4.97 µs | **2.61 µs** | 2.78 µs |
| 1 KiB | 9.17 µs | 17.8 µs | 18.2 µs | 18.1 µs | 17.9 µs | 18 µs | 18.2 µs | **9.03 µs** | 9.09 µs |
| 2 KiB | 33.3 µs | 66.6 µs | 70.9 µs | 70 µs | 71.7 µs | 69.3 µs | 68.4 µs | **32.7 µs** | 33.4 µs |
| 4 KiB | **124 µs** | 277 µs | 216 µs | 212 µs | 212 µs | 217 µs | 211 µs | 125 µs | 126 µs |
| 8 KiB | **504 µs** | 1.07 ms | 519 µs | 513 µs | 525 µs | 523 µs | 517 µs | — | 663 µs |
| 16 KiB | 1.22 ms | 4.17 ms | 1.23 ms | **1.22 ms** | 1.22 ms | 1.23 ms | 1.22 ms | — | 1.91 ms |
| 32 KiB | **2.81 ms** | 17 ms | 2.88 ms | 2.82 ms | 2.84 ms | 2.86 ms | 2.87 ms | — | 5.66 ms |
| 64 KiB | **6.46 ms** | 67.3 ms | 6.64 ms | 6.6 ms | 6.61 ms | 6.58 ms | 6.61 ms | — | 17 ms |
| 128 KiB | **14.9 ms** | — | 15 ms | 15 ms | 14.9 ms | 15.2 ms | 15 ms | — | 50.5 ms |
| 256 KiB | **33.5 ms** | — | 34 ms | 33.8 ms | 33.8 ms | 34.3 ms | 33.6 ms | — | 151 ms |
| 512 KiB | **75.1 ms** | — | 76.5 ms | 75.4 ms | 75.7 ms | 75.9 ms | 75.5 ms | — | 455 ms |
| 1 MiB | **168 ms** | — | 169 ms | 168 ms | 168 ms | 171 ms | 168 ms | — | 1.34 s |
| 2 MiB | **372 ms** | — | 376 ms | 375 ms | 378 ms | 375 ms | 375 ms | — | 4.04 s |
| 4 MiB | **818 ms** | — | 833 ms | 825 ms | 831 ms | 826 ms | 824 ms | — | 12.1 s |

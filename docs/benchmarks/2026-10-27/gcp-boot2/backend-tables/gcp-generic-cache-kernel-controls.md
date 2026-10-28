# gcp-generic / cache / kernel-controls

Frozen `4e5ac883…` backend comparison on AMD EPYC 9B45 / c4d-highcpu-4. Each CPU/cache cohort uses one immutable all-backend `__internals` worker. Generic uses default target flags; native uses actual `target-cpu=native`. These are separate from final-source304dc ordinary compact comparisons.

Time is the median of nine warm batch observations from three fresh processes × three batches. Time uses process CPU. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 2.91 ns | **2.42 ns** | 13.6 ns | 15.7 ns | 8.32 ns | 15.5 ns | 15.6 ns | — | 4.36 ns |
| 1 B | 3.15 ns | **2.91 ns** | 26 ns | 26 ns | 22.4 ns | 26.3 ns | 26.3 ns | 29 ns | 17.9 ns |
| 8 B | 28.6 ns | **28.1 ns** | 36.5 ns | 36.6 ns | 33.1 ns | 36.9 ns | 36.8 ns | 34.2 ns | 28.8 ns |
| 16 B | 40 ns | **39.6 ns** | 48.5 ns | 47.7 ns | 44.2 ns | 48.2 ns | 48.7 ns | 41.3 ns | 40.2 ns |
| 21 B | 41.3 ns | **40.7 ns** | 49.5 ns | 49.9 ns | 45.4 ns | 51.2 ns | 49.9 ns | 46.5 ns | 41.4 ns |
| 22 B | 46.9 ns | 46.2 ns | 54.7 ns | 54.2 ns | 51 ns | 54.8 ns | 54.9 ns | **46.2 ns** | 54.9 ns |
| 23 B | 46.8 ns | 53.2 ns | 61.4 ns | 61.3 ns | 58.1 ns | 62.4 ns | 61.8 ns | **46 ns** | 54.8 ns |
| 31 B | 53.9 ns | 62.3 ns | 73.9 ns | 74 ns | 67 ns | 73.9 ns | 74.4 ns | **52.8 ns** | 62 ns |
| 32 B | **31 ns** | 62.7 ns | 74.8 ns | 73.7 ns | 67.5 ns | 74.5 ns | 74.3 ns | 51.4 ns | 60.4 ns |
| 33 B | 64.8 ns | **62.9 ns** | 75 ns | 75.1 ns | 67.6 ns | 75.2 ns | 75.3 ns | 64.2 ns | 94.4 ns |
| 63 B | 88.5 ns | 136 ns | 146 ns | 147 ns | 140 ns | 149 ns | 147 ns | **88.3 ns** | 117 ns |
| 64 B | **32 ns** | 136 ns | 149 ns | 149 ns | 142 ns | 150 ns | 150 ns | 84.6 ns | 114 ns |
| 65 B | 102 ns | 141 ns | 153 ns | 152 ns | 144 ns | 153 ns | 153 ns | **101 ns** | 157 ns |
| 128 B | 169 ns | 374 ns | 388 ns | 392 ns | 390 ns | 384 ns | 385 ns | **166 ns** | 256 ns |
| 256 B | 402 ns | 1.19 µs | 1.2 µs | 1.2 µs | 1.2 µs | 1.2 µs | 1.2 µs | **393 ns** | 697 ns |
| 512 B | 1.1 µs | 4.25 µs | 4.27 µs | 4.26 µs | 4.26 µs | 4.27 µs | 4.28 µs | **1.09 µs** | 2.24 µs |
| 1 KiB | 3.37 µs | 16.2 µs | 8.55 µs | 6.7 µs | 6.68 µs | 6.68 µs | 6.69 µs | **3.36 µs** | 7.61 µs |
| 2 KiB | 11.9 µs | 63 µs | 20 µs | 15.7 µs | 15.7 µs | 15.8 µs | 15.7 µs | **11.8 µs** | 28 µs |
| 4 KiB | 41.3 µs | 247 µs | 51.8 µs | 41.4 µs | 41.5 µs | **41.3 µs** | 41.3 µs | 44.5 µs | 41.4 µs |
| 8 KiB | 117 µs | 979 µs | 143 µs | 116 µs | **116 µs** | 117 µs | 116 µs | 167 µs | 117 µs |
| 16 KiB | 345 µs | 3.89 ms | 421 µs | 347 µs | 350 µs | **345 µs** | 345 µs | — | 345 µs |
| 32 KiB | 1.06 ms | 15.5 ms | 1.24 ms | 1.06 ms | 1.06 ms | **1.05 ms** | 1.06 ms | — | 1.06 ms |
| 64 KiB | 2.78 ms | 62.1 ms | 3.58 ms | 2.78 ms | 6.62 ms | 2.94 ms | **2.78 ms** | — | 2.78 ms |
| 128 KiB | 5.98 ms | — | 7.53 ms | 5.98 ms | 26.5 ms | 6.3 ms | 5.98 ms | — | **5.96 ms** |
| 256 KiB | 13 ms | — | 16 ms | 13 ms | 72.8 ms | 13.7 ms | **13 ms** | — | 13.1 ms |
| 512 KiB | **29 ms** | — | 34.8 ms | 29.1 ms | 179 ms | 32.3 ms | 29.1 ms | — | 29.1 ms |
| 1 MiB | **69.8 ms** | — | 76.1 ms | 70.4 ms | 426 ms | 70 ms | 70.7 ms | — | 69.8 ms |
| 2 MiB | **144 ms** | — | 162 ms | 144 ms | 963 ms | 144 ms | 144 ms | — | 144 ms |
| 4 MiB | 326 ms | — | 353 ms | **325 ms** | 2.18 s | 326 ms | 327 ms | — | 327 ms |

## Decode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **3.39 ns** | 114 ns | 129 ns | 126 ns | 126 ns | 143 ns | 142 ns | — | 114 ns |
| 1 B | **3.4 ns** | 7.77 ns | 18.5 ns | 18.7 ns | 11.6 ns | 19 ns | 18.7 ns | 26 ns | 8.27 ns |
| 8 B | 15.1 ns | **13.6 ns** | 21.6 ns | 21.7 ns | 16.2 ns | 21.7 ns | 21.8 ns | 28.2 ns | 14.2 ns |
| 16 B | 17.9 ns | **17.2 ns** | 24.8 ns | 24.7 ns | 21.5 ns | 25.3 ns | 25.3 ns | 32.3 ns | 17.5 ns |
| 21 B | 21.1 ns | **20.6 ns** | 30.4 ns | 30.2 ns | 24.8 ns | 30.8 ns | 30.7 ns | 35.4 ns | 20.8 ns |
| 22 B | 21.8 ns | **21 ns** | 30.2 ns | 30.5 ns | 25.5 ns | 30.8 ns | 30.8 ns | 36.1 ns | 21.5 ns |
| 23 B | 23.5 ns | **22.9 ns** | 36.3 ns | 32.9 ns | 26.6 ns | 33.2 ns | 33.2 ns | 35.8 ns | 23.2 ns |
| 31 B | 27.5 ns | **26.9 ns** | 39.4 ns | 39.2 ns | 30.9 ns | 38.2 ns | 38.4 ns | 50.5 ns | 27.1 ns |
| 32 B | 27.2 ns | **26.1 ns** | 36.9 ns | 37.3 ns | 28.8 ns | 37.4 ns | 37.6 ns | 50.2 ns | 26.6 ns |
| 33 B | 28.4 ns | **27.3 ns** | 38.6 ns | 38.6 ns | 30.5 ns | 39.5 ns | 39 ns | 52.1 ns | 27.6 ns |
| 63 B | 50.2 ns | **49.7 ns** | 61 ns | 60.5 ns | 52.7 ns | 60.7 ns | 61.1 ns | 76.7 ns | 50.1 ns |
| 64 B | 48.2 ns | **47.9 ns** | 64.5 ns | 58.9 ns | 50.4 ns | 59.2 ns | 59.2 ns | 77.3 ns | 48.1 ns |
| 65 B | 48.8 ns | **48.5 ns** | 59.1 ns | 59.1 ns | 51.2 ns | 60 ns | 59.7 ns | 79.3 ns | 48.5 ns |
| 128 B | 110 ns | **110 ns** | 120 ns | 121 ns | 113 ns | 121 ns | 121 ns | 155 ns | 110 ns |
| 256 B | **346 ns** | 346 ns | 359 ns | 353 ns | 356 ns | 358 ns | 355 ns | 360 ns | 353 ns |
| 512 B | 1.23 µs | 1.23 µs | 1.24 µs | 1.24 µs | 1.24 µs | 1.24 µs | 1.24 µs | **953 ns** | 1.89 µs |
| 1 KiB | 2.92 µs | 4.81 µs | 4.82 µs | 4.82 µs | 4.82 µs | 4.83 µs | 4.82 µs | **2.91 µs** | 6.43 µs |
| 2 KiB | **10.1 µs** | 18.6 µs | 18.7 µs | 18.7 µs | 18.6 µs | 18.7 µs | 18.6 µs | 10.1 µs | 23.9 µs |
| 4 KiB | 33.3 µs | 72.4 µs | **33.3 µs** | 33.3 µs | 33.4 µs | 33.3 µs | 33.3 µs | 37.4 µs | 90 µs |
| 8 KiB | 75.9 µs | 284 µs | 75.8 µs | 75.8 µs | **75.6 µs** | 76.4 µs | 76.4 µs | — | 75.7 µs |
| 16 KiB | 193 µs | 1.12 ms | 192 µs | **191 µs** | 192 µs | 193 µs | 194 µs | — | 192 µs |
| 32 KiB | 521 µs | 4.47 ms | 519 µs | 520 µs | 521 µs | 519 µs | 520 µs | — | **516 µs** |
| 64 KiB | 1.29 ms | 17.8 ms | 1.37 ms | 1.29 ms | 3.06 ms | 1.36 ms | **1.28 ms** | — | 1.46 ms |
| 128 KiB | **2.89 ms** | — | 3.31 ms | 2.91 ms | 10.8 ms | 3.31 ms | 2.9 ms | — | 4.23 ms |
| 256 KiB | **6.44 ms** | — | 7.36 ms | 6.49 ms | 29.9 ms | 7.38 ms | 6.45 ms | — | 12.5 ms |
| 512 KiB | **14.3 ms** | — | 16.2 ms | 14.3 ms | 75.5 ms | 16.1 ms | 14.3 ms | — | 37 ms |
| 1 MiB | 31.7 ms | — | 35.6 ms | **31.6 ms** | 183 ms | 35.5 ms | 31.8 ms | — | 110 ms |
| 2 MiB | 77.3 ms | — | 77.6 ms | 77.5 ms | 437 ms | **77.1 ms** | 77.5 ms | — | 330 ms |
| 4 MiB | **163 ms** | — | 167 ms | 166 ms | 1.01 s | 166 ms | 169 ms | — | 989 ms |

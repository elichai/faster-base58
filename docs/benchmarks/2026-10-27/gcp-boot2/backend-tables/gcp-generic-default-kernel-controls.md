# gcp-generic / default / kernel-controls

Frozen `4e5ac883…` backend comparison on AMD EPYC 9B45 / c4d-highcpu-4. Each CPU/cache cohort uses one immutable all-backend `__internals` worker. Generic uses default target flags; native uses actual `target-cpu=native`. These are separate from final-source304dc ordinary compact comparisons.

Time is the median of nine warm batch observations from three fresh processes × three batches. Time uses process CPU. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 3.15 ns | **2.67 ns** | 12.2 ns | 12 ns | 7.31 ns | 13.5 ns | 11.8 ns | — | 4.61 ns |
| 1 B | 4.12 ns | **3.16 ns** | 23.5 ns | 23.5 ns | 21.1 ns | 23.3 ns | 23.3 ns | 23.1 µs | 18 ns |
| 8 B | 28.9 ns | **28.1 ns** | 33.5 ns | 33.2 ns | 31.6 ns | 33.6 ns | 33.1 ns | 23.1 µs | 28.8 ns |
| 16 B | 40.5 ns | **39.5 ns** | 44.4 ns | 44.3 ns | 42.9 ns | 44.4 ns | 44.4 ns | 23.1 µs | 40 ns |
| 21 B | 41.9 ns | **40.6 ns** | 45.8 ns | 46.2 ns | 44.1 ns | 46.6 ns | 46.1 ns | 23.2 µs | 40.7 ns |
| 22 B | 47.6 ns | **46.3 ns** | 51.4 ns | 51.2 ns | 49.9 ns | 51.1 ns | 51.2 ns | 23.2 µs | 23.2 µs |
| 23 B | 54.4 ns | **53.1 ns** | 58.2 ns | 58.1 ns | 56.8 ns | 58.2 ns | 58.3 ns | 23.2 µs | 23.2 µs |
| 31 B | 63.3 ns | **62.3 ns** | 67.6 ns | 67.4 ns | 65.9 ns | 67.2 ns | 67.4 ns | 22.9 µs | 23 µs |
| 32 B | **31.7 ns** | 62.8 ns | 67.8 ns | 68 ns | 66.1 ns | 67.6 ns | 67.7 ns | 22.9 µs | 23 µs |
| 33 B | 63.7 ns | **62.9 ns** | 68.5 ns | 69.1 ns | 66.4 ns | 68.4 ns | 68.7 ns | 23.1 µs | 23.1 µs |
| 63 B | 136 ns | **135 ns** | 141 ns | 141 ns | 139 ns | 141 ns | 140 ns | 23.2 µs | 23.2 µs |
| 64 B | **32.8 ns** | 137 ns | 141 ns | 142 ns | 140 ns | 141 ns | 142 ns | 23.1 µs | 23.2 µs |
| 65 B | 140 ns | **139 ns** | 145 ns | 144 ns | 143 ns | 145 ns | 145 ns | 23.2 µs | 23.2 µs |
| 128 B | **375 ns** | 376 ns | 389 ns | 380 ns | 378 ns | 379 ns | 382 ns | 23 µs | 23.2 µs |
| 256 B | **1.19 µs** | 1.19 µs | 1.21 µs | 1.2 µs | 1.2 µs | 1.2 µs | 1.2 µs | 23.5 µs | 23.8 µs |
| 512 B | **4.26 µs** | 4.27 µs | 4.29 µs | 4.28 µs | 4.26 µs | 4.26 µs | 4.26 µs | 24.2 µs | 25.3 µs |
| 1 KiB | **8.22 µs** | 16.4 µs | 8.46 µs | 8.49 µs | 8.46 µs | 8.45 µs | 8.49 µs | 95.3 µs | 99.5 µs |
| 2 KiB | **19.5 µs** | 62.9 µs | 19.9 µs | 20 µs | 19.8 µs | 19.9 µs | 19.8 µs | 377 µs | 394 µs |
| 4 KiB | **51 µs** | 247 µs | 52 µs | 52.1 µs | 51.9 µs | 52.1 µs | 51.9 µs | 1.5 ms | 52.1 µs |
| 8 KiB | 143 µs | 979 µs | 143 µs | **143 µs** | 143 µs | 143 µs | 143 µs | 6.02 ms | 143 µs |
| 16 KiB | 420 µs | 3.9 ms | 422 µs | 420 µs | **419 µs** | 419 µs | 419 µs | — | 419 µs |
| 32 KiB | 1.24 ms | 15.5 ms | 1.24 ms | 1.24 ms | 1.24 ms | 1.24 ms | **1.24 ms** | — | 1.24 ms |
| 64 KiB | **3.56 ms** | 62 ms | 3.57 ms | 3.57 ms | 7.43 ms | 3.57 ms | 3.57 ms | — | 3.58 ms |
| 128 KiB | 7.53 ms | — | 7.54 ms | 7.54 ms | 28.2 ms | 7.55 ms | 7.53 ms | — | **7.53 ms** |
| 256 KiB | 16 ms | — | 16.1 ms | 16 ms | 76.1 ms | 16.1 ms | **16 ms** | — | 16.1 ms |
| 512 KiB | **34.3 ms** | — | 34.5 ms | 34.3 ms | 186 ms | 34.4 ms | 34.5 ms | — | 34.6 ms |
| 1 MiB | **75.1 ms** | — | 75.9 ms | 75.5 ms | 433 ms | 75.9 ms | 75.7 ms | — | 75.7 ms |
| 2 MiB | **160 ms** | — | 162 ms | 160 ms | 980 ms | 160 ms | 161 ms | — | 161 ms |
| 4 MiB | 351 ms | — | 351 ms | 352 ms | 2.2 s | 352 ms | 352 ms | — | **348 ms** |

## Decode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **3.88 ns** | 116 ns | 127 ns | 125 ns | 121 ns | 140 ns | 141 ns | — | 115 ns |
| 1 B | **4.12 ns** | 8.24 ns | 16.2 ns | 16.6 ns | 11.2 ns | 16.5 ns | 16.5 ns | 11.8 µs | 8.5 ns |
| 8 B | 12.5 ns | **12.4 ns** | 19.2 ns | 19.2 ns | 15.8 ns | 19.5 ns | 19.5 ns | 11.8 µs | 12.9 ns |
| 16 B | 16.8 ns | **16.7 ns** | 23 ns | 22.5 ns | 20.3 ns | 22.8 ns | 23.2 ns | 11.9 µs | 16.9 ns |
| 21 B | 21.7 ns | **20.9 ns** | 28.2 ns | 27.6 ns | 23.7 ns | 27.7 ns | 28.6 ns | 11.8 µs | 21.4 ns |
| 22 B | 21.1 ns | **21.1 ns** | 28 ns | 27.5 ns | 25.2 ns | 28.1 ns | 28.6 ns | 11.9 µs | 21.5 ns |
| 23 B | 23.4 ns | **23.1 ns** | 30 ns | 34.9 ns | 26.6 ns | 30.1 ns | 30.4 ns | 11.9 µs | 23.2 ns |
| 31 B | **26.9 ns** | 27.2 ns | 35.6 ns | 36.2 ns | 30.8 ns | 36 ns | 35.8 ns | 11.8 µs | 27.5 ns |
| 32 B | 26.7 ns | **26.1 ns** | 35.3 ns | 35.4 ns | 28.6 ns | 35.5 ns | 36 ns | 11.9 µs | 26.6 ns |
| 33 B | **27.3 ns** | 28.4 ns | 36.8 ns | 36.2 ns | 30.4 ns | 36.6 ns | 36.6 ns | 11.9 µs | 27.7 ns |
| 63 B | **49.6 ns** | 49.8 ns | 58.9 ns | 58.8 ns | 52.3 ns | 59.1 ns | 59.3 ns | 11.9 µs | 52.9 ns |
| 64 B | 48.3 ns | **48.2 ns** | 56.9 ns | 57.1 ns | 50.2 ns | 57.2 ns | 57 ns | 11.9 µs | 48.7 ns |
| 65 B | 48.5 ns | **48.2 ns** | 57.6 ns | 57.5 ns | 52.6 ns | 57.7 ns | 57.4 ns | 11.9 µs | 48.5 ns |
| 128 B | 110 ns | **110 ns** | 119 ns | 119 ns | 112 ns | 119 ns | 119 ns | 11.9 µs | 110 ns |
| 256 B | 304 ns | **304 ns** | 313 ns | 313 ns | 307 ns | 314 ns | 313 ns | 12.3 µs | 304 ns |
| 512 B | 994 ns | **993 ns** | 1 µs | 1 µs | 1e+03 ns | 1 µs | 1.01 µs | 12.7 µs | 13.7 µs |
| 1 KiB | 3.72 µs | 3.73 µs | 3.73 µs | 3.73 µs | **3.72 µs** | 3.73 µs | 3.73 µs | 48.9 µs | 52.2 µs |
| 2 KiB | 14.2 µs | 14.2 µs | 14.2 µs | 14.2 µs | 14.2 µs | 14.2 µs | **14.2 µs** | 189 µs | 203 µs |
| 4 KiB | **33.1 µs** | 54.7 µs | 33.3 µs | 33.3 µs | 33.3 µs | 33.3 µs | 33.3 µs | 740 µs | 796 µs |
| 8 KiB | 75.5 µs | 213 µs | 75.5 µs | **75.4 µs** | 75.5 µs | 75.5 µs | 75.5 µs | — | 75.4 µs |
| 16 KiB | 191 µs | 845 µs | 191 µs | 192 µs | 192 µs | 192 µs | 192 µs | — | **191 µs** |
| 32 KiB | 519 µs | 3.36 ms | 517 µs | 518 µs | 516 µs | 517 µs | 517 µs | — | **515 µs** |
| 64 KiB | **1.36 ms** | 13.4 ms | 1.36 ms | 1.36 ms | 3.14 ms | 1.36 ms | 1.36 ms | — | 1.46 ms |
| 128 KiB | 3.31 ms | — | **3.3 ms** | 3.3 ms | 11.2 ms | 3.3 ms | 3.31 ms | — | 4.22 ms |
| 256 KiB | 7.25 ms | — | 7.34 ms | 7.32 ms | 30.7 ms | 7.32 ms | **7.25 ms** | — | 12.4 ms |
| 512 KiB | 15.9 ms | — | 16 ms | **15.8 ms** | 77.1 ms | 15.9 ms | 16 ms | — | 36.8 ms |
| 1 MiB | **34.7 ms** | — | 34.8 ms | 35.1 ms | 186 ms | 34.8 ms | 34.9 ms | — | 110 ms |
| 2 MiB | 76.9 ms | — | 76.7 ms | **76.3 ms** | 437 ms | 76.9 ms | 77.6 ms | — | 328 ms |
| 4 MiB | 165 ms | — | **162 ms** | 168 ms | 1.01 s | 167 ms | 167 ms | — | 983 ms |

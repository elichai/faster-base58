# gcp-native / cache / kernel-controls

Frozen `4e5ac883…` backend comparison on AMD EPYC 9B45 / c4d-highcpu-4. Each CPU/cache cohort uses one immutable all-backend `__internals` worker. Generic uses default target flags; native uses actual `target-cpu=native`. These are separate from final-source304dc ordinary compact comparisons.

Time is the median of nine warm batch observations from three fresh processes × three batches. Time uses process CPU. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 2.91 ns | **2.67 ns** | 7.48 ns | 7.46 ns | 7.46 ns | 7.47 ns | 7.46 ns | — | 4.36 ns |
| 1 B | **3.15 ns** | 3.16 ns | 21.4 ns | 21.4 ns | 21.4 ns | 21.3 ns | 21.3 ns | 27.8 ns | 17.2 ns |
| 8 B | 28 ns | 27.9 ns | 31.9 ns | 31.7 ns | 31.8 ns | 31.8 ns | 31.9 ns | 33.7 ns | **27.8 ns** |
| 16 B | 39.6 ns | 39.7 ns | 43.5 ns | 43.4 ns | 43.4 ns | 43.4 ns | 43.4 ns | 41.2 ns | **39.4 ns** |
| 21 B | 41.4 ns | 40.8 ns | 44.6 ns | 44.6 ns | 45.1 ns | 44.5 ns | 44.7 ns | 45.6 ns | **40.5 ns** |
| 22 B | 46.1 ns | 47 ns | 51.1 ns | 50.4 ns | 50.9 ns | 50.7 ns | 50.6 ns | **45.4 ns** | 45.7 ns |
| 23 B | 45.3 ns | 53.2 ns | 57.2 ns | 57.2 ns | 57.2 ns | 57.3 ns | 57.1 ns | **45 ns** | 45.6 ns |
| 31 B | 52.1 ns | 62.1 ns | 66.3 ns | 66 ns | 66.7 ns | 66.4 ns | 66 ns | **51.7 ns** | 52 ns |
| 32 B | **35.1 ns** | 61.7 ns | 66 ns | 65.9 ns | 65.8 ns | 65.9 ns | 66.3 ns | 50.6 ns | 50.9 ns |
| 33 B | 62.8 ns | **62.4 ns** | 66 ns | 66.1 ns | 66.1 ns | 66.2 ns | 66.4 ns | 62.6 ns | 63.4 ns |
| 63 B | 86.4 ns | 132 ns | 137 ns | 138 ns | 136 ns | 137 ns | 138 ns | **86.3 ns** | 87.2 ns |
| 64 B | **32.4 ns** | 133 ns | 139 ns | 138 ns | 138 ns | 139 ns | 139 ns | 82.9 ns | 84.1 ns |
| 65 B | 99.1 ns | 137 ns | 142 ns | 142 ns | 141 ns | 142 ns | 142 ns | **99.1 ns** | 153 ns |
| 128 B | 162 ns | 368 ns | 370 ns | 370 ns | 371 ns | 370 ns | 369 ns | **162 ns** | 261 ns |
| 256 B | 380 ns | 1.15 µs | 1.16 µs | 1.16 µs | 1.16 µs | 1.16 µs | 1.15 µs | **379 ns** | 787 ns |
| 512 B | 1.05 µs | 4.07 µs | 4.07 µs | 4.08 µs | 4.07 µs | 4.07 µs | 4.07 µs | **1.05 µs** | 2.93 µs |
| 1 KiB | **3.26 µs** | 15.1 µs | 8.17 µs | 5.82 µs | 5.82 µs | 5.82 µs | 5.82 µs | 3.26 µs | 11 µs |
| 2 KiB | **11.6 µs** | 58.8 µs | 19.3 µs | 14.3 µs | 14.3 µs | 14.3 µs | 14.3 µs | 11.7 µs | 43.4 µs |
| 4 KiB | 38.4 µs | 230 µs | 50.1 µs | 38.5 µs | 38.4 µs | 38.4 µs | **38.3 µs** | 42.8 µs | 38.3 µs |
| 8 KiB | **110 µs** | 903 µs | 139 µs | 111 µs | 111 µs | 110 µs | 110 µs | 164 µs | 110 µs |
| 16 KiB | 339 µs | 3.58 ms | 414 µs | 338 µs | **337 µs** | 339 µs | 338 µs | — | 338 µs |
| 32 KiB | 1.05 ms | 14.4 ms | 1.25 ms | 1.07 ms | **1.05 ms** | 1.07 ms | 1.05 ms | — | 1.05 ms |
| 64 KiB | 2.77 ms | 57.7 ms | 3.54 ms | **2.77 ms** | 3.16 ms | 2.89 ms | 2.77 ms | — | 2.77 ms |
| 128 KiB | **5.86 ms** | — | 7.34 ms | 5.87 ms | 7.96 ms | 6.16 ms | 5.9 ms | — | 5.88 ms |
| 256 KiB | 12.7 ms | — | 15.5 ms | **12.6 ms** | 18.5 ms | 13.3 ms | 12.6 ms | — | 12.6 ms |
| 512 KiB | 28 ms | — | 33.4 ms | **28 ms** | 42.3 ms | 30.5 ms | 28.1 ms | — | 28.3 ms |
| 1 MiB | 66.8 ms | — | 72.5 ms | 66.7 ms | 99.9 ms | **66.6 ms** | 66.6 ms | — | 67.2 ms |
| 2 MiB | **138 ms** | — | 155 ms | 138 ms | 213 ms | 138 ms | 138 ms | — | 139 ms |
| 4 MiB | **313 ms** | — | 339 ms | 314 ms | 478 ms | 314 ms | 315 ms | — | 314 ms |

## Decode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **3.63 ns** | 116 ns | 129 ns | 127 ns | 129 ns | 127 ns | 129 ns | — | 115 ns |
| 1 B | **3.63 ns** | 7.88 ns | 11.4 ns | 11.6 ns | 11.5 ns | 11.9 ns | 11.9 ns | 26 ns | 8.07 ns |
| 8 B | 13.9 ns | **13.7 ns** | 18.3 ns | 18.3 ns | 19.1 ns | 18.5 ns | 18.5 ns | 30.7 ns | 14.1 ns |
| 16 B | **20.3 ns** | 21.7 ns | 25.5 ns | 23.7 ns | 23.7 ns | 24.3 ns | 24 ns | 34.9 ns | 20.5 ns |
| 21 B | **22.8 ns** | 23.1 ns | 26 ns | 25.9 ns | 25.9 ns | 26.2 ns | 26.1 ns | 38.7 ns | 24.2 ns |
| 22 B | 27.2 ns | **25.3 ns** | 27.5 ns | 27.6 ns | 27.3 ns | 27.4 ns | 27.5 ns | 38.9 ns | 26.3 ns |
| 23 B | 27.7 ns | **26.3 ns** | 30.5 ns | 31.1 ns | 31.9 ns | 32.8 ns | 30.3 ns | 39.4 ns | 26.6 ns |
| 31 B | **41.3 ns** | 42 ns | 45.3 ns | 45.1 ns | 45.5 ns | 44.1 ns | 44.1 ns | 55.6 ns | 41.5 ns |
| 32 B | **25.9 ns** | 26 ns | 28.7 ns | 28.6 ns | 28.6 ns | 28.8 ns | 29.1 ns | 55.3 ns | 26.4 ns |
| 33 B | **26.5 ns** | 26.9 ns | 29.4 ns | 31.3 ns | 29.6 ns | 30.6 ns | 29.5 ns | 59 ns | 26.9 ns |
| 63 B | **51.3 ns** | 52.5 ns | 54.8 ns | 57.4 ns | 55.7 ns | 55 ns | 56.8 ns | 85.4 ns | 51.7 ns |
| 64 B | 47.4 ns | **46.7 ns** | 49.7 ns | 51.1 ns | 50.1 ns | 49.8 ns | 50.5 ns | 86 ns | 47.7 ns |
| 65 B | **47.6 ns** | 47.7 ns | 51.5 ns | 50.9 ns | 51.5 ns | 51.5 ns | 51.7 ns | 88.6 ns | 47.9 ns |
| 128 B | **104 ns** | 105 ns | 107 ns | 107 ns | 109 ns | 107 ns | 108 ns | 189 ns | 106 ns |
| 256 B | **266 ns** | 267 ns | 270 ns | 277 ns | 269 ns | 271 ns | 270 ns | 395 ns | 267 ns |
| 512 B | 819 ns | **818 ns** | 822 ns | 822 ns | 831 ns | 820 ns | 821 ns | 1.02 µs | 2.47 µs |
| 1 KiB | 2.74 µs | **2.74 µs** | 2.75 µs | 2.76 µs | 2.74 µs | 2.74 µs | 2.74 µs | 3.01 µs | 9.32 µs |
| 2 KiB | **10 µs** | 10 µs | 10 µs | 10.1 µs | 10 µs | 10 µs | 10 µs | 10.1 µs | 36.7 µs |
| 4 KiB | 22.2 µs | 38.9 µs | 22.1 µs | **22.1 µs** | 22.2 µs | 22.1 µs | 22.1 µs | 37.5 µs | 153 µs |
| 8 KiB | 56.4 µs | 152 µs | 56.4 µs | 56.1 µs | 56.4 µs | 56.6 µs | 56.4 µs | — | **56 µs** |
| 16 KiB | 157 µs | 592 µs | 157 µs | 157 µs | 157 µs | 157 µs | 157 µs | — | **156 µs** |
| 32 KiB | 466 µs | 2.33 ms | 460 µs | 466 µs | 460 µs | 460 µs | 461 µs | — | **456 µs** |
| 64 KiB | **1.17 ms** | 9.28 ms | 1.23 ms | 1.17 ms | 1.37 ms | 1.23 ms | 1.17 ms | — | 1.38 ms |
| 128 KiB | 2.65 ms | — | 3.02 ms | **2.63 ms** | 3.44 ms | 3.02 ms | 2.64 ms | — | 4.13 ms |
| 256 KiB | **5.89 ms** | — | 6.67 ms | 5.91 ms | 8.22 ms | 6.71 ms | 5.93 ms | — | 12.6 ms |
| 512 KiB | **13.1 ms** | — | 14.7 ms | 13.1 ms | 18.9 ms | 14.7 ms | 13.1 ms | — | 37.6 ms |
| 1 MiB | 29.2 ms | — | 32.2 ms | 29.3 ms | 43.3 ms | 32.2 ms | **29.2 ms** | — | 114 ms |
| 2 MiB | 71.3 ms | — | 71.3 ms | **71.1 ms** | 104 ms | 71.5 ms | 71.3 ms | — | 345 ms |
| 4 MiB | 154 ms | — | **154 ms** | 154 ms | 229 ms | 155 ms | 155 ms | — | 1.04 s |

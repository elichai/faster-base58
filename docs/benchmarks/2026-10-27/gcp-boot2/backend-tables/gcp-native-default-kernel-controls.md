# gcp-native / default / kernel-controls

Frozen `4e5ac883…` backend comparison on AMD EPYC 9B45 / c4d-highcpu-4. Each CPU/cache cohort uses one immutable all-backend `__internals` worker. Generic uses default target flags; native uses actual `target-cpu=native`. These are separate from final-source304dc ordinary compact comparisons.

Time is the median of nine warm batch observations from three fresh processes × three batches. Time uses process CPU. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 2.78 ns | **2.43 ns** | 6.19 ns | 6.46 ns | 6.9 ns | 6.31 ns | 6.41 ns | — | 4.12 ns |
| 1 B | 3.41 ns | **2.91 ns** | 19.8 ns | 20.5 ns | 20.2 ns | 19.8 ns | 20.5 ns | 23.1 µs | 16.8 ns |
| 8 B | 28.6 ns | 27.8 ns | 30.5 ns | 31.4 ns | 31.2 ns | 30.7 ns | 30.6 ns | 23.1 µs | **27.7 ns** |
| 16 B | 40 ns | **39.3 ns** | 42.1 ns | 41.9 ns | 42.3 ns | 42.1 ns | 42 ns | 23.2 µs | 39.4 ns |
| 21 B | 41 ns | 40.2 ns | 42.9 ns | 42.8 ns | 43 ns | 42.8 ns | 43.1 ns | 23 µs | **39.9 ns** |
| 22 B | 47 ns | **46.4 ns** | 49.1 ns | 49.1 ns | 49.4 ns | 49.1 ns | 49.3 ns | 23 µs | 22.9 µs |
| 23 B | 53.7 ns | **52.8 ns** | 55.7 ns | 56.3 ns | 56.4 ns | 55.8 ns | 55.9 ns | 23 µs | 23 µs |
| 31 B | 62.5 ns | **62 ns** | 64.8 ns | 64.8 ns | 64.9 ns | 64.9 ns | 64.8 ns | 22.8 µs | 22.8 µs |
| 32 B | **35.3 ns** | 61.8 ns | 64.6 ns | 64.6 ns | 64.6 ns | 64.5 ns | 64.6 ns | 22.8 µs | 22.8 µs |
| 33 B | 62.7 ns | **62.2 ns** | 64.9 ns | 64.8 ns | 65.1 ns | 65 ns | 64.8 ns | 23.1 µs | 23.1 µs |
| 63 B | 132 ns | **132 ns** | 135 ns | 135 ns | 135 ns | 135 ns | 135 ns | 23 µs | 22.9 µs |
| 64 B | **32.8 ns** | 133 ns | 136 ns | 137 ns | 137 ns | 137 ns | 137 ns | 22.9 µs | 23 µs |
| 65 B | 138 ns | **137 ns** | 140 ns | 140 ns | 141 ns | 141 ns | 141 ns | 23.2 µs | 23.2 µs |
| 128 B | 376 ns | **366 ns** | 370 ns | 371 ns | 374 ns | 370 ns | 378 ns | 23 µs | 23.1 µs |
| 256 B | **1.15 µs** | 1.16 µs | 1.16 µs | 1.16 µs | 1.16 µs | 1.16 µs | 1.16 µs | 23.5 µs | 23.8 µs |
| 512 B | 4.05 µs | 4.05 µs | **4.04 µs** | 4.05 µs | 4.06 µs | 4.05 µs | 4.06 µs | 24.1 µs | 25.9 µs |
| 1 KiB | **7.93 µs** | 15.1 µs | 8.24 µs | 8.26 µs | 8.28 µs | 8.27 µs | 8.23 µs | 94.2 µs | 102 µs |
| 2 KiB | **18.7 µs** | 59.1 µs | 19.4 µs | 19.4 µs | 19.3 µs | 19.3 µs | 19.3 µs | 374 µs | 406 µs |
| 4 KiB | **48.8 µs** | 231 µs | 50.2 µs | 50.4 µs | 50.2 µs | 50.4 µs | 50.2 µs | 1.5 ms | 50.3 µs |
| 8 KiB | 140 µs | 907 µs | **139 µs** | 140 µs | 140 µs | 140 µs | 140 µs | 6 ms | 140 µs |
| 16 KiB | 417 µs | 3.6 ms | **416 µs** | 417 µs | 417 µs | 417 µs | 417 µs | — | 417 µs |
| 32 KiB | 1.25 ms | 14.4 ms | 1.25 ms | 1.25 ms | 1.25 ms | 1.25 ms | 1.25 ms | — | **1.25 ms** |
| 64 KiB | 3.56 ms | 57.7 ms | **3.54 ms** | 3.54 ms | 3.95 ms | 3.54 ms | 3.54 ms | — | 3.55 ms |
| 128 KiB | 7.37 ms | — | 7.36 ms | 7.36 ms | 9.41 ms | **7.35 ms** | 7.36 ms | — | 7.37 ms |
| 256 KiB | 15.6 ms | — | 15.5 ms | 15.5 ms | 21.5 ms | **15.5 ms** | 15.6 ms | — | 15.6 ms |
| 512 KiB | 33.6 ms | — | 33.3 ms | 33.4 ms | 47.6 ms | 33.4 ms | **33.2 ms** | — | 33.3 ms |
| 1 MiB | 73.3 ms | — | 72.6 ms | **72.6 ms** | 106 ms | 72.6 ms | 72.9 ms | — | 72.9 ms |
| 2 MiB | **155 ms** | — | 155 ms | 155 ms | 230 ms | 155 ms | 155 ms | — | 156 ms |
| 4 MiB | 340 ms | — | 339 ms | 341 ms | 503 ms | 342 ms | **339 ms** | — | 341 ms |

## Decode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **3.63 ns** | 116 ns | 122 ns | 118 ns | 119 ns | 125 ns | 128 ns | — | 116 ns |
| 1 B | **3.64 ns** | 7.64 ns | 10.3 ns | 10.4 ns | 10.7 ns | 10.7 ns | 10.6 ns | 11.8 µs | 8.13 ns |
| 8 B | **13.8 ns** | 14.6 ns | 17.4 ns | 17.4 ns | 17.5 ns | 17.5 ns | 17.6 ns | 11.8 µs | 14.1 ns |
| 16 B | 20.3 ns | 20.8 ns | 22.8 ns | 23.9 ns | 23.1 ns | 22.9 ns | 22.9 ns | 11.8 µs | **20.3 ns** |
| 21 B | **22.6 ns** | 22.7 ns | 25.6 ns | 24.9 ns | 25.2 ns | 25 ns | 25 ns | 12 µs | 22.6 ns |
| 22 B | 25.2 ns | **24.7 ns** | 26.5 ns | 26.7 ns | 27.4 ns | 26.8 ns | 26.9 ns | 11.9 µs | 25.3 ns |
| 23 B | 26.5 ns | **25.7 ns** | 30.4 ns | 30 ns | 29.4 ns | 29.1 ns | 29.1 ns | 11.9 µs | 26.4 ns |
| 31 B | **40 ns** | 41.7 ns | 44.1 ns | 42 ns | 42.9 ns | 42.6 ns | 43.6 ns | 11.9 µs | 40.3 ns |
| 32 B | 27.1 ns | 26 ns | 29.2 ns | 27.5 ns | 30.2 ns | 27.7 ns | 27.7 ns | 11.9 µs | **26 ns** |
| 33 B | **26.7 ns** | 27.5 ns | 28.7 ns | 28.3 ns | 28.9 ns | 28.8 ns | 29.1 ns | 11.9 µs | 27.2 ns |
| 63 B | 51.6 ns | **51.6 ns** | 54.2 ns | 53.5 ns | 54.5 ns | 54 ns | 54.4 ns | 11.9 µs | 53.3 ns |
| 64 B | **46.7 ns** | 47.1 ns | 52.8 ns | 48.8 ns | 49.3 ns | 48.9 ns | 52.5 ns | 11.9 µs | 46.9 ns |
| 65 B | 47.9 ns | **47.5 ns** | 49.7 ns | 50.8 ns | 50.7 ns | 50.2 ns | 52.7 ns | 12 µs | 47.6 ns |
| 128 B | **105 ns** | 105 ns | 106 ns | 109 ns | 106 ns | 107 ns | 107 ns | 12 µs | 106 ns |
| 256 B | **266 ns** | 273 ns | 270 ns | 275 ns | 270 ns | 270 ns | 276 ns | 12.4 µs | 267 ns |
| 512 B | 815 ns | **812 ns** | 816 ns | 822 ns | 816 ns | 820 ns | 814 ns | 12.8 µs | 14.2 µs |
| 1 KiB | 2.74 µs | **2.74 µs** | 2.74 µs | 2.74 µs | 2.74 µs | 2.74 µs | 2.77 µs | 49 µs | 55.2 µs |
| 2 KiB | 10 µs | 10 µs | 10 µs | 10 µs | **10 µs** | 10 µs | 10 µs | 189 µs | 216 µs |
| 4 KiB | **21.9 µs** | 38.8 µs | 22.1 µs | 22.1 µs | 22.1 µs | 22.1 µs | 22.1 µs | 740 µs | 859 µs |
| 8 KiB | 56.2 µs | 151 µs | 56.1 µs | 56.3 µs | 56 µs | 56 µs | 56 µs | — | **55.8 µs** |
| 16 KiB | 156 µs | 592 µs | 156 µs | 156 µs | 156 µs | 157 µs | 156 µs | — | **156 µs** |
| 32 KiB | 458 µs | 2.33 ms | 457 µs | 458 µs | 459 µs | 458 µs | 457 µs | — | **455 µs** |
| 64 KiB | 1.24 ms | 9.29 ms | 1.23 ms | **1.23 ms** | 1.42 ms | 1.23 ms | 1.23 ms | — | 1.36 ms |
| 128 KiB | 3.01 ms | — | 3.01 ms | **3 ms** | 3.8 ms | 3 ms | 3.01 ms | — | 4.12 ms |
| 256 KiB | 6.69 ms | — | 6.7 ms | 6.69 ms | 8.94 ms | **6.68 ms** | 6.68 ms | — | 12.4 ms |
| 512 KiB | 14.6 ms | — | **14.5 ms** | 14.6 ms | 20.5 ms | 14.6 ms | 14.6 ms | — | 37.8 ms |
| 1 MiB | 32.4 ms | — | 32.2 ms | 32.3 ms | 46.4 ms | **32.1 ms** | 32.3 ms | — | 114 ms |
| 2 MiB | 71 ms | — | 71.2 ms | 71.2 ms | 105 ms | 71.3 ms | **71 ms** | — | 342 ms |
| 4 MiB | 156 ms | — | **153 ms** | 155 ms | 230 ms | 154 ms | 154 ms | — | 1.03 s |

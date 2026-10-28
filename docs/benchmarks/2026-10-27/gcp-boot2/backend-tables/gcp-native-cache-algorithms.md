# gcp-native / cache / algorithms

Frozen `4e5ac883…` backend comparison on AMD EPYC 9B45 / c4d-highcpu-4. Each CPU/cache cohort uses one immutable all-backend `__internals` worker. Generic uses default target flags; native uses actual `target-cpu=native`. These are separate from final-source304dc ordinary compact comparisons.

Time is the median of nine warm batch observations from three fresh processes × three batches. Time uses process CPU. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 2.91 ns | 2.91 ns | 4.61 ns | 4.85 ns | 4.61 ns | 7.46 ns | 5.34 ns | **2.91 ns** | — |
| 1 B | 3.15 ns | **3.15 ns** | 17.8 ns | 17.9 ns | 17.2 ns | 21.4 ns | 18.5 ns | 5.55 ns | — |
| 8 B | 28 ns | 28 ns | 28.5 ns | 28.5 ns | **27.9 ns** | 31.7 ns | 29.1 ns | 70.9 ns | — |
| 16 B | 39.6 ns | 39.7 ns | 41.2 ns | 40.1 ns | **39.4 ns** | 43.4 ns | 40.8 ns | 263 ns | — |
| 21 B | 41.4 ns | **40.4 ns** | 41.4 ns | 41.4 ns | 40.4 ns | 44.6 ns | 41.7 ns | 391 ns | — |
| 22 B | 46.1 ns | 46.6 ns | 47.1 ns | 47 ns | **45.4 ns** | 50.4 ns | 47.7 ns | 419 ns | — |
| 23 B | **45.3 ns** | 53.3 ns | 53.7 ns | 53.7 ns | 45.4 ns | 57.2 ns | 54.2 ns | 447 ns | — |
| 31 B | 52.1 ns | 62.1 ns | 62.6 ns | 62.6 ns | **51.6 ns** | 66 ns | 63.3 ns | 730 ns | — |
| 32 B | 35.1 ns | **31.3 ns** | 62.7 ns | 62.9 ns | 50.7 ns | 65.9 ns | 63.1 ns | 770 ns | 32.3 ns |
| 33 B | 62.8 ns | **62.2 ns** | 62.8 ns | 62.8 ns | 62.8 ns | 66.1 ns | 63.4 ns | 815 ns | — |
| 63 B | **86.4 ns** | 132 ns | 133 ns | 132 ns | 86.5 ns | 138 ns | 133 ns | 2.83 µs | — |
| 64 B | **32.4 ns** | 32.5 ns | 134 ns | 134 ns | 83.3 ns | 138 ns | 135 ns | 2.96 µs | 57.2 ns |
| 65 B | **99.1 ns** | 137 ns | 138 ns | 138 ns | 99.7 ns | 142 ns | 138 ns | 3.07 µs | — |
| 128 B | 162 ns | 367 ns | 368 ns | 368 ns | **162 ns** | 370 ns | 379 ns | 12.8 µs | — |
| 256 B | 380 ns | 1.15 µs | 1.15 µs | 1.15 µs | **379 ns** | 1.16 µs | 1.15 µs | 54.2 µs | — |
| 512 B | 1.05 µs | 4.07 µs | 4.08 µs | 4.07 µs | **1.05 µs** | 4.08 µs | 4.07 µs | 225 µs | — |
| 1 KiB | 3.26 µs | 15.1 µs | 7.93 µs | 7.96 µs | **3.26 µs** | 5.82 µs | 15.2 µs | 921 µs | — |
| 2 KiB | 11.6 µs | 58.8 µs | 18.6 µs | 18.9 µs | **11.6 µs** | 14.3 µs | 53.5 µs | 3.72 ms | — |
| 4 KiB | 38.4 µs | 230 µs | 48.8 µs | 49.1 µs | **38.4 µs** | 38.5 µs | 159 µs | 14.9 ms | — |
| 8 KiB | **110 µs** | 902 µs | 138 µs | 137 µs | 110 µs | 111 µs | 405 µs | 59.5 ms | — |
| 16 KiB | 339 µs | 3.59 ms | 411 µs | 411 µs | 340 µs | **338 µs** | 927 µs | 238 ms | — |
| 32 KiB | 1.05 ms | 14.4 ms | 1.24 ms | 1.23 ms | **1.05 ms** | 1.07 ms | 2.06 ms | 954 ms | — |
| 64 KiB | 2.77 ms | 57.6 ms | 3.85 ms | 3.75 ms | **2.76 ms** | 2.77 ms | 4.51 ms | 3.82 s | — |
| 128 KiB | 5.86 ms | — | 11.7 ms | 10.9 ms | **5.86 ms** | 5.87 ms | 10.1 ms | — | — |
| 256 KiB | 12.7 ms | — | 35.3 ms | 30.4 ms | **12.6 ms** | 12.6 ms | 22.1 ms | — | — |
| 512 KiB | 28 ms | — | 106 ms | 85.4 ms | 28.2 ms | **28 ms** | 48.1 ms | — | — |
| 1 MiB | 66.8 ms | — | 320 ms | 246 ms | 67.6 ms | **66.7 ms** | 103 ms | — | — |
| 2 MiB | **138 ms** | — | 962 ms | 697 ms | 139 ms | 138 ms | 222 ms | — | — |
| 4 MiB | **313 ms** | — | 2.9 s | 1.96 s | 316 ms | 314 ms | 476 ms | — | — |

## Decode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 3.63 ns | 3.64 ns | 113 ns | 117 ns | 117 ns | 127 ns | 117 ns | **2.7 ns** | — |
| 1 B | **3.63 ns** | 3.64 ns | 7.95 ns | 8.79 ns | 8.08 ns | 11.6 ns | 9.13 ns | 4.63 ns | — |
| 8 B | 13.9 ns | **13.8 ns** | 14.2 ns | 15.2 ns | 14.2 ns | 18.3 ns | 15.8 ns | 39 ns | — |
| 16 B | 20.3 ns | **20.3 ns** | 20.3 ns | 21.9 ns | 20.4 ns | 23.7 ns | 22.4 ns | 101 ns | — |
| 21 B | **22.8 ns** | 22.9 ns | 23.3 ns | 23.7 ns | 23.1 ns | 25.9 ns | 24 ns | 156 ns | — |
| 22 B | 27.2 ns | **25.7 ns** | 26.1 ns | 26.3 ns | 25.7 ns | 27.6 ns | 26.8 ns | 168 ns | — |
| 23 B | 27.7 ns | 27.3 ns | 27.2 ns | **27 ns** | 27.3 ns | 31.1 ns | 27.7 ns | 182 ns | — |
| 31 B | 41.3 ns | 41.9 ns | 42 ns | **41.1 ns** | 42.1 ns | 45.1 ns | 43.7 ns | 307 ns | — |
| 32 B | 25.9 ns | **25.9 ns** | 25.9 ns | 26.4 ns | 26.9 ns | 28.6 ns | 27.1 ns | 316 ns | 42.9 ns |
| 33 B | 26.5 ns | **26.5 ns** | 26.5 ns | 27.2 ns | 27.3 ns | 31.3 ns | 27.9 ns | 336 ns | — |
| 63 B | **51.3 ns** | 51.4 ns | 52.5 ns | 52.2 ns | 51.9 ns | 57.4 ns | 52.8 ns | 1.2 µs | — |
| 64 B | 47.4 ns | **46.6 ns** | 47.4 ns | 47.4 ns | 46.9 ns | 51.1 ns | 48.6 ns | 1.25 µs | 82.3 ns |
| 65 B | **47.6 ns** | 47.7 ns | 48 ns | 48.6 ns | 47.8 ns | 50.9 ns | 49.6 ns | 1.29 µs | — |
| 128 B | **104 ns** | 105 ns | 105 ns | 105 ns | 105 ns | 107 ns | 105 ns | 5.28 µs | — |
| 256 B | **266 ns** | 266 ns | 266 ns | 267 ns | 266 ns | 277 ns | 269 ns | 21.9 µs | — |
| 512 B | 819 ns | **819 ns** | 820 ns | 825 ns | 1.03 µs | 822 ns | 820 ns | 90.6 µs | — |
| 1 KiB | 2.74 µs | **2.73 µs** | 2.74 µs | 2.74 µs | 3.01 µs | 2.76 µs | 2.74 µs | 373 µs | — |
| 2 KiB | **10 µs** | 10 µs | 10 µs | 10 µs | 10.2 µs | 10.1 µs | 10 µs | 1.49 ms | — |
| 4 KiB | 22.2 µs | 38.9 µs | 22 µs | **21.9 µs** | 37.9 µs | 22.1 µs | 50.4 µs | 5.95 ms | — |
| 8 KiB | 56.4 µs | 152 µs | 56.1 µs | 56.1 µs | **56 µs** | 56.1 µs | 128 µs | 23.7 ms | — |
| 16 KiB | 157 µs | 593 µs | 157 µs | **156 µs** | 156 µs | 157 µs | 303 µs | 94.7 ms | — |
| 32 KiB | 466 µs | 2.33 ms | **457 µs** | 464 µs | 459 µs | 466 µs | 689 µs | 378 ms | — |
| 64 KiB | **1.17 ms** | 9.28 ms | 1.37 ms | 1.38 ms | 1.37 ms | 1.17 ms | 1.6 ms | 1.51 s | — |
| 128 KiB | 2.65 ms | — | 4.14 ms | 4.06 ms | 4.14 ms | **2.63 ms** | 3.89 ms | — | — |
| 256 KiB | **5.89 ms** | — | 12.5 ms | 11.4 ms | 12.6 ms | 5.91 ms | 8.64 ms | — | — |
| 512 KiB | **13.1 ms** | — | 37.6 ms | 32.4 ms | 37.7 ms | 13.1 ms | 19.1 ms | — | — |
| 1 MiB | **29.2 ms** | — | 114 ms | 93.4 ms | 114 ms | 29.3 ms | 41.7 ms | — | — |
| 2 MiB | 71.3 ms | — | 345 ms | 266 ms | 346 ms | **71.1 ms** | 90.8 ms | — | — |
| 4 MiB | 154 ms | — | 1.04 s | 757 ms | 1.04 s | **154 ms** | 197 ms | — | — |

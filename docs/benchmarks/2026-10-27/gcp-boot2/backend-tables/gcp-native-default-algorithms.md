# gcp-native / default / algorithms

Frozen `4e5ac883…` backend comparison on AMD EPYC 9B45 / c4d-highcpu-4. Each CPU/cache cohort uses one immutable all-backend `__internals` worker. Generic uses default target flags; native uses actual `target-cpu=native`. These are separate from final-source304dc ordinary compact comparisons.

Time is the median of nine warm batch observations from three fresh processes × three batches. Time uses process CPU. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 2.78 ns | 2.67 ns | 4.25 ns | 4.22 ns | 4.12 ns | 6.46 ns | 5.34 ns | **2.42 ns** | — |
| 1 B | 3.41 ns | **2.91 ns** | 17.5 ns | 17.6 ns | 16.9 ns | 20.5 ns | 18.2 ns | 5.7 ns | — |
| 8 B | 28.6 ns | 27.7 ns | 28.7 ns | 28.8 ns | **27.5 ns** | 31.4 ns | 28.9 ns | 71.5 ns | — |
| 16 B | 40 ns | 39.5 ns | 39.8 ns | 40 ns | **39.2 ns** | 41.9 ns | 40.4 ns | 263 ns | — |
| 21 B | 41 ns | 40 ns | 40.9 ns | 41 ns | **39.9 ns** | 42.8 ns | 41.3 ns | 392 ns | — |
| 22 B | 47 ns | **46.3 ns** | 47.2 ns | 47.1 ns | 23 µs | 49.1 ns | 47.5 ns | 419 ns | — |
| 23 B | 53.7 ns | **53 ns** | 53.5 ns | 53.4 ns | 23 µs | 56.3 ns | 54.2 ns | 447 ns | — |
| 31 B | 62.5 ns | **62.5 ns** | 62.5 ns | 62.6 ns | 22.8 µs | 64.8 ns | 63.2 ns | 731 ns | — |
| 32 B | 35.3 ns | **31 ns** | 62.3 ns | 62.4 ns | 22.9 µs | 64.6 ns | 63 ns | 765 ns | 32.3 ns |
| 33 B | 62.7 ns | **62.1 ns** | 62.7 ns | 62.8 ns | 23.1 µs | 64.8 ns | 63.3 ns | 815 ns | — |
| 63 B | 132 ns | **132 ns** | 132 ns | 132 ns | 22.9 µs | 135 ns | 133 ns | 2.83 µs | — |
| 64 B | 32.8 ns | **32.4 ns** | 134 ns | 134 ns | 22.9 µs | 137 ns | 135 ns | 2.96 µs | 58.5 ns |
| 65 B | 138 ns | **137 ns** | 138 ns | 139 ns | 23.1 µs | 140 ns | 138 ns | 3.07 µs | — |
| 128 B | 376 ns | 376 ns | **368 ns** | 376 ns | 23 µs | 371 ns | 377 ns | 12.8 µs | — |
| 256 B | 1.15 µs | 1.16 µs | 1.16 µs | 1.16 µs | 23.4 µs | 1.16 µs | **1.15 µs** | 54.2 µs | — |
| 512 B | 4.05 µs | 4.05 µs | 4.05 µs | **4.05 µs** | 24 µs | 4.05 µs | 4.06 µs | 225 µs | — |
| 1 KiB | **7.93 µs** | 15.1 µs | 8 µs | 7.96 µs | 94.1 µs | 8.26 µs | 15.1 µs | 920 µs | — |
| 2 KiB | 18.7 µs | 59.1 µs | **18.7 µs** | 18.9 µs | 374 µs | 19.4 µs | 53.4 µs | 3.72 ms | — |
| 4 KiB | **48.8 µs** | 231 µs | 49.2 µs | 49 µs | 50.2 µs | 50.4 µs | 158 µs | 14.9 ms | — |
| 8 KiB | 140 µs | 904 µs | **138 µs** | 138 µs | 140 µs | 140 µs | 401 µs | 59.7 ms | — |
| 16 KiB | 417 µs | 3.6 ms | 412 µs | **412 µs** | 417 µs | 417 µs | 932 µs | 238 ms | — |
| 32 KiB | 1.25 ms | 14.4 ms | 1.24 ms | **1.23 ms** | 1.25 ms | 1.25 ms | 2.06 ms | 953 ms | — |
| 64 KiB | 3.56 ms | 57.8 ms | 3.86 ms | 3.75 ms | 3.55 ms | **3.54 ms** | 4.5 ms | 3.81 s | — |
| 128 KiB | 7.37 ms | — | 11.6 ms | 10.8 ms | 7.37 ms | **7.36 ms** | 10.1 ms | — | — |
| 256 KiB | 15.6 ms | — | 35.2 ms | 30.4 ms | 15.6 ms | **15.5 ms** | 22.1 ms | — | — |
| 512 KiB | 33.6 ms | — | 106 ms | 85 ms | 33.4 ms | **33.4 ms** | 48.2 ms | — | — |
| 1 MiB | 73.3 ms | — | 320 ms | 245 ms | 73.3 ms | **72.6 ms** | 103 ms | — | — |
| 2 MiB | **155 ms** | — | 965 ms | 694 ms | 156 ms | 155 ms | 222 ms | — | — |
| 4 MiB | **340 ms** | — | 2.9 s | 1.96 s | 340 ms | 341 ms | 479 ms | — | — |

## Decode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 3.63 ns | 3.63 ns | 117 ns | 118 ns | 114 ns | 118 ns | 117 ns | **2.79 ns** | — |
| 1 B | **3.64 ns** | 3.65 ns | 7.74 ns | 8.82 ns | 8.15 ns | 10.4 ns | 9.31 ns | 4.95 ns | — |
| 8 B | 13.8 ns | **13.7 ns** | 13.8 ns | 15.3 ns | 14.1 ns | 17.4 ns | 15.7 ns | 39 ns | — |
| 16 B | 20.3 ns | **20.2 ns** | 20.2 ns | 22.6 ns | 20.3 ns | 23.9 ns | 21.6 ns | 101 ns | — |
| 21 B | **22.6 ns** | 22.6 ns | 23.3 ns | 23.9 ns | 22.6 ns | 24.9 ns | 23.8 ns | 156 ns | — |
| 22 B | 25.2 ns | **25 ns** | 25.3 ns | 25.8 ns | 25.2 ns | 26.7 ns | 26.1 ns | 169 ns | — |
| 23 B | 26.5 ns | 26.5 ns | **25.7 ns** | 26.5 ns | 26.4 ns | 30 ns | 27.6 ns | 183 ns | — |
| 31 B | **40 ns** | 42 ns | 42.1 ns | 43.2 ns | 40.7 ns | 42 ns | 42.9 ns | 307 ns | — |
| 32 B | 27.1 ns | 26 ns | 26 ns | 26.9 ns | **25.9 ns** | 27.5 ns | 26.7 ns | 317 ns | 45.9 ns |
| 33 B | 26.7 ns | **26.5 ns** | 26.7 ns | 27.3 ns | 26.8 ns | 28.3 ns | 27.5 ns | 337 ns | — |
| 63 B | **51.6 ns** | 52.3 ns | 51.7 ns | 52.9 ns | 51.8 ns | 53.5 ns | 52.9 ns | 1.21 µs | — |
| 64 B | **46.7 ns** | 47.2 ns | 47.2 ns | 48 ns | 46.9 ns | 48.8 ns | 47.9 ns | 1.25 µs | 82.5 ns |
| 65 B | 47.9 ns | 49.1 ns | **47.6 ns** | 48.5 ns | 47.6 ns | 50.8 ns | 48.7 ns | 1.29 µs | — |
| 128 B | **105 ns** | 105 ns | 105 ns | 105 ns | 106 ns | 109 ns | 106 ns | 5.28 µs | — |
| 256 B | **266 ns** | 272 ns | 267 ns | 275 ns | 268 ns | 275 ns | 269 ns | 21.9 µs | — |
| 512 B | 815 ns | **813 ns** | 819 ns | 817 ns | 12.8 µs | 822 ns | 814 ns | 90.7 µs | — |
| 1 KiB | **2.74 µs** | 2.74 µs | 2.74 µs | 2.74 µs | 49 µs | 2.74 µs | 2.74 µs | 373 µs | — |
| 2 KiB | 10 µs | 10 µs | 10 µs | 10 µs | 189 µs | 10 µs | **10 µs** | 1.49 ms | — |
| 4 KiB | **21.9 µs** | 38.8 µs | 21.9 µs | 21.9 µs | 739 µs | 22.1 µs | 50.5 µs | 5.96 ms | — |
| 8 KiB | 56.2 µs | 151 µs | 55.7 µs | 55.7 µs | **55.7 µs** | 56.3 µs | 128 µs | 23.7 ms | — |
| 16 KiB | 156 µs | 592 µs | **156 µs** | 156 µs | 156 µs | 156 µs | 303 µs | 94.9 ms | — |
| 32 KiB | 458 µs | 2.33 ms | **455 µs** | 463 µs | 458 µs | 458 µs | 689 µs | 378 ms | — |
| 64 KiB | 1.24 ms | 9.29 ms | 1.36 ms | 1.38 ms | 1.37 ms | **1.23 ms** | 1.6 ms | 1.51 s | — |
| 128 KiB | 3.01 ms | — | 4.11 ms | 4.07 ms | 4.12 ms | **3 ms** | 3.9 ms | — | — |
| 256 KiB | **6.69 ms** | — | 12.4 ms | 11.4 ms | 12.5 ms | 6.69 ms | 8.65 ms | — | — |
| 512 KiB | **14.6 ms** | — | 37.5 ms | 32.3 ms | 37.8 ms | 14.6 ms | 19.1 ms | — | — |
| 1 MiB | 32.4 ms | — | 114 ms | 93.2 ms | 113 ms | **32.3 ms** | 41.6 ms | — | — |
| 2 MiB | **71 ms** | — | 344 ms | 267 ms | 343 ms | 71.2 ms | 90.8 ms | — | — |
| 4 MiB | 156 ms | — | 1.04 s | 761 ms | 1.04 s | **155 ms** | 197 ms | — | — |

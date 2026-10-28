# gcp-generic / default / algorithms

Frozen `4e5ac883…` backend comparison on AMD EPYC 9B45 / c4d-highcpu-4. Each CPU/cache cohort uses one immutable all-backend `__internals` worker. Generic uses default target flags; native uses actual `target-cpu=native`. These are separate from final-source304dc ordinary compact comparisons.

Time is the median of nine warm batch observations from three fresh processes × three batches. Time uses process CPU. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 3.15 ns | 2.91 ns | 4.85 ns | 5.09 ns | 4.6 ns | 12 ns | 5.57 ns | **2.91 ns** | — |
| 1 B | 4.12 ns | **3.15 ns** | 17.9 ns | 18.1 ns | 18 ns | 23.5 ns | 18.7 ns | 5.43 ns | — |
| 8 B | 28.9 ns | **28.3 ns** | 28.7 ns | 28.8 ns | 28.8 ns | 33.2 ns | 29.3 ns | 60.3 ns | — |
| 16 B | 40.5 ns | **39.4 ns** | 40.1 ns | 40 ns | 40 ns | 44.3 ns | 40.7 ns | 210 ns | — |
| 21 B | 41.9 ns | **40.4 ns** | 41.5 ns | 41.6 ns | 40.8 ns | 46.2 ns | 42.1 ns | 373 ns | — |
| 22 B | 47.6 ns | **46.4 ns** | 46.8 ns | 47 ns | 23.2 µs | 51.2 ns | 47.5 ns | 409 ns | — |
| 23 B | 54.4 ns | **53.2 ns** | 53.8 ns | 54.2 ns | 23.2 µs | 58.1 ns | 54.5 ns | 443 ns | — |
| 31 B | 63.3 ns | **62.5 ns** | 62.9 ns | 62.9 ns | 23 µs | 67.4 ns | 63.5 ns | 838 ns | — |
| 32 B | 31.7 ns | **30.9 ns** | 63.3 ns | 63.3 ns | 23 µs | 68 ns | 63.9 ns | 887 ns | 54.1 ns |
| 33 B | 63.7 ns | **63.1 ns** | 63.5 ns | 63.7 ns | 23.1 µs | 69.1 ns | 64.2 ns | 961 ns | — |
| 63 B | 136 ns | **135 ns** | 136 ns | 136 ns | 23.1 µs | 141 ns | 136 ns | 3.73 µs | — |
| 64 B | 32.8 ns | **32.6 ns** | 137 ns | 137 ns | 23.2 µs | 142 ns | 138 ns | 3.9 µs | 117 ns |
| 65 B | 140 ns | 140 ns | **140 ns** | 140 ns | 23.2 µs | 144 ns | 141 ns | 4.05 µs | — |
| 128 B | 375 ns | **375 ns** | 375 ns | 375 ns | 23.1 µs | 380 ns | 376 ns | 16.7 µs | — |
| 256 B | **1.19 µs** | 1.19 µs | 1.19 µs | 1.19 µs | 23.4 µs | 1.2 µs | 1.19 µs | 69.3 µs | — |
| 512 B | 4.26 µs | 4.26 µs | **4.26 µs** | 4.26 µs | 24.2 µs | 4.28 µs | 4.26 µs | 281 µs | — |
| 1 KiB | **8.22 µs** | 16.3 µs | 8.24 µs | 8.33 µs | 95.4 µs | 8.49 µs | 16.3 µs | 1.13 ms | — |
| 2 KiB | 19.5 µs | 62.9 µs | **19.4 µs** | 19.4 µs | 378 µs | 20 µs | 47.2 µs | 4.51 ms | — |
| 4 KiB | 51 µs | 247 µs | **50.8 µs** | 51.3 µs | 51.9 µs | 52.1 µs | 133 µs | 18 ms | — |
| 8 KiB | 143 µs | 979 µs | **141 µs** | 142 µs | 144 µs | 143 µs | 346 µs | 72.1 ms | — |
| 16 KiB | 420 µs | 3.89 ms | **416 µs** | 418 µs | 419 µs | 420 µs | 841 µs | 289 ms | — |
| 32 KiB | 1.24 ms | 15.5 ms | **1.23 ms** | 1.23 ms | 1.24 ms | 1.24 ms | 1.95 ms | 1.15 s | — |
| 64 KiB | **3.56 ms** | 62 ms | 3.79 ms | 3.75 ms | 3.57 ms | 3.57 ms | 4.4 ms | 4.61 s | — |
| 128 KiB | **7.53 ms** | — | 11.3 ms | 10.8 ms | 7.55 ms | 7.54 ms | 10 ms | — | — |
| 256 KiB | 16 ms | — | 33.9 ms | 30 ms | 16.1 ms | **16 ms** | 22.5 ms | — | — |
| 512 KiB | **34.3 ms** | — | 102 ms | 84.2 ms | 34.6 ms | 34.3 ms | 49.2 ms | — | — |
| 1 MiB | **75.1 ms** | — | 306 ms | 241 ms | 76 ms | 75.5 ms | 107 ms | — | — |
| 2 MiB | 160 ms | — | 918 ms | 685 ms | **160 ms** | 160 ms | 234 ms | — | — |
| 4 MiB | 351 ms | — | 2.77 s | 1.93 s | **346 ms** | 352 ms | 503 ms | — | — |

## Decode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 3.88 ns | 3.87 ns | 116 ns | 114 ns | 117 ns | 125 ns | 119 ns | **2.91 ns** | — |
| 1 B | **4.12 ns** | 4.12 ns | 8.24 ns | 9.38 ns | 8.31 ns | 16.6 ns | 9.19 ns | 4.86 ns | — |
| 8 B | 12.5 ns | **12.4 ns** | 12.5 ns | 13.7 ns | 12.9 ns | 19.2 ns | 13.6 ns | 37.1 ns | — |
| 16 B | 16.8 ns | **16.7 ns** | 16.8 ns | 18 ns | 17 ns | 22.5 ns | 17.9 ns | 97.7 ns | — |
| 21 B | 21.7 ns | 20.7 ns | **20.5 ns** | 23.3 ns | 21.4 ns | 27.6 ns | 22.2 ns | 154 ns | — |
| 22 B | 21.1 ns | **21.1 ns** | 21.4 ns | 22.7 ns | 21.5 ns | 27.5 ns | 22.6 ns | 166 ns | — |
| 23 B | 23.4 ns | **22.9 ns** | 23.1 ns | 24.5 ns | 23.4 ns | 34.9 ns | 24.3 ns | 180 ns | — |
| 31 B | **26.9 ns** | 27.3 ns | 27.2 ns | 28.3 ns | 27.1 ns | 36.2 ns | 28.2 ns | 306 ns | — |
| 32 B | 26.7 ns | 26.4 ns | **26.3 ns** | 27.4 ns | 26.7 ns | 35.4 ns | 27.2 ns | 321 ns | 49.7 ns |
| 33 B | **27.3 ns** | 27.8 ns | 27.5 ns | 28.5 ns | 27.7 ns | 36.2 ns | 28.4 ns | 340 ns | — |
| 63 B | 49.6 ns | 50.2 ns | **49.6 ns** | 51 ns | 49.8 ns | 58.8 ns | 50.9 ns | 1.2 µs | — |
| 64 B | 48.3 ns | 48.3 ns | **47.8 ns** | 49.1 ns | 48.1 ns | 57.1 ns | 48.7 ns | 1.25 µs | 133 ns |
| 65 B | 48.5 ns | **48.1 ns** | 48.8 ns | 50.1 ns | 48.8 ns | 57.5 ns | 49.9 ns | 1.3 µs | — |
| 128 B | 110 ns | 110 ns | **110 ns** | 111 ns | 110 ns | 119 ns | 111 ns | 5.25 µs | — |
| 256 B | 304 ns | 304 ns | **304 ns** | 305 ns | 304 ns | 313 ns | 308 ns | 22 µs | — |
| 512 B | 994 ns | **993 ns** | 996 ns | 994 ns | 12.7 µs | 1 µs | 995 ns | 92.4 µs | — |
| 1 KiB | 3.72 µs | **3.72 µs** | 3.73 µs | 3.73 µs | 48.8 µs | 3.73 µs | 3.73 µs | 371 µs | — |
| 2 KiB | 14.2 µs | 14.3 µs | **14.2 µs** | 14.2 µs | 189 µs | 14.2 µs | 14.2 µs | 1.47 ms | — |
| 4 KiB | 33.1 µs | 54.7 µs | **33.1 µs** | 33.1 µs | 739 µs | 33.3 µs | 52.4 µs | 5.85 ms | — |
| 8 KiB | 75.5 µs | 213 µs | 75.3 µs | **75.3 µs** | 75.5 µs | 75.4 µs | 130 µs | 23.3 ms | — |
| 16 KiB | 191 µs | 845 µs | 191 µs | 191 µs | **191 µs** | 192 µs | 310 µs | 92.9 ms | — |
| 32 KiB | 519 µs | 3.36 ms | 516 µs | 526 µs | **515 µs** | 518 µs | 720 µs | 371 ms | — |
| 64 KiB | **1.36 ms** | 13.4 ms | 1.45 ms | 1.49 ms | 1.45 ms | 1.36 ms | 1.68 ms | 1.48 s | — |
| 128 KiB | 3.31 ms | — | 4.22 ms | 4.23 ms | 4.22 ms | **3.3 ms** | 4.1 ms | — | — |
| 256 KiB | **7.25 ms** | — | 12.4 ms | 11.7 ms | 12.4 ms | 7.32 ms | 9.16 ms | — | — |
| 512 KiB | 15.9 ms | — | 36.8 ms | 32.7 ms | 36.8 ms | **15.8 ms** | 20.3 ms | — | — |
| 1 MiB | **34.7 ms** | — | 110 ms | 93 ms | 109 ms | 35.1 ms | 44.6 ms | — | — |
| 2 MiB | 76.9 ms | — | 328 ms | 265 ms | 328 ms | **76.3 ms** | 98.3 ms | — | — |
| 4 MiB | **165 ms** | — | 983 ms | 751 ms | 983 ms | 168 ms | 214 ms | — | — |

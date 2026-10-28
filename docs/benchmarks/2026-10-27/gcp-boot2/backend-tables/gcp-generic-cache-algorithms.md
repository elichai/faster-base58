# gcp-generic / cache / algorithms

Frozen `4e5ac883…` backend comparison on AMD EPYC 9B45 / c4d-highcpu-4. Each CPU/cache cohort uses one immutable all-backend `__internals` worker. Generic uses default target flags; native uses actual `target-cpu=native`. These are separate from final-source304dc ordinary compact comparisons.

Time is the median of nine warm batch observations from three fresh processes × three batches. Time uses process CPU. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 2.91 ns | **2.67 ns** | 4.85 ns | 5.09 ns | 4.36 ns | 15.7 ns | 5.7 ns | 2.67 ns | — |
| 1 B | 3.15 ns | **2.91 ns** | 18 ns | 18.3 ns | 17.9 ns | 26 ns | 18.8 ns | 5.42 ns | — |
| 8 B | 28.6 ns | **28.2 ns** | 28.8 ns | 28.9 ns | 28.6 ns | 36.6 ns | 29.4 ns | 60.2 ns | — |
| 16 B | 40 ns | **39.7 ns** | 40.2 ns | 40.6 ns | 40.1 ns | 47.7 ns | 40.7 ns | 210 ns | — |
| 21 B | 41.3 ns | **40.4 ns** | 41.5 ns | 41.7 ns | 41.5 ns | 49.9 ns | 42 ns | 373 ns | — |
| 22 B | **46.9 ns** | 47.9 ns | 47 ns | 47.1 ns | 47.4 ns | 54.2 ns | 47.7 ns | 414 ns | — |
| 23 B | **46.8 ns** | 53.2 ns | 55 ns | 54 ns | 47.3 ns | 61.3 ns | 54.4 ns | 442 ns | — |
| 31 B | **53.9 ns** | 62.6 ns | 63 ns | 63.2 ns | 53.9 ns | 74 ns | 63.5 ns | 839 ns | — |
| 32 B | 31 ns | **30.9 ns** | 63.5 ns | 63.7 ns | 52.5 ns | 73.7 ns | 63.8 ns | 885 ns | 56.1 ns |
| 33 B | 64.8 ns | **63.1 ns** | 63.7 ns | 63.8 ns | 65.7 ns | 75.1 ns | 64.3 ns | 965 ns | — |
| 63 B | **88.5 ns** | 135 ns | 136 ns | 136 ns | 89.1 ns | 147 ns | 136 ns | 3.73 µs | — |
| 64 B | 32 ns | **31.8 ns** | 137 ns | 137 ns | 85.8 ns | 149 ns | 138 ns | 3.91 µs | 117 ns |
| 65 B | **102 ns** | 140 ns | 141 ns | 141 ns | 102 ns | 152 ns | 141 ns | 4.04 µs | — |
| 128 B | 169 ns | 376 ns | 376 ns | 375 ns | **167 ns** | 392 ns | 376 ns | 16.7 µs | — |
| 256 B | 402 ns | 1.2 µs | 1.19 µs | 1.2 µs | **395 ns** | 1.2 µs | 1.19 µs | 69.4 µs | — |
| 512 B | 1.1 µs | 4.26 µs | 4.27 µs | 4.27 µs | **1.09 µs** | 4.26 µs | 4.26 µs | 281 µs | — |
| 1 KiB | 3.37 µs | 16.3 µs | 8.38 µs | 8.47 µs | **3.37 µs** | 6.7 µs | 16.3 µs | 1.13 ms | — |
| 2 KiB | 11.9 µs | 62.9 µs | 19.4 µs | 20 µs | **11.9 µs** | 15.7 µs | 47.1 µs | 4.51 ms | — |
| 4 KiB | 41.3 µs | 247 µs | 51.1 µs | 51.7 µs | **41.3 µs** | 41.4 µs | 133 µs | 18 ms | — |
| 8 KiB | 117 µs | 978 µs | 142 µs | 143 µs | **116 µs** | 116 µs | 345 µs | 72.1 ms | — |
| 16 KiB | **345 µs** | 3.89 ms | 416 µs | 420 µs | 345 µs | 347 µs | 852 µs | 288 ms | — |
| 32 KiB | 1.06 ms | 15.5 ms | 1.24 ms | 1.24 ms | 1.06 ms | **1.06 ms** | 1.95 ms | 1.15 s | — |
| 64 KiB | 2.78 ms | 62.1 ms | 3.8 ms | 3.72 ms | **2.77 ms** | 2.78 ms | 4.4 ms | 4.61 s | — |
| 128 KiB | 5.98 ms | — | 11.4 ms | 10.7 ms | **5.96 ms** | 5.98 ms | 10 ms | — | — |
| 256 KiB | 13 ms | — | 34 ms | 30 ms | 13 ms | **13 ms** | 22.4 ms | — | — |
| 512 KiB | 29 ms | — | 102 ms | 84 ms | **28.9 ms** | 29.1 ms | 49.3 ms | — | — |
| 1 MiB | 69.8 ms | — | 306 ms | 241 ms | **69.7 ms** | 70.4 ms | 107 ms | — | — |
| 2 MiB | 144 ms | — | 923 ms | 685 ms | **143 ms** | 144 ms | 233 ms | — | — |
| 4 MiB | 326 ms | — | 2.77 s | 1.93 s | 325 ms | **325 ms** | 505 ms | — | — |

## Decode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 3.39 ns | 3.39 ns | 116 ns | 117 ns | 116 ns | 126 ns | 117 ns | **2.81 ns** | — |
| 1 B | **3.4 ns** | 3.47 ns | 7.78 ns | 9.23 ns | 8.29 ns | 18.7 ns | 9.31 ns | 4.92 ns | — |
| 8 B | 15.1 ns | **13.6 ns** | 13.7 ns | 15.2 ns | 14.1 ns | 21.7 ns | 14.9 ns | 37 ns | — |
| 16 B | 17.9 ns | 17.7 ns | **17.2 ns** | 18.5 ns | 17.5 ns | 24.7 ns | 18.5 ns | 97.5 ns | — |
| 21 B | 21.1 ns | 20.8 ns | **20.7 ns** | 22 ns | 20.8 ns | 30.2 ns | 24 ns | 154 ns | — |
| 22 B | 21.8 ns | **21 ns** | 21.1 ns | 22.9 ns | 21.5 ns | 30.5 ns | 23 ns | 167 ns | — |
| 23 B | 23.5 ns | **22.8 ns** | 22.9 ns | 24.4 ns | 23.2 ns | 32.9 ns | 24.2 ns | 180 ns | — |
| 31 B | 27.5 ns | **26.8 ns** | 26.9 ns | 28.3 ns | 27.1 ns | 39.2 ns | 28.1 ns | 306 ns | — |
| 32 B | 27.2 ns | **26.1 ns** | 26.3 ns | 27.5 ns | 26.6 ns | 37.3 ns | 27.1 ns | 321 ns | 49.6 ns |
| 33 B | 28.4 ns | 27.3 ns | **27.3 ns** | 28.7 ns | 28.5 ns | 38.6 ns | 28.5 ns | 345 ns | — |
| 63 B | 50.2 ns | 50.1 ns | **49.8 ns** | 51.1 ns | 49.9 ns | 60.5 ns | 50.7 ns | 1.2 µs | — |
| 64 B | 48.2 ns | 47.8 ns | **47.8 ns** | 49.2 ns | 48.1 ns | 58.9 ns | 49.1 ns | 1.25 µs | 123 ns |
| 65 B | 48.8 ns | 49 ns | **48.2 ns** | 49.4 ns | 48.8 ns | 59.1 ns | 49.2 ns | 1.3 µs | — |
| 128 B | 110 ns | **110 ns** | 110 ns | 111 ns | 110 ns | 121 ns | 111 ns | 5.25 µs | — |
| 256 B | **346 ns** | 353 ns | 356 ns | 354 ns | 353 ns | 353 ns | 347 ns | 21.8 µs | — |
| 512 B | 1.23 µs | 1.23 µs | 1.22 µs | 1.23 µs | **953 ns** | 1.24 µs | 1.23 µs | 91.6 µs | — |
| 1 KiB | 2.92 µs | 4.81 µs | 4.81 µs | 4.82 µs | **2.92 µs** | 4.82 µs | 4.81 µs | 368 µs | — |
| 2 KiB | 10.1 µs | 18.7 µs | 18.7 µs | 18.7 µs | **10.1 µs** | 18.7 µs | 18.7 µs | 1.47 ms | — |
| 4 KiB | 33.3 µs | 72.3 µs | 33.2 µs | **33.2 µs** | 37.7 µs | 33.3 µs | 52.5 µs | 5.85 ms | — |
| 8 KiB | 75.9 µs | 284 µs | 75.8 µs | **75.5 µs** | 75.6 µs | 75.8 µs | 130 µs | 23.2 ms | — |
| 16 KiB | 193 µs | 1.12 ms | 192 µs | 192 µs | 193 µs | **191 µs** | 311 µs | 92.8 ms | — |
| 32 KiB | 521 µs | 4.47 ms | **518 µs** | 529 µs | 520 µs | 520 µs | 718 µs | 371 ms | — |
| 64 KiB | 1.29 ms | 17.8 ms | 1.46 ms | 1.5 ms | 1.46 ms | **1.29 ms** | 1.68 ms | 1.48 s | — |
| 128 KiB | **2.89 ms** | — | 4.25 ms | 4.24 ms | 4.24 ms | 2.91 ms | 4.11 ms | — | — |
| 256 KiB | **6.44 ms** | — | 12.5 ms | 11.7 ms | 12.5 ms | 6.49 ms | 9.17 ms | — | — |
| 512 KiB | **14.3 ms** | — | 37.1 ms | 32.9 ms | 37 ms | 14.3 ms | 20.3 ms | — | — |
| 1 MiB | 31.7 ms | — | 110 ms | 93.2 ms | 110 ms | **31.6 ms** | 44.6 ms | — | — |
| 2 MiB | **77.3 ms** | — | 330 ms | 264 ms | 330 ms | 77.5 ms | 98.2 ms | — | — |
| 4 MiB | **163 ms** | — | 988 ms | 752 ms | 988 ms | 166 ms | 214 ms | — | — |

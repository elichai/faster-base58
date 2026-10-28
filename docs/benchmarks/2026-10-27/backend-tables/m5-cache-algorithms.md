# m5 / cache / algorithms

Frozen `4e5ac883…` backend comparison. Native M5 uses an all-backend `__internals` worker; Wasm links each backend separately and selectively enables internal hooks. These are not the later ordinary native public API curves.

Time is the median of nine warm batch observations from three fresh processes × three batches. Native uses process CPU; Wasm uses elapsed time. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 3.38 ns | 3.34 ns | 5.2 ns | 5 ns | 4.49 ns | 6.91 ns | 5.17 ns | **2.99 ns** | — |
| 1 B | 3.81 ns | **3.49 ns** | 12 ns | 11.9 ns | 11.8 ns | 14.2 ns | 12.2 ns | 4.7 ns | — |
| 8 B | 16.8 ns | 16.8 ns | 16.6 ns | 16.2 ns | **15.9 ns** | 18.6 ns | 17.4 ns | 49.5 ns | — |
| 16 B | 22.6 ns | **21.8 ns** | 22.3 ns | 21.9 ns | 22.6 ns | 24.7 ns | 22.5 ns | 197 ns | — |
| 21 B | 26.2 ns | 26.4 ns | **25.7 ns** | 26.2 ns | 26.3 ns | 27.2 ns | 26.9 ns | 377 ns | — |
| 22 B | 26.9 ns | 31.2 ns | 32 ns | 31.8 ns | **26.9 ns** | 34.8 ns | 31.9 ns | 425 ns | — |
| 23 B | 27.7 ns | 33.1 ns | 33.1 ns | 33.4 ns | **27.6 ns** | 35.3 ns | 32.5 ns | 420 ns | — |
| 31 B | **33.2 ns** | 37.1 ns | 38 ns | 37.1 ns | 33.3 ns | 39.1 ns | 37.2 ns | 750 ns | — |
| 32 B | 12.9 ns | **12.9 ns** | 37.1 ns | 37.1 ns | 33.1 ns | 38.7 ns | 37.6 ns | 764 ns | 33.6 ns |
| 33 B | 42.8 ns | 38.1 ns | 38.4 ns | **38.1 ns** | 42.3 ns | 39.7 ns | 39.6 ns | 819 ns | — |
| 63 B | 57.5 ns | 84.6 ns | 82.1 ns | 84.8 ns | **57.4 ns** | 85 ns | 82.9 ns | 2.94 µs | — |
| 64 B | 23.3 ns | **23.2 ns** | 92.7 ns | 92.7 ns | 56.9 ns | 92.7 ns | 93.1 ns | 3.14 µs | 76.1 ns |
| 65 B | 69.9 ns | 88.5 ns | 87.2 ns | 87.6 ns | **69.7 ns** | 90.4 ns | 91.6 ns | 3.28 µs | — |
| 128 B | **115 ns** | 213 ns | 211 ns | 215 ns | 117 ns | 209 ns | 212 ns | 13.6 µs | — |
| 256 B | **233 ns** | 652 ns | 641 ns | 627 ns | 243 ns | 628 ns | 625 ns | 56.5 µs | — |
| 512 B | **579 ns** | 2.14 µs | 2.17 µs | 2.13 µs | 584 ns | 2.11 µs | 2.13 µs | 234 µs | — |
| 1 KiB | **1.94 µs** | 7.84 µs | 7.71 µs | 7.55 µs | 1.97 µs | 7.46 µs | 7.72 µs | 958 µs | — |
| 2 KiB | 7.03 µs | 27.9 µs | 28.7 µs | 28.6 µs | **6.79 µs** | 28 µs | 27.9 µs | 3.8 ms | — |
| 4 KiB | 29.4 µs | 106 µs | 106 µs | 106 µs | **28.9 µs** | 106 µs | 90.9 µs | 15.2 ms | — |
| 8 KiB | 119 µs | 414 µs | 345 µs | 336 µs | **118 µs** | 175 µs | 240 µs | 60.9 ms | — |
| 16 KiB | 430 µs | 1.66 ms | 1.05 ms | 991 µs | 440 µs | **417 µs** | 611 µs | 246 ms | — |
| 32 KiB | 1 ms | 6.51 ms | 3.2 ms | 2.89 ms | **968 µs** | 976 µs | 1.4 ms | 979 ms | — |
| 64 KiB | 2.24 ms | 25.9 ms | 9.84 ms | 8.8 ms | 2.27 ms | **2.23 ms** | 3.24 ms | 3.94 s | — |
| 128 KiB | 5.06 ms | — | 29.7 ms | 24.6 ms | **5.03 ms** | 5.04 ms | 7.43 ms | — | — |
| 256 KiB | 11.4 ms | — | 89.2 ms | 70 ms | 11.2 ms | **11.2 ms** | 17.5 ms | — | — |
| 512 KiB | 25.4 ms | — | 273 ms | 199 ms | 25.4 ms | **25.1 ms** | 39.4 ms | — | — |
| 1 MiB | 57.7 ms | — | 825 ms | 572 ms | 57.4 ms | **57 ms** | 87.6 ms | — | — |
| 2 MiB | 128 ms | — | 2.55 s | 1.54 s | 125 ms | **125 ms** | 186 ms | — | — |
| 4 MiB | 278 ms | — | 7.51 s | 4.33 s | 275 ms | **274 ms** | 413 ms | — | — |

## Decode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 2.5 ns | 2.49 ns | 6.34 ns | 6.66 ns | 6.3 ns | 7.75 ns | 7.46 ns | **2.28 ns** | — |
| 1 B | 3.23 ns | **3.01 ns** | 7.05 ns | 7.23 ns | 6.74 ns | 8.33 ns | 7.58 ns | 3.75 ns | — |
| 8 B | 9.39 ns | 9.1 ns | **9 ns** | 10.4 ns | 9.32 ns | 11.2 ns | 10.4 ns | 25.2 ns | — |
| 16 B | 10.9 ns | **10.5 ns** | 10.8 ns | 11.5 ns | 11 ns | 12.4 ns | 11.9 ns | 82.9 ns | — |
| 21 B | 13.6 ns | 13.7 ns | 13.6 ns | 14.5 ns | **13.1 ns** | 15.4 ns | 14.9 ns | 133 ns | — |
| 22 B | 13.7 ns | 13.9 ns | 13.6 ns | 14.5 ns | **13.4 ns** | 15.5 ns | 14.6 ns | 147 ns | — |
| 23 B | 15.4 ns | 15.2 ns | 15.3 ns | 16.5 ns | **15.2 ns** | 16.7 ns | 16.9 ns | 171 ns | — |
| 31 B | 19.6 ns | **19.2 ns** | 19.2 ns | 20.3 ns | 19.4 ns | 20.9 ns | 21.1 ns | 315 ns | — |
| 32 B | 16.1 ns | 16 ns | 16.2 ns | 16.8 ns | **16 ns** | 18.2 ns | 17.6 ns | 336 ns | 32.5 ns |
| 33 B | 16.2 ns | 16 ns | **15.9 ns** | 16.8 ns | **15.9 ns** | 17.9 ns | 17.1 ns | 351 ns | — |
| 63 B | 32.1 ns | 32.6 ns | **31.7 ns** | 33.1 ns | 31.8 ns | 33.4 ns | 33.7 ns | 1.13 µs | — |
| 64 B | 28.8 ns | 28.9 ns | 29 ns | 29.3 ns | **27.8 ns** | 30.2 ns | 29.8 ns | 1.3 µs | 97.2 ns |
| 65 B | **28.6 ns** | 28.8 ns | 28.8 ns | 30.1 ns | 29.1 ns | 29.8 ns | 29.6 ns | 1.31 µs | — |
| 128 B | **70.1 ns** | 70.7 ns | 71.3 ns | 72.2 ns | 70.9 ns | 74.2 ns | 72.6 ns | 4.79 µs | — |
| 256 B | 187 ns | 187 ns | **185 ns** | 188 ns | 186 ns | 188 ns | 187 ns | 19.5 µs | — |
| 512 B | **603 ns** | 715 ns | 719 ns | 720 ns | 604 ns | 708 ns | 709 ns | 78.9 µs | — |
| 1 KiB | **1.82 µs** | 2.69 µs | 2.66 µs | 2.67 µs | 1.86 µs | 2.68 µs | 2.72 µs | 315 µs | — |
| 2 KiB | 6.29 µs | 10.1 µs | 9.9 µs | 10.1 µs | **6.29 µs** | 9.83 µs | 9.84 µs | 1.28 ms | — |
| 4 KiB | 25 µs | 37.4 µs | 39.7 µs | 39.1 µs | **24.8 µs** | 30 µs | 33.4 µs | 5.03 ms | — |
| 8 KiB | 76.3 µs | 145 µs | 127 µs | 125 µs | 130 µs | **75.2 µs** | 86.1 µs | 20.3 ms | — |
| 16 KiB | 188 µs | 574 µs | 390 µs | 374 µs | 394 µs | **183 µs** | 218 µs | 81 ms | — |
| 32 KiB | **451 µs** | 2.3 ms | 1.27 ms | 1.15 ms | 1.28 ms | 452 µs | 524 µs | 329 ms | — |
| 64 KiB | 1.03 ms | 9.15 ms | 3.84 ms | 3.25 ms | 3.81 ms | **1.02 ms** | 1.22 ms | 1.31 s | — |
| 128 KiB | 2.37 ms | — | 12 ms | 9.76 ms | 11.5 ms | **2.36 ms** | 2.96 ms | — | — |
| 256 KiB | 5.36 ms | — | 35 ms | 26.8 ms | 34.9 ms | **5.31 ms** | 6.85 ms | — | — |
| 512 KiB | 12.1 ms | — | 108 ms | 75.7 ms | 106 ms | **12 ms** | 15.5 ms | — | — |
| 1 MiB | 27.2 ms | — | 323 ms | 216 ms | 321 ms | **26.9 ms** | 34.9 ms | — | — |
| 2 MiB | 62.5 ms | — | 980 ms | 631 ms | 971 ms | **61.6 ms** | 80.9 ms | — | — |
| 4 MiB | 138 ms | — | 2.95 s | 1.69 s | 2.99 s | **137 ms** | 172 ms | — | — |

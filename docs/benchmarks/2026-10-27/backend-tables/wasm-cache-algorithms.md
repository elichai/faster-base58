# wasm / cache / algorithms

Frozen `4e5ac883…` backend comparison. Native M5 uses an all-backend `__internals` worker; Wasm links each backend separately and selectively enables internal hooks. These are not the later ordinary native public API curves.

Time is the median of nine warm batch observations from three fresh processes × three batches. Native uses process CPU; Wasm uses elapsed time. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **1.87 ns** | 2.18 ns | 4.73 ns | 4.7 ns | 4.6 ns | 9.17 ns | 5.08 ns | 1.92 ns | — |
| 1 B | **2.6 ns** | 2.93 ns | 13.4 ns | 14.2 ns | 13.7 ns | 17.1 ns | 16.6 ns | 3.56 ns | — |
| 8 B | **22.5 ns** | 28.3 ns | 23.4 ns | 25.2 ns | 23.4 ns | 29.2 ns | 24.1 ns | 44.2 ns | — |
| 16 B | **37.7 ns** | 44.5 ns | 39.5 ns | 39.8 ns | 41.5 ns | 43.4 ns | 39.6 ns | 140 ns | — |
| 21 B | **40.7 ns** | 44.5 ns | 43 ns | 41.2 ns | 41.5 ns | 48 ns | 42.7 ns | 236 ns | — |
| 22 B | 51.7 ns | 54.3 ns | **48.6 ns** | 51.4 ns | 56.3 ns | 54.2 ns | 49.8 ns | 260 ns | — |
| 23 B | **52.4 ns** | 61.8 ns | 53.7 ns | 55.8 ns | 56.3 ns | 58.9 ns | 55.1 ns | 284 ns | — |
| 31 B | **58.4 ns** | 79.9 ns | 76.6 ns | 76.1 ns | 61.8 ns | 81 ns | 78 ns | 604 ns | — |
| 32 B | **25.6 ns** | 25.7 ns | 75.8 ns | 76.8 ns | 62.8 ns | 80.4 ns | 77.7 ns | 644 ns | 45 ns |
| 33 B | 102 ns | 80.4 ns | 77.2 ns | 81.8 ns | 106 ns | 81.3 ns | **76.9 ns** | 628 ns | — |
| 63 B | **123 ns** | 181 ns | 178 ns | 179 ns | 126 ns | 182 ns | 179 ns | 2.68 µs | — |
| 64 B | 99.6 ns | 100 ns | 183 ns | 185 ns | 128 ns | 188 ns | 191 ns | 2.76 µs | **92.9 ns** |
| 65 B | **177 ns** | 194 ns | 194 ns | 194 ns | 183 ns | 198 ns | 203 ns | 2.85 µs | — |
| 128 B | **296 ns** | 586 ns | 579 ns | 594 ns | 301 ns | 587 ns | 582 ns | 12.7 µs | — |
| 256 B | **860 ns** | 2.08 µs | 2.12 µs | 2.07 µs | 881 ns | 2.1 µs | 2.11 µs | 55.7 µs | — |
| 512 B | **2.85 µs** | 7.69 µs | 7.94 µs | 8.48 µs | 2.92 µs | 7.71 µs | 7.82 µs | 235 µs | — |
| 1 KiB | **10 µs** | 29.9 µs | 30 µs | 31.1 µs | 10.3 µs | 30.3 µs | 30.9 µs | 940 µs | — |
| 2 KiB | **37.6 µs** | 118 µs | 121 µs | 118 µs | 38.3 µs | 118 µs | 119 µs | 3.77 ms | — |
| 4 KiB | **147 µs** | 460 µs | 473 µs | 482 µs | 150 µs | 469 µs | 260 µs | 15.4 ms | — |
| 8 KiB | **588 µs** | 1.86 ms | 1.48 ms | 1.43 ms | 598 µs | 1.14 ms | 2.74 ms | 60.4 ms | — |
| 16 KiB | **2.68 ms** | 7.38 ms | 4.56 ms | 4.34 ms | 2.77 ms | 2.82 ms | 14.1 ms | 243 ms | — |
| 32 KiB | 6.26 ms | 29.1 ms | 13.9 ms | 12.4 ms | 6.55 ms | **6.23 ms** | 45.1 ms | 975 ms | — |
| 64 KiB | **14 ms** | 116 ms | 42 ms | 34.2 ms | 14.3 ms | 14.1 ms | 120 ms | 3.9 s | — |
| 128 KiB | **31.3 ms** | — | 126 ms | 97 ms | 31.9 ms | 31.7 ms | 300 ms | — | — |
| 256 KiB | **69.3 ms** | — | 379 ms | 269 ms | 71.2 ms | 69.7 ms | 729 ms | — | — |
| 512 KiB | **154 ms** | — | 1.14 s | 749 ms | 156 ms | 154 ms | 1.72 s | — | — |
| 1 MiB | **339 ms** | — | 3.38 s | 2.06 s | 340 ms | 340 ms | 3.83 s | — | — |
| 2 MiB | **740 ms** | — | 10.2 s | 5.74 s | 744 ms | 745 ms | 8.68 s | — | — |
| 4 MiB | **1.62 s** | — | 30.6 s | 16.2 s | 1.62 s | 1.62 s | 19.9 s | — | — |

## Decode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 2.08 ns | **0.922 ns** | 9.46 ns | 8.69 ns | 9.45 ns | 12.6 ns | 9.78 ns | 1.97 ns | — |
| 1 B | 2.87 ns | **1.4 ns** | 12 ns | 11.8 ns | 11.1 ns | 15.2 ns | 12.8 ns | 3.3 ns | — |
| 8 B | 17.7 ns | **15.8 ns** | 18.2 ns | 17.9 ns | 18.1 ns | 21.3 ns | 18.5 ns | 22.9 ns | — |
| 16 B | 25.5 ns | **23.9 ns** | 26.8 ns | 26.3 ns | 26.3 ns | 29.2 ns | 30.9 ns | 67.8 ns | — |
| 21 B | 30.7 ns | 31.3 ns | 30.5 ns | **30 ns** | 31.5 ns | 33.4 ns | 32.6 ns | 107 ns | — |
| 22 B | 31.8 ns | **30.3 ns** | 31.5 ns | 32.3 ns | 30.7 ns | 34.2 ns | 32.5 ns | 120 ns | — |
| 23 B | 39.2 ns | **37.3 ns** | 38.7 ns | 39.1 ns | 38.9 ns | 42 ns | 39.4 ns | 125 ns | — |
| 31 B | 53.2 ns | **50.6 ns** | 53.8 ns | 54.6 ns | 54.5 ns | 54.4 ns | 61.7 ns | 203 ns | — |
| 32 B | 53.8 ns | 51.7 ns | 54.5 ns | 54.3 ns | 54.3 ns | 54.7 ns | 56.8 ns | 212 ns | **41.2 ns** |
| 33 B | 52.9 ns | **52.1 ns** | 54.5 ns | 54.1 ns | 54.9 ns | 55.4 ns | 64.4 ns | 226 ns | — |
| 63 B | 129 ns | 127 ns | 133 ns | 129 ns | 132 ns | **126 ns** | 138 ns | 810 ns | — |
| 64 B | 133 ns | 128 ns | 133 ns | 136 ns | 140 ns | 136 ns | 149 ns | 846 ns | **126 ns** |
| 65 B | 134 ns | **128 ns** | 133 ns | 135 ns | 140 ns | 130 ns | 147 ns | 859 ns | — |
| 128 B | 418 ns | **414 ns** | 428 ns | 426 ns | 428 ns | 424 ns | 455 ns | 3.4 µs | — |
| 256 B | 1.38 µs | **1.36 µs** | 1.39 µs | 1.41 µs | 1.42 µs | 1.38 µs | 1.42 µs | 16.7 µs | — |
| 512 B | 2.65 µs | 5.05 µs | 5.1 µs | 6.28 µs | **2.63 µs** | 4.93 µs | 5.83 µs | 72.4 µs | — |
| 1 KiB | 9.17 µs | 19.3 µs | 18 µs | 18.6 µs | **9.15 µs** | 18.1 µs | 20.1 µs | 311 µs | — |
| 2 KiB | **33.3 µs** | 69.6 µs | 75.6 µs | 71.6 µs | 33.4 µs | 70 µs | 76.8 µs | 1.26 ms | — |
| 4 KiB | **124 µs** | 276 µs | 220 µs | 218 µs | 129 µs | 212 µs | 153 µs | 5.15 ms | — |
| 8 KiB | **504 µs** | 1.07 ms | 639 µs | 605 µs | 641 µs | 513 µs | 1.2 ms | 20.9 ms | — |
| 16 KiB | 1.22 ms | 4.32 ms | 1.88 ms | 1.8 ms | 1.9 ms | **1.22 ms** | 5.65 ms | 81.4 ms | — |
| 32 KiB | **2.81 ms** | 17.8 ms | 5.66 ms | 4.98 ms | 5.64 ms | 2.82 ms | 17.4 ms | 332 ms | — |
| 64 KiB | **6.46 ms** | 70.1 ms | 17 ms | 13.8 ms | 17 ms | 6.6 ms | 47.2 ms | 1.31 s | — |
| 128 KiB | **14.9 ms** | — | 50.8 ms | 38.9 ms | 50.7 ms | 15 ms | 119 ms | — | — |
| 256 KiB | **33.5 ms** | — | 151 ms | 108 ms | 151 ms | 33.8 ms | 295 ms | — | — |
| 512 KiB | **75.1 ms** | — | 455 ms | 299 ms | 455 ms | 75.4 ms | 702 ms | — | — |
| 1 MiB | **168 ms** | — | 1.35 s | 834 ms | 1.34 s | 168 ms | 1.59 s | — | — |
| 2 MiB | **372 ms** | — | 4.04 s | 2.31 s | 4.04 s | 375 ms | 3.64 s | — | — |
| 4 MiB | **818 ms** | — | 12.1 s | 6.46 s | 12.1 s | 825 ms | 8.3 s | — | — |

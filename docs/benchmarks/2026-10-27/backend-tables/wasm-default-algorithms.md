# wasm / default / algorithms

Frozen `4e5ac883…` backend comparison. Native M5 uses an all-backend `__internals` worker; Wasm links each backend separately and selectively enables internal hooks. These are not the later ordinary native public API curves.

Time is the median of nine warm batch observations from three fresh processes × three batches. Native uses process CPU; Wasm uses elapsed time. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **1.85 ns** | 2.11 ns | 4.67 ns | 4.71 ns | 4.67 ns | 9.32 ns | 5.06 ns | 1.98 ns | — |
| 1 B | **2.55 ns** | 2.9 ns | 13.6 ns | 13.3 ns | 13.4 ns | 17 ns | 13.6 ns | 3.51 ns | — |
| 8 B | 23 ns | 31.1 ns | 23.9 ns | **22.8 ns** | 23.2 ns | 28.4 ns | 23.1 ns | 44 ns | — |
| 16 B | 39.8 ns | 46.6 ns | 40.2 ns | 39.1 ns | 38.3 ns | 44.5 ns | **38.1 ns** | 142 ns | — |
| 21 B | 41.2 ns | 44.1 ns | 41.5 ns | **40.8 ns** | 42.4 ns | 46.6 ns | 43 ns | 246 ns | — |
| 22 B | 49.3 ns | 53.7 ns | 49.7 ns | 50.3 ns | 21.6 µs | 53.5 ns | **48.5 ns** | 258 ns | — |
| 23 B | 53.4 ns | 59 ns | 55.3 ns | **52.8 ns** | 21.9 µs | 60.5 ns | 54.4 ns | 292 ns | — |
| 31 B | 76.8 ns | 81.1 ns | **75.9 ns** | 76.7 ns | 21.4 µs | 80.6 ns | 75.9 ns | 630 ns | — |
| 32 B | 25.5 ns | **25.5 ns** | 76.7 ns | 75.4 ns | 21.5 µs | 81.3 ns | 75.7 ns | 638 ns | 44.8 ns |
| 33 B | **76.5 ns** | 80.5 ns | 78.4 ns | 78.4 ns | 21.6 µs | 82.4 ns | 78.3 ns | 648 ns | — |
| 63 B | 174 ns | **174 ns** | 179 ns | 176 ns | 21.6 µs | 181 ns | 178 ns | 2.69 µs | — |
| 64 B | 99.1 ns | 101 ns | 190 ns | 185 ns | 21.7 µs | 189 ns | 185 ns | 2.87 µs | **94 ns** |
| 65 B | **191 ns** | 192 ns | 192 ns | 191 ns | 21.8 µs | 198 ns | 192 ns | 3.05 µs | — |
| 128 B | 577 ns | 581 ns | 574 ns | **569 ns** | 21.7 µs | 571 ns | 573 ns | 12.7 µs | — |
| 256 B | **2.07 µs** | 2.1 µs | 2.09 µs | 2.15 µs | 22.3 µs | 2.11 µs | 2.09 µs | 57.8 µs | — |
| 512 B | 7.76 µs | 7.76 µs | 7.75 µs | 7.78 µs | 24.7 µs | 7.76 µs | **7.75 µs** | 237 µs | — |
| 1 KiB | 30 µs | **29.8 µs** | 29.9 µs | 30.2 µs | 99.8 µs | 30 µs | 29.9 µs | 934 µs | — |
| 2 KiB | **116 µs** | 117 µs | 118 µs | 117 µs | 403 µs | 117 µs | 118 µs | 3.77 ms | — |
| 4 KiB | 479 µs | 483 µs | 463 µs | 473 µs | 1.62 ms | 468 µs | **248 µs** | 15.4 ms | — |
| 8 KiB | **1.16 ms** | 1.86 ms | 1.48 ms | 1.43 ms | 6.5 ms | 1.17 ms | 2.74 ms | 60.4 ms | — |
| 16 KiB | **2.72 ms** | 7.31 ms | 4.54 ms | 4.26 ms | 2.76 ms | 2.76 ms | 13.9 ms | 243 ms | — |
| 32 KiB | 6.31 ms | 29.3 ms | 13.8 ms | 12.2 ms | 6.33 ms | **6.29 ms** | 44.6 ms | 980 ms | — |
| 64 KiB | **14.1 ms** | 116 ms | 41.6 ms | 33.8 ms | 14.3 ms | 14.2 ms | 119 ms | 3.9 s | — |
| 128 KiB | **31.5 ms** | — | 123 ms | 95.5 ms | 31.6 ms | 31.9 ms | 287 ms | — | — |
| 256 KiB | **69.7 ms** | — | 363 ms | 265 ms | 70 ms | 70.2 ms | 697 ms | — | — |
| 512 KiB | 154 ms | — | 1.13 s | 725 ms | **154 ms** | 157 ms | 1.63 s | — | — |
| 1 MiB | **339 ms** | — | 3.37 s | 2.05 s | 340 ms | 344 ms | 3.81 s | — | — |
| 2 MiB | **740 ms** | — | 10.1 s | 5.73 s | 741 ms | 752 ms | 8.64 s | — | — |
| 4 MiB | **1.61 s** | — | 30.6 s | 16.5 s | 1.63 s | 1.64 s | 20.6 s | — | — |

## Decode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 2.12 ns | **0.927 ns** | 9.49 ns | 9.72 ns | 9.38 ns | 12.2 ns | 9.8 ns | 1.98 ns | — |
| 1 B | 2.92 ns | **1.48 ns** | 11.9 ns | 11.5 ns | 11.3 ns | 15.1 ns | 11.9 ns | 3.38 ns | — |
| 8 B | 17.5 ns | **15.8 ns** | 17.6 ns | 18 ns | 17.9 ns | 21 ns | 17.7 ns | 22.7 ns | — |
| 16 B | 25.5 ns | **24.4 ns** | 25.8 ns | 25.7 ns | 26.2 ns | 29.3 ns | 26.3 ns | 67.2 ns | — |
| 21 B | 29.9 ns | **29.5 ns** | 30.9 ns | 29.9 ns | 30.1 ns | 33.8 ns | 29.9 ns | 107 ns | — |
| 22 B | 30.9 ns | **29.2 ns** | 31.2 ns | 31.8 ns | 30.7 ns | 36.1 ns | 31.2 ns | 115 ns | — |
| 23 B | 39 ns | **37.8 ns** | 39.1 ns | 38.6 ns | 39.1 ns | 42.2 ns | 39.9 ns | 125 ns | — |
| 31 B | 55.6 ns | **50.8 ns** | 53.7 ns | 54.4 ns | 53.9 ns | 54.7 ns | 54.8 ns | 204 ns | — |
| 32 B | 54.1 ns | 51.1 ns | 53.4 ns | 55.3 ns | 53.9 ns | 57.2 ns | 54 ns | 212 ns | **41.2 ns** |
| 33 B | 53.7 ns | **52.8 ns** | 53.6 ns | 53.3 ns | 54.7 ns | 56.3 ns | 54.5 ns | 222 ns | — |
| 63 B | 129 ns | **126 ns** | 130 ns | 130 ns | 128 ns | 134 ns | 130 ns | 807 ns | — |
| 64 B | 133 ns | **126 ns** | 133 ns | 132 ns | 133 ns | 145 ns | 129 ns | 833 ns | 127 ns |
| 65 B | 132 ns | **131 ns** | 135 ns | 133 ns | 137 ns | 137 ns | 132 ns | 859 ns | — |
| 128 B | 421 ns | **413 ns** | 426 ns | 422 ns | 428 ns | 428 ns | 419 ns | 3.48 µs | — |
| 256 B | 1.4 µs | **1.34 µs** | 1.37 µs | 1.39 µs | 1.38 µs | 1.39 µs | 1.39 µs | 16.6 µs | — |
| 512 B | 5.04 µs | 4.98 µs | **4.93 µs** | 5 µs | 14.2 µs | 4.95 µs | 5.05 µs | 72.1 µs | — |
| 1 KiB | 17.9 µs | 18.3 µs | 18.2 µs | 18.2 µs | 56.2 µs | **17.9 µs** | 18.3 µs | 308 µs | — |
| 2 KiB | 68.6 µs | 71.6 µs | 70.7 µs | 68.6 µs | 219 µs | **67.6 µs** | 73.5 µs | 1.29 ms | — |
| 4 KiB | 216 µs | 281 µs | 217 µs | 216 µs | 896 µs | 216 µs | **152 µs** | 5.34 ms | — |
| 8 KiB | 521 µs | 1.06 ms | 634 µs | 607 µs | 631 µs | **520 µs** | 1.22 ms | 20.8 ms | — |
| 16 KiB | **1.23 ms** | 4.37 ms | 1.88 ms | 1.75 ms | 1.88 ms | 1.23 ms | 5.61 ms | 81.3 ms | — |
| 32 KiB | **2.87 ms** | 17.8 ms | 5.79 ms | 4.95 ms | 5.68 ms | 2.88 ms | 17.4 ms | 329 ms | — |
| 64 KiB | **6.58 ms** | 70.4 ms | 16.9 ms | 13.9 ms | 17.1 ms | 6.62 ms | 46.8 ms | 1.31 s | — |
| 128 KiB | **15.1 ms** | — | 49.3 ms | 37.9 ms | 49.6 ms | 15.3 ms | 113 ms | — | — |
| 256 KiB | **33.7 ms** | — | 148 ms | 105 ms | 148 ms | 34.2 ms | 283 ms | — | — |
| 512 KiB | **75.6 ms** | — | 441 ms | 293 ms | 440 ms | 76.6 ms | 664 ms | — | — |
| 1 MiB | 169 ms | — | 1.35 s | 827 ms | 1.34 s | **168 ms** | 1.58 s | — | — |
| 2 MiB | **371 ms** | — | 4.02 s | 2.29 s | 4 s | 378 ms | 3.62 s | — | — |
| 4 MiB | **821 ms** | — | 12.2 s | 6.51 s | 12.1 s | 826 ms | 8.7 s | — | — |

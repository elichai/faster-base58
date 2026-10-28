# m5 / default / algorithms

Frozen `4e5ac883…` backend comparison. Native M5 uses an all-backend `__internals` worker; Wasm links each backend separately and selectively enables internal hooks. These are not the later ordinary native public API curves.

Time is the median of nine warm batch observations from three fresh processes × three batches. Native uses process CPU; Wasm uses elapsed time. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 3.36 ns | 3.19 ns | 4.57 ns | 4.97 ns | 4.3 ns | 6.63 ns | 5.3 ns | **2.88 ns** | — |
| 1 B | 3.66 ns | **3.49 ns** | 12.4 ns | 11.7 ns | 11.8 ns | 13.8 ns | 12.4 ns | 4.47 ns | — |
| 8 B | 16.4 ns | 16 ns | 15.8 ns | **15.8 ns** | 15.9 ns | 17.4 ns | 16.1 ns | 48.8 ns | — |
| 16 B | 22.4 ns | 22 ns | 22 ns | **21.4 ns** | 21.7 ns | 23.1 ns | 22.1 ns | 198 ns | — |
| 21 B | 25.9 ns | **25.8 ns** | 26.1 ns | 25.8 ns | 26.4 ns | 27.4 ns | 26.3 ns | 381 ns | — |
| 22 B | **29.9 ns** | 30.5 ns | 30.8 ns | 30.8 ns | 17.1 µs | 32.6 ns | 31.8 ns | 423 ns | — |
| 23 B | 32.9 ns | **31.9 ns** | 32.4 ns | 32.9 ns | 17.2 µs | 33.7 ns | 33 ns | 435 ns | — |
| 31 B | **35.9 ns** | 36.4 ns | 37 ns | 37.1 ns | 16.7 µs | 37.6 ns | 37.6 ns | 751 ns | — |
| 32 B | 13.2 ns | **13 ns** | 37.8 ns | 37.8 ns | 16.7 µs | 39.1 ns | 38 ns | 770 ns | 31.5 ns |
| 33 B | **38 ns** | 38.3 ns | 38.9 ns | 38.9 ns | 17.1 µs | 39.9 ns | 38.5 ns | 811 ns | — |
| 63 B | **80.7 ns** | 83 ns | 84.6 ns | 83.8 ns | 16.6 µs | 84.2 ns | 83.3 ns | 2.9 µs | — |
| 64 B | **23 ns** | 23.3 ns | 91.1 ns | 91 ns | 16.7 µs | 92.1 ns | 91.2 ns | 3.02 µs | 71.9 ns |
| 65 B | 87.3 ns | **86.9 ns** | 91.3 ns | 88 ns | 16.4 µs | 88.5 ns | 87 ns | 3.27 µs | — |
| 128 B | 212 ns | 216 ns | 216 ns | 211 ns | 16.7 µs | **210 ns** | 213 ns | 13.9 µs | — |
| 256 B | 631 ns | 626 ns | 631 ns | 635 ns | 17.3 µs | **623 ns** | 631 ns | 56.4 µs | — |
| 512 B | 2.11 µs | 2.15 µs | 2.1 µs | 2.18 µs | 17.3 µs | **2.08 µs** | 2.12 µs | 240 µs | — |
| 1 KiB | 7.58 µs | **7.38 µs** | 7.75 µs | 7.59 µs | 69.7 µs | 7.5 µs | 7.63 µs | 960 µs | — |
| 2 KiB | 28.2 µs | 27.6 µs | 28.5 µs | 28.8 µs | 295 µs | **27.4 µs** | 28 µs | 3.88 ms | — |
| 4 KiB | 107 µs | 107 µs | 106 µs | 105 µs | 1.2 ms | 106 µs | **89.1 µs** | 15.2 ms | — |
| 8 KiB | **236 µs** | 415 µs | 344 µs | 334 µs | 4.53 ms | 237 µs | 247 µs | 61.1 ms | — |
| 16 KiB | **537 µs** | 1.6 ms | 1.05 ms | 1.01 ms | 563 µs | 547 µs | 594 µs | 246 ms | — |
| 32 KiB | 1.24 ms | 6.31 ms | 3.21 ms | 2.99 ms | 1.22 ms | **1.2 ms** | 1.4 ms | 984 ms | — |
| 64 KiB | **2.71 ms** | 25.6 ms | 9.84 ms | 8.3 ms | 2.77 ms | 2.73 ms | 3.31 ms | 3.94 s | — |
| 128 KiB | 6 ms | — | 29.6 ms | 24.2 ms | 6.1 ms | **5.84 ms** | 7.42 ms | — | — |
| 256 KiB | 13.4 ms | — | 91.5 ms | 70.5 ms | 13.2 ms | **13.1 ms** | 16.8 ms | — | — |
| 512 KiB | 28.9 ms | — | 272 ms | 196 ms | 29.6 ms | **28.8 ms** | 37.9 ms | — | — |
| 1 MiB | 63.5 ms | — | 825 ms | 547 ms | 63.4 ms | **62.9 ms** | 84.7 ms | — | — |
| 2 MiB | 139 ms | — | 2.47 s | 1.54 s | 138 ms | **137 ms** | 192 ms | — | — |
| 4 MiB | 301 ms | — | 7.45 s | 4.31 s | **298 ms** | 300 ms | 412 ms | — | — |

## Decode

| Payload | auto | no-alloc | karatsuba | toom3 | matrix | ntt-cached | malachite | bs58 | five8 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 2.4 ns | 2.61 ns | 5.58 ns | 6.32 ns | 5.64 ns | 7.72 ns | 6.59 ns | **2.25 ns** | — |
| 1 B | **3.13 ns** | 3.18 ns | 5.91 ns | 6.63 ns | 5.94 ns | 7.76 ns | 6.64 ns | 3.84 ns | — |
| 8 B | 9.12 ns | 8.91 ns | 9.15 ns | 9.88 ns | **8.81 ns** | 11.3 ns | 10.3 ns | 25.3 ns | — |
| 16 B | 10.9 ns | **10.6 ns** | 10.6 ns | 11.4 ns | 10.9 ns | 12.9 ns | 11.6 ns | 82.1 ns | — |
| 21 B | 13.1 ns | **12.8 ns** | 12.9 ns | 14 ns | 13.1 ns | 14.9 ns | 14.3 ns | 136 ns | — |
| 22 B | 13.6 ns | **13.5 ns** | 13.5 ns | 14.6 ns | 13.6 ns | 14.8 ns | 14.6 ns | 145 ns | — |
| 23 B | 14.8 ns | **14.7 ns** | 14.8 ns | 15.7 ns | 14.8 ns | 16.9 ns | 16.1 ns | 173 ns | — |
| 31 B | 19 ns | **18.8 ns** | 19 ns | 19.8 ns | 18.9 ns | 20.4 ns | 20.3 ns | 317 ns | — |
| 32 B | 16 ns | 16 ns | **16 ns** | 16.7 ns | 16 ns | 17.7 ns | 17.1 ns | 307 ns | 32.9 ns |
| 33 B | **15.7 ns** | 16.2 ns | 16 ns | 16.8 ns | 15.7 ns | 17.6 ns | 17.1 ns | 352 ns | — |
| 63 B | 31.4 ns | 31.6 ns | 31.8 ns | 32.8 ns | **31.3 ns** | 33.2 ns | 33.2 ns | 1.22 µs | — |
| 64 B | 28.1 ns | 27.6 ns | 28.6 ns | 28.9 ns | **27.4 ns** | 28.8 ns | 28.8 ns | 1.21 µs | 94.6 ns |
| 65 B | 28.6 ns | 29.5 ns | 29 ns | 29.7 ns | **28 ns** | 29 ns | 28.9 ns | 1.25 µs | — |
| 128 B | 71.1 ns | 70.3 ns | 71.4 ns | 71.5 ns | **69.1 ns** | 71.4 ns | 72.3 ns | 4.66 µs | — |
| 256 B | **187 ns** | 189 ns | 189 ns | 190 ns | 190 ns | 188 ns | 188 ns | 19.3 µs | — |
| 512 B | 699 ns | **686 ns** | 713 ns | 697 ns | 9.81 µs | 695 ns | 697 ns | 78.2 µs | — |
| 1 KiB | 2.64 µs | 2.67 µs | 2.66 µs | 2.66 µs | 39.5 µs | **2.61 µs** | 2.65 µs | 320 µs | — |
| 2 KiB | 9.96 µs | **9.76 µs** | 9.9 µs | 9.91 µs | 151 µs | 9.77 µs | 9.92 µs | 1.27 ms | — |
| 4 KiB | 40.5 µs | 36.8 µs | 39.9 µs | 39 µs | 644 µs | 37.1 µs | **32.3 µs** | 5.04 ms | — |
| 8 KiB | 92.4 µs | 143 µs | 126 µs | 122 µs | 128 µs | 89.8 µs | **86.6 µs** | 20.2 ms | — |
| 16 KiB | 213 µs | 558 µs | 409 µs | 373 µs | 394 µs | **211 µs** | 220 µs | 81.3 ms | — |
| 32 KiB | 492 µs | 2.27 ms | 1.23 ms | 1.11 ms | 1.24 ms | **488 µs** | 526 µs | 328 ms | — |
| 64 KiB | **1.11 ms** | 9.3 ms | 3.78 ms | 3.27 ms | 3.77 ms | 1.12 ms | 1.31 ms | 1.31 s | — |
| 128 KiB | **2.57 ms** | — | 11.6 ms | 9.25 ms | 11.9 ms | 2.61 ms | 2.97 ms | — | — |
| 256 KiB | 5.8 ms | — | 35 ms | 26.7 ms | 36.5 ms | **5.75 ms** | 7.07 ms | — | — |
| 512 KiB | 12.9 ms | — | 109 ms | 79.3 ms | 111 ms | **12.8 ms** | 16.2 ms | — | — |
| 1 MiB | 28.8 ms | — | 322 ms | 213 ms | 325 ms | **28.4 ms** | 35.3 ms | — | — |
| 2 MiB | 64.1 ms | — | 973 ms | 599 ms | 972 ms | **63.7 ms** | 78.5 ms | — | — |
| 4 MiB | 140 ms | — | 2.95 s | 1.69 s | 2.94 s | **139 ms** | 173 ms | — | — |

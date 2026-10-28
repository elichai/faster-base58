# wasm / default / kernel-controls

Frozen `4e5ac883…` backend comparison. Native M5 uses an all-backend `__internals` worker; Wasm links each backend separately and selectively enables internal hooks. These are not the later ordinary native public API curves.

Time is the median of nine warm batch observations from three fresh processes × three batches. Native uses process CPU; Wasm uses elapsed time. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **1.85 ns** | 2.41 ns | 9.04 ns | 9.32 ns | 9.09 ns | 9.03 ns | 9.19 ns | — | 4.7 ns |
| 1 B | **2.55 ns** | 2.89 ns | 17 ns | 17 ns | 16.7 ns | 17.1 ns | 17.7 ns | 22.4 µs | 13.3 ns |
| 8 B | **23 ns** | 27.2 ns | 27.3 ns | 28.4 ns | 27.2 ns | 28.6 ns | 27.7 ns | 22.4 µs | 23.8 ns |
| 16 B | **39.8 ns** | 45.2 ns | 43.6 ns | 44.5 ns | 45.6 ns | 45.4 ns | 43.5 ns | 22.6 µs | 40 ns |
| 21 B | **41.2 ns** | 44.1 ns | 46 ns | 46.6 ns | 45.6 ns | 45.7 ns | 47.3 ns | 22.6 µs | 42.6 ns |
| 22 B | **49.3 ns** | 55 ns | 54.2 ns | 53.5 ns | 53.4 ns | 52.9 ns | 55.6 ns | 22.5 µs | 21.4 µs |
| 23 B | **53.4 ns** | 58.7 ns | 59.6 ns | 60.5 ns | 59.8 ns | 58.9 ns | 59.2 ns | 22.4 µs | 21.5 µs |
| 31 B | **76.8 ns** | 80.6 ns | 84.1 ns | 80.6 ns | 83.5 ns | 81.1 ns | 82.1 ns | 23 µs | 21.6 µs |
| 32 B | **25.5 ns** | 80.2 ns | 83.6 ns | 81.3 ns | 82.1 ns | 81.4 ns | 81.7 ns | 22.7 µs | 22 µs |
| 33 B | **76.5 ns** | 83.6 ns | 81.2 ns | 82.4 ns | 80.5 ns | 81.5 ns | 82.4 ns | 22.6 µs | 21.5 µs |
| 63 B | **174 ns** | 179 ns | 187 ns | 181 ns | 180 ns | 180 ns | 183 ns | 22.8 µs | 21.6 µs |
| 64 B | **99.1 ns** | 190 ns | 189 ns | 189 ns | 188 ns | 191 ns | 189 ns | 22.5 µs | 21.4 µs |
| 65 B | **191 ns** | 202 ns | 198 ns | 198 ns | 196 ns | 197 ns | 197 ns | 22.6 µs | 21.6 µs |
| 128 B | 577 ns | 583 ns | 583 ns | **571 ns** | 573 ns | 578 ns | 583 ns | 22.9 µs | 21.9 µs |
| 256 B | **2.07 µs** | 2.08 µs | 2.1 µs | 2.11 µs | 2.1 µs | 2.08 µs | 2.09 µs | 23.2 µs | 22.3 µs |
| 512 B | 7.76 µs | 7.82 µs | 7.79 µs | 7.76 µs | 7.81 µs | **7.69 µs** | 7.72 µs | 25.1 µs | 24.2 µs |
| 1 KiB | 30 µs | 30 µs | 30.1 µs | 30 µs | 29.8 µs | 30.2 µs | **29.7 µs** | 103 µs | 98.9 µs |
| 2 KiB | 116 µs | 117 µs | 117 µs | 117 µs | **116 µs** | 117 µs | 119 µs | 417 µs | 405 µs |
| 4 KiB | 479 µs | 468 µs | **465 µs** | 468 µs | 469 µs | 470 µs | 470 µs | 1.68 ms | 1.63 ms |
| 8 KiB | 1.16 ms | 1.82 ms | **1.16 ms** | 1.17 ms | 1.17 ms | 1.17 ms | 1.17 ms | 6.68 ms | 6.53 ms |
| 16 KiB | **2.72 ms** | 7.5 ms | 2.75 ms | 2.76 ms | 2.78 ms | 2.76 ms | 2.77 ms | — | 2.76 ms |
| 32 KiB | 6.31 ms | 29.3 ms | 6.3 ms | **6.29 ms** | 6.29 ms | 6.33 ms | 6.31 ms | — | 6.31 ms |
| 64 KiB | **14.1 ms** | 118 ms | 14.3 ms | 14.2 ms | 14.3 ms | 14.3 ms | 14.2 ms | — | 14.3 ms |
| 128 KiB | **31.5 ms** | — | 32 ms | 31.9 ms | 31.9 ms | 31.9 ms | 32 ms | — | 31.6 ms |
| 256 KiB | 69.7 ms | — | 71 ms | 70.2 ms | 71 ms | 71 ms | 71 ms | — | **69.1 ms** |
| 512 KiB | **154 ms** | — | 157 ms | 157 ms | 156 ms | 157 ms | 157 ms | — | 155 ms |
| 1 MiB | 339 ms | — | 344 ms | 344 ms | 343 ms | 344 ms | 345 ms | — | **338 ms** |
| 2 MiB | 740 ms | — | 745 ms | 752 ms | 745 ms | 752 ms | 754 ms | — | **738 ms** |
| 4 MiB | **1.61 s** | — | 1.63 s | 1.64 s | 1.62 s | 1.64 s | 1.62 s | — | 1.62 s |

## Decode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **2.12 ns** | 9.44 ns | 12.1 ns | 12.2 ns | 12.2 ns | 12.2 ns | 12 ns | — | 9.43 ns |
| 1 B | **2.92 ns** | 11.6 ns | 15.2 ns | 15.1 ns | 15.2 ns | 15.3 ns | 15.4 ns | 11.5 µs | 11.5 ns |
| 8 B | 17.5 ns | 17.7 ns | 21 ns | 21 ns | 20.8 ns | 21.1 ns | 20.8 ns | 11.8 µs | **17.5 ns** |
| 16 B | **25.5 ns** | 25.6 ns | 28.8 ns | 29.3 ns | 29.3 ns | 29.1 ns | 29.4 ns | 11.8 µs | 26.2 ns |
| 21 B | 29.9 ns | **29.3 ns** | 34.1 ns | 33.8 ns | 33 ns | 34.4 ns | 33.1 ns | 11.7 µs | 30.4 ns |
| 22 B | 30.9 ns | 31.6 ns | 35 ns | 36.1 ns | 34.6 ns | 36.9 ns | 34.7 ns | 11.7 µs | **30.7 ns** |
| 23 B | 39 ns | **37.5 ns** | 42.1 ns | 42.2 ns | 41.8 ns | 43.8 ns | 42.1 ns | 12 µs | 38.6 ns |
| 31 B | 55.6 ns | **52.2 ns** | 56.3 ns | 54.7 ns | 55 ns | 55.7 ns | 55.6 ns | 11.6 µs | 53.8 ns |
| 32 B | 54.1 ns | **54 ns** | 54.8 ns | 57.2 ns | 54.4 ns | 56.6 ns | 55.3 ns | 12 µs | 54.9 ns |
| 33 B | **53.7 ns** | 54.1 ns | 57.7 ns | 56.3 ns | 55.4 ns | 55.9 ns | 56.8 ns | 11.7 µs | 54.1 ns |
| 63 B | 129 ns | 129 ns | 133 ns | 134 ns | 133 ns | 128 ns | 133 ns | 11.7 µs | **128 ns** |
| 64 B | 133 ns | 135 ns | 135 ns | 145 ns | 137 ns | **131 ns** | 139 ns | 12 µs | 132 ns |
| 65 B | 132 ns | **132 ns** | 137 ns | 137 ns | 136 ns | 136 ns | 136 ns | 11.9 µs | 134 ns |
| 128 B | 421 ns | 426 ns | **420 ns** | 428 ns | 434 ns | 423 ns | 425 ns | 12 µs | 428 ns |
| 256 B | 1.4 µs | **1.37 µs** | 1.4 µs | 1.39 µs | 1.4 µs | 1.4 µs | 1.39 µs | 12.7 µs | 1.37 µs |
| 512 B | 5.04 µs | 4.91 µs | 4.92 µs | 4.95 µs | 4.94 µs | 4.91 µs | **4.88 µs** | 14.5 µs | 14.4 µs |
| 1 KiB | 17.9 µs | 18.2 µs | 18 µs | 17.9 µs | **17.7 µs** | 18.4 µs | 18.1 µs | 55.8 µs | 55.3 µs |
| 2 KiB | 68.6 µs | 69.2 µs | 69.7 µs | 67.6 µs | 67.9 µs | 68.6 µs | **67.5 µs** | 230 µs | 218 µs |
| 4 KiB | 216 µs | 273 µs | 218 µs | 216 µs | **215 µs** | 216 µs | 216 µs | 910 µs | 931 µs |
| 8 KiB | 521 µs | 1.09 ms | 522 µs | 520 µs | 522 µs | **519 µs** | 523 µs | — | 626 µs |
| 16 KiB | **1.23 ms** | 4.27 ms | 1.24 ms | 1.23 ms | 1.24 ms | 1.23 ms | 1.23 ms | — | 1.87 ms |
| 32 KiB | **2.87 ms** | 17 ms | 2.87 ms | 2.88 ms | 2.88 ms | 2.89 ms | 2.9 ms | — | 5.64 ms |
| 64 KiB | **6.58 ms** | 67.9 ms | 6.62 ms | 6.62 ms | 6.68 ms | 6.68 ms | 6.63 ms | — | 17 ms |
| 128 KiB | 15.1 ms | — | 15.1 ms | 15.3 ms | 15.1 ms | **15.1 ms** | 15.2 ms | — | 49.7 ms |
| 256 KiB | **33.7 ms** | — | 34.2 ms | 34.2 ms | 34.2 ms | 34.4 ms | 34.1 ms | — | 147 ms |
| 512 KiB | **75.6 ms** | — | 75.8 ms | 76.6 ms | 76.9 ms | 76.7 ms | 76.6 ms | — | 445 ms |
| 1 MiB | 169 ms | — | 171 ms | **168 ms** | 169 ms | 170 ms | 171 ms | — | 1.34 s |
| 2 MiB | **371 ms** | — | 373 ms | 378 ms | 374 ms | 378 ms | 374 ms | — | 4.01 s |
| 4 MiB | 821 ms | — | **821 ms** | 826 ms | 831 ms | 830 ms | 830 ms | — | 12.2 s |

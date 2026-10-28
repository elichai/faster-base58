# m5 / default / kernel-controls

Frozen `4e5ac883…` backend comparison. Native M5 uses an all-backend `__internals` worker; Wasm links each backend separately and selectively enables internal hooks. These are not the later ordinary native public API curves.

Time is the median of nine warm batch observations from three fresh processes × three batches. Native uses process CPU; Wasm uses elapsed time. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 3.36 ns | **2.86 ns** | 6.15 ns | 6.63 ns | 5.96 ns | 6.33 ns | 6.16 ns | — | 4.76 ns |
| 1 B | 3.66 ns | **3.14 ns** | 14 ns | 13.8 ns | 13.3 ns | 13.6 ns | 13.9 ns | 16.6 µs | 11.6 ns |
| 8 B | 16.4 ns | 16.1 ns | 17.5 ns | 17.4 ns | 17.7 ns | 17.3 ns | 17.9 ns | 16.3 µs | **15.6 ns** |
| 16 B | 22.4 ns | 21.7 ns | 23.3 ns | 23.1 ns | 23.7 ns | 23.9 ns | 23.5 ns | 16.5 µs | **21.5 ns** |
| 21 B | **25.9 ns** | 26.1 ns | 26.8 ns | 27.4 ns | 27.1 ns | 28 ns | 26.7 ns | 16.3 µs | 26 ns |
| 22 B | **29.9 ns** | 30.6 ns | 31.6 ns | 32.6 ns | 31.7 ns | 31.2 ns | 31.6 ns | 16.6 µs | 16.5 µs |
| 23 B | 32.9 ns | **31.7 ns** | 33.7 ns | 33.7 ns | 33.5 ns | 33.6 ns | 33.4 ns | 16.6 µs | 16.7 µs |
| 31 B | **35.9 ns** | 37.3 ns | 37.1 ns | 37.6 ns | 37.4 ns | 37.5 ns | 37.5 ns | 17.4 µs | 16.5 µs |
| 32 B | **13.2 ns** | 36.5 ns | 39.7 ns | 39.1 ns | 38.3 ns | 39 ns | 38.2 ns | 16.6 µs | 17.1 µs |
| 33 B | **38 ns** | 38.1 ns | 38.9 ns | 39.9 ns | 39.3 ns | 39 ns | 38.6 ns | 16.6 µs | 16.9 µs |
| 63 B | **80.7 ns** | 83.6 ns | 83.7 ns | 84.2 ns | 84.4 ns | 86 ns | 82.2 ns | 16.8 µs | 17.2 µs |
| 64 B | **23 ns** | 91.4 ns | 93 ns | 92.1 ns | 91.7 ns | 92.7 ns | 93.6 ns | 16.4 µs | 16.8 µs |
| 65 B | **87.3 ns** | 87.9 ns | 89 ns | 88.5 ns | 88.1 ns | 91.9 ns | 88 ns | 17 µs | 16.5 µs |
| 128 B | 212 ns | 214 ns | 208 ns | 210 ns | 216 ns | **208 ns** | 210 ns | 16.8 µs | 17.7 µs |
| 256 B | 631 ns | 619 ns | 627 ns | 623 ns | 630 ns | **619 ns** | 627 ns | 16.8 µs | 17.5 µs |
| 512 B | 2.11 µs | 2.14 µs | 2.11 µs | **2.08 µs** | 2.12 µs | 2.11 µs | 2.12 µs | 17.4 µs | 17.7 µs |
| 1 KiB | 7.58 µs | 7.44 µs | 7.54 µs | 7.5 µs | 7.56 µs | 7.51 µs | **7.39 µs** | 69.7 µs | 74.6 µs |
| 2 KiB | 28.2 µs | 28.7 µs | 27.9 µs | **27.4 µs** | 27.5 µs | 27.9 µs | 27.9 µs | 290 µs | 290 µs |
| 4 KiB | 107 µs | 106 µs | 107 µs | 106 µs | 106 µs | 105 µs | **104 µs** | 1.13 ms | 1.22 ms |
| 8 KiB | 236 µs | 411 µs | 235 µs | 237 µs | 347 µs | **233 µs** | 237 µs | 4.51 ms | 4.87 ms |
| 16 KiB | **537 µs** | 1.63 ms | 547 µs | 547 µs | 822 µs | 559 µs | 542 µs | — | 551 µs |
| 32 KiB | 1.24 ms | 6.27 ms | 1.21 ms | 1.2 ms | 1.92 ms | 1.21 ms | **1.19 ms** | — | 1.22 ms |
| 64 KiB | 2.71 ms | 25.3 ms | 2.7 ms | 2.73 ms | 4.41 ms | 2.69 ms | **2.69 ms** | — | 2.7 ms |
| 128 KiB | 6 ms | — | 5.93 ms | **5.84 ms** | 9.62 ms | 5.88 ms | 5.95 ms | — | 6.01 ms |
| 256 KiB | 13.4 ms | — | 13.3 ms | 13.1 ms | 21.2 ms | 13.1 ms | **13.1 ms** | — | 13.3 ms |
| 512 KiB | 28.9 ms | — | **28.7 ms** | 28.8 ms | 46.6 ms | 28.8 ms | 28.7 ms | — | 28.9 ms |
| 1 MiB | 63.5 ms | — | 63.3 ms | 62.9 ms | 103 ms | **62.9 ms** | 63.2 ms | — | 64.2 ms |
| 2 MiB | 139 ms | — | **137 ms** | 137 ms | 221 ms | 137 ms | 138 ms | — | 138 ms |
| 4 MiB | 301 ms | — | **297 ms** | 300 ms | 481 ms | 298 ms | 297 ms | — | 304 ms |

## Decode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **2.4 ns** | 5.7 ns | 7.76 ns | 7.72 ns | 7.7 ns | 7.46 ns | 8.17 ns | — | 5.9 ns |
| 1 B | **3.13 ns** | 5.73 ns | 7.82 ns | 7.76 ns | 7.91 ns | 7.9 ns | 7.72 ns | 9.14 µs | 5.91 ns |
| 8 B | 9.12 ns | **8.78 ns** | 11.2 ns | 11.3 ns | 11.1 ns | 11.2 ns | 11 ns | 9.14 µs | 8.94 ns |
| 16 B | 10.9 ns | 10.8 ns | 12.8 ns | 12.9 ns | 12.7 ns | 12.8 ns | 12.9 ns | 9.16 µs | **10.7 ns** |
| 21 B | 13.1 ns | **12.8 ns** | 15 ns | 14.9 ns | 14.6 ns | 15.2 ns | 15.3 ns | 9.03 µs | 13.2 ns |
| 22 B | 13.6 ns | **13.3 ns** | 14.7 ns | 14.8 ns | 15 ns | 14.8 ns | 15.1 ns | 9.12 µs | 13.4 ns |
| 23 B | **14.8 ns** | 14.8 ns | 16.9 ns | 16.9 ns | 17.1 ns | 16.6 ns | 16.7 ns | 9.04 µs | 14.8 ns |
| 31 B | 19 ns | **18.6 ns** | 20.8 ns | 20.4 ns | 20.6 ns | 20.3 ns | 20.3 ns | 9.2 µs | 19.4 ns |
| 32 B | 16 ns | **15.7 ns** | 17.9 ns | 17.7 ns | 17.8 ns | 17.7 ns | 17.7 ns | 9.18 µs | 15.7 ns |
| 33 B | 15.7 ns | **15.7 ns** | 17.7 ns | 17.6 ns | 17.3 ns | 17.9 ns | 18 ns | 9.13 µs | 15.8 ns |
| 63 B | **31.4 ns** | 31.8 ns | 33.4 ns | 33.2 ns | 35.5 ns | 33.1 ns | 33.4 ns | 9.06 µs | 31.6 ns |
| 64 B | 28.1 ns | 30.1 ns | 28.8 ns | 28.8 ns | 29.2 ns | 29.5 ns | 29.6 ns | 9.21 µs | **27.4 ns** |
| 65 B | 28.6 ns | 28.7 ns | 29.1 ns | 29 ns | 28.6 ns | 30 ns | 29.7 ns | 9.02 µs | **27.8 ns** |
| 128 B | 71.1 ns | 69.2 ns | 71.2 ns | 71.4 ns | 70.6 ns | 72.2 ns | 71.9 ns | 9.16 µs | **68.9 ns** |
| 256 B | 187 ns | **185 ns** | 188 ns | 188 ns | 193 ns | 186 ns | 190 ns | 9.44 µs | 189 ns |
| 512 B | 699 ns | 692 ns | **684 ns** | 695 ns | 690 ns | 695 ns | 696 ns | 9.75 µs | 10.6 µs |
| 1 KiB | 2.64 µs | 2.63 µs | 2.62 µs | **2.61 µs** | 2.65 µs | 2.64 µs | 2.64 µs | 39.5 µs | 41.4 µs |
| 2 KiB | 9.96 µs | 9.89 µs | 9.87 µs | **9.77 µs** | 9.84 µs | 9.78 µs | 9.77 µs | 152 µs | 164 µs |
| 4 KiB | 40.5 µs | 36.7 µs | 36.6 µs | 37.1 µs | 50.2 µs | 36.8 µs | **36.4 µs** | 624 µs | 674 µs |
| 8 KiB | 92.4 µs | 144 µs | 89.9 µs | 89.8 µs | 139 µs | **89.1 µs** | 89.7 µs | — | 127 µs |
| 16 KiB | 213 µs | 562 µs | 211 µs | 211 µs | 329 µs | **211 µs** | 212 µs | — | 408 µs |
| 32 KiB | 492 µs | 2.28 ms | **485 µs** | 488 µs | 787 µs | 488 µs | 486 µs | — | 1.28 ms |
| 64 KiB | **1.11 ms** | 8.96 ms | 1.12 ms | 1.12 ms | 1.81 ms | 1.13 ms | 1.12 ms | — | 3.73 ms |
| 128 KiB | 2.57 ms | — | 2.63 ms | 2.61 ms | 4.25 ms | **2.55 ms** | 2.57 ms | — | 11.7 ms |
| 256 KiB | 5.8 ms | — | 5.76 ms | **5.75 ms** | 9.5 ms | 5.75 ms | 5.75 ms | — | 35.4 ms |
| 512 KiB | 12.9 ms | — | 12.8 ms | 12.8 ms | 21.3 ms | **12.8 ms** | 12.9 ms | — | 107 ms |
| 1 MiB | 28.8 ms | — | **28.4 ms** | 28.4 ms | 47.4 ms | 28.6 ms | 28.5 ms | — | 322 ms |
| 2 MiB | 64.1 ms | — | **63.2 ms** | 63.7 ms | 105 ms | 63.4 ms | 63.3 ms | — | 976 ms |
| 4 MiB | 140 ms | — | **139 ms** | 139 ms | 232 ms | 140 ms | 139 ms | — | 2.95 s |

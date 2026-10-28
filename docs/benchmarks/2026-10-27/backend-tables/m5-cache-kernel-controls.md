# m5 / cache / kernel-controls

Frozen `4e5ac883…` backend comparison. Native M5 uses an all-backend `__internals` worker; Wasm links each backend separately and selectively enables internal hooks. These are not the later ordinary native public API curves.

Time is the median of nine warm batch observations from three fresh processes × three batches. Native uses process CPU; Wasm uses elapsed time. Bold marks the lowest observed median within this table, not statistical significance.

`—` means an explicitly skipped domain/cap; `pending` means the declared measurement has not completed. No value is extrapolated. Wrappers may select the same fallback; `route_inference` in timings.csv records the source-based route.

## Encode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | 3.38 ns | **3.03 ns** | 7.98 ns | 6.91 ns | 6.88 ns | 6.98 ns | 6.66 ns | — | 4.72 ns |
| 1 B | 3.81 ns | **3.32 ns** | 14.5 ns | 14.2 ns | 14.4 ns | 13.8 ns | 15 ns | 19.9 ns | 11.9 ns |
| 8 B | 16.8 ns | **16.1 ns** | 18.8 ns | 18.6 ns | 18.3 ns | 18.6 ns | 18.6 ns | 22.6 ns | 16.5 ns |
| 16 B | 22.6 ns | **21.6 ns** | 24 ns | 24.7 ns | 24.7 ns | 24.3 ns | 24 ns | 24.4 ns | 22.1 ns |
| 21 B | 26.2 ns | 25.8 ns | 27.5 ns | 27.2 ns | 27.3 ns | 27 ns | 28.3 ns | 27.3 ns | **25.6 ns** |
| 22 B | 26.9 ns | 31.3 ns | 33.1 ns | 34.8 ns | 33.7 ns | 33.7 ns | 33.5 ns | **26.5 ns** | 35 ns |
| 23 B | 27.7 ns | 32.8 ns | 35.4 ns | 35.3 ns | 35.1 ns | 34.8 ns | 34.6 ns | **27.1 ns** | 35.3 ns |
| 31 B | 33.2 ns | 36.9 ns | 39 ns | 39.1 ns | 38 ns | 39 ns | 38.1 ns | **32.9 ns** | 40.8 ns |
| 32 B | **12.9 ns** | 37.3 ns | 39.5 ns | 38.7 ns | 39.7 ns | 39.6 ns | 39.8 ns | 32.7 ns | 41 ns |
| 33 B | 42.8 ns | **38.4 ns** | 40.4 ns | 39.7 ns | 39.9 ns | 40.3 ns | 41.4 ns | 42.7 ns | 61.1 ns |
| 63 B | 57.5 ns | 84.7 ns | 83.4 ns | 85 ns | 83.4 ns | 84 ns | 84 ns | **56.6 ns** | 74 ns |
| 64 B | **23.3 ns** | 91.5 ns | 92.9 ns | 92.7 ns | 92.3 ns | 93.1 ns | 102 ns | 55.6 ns | 75.4 ns |
| 65 B | 69.9 ns | 88.5 ns | 92.3 ns | 90.4 ns | 88.6 ns | 89.9 ns | 90.2 ns | **68.9 ns** | 98.8 ns |
| 128 B | 115 ns | 210 ns | 208 ns | 209 ns | 211 ns | 210 ns | 214 ns | **115 ns** | 160 ns |
| 256 B | 233 ns | 627 ns | 615 ns | 628 ns | 618 ns | 647 ns | 628 ns | **226 ns** | 420 ns |
| 512 B | 579 ns | 2.12 µs | 2.1 µs | 2.11 µs | 2.14 µs | 2.12 µs | 2.15 µs | **578 ns** | 1.28 µs |
| 1 KiB | 1.94 µs | 7.57 µs | 7.53 µs | 7.46 µs | 7.49 µs | 7.56 µs | 7.58 µs | **1.91 µs** | 4.52 µs |
| 2 KiB | 7.03 µs | 27.9 µs | 28 µs | 28 µs | 28 µs | 27.6 µs | 28 µs | **6.8 µs** | 17 µs |
| 4 KiB | **29.4 µs** | 106 µs | 106 µs | 106 µs | 106 µs | 106 µs | 106 µs | 29.6 µs | 66.1 µs |
| 8 KiB | 119 µs | 418 µs | 243 µs | 175 µs | 233 µs | 186 µs | 181 µs | **117 µs** | 261 µs |
| 16 KiB | 430 µs | 1.64 ms | 548 µs | **417 µs** | 630 µs | 443 µs | 426 µs | — | 429 µs |
| 32 KiB | 1 ms | 6.38 ms | 1.21 ms | 976 µs | 1.54 ms | 1.01 ms | 972 µs | — | **970 µs** |
| 64 KiB | 2.24 ms | 25.6 ms | 2.74 ms | **2.23 ms** | 3.64 ms | 2.32 ms | 2.24 ms | — | 2.24 ms |
| 128 KiB | 5.06 ms | — | 5.96 ms | 5.04 ms | 8.51 ms | 5.16 ms | 5.03 ms | — | **5 ms** |
| 256 KiB | 11.4 ms | — | 13.6 ms | **11.2 ms** | 18.7 ms | 11.6 ms | 11.2 ms | — | **11.2 ms** |
| 512 KiB | 25.4 ms | — | 28.9 ms | 25.1 ms | 41.8 ms | 25.8 ms | **25.1 ms** | — | 25.5 ms |
| 1 MiB | 57.7 ms | — | 63.4 ms | **57 ms** | 94.3 ms | 57.1 ms | 57.2 ms | — | 57.3 ms |
| 2 MiB | 128 ms | — | 138 ms | **125 ms** | 208 ms | 126 ms | 125 ms | — | 126 ms |
| 4 MiB | 278 ms | — | 298 ms | 274 ms | 456 ms | 277 ms | **274 ms** | — | 274 ms |

## Decode

| Payload | auto | batched | ntt | ntt-cached | ntt-portable | ntt-same-owned | ntt-same-cached | matrix-forced | matrix-portable |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0 B | **2.5 ns** | 6.01 ns | 8.82 ns | 7.75 ns | 8.09 ns | 8.36 ns | 7.9 ns | — | 6.46 ns |
| 1 B | **3.23 ns** | 6.36 ns | 8.16 ns | 8.33 ns | 8.36 ns | 8.71 ns | 8.37 ns | 21.5 ns | 6.34 ns |
| 8 B | 9.39 ns | **9.01 ns** | 10.8 ns | 11.2 ns | 10.9 ns | 11 ns | 11.1 ns | 22.2 ns | 9.44 ns |
| 16 B | 10.9 ns | **10.4 ns** | 12.6 ns | 12.4 ns | 12.7 ns | 12.7 ns | 12.7 ns | 22.5 ns | 11.2 ns |
| 21 B | 13.6 ns | 13.3 ns | 15.1 ns | 15.4 ns | 15.2 ns | 15.6 ns | 15.3 ns | 27 ns | **13.2 ns** |
| 22 B | 13.7 ns | 13.6 ns | 15.5 ns | 15.5 ns | 15.8 ns | 15.2 ns | 16 ns | 28.4 ns | **13.4 ns** |
| 23 B | 15.4 ns | 15.7 ns | 17.1 ns | 16.7 ns | 17.1 ns | 17 ns | 17.1 ns | 24 ns | **15.3 ns** |
| 31 B | 19.6 ns | **19 ns** | 20.7 ns | 20.9 ns | 21.1 ns | 20.6 ns | 21 ns | 48.5 ns | 19.4 ns |
| 32 B | 16.1 ns | **15.9 ns** | 18.3 ns | 18.2 ns | 18.2 ns | 18.4 ns | 18.5 ns | 46.6 ns | 16.3 ns |
| 33 B | 16.2 ns | **15.9 ns** | 18.3 ns | 17.9 ns | 17.9 ns | 18.3 ns | 18.4 ns | 45 ns | 16 ns |
| 63 B | 32.1 ns | **31.7 ns** | 33.7 ns | 33.4 ns | 33.8 ns | 33.6 ns | 33.5 ns | 80.4 ns | 31.8 ns |
| 64 B | 28.8 ns | 29 ns | 29.7 ns | 30.2 ns | 29.8 ns | 29.9 ns | 30.2 ns | 72.2 ns | **28.2 ns** |
| 65 B | 28.6 ns | **28.2 ns** | 29.7 ns | 29.8 ns | 30 ns | 30.1 ns | 30 ns | 59.8 ns | 29.3 ns |
| 128 B | 70.1 ns | **70 ns** | 73.3 ns | 74.2 ns | 70.9 ns | 72.1 ns | 72.8 ns | 144 ns | 70.7 ns |
| 256 B | 187 ns | 186 ns | 188 ns | 188 ns | 188 ns | 187 ns | **185 ns** | 242 ns | 187 ns |
| 512 B | **603 ns** | 707 ns | 703 ns | 708 ns | 718 ns | 706 ns | 707 ns | 616 ns | 1.18 µs |
| 1 KiB | **1.82 µs** | 2.62 µs | 2.68 µs | 2.68 µs | 2.66 µs | 2.78 µs | 2.66 µs | 1.96 µs | 4.14 µs |
| 2 KiB | 6.29 µs | 9.82 µs | 9.82 µs | 9.83 µs | 9.75 µs | 9.96 µs | 10.2 µs | **6.28 µs** | 15 µs |
| 4 KiB | 25 µs | 37.4 µs | 37 µs | 30 µs | 43.4 µs | 33.1 µs | 30 µs | **24.5 µs** | 55 µs |
| 8 KiB | 76.3 µs | 147 µs | 88.9 µs | **75.2 µs** | 125 µs | 85.9 µs | 75.4 µs | — | 127 µs |
| 16 KiB | 188 µs | 583 µs | 212 µs | **183 µs** | 315 µs | 198 µs | 185 µs | — | 403 µs |
| 32 KiB | 451 µs | 2.33 ms | 488 µs | 452 µs | 774 µs | 475 µs | **450 µs** | — | 1.24 ms |
| 64 KiB | 1.03 ms | 9.04 ms | 1.13 ms | **1.02 ms** | 1.81 ms | 1.06 ms | 1.05 ms | — | 3.79 ms |
| 128 KiB | 2.37 ms | — | 2.56 ms | **2.36 ms** | 4.2 ms | 2.44 ms | 2.36 ms | — | 11.5 ms |
| 256 KiB | 5.36 ms | — | 5.88 ms | **5.31 ms** | 9.49 ms | 5.5 ms | 5.34 ms | — | 35.1 ms |
| 512 KiB | 12.1 ms | — | 12.9 ms | **12 ms** | 21.4 ms | 12.4 ms | 12 ms | — | 111 ms |
| 1 MiB | 27.2 ms | — | 28.8 ms | 26.9 ms | 48 ms | 27.6 ms | **26.9 ms** | — | 322 ms |
| 2 MiB | 62.5 ms | — | 63.5 ms | 61.6 ms | 109 ms | 61.7 ms | **61.5 ms** | — | 984 ms |
| 4 MiB | 138 ms | — | 140 ms | 137 ms | 239 ms | **137 ms** | 140 ms | — | 2.95 s |

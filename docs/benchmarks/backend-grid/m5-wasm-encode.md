# m5-wasm: encode

[Dataset scope and source metadata](README.md)

Historical 2026-10-24 snapshot. Median of three process medians.
Native clock: process CPU. Wasm clock: elapsed. Bold is observational,
not a significance test. `SKIP` is a recorded support/policy limit.

| Decoded payload bytes | Auto | Batched | Karatsuba | Toom-3 | NTT | Cached NTT | Portable NTT | Matrix | Portable matrix | Malachite | bs58 | five8 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 2.4065 ns | 2.7419 ns | 3.5679 ns | 3.5358 ns | 8.79 ns | 8.5396 ns | 8.3374 ns | 4.7406 ns | 4.5444 ns | 4.6278 ns | **1.9193 ns** | SKIP |
| 1 | 14.129 ns | 13.888 ns | 14.856 ns | 15.335 ns | 19.167 ns | 19.203 ns | 19.418 ns | 16.014 ns | 15.934 ns | 16.037 ns | **3.4737 ns** | SKIP |
| 8 | **27.922 ns** | 28.607 ns | 28.516 ns | 28.147 ns | 32.352 ns | 31.842 ns | 32.12 ns | 31.378 ns | 28.65 ns | 29.541 ns | 46.507 ns | SKIP |
| 16 | **41.977 ns** | 44.337 ns | 45.449 ns | 44.335 ns | 50.631 ns | 49.631 ns | 50.138 ns | 46.906 ns | 48.461 ns | 48.451 ns | 146.45 ns | SKIP |
| 21 | 44.395 ns | **44.207 ns** | 45.153 ns | 44.865 ns | 49.805 ns | 50.46 ns | 50.35 ns | 46.242 ns | 47.198 ns | 46.313 ns | 244.66 ns | SKIP |
| 22 | **49.5 ns** | 54.144 ns | 56.768 ns | 56.259 ns | 60.568 ns | 61.352 ns | 62.478 ns | 50.292 ns | 50.298 ns | 59.141 ns | 283.63 ns | SKIP |
| 23 | **50.005 ns** | 60.047 ns | 59.89 ns | 60.734 ns | 66.333 ns | 67.314 ns | 68.41 ns | 51.154 ns | 50.669 ns | 63.767 ns | 282.06 ns | SKIP |
| 31 | 56.445 ns | 80.625 ns | 81.654 ns | 83.823 ns | 84.436 ns | 83.808 ns | 85.125 ns | 56.295 ns | **56 ns** | 82.452 ns | 587.63 ns | SKIP |
| 32 | 26.82 ns | 80.284 ns | 80.481 ns | 81.186 ns | 86.985 ns | 86.498 ns | 85.117 ns | 26.065 ns | **26.045 ns** | 82.991 ns | 582.33 ns | 43.772 ns |
| 33 | 97.905 ns | 80.807 ns | **80.651 ns** | 83.114 ns | 85.33 ns | 84.441 ns | 84.506 ns | 100.5 ns | 99.811 ns | 83.806 ns | 643 ns | SKIP |
| 63 | **118.1 ns** | 175.96 ns | 182.7 ns | 179.11 ns | 179.69 ns | 182.41 ns | 180.53 ns | 120.77 ns | 120.93 ns | 179.16 ns | 2.691 µs | SKIP |
| 64 | 100.66 ns | 188.92 ns | 188.61 ns | 187.68 ns | 195.74 ns | 194.93 ns | 193.33 ns | 100.01 ns | 100 ns | 187.96 ns | 2.7813 µs | **93.472 ns** |
| 65 | 177.97 ns | 199.43 ns | 198.99 ns | 200 ns | 204.14 ns | 202.26 ns | 204.16 ns | 177.01 ns | **175.74 ns** | 197.46 ns | 2.878 µs | SKIP |
| 128 | 302.05 ns | 575.19 ns | 574.64 ns | 578.43 ns | 584.06 ns | 589.05 ns | 585.28 ns | 293.2 ns | **292.59 ns** | 581.55 ns | 13.734 µs | SKIP |
| 256 | **834.09 ns** | 2.063 µs | 2.0906 µs | 2.0819 µs | 2.0875 µs | 2.1452 µs | 2.1044 µs | 868.04 ns | 860.38 ns | 2.093 µs | 54.855 µs | SKIP |
| 512 | **2.75 µs** | 7.8267 µs | 7.9427 µs | 7.7257 µs | 7.9876 µs | 7.7419 µs | 7.7802 µs | 2.8255 µs | 2.8011 µs | 7.7715 µs | 233.53 µs | SKIP |
| 1024 | **9.7511 µs** | 29.846 µs | 30.181 µs | 29.839 µs | 31.779 µs | 29.976 µs | 30.163 µs | 10.065 µs | 10.097 µs | 30.229 µs | 953.82 µs | SKIP |
| 2048 | **36.318 µs** | 117.75 µs | 118.62 µs | 118.16 µs | 118.31 µs | 119.39 µs | 118.53 µs | 38.024 µs | 37.742 µs | 118.21 µs | 3.9337 ms | SKIP |
| 4096 | **145.12 µs** | 463.65 µs | 468.91 µs | 470.34 µs | 469.93 µs | 473.26 µs | 463.92 µs | 148.54 µs | 147.42 µs | 249.57 µs | 15.218 ms | SKIP |
| 8192 | **566.66 µs** | 1.8584 ms | 1.4856 ms | 1.415 ms | 1.1718 ms | 1.1571 ms | 1.1602 ms | 626.86 µs | 587.01 µs | 2.6857 ms | 60.951 ms | SKIP |
| 16384 | 2.7254 ms | 7.4075 ms | 4.531 ms | 4.3624 ms | 2.747 ms | 2.7347 ms | 2.7479 ms | **2.691 ms** | 2.7184 ms | 13.496 ms | 243.79 ms | SKIP |
| 32768 | 6.2151 ms | 29.764 ms | 13.655 ms | 12.061 ms | 6.268 ms | 6.2138 ms | 6.2169 ms | 6.3031 ms | **6.193 ms** | 43.181 ms | 974.55 ms | SKIP |
| 65536 | 14.222 ms | 117 ms | 41.051 ms | 33.717 ms | 14.322 ms | 14.148 ms | 14.139 ms | **14.064 ms** | 14.27 ms | 116.18 ms | 3.903 s | SKIP |
| 131072 | **31.403 ms** | SKIP | 123.01 ms | 95.292 ms | 31.833 ms | 31.587 ms | 31.496 ms | 31.442 ms | 31.645 ms | 288.01 ms | SKIP | SKIP |
| 262144 | **69.523 ms** | SKIP | 372.38 ms | 268.19 ms | 70.182 ms | 69.664 ms | 69.657 ms | 69.66 ms | 69.697 ms | 702.03 ms | SKIP | SKIP |
| 524288 | 153.96 ms | SKIP | 1.1311 s | 743.34 ms | 154.96 ms | **153.25 ms** | 153.85 ms | 154.12 ms | 154.46 ms | 1.6322 s | SKIP | SKIP |
| 1048576 | 342.86 ms | SKIP | 3.3909 s | 2.0476 s | 339.46 ms | 341.17 ms | **338.12 ms** | 338.51 ms | 340.78 ms | 3.8318 s | SKIP | SKIP |
| 2097152 | 739.16 ms | SKIP | 10.202 s | 5.7511 s | 744.99 ms | 740.42 ms | 738.64 ms | **738.13 ms** | 739.78 ms | 8.5852 s | SKIP | SKIP |
| 4194304 | 1.6136 s | SKIP | 30.825 s | 16.107 s | 1.6202 s | 1.6102 s | **1.6101 s** | 1.6298 s | 1.6134 s | 19.481 s | SKIP | SKIP |

## Matched kernel controls

Minima below are scoped to these controls separately. Owned twiddles
can still retain matrix-leaf tables in this historical source.

| Decoded payload bytes | Forced matrix | Same-leaf owned NTT | Same-leaf cached NTT |
| ---: | ---: | ---: | ---: |
| 0 | SKIP | 8.4405 ns | **8.4003 ns** |
| 1 | 35.537 ns | **19.208 ns** | 19.59 ns |
| 8 | 38.75 ns | 32.211 ns | **31.568 ns** |
| 16 | **44.524 ns** | 49.04 ns | 47.913 ns |
| 21 | **49.347 ns** | 50.223 ns | 49.647 ns |
| 22 | **48.593 ns** | 61.89 ns | 60.866 ns |
| 23 | **49.436 ns** | 66.489 ns | 66.663 ns |
| 31 | **55.308 ns** | 84.418 ns | 85.351 ns |
| 32 | **55.698 ns** | 84.698 ns | 84.918 ns |
| 33 | 99.49 ns | 87.047 ns | **84.654 ns** |
| 63 | **120.54 ns** | 183.4 ns | 185.69 ns |
| 64 | **120.3 ns** | 196.17 ns | 196.9 ns |
| 65 | **173.1 ns** | 202.9 ns | 204.15 ns |
| 128 | **293.08 ns** | 582.64 ns | 582.84 ns |
| 256 | **858.06 ns** | 2.1017 µs | 2.0975 µs |
| 512 | **2.797 µs** | 7.7645 µs | 7.7924 µs |
| 1024 | **10.144 µs** | 30.097 µs | 30.01 µs |
| 2048 | **37.627 µs** | 118.68 µs | 117.05 µs |
| 4096 | **148.81 µs** | 469.33 µs | 471.19 µs |
| 8192 | **581.16 µs** | 1.1669 ms | 1.1488 ms |
| 16384 | SKIP | 2.7837 ms | **2.7094 ms** |
| 32768 | SKIP | 6.2577 ms | **6.2222 ms** |
| 65536 | SKIP | 14.211 ms | **14.091 ms** |
| 131072 | SKIP | 31.73 ms | **31.392 ms** |
| 262144 | SKIP | 70.306 ms | **69.589 ms** |
| 524288 | SKIP | **154.8 ms** | 155.42 ms |
| 1048576 | SKIP | 339.62 ms | **337.63 ms** |
| 2097152 | SKIP | 753.67 ms | **739.39 ms** |
| 4194304 | SKIP | **1.6185 s** | 1.6338 s |

Exact ranges, inferred routes, bs58 ratios and skip reasons: [observations.csv](observations.csv).
Batched and bs58 stop at 64 KiB by policy. five8 supports 32/64 B only.
No unmeasured cells are interpolated.

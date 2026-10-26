# gcp-default: encode

[Dataset scope and source metadata](README.md)

Historical 2026-10-24 snapshot. Median of three process medians.
Native clock: process CPU. Wasm clock: elapsed. Bold is observational,
not a significance test. `SKIP` is a recorded support/policy limit.

| Decoded payload bytes | Auto | Batched | Karatsuba | Toom-3 | NTT | Cached NTT | Portable NTT | Matrix | Portable matrix | Malachite | bs58 | five8 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 4.1198 ns | 3.8781 ns | 5.5212 ns | 6.2975 ns | 11.564 ns | 11.561 ns | 8.958 ns | 4.394 ns | 4.3668 ns | 5.8224 ns | **2.4225 ns** | SKIP |
| 1 | 17.558 ns | 17.652 ns | 19.646 ns | 20.184 ns | 26.158 ns | 25.971 ns | 23.323 ns | 18.819 ns | 18.448 ns | 19.586 ns | **5.3788 ns** | SKIP |
| 8 | 29.566 ns | **29.548 ns** | 31.407 ns | 31.927 ns | 37.692 ns | 37.953 ns | 35.123 ns | 30.755 ns | 30.363 ns | 31.492 ns | 59.965 ns | SKIP |
| 16 | **41.302 ns** | 41.356 ns | 43.325 ns | 43.969 ns | 49.903 ns | 49.756 ns | 47.228 ns | 42.397 ns | 42.357 ns | 43.227 ns | 209.85 ns | SKIP |
| 21 | 42.096 ns | **41.946 ns** | 43.956 ns | 44.706 ns | 50.317 ns | 50.314 ns | 47.659 ns | 43.098 ns | 42.897 ns | 44.026 ns | 372.98 ns | SKIP |
| 22 | **48.02 ns** | 48.564 ns | 50.28 ns | 50.768 ns | 56.491 ns | 56.482 ns | 53.884 ns | 48.047 ns | 55.622 ns | 50.054 ns | 416.6 ns | SKIP |
| 23 | **47.507 ns** | 54.895 ns | 56.786 ns | 57.393 ns | 63.755 ns | 63.417 ns | 60.745 ns | 47.859 ns | 55.114 ns | 57.023 ns | 443.55 ns | SKIP |
| 31 | **54.316 ns** | 67.377 ns | 65.695 ns | 66.281 ns | 71.998 ns | 72.233 ns | 69.766 ns | 54.577 ns | 61.763 ns | 65.621 ns | 843.38 ns | SKIP |
| 32 | 41.347 ns | 63.925 ns | 65.956 ns | 66.619 ns | 73.039 ns | 72.888 ns | 69.787 ns | 31.541 ns | **31.182 ns** | 65.754 ns | 885.34 ns | 53.825 ns |
| 33 | 65.544 ns | **64.135 ns** | 69.855 ns | 66.823 ns | 72.632 ns | 72.528 ns | 69.924 ns | 65.591 ns | 95.464 ns | 66.048 ns | 958.07 ns | SKIP |
| 63 | 89.473 ns | 135.08 ns | 137.18 ns | 138.38 ns | 143.49 ns | 143.55 ns | 141.71 ns | **89.463 ns** | 118.34 ns | 137.37 ns | 3.7325 µs | SKIP |
| 64 | **31.785 ns** | 136.71 ns | 138.7 ns | 139.37 ns | 145.23 ns | 145.26 ns | 142.77 ns | 32.246 ns | 116.23 ns | 138.77 ns | 3.9004 µs | 116.67 ns |
| 65 | 103.6 ns | 140.1 ns | 142.1 ns | 142.59 ns | 148.48 ns | 148.55 ns | 145.99 ns | **103.53 ns** | 158.12 ns | 142.07 ns | 4.05 µs | SKIP |
| 128 | 168.55 ns | 373.33 ns | 375.01 ns | 375.16 ns | 385.32 ns | 380.09 ns | 386.85 ns | **168.45 ns** | 256.53 ns | 374.98 ns | 16.714 µs | SKIP |
| 256 | 398.74 ns | 1.1859 µs | 1.1928 µs | 1.1967 µs | 1.1928 µs | 1.1924 µs | 1.19 µs | **396.92 ns** | 690.29 ns | 1.1881 µs | 69.229 µs | SKIP |
| 512 | 1.1004 µs | 4.2445 µs | 4.2495 µs | 4.2459 µs | 4.2634 µs | 4.258 µs | 4.2512 µs | **1.0975 µs** | 2.1782 µs | 4.2551 µs | 280.84 µs | SKIP |
| 1024 | **3.3982 µs** | 16.216 µs | 8.6082 µs | 8.5634 µs | 8.706 µs | 6.4088 µs | 6.4095 µs | 3.3983 µs | 7.5201 µs | 16.217 µs | 1.1254 ms | SKIP |
| 2048 | **11.932 µs** | 62.746 µs | 19.8 µs | 19.927 µs | 20.119 µs | 15.523 µs | 15.542 µs | 11.99 µs | 27.965 µs | 47.378 µs | 4.5083 ms | SKIP |
| 4096 | 40.909 µs | 246.83 µs | 51.376 µs | 51.063 µs | 57.971 µs | 40.87 µs | 40.93 µs | **40.855 µs** | 40.987 µs | 132.73 µs | 18.034 ms | SKIP |
| 8192 | 115.01 µs | 978.14 µs | 142.2 µs | 141.59 µs | 142.28 µs | 114.88 µs | 114.81 µs | **114.7 µs** | 114.92 µs | 344.74 µs | 72.109 ms | SKIP |
| 16384 | 363.66 µs | 3.8906 ms | 432.84 µs | 431.8 µs | 439.54 µs | 366.64 µs | 365.98 µs | 364.54 µs | **362.8 µs** | 847.31 µs | 288.43 ms | SKIP |
| 32768 | 1.0552 ms | 15.51 ms | 1.2667 ms | 1.2562 ms | 1.253 ms | **1.0523 ms** | 1.0524 ms | 1.0612 ms | 1.0562 ms | 1.9836 ms | 1.1546 s | SKIP |
| 65536 | 2.7868 ms | 62.008 ms | 3.7918 ms | 3.7947 ms | 3.6056 ms | 2.7809 ms | 6.607 ms | 2.7713 ms | **2.7648 ms** | 4.4691 ms | 4.6151 s | SKIP |
| 131072 | **5.9542 ms** | SKIP | 11.359 ms | 10.721 ms | 7.6878 ms | 5.9575 ms | 26.519 ms | 5.9611 ms | 5.9705 ms | 10.172 ms | SKIP | SKIP |
| 262144 | 12.932 ms | SKIP | 34.203 ms | 29.942 ms | 16.368 ms | **12.911 ms** | 72.661 ms | 12.922 ms | 12.931 ms | 22.58 ms | SKIP | SKIP |
| 524288 | 28.351 ms | SKIP | 104.37 ms | 84.045 ms | 35.623 ms | **28.326 ms** | 178.56 ms | 28.376 ms | 28.408 ms | 49.515 ms | SKIP | SKIP |
| 1048576 | 64.032 ms | SKIP | 306.06 ms | 240.73 ms | 77.432 ms | 64.026 ms | 419.42 ms | **63.503 ms** | 63.635 ms | 107.99 ms | SKIP | SKIP |
| 2097152 | 140.28 ms | SKIP | 924.93 ms | 683.8 ms | 163.08 ms | 139.95 ms | 960.16 ms | **139.42 ms** | 140.23 ms | 234.27 ms | SKIP | SKIP |
| 4194304 | 304.56 ms | SKIP | 2.7671 s | 1.9319 s | 351.13 ms | 304.88 ms | 2.1573 s | **304.25 ms** | 305.04 ms | 509.24 ms | SKIP | SKIP |

## Matched kernel controls

Minima below are scoped to these controls separately. Owned twiddles
can still retain matrix-leaf tables in this historical source.

| Decoded payload bytes | Forced matrix | Same-leaf owned NTT | Same-leaf cached NTT |
| ---: | ---: | ---: | ---: |
| 0 | SKIP | **11.469 ns** | 11.492 ns |
| 1 | 28.534 ns | **25.816 ns** | 25.918 ns |
| 8 | **34.208 ns** | 37.567 ns | 37.673 ns |
| 16 | **41.252 ns** | 49.668 ns | 49.689 ns |
| 21 | **46.815 ns** | 50.138 ns | 50.128 ns |
| 22 | **46.362 ns** | 56.408 ns | 56.354 ns |
| 23 | **46.167 ns** | 63.53 ns | 63.143 ns |
| 31 | **52.564 ns** | 72.013 ns | 71.988 ns |
| 32 | **51.707 ns** | 72.188 ns | 72.265 ns |
| 33 | **63.138 ns** | 72.601 ns | 72.564 ns |
| 63 | **87.114 ns** | 143.43 ns | 143.52 ns |
| 64 | **84.468 ns** | 145.05 ns | 145.1 ns |
| 65 | **103.58 ns** | 148.17 ns | 149.13 ns |
| 128 | **166.32 ns** | 379.5 ns | 379.39 ns |
| 256 | **393.18 ns** | 1.1931 µs | 1.194 µs |
| 512 | **1.0983 µs** | 4.252 µs | 4.2577 µs |
| 1024 | **3.3951 µs** | 6.4264 µs | 6.3905 µs |
| 2048 | **11.92 µs** | 15.476 µs | 15.617 µs |
| 4096 | 44.763 µs | **40.909 µs** | 40.979 µs |
| 8192 | 168.43 µs | 115.33 µs | **115.22 µs** |
| 16384 | SKIP | 365.67 µs | **363.89 µs** |
| 32768 | SKIP | **1.0506 ms** | 1.0576 ms |
| 65536 | SKIP | 2.9263 ms | **2.775 ms** |
| 131072 | SKIP | 6.2774 ms | **5.9598 ms** |
| 262144 | SKIP | 13.589 ms | **12.92 ms** |
| 524288 | SKIP | 29.766 ms | **28.361 ms** |
| 1048576 | SKIP | 69.478 ms | **63.798 ms** |
| 2097152 | SKIP | 148.94 ms | **139.63 ms** |
| 4194304 | SKIP | 321.62 ms | **305.07 ms** |

Exact ranges, inferred routes, bs58 ratios and skip reasons: [observations.csv](observations.csv).
Batched and bs58 stop at 64 KiB by policy. five8 supports 32/64 B only.
No unmeasured cells are interpolated.

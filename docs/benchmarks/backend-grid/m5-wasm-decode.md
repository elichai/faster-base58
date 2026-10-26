# m5-wasm: decode

[Dataset scope and source metadata](README.md)

Historical 2026-10-24 snapshot. Median of three process medians.
Native clock: process CPU. Wasm clock: elapsed. Bold is observational,
not a significance test. `SKIP` is a recorded support/policy limit.

| Decoded payload bytes | Auto | Batched | Karatsuba | Toom-3 | NTT | Cached NTT | Portable NTT | Matrix | Portable matrix | Malachite | bs58 | five8 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 9.8139 ns | 9.4086 ns | 8.9378 ns | 9.4524 ns | 12.052 ns | 11.912 ns | 11.972 ns | 9.3355 ns | 9.4916 ns | 9.6091 ns | **1.9568 ns** | SKIP |
| 1 | 12.349 ns | 11.216 ns | 11.662 ns | 11.957 ns | 15.213 ns | 14.944 ns | 14.982 ns | 11.44 ns | 11.263 ns | 11.53 ns | **3.2824 ns** | SKIP |
| 8 | 18.944 ns | 17.483 ns | 17.559 ns | 17.845 ns | 20.605 ns | 20.898 ns | 20.544 ns | **17.029 ns** | 17.974 ns | 18.075 ns | 22.858 ns | SKIP |
| 16 | 26.82 ns | 25.26 ns | 25.958 ns | 25.127 ns | 28.715 ns | 29.144 ns | 28.938 ns | 25.01 ns | **24.976 ns** | 25.846 ns | 68.017 ns | SKIP |
| 21 | 30.873 ns | 29.299 ns | 30.894 ns | 30.852 ns | 32.571 ns | 32.863 ns | 33.284 ns | 29.719 ns | **29.272 ns** | 30.888 ns | 106.78 ns | SKIP |
| 22 | 33.134 ns | 30.431 ns | 32.22 ns | 32.259 ns | 34.186 ns | 34.839 ns | 34.244 ns | 30.627 ns | **29.828 ns** | 32.82 ns | 115.75 ns | SKIP |
| 23 | 41.058 ns | **37.601 ns** | 38.111 ns | 38.987 ns | 40.938 ns | 40.575 ns | 41.59 ns | 37.895 ns | 38.092 ns | 38.19 ns | 124.49 ns | SKIP |
| 31 | 54.663 ns | 52.843 ns | 53.312 ns | 53.554 ns | 56.497 ns | 56.542 ns | 54.308 ns | 53.635 ns | 53.627 ns | **52.827 ns** | 205.76 ns | SKIP |
| 32 | 53.053 ns | 56.72 ns | 54.586 ns | 52.378 ns | 57.956 ns | 55.491 ns | 57.511 ns | 53.355 ns | 53.353 ns | 54.572 ns | 217.97 ns | **41.101 ns** |
| 33 | 55.451 ns | 53.543 ns | 55.069 ns | 53.177 ns | 55.138 ns | 58.179 ns | 55.431 ns | **52.588 ns** | 54.223 ns | 53.988 ns | 228.24 ns | SKIP |
| 63 | 131.8 ns | **125.89 ns** | 132.15 ns | 129.66 ns | 129.56 ns | 128.83 ns | 127.46 ns | 131.58 ns | 126.78 ns | 127.64 ns | 795.5 ns | SKIP |
| 64 | 136.72 ns | 131.48 ns | 135.43 ns | 132.77 ns | 132.81 ns | 134.06 ns | 135.12 ns | 129.93 ns | 131.36 ns | 127.56 ns | 824.83 ns | **125.39 ns** |
| 65 | 135.91 ns | 134.03 ns | 138.03 ns | 131.09 ns | 138.5 ns | **128.91 ns** | 134.06 ns | 137.89 ns | 130.01 ns | 130.13 ns | 875.54 ns | SKIP |
| 128 | 434.72 ns | 417.69 ns | 426.72 ns | 418.38 ns | 418.78 ns | **413.05 ns** | 426.25 ns | 421.52 ns | 416.95 ns | 416.5 ns | 3.472 µs | SKIP |
| 256 | 1.3708 µs | 1.3568 µs | 1.3667 µs | 1.4047 µs | 1.3851 µs | 1.3977 µs | 1.3791 µs | **1.3519 µs** | 1.3619 µs | 1.3645 µs | 16.688 µs | SKIP |
| 512 | 4.894 µs | 4.9057 µs | 4.8697 µs | 4.8682 µs | 4.9343 µs | 4.9391 µs | 4.9138 µs | 2.62 µs | **2.6137 µs** | 4.8439 µs | 75.704 µs | SKIP |
| 1024 | 17.636 µs | 18.126 µs | 17.66 µs | 18.176 µs | 18.117 µs | 18.415 µs | 18.018 µs | 9.1277 µs | **9.0587 µs** | 17.847 µs | 314.94 µs | SKIP |
| 2048 | 68.039 µs | 69.131 µs | 69.466 µs | 67.628 µs | 69.318 µs | 69.835 µs | 70.182 µs | **32.822 µs** | 33.287 µs | 71.473 µs | 1.28 ms | SKIP |
| 4096 | 265.35 µs | 268.74 µs | 216.18 µs | 213.87 µs | 214.21 µs | 215.94 µs | 211.56 µs | **123.99 µs** | 125.17 µs | 151.44 µs | 5.1427 ms | SKIP |
| 8192 | 513.72 µs | 1.0685 ms | 632.62 µs | 649.94 µs | 527.12 µs | **510.1 µs** | 535.18 µs | 626.77 µs | 629.2 µs | 1.2161 ms | 20.447 ms | SKIP |
| 16384 | 1.2159 ms | 4.5959 ms | 1.8763 ms | 1.7833 ms | 1.254 ms | **1.2063 ms** | 1.2121 ms | 1.8654 ms | 1.8668 ms | 5.619 ms | 81.592 ms | SKIP |
| 32768 | **2.8263 ms** | 16.945 ms | 5.6164 ms | 4.9328 ms | 2.8869 ms | 3.0174 ms | 2.8635 ms | 5.605 ms | 5.5882 ms | 17.297 ms | 327.86 ms | SKIP |
| 65536 | **6.5135 ms** | 68.348 ms | 16.669 ms | 13.73 ms | 6.6505 ms | 6.5248 ms | 6.6583 ms | 16.808 ms | 16.785 ms | 45.343 ms | 1.3129 s | SKIP |
| 131072 | **14.893 ms** | SKIP | 50.009 ms | 38.471 ms | 15.097 ms | 15.041 ms | 14.911 ms | 49.82 ms | 49.876 ms | 117.37 ms | SKIP | SKIP |
| 262144 | 33.791 ms | SKIP | 149.03 ms | 105.35 ms | 34.071 ms | 33.685 ms | **33.486 ms** | 149.31 ms | 148.88 ms | 282.79 ms | SKIP | SKIP |
| 524288 | 75.35 ms | SKIP | 446.25 ms | 293.92 ms | 75.804 ms | 75.929 ms | **75.292 ms** | 447.63 ms | 445.09 ms | 670.52 ms | SKIP | SKIP |
| 1048576 | **167.15 ms** | SKIP | 1.3591 s | 827.97 ms | 168.05 ms | 167.92 ms | 167.37 ms | 1.3388 s | 1.3591 s | 1.5759 s | SKIP | SKIP |
| 2097152 | **370.47 ms** | SKIP | 4.0431 s | 2.2824 s | 371.52 ms | 371.15 ms | 375.09 ms | 4.0125 s | 4.0276 s | 3.6151 s | SKIP | SKIP |
| 4194304 | 818.32 ms | SKIP | 12.063 s | 6.4229 s | 820.22 ms | 815.05 ms | **813.75 ms** | 12.118 s | 12.074 s | 8.3257 s | SKIP | SKIP |

## Matched kernel controls

Minima below are scoped to these controls separately. Owned twiddles
can still retain matrix-leaf tables in this historical source.

| Decoded payload bytes | Forced matrix | Same-leaf owned NTT | Same-leaf cached NTT |
| ---: | ---: | ---: | ---: |
| 0 | SKIP | **12.168 ns** | 12.187 ns |
| 1 | 29.987 ns | 15.3 ns | **15.021 ns** |
| 8 | 33.88 ns | **20.538 ns** | 21.026 ns |
| 16 | 41.194 ns | **28.974 ns** | 29.295 ns |
| 21 | 45.142 ns | **32.685 ns** | 32.986 ns |
| 22 | 46.814 ns | **33.435 ns** | 33.697 ns |
| 23 | 47.309 ns | **41.175 ns** | 43.624 ns |
| 31 | 79.396 ns | 55.656 ns | **54.561 ns** |
| 32 | 77.129 ns | 57.694 ns | **56.738 ns** |
| 33 | 80.176 ns | **56.762 ns** | 57.52 ns |
| 63 | 136.44 ns | 129.61 ns | **126.77 ns** |
| 64 | 134.69 ns | 134.65 ns | **133.4 ns** |
| 65 | 139.79 ns | **132.92 ns** | 132.97 ns |
| 128 | **344.83 ns** | 417.39 ns | 417.65 ns |
| 256 | **849.58 ns** | 1.3854 µs | 1.3748 µs |
| 512 | **2.6182 µs** | 4.8952 µs | 4.9819 µs |
| 1024 | **9.0246 µs** | 17.843 µs | 18.072 µs |
| 2048 | **32.75 µs** | 69.851 µs | 70.525 µs |
| 4096 | **124.17 µs** | 216.46 µs | 211.66 µs |
| 8192 | SKIP | 521.05 µs | **512.06 µs** |
| 16384 | SKIP | 1.2307 ms | **1.2129 ms** |
| 32768 | SKIP | 2.9114 ms | **2.8205 ms** |
| 65536 | SKIP | 6.6693 ms | **6.5336 ms** |
| 131072 | SKIP | 15.181 ms | **14.932 ms** |
| 262144 | SKIP | 33.895 ms | **33.559 ms** |
| 524288 | SKIP | 75.689 ms | **75.39 ms** |
| 1048576 | SKIP | 168.06 ms | **167.6 ms** |
| 2097152 | SKIP | **373.5 ms** | 383.38 ms |
| 4194304 | SKIP | **819.45 ms** | 819.54 ms |

Exact ranges, inferred routes, bs58 ratios and skip reasons: [observations.csv](observations.csv).
Batched and bs58 stop at 64 KiB by policy. five8 supports 32/64 B only.
No unmeasured cells are interpolated.

# m5-native: encode

[Dataset scope and source metadata](README.md)

Historical 2026-10-24 snapshot. Median of three process medians.
Native clock: process CPU. Wasm clock: elapsed. Bold is observational,
not a significance test. `SKIP` is a recorded support/policy limit.

| Decoded payload bytes | Auto | Batched | Karatsuba | Toom-3 | NTT | Cached NTT | Portable NTT | Matrix | Portable matrix | Malachite | bs58 | five8 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 3.8656 ns | 3.8736 ns | 4.6556 ns | 4.8517 ns | 6.6437 ns | 7.4437 ns | 7.1272 ns | 4.4365 ns | 4.412 ns | 5.2285 ns | **2.8235 ns** | SKIP |
| 1 | 11.283 ns | 11.003 ns | 11.813 ns | 12.327 ns | 14.335 ns | 13.89 ns | 14.139 ns | 11.892 ns | 11.803 ns | 12.553 ns | **4.5147 ns** | SKIP |
| 8 | **15.136 ns** | 15.422 ns | 15.758 ns | 15.755 ns | 18.002 ns | 18.85 ns | 18.725 ns | 16.189 ns | 16.283 ns | 16.363 ns | 49.522 ns | SKIP |
| 16 | **21.395 ns** | 21.444 ns | 21.893 ns | 22.131 ns | 24.246 ns | 24.883 ns | 24.951 ns | 21.717 ns | 22.028 ns | 22.509 ns | 199.94 ns | SKIP |
| 21 | 25.4 ns | **23.486 ns** | 25.546 ns | 24.987 ns | 26.18 ns | 27.678 ns | 26.8 ns | 25.699 ns | 26.081 ns | 25.878 ns | 368.86 ns | SKIP |
| 22 | 27.783 ns | 32.15 ns | 32.333 ns | 31.542 ns | 35.49 ns | 34.317 ns | 33.9 ns | **26.563 ns** | 35.342 ns | 32.479 ns | 426.52 ns | SKIP |
| 23 | 28.156 ns | 31.851 ns | 32.749 ns | 33.7 ns | 36.017 ns | 35.692 ns | 36.412 ns | **27.483 ns** | 36.508 ns | 32.525 ns | 424.85 ns | SKIP |
| 31 | 33.3 ns | 36.475 ns | 36.067 ns | 37.027 ns | 38.948 ns | 38.474 ns | 38.721 ns | **33.097 ns** | 41.483 ns | 38.142 ns | 770.67 ns | SKIP |
| 32 | **14.25 ns** | 36.142 ns | 38.158 ns | 37.825 ns | 39.883 ns | 39.683 ns | 39.758 ns | 14.257 ns | 18.697 ns | 38.38 ns | 794.13 ns | 31.042 ns |
| 33 | 42.917 ns | 37.615 ns | 39.003 ns | **36.993 ns** | 41.808 ns | 41.608 ns | 40.633 ns | 43.435 ns | 60.6 ns | 39.465 ns | 830.67 ns | SKIP |
| 63 | **56.197 ns** | 83.417 ns | 81.001 ns | 85.165 ns | 83.463 ns | 82.75 ns | 84.686 ns | 56.667 ns | 74.648 ns | 83.957 ns | 2.9787 µs | SKIP |
| 64 | **23.094 ns** | 87.962 ns | 92.242 ns | 91.906 ns | 92.073 ns | 95.583 ns | 93.601 ns | 23.651 ns | 75.631 ns | 92.26 ns | 2.9899 µs | 73.477 ns |
| 65 | **72.743 ns** | 86.667 ns | 91.2 ns | 88.517 ns | 90.422 ns | 90.9 ns | 91.333 ns | 74.514 ns | 100.53 ns | 87.6 ns | 3.3028 µs | SKIP |
| 128 | 120.68 ns | 216.27 ns | 216.88 ns | 206.93 ns | 214.98 ns | 216.75 ns | 217.94 ns | **118.34 ns** | 160.41 ns | 208.25 ns | 13.784 µs | SKIP |
| 256 | 228.84 ns | 618.8 ns | 648.4 ns | 645.87 ns | 633.47 ns | 632.67 ns | 628.4 ns | **221.6 ns** | 408.27 ns | 635.87 ns | 59.729 µs | SKIP |
| 512 | **579.47 ns** | 2.1008 µs | 2.1482 µs | 2.172 µs | 2.1189 µs | 2.1864 µs | 2.1748 µs | 588.27 ns | 1.2968 µs | 2.0923 µs | 238.88 µs | SKIP |
| 1024 | 1.9428 µs | 7.6141 µs | 7.6055 µs | 7.6588 µs | 7.5757 µs | 7.5612 µs | 7.7591 µs | **1.9348 µs** | 4.5774 µs | 7.8145 µs | 978 µs | SKIP |
| 2048 | **6.7806 µs** | 28.715 µs | 28.192 µs | 28.769 µs | 28.023 µs | 28.88 µs | 27.952 µs | 6.868 µs | 17 µs | 28.831 µs | 3.775 ms | SKIP |
| 4096 | 28.763 µs | 106.41 µs | 109.24 µs | 108.83 µs | 106 µs | 104.43 µs | 108.97 µs | **28.603 µs** | 67.037 µs | 89.265 µs | 15.855 ms | SKIP |
| 8192 | 121.64 µs | 409.8 µs | 346.36 µs | 342.09 µs | 239.31 µs | 184.15 µs | 232.06 µs | **118.87 µs** | 262.67 µs | 240.13 µs | 60.745 ms | SKIP |
| 16384 | **420.44 µs** | 1.6703 ms | 1.062 ms | 1.0037 ms | 559.29 µs | 435.33 µs | 628.17 µs | 427.44 µs | 425.89 µs | 597.83 µs | 243.03 ms | SKIP |
| 32768 | 984.75 µs | 6.368 ms | 3.227 ms | 2.894 ms | 1.2357 ms | 983.5 µs | 1.5243 ms | 986 µs | **980.25 µs** | 1.378 ms | 975.79 ms | SKIP |
| 65536 | 2.256 ms | 25.664 ms | 9.723 ms | 8.361 ms | 2.8095 ms | **2.2365 ms** | 3.631 ms | 2.285 ms | 2.25 ms | 3.25 ms | 3.9017 s | SKIP |
| 131072 | 5.131 ms | SKIP | 29.448 ms | 24.74 ms | 6.057 ms | 5.074 ms | 8.26 ms | 5.136 ms | **5.07 ms** | 7.582 ms | SKIP | SKIP |
| 262144 | 11.368 ms | SKIP | 91.09 ms | 69.194 ms | 13.433 ms | **11.336 ms** | 18.929 ms | 11.554 ms | 11.531 ms | 16.997 ms | SKIP | SKIP |
| 524288 | **25.397 ms** | SKIP | 282.39 ms | 203.57 ms | 29.2 ms | 25.506 ms | 40.993 ms | 25.918 ms | 25.521 ms | 38.092 ms | SKIP | SKIP |
| 1048576 | 57.267 ms | SKIP | 824.72 ms | 540.16 ms | 64.347 ms | **56.324 ms** | 92.253 ms | 56.968 ms | 57.126 ms | 84.289 ms | SKIP | SKIP |
| 2097152 | **123.63 ms** | SKIP | 2.453 s | 1.5459 s | 139.08 ms | 124.82 ms | 200.54 ms | 124.2 ms | 123.81 ms | 187.94 ms | SKIP | SKIP |
| 4194304 | 276.98 ms | SKIP | 7.3998 s | 4.2672 s | 306.25 ms | 273.94 ms | 443.37 ms | 275.56 ms | **273.88 ms** | 411.86 ms | SKIP | SKIP |

## Matched kernel controls

Minima below are scoped to these controls separately. Owned twiddles
can still retain matrix-leaf tables in this historical source.

| Decoded payload bytes | Forced matrix | Same-leaf owned NTT | Same-leaf cached NTT |
| ---: | ---: | ---: | ---: |
| 0 | SKIP | 7.1581 ns | **6.9333 ns** |
| 1 | 19.043 ns | **14.337 ns** | 14.657 ns |
| 8 | 21.256 ns | 18.443 ns | **18.053 ns** |
| 16 | **23.119 ns** | 24.4 ns | 24.753 ns |
| 21 | **26.747 ns** | 27.889 ns | 27.139 ns |
| 22 | **26.131 ns** | 34.658 ns | 33.808 ns |
| 23 | **26.006 ns** | 36.23 ns | 35.635 ns |
| 31 | **32.917 ns** | 38.575 ns | 39.475 ns |
| 32 | **32.107 ns** | 39.83 ns | 38.707 ns |
| 33 | 41.453 ns | **41.094 ns** | 41.601 ns |
| 63 | **55.033 ns** | 86.085 ns | 85.8 ns |
| 64 | **55.083 ns** | 93.212 ns | 93.168 ns |
| 65 | **71.905 ns** | 86.81 ns | 87.511 ns |
| 128 | **110.37 ns** | 213.87 ns | 212.8 ns |
| 256 | **232.36 ns** | 632.27 ns | 645.6 ns |
| 512 | **584 ns** | 2.0891 µs | 2.1285 µs |
| 1024 | **1.9532 µs** | 7.7386 µs | 7.6055 µs |
| 2048 | **6.9018 µs** | 28.753 µs | 28.28 µs |
| 4096 | **29 µs** | 107.03 µs | 105.29 µs |
| 8192 | **121.06 µs** | 193.61 µs | 178.37 µs |
| 16384 | SKIP | 446.89 µs | **420.44 µs** |
| 32768 | SKIP | 1.046 ms | **956 µs** |
| 65536 | SKIP | 2.395 ms | **2.241 ms** |
| 131072 | SKIP | 5.345 ms | **5.078 ms** |
| 262144 | SKIP | 11.93 ms | **11.521 ms** |
| 524288 | SKIP | 26.323 ms | **25.483 ms** |
| 1048576 | SKIP | 57.715 ms | **57.402 ms** |
| 2097152 | SKIP | 128.44 ms | **124.68 ms** |
| 4194304 | SKIP | 281.02 ms | **271.11 ms** |

Exact ranges, inferred routes, bs58 ratios and skip reasons: [observations.csv](observations.csv).
Batched and bs58 stop at 64 KiB by policy. five8 supports 32/64 B only.
No unmeasured cells are interpolated.

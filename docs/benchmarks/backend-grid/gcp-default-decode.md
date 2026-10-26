# gcp-default: decode

[Dataset scope and source metadata](README.md)

Historical 2026-10-24 snapshot. Median of three process medians.
Native clock: process CPU. Wasm clock: elapsed. Bold is observational,
not a significance test. `SKIP` is a recorded support/policy limit.

| Decoded payload bytes | Auto | Batched | Karatsuba | Toom-3 | NTT | Cached NTT | Portable NTT | Matrix | Portable matrix | Malachite | bs58 | five8 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 116.3 ns | 108.46 ns | 117.29 ns | 117.95 ns | 129 ns | 132.67 ns | 130.4 ns | 116.78 ns | 117.58 ns | 119.02 ns | **3.0629 ns** | SKIP |
| 1 | 8.879 ns | 8.0064 ns | 9.3522 ns | 10.244 ns | 15.551 ns | 15.286 ns | 14.29 ns | 8.7238 ns | 8.7303 ns | 9.9966 ns | **4.8906 ns** | SKIP |
| 8 | 13.098 ns | **12.336 ns** | 13.587 ns | 14.282 ns | 19.819 ns | 19.939 ns | 17.296 ns | 12.876 ns | 12.843 ns | 14.058 ns | 37.579 ns | SKIP |
| 16 | 18.556 ns | **16.527 ns** | 17.664 ns | 18.55 ns | 23.955 ns | 24.429 ns | 21.569 ns | 16.928 ns | 16.923 ns | 18.283 ns | 97.958 ns | SKIP |
| 21 | 21.374 ns | **20.599 ns** | 21.863 ns | 23.14 ns | 28.026 ns | 27.811 ns | 25.473 ns | 21.08 ns | 21.206 ns | 22.426 ns | 154.59 ns | SKIP |
| 22 | 22.106 ns | **21.158 ns** | 22.488 ns | 23.368 ns | 28.704 ns | 28.679 ns | 26.205 ns | 21.581 ns | 21.771 ns | 22.974 ns | 166.28 ns | SKIP |
| 23 | 23.784 ns | **22.833 ns** | 24.358 ns | 25.042 ns | 32.055 ns | 30.322 ns | 28.063 ns | 23.375 ns | 23.397 ns | 25.98 ns | 179.86 ns | SKIP |
| 31 | 30.283 ns | **26.902 ns** | 28.209 ns | 29.068 ns | 34.422 ns | 34.357 ns | 33.669 ns | 29.064 ns | 28.108 ns | 28.834 ns | 311.64 ns | SKIP |
| 32 | 26.992 ns | **26.267 ns** | 27.628 ns | 28.892 ns | 31.976 ns | 34.328 ns | 30.948 ns | 27.817 ns | 27.026 ns | 27.894 ns | 320.85 ns | 49.717 ns |
| 33 | 27.869 ns | **27.819 ns** | 28.452 ns | 29.204 ns | 34.605 ns | 32.882 ns | 30.911 ns | 27.933 ns | 28.26 ns | 28.893 ns | 341.61 ns | SKIP |
| 63 | 50.411 ns | **49.764 ns** | 51.179 ns | 51.884 ns | 55.783 ns | 55.226 ns | 53.628 ns | 50.581 ns | 50.612 ns | 51.396 ns | 1.2013 µs | SKIP |
| 64 | 48.272 ns | **47.768 ns** | 49.155 ns | 50.269 ns | 53.845 ns | 53.823 ns | 52.089 ns | 48.449 ns | 48.46 ns | 49.504 ns | 1.2557 µs | 124.29 ns |
| 65 | **48.768 ns** | 48.914 ns | 49.616 ns | 50.286 ns | 54.433 ns | 54.662 ns | 52.84 ns | 48.957 ns | 49.154 ns | 49.953 ns | 1.298 µs | SKIP |
| 128 | 111.22 ns | **110.11 ns** | 111.16 ns | 111.97 ns | 115.72 ns | 116.25 ns | 114.52 ns | 111.51 ns | 110.58 ns | 114.81 ns | 5.2426 µs | SKIP |
| 256 | 343.94 ns | **343.02 ns** | 352.86 ns | 349.9 ns | 357.46 ns | 353.86 ns | 349.71 ns | 344.91 ns | 344.16 ns | 353.13 ns | 21.841 µs | SKIP |
| 512 | 1.2257 µs | 1.2232 µs | 1.2245 µs | 1.2269 µs | 1.2347 µs | 1.24 µs | 1.2309 µs | **952.81 ns** | 1.899 µs | 1.2245 µs | 91.83 µs | SKIP |
| 1024 | 2.9216 µs | 4.7922 µs | 4.7953 µs | 4.8019 µs | 4.8043 µs | 4.8006 µs | 4.8039 µs | **2.9171 µs** | 6.4216 µs | 4.7976 µs | 368.55 µs | SKIP |
| 2048 | **10.121 µs** | 18.597 µs | 18.593 µs | 18.591 µs | 18.593 µs | 18.599 µs | 18.598 µs | 10.14 µs | 23.798 µs | 18.587 µs | 1.4661 ms | SKIP |
| 4096 | 27.926 µs | 72.181 µs | **27.724 µs** | 27.828 µs | 28.125 µs | 27.934 µs | 27.955 µs | 37.604 µs | 90.538 µs | 47.1 µs | 5.8274 ms | SKIP |
| 8192 | 66.19 µs | 283.7 µs | 66.054 µs | **65.875 µs** | 66.331 µs | 66.057 µs | 66.553 µs | 66.002 µs | 66.071 µs | 119.47 µs | 23.275 ms | SKIP |
| 16384 | 173.9 µs | 1.1216 ms | **172.48 µs** | 173.91 µs | 173.34 µs | 173.75 µs | 173.38 µs | 172.83 µs | 173.44 µs | 292.58 µs | 92.861 ms | SKIP |
| 32768 | 484.01 µs | 4.4645 ms | 481.72 µs | 495.07 µs | 484.54 µs | 483.72 µs | 483.79 µs | 481.04 µs | **480.6 µs** | 681.59 µs | 371.95 ms | SKIP |
| 65536 | **1.2161 ms** | 17.792 ms | 1.3887 ms | 1.4349 ms | 1.2982 ms | 1.2167 ms | 2.9989 ms | 1.3861 ms | 1.3879 ms | 1.6104 ms | 1.4878 s | SKIP |
| 131072 | 2.757 ms | SKIP | 4.1075 ms | 4.1073 ms | 3.1712 ms | **2.7538 ms** | 10.611 ms | 4.0927 ms | 4.1021 ms | 3.9634 ms | SKIP | SKIP |
| 262144 | 6.1806 ms | SKIP | 12.191 ms | 11.462 ms | 6.999 ms | **6.1673 ms** | 29.531 ms | 12.205 ms | 12.169 ms | 8.8829 ms | SKIP | SKIP |
| 524288 | 13.684 ms | SKIP | 36.334 ms | 32.267 ms | 15.42 ms | **13.683 ms** | 74.907 ms | 36.382 ms | 36.362 ms | 19.722 ms | SKIP | SKIP |
| 1048576 | 30.301 ms | SKIP | 108.83 ms | 91.928 ms | 33.809 ms | **30.28 ms** | 181.7 ms | 108.99 ms | 109.07 ms | 43.425 ms | SKIP | SKIP |
| 2097152 | 67.91 ms | SKIP | 326.64 ms | 263.26 ms | 75.336 ms | **67.805 ms** | 427.67 ms | 327.08 ms | 327.07 ms | 95.697 ms | SKIP | SKIP |
| 4194304 | **148.91 ms** | SKIP | 982.58 ms | 746.5 ms | 161.4 ms | 149 ms | 991.47 ms | 982.57 ms | 983.18 ms | 209.36 ms | SKIP | SKIP |

## Matched kernel controls

Minima below are scoped to these controls separately. Owned twiddles
can still retain matrix-leaf tables in this historical source.

| Decoded payload bytes | Forced matrix | Same-leaf owned NTT | Same-leaf cached NTT |
| ---: | ---: | ---: | ---: |
| 0 | SKIP | 136.64 ns | **134.73 ns** |
| 1 | 25.737 ns | 15.675 ns | **15.527 ns** |
| 8 | 28.332 ns | **20.101 ns** | 20.137 ns |
| 16 | 33.921 ns | 25.602 ns | **24.362 ns** |
| 21 | 35.889 ns | **28.158 ns** | 28.184 ns |
| 22 | 36.094 ns | 31.045 ns | **29.13 ns** |
| 23 | 35.99 ns | 32.373 ns | **31.767 ns** |
| 31 | 50.761 ns | **34.842 ns** | 35.074 ns |
| 32 | 50.383 ns | **32.151 ns** | 32.608 ns |
| 33 | 52.216 ns | 33.051 ns | **32.963 ns** |
| 63 | 77.921 ns | **58.094 ns** | 59.225 ns |
| 64 | 77.432 ns | **54.1 ns** | 55.4 ns |
| 65 | 79.427 ns | **55.379 ns** | 56.317 ns |
| 128 | 160.09 ns | 115.87 ns | **115.41 ns** |
| 256 | 355.87 ns | **354.01 ns** | 355.26 ns |
| 512 | **955.69 ns** | 1.2401 µs | 1.2333 µs |
| 1024 | **2.9205 µs** | 4.8026 µs | 4.8096 µs |
| 2048 | **10.125 µs** | 18.597 µs | 18.599 µs |
| 4096 | 38.021 µs | **27.895 µs** | 27.94 µs |
| 8192 | SKIP | 66.139 µs | **66.052 µs** |
| 16384 | SKIP | 173.79 µs | **173.3 µs** |
| 32768 | SKIP | **483.19 µs** | 483.76 µs |
| 65536 | SKIP | 1.2993 ms | **1.2237 ms** |
| 131072 | SKIP | 3.1827 ms | **2.7572 ms** |
| 262144 | SKIP | 7.0589 ms | **6.1734 ms** |
| 524288 | SKIP | 15.424 ms | **13.688 ms** |
| 1048576 | SKIP | 33.807 ms | **30.295 ms** |
| 2097152 | SKIP | 75.03 ms | **67.724 ms** |
| 4194304 | SKIP | 160.88 ms | **150.12 ms** |

Exact ranges, inferred routes, bs58 ratios and skip reasons: [observations.csv](observations.csv).
Batched and bs58 stop at 64 KiB by policy. five8 supports 32/64 B only.
No unmeasured cells are interpolated.

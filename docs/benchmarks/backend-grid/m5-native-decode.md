# m5-native: decode

[Dataset scope and source metadata](README.md)

Historical 2026-10-24 snapshot. Median of three process medians.
Native clock: process CPU. Wasm clock: elapsed. Bold is observational,
not a significance test. `SKIP` is a recorded support/policy limit.

| Decoded payload bytes | Auto | Batched | Karatsuba | Toom-3 | NTT | Cached NTT | Portable NTT | Matrix | Portable matrix | Malachite | bs58 | five8 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 5.3552 ns | 4.6075 ns | 5.8136 ns | 5.5699 ns | 7.8128 ns | 7.808 ns | 8.225 ns | 4.8051 ns | 4.9792 ns | 5.9355 ns | **2.4451 ns** | SKIP |
| 1 | 5.8563 ns | 5.8083 ns | 6.8796 ns | 6.8645 ns | 8.9308 ns | 8.9264 ns | 8.6376 ns | 5.5717 ns | 6.2787 ns | 7.2542 ns | **3.9708 ns** | SKIP |
| 8 | 9.1696 ns | **8.6333 ns** | 9.9293 ns | 9.4051 ns | 12.222 ns | 12.064 ns | 12.419 ns | 8.8811 ns | 9.081 ns | 10.464 ns | 25.611 ns | SKIP |
| 16 | 10.367 ns | **10.029 ns** | 11.288 ns | 11.108 ns | 14.26 ns | 13.11 ns | 13.059 ns | 10.465 ns | 10.422 ns | 11.625 ns | 80.575 ns | SKIP |
| 21 | 12.827 ns | **12.359 ns** | 13.65 ns | 13.716 ns | 15.883 ns | 15.812 ns | 15.75 ns | 12.43 ns | 12.873 ns | 14.82 ns | 137.84 ns | SKIP |
| 22 | 13.099 ns | **12.65 ns** | 14.379 ns | 14.33 ns | 16.177 ns | 17.258 ns | 16.829 ns | 12.96 ns | 13.263 ns | 14.617 ns | 144.69 ns | SKIP |
| 23 | 14.722 ns | **13.839 ns** | 15.35 ns | 15.167 ns | 17.582 ns | 17.236 ns | 17.225 ns | 14.141 ns | 14.187 ns | 15.963 ns | 177.69 ns | SKIP |
| 31 | 18.671 ns | 18.622 ns | 19.321 ns | 20.143 ns | 21.794 ns | 21.672 ns | 21.572 ns | **18.262 ns** | 18.543 ns | 20.329 ns | 325.6 ns | SKIP |
| 32 | 15.504 ns | **15.33 ns** | 16.758 ns | 16.371 ns | 18.771 ns | 19.229 ns | 18.704 ns | 15.748 ns | 15.793 ns | 17.03 ns | 338.67 ns | 32.6 ns |
| 33 | 15.596 ns | 15.488 ns | 16.675 ns | 16.747 ns | 18.429 ns | 18.704 ns | 18.463 ns | **15.233 ns** | 15.488 ns | 16.913 ns | 365.87 ns | SKIP |
| 63 | 31.1 ns | 30.693 ns | 32.653 ns | 32.836 ns | 35.05 ns | 34.915 ns | 35.175 ns | **30.305 ns** | 30.981 ns | 33.855 ns | 1.1429 µs | SKIP |
| 64 | 27.717 ns | **27.65 ns** | 29.467 ns | 28.184 ns | 30.257 ns | 30.583 ns | 30.789 ns | 27.978 ns | 27.934 ns | 28.823 ns | 1.1555 µs | 98.107 ns |
| 65 | 28.392 ns | 28.429 ns | 29.849 ns | 29.537 ns | 30.817 ns | 30.362 ns | 30.939 ns | 27.983 ns | **27.754 ns** | 29.061 ns | 1.2429 µs | SKIP |
| 128 | 70.329 ns | 69.201 ns | 70.8 ns | 71.886 ns | 72.895 ns | 72.902 ns | 74.748 ns | **68.853 ns** | 70.095 ns | 70.669 ns | 4.7453 µs | SKIP |
| 256 | 188.22 ns | 191.02 ns | 193.45 ns | 188.34 ns | 189.04 ns | 189.45 ns | 191.99 ns | 190.31 ns | **188.12 ns** | 189.91 ns | 19.184 µs | SKIP |
| 512 | 701.73 ns | 699.33 ns | 694 ns | 693.87 ns | 691.47 ns | 695.47 ns | 695.33 ns | 694.8 ns | 1.2168 µs | **688.4 ns** | 77.064 µs | SKIP |
| 1024 | 2.2216 µs | 2.7227 µs | 2.7244 µs | 2.6784 µs | 2.7383 µs | 2.7271 µs | 2.7276 µs | **2.1808 µs** | 4.1928 µs | 2.7447 µs | 313 µs | SKIP |
| 2048 | **6.6844 µs** | 9.8475 µs | 10.163 µs | 9.9493 µs | 10.161 µs | 10.152 µs | 9.9425 µs | 6.7591 µs | 15 µs | 10.163 µs | 1.328 ms | SKIP |
| 4096 | 25.323 µs | 38.512 µs | 39.716 µs | 38.889 µs | 38.263 µs | 30.89 µs | 43.863 µs | **24.978 µs** | 55.046 µs | 32.526 µs | 5.04 ms | SKIP |
| 8192 | 76.674 µs | 144.22 µs | 127.43 µs | 122.77 µs | 93.105 µs | **76.6 µs** | 121.2 µs | 127.52 µs | 130.29 µs | 85.4 µs | 21.402 ms | SKIP |
| 16384 | 188.57 µs | 590 µs | 409.8 µs | 381.4 µs | 213.78 µs | **186.72 µs** | 311.58 µs | 391.7 µs | 403.8 µs | 225.18 µs | 80.684 ms | SKIP |
| 32768 | 459 µs | 2.248 ms | 1.2568 ms | 1.1202 ms | 504 µs | **455.11 µs** | 756 µs | 1.2683 ms | 1.2713 ms | 525 µs | 329.09 ms | SKIP |
| 65536 | 1.0425 ms | 8.936 ms | 3.754 ms | 3.361 ms | 1.1375 ms | **1.0252 ms** | 1.741 ms | 3.763 ms | 3.905 ms | 1.2488 ms | 1.3332 s | SKIP |
| 131072 | **2.386 ms** | SKIP | 12.054 ms | 9.757 ms | 2.6225 ms | 2.3945 ms | 4.094 ms | 12.027 ms | 11.961 ms | 2.946 ms | SKIP | SKIP |
| 262144 | 5.41 ms | SKIP | 36.409 ms | 26.5 ms | 5.848 ms | **5.368 ms** | 9.318 ms | 34.845 ms | 35.195 ms | 7.064 ms | SKIP | SKIP |
| 524288 | 12.295 ms | SKIP | 110.78 ms | 79.362 ms | 13.206 ms | **12.173 ms** | 21.069 ms | 107.8 ms | 106.08 ms | 15.778 ms | SKIP | SKIP |
| 1048576 | **27.475 ms** | SKIP | 332.91 ms | 223.87 ms | 29.388 ms | 27.52 ms | 46.439 ms | 335.56 ms | 328.92 ms | 35.285 ms | SKIP | SKIP |
| 2097152 | **60.728 ms** | SKIP | 981 ms | 620.05 ms | 64.148 ms | 61.928 ms | 103.33 ms | 961.03 ms | 1.0069 s | 78.773 ms | SKIP | SKIP |
| 4194304 | **136.13 ms** | SKIP | 2.9746 s | 1.7645 s | 140.85 ms | 137.46 ms | 232.71 ms | 3.0455 s | 2.9511 s | 173.01 ms | SKIP | SKIP |

## Matched kernel controls

Minima below are scoped to these controls separately. Owned twiddles
can still retain matrix-leaf tables in this historical source.

| Decoded payload bytes | Forced matrix | Same-leaf owned NTT | Same-leaf cached NTT |
| ---: | ---: | ---: | ---: |
| 0 | SKIP | **7.8605 ns** | 8.0533 ns |
| 1 | 21.562 ns | 8.8714 ns | **8.8693 ns** |
| 8 | 21.491 ns | 12.742 ns | **12.063 ns** |
| 16 | 21.904 ns | 14.23 ns | **13.659 ns** |
| 21 | 26.927 ns | 16.093 ns | **15.954 ns** |
| 22 | 27.366 ns | 16.675 ns | **16.56 ns** |
| 23 | 23.608 ns | 17.679 ns | **17.587 ns** |
| 31 | 38.195 ns | **21.826 ns** | 21.85 ns |
| 32 | 34.925 ns | **18.615 ns** | 18.971 ns |
| 33 | 34.996 ns | **18.657 ns** | 19.283 ns |
| 63 | 64.3 ns | **34.687 ns** | 35.692 ns |
| 64 | 57.5 ns | 30.698 ns | **30.639 ns** |
| 65 | 56.447 ns | **30.647 ns** | 30.728 ns |
| 128 | 134.47 ns | 73.749 ns | **73.277 ns** |
| 256 | 251.63 ns | 194.21 ns | **193.29 ns** |
| 512 | **671.47 ns** | 702.93 ns | 703.73 ns |
| 1024 | **2.2281 µs** | 2.6928 µs | 2.652 µs |
| 2048 | **6.796 µs** | 10.006 µs | 9.885 µs |
| 4096 | **25.085 µs** | 33.893 µs | 30.165 µs |
| 8192 | SKIP | 84.235 µs | **77.489 µs** |
| 16384 | SKIP | 199.58 µs | **186.1 µs** |
| 32768 | SKIP | 484.25 µs | **449.11 µs** |
| 65536 | SKIP | 1.0837 ms | **1.0292 ms** |
| 131072 | SKIP | 2.4685 ms | **2.3855 ms** |
| 262144 | SKIP | 5.561 ms | **5.425 ms** |
| 524288 | SKIP | 12.58 ms | **12.303 ms** |
| 1048576 | SKIP | 27.882 ms | **27.504 ms** |
| 2097152 | SKIP | 62.579 ms | **61.332 ms** |
| 4194304 | SKIP | 139.22 ms | **135.18 ms** |

Exact ranges, inferred routes, bs58 ratios and skip reasons: [observations.csv](observations.csv).
Batched and bs58 stop at 64 KiB by policy. five8 supports 32/64 B only.
No unmeasured cells are interpolated.

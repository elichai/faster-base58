# gcp-native: decode

[Dataset scope and source metadata](README.md)

Historical 2026-10-24 snapshot. Median of three process medians.
Native clock: process CPU. Wasm clock: elapsed. Bold is observational,
not a significance test. `SKIP` is a recorded support/policy limit.

| Decoded payload bytes | Auto | Batched | Karatsuba | Toom-3 | NTT | Cached NTT | Portable NTT | Matrix | Portable matrix | Malachite | bs58 | five8 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 114.54 ns | 106.03 ns | 117.65 ns | 115.38 ns | 120.79 ns | 118.9 ns | 119.34 ns | 113.75 ns | 116.6 ns | 118.51 ns | **3.1468 ns** | SKIP |
| 1 | 8.9429 ns | 7.9928 ns | 10.159 ns | 9.7458 ns | 12.922 ns | 12.772 ns | 12.988 ns | 9.2137 ns | 9.1821 ns | 10.906 ns | **4.8859 ns** | SKIP |
| 8 | 14.115 ns | **13.555 ns** | 15.41 ns | 15.459 ns | 19.174 ns | 19.314 ns | 19.328 ns | 14.112 ns | 14.135 ns | 16.209 ns | 39.105 ns | SKIP |
| 16 | 19.842 ns | **19.415 ns** | 20.601 ns | 22.532 ns | 24.008 ns | 24.136 ns | 24.149 ns | 19.726 ns | 19.725 ns | 21.773 ns | 100.94 ns | SKIP |
| 21 | 22.851 ns | **22.326 ns** | 23.614 ns | 23.845 ns | 27.834 ns | 27.093 ns | 26.931 ns | 22.748 ns | 22.803 ns | 24.119 ns | 155.59 ns | SKIP |
| 22 | 26.436 ns | **26.362 ns** | 27.533 ns | 27.398 ns | 30.83 ns | 30.795 ns | 30.662 ns | 26.85 ns | 26.833 ns | 28.242 ns | 167.89 ns | SKIP |
| 23 | **31.291 ns** | 32.859 ns | 34.039 ns | 34.711 ns | 36.737 ns | 37.432 ns | 37.016 ns | 32.855 ns | 33.217 ns | 34.277 ns | 181.94 ns | SKIP |
| 31 | 37.917 ns | **37.333 ns** | 39.02 ns | 39.848 ns | 42.817 ns | 42.881 ns | 42.684 ns | 38.561 ns | 37.359 ns | 39.728 ns | 307.5 ns | SKIP |
| 32 | 25.815 ns | **25.271 ns** | 26.595 ns | 27.594 ns | 30.951 ns | 31.278 ns | 29.196 ns | 25.911 ns | 25.936 ns | 27.654 ns | 316.92 ns | 42.304 ns |
| 33 | 26.788 ns | 26.568 ns | 27.02 ns | 27.779 ns | 30.04 ns | 31.731 ns | 30.195 ns | **26.385 ns** | 26.473 ns | 29.137 ns | 337.33 ns | SKIP |
| 63 | 50.487 ns | **50.304 ns** | 51.426 ns | 51.434 ns | 55.341 ns | 54.871 ns | 54.698 ns | 50.569 ns | 50.9 ns | 51.921 ns | 1.1998 µs | SKIP |
| 64 | 46.858 ns | **46.291 ns** | 47.647 ns | 47.826 ns | 50.718 ns | 51.311 ns | 51.192 ns | 46.87 ns | 46.787 ns | 48.244 ns | 1.247 µs | 87.611 ns |
| 65 | 47.823 ns | 48.318 ns | 49.948 ns | 48.721 ns | 52.489 ns | 52.105 ns | 54.146 ns | 48.689 ns | **47.635 ns** | 49.929 ns | 1.2927 µs | SKIP |
| 128 | 106.56 ns | **103.43 ns** | 109.03 ns | 105.48 ns | 107.83 ns | 107.6 ns | 107.16 ns | 104.53 ns | 104.09 ns | 105.54 ns | 5.2816 µs | SKIP |
| 256 | 262.95 ns | **262.49 ns** | 263.97 ns | 263.39 ns | 267.16 ns | 268.01 ns | 267.41 ns | 263.11 ns | 263.47 ns | 264.09 ns | 21.911 µs | SKIP |
| 512 | 800.63 ns | **799.14 ns** | 801.3 ns | 800.93 ns | 809.06 ns | 804.68 ns | 806.48 ns | 1.0285 µs | 2.46 µs | 803.98 ns | 90.823 µs | SKIP |
| 1024 | 2.6992 µs | 2.6995 µs | **2.6973 µs** | 2.7009 µs | 2.7093 µs | 2.7029 µs | 2.7012 µs | 3.0155 µs | 9.3063 µs | 2.7069 µs | 373.73 µs | SKIP |
| 2048 | **9.8775 µs** | 9.8838 µs | 9.8944 µs | 9.8777 µs | 9.9358 µs | 10.143 µs | 9.8842 µs | 10.157 µs | 36.398 µs | 9.9302 µs | 1.4944 ms | SKIP |
| 4096 | 22.12 µs | 38.316 µs | **21.985 µs** | 22.113 µs | 22.239 µs | 22.481 µs | 22.119 µs | 37.721 µs | 155.16 µs | 50.455 µs | 5.9495 ms | SKIP |
| 8192 | 56.833 µs | 149.66 µs | 55.882 µs | 55.877 µs | 56.086 µs | 56.163 µs | 56.04 µs | **55.821 µs** | 56.075 µs | 128.18 µs | 23.723 ms | SKIP |
| 16384 | 158.12 µs | 584.13 µs | 156.03 µs | 157.44 µs | 156.66 µs | 157.54 µs | 156.19 µs | 156.22 µs | **155.98 µs** | 303.32 µs | 94.718 ms | SKIP |
| 32768 | 461.56 µs | 2.3058 ms | 459.22 µs | 462.94 µs | 460.56 µs | 456.49 µs | 456.89 µs | 456.92 µs | **456.4 µs** | 688.1 µs | 378.7 ms | SKIP |
| 65536 | 1.1665 ms | 9.2353 ms | 1.3641 ms | 1.3823 ms | 1.2247 ms | **1.164 ms** | 1.3501 ms | 1.361 ms | 1.3647 ms | 1.5946 ms | 1.5139 s | SKIP |
| 131072 | 2.6429 ms | SKIP | 4.1352 ms | 4.0431 ms | 3.0214 ms | **2.6366 ms** | 3.4272 ms | 4.1343 ms | 4.1189 ms | 3.8913 ms | SKIP | SKIP |
| 262144 | **5.8721 ms** | SKIP | 12.457 ms | 11.391 ms | 6.7122 ms | 5.8858 ms | 8.1703 ms | 12.488 ms | 12.439 ms | 8.6546 ms | SKIP | SKIP |
| 524288 | **13.094 ms** | SKIP | 37.709 ms | 32.322 ms | 14.575 ms | 13.115 ms | 18.979 ms | 37.704 ms | 37.663 ms | 19.019 ms | SKIP | SKIP |
| 1048576 | 28.972 ms | SKIP | 113.77 ms | 92.834 ms | 32.224 ms | **28.94 ms** | 43.476 ms | 113.84 ms | 113.74 ms | 41.429 ms | SKIP | SKIP |
| 2097152 | 65.23 ms | SKIP | 343 ms | 265.28 ms | 71.85 ms | **64.947 ms** | 98.205 ms | 342.77 ms | 342.95 ms | 90.291 ms | SKIP | SKIP |
| 4194304 | 143.4 ms | SKIP | 1.0341 s | 755.88 ms | 153.28 ms | **143.36 ms** | 219.86 ms | 1.034 s | 1.0375 s | 197.26 ms | SKIP | SKIP |

## Matched kernel controls

Minima below are scoped to these controls separately. Owned twiddles
can still retain matrix-leaf tables in this historical source.

| Decoded payload bytes | Forced matrix | Same-leaf owned NTT | Same-leaf cached NTT |
| ---: | ---: | ---: | ---: |
| 0 | SKIP | **119.55 ns** | 121.01 ns |
| 1 | 24.593 ns | **12.773 ns** | 12.774 ns |
| 8 | 28.829 ns | **19.241 ns** | 19.265 ns |
| 16 | 34.91 ns | **24.068 ns** | 24.096 ns |
| 21 | 38.857 ns | 27.359 ns | **27.066 ns** |
| 22 | 39.005 ns | **30.679 ns** | 30.753 ns |
| 23 | 39.823 ns | **37.199 ns** | 37.391 ns |
| 31 | 55.286 ns | **42.592 ns** | 42.936 ns |
| 32 | 55.597 ns | **29.145 ns** | 29.264 ns |
| 33 | 57.1 ns | 30.224 ns | **30.023 ns** |
| 63 | 85.499 ns | **54.703 ns** | 54.759 ns |
| 64 | 86.545 ns | **51.036 ns** | 51.517 ns |
| 65 | 88.151 ns | 52.284 ns | **52.112 ns** |
| 128 | 194.37 ns | **107.05 ns** | 107.37 ns |
| 256 | 396.17 ns | **266.79 ns** | 266.86 ns |
| 512 | 1.0262 µs | **804.43 ns** | 811.62 ns |
| 1024 | 3.0161 µs | **2.7035 µs** | 2.7066 µs |
| 2048 | 10.157 µs | 9.9214 µs | **9.8678 µs** |
| 4096 | 37.597 µs | **22.122 µs** | 22.496 µs |
| 8192 | SKIP | 56.401 µs | **56.202 µs** |
| 16384 | SKIP | **156.29 µs** | 158.92 µs |
| 32768 | SKIP | 456.4 µs | **456.19 µs** |
| 65536 | SKIP | 1.2361 ms | **1.1653 ms** |
| 131072 | SKIP | 3.0172 ms | **2.6288 ms** |
| 262144 | SKIP | 6.6885 ms | **5.8988 ms** |
| 524288 | SKIP | 14.609 ms | **13.153 ms** |
| 1048576 | SKIP | 32.198 ms | **28.997 ms** |
| 2097152 | SKIP | 71.803 ms | **64.982 ms** |
| 4194304 | SKIP | 153.61 ms | **144.08 ms** |

Exact ranges, inferred routes, bs58 ratios and skip reasons: [observations.csv](observations.csv).
Batched and bs58 stop at 64 KiB by policy. five8 supports 32/64 B only.
No unmeasured cells are interpolated.

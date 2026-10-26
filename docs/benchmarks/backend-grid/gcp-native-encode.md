# gcp-native: encode

[Dataset scope and source metadata](README.md)

Historical 2026-10-24 snapshot. Median of three process medians.
Native clock: process CPU. Wasm clock: elapsed. Bold is observational,
not a significance test. `SKIP` is a recorded support/policy limit.

| Decoded payload bytes | Auto | Batched | Karatsuba | Toom-3 | NTT | Cached NTT | Portable NTT | Matrix | Portable matrix | Malachite | bs58 | five8 |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 4.3678 ns | 4.124 ns | 5.3357 ns | 5.333 ns | 8.2653 ns | 8.4875 ns | 8.2991 ns | 5.3402 ns | 4.37 ns | 5.5782 ns | **2.9073 ns** | SKIP |
| 1 | 17.519 ns | 17.366 ns | 18.292 ns | 18.391 ns | 22.475 ns | 22.523 ns | 22.482 ns | 19.027 ns | 18.05 ns | 19.085 ns | **5.7106 ns** | SKIP |
| 8 | 29.567 ns | **29.489 ns** | 30.977 ns | 30.594 ns | 34.291 ns | 34.477 ns | 34.364 ns | 30.57 ns | 30.197 ns | 31.001 ns | 70.825 ns | SKIP |
| 16 | 41.687 ns | **41.475 ns** | 42.716 ns | 42.711 ns | 46.903 ns | 46.852 ns | 46.938 ns | 43.4 ns | 42.277 ns | 43.351 ns | 262.58 ns | SKIP |
| 21 | **42.274 ns** | 42.341 ns | 44.018 ns | 43.338 ns | 47.54 ns | 47.587 ns | 47.471 ns | 43.944 ns | 42.829 ns | 43.804 ns | 391.49 ns | SKIP |
| 22 | 47.725 ns | 48.305 ns | 49.337 ns | 49.426 ns | 53.671 ns | 53.687 ns | 55.055 ns | 47.521 ns | **46.986 ns** | 50.195 ns | 419.06 ns | SKIP |
| 23 | 47.572 ns | 55.219 ns | 56.463 ns | 56.389 ns | 60.559 ns | 61.648 ns | 60.561 ns | 47.77 ns | **46.755 ns** | 57.122 ns | 445.91 ns | SKIP |
| 31 | 54.363 ns | 64.182 ns | 65.36 ns | 65.357 ns | 69.961 ns | 69.537 ns | 69.64 ns | 54.058 ns | **53.247 ns** | 66.096 ns | 729.85 ns | SKIP |
| 32 | 31.686 ns | 64.288 ns | 65.556 ns | 65.494 ns | 70.137 ns | 69.609 ns | 69.64 ns | 31.632 ns | **31.385 ns** | 66.481 ns | 764.33 ns | 32.366 ns |
| 33 | 65.404 ns | **64.382 ns** | 65.527 ns | 65.768 ns | 70.046 ns | 69.71 ns | 69.729 ns | 65.089 ns | 65.308 ns | 66.261 ns | 815.7 ns | SKIP |
| 63 | 89.029 ns | 134.62 ns | 137.15 ns | 136.94 ns | 142.32 ns | 142.69 ns | 142.31 ns | **88.654 ns** | 88.831 ns | 137.99 ns | 2.8289 µs | SKIP |
| 64 | 32.107 ns | 135.52 ns | 137.5 ns | 137.14 ns | 143.19 ns | 144.66 ns | 142.8 ns | **31.905 ns** | 124.31 ns | 138.73 ns | 2.9578 µs | 57.409 ns |
| 65 | 101.61 ns | 139.79 ns | 142 ns | 141.92 ns | 147.15 ns | 146.94 ns | 146.98 ns | **101.17 ns** | 154.8 ns | 144.3 ns | 3.0711 µs | SKIP |
| 128 | **168.21 ns** | 370.32 ns | 370.92 ns | 370.31 ns | 374.86 ns | 374.36 ns | 374.16 ns | 168.35 ns | 265.32 ns | 371.12 ns | 12.801 µs | SKIP |
| 256 | **401.47 ns** | 1.158 µs | 1.1583 µs | 1.1574 µs | 1.1625 µs | 1.1616 µs | 1.1629 µs | 402.14 ns | 808.32 ns | 1.1583 µs | 54.177 µs | SKIP |
| 512 | **1.1012 µs** | 4.0522 µs | 4.0478 µs | 4.0481 µs | 4.0535 µs | 4.053 µs | 4.0547 µs | 1.1013 µs | 2.9759 µs | 4.0508 µs | 224.53 µs | SKIP |
| 1024 | **3.3705 µs** | 15.087 µs | 8.1368 µs | 8.1627 µs | 8.3139 µs | 5.8963 µs | 5.8706 µs | 3.3732 µs | 11.104 µs | 15.084 µs | 919.26 µs | SKIP |
| 2048 | 11.808 µs | 59.195 µs | 18.84 µs | 18.929 µs | 19.303 µs | 14.356 µs | 14.429 µs | **11.749 µs** | 43.434 µs | 53.686 µs | 3.7147 ms | SKIP |
| 4096 | 38.707 µs | 232.12 µs | 49.22 µs | 49.608 µs | 56.04 µs | **38.605 µs** | 38.668 µs | 38.743 µs | 38.738 µs | 158.71 µs | 14.882 ms | SKIP |
| 8192 | **110.19 µs** | 910.08 µs | 137.16 µs | 137.21 µs | 138.38 µs | 110.98 µs | 110.48 µs | 110.65 µs | 110.58 µs | 401.48 µs | 59.573 ms | SKIP |
| 16384 | **357.11 µs** | 3.6144 ms | 425.5 µs | 426.66 µs | 432.33 µs | 358.21 µs | 358.46 µs | 358.59 µs | 358.25 µs | 939.28 µs | 238.39 ms | SKIP |
| 32768 | **1.0534 ms** | 14.515 ms | 1.2657 ms | 1.2579 ms | 1.2511 ms | 1.0584 ms | 1.0557 ms | 1.0554 ms | 1.0553 ms | 2.0751 ms | 953.81 ms | SKIP |
| 65536 | 2.7715 ms | 57.958 ms | 3.8398 ms | 3.75 ms | 3.5615 ms | **2.7616 ms** | 3.1701 ms | 2.7634 ms | 2.768 ms | 4.5495 ms | 3.8161 s | SKIP |
| 131072 | **5.8322 ms** | SKIP | 11.634 ms | 10.831 ms | 7.5153 ms | 5.8453 ms | 7.9213 ms | 5.8521 ms | 5.8543 ms | 10.175 ms | SKIP | SKIP |
| 262144 | **12.572 ms** | SKIP | 35.193 ms | 30.291 ms | 15.775 ms | 12.605 ms | 18.453 ms | 12.599 ms | 12.611 ms | 22.285 ms | SKIP | SKIP |
| 524288 | 27.582 ms | SKIP | 105.92 ms | 85.095 ms | 34.189 ms | 27.559 ms | 41.84 ms | **27.557 ms** | 27.574 ms | 48.244 ms | SKIP | SKIP |
| 1048576 | 62.06 ms | SKIP | 320.59 ms | 244.15 ms | 74.231 ms | 62.171 ms | 95.042 ms | **61.878 ms** | 62.292 ms | 103.77 ms | SKIP | SKIP |
| 2097152 | **135.5 ms** | SKIP | 963.94 ms | 693.46 ms | 157.56 ms | 135.55 ms | 210.85 ms | 135.85 ms | 136.16 ms | 223.43 ms | SKIP | SKIP |
| 4194304 | 294.76 ms | SKIP | 2.9095 s | 1.9558 s | 337.02 ms | 295.85 ms | 459.81 ms | **294.47 ms** | 297.03 ms | 481.13 ms | SKIP | SKIP |

## Matched kernel controls

Minima below are scoped to these controls separately. Owned twiddles
can still retain matrix-leaf tables in this historical source.

| Decoded payload bytes | Forced matrix | Same-leaf owned NTT | Same-leaf cached NTT |
| ---: | ---: | ---: | ---: |
| 0 | SKIP | 8.5184 ns | **8.2586 ns** |
| 1 | 27.949 ns | **22.481 ns** | 22.539 ns |
| 8 | **33.794 ns** | 34.343 ns | 35.171 ns |
| 16 | **41.399 ns** | 46.94 ns | 47.862 ns |
| 21 | **46.977 ns** | 47.542 ns | 47.504 ns |
| 22 | **45.571 ns** | 53.669 ns | 53.638 ns |
| 23 | **45.575 ns** | 60.613 ns | 60.934 ns |
| 31 | **52.198 ns** | 69.601 ns | 69.98 ns |
| 32 | **50.853 ns** | 69.66 ns | 70.608 ns |
| 33 | **63.014 ns** | 69.969 ns | 70.066 ns |
| 63 | **86.256 ns** | 142.14 ns | 142.37 ns |
| 64 | **83.371 ns** | 143.45 ns | 142.57 ns |
| 65 | **99.232 ns** | 147.1 ns | 147.15 ns |
| 128 | **166.21 ns** | 374.87 ns | 375.12 ns |
| 256 | **399.94 ns** | 1.1623 µs | 1.1634 µs |
| 512 | **1.0967 µs** | 4.0523 µs | 4.0504 µs |
| 1024 | **3.3781 µs** | 5.8712 µs | 5.8816 µs |
| 2048 | **11.745 µs** | 14.303 µs | 14.417 µs |
| 4096 | 44.233 µs | **38.592 µs** | 38.594 µs |
| 8192 | 167.36 µs | **110.36 µs** | 110.41 µs |
| 16384 | SKIP | 358.22 µs | **357.82 µs** |
| 32768 | SKIP | 1.0592 ms | **1.059 ms** |
| 65536 | SKIP | 2.8896 ms | **2.7611 ms** |
| 131072 | SKIP | 6.1067 ms | **5.8663 ms** |
| 262144 | SKIP | 13.136 ms | **12.615 ms** |
| 524288 | SKIP | 28.685 ms | **27.611 ms** |
| 1048576 | SKIP | 67.29 ms | **61.938 ms** |
| 2097152 | SKIP | 143.84 ms | **135.85 ms** |
| 4194304 | SKIP | 311.48 ms | **296.27 ms** |

Exact ranges, inferred routes, bs58 ratios and skip reasons: [observations.csv](observations.csv).
Batched and bs58 stop at 64 KiB by policy. five8 supports 32/64 B only.
No unmeasured cells are interpolated.

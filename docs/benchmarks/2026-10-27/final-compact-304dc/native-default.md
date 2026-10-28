# Final compact comparison: native, default

Ordinary Auto consumer on source 304dc. Latencies are process CPU nanoseconds per conversion, formatted by magnitude. Each value is a median of five fresh-process warm medians. Each process has three 100 ms warm batches. Ratio is the median of five paired compact/wide ratios and can differ from the displayed-median quotient. Bold marks the lower observed median, without a significance claim.

| Payload | Wide encode | Compact encode | Ratio | Wide decode | Compact decode | Ratio |
|---|---:|---:|---:|---:|---:|---:|
| 0 B | 2.744 ns | **2.740 ns** | 1.000968 | **2.911 ns** | 2.918 ns | 1.002336 |
| 1 B | 3.173 ns | **3.165 ns** | 0.997673 | **3.161 ns** | 3.179 ns | 1.006155 |
| 8 B | **28.663 ns** | 28.982 ns | 1.011962 | 12.873 ns | **12.867 ns** | 1.001840 |
| 16 B | 40.721 ns | **40.469 ns** | 0.996917 | **18.565 ns** | 18.568 ns | 1.000806 |
| 21 B | 41.259 ns | **41.252 ns** | 0.999756 | 22.008 ns | **21.990 ns** | 0.998956 |
| 22 B | 48.301 ns | **48.261 ns** | 0.998083 | 29.413 ns | **22.026 ns** | 0.749066 |
| 23 B | 54.837 ns | **54.577 ns** | 0.995264 | 33.952 ns | **28.838 ns** | 0.842237 |
| 31 B | 63.267 ns | **63.051 ns** | 1.000033 | 38.997 ns | **38.386 ns** | 0.982052 |
| 32 B | 30.702 ns | **30.636 ns** | 0.997756 | **24.139 ns** | 24.174 ns | 1.001401 |
| 33 B | **63.095 ns** | 63.207 ns | 1.002143 | 24.425 ns | **24.371 ns** | 0.997520 |
| 63 B | **132.662 ns** | 132.731 ns | 1.001107 | 51.522 ns | **51.475 ns** | 0.999995 |
| 64 B | **33.606 ns** | 33.654 ns | 0.999383 | **46.332 ns** | 46.572 ns | 1.004787 |
| 65 B | **138.556 ns** | 138.584 ns | 0.999955 | **47.011 ns** | 47.020 ns | 1.000266 |
| 128 B | **367.235 ns** | 367.911 ns | 1.000513 | 103.813 ns | **103.640 ns** | 0.998528 |
| 256 B | **1.145 µs** | 1.145 µs | 1.000822 | **265.667 ns** | 265.685 ns | 1.000656 |
| 512 B | **4.043 µs** | 4.046 µs | 1.001537 | **815.283 ns** | 815.387 ns | 1.000251 |
| 1 KiB | 8.279 µs | **8.011 µs** | 0.964444 | 2.764 µs | **2.764 µs** | 0.999904 |
| 2 KiB | 19.759 µs | **18.615 µs** | 0.944056 | 10.184 µs | **10.182 µs** | 0.999174 |
| 4 KiB | 51.337 µs | **46.903 µs** | 0.909468 | **21.886 µs** | 22.230 µs | 1.017231 |
| 8 KiB | 140.936 µs | **131.859 µs** | 0.936514 | **56.285 µs** | 57.384 µs | 1.019273 |
| 16 KiB | 415.115 µs | **389.281 µs** | 0.937766 | **156.257 µs** | 159.766 µs | 1.023810 |
| 32 KiB | 1.245 ms | **1.171 ms** | 0.940836 | **455.836 µs** | 467.344 µs | 1.024609 |
| 64 KiB | 3.282 ms | **3.107 ms** | 0.946250 | **1.224 ms** | 1.241 ms | 1.013103 |
| 128 KiB | 7.351 ms | **7.054 ms** | 0.960475 | **2.748 ms** | 2.797 ms | 1.017969 |
| 256 KiB | 15.518 ms | **14.975 ms** | 0.964523 | **6.641 ms** | 6.747 ms | 1.016000 |
| 512 KiB | 33.133 ms | **32.244 ms** | 0.974212 | **14.675 ms** | 14.881 ms | 1.014082 |
| 1 MiB | 73.023 ms | **71.520 ms** | 0.981936 | **32.311 ms** | 32.801 ms | 1.017881 |
| 2 MiB | 154.777 ms | **150.937 ms** | 0.975190 | **71.389 ms** | 72.635 ms | 1.017217 |
| 4 MiB | 337.641 ms | **330.488 ms** | 0.974818 | **153.211 ms** | 155.818 ms | 1.017026 |

All five paired ratios remain in [feature-cells.csv](feature-cells.csv); [identical-content controls](identical-controls.md) and [all pairs](all-pairs.csv) are retained separately. Tiny conversions include call/context costs and do not execute IFMA.

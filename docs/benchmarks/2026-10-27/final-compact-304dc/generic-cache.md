# Final compact comparison: generic, cache

Ordinary Auto consumer on source 304dc. Latencies are process CPU nanoseconds per conversion, formatted by magnitude. Each value is a median of five fresh-process warm medians. Each process has three 100 ms warm batches. Ratio is the median of five paired compact/wide ratios and can differ from the displayed-median quotient. Bold marks the lower observed median, without a significance claim.

| Payload | Wide encode | Compact encode | Ratio | Wide decode | Compact decode | Ratio |
|---|---:|---:|---:|---:|---:|---:|
| 0 B | **2.552 ns** | 2.552 ns | 0.999986 | 1.816 ns | **1.816 ns** | 0.999751 |
| 1 B | **2.911 ns** | 2.917 ns | 1.001779 | 2.446 ns | **2.425 ns** | 0.991338 |
| 8 B | **28.580 ns** | 28.899 ns | 1.007691 | 13.004 ns | **12.268 ns** | 0.943696 |
| 16 B | 39.848 ns | **39.769 ns** | 0.998203 | 16.346 ns | **16.256 ns** | 0.994355 |
| 21 B | 40.645 ns | **40.488 ns** | 0.996145 | 19.822 ns | **19.787 ns** | 0.998157 |
| 22 B | 46.719 ns | **46.711 ns** | 0.999627 | 20.565 ns | **20.536 ns** | 0.998669 |
| 23 B | 46.717 ns | **46.678 ns** | 0.999869 | 22.299 ns | **22.290 ns** | 0.999449 |
| 31 B | **53.363 ns** | 53.457 ns | 1.000537 | 28.337 ns | **26.228 ns** | 0.925918 |
| 32 B | **30.163 ns** | 30.205 ns | 1.000750 | **25.236 ns** | 25.284 ns | 1.001358 |
| 33 B | **63.669 ns** | 63.947 ns | 1.004375 | **25.675 ns** | 25.680 ns | 0.999630 |
| 63 B | **87.262 ns** | 88.189 ns | 1.010648 | **48.866 ns** | 49.240 ns | 1.007862 |
| 64 B | 31.268 ns | **31.233 ns** | 0.998867 | 47.711 ns | **47.610 ns** | 0.998626 |
| 65 B | 102.958 ns | **101.922 ns** | 0.989573 | 47.893 ns | **47.742 ns** | 0.996862 |
| 128 B | 169.355 ns | **166.841 ns** | 0.983410 | 110.352 ns | **110.182 ns** | 0.999505 |
| 256 B | 396.658 ns | **393.828 ns** | 0.997039 | **301.841 ns** | 305.300 ns | 1.011458 |
| 512 B | **1.098 µs** | 1.098 µs | 0.998334 | **1.003 µs** | 1.004 µs | 1.001401 |
| 1 KiB | 3.383 µs | **3.381 µs** | 0.999876 | 2.901 µs | **2.885 µs** | 0.995368 |
| 2 KiB | 12.090 µs | **11.878 µs** | 0.993726 | 10.118 µs | **10.052 µs** | 0.994441 |
| 4 KiB | **40.787 µs** | 41.226 µs | 1.013158 | **27.311 µs** | 28.019 µs | 1.025414 |
| 8 KiB | **114.379 µs** | 117.430 µs | 1.025092 | **64.822 µs** | 66.700 µs | 1.027391 |
| 16 KiB | **343.232 µs** | 352.786 µs | 1.026122 | **171.316 µs** | 175.409 µs | 1.024696 |
| 32 KiB | **1.045 ms** | 1.076 ms | 1.029748 | **477.127 µs** | 490.596 µs | 1.028833 |
| 64 KiB | **2.749 ms** | 2.790 ms | 1.015936 | **1.207 ms** | 1.223 ms | 1.013358 |
| 128 KiB | **5.925 ms** | 6.032 ms | 1.017911 | **2.735 ms** | 2.796 ms | 1.020981 |
| 256 KiB | **12.915 ms** | 13.122 ms | 1.017352 | **6.121 ms** | 6.253 ms | 1.021111 |
| 512 KiB | **28.290 ms** | 28.723 ms | 1.016120 | **13.612 ms** | 13.889 ms | 1.020392 |
| 1 MiB | **68.003 ms** | 68.675 ms | 1.013342 | **30.177 ms** | 30.821 ms | 1.019935 |
| 2 MiB | **146.800 ms** | 148.326 ms | 1.010470 | **74.905 ms** | 76.214 ms | 1.019030 |
| 4 MiB | **323.058 ms** | 326.060 ms | 1.011741 | **159.648 ms** | 162.423 ms | 1.017384 |

All five paired ratios remain in [feature-cells.csv](feature-cells.csv); [identical-content controls](identical-controls.md) and [all pairs](all-pairs.csv) are retained separately. Tiny conversions include call/context costs and do not execute IFMA.

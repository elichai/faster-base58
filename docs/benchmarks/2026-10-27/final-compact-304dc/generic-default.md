# Final compact comparison: generic, default

Ordinary Auto consumer on source 304dc. Latencies are process CPU nanoseconds per conversion, formatted by magnitude. Each value is a median of five fresh-process warm medians. Each process has three 100 ms warm batches. Ratio is the median of five paired compact/wide ratios and can differ from the displayed-median quotient. Bold marks the lower observed median, without a significance claim.

| Payload | Wide encode | Compact encode | Ratio | Wide decode | Compact decode | Ratio |
|---|---:|---:|---:|---:|---:|---:|
| 0 B | **2.507 ns** | 2.749 ns | 1.096773 | 2.907 ns | **2.907 ns** | 0.999902 |
| 1 B | 3.198 ns | **3.048 ns** | 0.955040 | 3.268 ns | **3.266 ns** | 0.999826 |
| 8 B | 28.530 ns | **28.347 ns** | 0.993492 | **11.598 ns** | 11.600 ns | 0.999919 |
| 16 B | 39.877 ns | **39.759 ns** | 0.997819 | **15.800 ns** | 15.819 ns | 1.000390 |
| 21 B | 41.108 ns | **40.934 ns** | 0.995594 | 19.541 ns | **19.541 ns** | 1.000684 |
| 22 B | 46.787 ns | **46.752 ns** | 0.998831 | **19.879 ns** | 19.884 ns | 1.000202 |
| 23 B | 53.794 ns | **53.561 ns** | 0.996847 | 23.719 ns | **21.765 ns** | 0.916393 |
| 31 B | 62.768 ns | **62.764 ns** | 0.999484 | **25.925 ns** | 25.931 ns | 1.001430 |
| 32 B | **30.432 ns** | 30.433 ns | 1.000053 | **24.906 ns** | 24.910 ns | 0.999503 |
| 33 B | **63.080 ns** | 63.081 ns | 1.000612 | **25.494 ns** | 25.504 ns | 0.999765 |
| 63 B | 135.531 ns | **135.382 ns** | 0.998834 | **49.271 ns** | 49.296 ns | 0.999644 |
| 64 B | 33.564 ns | **33.424 ns** | 1.004503 | **47.966 ns** | 47.968 ns | 0.999641 |
| 65 B | 140.300 ns | **140.202 ns** | 1.000140 | 48.047 ns | **48.037 ns** | 1.000132 |
| 128 B | **377.320 ns** | 383.574 ns | 1.000399 | 110.761 ns | **110.739 ns** | 0.999966 |
| 256 B | 1.184 µs | **1.184 µs** | 1.000198 | **304.037 ns** | 305.914 ns | 1.005582 |
| 512 B | **4.238 µs** | 4.242 µs | 1.001048 | 1.009 µs | **1.005 µs** | 0.998925 |
| 1 KiB | 8.581 µs | **8.407 µs** | 0.976331 | **3.775 µs** | 3.796 µs | 1.005368 |
| 2 KiB | 20.793 µs | **19.629 µs** | 0.943250 | **14.332 µs** | 14.371 µs | 1.002642 |
| 4 KiB | 53.754 µs | **49.645 µs** | 0.924284 | **33.172 µs** | 33.503 µs | 1.010433 |
| 8 KiB | 147.185 µs | **140.289 µs** | 0.952585 | **76.078 µs** | 76.980 µs | 1.011659 |
| 16 KiB | 427.661 µs | **404.844 µs** | 0.946824 | **191.880 µs** | 195.242 µs | 1.018132 |
| 32 KiB | 1.248 ms | **1.182 ms** | 0.945247 | **519.586 µs** | 530.568 µs | 1.021385 |
| 64 KiB | 3.321 ms | **3.166 ms** | 0.954280 | **1.365 ms** | 1.380 ms | 1.011273 |
| 128 KiB | 7.532 ms | **7.267 ms** | 0.965185 | **3.055 ms** | 3.103 ms | 1.014940 |
| 256 KiB | 15.961 ms | **15.612 ms** | 0.974883 | **7.328 ms** | 7.461 ms | 1.017629 |
| 512 KiB | 34.410 ms | **33.605 ms** | 0.975938 | **15.977 ms** | 16.207 ms | 1.017676 |
| 1 MiB | 75.586 ms | **74.019 ms** | 0.980540 | **34.930 ms** | 35.467 ms | 1.014533 |
| 2 MiB | 160.405 ms | **157.274 ms** | 0.979462 | **77.459 ms** | 78.526 ms | 1.011058 |
| 4 MiB | 347.883 ms | **342.486 ms** | 0.981802 | **164.867 ms** | 167.434 ms | 1.015573 |

All five paired ratios remain in [feature-cells.csv](feature-cells.csv); [identical-content controls](identical-controls.md) and [all pairs](all-pairs.csv) are retained separately. Tiny conversions include call/context costs and do not execute IFMA.

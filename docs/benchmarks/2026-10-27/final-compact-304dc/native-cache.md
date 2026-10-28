# Final compact comparison: native, cache

Ordinary Auto consumer on source 304dc. Latencies are process CPU nanoseconds per conversion, formatted by magnitude. Each value is a median of five fresh-process warm medians. Each process has three 100 ms warm batches. Ratio is the median of five paired compact/wide ratios and can differ from the displayed-median quotient. Bold marks the lower observed median, without a significance claim.

| Payload | Wide encode | Compact encode | Ratio | Wide decode | Compact decode | Ratio |
|---|---:|---:|---:|---:|---:|---:|
| 0 B | **2.425 ns** | 2.434 ns | 1.002741 | 1.452 ns | **1.210 ns** | 0.833631 |
| 1 B | **2.669 ns** | 2.674 ns | 1.001798 | **1.888 ns** | 1.949 ns | 1.032335 |
| 8 B | 28.552 ns | **28.521 ns** | 0.998928 | **12.809 ns** | 12.838 ns | 1.001511 |
| 16 B | 40.441 ns | **40.370 ns** | 0.998917 | 19.295 ns | **19.155 ns** | 0.992285 |
| 21 B | **41.233 ns** | 41.234 ns | 0.999478 | 22.592 ns | **21.647 ns** | 0.957797 |
| 22 B | **45.162 ns** | 45.200 ns | 1.001668 | 23.955 ns | **23.902 ns** | 1.000174 |
| 23 B | **45.170 ns** | 45.214 ns | 1.001490 | **25.292 ns** | 29.823 ns | 1.178468 |
| 31 B | **51.625 ns** | 51.772 ns | 1.004303 | 38.608 ns | **38.532 ns** | 0.998050 |
| 32 B | **30.499 ns** | 30.555 ns | 1.001931 | **24.704 ns** | 26.521 ns | 1.063588 |
| 33 B | **60.362 ns** | 60.415 ns | 1.001725 | **25.016 ns** | 25.080 ns | 1.001958 |
| 63 B | **84.344 ns** | 84.483 ns | 1.001654 | **50.320 ns** | 51.301 ns | 1.019850 |
| 64 B | 31.615 ns | **31.610 ns** | 1.000639 | **46.086 ns** | 47.433 ns | 1.029023 |
| 65 B | 96.853 ns | **96.354 ns** | 1.000081 | **46.799 ns** | 47.001 ns | 1.003928 |
| 128 B | **158.473 ns** | 159.308 ns | 1.006213 | 104.669 ns | **104.659 ns** | 1.000554 |
| 256 B | 383.819 ns | **375.218 ns** | 0.977934 | **267.381 ns** | 272.743 ns | 1.019722 |
| 512 B | **1.049 µs** | 1.049 µs | 1.000254 | 818.897 ns | **816.656 ns** | 1.005468 |
| 1 KiB | **3.252 µs** | 3.253 µs | 1.000021 | **2.743 µs** | 2.745 µs | 1.000109 |
| 2 KiB | 11.444 µs | **11.436 µs** | 0.998991 | **10.041 µs** | 10.050 µs | 1.001324 |
| 4 KiB | **38.732 µs** | 38.911 µs | 1.004279 | **21.959 µs** | 22.458 µs | 1.024514 |
| 8 KiB | **110.446 µs** | 112.744 µs | 1.021161 | **55.719 µs** | 57.371 µs | 1.029004 |
| 16 KiB | **337.874 µs** | 346.349 µs | 1.024925 | **155.526 µs** | 159.765 µs | 1.026710 |
| 32 KiB | **1.047 ms** | 1.076 ms | 1.027104 | **455.908 µs** | 467.808 µs | 1.026790 |
| 64 KiB | **2.750 ms** | 2.786 ms | 1.011637 | **1.160 ms** | 1.176 ms | 1.014149 |
| 128 KiB | **5.832 ms** | 5.924 ms | 1.016653 | **2.623 ms** | 2.672 ms | 1.018454 |
| 256 KiB | **12.561 ms** | 12.784 ms | 1.018739 | **5.862 ms** | 5.986 ms | 1.022524 |
| 512 KiB | **27.361 ms** | 27.859 ms | 1.020260 | **13.049 ms** | 13.312 ms | 1.020073 |
| 1 MiB | **65.534 ms** | 66.547 ms | 1.015457 | **29.060 ms** | 29.494 ms | 1.017147 |
| 2 MiB | **140.980 ms** | 143.435 ms | 1.015546 | **71.580 ms** | 72.553 ms | 1.006910 |
| 4 MiB | **311.439 ms** | 315.978 ms | 1.016850 | **154.002 ms** | 155.768 ms | 1.010900 |

All five paired ratios remain in [feature-cells.csv](feature-cells.csv); [identical-content controls](identical-controls.md) and [all pairs](all-pairs.csv) are retained separately. Tiny conversions include call/context costs and do not execute IFMA.

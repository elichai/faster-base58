### Apple M5 Pro

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 11.9 ns / 15.2 ns | 11.5 ns / **14.1 ns** | **9.79 ns** / 14.4 ns | 779 ns / 324 ns | 33.3 ns / 31.2 ns |
| 64 B | 24.4 ns / 27.2 ns | 24.3 ns / **27.2 ns** | **24 ns** / 28 ns | 3.08 µs / 1.17 µs | 76.9 ns / 94.1 ns |
| 512 B | 2.16 µs / 671 ns | **566 ns** / **595 ns** | 2.18 µs / 668 ns | 239 µs / 75.8 µs | — / — |
| 4 KiB | 110 µs / 39.9 µs | **28.3 µs** / **25.2 µs** | 109 µs / 38.6 µs | 15.1 ms / 5.34 ms | — / — |
| 64 KiB | 2.77 ms / 1.15 ms | **2.31 ms** / **1.01 ms** | 24.9 ms / 8.89 ms | 3.9 s / 1.37 s | — / — |
| 1 MiB | 62.9 ms / 28.3 ms | **58 ms** / **27.4 ms** | — / — | — / — | — / — |
| 4 MiB | 298 ms / 142 ms | **280 ms** / **139 ms** | — / — | — / — | — / — |

Each cell is **encode / decode**. Bold marks the lowest observed median, not statistical significance.

### WebAssembly on Apple M5 Pro (Node/V8)

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 25.7 ns / 54.7 ns | 25.6 ns / 53.8 ns | **25.5 ns** / 50.6 ns | 635 ns / 212 ns | 44.8 ns / **41.2 ns** |
| 64 B | 99.1 ns / 133 ns | 99.6 ns / 133 ns | 101 ns / **126 ns** | 2.87 µs / 833 ns | **94 ns** / 127 ns |
| 512 B | 7.76 µs / 5.04 µs | **2.84 µs** / **2.65 µs** | 7.71 µs / 5 µs | 237 µs / 72.1 µs | — / — |
| 4 KiB | 489 µs / 217 µs | **149 µs** / **124 µs** | 486 µs / 281 µs | 15.4 ms / 5.35 ms | — / — |
| 64 KiB | 14.1 ms / 6.58 ms | **14 ms** / **6.45 ms** | 116 ms / 70.6 ms | 3.9 s / 1.31 s | — / — |
| 1 MiB | 339 ms / 168 ms | **339 ms** / **167 ms** | — / — | — / — | — / — |
| 4 MiB | **1.61 s** / 819 ms | 1.62 s / **818 ms** | — / — | — / — | — / — |

Each cell is **encode / decode**. Bold marks the lowest observed median, not statistical significance.

### Linked budgets above the empty probe

| Consumer | M5 code / readonly | Wasm code / data section |
|---|---:|---:|
| Auto | 58.5 KiB / 22.6 KiB | 70.1 KiB / 14.4 KiB |
| Auto + cache | 60.8 KiB / 25.0 KiB | 71.8 KiB / 15.2 KiB |
| No allocation | 9.6 KiB / 4.1 KiB | 13.5 KiB / 9.6 KiB |
| bs58 | 1.3 KiB / 0.9 KiB | 2.7 KiB / 0.8 KiB |

Both API directions, opt3/fat LTO. These linked probe deltas include associated code and metadata; they are not standalone rlib sizes.
Cache adds 6,280 native writable static header bytes plus history-dependent heap tables. Heap and RSS are reported separately.

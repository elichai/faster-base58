| Host / operation (4 KiB) | Auto | Auto + `cache` | `no_alloc` | bs58 |
| --- | ---: | ---: | ---: | ---: |
| M5 Pro / encode | 105.31 µs | **28.68 µs** | 106.48 µs | 15.23 ms |
| M5 Pro / decode | 38.37 µs | **24.89 µs** | 37.29 µs | 5.01 ms |
| GCP AMD EPYC 9B45 (generic) / encode | 51.45 µs | **41.27 µs** | 247.16 µs | 18.05 ms |
| GCP AMD EPYC 9B45 (generic) / decode | **27.46 µs** | 33.39 µs | 72.40 µs | 5.87 ms |
| GCP AMD EPYC 9B45 (native) / encode | 49.47 µs | **38.88 µs** | 228.19 µs | 14.90 ms |
| GCP AMD EPYC 9B45 (native) / decode | **22.03 µs** | 22.37 µs | 38.56 µs | 5.97 ms |
| Wasm / V8 on M5 Pro / encode | 458.00 µs | **149.54 µs** | 465.10 µs | 15.75 ms |
| Wasm / V8 on M5 Pro / decode | 214.61 µs | **125.10 µs** | 293.05 µs | 5.06 ms |

| Target | Auto | Auto + `cache` | `no_alloc` | bs58 |
| --- | ---: | ---: | ---: | ---: |
| M5 Pro | 360.4 | 376.5 | 295.8 | 295.7 |
| GCP AMD EPYC 9B45 (generic) | 602.0 | 600.1 | 317.6 | 284.3 |
| GCP AMD EPYC 9B45 (native) | 584.1 | 572.3 | 307.8 | 285.6 |
| Wasm / V8 on M5 Pro | 101.4 | 103.5 | 39.2 | 19.6 |

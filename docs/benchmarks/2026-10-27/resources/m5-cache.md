# Apple M5 Pro: cache

All sizes are exact requested Rust layouts. The allocation sample is one fresh process/instance per backend and payload; the warm window follows its cold call. Caller input/output and independent fixture storage are outside each operation window. Retained heap is table storage, separate from caller output. It can increase after other input sizes. Peak extra is additional live requested heap above the start of the cold operation, including new caches. Allocator metadata, resident pages and stack are excluded.

Linked sizes use both conversion directions, opt3/fat LTO, and the corresponding empty consumer as a control. They are deployment probe deltas, not standalone crate or rlib sizes. Internal wrappers enable `__internals`; public Auto/no_alloc and competitors do not. The empty scaffold is reported in the CSV.

| Backend | Sample | Code | Readonly / data section | Cold peak extra: encode / decode | Retained: encode / decode | Allocation calls: encode / decode |
|---|---:|---:|---:|---:|---:|---:|
| auto | 4.00 MiB | 60.76 KiB | 25.03 KiB | 204.44 MiB / 105.51 MiB | 13.73 MiB / 602.25 KiB | 46 / 30 |
| no-alloc | 64.00 KiB | 9.67 KiB | 4.10 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| batched | 64.00 KiB | 5.17 KiB | 1.48 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| karatsuba | 4.00 MiB | 29.61 KiB | 14.32 KiB | 36.67 MiB / 20.84 MiB | 0 B / 0 B | 6 / 1 |
| toom3 | 4.00 MiB | 34.48 KiB | 14.80 KiB | 36.67 MiB / 20.84 MiB | 0 B / 0 B | 6 / 1 |
| ntt | 4.00 MiB | 51.63 KiB | 20.65 KiB | 192.24 MiB / 104.90 MiB | 0 B / 0 B | 44 / 27 |
| ntt-cached | 4.00 MiB | 56.20 KiB | 21.88 KiB | 204.44 MiB / 105.51 MiB | 13.73 MiB / 602.25 KiB | 46 / 30 |
| ntt-portable | 4.00 MiB | 59.07 KiB | 20.67 KiB | 204.44 MiB / 105.51 MiB | 13.73 MiB / 602.25 KiB | 46 / 30 |
| malachite | 4.00 MiB | 325.51 KiB | 662.91 KiB | 86.85 MiB / 68.79 MiB | 0 B / 0 B | 53042 / 18048 |
| matrix | 4.00 MiB | 58.07 KiB | 21.99 KiB | 204.44 MiB / 20.84 MiB | 13.73 MiB / 0 B | 46 / 1 |
| matrix-portable | 4.00 MiB | 58.38 KiB | 22.29 KiB | 204.44 MiB / 20.84 MiB | 13.73 MiB / 0 B | 46 / 1 |
| bs58 | 64.00 KiB | 1.38 KiB | 950 B | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| five8 | 64 B | 11.75 KiB | 1.97 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| matrix-forced | 2.00 KiB | 18.32 KiB | 12.26 KiB | 734.78 KiB / 618.75 KiB | 712.50 KiB / 602.25 KiB | 3 / 3 |
| ntt-same-owned | 4.00 MiB | 56.13 KiB | 21.88 KiB | 204.44 MiB / 105.51 MiB | 13.73 MiB / 602.25 KiB | 46 / 30 |
| ntt-same-cached | 4.00 MiB | 56.20 KiB | 21.88 KiB | 204.44 MiB / 105.51 MiB | 13.73 MiB / 602.25 KiB | 46 / 30 |

The `matrix` decoder uses Karatsuba above its matrix domain; the matrix encoder uses NTT above its domain. Small recursive wrappers use batched kernels. `ntt-portable` permits compiler auto-vectorization. `ntt-same-owned` and `ntt-same-cached` isolate twiddle policy while keeping leaf policy equal; feature-enabled matrix leaves can still retain their own tables in both controls. These routes are listed per payload in the timing CSV.

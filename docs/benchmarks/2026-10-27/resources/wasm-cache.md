# WebAssembly: cache

All sizes are exact requested Rust layouts. The allocation sample is one fresh process/instance per backend and payload; the warm window follows its cold call. Caller input/output and independent fixture storage are outside each operation window. Retained heap is table storage, separate from caller output. It can increase after other input sizes. Peak extra is additional live requested heap above the start of the cold operation, including new caches. Allocator metadata, resident pages and stack are excluded.

Linked sizes use both conversion directions, opt3/fat LTO, and the corresponding empty consumer as a control. They are deployment probe deltas, not standalone crate or rlib sizes. Internal wrappers enable `__internals`; public Auto/no_alloc and competitors do not. The empty scaffold is reported in the CSV.

| Backend | Sample | Code | Readonly / data section | Cold peak extra: encode / decode | Retained: encode / decode | Allocation calls: encode / decode |
|---|---:|---:|---:|---:|---:|---:|
| auto | 4.00 MiB | 71.80 KiB | 15.24 KiB | 192.24 MiB / 104.90 MiB | 0 B / 0 B | 44 / 27 |
| no-alloc | 64.00 KiB | 13.55 KiB | 9.59 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| batched | 64.00 KiB | 8.37 KiB | 1.22 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| karatsuba | 4.00 MiB | 35.94 KiB | 10.75 KiB | 36.67 MiB / 20.84 MiB | 0 B / 0 B | 6 / 1 |
| toom3 | 4.00 MiB | 43.49 KiB | 10.98 KiB | 36.67 MiB / 20.84 MiB | 0 B / 0 B | 6 / 1 |
| ntt | 4.00 MiB | 63.03 KiB | 13.00 KiB | 192.24 MiB / 104.90 MiB | 0 B / 0 B | 44 / 27 |
| ntt-cached | 4.00 MiB | 63.03 KiB | 13.00 KiB | 192.24 MiB / 104.90 MiB | 0 B / 0 B | 44 / 27 |
| ntt-portable | 4.00 MiB | 63.03 KiB | 13.00 KiB | 192.24 MiB / 104.90 MiB | 0 B / 0 B | 44 / 27 |
| malachite | 4.00 MiB | 480.80 KiB | 638.06 KiB | 86.85 MiB / 68.79 MiB | 0 B / 0 B | 53042 / 18048 |
| matrix | 4.00 MiB | 66.82 KiB | 13.46 KiB | 192.24 MiB / 20.84 MiB | 0 B / 0 B | 44 / 1 |
| matrix-portable | 4.00 MiB | 66.82 KiB | 13.46 KiB | 192.24 MiB / 20.84 MiB | 0 B / 0 B | 44 / 1 |
| bs58 | 64.00 KiB | 2.75 KiB | 776 B | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| five8 | 64 B | 10.00 KiB | 1.62 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| matrix-forced | 2.00 KiB | 19.51 KiB | 9.66 KiB | 734.28 KiB / 618.06 KiB | 712.00 KiB / 601.56 KiB | 3 / 3 |
| ntt-same-owned | 4.00 MiB | 63.02 KiB | 13.00 KiB | 192.24 MiB / 104.90 MiB | 0 B / 0 B | 44 / 27 |
| ntt-same-cached | 4.00 MiB | 63.02 KiB | 13.00 KiB | 192.24 MiB / 104.90 MiB | 0 B / 0 B | 44 / 27 |

The `matrix` decoder uses Karatsuba above its matrix domain; the matrix encoder uses NTT above its domain. Small recursive wrappers use batched kernels. `ntt-portable` permits compiler auto-vectorization. `ntt-same-owned` and `ntt-same-cached` isolate twiddle policy while keeping leaf policy equal; feature-enabled matrix leaves can still retain their own tables in both controls. These routes are listed per payload in the timing CSV.

# AMD EPYC 9B45 / c4d-highcpu-4 · gcp-native: cache

All sizes are exact requested Rust layouts. The allocation sample is one fresh process/instance per backend and payload; the warm window follows its cold call. Caller input/output and independent fixture storage are outside each operation window. Retained heap is table storage, separate from caller output. It can increase after other input sizes. Peak extra is additional live requested heap above the start of the cold operation, including new caches. Allocator metadata, resident pages and stack are excluded.

Linked sizes use both conversion directions, opt3/fat LTO, and the corresponding empty consumer as a control. They are deployment probe deltas, not standalone crate or rlib sizes. Internal wrappers enable `__internals`; public Auto/no_alloc and competitors do not. The empty scaffold is reported in the CSV.

| Backend | Sample | Code above empty | Readonly above empty | Cold peak extra: encode / decode | Retained: encode / decode | Allocation calls: encode / decode |
|---|---:|---:|---:|---:|---:|---:|
| auto | 4.00 MiB | 249.33 KiB | 29.42 KiB | 190.56 MiB / 104.72 MiB | 228.50 KiB / 0 B | 41 / 22 |
| no-alloc | 64.00 KiB | 14.77 KiB | 10.94 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| batched | 64.00 KiB | 8.06 KiB | 1.45 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| karatsuba | 4.00 MiB | 188.23 KiB | 18.49 KiB | 37.61 MiB / 20.84 MiB | 0 B / 0 B | 6 / 1 |
| toom3 | 4.00 MiB | 199.51 KiB | 19.42 KiB | 37.61 MiB / 20.84 MiB | 0 B / 0 B | 6 / 1 |
| ntt | 4.00 MiB | 241.95 KiB | 26.38 KiB | 191.89 MiB / 104.72 MiB | 0 B / 0 B | 35 / 22 |
| ntt-cached | 4.00 MiB | 243.01 KiB | 26.59 KiB | 190.56 MiB / 104.72 MiB | 228.50 KiB / 0 B | 41 / 22 |
| ntt-portable | 4.00 MiB | 229.33 KiB | 25.77 KiB | 190.56 MiB / 104.72 MiB | 228.50 KiB / 0 B | 41 / 22 |
| malachite | 4.00 MiB | 386.20 KiB | 668.88 KiB | 86.85 MiB / 68.79 MiB | 0 B / 0 B | 53042 / 18048 |
| matrix | 4.00 MiB | 247.79 KiB | 27.75 KiB | 190.56 MiB / 20.84 MiB | 228.50 KiB / 0 B | 41 / 1 |
| matrix-portable | 4.00 MiB | 249.81 KiB | 28.57 KiB | 190.56 MiB / 20.84 MiB | 228.50 KiB / 0 B | 41 / 1 |
| bs58 | 64.00 KiB | 2.39 KiB | 1.12 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| five8 | 64 B | 10.59 KiB | 3.77 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| matrix-forced | 2.00 KiB | 169.09 KiB | 16.01 KiB | 734.78 KiB / 618.75 KiB | 712.50 KiB / 602.25 KiB | 3 / 3 |
| ntt-same-owned | 4.00 MiB | 243.01 KiB | 26.59 KiB | 190.56 MiB / 104.72 MiB | 228.50 KiB / 0 B | 41 / 22 |
| ntt-same-cached | 4.00 MiB | 243.01 KiB | 26.59 KiB | 190.56 MiB / 104.72 MiB | 228.50 KiB / 0 B | 41 / 22 |

The `matrix` decoder uses Karatsuba above its matrix domain; the matrix encoder uses NTT above its domain. Small recursive wrappers use batched kernels. `ntt-portable` permits compiler auto-vectorization and still uses normal IFMA short-product fallback. `ntt-cached` is a legacy hook name: with cache off, it owns twiddles per call. `matrix-portable` can route above its domain to normal NTT kernels. `ntt-same-owned` and `ntt-same-cached` isolate twiddle policy while keeping leaf policy equal; feature-enabled matrix leaves can still retain their own tables in both controls. These routes are listed per payload in the timing CSV.

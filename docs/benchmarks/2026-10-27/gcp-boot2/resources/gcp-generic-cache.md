# AMD EPYC 9B45 / c4d-highcpu-4 · gcp-generic: cache

All sizes are exact requested Rust layouts. The allocation sample is one fresh process/instance per backend and payload; the warm window follows its cold call. Caller input/output and independent fixture storage are outside each operation window. Retained heap is table storage, separate from caller output. It can increase after other input sizes. Peak extra is additional live requested heap above the start of the cold operation, including new caches. Allocator metadata, resident pages and stack are excluded.

Linked sizes use both conversion directions, opt3/fat LTO, and the corresponding empty consumer as a control. They are deployment probe deltas, not standalone crate or rlib sizes. Internal wrappers enable `__internals`; public Auto/no_alloc and competitors do not. The empty scaffold is reported in the CSV.

| Backend | Sample | Code above empty | Readonly above empty | Cold peak extra: encode / decode | Retained: encode / decode | Allocation calls: encode / decode |
|---|---:|---:|---:|---:|---:|---:|
| auto | 4.00 MiB | 270.67 KiB | 35.61 KiB | 190.56 MiB / 104.72 MiB | 228.50 KiB / 0 B | 41 / 22 |
| no-alloc | 64.00 KiB | 22.02 KiB | 14.41 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| batched | 64.00 KiB | 6.75 KiB | 1.39 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| karatsuba | 4.00 MiB | 179.53 KiB | 17.26 KiB | 37.61 MiB / 20.84 MiB | 0 B / 0 B | 6 / 1 |
| toom3 | 4.00 MiB | 185.22 KiB | 18.07 KiB | 37.61 MiB / 20.84 MiB | 0 B / 0 B | 6 / 1 |
| ntt | 4.00 MiB | 251.04 KiB | 27.93 KiB | 191.89 MiB / 104.72 MiB | 0 B / 0 B | 35 / 22 |
| ntt-cached | 4.00 MiB | 252.08 KiB | 28.12 KiB | 190.56 MiB / 104.72 MiB | 228.50 KiB / 0 B | 41 / 22 |
| ntt-portable | 4.00 MiB | 210.75 KiB | 23.31 KiB | 190.56 MiB / 104.72 MiB | 228.50 KiB / 0 B | 41 / 22 |
| malachite | 4.00 MiB | 1.18 MiB | 682.17 KiB | 86.85 MiB / 68.79 MiB | 0 B / 0 B | 53042 / 18048 |
| matrix | 4.00 MiB | 256.87 KiB | 29.41 KiB | 190.56 MiB / 20.84 MiB | 228.50 KiB / 0 B | 41 / 1 |
| matrix-portable | 4.00 MiB | 256.90 KiB | 29.48 KiB | 190.56 MiB / 20.84 MiB | 228.50 KiB / 0 B | 41 / 1 |
| bs58 | 64.00 KiB | 2.03 KiB | 964 B | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| five8 | 64 B | 11.81 KiB | 1.74 KiB | 0 B / 0 B | 0 B / 0 B | 0 / 0 |
| matrix-forced | 2.00 KiB | 164.18 KiB | 15.34 KiB | 734.78 KiB / 618.75 KiB | 712.50 KiB / 602.25 KiB | 3 / 3 |
| ntt-same-owned | 4.00 MiB | 252.03 KiB | 28.12 KiB | 190.56 MiB / 104.72 MiB | 228.50 KiB / 0 B | 41 / 22 |
| ntt-same-cached | 4.00 MiB | 252.03 KiB | 28.12 KiB | 190.56 MiB / 104.72 MiB | 228.50 KiB / 0 B | 41 / 22 |

The `matrix` decoder uses Karatsuba above its matrix domain; the matrix encoder uses NTT above its domain. Small recursive wrappers use batched kernels. `ntt-portable` permits compiler auto-vectorization and still uses normal IFMA short-product fallback. `ntt-cached` is a legacy hook name: with cache off, it owns twiddles per call. `matrix-portable` can route above its domain to normal NTT kernels. `ntt-same-owned` and `ntt-same-cached` isolate twiddle policy while keeping leaf policy equal; feature-enabled matrix leaves can still retain their own tables in both controls. These routes are listed per payload in the timing CSV.

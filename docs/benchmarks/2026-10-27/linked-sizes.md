# Linked code and data budgets

All values below subtract the matching empty observer. Both API directions are linked with opt3/fat LTO.
They describe these linked consumers, not a standalone rlib or exact whole-application increment.
Source: `4e5ac883…`. Linux values are local cross/code-generation results, not GCP CPU measurements.

For ELF, the data column counts sections with SHF_WRITE, including relocation-only `.data.rel.ro`.
It does not imply every byte remains writable after the loader applies RELRO.
File padding and platform runtime scaffolds can dominate file size; the CSV also retains stripped file and gzip lengths.

## m5

| Backend | Default code / data | Cache code / data |
|---|---:|---:|
| empty | 0.0 KiB / 0.0 KiB | 0.0 KiB / 0.0 KiB |
| auto | 58.5 KiB / 22.6 KiB | 60.8 KiB / 25.0 KiB |
| no-alloc | 9.6 KiB / 4.1 KiB | 9.7 KiB / 4.1 KiB |
| batched | 5.1 KiB / 1.5 KiB | 5.2 KiB / 1.5 KiB |
| karatsuba | 28.7 KiB / 13.9 KiB | 29.6 KiB / 14.3 KiB |
| toom3 | 33.5 KiB / 14.4 KiB | 34.5 KiB / 14.8 KiB |
| ntt | 50.0 KiB / 19.8 KiB | 51.6 KiB / 20.7 KiB |
| ntt-cached | 50.0 KiB / 19.8 KiB | 56.2 KiB / 21.9 KiB |
| ntt-portable | 52.4 KiB / 18.6 KiB | 59.1 KiB / 20.7 KiB |
| malachite | 324.6 KiB / 662.4 KiB | 325.5 KiB / 662.9 KiB |
| matrix | 55.6 KiB / 20.9 KiB | 58.1 KiB / 22.0 KiB |
| matrix-portable | 55.2 KiB / 21.1 KiB | 58.4 KiB / 22.3 KiB |
| bs58 | 1.3 KiB / 0.9 KiB | 1.4 KiB / 0.9 KiB |
| five8 | 11.7 KiB / 2.0 KiB | 11.8 KiB / 2.0 KiB |
| matrix-forced | 16.6 KiB / 11.5 KiB | 18.3 KiB / 12.3 KiB |
| ntt-same-owned | 49.4 KiB / 19.8 KiB | 56.1 KiB / 21.9 KiB |
| ntt-same-cached | 49.4 KiB / 19.8 KiB | 56.2 KiB / 21.9 KiB |

The second column component is readonly section bytes; writable/RELRO sections and BSS remain separate in the CSV.

## x86_64-linux

| Backend | Default code / data | Cache code / data |
|---|---:|---:|
| empty | 0.0 KiB / 0.0 KiB | 0.0 KiB / 0.0 KiB |
| auto | 279.6 KiB / 34.9 KiB | 270.7 KiB / 35.5 KiB |
| no-alloc | 20.5 KiB / 14.5 KiB | 22.0 KiB / 14.4 KiB |
| batched | 6.7 KiB / 1.4 KiB | 6.8 KiB / 1.4 KiB |
| karatsuba | 177.1 KiB / 17.0 KiB | 179.5 KiB / 17.2 KiB |
| toom3 | 183.1 KiB / 17.8 KiB | 185.2 KiB / 18.0 KiB |
| ntt | 249.0 KiB / 27.3 KiB | 251.1 KiB / 27.9 KiB |
| ntt-cached | 249.0 KiB / 27.3 KiB | 252.1 KiB / 28.1 KiB |
| ntt-portable | 206.4 KiB / 22.2 KiB | 210.8 KiB / 23.3 KiB |
| malachite | 1207.9 KiB / 681.1 KiB | 1207.9 KiB / 681.2 KiB |
| matrix | 253.4 KiB / 28.3 KiB | 256.9 KiB / 29.4 KiB |
| matrix-portable | 253.3 KiB / 28.4 KiB | 256.9 KiB / 29.4 KiB |
| bs58 | 2.0 KiB / 0.9 KiB | 2.0 KiB / 0.9 KiB |
| five8 | 11.8 KiB / 1.7 KiB | 11.8 KiB / 1.7 KiB |
| matrix-forced | 161.5 KiB / 14.7 KiB | 164.2 KiB / 15.3 KiB |
| ntt-same-owned | 249.0 KiB / 27.3 KiB | 252.0 KiB / 28.1 KiB |
| ntt-same-cached | 249.0 KiB / 27.3 KiB | 252.0 KiB / 28.1 KiB |

The second column component is readonly section bytes; writable/RELRO sections and BSS remain separate in the CSV.

## aarch64-linux

| Backend | Default code / data | Cache code / data |
|---|---:|---:|
| empty | 0.0 KiB / 0.0 KiB | 0.0 KiB / 0.0 KiB |
| auto | 57.6 KiB / 21.5 KiB | 59.3 KiB / 23.9 KiB |
| no-alloc | 9.7 KiB / 4.0 KiB | 9.7 KiB / 4.0 KiB |
| batched | 5.0 KiB / 1.4 KiB | 5.0 KiB / 1.4 KiB |
| karatsuba | 28.1 KiB / 13.2 KiB | 32.5 KiB / 13.4 KiB |
| toom3 | 32.7 KiB / 13.6 KiB | 33.2 KiB / 13.8 KiB |
| ntt | 49.1 KiB / 18.7 KiB | 50.0 KiB / 19.4 KiB |
| ntt-cached | 49.1 KiB / 18.7 KiB | 54.5 KiB / 20.8 KiB |
| ntt-portable | 50.9 KiB / 17.5 KiB | 56.8 KiB / 19.6 KiB |
| malachite | 327.6 KiB / 660.6 KiB | 328.0 KiB / 660.9 KiB |
| matrix | 54.4 KiB / 20.0 KiB | 56.3 KiB / 20.9 KiB |
| matrix-portable | 54.1 KiB / 20.1 KiB | 56.4 KiB / 21.2 KiB |
| bs58 | 1.4 KiB / 0.9 KiB | 1.4 KiB / 0.9 KiB |
| five8 | 11.8 KiB / 1.9 KiB | 11.8 KiB / 1.9 KiB |
| matrix-forced | 15.9 KiB / 11.1 KiB | 17.1 KiB / 11.7 KiB |
| ntt-same-owned | 48.5 KiB / 18.7 KiB | 54.5 KiB / 20.8 KiB |
| ntt-same-cached | 48.5 KiB / 18.7 KiB | 54.5 KiB / 20.8 KiB |

The second column component is readonly section bytes; writable/RELRO sections and BSS remain separate in the CSV.

## wasm

| Backend | Default code / data | Cache code / data |
|---|---:|---:|
| empty | 0.0 KiB / 0.0 KiB | 0.0 KiB / 0.0 KiB |
| auto | 70.1 KiB / 14.4 KiB | 71.8 KiB / 15.2 KiB |
| no-alloc | 13.5 KiB / 9.6 KiB | 13.5 KiB / 9.6 KiB |
| batched | 8.4 KiB / 1.2 KiB | 8.4 KiB / 1.2 KiB |
| karatsuba | 36.2 KiB / 10.5 KiB | 35.9 KiB / 10.8 KiB |
| toom3 | 44.3 KiB / 10.8 KiB | 43.5 KiB / 11.0 KiB |
| ntt | 62.9 KiB / 12.7 KiB | 63.0 KiB / 13.0 KiB |
| ntt-cached | 62.9 KiB / 12.7 KiB | 63.0 KiB / 13.0 KiB |
| ntt-portable | 62.9 KiB / 12.7 KiB | 63.0 KiB / 13.0 KiB |
| malachite | 481.0 KiB / 637.8 KiB | 480.8 KiB / 638.1 KiB |
| matrix | 66.7 KiB / 13.1 KiB | 66.8 KiB / 13.5 KiB |
| matrix-portable | 66.7 KiB / 13.1 KiB | 66.8 KiB / 13.5 KiB |
| bs58 | 2.7 KiB / 0.8 KiB | 2.7 KiB / 0.8 KiB |
| five8 | 10.0 KiB / 1.6 KiB | 10.0 KiB / 1.6 KiB |
| matrix-forced | 19.8 KiB / 9.4 KiB | 19.5 KiB / 9.7 KiB |
| ntt-same-owned | 62.9 KiB / 12.7 KiB | 63.0 KiB / 13.0 KiB |
| ntt-same-cached | 62.9 KiB / 12.7 KiB | 63.0 KiB / 13.0 KiB |

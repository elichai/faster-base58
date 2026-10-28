# x86 linked code and static-data costs

The large x86 footprint is real: unrolled IFMA multiplication dominates it. Matching standalone executables retain almost the same conversion code as the shared-library probes. The difference is not a newly linked 200 KiB standard-library penalty.

All values below are bytes above each matching empty probe, using Rust 1.99.1/LLVM 23.1.1, opt 3/fat LTO/16 codegen units. Shared libraries and executables remain separate deployment kinds. Inlining prevents exact source-level crate ownership; these are deployed-artifact deltas, not standalone crate sizes. [Full partitions](x86-linked-partitions.csv) preserve all 56 reviewed artifacts and exact hashes.

| CPU / cache policy | Auto shared-library code | Auto executable code | No-alloc shared-library code | bs58 shared-library code |
|---|---:|---:|---:|---:|
| Generic / default |286,285|286,048|20,992|2,080|
| Generic / cache |277,165|276,576|22,548|2,080|
| Native / default |274,029|273,761|15,120|2,448|
| Native / cache |255,309|254,689|15,120|2,448|

The generic/default Auto shared-library delta decomposes as 271,271 named base58 bytes, 10,443 observer/inlined bytes, 1,856 generic support bytes specialized for base58 types, 1,662 other Rust/runtime bytes and 1,053 unnamed/padding bytes. The empty shared library already contains 211,561 executable bytes. Backtrace/formatting support is largely shared with that baseline; subtracting unlike probe kinds would obscure the actual cost.

## Concentrated kernel cost

Generic/default emitted IFMA multiplication occupies 144,527 named bytes, 50.5% of the Auto code delta. All twelve `kernel_tf::<NW>` specializations are present. The NW9–12 bodies total 95,407 B. NW12 alone has 4,287 instructions, including 2,304 `vpmadd52` instructions; its deliberate unrolling is not duplicate code.

The separate NW_MAX12→8 prototype removes about 95,440–95,456 code bytes after secondary layout changes. Wide12 remains the default, with an opt-in compact feature integrated separately. Prototype timings have their own source and include mixed outcomes; later byte-identical controls limit close causal rankings. Feature-wired source needs its own qualification and measurements. The code reduction is a verified static fact; no universal speed benefit is claimed.

Other generic/default named families include 75,239 B NTT, 23,294 B encoder divide-and-conquer and 9,790 B x86 matrix code. These family counts exclude separately classified inlined observer code and generic type support.

## Static headers, RELRO and heap

Generic/cache links the full 6,280 B 64-bit cache-header inventory: 5,776 B NTT, 224 B decoder Matrix and 280 B encoder Matrix. Native/cache Auto can discard decoder Matrix routing and link 6,056 B. These are empty static container headers. Retained cache tables are separate heap allocations.

ELF `data_bytes` includes relocation-only `.data.rel.ro`; it is not all writable after relocation. BSS includes linker `.relro_padding`, so a negative BSS delta is a placement effect, not saved cache heap. The partition CSV retains RELRO and writable-after-relocation columns separately. Full section/file metrics belong beside requested allocation counts and RSS, not in place of them.

## Calls, panics and duplicate routines

The independent relocation-aware review found no distinct-address byte-identical named base58 routines in the ordinary Auto shared-library/executable artifacts. Previously similar normalized NTT thunks have different emitted bytes and different ISA targets. A normalization match is not a merge proof.

Generic NTT adapter closures total 821 B across 28 bodies. Native closure-rooted bodies total 27,999 B because actual kernels inline into them; calling that total dispatch overhead would be wrong. Register/GOT-indirect transfers also include jump tables. They are not one-to-one counts of dynamic backend calls.

Recognizable static panic/assert/failure transfers occupy about 1,215–1,800 instruction bytes per Auto cohort. That excludes checks, argument setup, cold blocks and unwind metadata. Static sites do not prove valid inputs can panic and do not justify removing validation. [Assembly notes](assembly.md) distinguish true CPU-detector loads, folded tuning state, callbacks and static frames.

## GCP versus local cross-codegen

The GCP generic and older Rosetta-generated generic ELF builds use the same frozen4e5 source and Rust/LLVM versions. Both actual `.comment` sections identify LLD 23.1.1, LLVM commit `1b9c0d5ff9bbe7634aead059efe6b11a7eeba145`. Installed GNU ld 2.40/2.44 are environment facts, not the actual linker. GCC startup-object provenance differs 12.2/14.2.

Their Auto executable totals and all category/family totals match. All twelve IFMA kernel bodies are byte-identical. Other text bytes/relocations are not wholly identical; GCP readonly grows 64/72 B and BSS shrinks 64 B. There is no demonstrated new GCP kernel-bloat regression. Local Rosetta code generation remains separate from native GCP CPU measurements.

# Performance and resource survey · 2026-10-27

The public API, internal backend wrappers, retained caches and linked deployment costs are measured separately. The [public tables](public-tables.md) and [M5](../../../assets/graphs/m5-public.svg) / [Wasm](../../../assets/graphs/wasm-public.svg) graphs use ordinary consumers without `__internals`. All sizes refer to decoded binary payload length; decoder text is longer.

The ordinary M5 Auto path takes about 298 ms to encode 4 MiB and 142 ms to decode it. Cache mode takes 280/139 ms in this repeated-input cohort. At 4 KiB, cache helps more: 28.3/25.2 µs versus 110/39.9 µs, with retained heap of 2,891,136/2,437,248 B. Those gains do not make caches free or equally useful at every size.

**Completion scope:** all local M5/Wasm timing campaigns, allocation/resource work and all 408 local/cross-linked size artifacts are complete. The full internal grid has 3,828 measured/skipped cells and 464 separate temporal Auto controls. Fresh GCP generic/native build, linked-size and ordinary ABI qualification also passed. Its actual-source, candidate and observer-control studies are complete; the remaining single-worker backend grids and counted units are a separate pending supplement. Native Windows is unavailable. Historical GCP results elsewhere are not blended into this snapshot.

**Measurement limitation:** byte-identical GCP libraries measured about 27% apart in tiny decoding when loaded from two files, but were near parity when both labels used the same file. The physical cause is unknown. [Observer controls](observer-controls/README.md) retain every calibration condition and pair. Ordinary old/current and candidate timings remain descriptive observations; they do not establish source-level causes. The backend grid uses one executable per CPU/cache cohort and keeps temporal Auto controls separate.

The full backend graphs cover [M5](graphs/m5-backends.svg) and [Wasm](graphs/wasm-backends.svg), with default/cache × encode/decode panels. The readable [algorithm](backend-tables/m5-default-algorithms.md) and [kernel-control](backend-tables/m5-default-kernel-controls.md) tables mark the lowest observed median in bold. [timings-local.csv](timings-local.csv) retains every local domain/cap/route label; [local-completion.json](local-completion.json) binds the complete data and figures. These internal comparisons remain separate from the ordinary public figures.

## Source and platform contracts

| Scope | Identity |
|---|---|
| Backend/heap/linked-size source | `4e5ac8830ccf82a69488cb8e351060bf8e97fa8c3d108f63bebffc1c4c41e760` |
| Selected ordinary native public source | `a479a1430a4fafa254edc8471b3cdd0c29486572ad0ebed434367adfeb7f474d` |
| Ordinary source bridge | `0915a8395c3f9f92a8a98bf9fed06011285ebe501cc4436e77494180c6e8e360`; matching executable src bodies and observer, one API comment annotation |
| Later proof-only source | `07ec12777e6065bd644baa75af4fd2cd0fe38fc39674957dc1eb220dc220f3b1`; one Kani unwind attribute, unchanged normal executable bodies |
| Final integrated source, separate qualification | `304dc4c5dbf18be9c6e1eab7a62b5dabb98022ff6532c1e5ba4fe5e6df000add`; compact feature and threshold-state cleanup; older curves are not relabeled as this source |
| Actual old library for regressions | Git `09862629f8ca52bc874ff3c7895524fdc81a7cff` |
| Rust / LLVM | Rust 1.99.1 / LLVM 23.1.1; opt 3/fat LTO for linked consumers |

| Platform | Timings | Requested heap | Linked size |
|---|---|---|---|
| Apple M5 Pro, compiler-default target flags | Native process CPU | Counted native workers | Mach-O |
| Wasm on M5 Pro, Node/V8, SIMD128 off | Elapsed conversion time | Separate counted Wasm modules | Wasm sections and stripped modules |
| Generic x86_64 Linux, local amd64 container under Rosetta | No native CPU claim | Not measured in this cohort | ELF / GCC startup objects 14.2 / actual LLD 23.1.1 |
| Generic aarch64 Linux, installed cross compiler | Not executed | Not executed | ELF / cross GCC 14.2 / GNU ld 2.44 |
| GCP AMD EPYC 9B45 / c4d-highcpu-4, generic/native | Completed bounded studies; full grid supplement pending | Full counted supplement pending | 204 backend ELF artifacts plus ordinary controls; [code decomposition](x86-linked-size.md), actual LLD 23.1.1 |

[Compiler contracts](compiler-contracts.json), [snapshot source files](current-source.json), [source bridges](source-bridges.json) and the CSV artifact hashes identify each cohort. The snapshot file is frozen4e5, not a claim about the latest main. [Final-source metadata](final-source-304dc.json) records the separate integrated source. Source fingerprints include observer/proof text; they are not interchangeable with the code-size tool's hash namespace. No later source change retargets a measured epoch.

## Reading the measurements

Each timing case uses 3 fresh processes and 3 warm batches per process. The ordinary public CSV reports the median of 3 process medians and their observed range. The all-backend CSV uses the median of 9 warm batches. These descriptive ranges are not confidence intervals or statistical significance tests. Cold and second-call values remain separate. Tiny cold clocks and ABI overhead can dominate; empty controls are reported without subtraction.

Other compiler, test, proof and fuzz jobs are paused for each timed segment. AC power is recorded, with available thermal status at later boundaries. This does not fix frequency or temperature. Native process CPU and Wasm elapsed clocks must not be compared as the same quantity.

Fixtures are checked against independent Malachite Natural conversion, with source/module/fixture hashes. Warm timing excludes fixture preparation, caller-output allocation and post-batch checks. Decoder repeat calls use prevalidated `&str`. Ordinary consumer qualification uses guarded, unaligned buffers and exact expected output capacity. The frozen Rust backend worker checks complete output after each batch, but does not re-poison that output or add benchmark canaries between batches. Owned timing separately includes Vec allocation, decode and destruction.

The 29-size grid covers 0, 1, 8, 16, 21, 22, 23, 31, 32, 33, 63, 64, 65 B; 128 B through 4 MiB by powers of two. Quadratic no-allocation/batched/bs58 rows stop at 64 KiB by measurement policy. This is not an API limit. Five8 has only 32/64 B rows. Forced Matrix has its own actual kernel domain. Skips are explicit; no 4 MiB quadratic row is extrapolated.

The backend labels are wrappers, not seventeen independent algorithms. Small recursive wrappers fall back to batched conversion. Matrix encoding outside its domain can use NTT; Matrix decoding can use Karatsuba. `*_portable` replaces a kernel family, not every downstream fallback; an NTT portable wrapper can still use IFMA multiplication, and compilers can vectorize plain Rust. `ntt-cached` is a legacy hook name for feature-selected policy: with `cache` off it owns twiddles per conversion. The CSV's `route_inference` column explains these aliases from source/target contracts, not an execution trace. Repeated Auto cells are temporal controls, not extra algorithms.

## Heap, cache, RSS and stack

[Per-backend resource tables](resources/m5-default.md) combine representative allocation counts, peak/retained heap and linked code costs. Separate [M5 cache](resources/m5-cache.md), [Wasm default](resources/wasm-default.md) and [Wasm cache](resources/wasm-cache.md) tables retain each target's scope.

[allocations.csv](allocations.csv) contains 6,768 cold/warm rows across M5 and Wasm. Requested layouts come from Rust `GlobalAlloc`. They exclude allocator headers/slack and allocator-internal realloc-copy overlap. `alloc_calls` includes `zeroed_calls`; the latter is a subset and must not be added twice. Caller storage is outside the measured operation window and remains explicit in the CSV.

For a random 4 MiB borrowed Auto conversion:

| M5 mode | Encode peak extra heap | Decode peak extra heap | Retained encode / decode heap |
|---|---:|---:|---:|
| Default | 201,582,112 B | 109,997,816 B | 0 /0 B |
| Cache | 214,372,960 B | 110,631,416 B | 14,401,152 /616,704 B |

Wasm's corresponding default peaks are 201,580,924/109,997,432 B. The 4 MiB Wasm cache path retains zero backend heap in a fresh instance: it uses Horner leaves and its transform exceeds the twiddle retention cap. A nonzero retained cache is not an owned output buffer. No-allocation, bs58 and five8 borrowed conversions make zero allocator operations in every supported measured case.

The full cache-family header inventory occupies 6,280 B in the recorded 64-bit static layout; an ordinary consumer can link a subset. Native/cache Auto links 6,056 B when unused decoder-Matrix headers are discarded. Fixed encoder tables have 2,208 B of nominal readonly layout. Cache data lives on the heap. Production NTT retention for logs 5–18 permits `48 * (2^19 - 2^5) = 25,164,288` requested data bytes; larger twiddles are per-conversion owners. Internal diagnostic logs 1–4 add 1,440 B, giving 25,165,728 B if every diagnostic slot is initialized. Matrix size classes add separate retained bytes.

In the six [Wasm histories](cache-histories.csv), ascending/descending/mixed cache runs retain 43,482,064/26,526,720/34,578,352 B and end at 277,807,104/223,084,544/238,419,968 B of linear memory. Default retains zero backend heap. These histories are observations, not maxima. Released heap does not imply Wasm pages return to the host.

[RSS](rss.csv) is a separate fresh-process peak, including Python, module/runtime, fixtures, touched caller buffers and validation copies. At 4 MiB the default Auto encoder/decoder peaks are about 243.2/130.5 MB, cache 257.0/131.1 MB. Warm peaks include the earlier cold call and validation. Never subtract two process peaks as exact backend memory.

[Assembly frames](assembly.md) are also separate: default M5/x86 decoder entries reserve 4,352/4,296 B before the tiny branch; cache 128/56 B. Aligned Matrix frames are larger. Dynamic stack high water and a full recursive stack bound were not measured.

## Owned API regression and capacity policy

The tight `decode_vec` bound saves 1,521,495 B for the random 4 MiB fixture: returned capacity 4,206,491 B versus 5,727,986 B from allocating one byte per input character. Backend scratch is additional. The [policy control](owned-policy.csv) uses the same current borrowed kernel in both cases; it is not mislabeled as the old library.

[Actual old/current consumers](owned-actual-old-current.csv) use matched build paths with fresh target trees and explicit recompilation logs. An empty cache-enabled owned result initially regressed from 2.75 to 3.50 ns. The accepted thin inline wrapper returns empty before entering the nonempty decoder, taking about 1.9 ns in the matched caller. Its normal default/std/cache configurations passed 512 property cases and all-size ordinary ABI goldens.

The M5 default 32 B owned result observed about 4.7%/~1.3 ns more than actual old; the three paired ratios range 1.017–1.079. Its large and long-zero cases were near parity. GCP native/cache owned observations were +17.46% at 4 KiB and +8.42% at 4 MiB. The [same-kernel capacity control](observer-controls/capacity-timing.csv) did not reproduce those costs, and the later file-identity control prevents assigning a source-level cause. The [borrowed old/current grid](borrowed-actual-old-current.csv) remains separate.

The [candidate ledger](candidates/README.md) retains every bounded experiment. The direct tiny-call rewrite produced byte-identical owned code. Leaf specialization, decoder outlining and their combination completed both local and GCP studies but remain unmerged with no established benefit. Their static code/frame changes are verified; close timing attribution remains limited by observer controls. The [DC3072 crossover](candidates/dc3072/dc3072-review.md) was rejected for verified code growth and no demonstrated benefit, not a proven slowdown.

## Cache and SIMD tradeoffs

[Feature comparison](cache-feature.csv) includes route/leaf changes. At 4 KiB, warm cache/default ratios are 0.275 encode/0.616 decode on M5 and 0.307/0.574 on Wasm. A rough cold-plus-warm model recovers setup after 17/47 repeated M5 calls or 5/11 Wasm calls. Those separately scheduled medians are not causal or significance bounds; different histories reuse different tables.

[Equal-leaf NTT controls](cache-twiddles.csv) isolate twiddle policy. For example, M5's 8 KiB decoder gains about 12% for 98,304 additional retained bytes; 1 MiB decoding gains about 3% for 12,582,912 B in this cohort. Many large/Wasm differences are near parity; zero-retained 4 MiB paths do not establish a cache benefit. Medium-size Matrix routes explain much of the feature's gain.

[Wasm SIMD128](wasm-simd.csv) enables the compiler feature for both library and ordinary observer, without a new handwritten backend. It improves 4 MiB Auto latency about 16–19% and adds 8–9 KiB of code above empty, but cache 512 B/4 KiB cases regress about 21–30%. Generic/SIMD cohorts are separately scheduled. This is a deployment tradeoff, not a blanket default recommendation.

## Linked size and next decisions

[linked-sizes.md](linked-sizes.md) compares every backend on four target formats; [linked-sizes.csv](linked-sizes.csv) retains all 408 artifact hashes and encode/decode/both sections. Matching empty probes separate observer/runtime scaffolds. Code/readonly/data/BSS are not interchangeable. ELF `SHF_WRITE` includes relocation-only `.data.rel.ro`; file alignment can dominate stripped ARM file lengths.

An ordinary borrowed-only consumer can let the linker discard unused large backends. M5 no-allocation uses about 9.6 KiB of code above empty versus 58.5 KiB for Auto; bs58 uses 1.3 KiB. Wasm values are 13.5/70.1/2.7 KiB. These are linked consumers, not standalone crate sizes. The no-allocation API reduces deployment cost by avoiding Auto's large kernels. The separate x86 `compact` feature addresses Auto itself: its NW8 prototype saves about 93 KiB of linked code, roughly one third of the generic Auto delta, with mixed observed CPU outcomes. Wide12 remains the default. Prototype measurements are not final feature-wired measurements.

Keep batched/fixed paths for small and no-allocation work, Karatsuba for recursive fallbacks, NTT for large conversions and Matrix for the opt-in cache's useful medium domain. Toom-3 remains experimental; Malachite remains a development adapter. Do not drop architecture fallbacks because wrapper timings tie: aliases and short fallback paths explain many ties. Evaluate policy specialization, decoder outlining and [detector-load hoisting](detector-hoist.md) with native x86/GCP evidence before merging further tuning.

## Reproduction

Use new output directories and separate counted/uncounted builds. Existing tools are `scripts/resource_report.py`, `scripts/code_size.py`, `scripts/wasm_tradeoffs.mjs`, `scripts/wasm_allocations.mjs` and `scripts/ac_power_run.py`. [Reproduction notes](reproduce.md) give the exact scope and parameters; [public-graphs.json](public-graphs.json) binds graph/table inputs and sources. The normal graph check uses only Python's standard library; optional figure regeneration uses pinned Matplotlib 3.11.2. It does not claim a CI pixel rebuild.

Large binaries, Cargo caches, raw process logs and duplicate source archives remain private. Compact CSVs, exact hashes and measured/rejected recipes stay in this report so later changes can be compared without retargeting older data.

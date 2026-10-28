# Owned decoder and rejected ablation review

The actual ordinary native/cache decoder regressed **17.46% at 4 KiB** and **8.42% at 4 MiB**. The cause remains unresolved. Three paired fresh-process observations had the same sign in each case.

| Actual native/cache owned decode | Minimum | Median | Maximum |
|---|---:|---:|---:|
| 4 KiB | +17.239% | +17.456% | +17.527% |
| 4 MiB | +8.364% | +8.415% | +9.208% |

CPU and wall measurements agree closely. Matched borrowed Auto controls were +0.065% and -0.357%; owned empty-observer controls were +0.058% and +0.100%. These controls do not isolate an allocator or layout effect. The ranges describe three paired ratios, not confidence intervals or universal regression bounds.

The native/cache comparison found 43 common conversion and specialized-support symbol bodies with equal sizes and instruction/reference sequences after relocation normalization. Twelve IFMA kernel bodies are byte-identical. The 7,453-byte NTT decoder moved from `0x38910` to `0x38980`; its arithmetic and callback families did not change in this bounded comparison. The renamed API wrapper and timed observer are outside the 43-body equality scope. Their capacity computation, stack frames, Result copies, `vzeroupper` instructions, and placement differ. Both output allocations call `calloc`; neither output success path resizes the allocation. This evidence does not establish a cause for the slowdown.

Output capacity for the 4 MiB fixture fell from 5,727,986 to 4,206,491 bytes: **1,521,495 bytes saved (26.56%)**. This measures output capacity, not total peak memory.

The separate frozen-source ablation study also showed adverse tiny-input rows. The encode-only leaf candidate regressed 31.11% at 22 decoded bytes despite retaining all 1,660 native decoder instructions, relative offsets, register operands and named call targets. After call-target normalization, differences are RIP references. Lookup/SIMD constants or GOT targets were checked at 63 of 92 sites; 29 error-metadata/string/object address sites were not asserted byte-identical. The decoder moved from `0x59b70` to `0x57b20`.

Outline and combined reduce the API frame from 4,296 to 232 bytes, including pushes. Their timed prevalidated loop remains 184 bytes under relocation. Tiny inputs do not call the new medium helper, so its call overhead does not explain their tiny regressions. Outline/combined nevertheless regress roughly 5–6% at 32/64 bytes; combined regresses 20.71% at 22 bytes. Exact adverse rows and ranges are in the companion JSON. Placement changes do not prove a hardware cause.

Actual timing identity remains commit `0986262` versus selected resource `a479a143`; the ablations remain based on frozen resource `4e5ac883`. The a479 → 0915 → 07ec bridge contains documentation and a proof-only attribute, and does not retarget ordinary timings. Generic and native contracts remain separate. The ELF receipts use Rust 1.99.1 / LLVM 23.1.1; disassembly used LLVM 23.1.4.

[Companion evidence](owned-and-ablation-review.json) records full source/artifact identities, summary/raw-receipt hashes, adverse rows, key code addresses, and hashes of sealed private evidence using logical IDs. No private absolute paths or binaries are exported.

Not checked: no new project execution, build, timing, transfer, hardware counter, allocator trace, or whole-program equivalence proof. Retain the measured regressions and the unresolved-cause caveat.

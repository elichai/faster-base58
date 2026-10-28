# Ordinary-consumer assembly review

This review uses the frozen `4e5ac883…` source, Rust 1.99.1/LLVM 23.1.1 and opt 3/fat LTO. The Linux artifacts are generic x86 code generated in the local amd 64 container under Rosetta. They were not timed on GCP. The normal Auto, no-allocation and competitor consumers do not enable `__internals`.

The selected later main changes only the owned empty wrapper relative to this kernel snapshot. Its separate ordinary owned-API measurements are in [the report](README.md). Static assembly frames below are not dynamic stack high-water measurements.

## Detector loads and dispatch

The mutable internal IFMA threshold atomic is absent from both ordinary x86 Auto symbol tables. That tuning atomic folds when its setter is unreachable. CPU-detector loads remain in production.

In `<faster_base58::ntt::Ntt as faster_base58::dc::Multiplier>::mul`, the short-product branch reaches:

| Consumer | Instruction address | Emitted access |
|---|---:|---|
| Default/no_std | `0x480bd` | `movzbl` from `detect_ifma::STORAGE` at `0x94151` |
| Cache/std | `0x4574a` | `movq` from std detector `CACHE` at `0x94598`, then the AVX-512F test |
| Cache/std | `0x45761` | A second `movq` from that cache, then the IFMA test |

The default getter has the initialized/unsupported/uninitialized states of cpufeatures. The std build uses std's feature macro. Atomic reads on x86 can compile to plain MOV instructions; searching only for LOCK/acquire opcodes misses them. The sites above establish real production detector access, not merely an artifact of the all-backend benchmark worker.

`Ntt::new` already chooses transform/prepared thresholds from the IFMA result once. Short fallback still calls `mul::mul`, which asks again. The free NTT multiplier and some Karatsuba paths also retain detector reads. A [follow-up recipe](detector-hoist.md) describes the bounded experiment. The IFMA primitive itself still checks availability, so passing only the outer threshold would not remove every detector access.

The `Kernels` callbacks remain indirect. For example, default `ntt::transform` has stack-spilled function-pointer calls at `0x466b4`, `0x466d6` and `0x46739`; the cache counterparts are `0x43c54`, `0x43c76` and `0x43cd9`. These correspond to the callback-based transform implementation, not a `dyn` trait API. The compiler has not specialized this consumer to one fixed runtime CPU choice.

PIC/GOT calls must be treated separately. The default NTT multiplier call at `0x4812a` uses GOT slot `0x92fd8`. The ELF dynamic relocation identifies it as `memcpy`; objdump's nearby `<write+…>` annotation is not its target. The relocation-aware scan resolves all RIP-indirect call/jump sites in the selected ordinary Auto functions. Register/aggregate indirect sites can include callbacks, tail dispatch and computed jump tables. A total count is not a count of kernel callbacks or runtime executions.

## Frames, panics and leaf linkage

| Entry / function | Default | Cache | Scope |
|---|---:|---:|---|
| M5 `api::decode` | 4,352 B | 128 B | Fixed crate frame before the tiny branch |
| Generic x86 `api::decode` | 4,296 B | 56 B | Recognized pushes and stack reservations before the tiny branch |
| Generic x86 `Ntt::mul` | 360 B | 376 B | Initial fixed prologue only |
| M5 observer `size_decode` | 64 B | 64 B | ABI scaffold, distinct from its callee |

The default decoder's large local divide-and-conquer state is inlined into the tiny entry. Its large NTT branch restores that frame before the tail call. A private outline ablation moved the medium decoder frame out of tiny calls and improved those calls locally; it remains unmerged pending the requested native x86/GCP measurement.

Aligned Matrix encoder frames are larger: the M5 cached encoder's reservation is about 31 KiB, and a recursive matrix leaf about 39 KiB. Those are manually inspected aligned reservations, not values from the simple fixed-prologue parser and not a full recursive-stack bound. The no-cache ordinary encoder still links about 4,400 instruction bytes for an unused Matrix leaf because the general leaf-policy implementation remains reachable. A private closed-policy specialization saved 4,864 code +8,236 readonly bytes locally. It is also deferred pending GCP measurement.

Panic/bounds/expect paths remain in arithmetic/table code. Examples include scratch/slice bounds and the `prepare runs first` internal precondition. Emitted panic sites do not establish that a valid public input can reach them. Removing checks solely because they appear in assembly would weaken robustness without proving a speed or size benefit. Existing property, fuzz, Miri and Kani evidence should accompany any such change.

## Duplicate candidates

The first instruction-normalization pass found eight AVX-512F/AVX2 adapter pairs per Auto configuration. Their noncapturing callback signatures match. The source builds their target-feature kernels separately after runtime detection.

All 16 pairs have **different emitted bytes**, distinct code addresses and different feature-kernel targets. For example, default closure#0 calls `tf_load` at `0x584c0`, while closure#12 calls a differently compiled `tf_load` at `0x54ef0`. A normalizer that retains only their demangled name erases that distinction. Constant and GOT targets also have to remain part of the comparison.

The nominal repeated adapter-body budget is only 293 B per consumer, before considering alignment or a replacement helper. This is not a measured saving or proof that the adapters can share a body. Any deduplication needs a preserved call ABI, feature assumptions and constant/relocation targets, followed by paired linked-size and latency tests on every affected ISA. No deduplication is merged.

## Exact artifacts

| Generic x86 ordinary Auto | SHA-256 |
|---|---|
| Default | `0dfb3d9a111887f303933e351428dd862f5b5e5e99748a75c1111cda1493b696` |
| Cache | `fbe5d944bfba46d42fb41ce482d5f666fe08839e42c7546042b35a0268d9ec5a` |

The compact assembly evidence records exact symbol addresses, raw-body hashes, relocation targets and source hashes. Full linked files and disassembly are retained privately. The reproduction recipe uses `llvm-objdump --disassemble --demangle --no-show-raw-insn`, `llvm-objdump --dynamic-reloc` and `llvm-nm --defined-only --print-size --demangle` on the same unstripped artifacts. Do not use stripped files when comparing symbol names.

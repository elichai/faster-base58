# Linked assembly and code size

The linked-code audit did not establish a runtime change that was both faster
and smaller across the measured hosts. The production loops and ISA guards were
retained. Static call counts and instruction counts identify candidates; they do
not measure their execution cost.

The baseline is commit `7c6ffa325164fc38652d3927b3172c267eaa38cd`, resource
identity `17ab5f264ed4104513e78f8cde41210e2d294332e2c99ce266b888fa0e9dd9cc`.
Normal Auto/no_alloc consumers omit `__internals`; named backend controls expose
more code. Public consumers use opt-level 3, fat LTO and 16 codegen units. The
initial x86 inventory used Rust 1.95 / LLVM 22; the paired loop experiment uses
matched Rust 1.99.0 / LLVM 23.1.1 cloud builds and Rust 1.99.1 local M5 builds.
[Receipts](benchmarks/assembly/receipts.json) bind these separate scopes.

## Findings

| Area | Observed linked code and implication |
| --- | --- |
| IFMA tuning threshold | The private `IFMA_THRESHOLD` atomic folds in inspected ordinary consumers. Generic x86 passes literal 384 when IFMA is available; native feature folding removes the detector path. A tuning-enabled internal worker is a different build. |
| CPU detection | Generic no_std reads the cpufeatures detector byte. With std, the inspected fallback contains std detection-cache loads. Native builds fold the checks. These are feature-selection costs, not repeated CPUID instructions in the arithmetic loop. |
| Matrix kernel pointers | The selected matrix pointer calls devirtualize in the initial linked inventory. Their source representation alone does not justify replacing the kernel table. |
| NTT callbacks | Each inspected transform has 12 static kernel-callback sites on both architectures. Native x86 has 14 register-call sites in that function: 12 callbacks and two resolved memset calls. This is not 14 backend dispatches. |
| no_alloc indirect-looking calls | The five inspected native x86 register-call sites resolve to memcpy/memset imports. No backend callback is present there. |
| Cache synchronization | The inspected M5 cache consumer contains six own-library acquire LDAPR sites. These belong to cache lookup/publication, outside the butterfly arithmetic. |
| Bounds and panic paths | The inspected AArch64 schoolbook inner limb loop has no per-limb bounds checks; a row guard remains outside it. NTT entry/stage checks remain. Removing a cold panic path can alter code size without reducing hot-loop work. |

[Detector excerpts](benchmarks/assembly/selected-assembly.md),
[detector bindings](benchmarks/assembly/detector-evidence.json) and
[resolved no_alloc imports](benchmarks/assembly/no-alloc-import-calls.json)
retain the concrete evidence. `std` selects `is_x86_feature_detected!`; no_std
selects cpufeatures. Neither route permits calling an ISA leaf without the
required feature conjunction and OS state support.

The older [threshold-state experiment](benchmarks/decisions/README.md) also
inspected an ordinary consumer with `CARGO_PROFILE_RELEASE_LTO=false`: the
threshold object was absent and literal 384 remained. Cargo can still use local
ThinLTO with that setting; this is not evidence for every LTO mode or for current
non-LTO consumers. Passing a threshold snapshot through multiplier state did not
establish a broad gain and regressed several ordinary-consumer decode cases.
That experiment remains rejected.

An exact-body search across 87 artifacts found no separate identical
own-library functions of at least 32 bytes within each artifact. This does not
rule out partial duplication or semantically equivalent code. The twelve IFMA
shape specializations occupy 141,181 bytes of generic x86 function spans and
141,769 bytes in native builds; the largest is 30,643 bytes. They have different
shape code rather than identical bodies. These spans are not whole-library
size, and Wasm does not include them. See the
[shape table](benchmarks/assembly/ifma-kernel-sizes.csv).

## Rejected variable-step loop rewrite

Six NEON/AVX512 stage loops were changed from variable-step iterators to explicit
index loops in a scratch candidate. Vector bodies, entry checks and ISA guards
were unchanged. An explicit nonzero-step rejection remained, so malformed
zero-step input could not become an infinite loop. An index-sequence regression
covered 642 supported stages and 8,796,334 blocks through log 23. This checks
iteration equivalence; it is not a proof of the arithmetic.

| Both-direction Auto consumer | Code delta B | Read-only delta B |
| --- | ---: | ---: |
| M5 default | -64 | -173 |
| M5 cache | -128 | -173 |
| GCP generic, default/cache | -16 | +48 |
| GCP native, default/cache | +80 | +48 |

The x86 linked sections grew. Across 696 fresh processes, three AB/BA pairs per
case and three warm CPU batches of at least 10 ms, timings were mixed. Native
default 4 MiB encode took 1.60% more time (paired range +1.25% to +2.18%); native
cache 64 KiB encode took 1.95% more (+0.28% to +2.30%). Other cells improved,
including an unchanged route below the NTT threshold, so those changes cannot
all be caused by the rewritten loops. Literal unchanged-artifact load controls
also varied. These ranges are descriptive, not confidence intervals.

The candidate passed direct native and converter checks on both hosts, including
required IFMA execution on GCP and 512-case property suites. Correctness did not
make the mixed size/time tradeoff useful. The original loops remain.
[Sizes](benchmarks/assembly/candidate-sizes.csv),
[CPU results](benchmarks/assembly/candidate-warm-cpu.csv),
[other clocks](benchmarks/assembly/candidate-all-clocks.csv) and the
[source-change recipe](benchmarks/assembly/loop-recipe.json) preserve the decision.

For a fresh audit, build ordinary consumers with `scripts/code_size.py`, retain
their metadata and inspect the final linked artifact. Resolve register-call
targets and relocations before labeling them backend indirection. Compare
entry, stage and inner-loop paths separately. Every accepted optimization still
needs source-matched correctness checks, allocation/size accounting and paired
local/GCP timing; smaller assembly alone is insufficient. See
[performance methodology](performance.md) and [safety contracts](../CORRECTNESS.md).

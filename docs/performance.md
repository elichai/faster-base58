# Performance and resource tradeoffs

The [README tables](../README.md#performance) show selected conversion times and
linked sizes. Auto trades workspace for large-input speed. `no_alloc` avoids
internal allocation; its general large conversions remain quadratic.
[DESIGN.md](../DESIGN.md) describes dispatch and algorithms;
[CORRECTNESS.md](../CORRECTNESS.md) describes validation and remaining gaps.

## Measurement scope

The broad public timings, linked sizes and M5 allocations below are historical,
before the [NTT heap reductions](#bounded-inverse-root-tables). They were measured
**2026-10-29**, from combined Git snapshot `06e081591a8ff444f990133411f093e5512610dc`.
Resource fingerprint: `c9d7163caf8dfa8f52355c09e2da04a84bb7dc205acc2386b5d9b087371580f5`
(source/manifest/lock/support files, not a Git ID). All platforms use this same
source, including the formatter, table alignment and radix scratch improvements.

| Platform | Compiler / runtime | Target / clock |
| --- | --- | --- |
| Apple M5 Pro | Rust 1.99.1 (`58cf8f9c3`), LLVM 23.1.1 | `aarch64-apple-darwin`, compiler-default flags; process CPU time |
| GCP AMD EPYC 9B45 | Rust 1.99.0 (`b940084d7`), LLVM 23.1.1 | `x86_64-unknown-linux-gnu`, generic flags, CPU 0 pinned; process CPU time |
| Scalar Wasm on M5 | Rust 1.99.1, LLVM 23.1.1; Node 26.13.0 / V8 14.6.202.34-node.40 | `wasm32-unknown-unknown`, generic flags; V8 elapsed time |

These are separate ordinary public consumers without `__internals`. Default and
cache-enabled Auto are separate builds; competitor/no-allocation series use
default features. Both encode and decode are reachable. Size and timing builds
are uncounted, release opt-level 3, fat LTO, 16 codegen units. Counted allocation
consumers are separate and are never timed. bs58 is 0.5.1; five8 is 1.0.0.

Each measured cell has three fresh processes, each with three warm batches of
at least 10 ms. The reported center is the median of three process medians.
Figure bars span those process medians; they are not confidence intervals.
Empty consumers are size scaffolds, never subtracted from conversion latency.
The grid has 29 decoded sizes from empty through 4 MiB. Figures omit zero on
logarithmic axes. Quadratic `no_alloc` and bs58 stop at 64 KiB by policy;
five8 covers supported 32/64-byte inputs. Skipped cells are not extrapolated.

Warm loops reuse caller output buffers and include Auto's workspace allocation
and release. Fixture construction, caller-buffer allocation and validation are
outside timing. Repeat decode uses text prevalidated as UTF-8, matching Rust's
`&str` input. These are borrowed APIs; owned String/Vec helpers were not measured
in this refresh. Malachite Natural independently checks the deterministic input
fixtures and encoded text, and outputs/canaries are validated outside timing.
This distribution does not replace leading-zero, malformed-input, short-buffer
or boundary correctness tests. Sources, artifacts and fixture hashes were
checked before/after the campaigns. Raw outputs stay outside Git.

### Timing limits

The current tables measure one combined source snapshot. Earlier separate PR
cohorts showed substantial host variation: byte-identical bs58/five8 modules
had later/earlier latency ratios from 0.7354 to 1.0316 across 50 cells. Current
versus historical table differences therefore do not isolate source changes.
For the current snapshot, 4 MiB Auto ranges are shown below; all clean process
observations were retained.

| Platform / policy | Encode median [min, max] | Decode median [min, max] |
| --- | ---: | ---: |
| M5 / default | 264 ms [256 ms, 264 ms] | 143 ms [143 ms, 146 ms] |
| M5 / cache | 255 ms [254 ms, 258 ms] | 137 ms [137 ms, 139 ms] |
| x86-64 / default | 349 ms [346 ms, 349 ms] | 161 ms [161 ms, 161 ms] |
| x86-64 / cache | 321 ms [321 ms, 323 ms] | 160 ms [160 ms, 163 ms] |
| Wasm / default | 1.67 s [1.64 s, 1.67 s] | 837 ms [835 ms, 884 ms] |
| Wasm / cache | 1.65 s [1.64 s, 1.73 s] | 835 ms [834 ms, 863 ms] |

No compilation, tests, fuzzing or plotting scheduled by this study overlapped
accepted timing workers. The x86
campaign completed all 984 workers with CPU 0 pinned. Mac processes that
overlapped a detected compiler/test job were preserved and rejected in full,
then rerun from their first case; this rule was independent of measured latency.
M5/Wasm used AC power. Thermal/performance status
was unavailable, and CPU frequency, temperature and background host activity
were not fixed. Native CPU and Wasm elapsed clocks measure different quantities.
These tables describe observations, not matched before/after optimization tests.
Use unchanged-artifact controls and a workload-specific repeat before tuning
dispatch thresholds from small differences.

Historical empty native decode also depended strongly on the address of its
zero-length buffer; those cells do not characterize general empty-input latency.

Compared with the superseded `11d0c16` snapshot, the final x86 default
512 B decode observation changed from 1.000 to 1.221 µs, while 4 KiB
changed from 33.170 to 27.537 µs. These separate runs include upstream
source changes; neither difference is attributed to the alignment fix.

### Matched x86-64 follow-up

A separate 2026-10-29 generic EPYC comparison of pre-merge main `a18406d` with
`06e0815` did not reproduce the apparent 22% default 512 B decode slowdown.
Both libraries used matching physical source/probe/build/package paths, Rust
1.99.0 / LLVM 23.1.1, opt3/fat LTO/16 codegen units and CPU 0. Each policy used
three fresh paired processes, reversed load order and nine alternating batches
per case of at least 10 ms process CPU time, with independent fixture checks.

With separate output buffers, the median paired final/main ratio was 1.0006
for default 512 B decoding and 1.0356 for default 4 KiB encoding. Reusing the
same input/output addresses for both immutable artifacts gave 0.9995
[0.9986, 1.0006] and 0.9885 [0.9822, 0.9885], respectively. Brackets span the
three within-process median ratios, not confidence intervals. The latter
slowdown was therefore not stable under caller-buffer control.

For these default artifacts, executable bytes, ELF code placement and relocation
sections were identical; differences were build-ID and panic source-line
metadata. The changed NTT table constructor is outside the 4 KiB
Karatsuba/Horner path. The exact runtime mechanism remains unproven, and no
source-caused x86 speedup is claimed. This diagnostic has a different artifact
and loading context from the full public cohort and does not replace its values.

### Small-input detail

Default Auto encoding around the fixed 32/64-byte routes. These are observations
from this source, not a causal comparison with another branch:

| Payload | M5 Auto encode | x86-64 Auto encode | Wasm Auto encode |
| --- | ---: | ---: | ---: |
| 16 B | 17.7 ns | 30.6 ns | 36.4 ns |
| 21 B | 21.5 ns | 31.2 ns | 35.8 ns |
| 22 B | 27.4 ns | 37 ns | 45.7 ns |
| 23 B | 26.4 ns | 42 ns | 48.4 ns |
| 31 B | 28.4 ns | 48.8 ns | 68.6 ns |
| 32 B | 12 ns | 31.2 ns | 26.1 ns |
| 33 B | 28.3 ns | 48.4 ns | 68.8 ns |
| 63 B | 73.4 ns | 111 ns | 155 ns |
| 64 B | 24.4 ns | 31.3 ns | 102 ns |
| 65 B | 73.5 ns | 114 ns | 178 ns |
| 128 B | 188 ns | 331 ns | 553 ns |

### First and second calls

For 4 KiB, each cell is **encode / decode**, median of three separate process
observations. First/second calls are single operations and include timer/FFI
overhead and cold code/data effects. Process/module startup and caller-buffer
allocation are excluded. They do not isolate cache construction; do not infer a
universal cache break-even count from these separate medians.

| Platform / policy | First call | Second call | Warm |
| --- | ---: | ---: | ---: |
| M5 / default | 124 µs / 56 µs | 111 µs / 40 µs | 109 µs / 40.6 µs |
| M5 / cache | 1.34 ms / 808 µs | 31 µs / 31 µs | 29 µs / 25.9 µs |
| x86-64 / default | 122 µs / 50.6 µs | 54.4 µs / 29.4 µs | 49.7 µs / 27.5 µs |
| x86-64 / cache | 254 µs / 66.5 µs | 51.9 µs / 29.2 µs | 40.8 µs / 27.3 µs |
| Wasm / default | 486 µs / 226 µs | 474 µs / 223 µs | 466 µs / 218 µs |
| Wasm / cache | 1.86 ms / 1.07 ms | 150 µs / 129 µs | 148 µs / 127 µs |

## Generic and native x86-64

Generic builds retain runtime CPU selection. Native builds use
`-C target-cpu=native` on the EPYC VM, whose AVX2, AVX-512 IFMA and VBMI support
was checked. These historical internal-worker observations were measured 2026-10-28 with
Rust 1.99.1 / LLVM 23.1.1, resource fingerprint `4e5ac883…1e760`. They use a
different source, compiler and call context from the current public tables.
Their estimator is the median of nine batches, rather than process medians.
Do not compare them as a matched generic/native pair with the new README data.

Each cell is **encode / decode**, using warm process-CPU medians:

| Build / policy | 4 KiB | 64 KiB | 4 MiB |
| --- | ---: | ---: | ---: |
| Generic / default | 50.963 µs / 33.094 µs | 3.558 ms / 1.363 ms | 350.689 ms / 165.306 ms |
| Generic / cache | 41.331 µs / 33.293 µs | 2.776 ms / 1.295 ms | 325.575 ms / 163.279 ms |
| Native / default | 48.796 µs / 21.894 µs | 3.557 ms / 1.236 ms | 339.745 ms / 156.087 ms |
| Native / cache | 38.430 µs / 22.210 µs | 2.770 ms / 1.169 ms | 313.082 ms / 154.469 ms |

Native code generation helps several decode sizes in this cohort. It is not a
portable deployment setting or evidence that every backend improves. Shared
wrapper fallbacks explain some equal timings and do not justify deleting an
underlying algorithm family.

## basexx comparison

`basexx` remains a benchmark comparator: its `Base58` implementation is much
faster than bs58 at the sampled sizes. Auto is faster than both here, but its
margin over basexx is much smaller than its margin over bs58.

Measured **2026-10-29**, Git source `bf58dc8`, Apple M5 Pro, Rust 1.99.1 / LLVM
23.1.1, `-C target-cpu=apple-m4`, release/fat LTO, `__internals` enabled,
cache off. These are the existing Criterion 0.8.2 encode/decode benchmarks,
which link the development backends; they are separate from the earlier public
consumer surveys. Values are Criterion median estimates of elapsed wall time per
conversion, from 30 samples per case, with 1 second warm-up and a 2 second
measurement target, in one process per direction. Input is one deterministic fixture per size with
its first byte nonzero, checked before timing.

| Operation / payload | Auto | bs58 0.5.1 | basexx 0.2.0 / Base58 | basexx 0.2.0 / Base58B |
| --- | ---: | ---: | ---: | ---: |
| Encode / 1 KiB | 12.5 µs | 1.14 ms | 28.0 µs | 968 µs |
| Encode / 4 KiB | 109 µs | 17.2 ms | 188 µs | 13.7 ms |
| Decode / 1 KiB | 2.77 µs | 333 µs | 5.45 µs | 315 µs |
| Decode / 4 KiB | 49.9 µs | 5.34 ms | 73.0 µs | 5.38 ms |

The basexx APIs allocate an owned result, which the adapter copies into the
output buffer and drops inside the timing loop. bs58 and Auto reuse caller
output. These measure the available API paths, not identical allocation costs.
The M5 host was on AC, but other work was active. This exploratory comparison
has no fresh-process replication or architecture/input-distribution sweep; it
supports retaining the comparator, not a universal performance ranking.

Reproduce with `RUSTFLAGS='-C target-cpu=apple-m4'` when building
`benches/encode.rs` and `benches/decode.rs`, filtering
`encode/(bs58|basexx|basexx-b|auto)/(1024|4096)$` or its `decode/` counterpart,
and Criterion options `--warm-up-time 1 --measurement-time 2 --sample-size 30
--noplot`. Build both before timing and run them serially. Keep raw outputs
under ignored `target/measurements/`.

## Cache and memory

Enabling `cache` changes dispatch and leaf policy as well as retention. The
current README reports warm behavior; first-call costs above are separate.
The following historical ordinary-public M5 allocation windows use the same source and
fixtures as the timing refresh. Each size/operation starts in a fresh process;
the warm window follows its first conversion. No other sizes prime the cache.

| Payload | Policy | Operation | Call | Requests | Total requested | Peak extra | Newly retained |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| 1 MiB | default | decode | cold | 25 | 27,496,000 B | 27,495,424 B | 0 B |
| 1 MiB | default | decode | warm | 25 | 27,496,000 B | 27,495,424 B | 0 B |
| 4 MiB | default | decode | cold | 27 | 109,998,392 B | 109,997,816 B | 0 B |
| 4 MiB | default | decode | warm | 27 | 109,998,392 B | 109,997,816 B | 0 B |
| 1 MiB | default | encode | cold | 14 | 32,992,644 B | 32,992,644 B | 0 B |
| 1 MiB | default | encode | warm | 14 | 32,992,644 B | 32,992,644 B | 0 B |
| 4 MiB | default | encode | cold | 14 | 131,965,668 B | 131,965,668 B | 0 B |
| 4 MiB | default | encode | warm | 14 | 131,965,668 B | 131,965,668 B | 0 B |
| 1 MiB | cache | decode | cold | 28 | 27,662,336 B | 27,661,760 B | 12,740,800 B |
| 1 MiB | cache | decode | warm | 13 | 14,913,088 B | 14,912,512 B | 0 B |
| 4 MiB | cache | decode | cold | 30 | 110,631,992 B | 110,631,416 B | 616,704 B |
| 4 MiB | cache | decode | warm | 27 | 109,998,392 B | 109,997,816 B | 0 B |
| 1 MiB | cache | encode | cold | 14 | 32,992,644 B | 32,992,644 B | 0 B |
| 1 MiB | cache | encode | warm | 14 | 32,992,644 B | 32,992,644 B | 0 B |
| 4 MiB | cache | encode | cold | 14 | 131,965,668 B | 131,965,668 B | 0 B |
| 4 MiB | cache | encode | warm | 14 | 131,965,668 B | 131,965,668 B | 0 B |

At these exact 1/4 MiB encode sizes, the cache-enabled radix route uses owned
large tables and retains none; this does not negate retention at other sizes.

Requests are `alloc_calls + realloc_calls`; zeroed allocation calls are already
included in `alloc_calls`. Total requested bytes include allocation layouts and
each reallocation's new layout; they are not peak live memory.
Peak extra excludes caller buffers and memory retained
before that window. Newly retained is the live-heap increase during the window;
zero for a warm call does not mean its earlier cache disappeared. Layout counts
exclude allocator overhead, RSS, realloc-internal overlap and stack high water.
No-allocation is an API guarantee; other-platform allocations were not remeasured
in this refresh.

Matrix table capacities sum to 18,617,760 B; production NTT retention for logs
5–18 permits 9,436,416 B after the NTT heap reductions. Their combined 26.8 MiB
capacity depends on call history. Cache families have 3,544 B of static headers
in the recorded 64-bit layout, with unused families subject to dead stripping. Fixed 32/64-byte encoder
constants have a separate 2,208 B reference layout. Construction scratch,
larger per-call tables, allocator overhead and concurrency add memory; this is
not an active-heap bound. Wasm linear-memory pages need not return to the host
when allocations are freed. RSS and full dynamic stack high water were not
remeasured. See [design memory accounting](../DESIGN.md#heap-stack-and-cache-costs).

### Bounded inverse-root tables

The 2026-10-29 comparison against main at `7962809` covers both NTT heap changes.
Montgomery companions are derived with a wrapping 32-bit multiply when loaded,
including SIMD loads. Inverse roots are retained only through 65,536 points per
prime; larger transforms generate the remaining stages in the existing product
temporary after pointwise multiplication. The scratch bound falls from
`16*n + 64` to `12*n + 128` limbs. Primes, transform plans and dispatch thresholds
are unchanged. This comparison includes main's pooled two-prime AArch64 radix
encoder. The candidate's public-probe source fingerprint starts `692e6530a28b`.

For a 4 MiB significant binary payload, local aarch64 Auto workers measured:

| Policy | Operation | Peak extra, before → after | Warm requests, before → after |
| --- | --- | ---: | ---: |
| Default | Encode | 131,965,668 → 82,158,308 B | 14 → 10 |
| Default | Decode | 109,997,816 → 63,860,984 B | 27 → 21 |
| Cache | Encode | 131,965,668 → 82,158,308 B | 14 → 10 |
| Cache | Decode | 109,997,816 → 63,860,984 B | 27 → 21 |

These are warm operation windows excluding caller buffers and any previously
retained cache. Cache-enabled decode's first-call peak was
110,631,416 → 64,494,584 B, retaining the same 616,704 B of Matrix tables.
Encoding retained no tables at this size. The separate allocation-counted
Auto workers use `__internals`; they are not timed or pooled with the historical
ordinary-public cohort above. Worker fixtures are independently checked against
Malachite. Raw output stays under ignored `target/measurements/` or temporary
directories; it is not part of the published source tree. Relative to the
companion-only change at `b84490a`, bounding inverse tables saves another
15.5 MiB for local encoding and 11.25 MiB for decoding.

The paired public-API timing builds use default CPU flags, opt-level 3 and fat
LTO, with only default features or `cache`, without `__internals`. Both libraries
are loaded in one local process and checked byte for byte before and after
timing. Each of three runs has seven alternating samples, calibrated to at least
20 ms per batch. The local host is an Apple M5 Pro using aarch64 macOS,
Rust 1.99.1 and LLVM 23.1.1 on AC power.

The x86-64 comparison uses uncounted Auto workers with `__internals`, not the
ordinary-public libraries used locally. Each of three alternating process pairs
has five warm samples, calibrated to at least 3 ms per batch and pinned to CPU 0.
The host is the GCP EPYC 9B45, using Rust 1.99.0 / LLVM 23.1.1 and generic CPU
flags with runtime dispatch. Its default 4 MiB encode/decode peaks fell from
201,207,328 / 109,807,352 B to 117,858,816 / 63,670,520 B. Cache-enabled warm
peaks were 199,567,960 / 109,807,352 B before and 116,459,384 / 63,670,520 B after.

Both refreshed timing harnesses measure process CPU time. Warm 4 MiB medians,
with positive percentages meaning slower:

| Platform / policy | Encode, before → after | Decode, before → after |
| --- | ---: | ---: |
| Local / default | 251.20 → 251.07 ms (−0.05%) | 142.33 → 142.38 ms (+0.03%) |
| Local / cache | 253.79 → 252.31 ms (−0.58%) | 138.46 → 138.91 ms (+0.32%) |
| GCP / default | 345.38 → 334.10 ms (−3.27%) | 158.18 → 151.11 ms (−4.47%) |
| GCP / cache | 321.44 → 311.39 ms (−3.13%) | 160.06 → 151.64 ms (−5.26%) |

A separate follow-up comparison against `b84490a` used the same harnesses at
8, 32, 36, 128, 144, 1024 and 4096 KiB with default and cache builds. Its largest
observed increase was 1.56% locally (32 KiB cache encoding) and 1.04% on x86-64
(128 KiB cache encoding). Synthesizing every inverse table had slowed local
cache cases by up to 3.7%; retaining the prefix avoids most of that setup cost.
The full historical size grid and Wasm timing were not refreshed. These are
machine-specific medians, not performance guarantees.

## Linked size and compact

The [README size table](../README.md#linked-size) subtracts matching empty
consumers with both API directions reachable, separately for each source,
platform and cache policy. It reports linked cdylib sections, not rlib sizes or
crate-only instruction counts. Native readonly includes constants and relevant
metadata. Initialized data, BSS and stripped file size are distinct metrics;
Wasm CODE/DATA sections are distinct from 64 KiB linear-memory pages.

`CHAR_PAIRS` is 3,364 two-byte entries: 6,728 B (6.57 KiB). On M5 the current
no-allocation and Auto probes each retain one complete pair table.
This is linked readonly storage, not a heap allocation or cache retention.
The exact delta depends on reachable paths, compiler folding, LTO and metadata;
an existing source-level constant does not imply every consumer already linked
it. These figures require remeasurement when source, compiler or API reachability
changes.

The compact comparison below is **historical**, measured 2026-10-28 on resource
fingerprint `304dc4c5…000add`, Rust 1.99.1 / LLVM 23.1.1. It was not rerun for
this combined snapshot. Its dedicated paired controls used one fixed path/inode;
56
identical-content control medians ranged from 0.994 to 1.018. An earlier internal
study showed about 27% variation for byte-identical libraries at two paths, so
artifact identity and loading context also matter.

On x86-64, `compact` keeps eight IFMA widths instead of twelve, increasing chunk
counts for some operand shapes. Historical generic Auto code above empty
fell from 286,445 to 191,005 B by default, and 277,101 to 181,661 B with cache:
95,440 B, about 93.2 KiB, saved in each case. Native EPYC consumers saved the
same amount, about 35–37% of their Auto code delta. M5 and scalar Wasm had no
code/data-section saving in the separate final-source size builds.

On the measured EPYC VM, compact/wide paired latency ratios for 4 KiB–4 MiB
cache-off encoding ranged from 0.9095 to 0.9819. Default decoding ranged from
1.0104 to 1.0246; cache encoding from 1.0043 to 1.0297; cache decoding from
1.0069 to 1.0290. These are whole-consumer observations with the timing limits
above. Wide remains the default. Tiny inputs bypass IFMA, so their layout-driven
differences do not measure an IFMA primitive.

## Pooled radix scratch

Measured 2026-10-29 on Apple M5 Pro, little-endian AArch64 with NEON,
Rust 1.99.1 / LLVM 23.1.1, generic target, opt3/fat LTO/16 codegen units.
The baseline is PR #3 at `5d045f32523a107f56d2094badf80106dbd2f71a`;
the candidate pools power digits and lower prepared spectra, omits the stored
top spectrum and reserves schoolbook sums. The intermediate combined snapshot
`11d0c16177cda1278bb4d1225afb213697c8f6f2` contains this same radix implementation;
the final `06e0815` cohort above confirms the allocation counts and peaks.
No API, dispatch, dependency or cache-retention policy changes.

Ordinary-public counted windows exclude caller buffers and oracle construction.
Default/cache and cold/warm encode windows agree at these sizes:

| Payload | Earlier requests | Pooled requests | Earlier peak extra | Pooled peak extra |
| --- | ---: | ---: | ---: | ---: |
| 1 MiB | 54 | 14 | 37,185,461 B | 32,992,644 B |
| 4 MiB | 60 | 14 | 148,741,445 B | 131,965,668 B |

Requests include reallocations: 2 earlier, zero after pooling. Requests fall
74.1% / 76.7%; peak temporary heap falls about 11.3%. Pooled total requested
bytes equal the pooled peaks above, with no retained heap. Earlier allocation
plus reallocation-new-layout traffic was 37,186,181 / 148,742,165 B. These are
Rust layouts, not RSS, allocator usable sizes or stack high water. Sparse and
unbalanced products can still grow the schoolbook-sum buffer.

The complete borrowed-API paired timing includes workspace allocation and table
setup. Both immutable libraries were built at identical physical source/probe
paths with matching flags. Each policy has three clean fresh processes, with
nine alternating batches per case and at least 10 ms process CPU time per batch.
One process reverses library load order; outputs and function identities are
checked outside timing. Centers below are medians of three process medians;
ranges describe the three within-process percentage changes, not confidence
intervals. Negative change means less time.

| Payload / policy | Earlier time | Pooled time | Change; paired range |
| --- | ---: | ---: | ---: |
| 1 MiB / default | 53.156 ms | 53.212 ms | +0.11%; −0.49% to +0.11% |
| 4 MiB / default | 250.268 ms | 250.623 ms | +0.14%; −0.05% to +0.29% |
| 1 MiB / cache | 53.792 ms | 54.274 ms | +0.90%; −1.99% to +0.90% |
| 4 MiB / cache | 252.493 ms | 251.297 ms | −0.47%; −1.06% to +0.12% |

Across 15 encode sizes around dispatch and power-of-two boundaries (8191 B
through 4 MiB), aggregate changes ranged from −0.66% to +0.55% by default and
−1.99% to +0.96% with cache. Fixed 32/64-byte encode and six decode controls
were mostly within 1%; cached 32-byte decode differed by −1.90%. An earlier
identical-artifact control across five principal encode sizes varied from
−0.71% to +0.30%. These observations establish no material latency improvement;
the reason to keep pooling is fewer allocations and lower temporary heap.

Two earlier timing attempts overlapped unrelated tests and were rejected. In
the final attempt, a compiler was detected during the fifth process. The
acceptance rule was then amended explicitly: preserve and reject an entire
process on detected compiler/test/fuzzer load, and retry it from its first case
once idle, independently of latency. Four completed processes were retained
after auditing their full per-case load diagnostics; the failed fifth was
preserved and rerun, and the sixth completed cleanly. AC power was checked.
Background activity and code-loading effects remain limits. No gain is claimed
for unchanged fallbacks, x86-64 or Wasm, which do not select this radix route.


## Historical paired radix allocation comparison

Measured 2026-10-29 on Apple M5 Pro, AArch64 macOS, Rust 1.99.1 / LLVM 23.1.1,
generic CPU, opt3/fat LTO: PR #2 at
`0a46d4722dd38dcc3ef0b6fb8fa62571841ea770` versus allocation-improvement source
`1e165772b6318deb49773ede8c29779ff3403315`. Only little-endian AArch64 with NEON
selects this production route; these gains do not describe x86-64 dispatch.

At 1 MiB, the counted conversion excluded caller buffers and oracle construction.
Cold and warm windows gave the same values:

| Requested allocation metric | PR #2 | Reused scratch / two owned twiddle sets |
| --- | ---: | ---: |
| Allocation calls, including zeroed calls | 65,590 | 52 |
| Reallocation calls | 2 | 2 |
| Requested allocation bytes (excluding realloc) | 72,510,009 | 37,184,741 |
| Peak extra live bytes | 45,574,053 | 37,185,461 |
| Retained bytes after return | 0 | 0 |

Allocation-only requested traffic fell 48.7%; peak extra live bytes fell 18.4%. These are requested
Rust layouts, not RSS or allocator usable sizes. Current allocation windows and
their counting conventions are in [Cache and memory](#cache-and-memory).

Complete borrowed-API encode timings include workspace allocation and owned table
setup. The two immutable libraries used identical physical source/probe paths,
package identity, compiler, flags and Cargo command. Each feature policy used
three fresh processes, each with three rounds of nine alternating batches per
variant, at least 10 ms process CPU time per batch. One process reversed load
order; outputs were compared before timing and distinct function addresses were
checked. The table spans per-process reductions in median CPU time:

| Payload | Default features | `cache` |
| --- | ---: | ---: |
| 8 KiB | 5.14–6.08% | New route not selected |
| 64 KiB | 2.78–4.78% | New route not selected |
| 256 KiB | 1.62–2.76% | 2.14–5.93% |
| 1 MiB | 2.07–3.86% | 2.83–4.50% |
| 4 MiB | 2.14–3.13% | 2.46–3.53% |

These are incremental gains beyond PR #2, not a comparison with main or the
current generic-x86 table. Unchanged cached 64 KiB encoding varied 0.4–2.0%
slower; default 8191 B had a 5.1% outlier that did not recur with reversed load
order. No gain is attributed to fallback routes. The third process measured
the five principal sizes rather than the full boundary sweep. This is finite
warm sampling, not a confidence interval, cold-cache result or universal optimum.

`scripts/compare_public_api.py` preserves the general paired encode/decode
consumer. `scripts/compare_radix_encoding.py` preserves the encoding comparison
with `--reverse` and explicit `--sizes`. Each takes baseline/candidate library
paths and a new output JSON path. Build at identical physical source/probe paths
and flags, preserve immutable artifacts, and store outputs under ignored
`target/measurements/`. Raw samples, receipts and one-off reports are not
maintained documentation; the dated experiment bundle was removed from the tree.

## Fixed-table alignment

Final combined-source checks exposed an AArch64 cache-enabled 32-byte encode
regression in snapshot `11d0c16177cda1278bb4d1225afb213697c8f6f2`. Three fresh
processes per artifact form reproduced about 37 ns instead of the earlier
12 ns, using both stripped and unstripped ordinary-public libraries. Paired
loading with reversed order reproduced the slowdown. The repeat loop and
32-byte instructions matched apart from relocated constants; the table row
address had changed from 16-byte aligned to 8 bytes past that alignment.

The private fixed-table type now uses `#[repr(C, align(16))]`: rows start at
offset zero and both fixed row strides are multiples of 16. Emitted 32/64-byte
table addresses were checked in default/cache builds. Its recorded 64-bit table
sizes remain 480/1728 bytes; linked padding can change. There is no public API,
arithmetic, dispatch or allocation-policy change.

Measured 2026-10-29 on Apple M5 Pro, Rust 1.99.1 / LLVM 23.1.1, generic target,
opt3/fat LTO/16 codegen units: three fresh paired processes per feature policy,
nine alternating batches per case, at least 10 ms process CPU per batch,
matching physical build paths and checked outputs. Cache-enabled 32-byte encode
fell from 38.161 ns to 12.003 ns (68.5% less time); a separate single-artifact
check gave 11.989 ns. Default 32-byte encode stayed 11.847/11.870 ns, and
64-byte encode stayed near 24 ns in both policies. Other encode controls at
31/33/63/65 B, 8/64 KiB and 1 MiB ranged from −1.65% to +0.49% in aggregate;
their median within-process changes ranged from −0.77% to +0.92%.

One process affected by a detected unrelated compiler was preserved and retried
in full, independently of latency. AC power was checked. The placement change
and reproduced fix support explicit alignment; the exact CPU mechanism remains
an inference. This is a focused M5 result, not a speedup claim for other hosts.


## Historical paired formatter comparison

Measured 2026-10-29: PR #1 at `053f00810de3f39b03f2891a2771348e67bfa4e6`
versus whole-limb formatter source `47974045e02f7b015c4b5cccc0fca7f96f04e2c5`.
The formatter emits five pairs using independent quotients instead of two
five-digit halves. Linked AArch64/x86 assembly uses scalar reciprocal multiplies
and two-byte table loads; no hardware or SIMD division was observed.

These are complete borrowed-API encodes, not an isolated formatter benchmark.
Both immutable libraries ran in one process, with identical physical source and
probe paths, compiler, features, package identity and opt3/fat-LTO settings. Each
result is the median of three round medians, each with nine alternating batches
of at least 10 ms process CPU time. Inputs repeat with warm caches. Outputs and
decoded round trips are checked outside timing.

| Host / build | 16/22/31/33/65 B Auto | Same sizes `no_alloc` | 128 B Auto / `no_alloc` |
| --- | ---: | ---: | ---: |
| Apple M5 Pro, AArch64 macOS; Rust 1.99.1 / LLVM 23.1.1; generic CPU | 6.5–17.5% less time | 4.4–16.5% less time | 5.7% / 4.0% less time |
| AMD EPYC 9B45 Linux; Rust 1.99.0 / LLVM 23.1.1; `target-cpu=native`, one CPU pinned | 6.5–9.5% less time | 6.8–10.2% less time | 4.7% / 3.4% less time |

The benefit tapers by 512 B. Fixed 32/64-byte encoders, decoding and zero scans
are unchanged-route controls. A native fixed-64 outlier did not reproduce across
six fresh processes with both load orders: aggregate candidate/reference ratios
were 0.999 for Auto and 1.000 for `no_alloc`. Code alignment/loading effects
remain a confound; no gain is attributed to unchanged routes. Cold-cache latency
and other CPUs were not measured in this paired study. Native linked code shrank
112/96 B for Auto/`no_alloc`, with unchanged readonly bytes **beyond PR #1**;
this does not reverse the original pair-table storage cost described above.

The reusable `scripts/compare_public_api.py` accepts two qualified public probe
libraries, a new output JSON path and optional sizes. Build both at identical
physical source/probe paths and flags, preserving each immutable artifact before
replacing the source. A new filename alone does not control Cargo-path-dependent
code layout. Store outputs under ignored `target/measurements/`; raw samples,
generated assembly, receipts and path-specific build scripts are not maintained
documentation. The dated experiment bundle was removed from the branch tree.

## Measure and update

Run from the repository root. Freeze the revision, use fresh output directories
under ignored `target/measurements/`, and record source/compiler/target/features,
flags, clocks and artifact identities. Build compared variants before timing,
then run them serially on an idle host. Keep counted allocation builds separate
from timing and size builds. Use `scripts/ac_power_run.py` to record laptop AC
and available thermal status when applicable.

For an internal backend survey, build an uncounted worker, then run it:

```sh
rtk proxy python3 scripts/resource_report.py build --target-dir target/measurements/default-build --binary-output target/measurements/default-worker --output target/measurements/default-build.json
```

```sh
rtk proxy python3 scripts/resource_report.py run --build-metadata target/measurements/default-build.json --output target/measurements/default-timing --mode timing --repeats 3 --samples 3 --timeout 600
```

Use `--cache` or `--compact` on separate build cohorts. For allocations, build
with `--count-allocations` into new paths and run with `--mode allocations`.
The worker enforces backend domains and policy skips. Its metadata identifies
the source under test; these commands do not recreate the historical tables.

`scripts/public_api_campaign.py` provides build, measure and allocations phases
for public modes, but its timing phase uses internal workers. Ordinary public
consumer measurements require separate qualification of the standalone
`scripts/code_size.py` probes and their repeat-call interfaces. Do not relabel an
internal-worker result as a public-consumer result.

For deployment size, use `scripts/code_size.py` with an explicit target and CPU.
Its legacy CPU default is model-specific, so do not omit `--cpu`:

```sh
rtk proxy python3 scripts/code_size.py --source . --output target/measurements/linked --target x86_64-unknown-linux-gnu --cpu generic --backends empty,auto --directions both --kinds cdylib --strip-tool /path/to/llvm-strip
```

Use a matching linker/strip tool. Native timing must run on the actual target;
cross-compilation or Rosetta-hosted code generation establishes size only. For
Wasm, build separate modules with `--target wasm32-unknown-unknown --cpu generic`;
`--wasm-simd128` selects a separate cohort. The metadata-bound
`scripts/wasm_tradeoffs.mjs` timing driver uses Node with
`--no-liftoff --no-wasm-lazy-compilation --no-wasm-async-compilation`.
`scripts/wasm_allocations.mjs` qualifies separate counted modules.

The current refresh used the standalone probe and Natural-only fixture
exporter frozen from tooling revision `6165a97`, separately from each library
source snapshot. That tooling is included in the combined repository history.
Its native
driver was adapted for Darwin's coarse process-CPU ticks; calibration accepted
only batches of at least 10 ms. The Wasm driver used the same 10 ms minimum and
a 100-million-iteration calibration cap. Qualify any timing driver against the
frozen probe interfaces before repeating these public-consumer measurements.

Keep new CSV/JSON outputs in the measurement directory. Public figures can be
rendered from external inputs with `assets/graphs/render_current.py --data
TIMINGS.csv --sizes SIZES.csv --output NEW_DIRECTORY`; internal backend figures
use `assets/graphs/render_backends.py --data TIMINGS.csv --output NEW_DIRECTORY`.
Use the pinned plotting environment in `assets/graphs/requirements.txt`.
`python3 -B assets/graphs/render.py --check` checks the selected committed
figures and documentation without running benchmarks. It does not reconstruct
removed measurement inputs or independently reproduce their numbers.

When updating results, change each table, figure, date, source and build scope
together. Include cold costs, memory and linked-size effects. Check unchanged
routes and identical artifacts, preserve failed cases in the new measurement
output, and state unresolved variation before drawing a causal conclusion.

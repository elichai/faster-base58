# Performance and resource tradeoffs

The [README tables](../README.md#performance) show selected conversion times and
linked sizes. Auto trades workspace for large-input speed. `no_alloc` avoids
internal allocation; its general large conversions remain quadratic.
[DESIGN.md](../DESIGN.md) describes dispatch and algorithms;
[CORRECTNESS.md](../CORRECTNESS.md) describes validation and remaining gaps.

## Measurement scope

The current public timings, linked sizes and M5 allocations were measured
**2026-10-29**, from Git commit
`8ca392a84c48d5ffbe2ac0711398901bd8084bda`. Resource fingerprint:
`28bc90cd04fc83305c400ff26d36878371023fd38f0942b1604b9df9d1ecc8fb` (source/manifest/lock/support files, not a Git ID).
All platforms use that same source fingerprint. This snapshot includes only
this PR's source changes, not the independent other improvement PR.

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

Wasm showed substantial host variation. Across the two independently measured
PR source cohorts, byte-identical bs58/five8 modules had later/earlier latency
ratios from 0.7354 to 1.0316 across 50 cells. Thus source-cohort differences do
not establish a source-caused speedup. For this snapshot, the 4 MiB Auto ranges
are shown below; slower observations were retained.

| Platform / policy | Encode median [min, max] | Decode median [min, max] |
| --- | ---: | ---: |
| M5 / default | 301 ms [299 ms, 303 ms] | 140 ms [140 ms, 140 ms] |
| M5 / cache | 276 ms [275 ms, 280 ms] | 135 ms [135 ms, 137 ms] |
| x86-64 / default | 373 ms [373 ms, 376 ms] | 171 ms [168 ms, 171 ms] |
| x86-64 / cache | 345 ms [340 ms, 348 ms] | 175 ms [175 ms, 175 ms] |
| Wasm / default | 2.33 s [1.67 s, 2.52 s] | 887 ms [868 ms, 914 ms] |
| Wasm / cache | 1.71 s [1.7 s, 1.78 s] | 889 ms [883 ms, 1.22 s] |

No compilation, tests, fuzzing or plotting scheduled by this study overlapped timing. The x86
campaign restarted after unrelated VM work interrupted its first attempt; only
the complete run is published. M5/Wasm used AC power. Thermal/performance status
was unavailable, and CPU frequency, temperature and background host activity
were not fixed. Native CPU and Wasm elapsed clocks measure different quantities.
These tables describe observations, not matched before/after optimization tests.
Use unchanged-artifact controls and a workload-specific repeat before tuning
dispatch thresholds from small differences.

### Small-input detail

Default Auto encoding around the fixed 32/64-byte routes. These are observations
from this source, not a causal comparison with another branch:

| Payload | M5 Auto encode | x86-64 Auto encode | Wasm Auto encode |
| --- | ---: | ---: | ---: |
| 16 B | 18.1 ns | 30.5 ns | 36.7 ns |
| 21 B | 21.8 ns | 31.2 ns | 37 ns |
| 22 B | 26.4 ns | 37.5 ns | 47 ns |
| 23 B | 26.5 ns | 41.7 ns | 49 ns |
| 31 B | 27.3 ns | 48.6 ns | 69.6 ns |
| 32 B | 11.6 ns | 30.6 ns | 26 ns |
| 33 B | 28 ns | 48.6 ns | 72.1 ns |
| 63 B | 70 ns | 110 ns | 156 ns |
| 64 B | 23.8 ns | 31.2 ns | 103 ns |
| 65 B | 71.5 ns | 114 ns | 174 ns |
| 128 B | 180 ns | 324 ns | 550 ns |

### First and second calls

For 4 KiB, each cell is **encode / decode**, median of three separate process
observations. First/second calls are single operations and include timer/FFI
overhead and cold code/data effects. Process/module startup and caller-buffer
allocation are excluded. They do not isolate cache construction; do not infer a
universal cache break-even count from these separate medians.

| Platform / policy | First call | Second call | Warm |
| --- | ---: | ---: | ---: |
| M5 / default | 112 µs / 56 µs | 102 µs / 40 µs | 103 µs / 39 µs |
| M5 / cache | 1.28 ms / 754 µs | 30 µs / 27 µs | 28.6 µs / 25.3 µs |
| x86-64 / default | 132 µs / 73.8 µs | 53.8 µs / 29.4 µs | 49.6 µs / 27.4 µs |
| x86-64 / cache | 301 µs / 86.1 µs | 46.7 µs / 35.1 µs | 41.6 µs / 33.2 µs |
| Wasm / default | 468 µs / 230 µs | 471 µs / 221 µs | 466 µs / 224 µs |
| Wasm / cache | 1.85 ms / 1.12 ms | 155 µs / 130 µs | 158 µs / 134 µs |

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

## Cache and memory

Enabling `cache` changes dispatch and leaf policy as well as retention. The
current README reports warm behavior; first-call costs above are separate.
The following ordinary-public M5 allocation windows use the same source and
fixtures as the timing refresh. Each size/operation starts in a fresh process;
the warm window follows its first conversion. No other sizes prime the cache.

| Payload | Policy | Operation | Call | Requests | Total requested | Peak extra | Newly retained |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| 1 MiB | default | encode | cold | 40 | 62,161,312 B | 50,383,888 B | 0 B |
| 1 MiB | default | encode | warm | 40 | 62,161,312 B | 50,383,888 B | 0 B |
| 4 MiB | default | encode | cold | 44 | 248,693,680 B | 201,582,112 B | 0 B |
| 4 MiB | default | encode | warm | 44 | 248,693,680 B | 201,582,112 B | 0 B |
| 1 MiB | default | decode | cold | 25 | 27,494,848 B | 27,495,424 B | 0 B |
| 1 MiB | default | decode | warm | 25 | 27,494,848 B | 27,495,424 B | 0 B |
| 4 MiB | default | decode | cold | 27 | 109,997,240 B | 109,997,816 B | 0 B |
| 4 MiB | default | decode | warm | 27 | 109,997,240 B | 109,997,816 B | 0 B |
| 1 MiB | cache | encode | cold | 41 | 75,897,280 B | 64,403,008 B | 14,401,152 B |
| 1 MiB | cache | encode | warm | 35 | 61,359,328 B | 49,910,656 B | 0 B |
| 4 MiB | cache | encode | cold | 46 | 260,219,104 B | 214,372,960 B | 14,401,152 B |
| 4 MiB | cache | encode | warm | 40 | 245,681,152 B | 199,880,608 B | 0 B |
| 1 MiB | cache | decode | cold | 28 | 27,661,184 B | 27,661,760 B | 12,740,800 B |
| 1 MiB | cache | decode | warm | 13 | 14,911,936 B | 14,912,512 B | 0 B |
| 4 MiB | cache | decode | cold | 30 | 110,630,840 B | 110,631,416 B | 616,704 B |
| 4 MiB | cache | decode | warm | 27 | 109,997,240 B | 109,997,816 B | 0 B |

Requests are `alloc_calls + realloc_calls`; zeroed allocation calls are already
included in `alloc_calls`. Total requested bytes sum requested layouts; they
are not peak live memory. Peak extra excludes caller buffers and memory retained
before that window. Newly retained is the live-heap increase during the window;
zero for a warm call does not mean its earlier cache disappeared. Layout counts
exclude allocator overhead, RSS, realloc-internal overlap and stack high water.
No-allocation is an API guarantee; other-platform allocations were not remeasured
in this refresh.

Matrix table capacities sum to 18,617,760 B; production NTT retention for logs
5–18 permits 25,164,288 B. Their combined 41.8 MiB capacity depends on call
history. Cache families have 6,280 B of static headers in the recorded 64-bit
layout, with unused families subject to dead stripping. Fixed 32/64-byte encoder
constants have a separate 2,208 B reference layout. Construction scratch,
larger per-call tables, allocator overhead and concurrency add memory; this is
not an active-heap bound. Wasm linear-memory pages need not return to the host
when allocations are freed. RSS and full dynamic stack high water were not
remeasured. See [design memory accounting](../DESIGN.md#heap-stack-and-cache-costs).

## Linked size and compact

The [README size table](../README.md#linked-size) subtracts matching empty
consumers with both API directions reachable, separately for each source,
platform and cache policy. It reports linked cdylib sections, not rlib sizes or
crate-only instruction counts. Native readonly includes constants and relevant
metadata. Initialized data, BSS and stripped file size are distinct metrics;
Wasm CODE/DATA sections are distinct from 64 KiB linear-memory pages.

`CHAR_PAIRS` is 3,364 two-byte entries: 6,728 B (6.57 KiB). On M5 the current
no-allocation probe retains one complete table, explaining the roughly 6.6 KiB increase over the old 4.1 KiB readonly headline.
This is linked readonly storage, not a heap allocation or cache retention.
The exact delta depends on reachable paths, compiler folding, LTO and metadata;
an existing source-level constant does not imply every consumer already linked
it. These figures require remeasurement when source, compiler or API reachability
changes.

The compact comparison below is **historical**, measured 2026-10-28 on resource
fingerprint `304dc4c5…000add`, Rust 1.99.1 / LLVM 23.1.1. It was not rerun for
this PR. Its dedicated paired controls used one fixed path/inode; 56
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
source snapshot. That newer tooling is not imported into these PRs. Its native
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

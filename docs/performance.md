# Performance and resource tradeoffs

The [README tables](../README.md#performance) show selected conversion times and
linked sizes. Auto trades workspace for large-input speed. `no_alloc` avoids
internal allocation; its general large conversions remain quadratic.
[DESIGN.md](../DESIGN.md) describes dispatch and algorithms;
[CORRECTNESS.md](../CORRECTNESS.md) describes validation and remaining gaps.

## Measurement scope

The current public timings, linked sizes and M5 allocations were measured
**2026-10-29**, from Git commit
`daf6403b2c459cd2dff57e903541276d30a68cbf`. Resource fingerprint:
`ce56edbeaf63ff7b56f7ec3029da5f981f21ff32f8881b419701bf0039d989a4` (source/manifest/lock/support files, not a Git ID).
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
| M5 / default | 250 ms [247 ms, 253 ms] | 140 ms [139 ms, 141 ms] |
| M5 / cache | 252 ms [250 ms, 256 ms] | 135 ms [135 ms, 139 ms] |
| x86-64 / default | 375 ms [373 ms, 377 ms] | 170 ms [170 ms, 171 ms] |
| x86-64 / cache | 347 ms [347 ms, 348 ms] | 170 ms [169 ms, 170 ms] |
| Wasm / default | 1.65 s [1.65 s, 1.71 s] | 836 ms [835 ms, 844 ms] |
| Wasm / cache | 1.66 s [1.66 s, 1.81 s] | 844 ms [840 ms, 845 ms] |

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
| 16 B | 20.8 ns | 39.9 ns | 39.1 ns |
| 21 B | 25.5 ns | 42 ns | 42.6 ns |
| 22 B | 29.9 ns | 47.2 ns | 50.4 ns |
| 23 B | 32.3 ns | 53.9 ns | 56.3 ns |
| 31 B | 36.5 ns | 63.2 ns | 78.5 ns |
| 32 B | 11.6 ns | 30.5 ns | 26 ns |
| 33 B | 38.7 ns | 63.3 ns | 78.2 ns |
| 63 B | 85.4 ns | 136 ns | 176 ns |
| 64 B | 23.4 ns | 31.3 ns | 104 ns |
| 65 B | 86.4 ns | 140 ns | 202 ns |
| 128 B | 209 ns | 376 ns | 576 ns |

### First and second calls

For 4 KiB, each cell is **encode / decode**, median of three separate process
observations. First/second calls are single operations and include timer/FFI
overhead and cold code/data effects. Process/module startup and caller-buffer
allocation are excluded. They do not isolate cache construction; do not infer a
universal cache break-even count from these separate medians.

| Platform / policy | First call | Second call | Warm |
| --- | ---: | ---: | ---: |
| M5 / default | 110 µs / 64 µs | 103 µs / 40 µs | 105 µs / 39.4 µs |
| M5 / cache | 1.33 ms / 733 µs | 32 µs / 27 µs | 28.6 µs / 25.2 µs |
| x86-64 / default | 128 µs / 69.2 µs | 55.1 µs / 29.2 µs | 51 µs / 27.5 µs |
| x86-64 / cache | 296 µs / 80 µs | 45.6 µs / 29.3 µs | 40.8 µs / 27.4 µs |
| Wasm / default | 484 µs / 228 µs | 482 µs / 219 µs | 474 µs / 216 µs |
| Wasm / cache | 1.83 ms / 1.07 ms | 150 µs / 128 µs | 151 µs / 127 µs |

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
| 1 MiB | default | encode | cold | 54 | 37,184,741 B | 37,185,461 B | 0 B |
| 1 MiB | default | encode | warm | 54 | 37,184,741 B | 37,185,461 B | 0 B |
| 4 MiB | default | encode | cold | 60 | 148,740,725 B | 148,741,445 B | 0 B |
| 4 MiB | default | encode | warm | 60 | 148,740,725 B | 148,741,445 B | 0 B |
| 1 MiB | default | decode | cold | 25 | 27,494,848 B | 27,495,424 B | 0 B |
| 1 MiB | default | decode | warm | 25 | 27,494,848 B | 27,495,424 B | 0 B |
| 4 MiB | default | decode | cold | 27 | 109,997,240 B | 109,997,816 B | 0 B |
| 4 MiB | default | decode | warm | 27 | 109,997,240 B | 109,997,816 B | 0 B |
| 1 MiB | cache | encode | cold | 54 | 37,184,741 B | 37,185,461 B | 0 B |
| 1 MiB | cache | encode | warm | 54 | 37,184,741 B | 37,185,461 B | 0 B |
| 4 MiB | cache | encode | cold | 60 | 148,740,725 B | 148,741,445 B | 0 B |
| 4 MiB | cache | encode | warm | 60 | 148,740,725 B | 148,741,445 B | 0 B |
| 1 MiB | cache | decode | cold | 28 | 27,661,184 B | 27,661,760 B | 12,740,800 B |
| 1 MiB | cache | decode | warm | 13 | 14,911,936 B | 14,912,512 B | 0 B |
| 4 MiB | cache | decode | cold | 30 | 110,630,840 B | 110,631,416 B | 616,704 B |
| 4 MiB | cache | decode | warm | 27 | 109,997,240 B | 109,997,816 B | 0 B |

At these exact 1/4 MiB encode sizes, the cache-enabled radix route uses owned
large tables and retains none; this does not negate retention at other sizes.

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
no-allocation probe does not retain the complete table, while the Auto probe retains one.
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
| Total requested bytes | 72,510,009 | 37,184,741 |
| Peak extra live bytes | 45,574,053 | 37,185,461 |
| Retained bytes after return | 0 | 0 |

Requested traffic fell 48.7%; peak extra live bytes fell 18.4%. These are requested
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

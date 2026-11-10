# Performance and resource tradeoffs

Public-API timings and linked sizes are awaiting remeasurement. Auto trades
workspace for large-input speed; disabling `alloc`
avoids internal allocation, and its general large conversions remain quadratic.
[DESIGN.md](../DESIGN.md) describes dispatch and algorithms;
[CORRECTNESS.md](../CORRECTNESS.md) describes validation and remaining gaps.

## Measure and update

Run from the repository root on an idle host (laptops on AC power); keep output
under ignored `target/measurements/`. Build every compared artifact before timing
and never time while compiling.

Public consumer measurements:

```sh
python3 -B scripts/code_size.py --source . --output target/measurements/m5-default --target aarch64-apple-darwin --cpu generic --backends empty,auto,no-alloc,bs58,five8 --directions both --kinds cdylib
python3 -B scripts/code_size.py --source . --output target/measurements/m5-cache --target aarch64-apple-darwin --cpu generic --backends empty,auto --directions both --kinds cdylib --cache
python3 -B scripts/public_bench.py --platform m5 --output target/measurements/m5.csv auto=target/measurements/m5-default/auto-both-cdylib.dylib auto+cache=target/measurements/m5-cache/auto-both-cdylib.dylib no-alloc=target/measurements/m5-default/no-alloc-both-cdylib.dylib bs58=target/measurements/m5-default/bs58-both-cdylib.dylib five8=target/measurements/m5-default/five8-both-cdylib.dylib
```

On Linux add `--strip-tool strip` to `code_size.py` (it expects `llvm-strip`) and
`--cpu N` to `public_bench.py` to pin workers. Native workers place every input and
output buffer at a 64-byte boundary.

On the GCP Zen 5 VM (`c4d-highcpu-4`: vCPUs 0/2 and 1/3 are SMT siblings), time on
vCPU 1 and keep vCPU 3 idle:

- before and after each run, the metadata server's `instance/maintenance-event`
  must read `NONE`;
- run a 10-second unmeasured busy loop on vCPU 1 first;
- start `public_bench.py --cpu 1 --copies 4` under `sudo chrt -f 99`, `setarch -R`
  (no address randomization) and `prctl(PR_SET_THP_DISABLE)`; the workers inherit
  all three, and no system setting changes;
- sample `vmstat -t 1` on vCPU 0 and discard a run with any steal above 0.

The guest has no `cpufreq` driver (the host sets the frequency) and already uses the
`tsc` clocksource. SCHED_FIFO tasks are still throttled to 950 ms per second. Report
a cell whose process or copy medians spread by more than 3% with its range instead
of dropping it: on Zen 5 such spread usually comes from code placement, which is
real variation between builds.

For Wasm, build with `--target
wasm32-unknown-unknown` (add `--wasm-simd128` for the SIMD cohort) and time with
`public_bench.py --wasm`; it runs Node with
`--no-liftoff --no-wasm-lazy-compilation --no-wasm-async-compilation` and refuses
allocation-counting modules. Render the README figures and tables from the CSVs and
each cohort's `sizes.csv`:

```sh
python3 -B assets/graphs/render_current.py --data m5.csv gcp.csv wasm.csv --sizes m5=m5-default/sizes.csv m5+cache=m5-cache/sizes.csv ... --output NEW_DIRECTORY
```

Rendering needs the pinned Matplotlib in `assets/graphs/requirements.txt`.
`python3 -B assets/graphs/render.py --check` validates links and figure syntax
only; it does not reproduce numbers.

Paired comparisons: build both variants' Auto probes, then pass both to
`public_bench.py` with `--baseline LABEL` (more `--processes` and `--batches` help
for small effects). It prints each cell's median and range of per-process
ratios against the baseline with the same cache policy. To see what a change
inlined or moved out of line, compare the two unstripped probes:
`python3 -B scripts/check_simd_inlining.py --compare OLD NEW` (sizes run to the
next symbol, so they include alignment padding).

Internal backends and allocations: `scripts/resource_report.py build` / `run`
times internal workers (`__internals`, not ordinary public consumers) and counts
allocations with `--count-allocations` in separate, untimed builds;
`scripts/wasm_allocations.mjs` counts Wasm allocations. The Criterion benches
(`benches/encode.rs`, `decode.rs`, `check.rs`, `dispatch.rs`) compare backends and
comparators in one process with wall time. `benches/encode_tune.rs` and
`encode_matrix_tune.rs` tune the encoder leaves, `MATRIX_MIN` and `IFMA_CAP`;
`benches/radix_tune.rs` times the forced radix and division trees in one binary
(with a division A/A arm) for the [radix utilization rule](#radix-route-utilization).
`benches/threshold_tune.rs` sweeps the dispatch crossovers and the NTT and IFMA
product thresholds in one binary; its internal arms fall back below their own
minimums (divide-and-conquer decoding needs two leaves). Constants without a runtime
parameter, such as leaf sizes and the cutoffs in `src/api.rs`, are compared as rebuilt
Auto probes with `public_bench.py --baseline`; on Zen 5 the rebuild alone moved
untouched cells by up to 7%, so judge only the cells the change reaches.

When the code changes, remeasure the affected tables, figures and scope together,
or withdraw them.

## Timing limits

Code placement can move a cell more than many source changes do. On Zen 5 (GCP EPYC
9B45, 2026-11-07, Auto probes):

- a loop whose closing branch straddles a 64-byte line ran 1.2–1.3× slower, so a
  16-byte shift of unrelated code moved such cells by that much;
- the placement of a library file's pages matters as well: in one run the original
  probe file took 1.87× as long as each of three byte-identical copies on a 32 KiB
  decode, and 1.2× on an 8 KiB encode, so the same binary can change speed between
  files and VM sessions;
- loading the same library at another address can move a 1 KiB encode by 12%.

On the M5, LLVM function and loop alignment options moved cells by up to 7%.
Therefore:

- compare variants only within one paired run (`--baseline`); on Zen 5 also pass
  `--copies 4`, which times fresh copies of each library file and compares
  medians over copies;
- judge a change by the cells its code reaches, and treat movement in untouched
  cells as the noise floor;
- for effects below about 5%, add an aligned build
  (`-C llvm-args=-align-all-functions=6 -C llvm-args=-align-loops=64`) as a
  diagnostic; it is not the shipped configuration.

Background load on a laptop can make one process of a cell slow (a bimodal spread);
rerun with more processes rather than dropping observations.

## Wasm SIMD128

Builds with `-C target-feature=+simd128` use SIMD kernels and need a compatible
engine. Performance comparisons are awaiting remeasurement.

## basexx comparison

`basexx` 0.2.0 is a general-purpose benchmark comparator. Reproduce comparisons
with `cargo bench --features __internals --bench encode` (or `decode`).

## Cache and memory

Enabling `cache` changes dispatch and leaf policy as well as retention. Maximum
retained payloads are in [DESIGN.md](../DESIGN.md#heap-stack-and-cache-costs):
Matrix tables 18,617,760 B and NTT twiddles 9,436,416 B (26.8 MiB combined,
depending on call history), plus 2,176 B of static headers. Per-call scratch,
larger owned tables, allocator overhead and concurrent calls add to that; it is not
an active-heap bound. Wasm linear-memory pages need not return to the host. First
cached calls build the tables they need; whether reuse repays that depends on the
workload.

Warm peaks on the M5 (2026-11-09, counted `__internals` Auto workers, caller buffers
and previously retained tables excluded):

| Payload | Policy | Operation | Warm requests | Warm peak | First call retains |
| --- | --- | --- | ---: | ---: | ---: |
| 1 MiB | default | encode | 7 | 17,787,780 B | 0 B |
| 1 MiB | default | decode | 16 | 14,781,952 B | 0 B |
| 1 MiB | cache | encode | 7 | 17,787,780 B | 0 B |
| 1 MiB | cache | decode | 13 | 10,849,792 B | 4,090,048 B |
| 4 MiB | default | encode | 7 | 69,574,884 B | 0 B |
| 4 MiB | default | decode | 18 | 56,783,096 B | 0 B |
| 4 MiB | cache | encode | 7 | 69,574,884 B | 0 B |
| 4 MiB | cache | decode | 18 | 56,783,096 B | 616,704 B |

Requests count allocations plus reallocations; peaks are requested Rust layouts,
not RSS, allocator usable sizes or stack high water.

## Linked size and compact

Linked-size measurements subtract each cohort's empty consumer (same scaffold, no conversion call) from cdylibs with both directions
reachable. Native columns are code and readonly sections; Wasm columns are CODE and
DATA sections. Initialized data, BSS, file size and linear-memory pages are
different costs. `CHAR_PAIRS` (3,364 two-byte entries, 6,728 B) is in readonly data
whenever a reachable path uses it.

On x86-64, `compact` keeps eight IFMA kernel widths instead of twelve: less code,
more chunks for some multiplications. Other targets do not use these kernels; wide
is the default.

## Base58Check cost

`cargo bench --features check --bench check` compares checked buffer conversion with
plain conversion of the same complete frame, with and without separate hashing and
validation; repeat with `check,cache` or `--no-default-features --features check`.
Double SHA-256 adds a fixed cost to every checked call.

## Radix route utilization

The radix route needs a used fraction of its root transform (base-`58^3` digit
bound over transform points) of 5/8, or 3/4 with `cache`; on scalar Wasm 9/16 and
on Wasm `simd128` none, under both policies. The fraction rises
linearly with the input from 0.48 just above each transform-length step
(`17 * 2^k + 1` bytes) to 0.97 below the next one.

Measured **2026-11-07** with `benches/radix_tune.rs`: one `__internals` binary per
feature set times the forced radix tree, the forced division tree and a division
A/A arm, in 15 rounds of interleaved blocks of at least 10 ms. A cell is the median
of the per-round radix/division ratios. Generic flags (`RUSTFLAGS=""`), fat LTO,
default and `cache` builds. M5 Pro: AC power, Rust 1.99.1, two runs, which agree
within 0.02. Zen 5 (EPYC 9B45, 4 vCPU): Rust 1.99.0, pinned to CPU 1, one run.
Process samplers showed no other load. A/A medians stay within 0.97–1.02 (M5) and
0.99–1.01 (Zen 5). Ranges cover the size medians, 34 KiB–4 MiB on M5 and
40 KiB–4 MiB on Zen 5:

| Used | M5 default | M5 cache | Zen 5 default | Zen 5 cache |
| ---: | ---: | ---: | ---: | ---: |
| 0.48 | 1.02–1.12 | 1.17–1.22 | 1.07–1.16 | 1.13–1.24 |
| 0.56 | 0.93–0.95 | 1.05–1.13 | 1.04–1.25 | 1.10–1.24 |
| 0.62 | 0.91–0.94 | 0.96–1.03 | 0.95–1.07 | 0.97–1.05 |
| 0.68 | 0.84–0.87 | 0.99–1.06 | 0.90–0.94 | 0.97–1.04 |
| 0.75 | 0.80–0.81 | 0.89–0.95 | 0.83–0.86 | 0.84–0.90 |
| 0.81 | 0.77–0.78 | 0.83–0.91 | 0.84–0.89 | 0.82–0.89 |
| 0.88 | 0.77 | 0.81–0.89 | 0.83–0.85 | 0.84–0.88 |
| 0.94 | 0.67–0.69 | 0.73–0.81 | 0.74–0.77 | 0.76–0.79 |

The Zen 5 cells above 1 at 0.62 are 44 KiB (below the x86 minimum) and 349 KiB
(1.02). On M5 at 8.5–34 KiB, default reaches 0.95–0.99 at 0.56 used and `cache`
0.98–1.03 at 0.75. At 4.25–8.5 KiB, default takes 1.01–1.02 at 0.62 and 0.88–0.89
at 0.68. With `cache`, cached Matrix leaves speed up only the division tree, which
moves its crossover from about 0.55–0.62 to about 0.7.

### Wasm

Measured **2026-11-09** on the M5 Pro (AC power), Rust 1.99.1, Node 26.13.0 /
V8 14.6.202.34 with `--no-liftoff --no-wasm-lazy-compilation
--no-wasm-async-compilation`: `wasm32-wasip1` builds of `benches/radix_tune.rs`
(11 rounds, 7 from 640 KiB) and of a same-binary sweep of every encoder backend
(15–21 rounds), scalar and `+simd128`, default and `cache` builds. Other sessions
loaded the host (load average about 5); A/A medians stay within 0.97–1.03. Wasm
division trees keep Horner leaves under both policies, and the `cache` cells (to
544 KiB) match the default ones within 0.04, so the table gives the default build.

Each time cell is the radix route's time divided by the division tree's time, for
the same input: below 1 the radix route is faster. A cell is the median of the
per-round ratios; a range covers the measured sizes with that used fraction, from
the smallest size to the largest. The last column says which build selects the
route at that fraction (scalar Wasm needs 9/16 = 0.5625; `simd128` has no rule).

| Root transform used | Sizes measured | Scalar: radix / division | `simd128`: radix / division | Route selected |
| ---: | --- | ---: | ---: | --- |
| 0.48 | 8.5 KiB–8.5 MiB | 0.93 → 1.10 | 0.59 → 0.91 | `simd128` only |
| 0.51 | 9 KiB | 1.06 | 0.67 | `simd128` only |
| 0.57 | 10 KiB–5 MiB | 0.89–0.92 | 0.57 → 0.75 | both |
| 0.62 | 11 KiB, 88 KiB | 0.84–0.86 | 0.53–0.63 | both |
| 0.68 | 12–96 KiB | 0.76–0.79 | 0.52–0.59 | both |
| 0.75 | 105 KiB | 0.72 | 0.55 | both |
| 0.91 | 16 KiB | 0.58 | 0.45 | both |

An arrow marks a ratio that rises steadily with input size. At 0.48 used, the
scalar route is faster up to 34 KiB and slower from 68 KiB (scalar sizes stop at
1.06 MiB). With `simd128` it stays faster at every size, though the gain shrinks:
0.91 at 8.5 MiB. The transform limit stops the route at 16 MiB.

Below the default encoder's NTT threshold the alternative is batched encoding. The
forced route took 0.97 of batched time at 768 B with `simd128` (1.03 at 736 B,
0.71 at 1 KiB, 0.93–0.94 at 1152 B with 0.51 used) and 0.965 at 3 KiB scalar (0.84 at
3.25 KiB, 0.59 at 4 KiB, 1.15–1.64 at 2.25–2.75 KiB with 0.51–0.63 used). The
forced division tree took 1.0–1.8 times batched time below 3 KiB (`simd128`) and
4.3 KiB (scalar). Hence Wasm uses the route's minimum, 768 B (`simd128`) or 3 KiB,
as the NTT threshold; from 3 KiB the scalar sizes up to 4352 B are used at least
0.68. With `cache`, cached Matrix took 0.80–0.90 of the route's time at 8 KiB and
less below, so the Matrix range is unchanged.

## Open levers

Unmeasured candidates, largest expected effect first:

- Radix tree: fuse the first forward stage, whose upper half is always zero
  (about 2–3% of encode); build sparse schoolbook products column by column
  (sparse inputs need more heap); share the digit scratch with the output (about
  11% of the 4 MiB peak).
- NTT: clip the scratch envelope at the prepared length (−3.5 MiB at a 4 MiB
  decode) and reserve the cyclic sums only when used (−2 MiB); quantize cached
  twiddle classes (−49% worst-case retention, larger first builds); a forward
  transform with Cooley–Tukey butterflies and one twiddle per block, which could
  use the inverse's reduction schedule (bit-reversed twiddles for the small stages).
- Division encoder: 5 allocations per call down to 1–2; reuse the dead quotient
  buffer during the remainder product (−10–14% node scratch); non-power-of-two
  wrapped remainders; a scaled remainder tree; Karp–Markstein at the top node.
- Matrix: build table panels as a wavefront (faster first cached calls); vector
  digit conversion in the general x86 encoder; retune the 8192-byte cached encode
  cap on AVX2-only x86; retune the 1024-digit cached x86 decode start for
  AVX2 Matrix parsing.

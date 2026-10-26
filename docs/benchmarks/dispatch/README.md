# Centralized dispatch and its costs

Production backend selection now lives in `src/api.rs`. Auto chooses fixed
32/64-byte encoders first, strips other leading zeros once, then selects a
backend by significant length. Selected kernels check their preconditions and
do not fall back to another backend. Internal comparison wrappers retain their
all-size fallbacks. [DESIGN.md](../../../DESIGN.md) lists the current routes.

This cleanup changes code generation and the backend chosen for some zero-heavy
inputs. It is not an unconditional speed or memory improvement.

## Measurement generations

All timings below are local Apple M5 Pro results on sampled AC power, using
Rust 1.99.1 / LLVM 23.1.1 and compiler-default AArch64 CPU settings. Three paired
fresh-process comparisons describe variation; they are not confidence intervals.
The [receipts](receipts.json) bind each dataset to its own source and harness.

| Generation | Identity and role |
| --- | --- |
| Before | Commit `f1a710f4e774f946e59a433c0b210bd8ae6a90db` |
| Rewrite | Resource digest `84992ffd…`; source digest `f5828be6…`. Centralized routing before the two NTT attributes below |
| Outlined | Source digest `16d91134…`. The rewrite plus `#[inline(never)]` on production NTT encode/decode; adopted in the library |
| Portable x86 gate | Later resource digest `61c0ba89…`; only native x86 predicates change. Its scoped frontend-equivalence checks are in the [verification report](../../verification/local-ci.md). It was not timed here. |

Source digests use the `code_size.py` contract; resource digests cover a
different set of inputs. Full hashes are in the receipt. Earlier datasets and
README graphs retain their original source bindings.

## Timing and entry frames

The first survey covers 18 decoded sizes from empty through 4 MiB, both
directions and both cache policies: 1,200 fresh processes. It links internal
resource hooks, so it is separate from an ordinary public consumer. Warm CPU,
warm elapsed and cold elapsed results are retained in
[default](rewrite-default-summary.csv) and [cache](rewrite-cache-summary.csv).
Quadratic no_alloc/bs58 controls stop at 64 KiB by measurement policy; explicit
[default](rewrite-default-skipped.csv) and [cache](rewrite-cache-skipped.csv)
skips are not API limits. In these CSVs, a positive paired **decrease** is faster.
Large default Auto warm CPU results were within about 1% of the earlier source;
small cases included regressions.

An ordinary encode-only consumer reproduced tiny-input regressions. Inspection
showed that NTT inlining put a local frame larger than 4 KiB, with stack probes,
before even the fixed-size branches. Outlining the two NTT entries reduced the
observed Auto encode local adjustment to 512 bytes, plus saved registers.
Static instruction counts fell from 2,758 to 982 by default and 2,699 to 994 with
cache. These counts describe this linked observer, not all consumers.

The following separate experiment uses ordinary public cdylibs without
`__internals`, matched absolute build paths, reusable exactly sized unaligned
buffers, three AB/BA pairs and three CPU batches of at least 10 ms. It ran 216
processes. Positive changes here mean **slower**, unlike the first survey's CSV.

| Feature | Decoded B | Before → rewrite | Rewrite → outlined |
| --- | ---: | ---: | ---: |
| Default | 21 | +6.48% | -5.66% |
| Default | 32 | +3.15% | -4.93% |
| Default | 64 | +5.38% | -4.75% |
| cache | 21 | +8.53% | -2.56% |
| cache | 32 | +3.13% | -2.65% |
| cache | 64 | +4.24% | -4.88% |

[CPU results](ordinary-timings.csv) and [elapsed results](ordinary-elapsed-timings.csv)
include every range and control. The cache 21-byte outlining range crosses zero;
cache 32-byte has a large outlier. Byte-identical no_alloc/bs58/empty artifacts
also changed sampled times: cache no_alloc at 32 bytes moved +8.13%, with a
-3.58% to +12.33% pair range. No samples were filtered. These data do not establish
an all-size or all-target speedup. This ordinary experiment timed only 21/32/64
byte encodes, not decode or large inputs.

A separate 144-process check compares the rewrite with outlining at six larger
sizes, in both directions and cache modes. It uses uncounted resource workers
linking `__internals`, three paired processes and three CPU batches of at least
3 ms. These are not the ordinary cdylibs above. For consistency with the previous
table, the following signs report time increases; the source CSV stores decreases.

| Decoded size | Default encode | Default decode | cache encode | cache decode |
| --- | ---: | ---: | ---: | ---: |
| 512 B | -0.08% | +0.79% | -0.04% | -1.85% |
| 4 KiB | -0.21% | +1.47% | -1.25% | -0.47% |
| 8 KiB | -0.14% | -0.07% | +0.43% | +0.04% |
| 64 KiB | +0.33% | +0.82% | +0.21% | -1.09% |
| 1 MiB | +0.33% | -0.14% | -0.27% | -0.40% |
| 4 MiB | +0.16% | +0.43% | -0.90% | -0.22% |

The [24 warm CPU comparisons](outlining/warm-cpu-comparison.csv), full
[default](outlining/summary-default.csv) / [cache](outlining/summary-cache.csv)
three-clock summaries and [receipt](outlining/receipt.json) retain the source
and worker bindings. Differences stay within about 2% in this sample; three
pairs do not establish significance. This check precedes the SSE2 gate and
adds no x86, Wasm, ordinary-consumer or linked-size measurement.

## Linked size

[Rewrite sizes](rewrite-linked-sizes.csv) cover 72 linked artifacts across M5
and Wasm, both directions and features, with matched empty scaffolds. Before
outlining, Auto Wasm files shrank by 261–985 bytes; M5 cache Auto in both
directions lost 1,024 code and 656 read-only bytes. Small no_alloc increases
remain visible. Source/probe paths differ in this survey, so it does not isolate
individual instruction or panic-string causes.

The separate, path-matched [ordinary sizes](ordinary-linked-sizes.csv) show the
additional outlining cost for encode-only Auto: default code +64 B and read-only
data +104 B; cache code -64 B and read-only data +100 B. Stripped file totals are
unchanged due to Mach-O layout. This is a small section-size cost for the smaller
entry frame, not a uniform size reduction. These are linked-observer bytes,
not exact crate-only bytes; no final outlining Wasm size is implied here.

## Zero-prefix allocation tradeoff

Significant-length selection can choose a different algorithm from the former
raw-length entry plus nested fallback. These cold exact-buffer decodes have
6,000 leading `'1'` characters. Bytes count requested Rust allocation layouts.

| Feature | Significant chars | Allocations before → rewrite | Extra peak B before → rewrite | Retained B before → rewrite |
| --- | ---: | ---: | ---: | ---: |
| Default | 5,121 | 15 → 1 | 97,808 → 19,616 | 0 → 0 |
| cache | 1,024 | 0 → 3 | 0 → 166,336 | 0 → 157,888 |
| cache | 5,119–5,120 | 0 → 3 | 0 → 2,471,040 | 0 → 2,437,248 |
| cache | 5,121 | 18 → 3 | 731,408 → 2,471,040 | 665,856 → 2,437,248 |

The first route changes NTT to Karatsuba; the cache cases select retained Matrix
tables. Same-input warm Matrix calls allocate nothing, while their cold tables
remain live. The existing total cache bound is unchanged. This is an explicit
opt-in retention tradeoff, not an allocation-neutral refactor.

The [608 allocation windows](prefix-allocations.csv),
[304 paired rows](prefix-comparisons.csv) and [analysis](prefix-analysis.json)
cover fresh processes, both features, Auto/no_alloc and zero-prefix boundaries.
All 304 no_alloc windows have zero allocator operations, extra peak and retention.
Independent bs58 oracles, poisoned output and canaries passed. This survey
precedes outlining. It measures neither CPU time, stack, allocator overhead,
RSS nor mixed-input cache history. Raw logs, frozen source and full harnesses
remain outside the repository; receipt hashes alone do not recreate them.

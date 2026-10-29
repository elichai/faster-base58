# Reuse digit scratch and avoid unused owned twiddles

These measurements compare PR #2's parent `0a46d4722dd38dcc3ef0b6fb8fa62571841ea770`
with the allocation improvement code in `1e165772b6318deb49773ede8c29779ff3403315`.
The public API and selection rules are unchanged. The new production path remains
limited to little-endian AArch64 with NEON.

The conversion now owns only two twiddle sets, while eligible cached conversions
still borrow the existing shared three-prime tables. One digit workspace replaces
the per-node result vectors. Each recursive call holds its high child in a
disjoint scratch slice and builds its low child in the output; the fused carry
reads only the live low prefix before overwriting it.

## Allocation measurements

The existing `tools/resource-profile` counter used the same 1 MiB fixture as the
parent's committed measurements. Caller input/output and oracle construction are
outside the observed conversion. Both cold and warm calls give the same results:

| Requested Rust allocation metric | Parent | Candidate |
| --- | ---: | ---: |
| Allocation calls, including zeroed allocations | 65,590 | 52 |
| Reallocation calls | 2 | 2 |
| Total requested allocation bytes | 72,510,009 | 37,184,741 |
| Peak temporary live bytes | 45,574,053 | 37,185,461 |
| Retained temporary bytes | 0 | 0 |

This removes over 99.9% of allocation calls, reduces requested allocation traffic
by 48.7%, and reduces peak temporary live bytes by 18.4%. These counters measure
requested Rust layouts, not RSS or allocator usable sizes. Raw records:
[parent](../large-final-allocations.jsonl), [candidate](allocations.jsonl).

Reproduce the counters with the repository's existing release probe:

```sh
cargo run --offline --release --manifest-path tools/resource-profile/Cargo.toml --no-default-features --features count-allocations -- allocations encode auto 1048576 1
```

## Complete-API timings

Host: Apple M5 Pro, macOS AArch64; Rust 1.99.1 / LLVM 23.1.1; generic CPU,
release optimization, fat LTO. Allocations and owned table setup remain timed.
The same source path, probe manifest path, package identity, compiler, flags and
Cargo command were used for both revisions. The original copied library remains
immutable; rebuilding from the changed source produces a separately copied
candidate library. This avoids Cargo-path-dependent symbol ordering as a
comparison confound. Commands, compiler version, source hashes and binary hashes
are in [builds.json](builds.json).

Each feature configuration was measured in three fresh processes. Each process
contains three rounds of nine alternating batches per variant, with at least
10 ms CPU time per batch. One process reverses library load order. Output bytes
are compared before timing. The final process also checks that the two libraries
resolve to different native function addresses. Inputs are deterministic random
bytes with a nonzero first byte and repeat within a batch.

The table reports the range of per-process reductions in median CPU time:

| Input bytes | Default features | With `cache` |
| ---: | ---: | ---: |
| 8192 | 5.14–6.08% | New route not selected |
| 65536 | 2.78–4.78% | New route not selected |
| 262144 | 1.62–2.76% | 2.14–5.93% |
| 1048576 | 2.07–3.86% | 2.83–4.50% |
| 4194304 | 2.14–3.13% | 2.46–3.53% |

These are incremental gains over PR #2, not over `main`. Unchanged fallback
controls vary: cached 65536-byte encoding was 0.4–2.0% slower in these processes;
default 8191 bytes had a 5.1% outlier that did not reproduce in the reversed-load
process. No gain is attributed to fallback paths. No statistical confidence
interval, cold-cache result or universal performance optimum is claimed.

Raw samples are `canonical-default-{1,2,3}.json` and
`canonical-cache-{1,2,3}.json`; [summary.csv](summary.csv) contains per-process
medians. The third process covers the five principal sizes rather than the full
boundary sweep.

Run [compare.py](compare.py) in a fresh process for each cohort, without Python
optimization flags:

```sh
python3 compare.py PARENT.dylib CANDIDATE.dylib result-1.json
python3 compare.py PARENT.dylib CANDIDATE.dylib result-2.json --reverse
python3 compare.py PARENT.dylib CANDIDATE.dylib result-3.json --sizes 8192 65536 262144 1048576 4194304
```

Build the parent first with `scripts/code_size.py --backends auto --directions both
--kinds cdylib --cpu generic`. Preserve its copied artifact. At the same source and
probe paths, update the source to the candidate and replay the Cargo command from
the generated metadata, using the same `CARGO_TARGET_DIR` and `RUSTFLAGS`; preserve
the new target library separately. Add `--cache` to both builds for that cohort.
Do not overwrite a library already loaded by a measurement process. Separate
probe package paths are not equivalent controls, even with identical flags.

## Correctness

The allocation budget regression first failed on the parent with 65,590 allocation
calls, then passed with the reusable workspace. Existing poisoned-output product
tests and independent bs58/Malachite oracles cover the fused addend's live length.
Public boundary tests include exact/short buffers, guards and leading zeros.
Both default and cache configurations passed the explicit 16 MiB boundary test.
Independent source review found no correctness or safety issue in the change.

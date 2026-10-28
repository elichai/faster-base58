# Reproduction notes

Run from the repository root. Use a fresh output directory for every source/configuration and keep counted builds separate from timing and linked-size builds. Inspect the output metadata before combining results. The commands below re-run the study on the checkout under test; they do not relabel the older CSVs as that checkout.

The exact backend epoch can be reconstructed without a source archive. [source-0915-to-4e5.patch](source-0915-to-4e5.patch) is a one-file reverse bridge from the source whose resource fingerprint is `0915a839…`. Apply it only in a separate checkout with that matching base. [The replay receipt](source-epoch-replay.json) verifies that every fingerprinted file then matches `current-source.json` and restores `4e5ac883…`. The later `07ec1277…` source first needs its separate [proof-only reverse patch](source-07ec-to-0915-proof.patch), whose [receipt](proof-source-bridge.json) verifies both directions. Neither bridge retargets old numeric data. Build in fresh Cargo targets after a source swap; source hashes alone do not prove Cargo recompiled a dependency.

## Native backend worker

The default 29-size grid and backend registry are in `scripts/resource_report.py` and the resource worker. The registry enforces the quadratic 64 KiB measurement cap, fixed-width domains and forced-Matrix domains. `dc` is the native worker's Karatsuba label.

Build one uncounted worker:

```sh
rtk proxy env RUSTUP_TOOLCHAIN=stable CARGO_BUILD_JOBS=1 RUSTFLAGS= CARGO_ENCODED_RUSTFLAGS= python3 scripts/resource_report.py build --target-dir /tmp/base58-study-target --binary-output /tmp/base58-study-worker --output /tmp/base58-study-build.json
```

Run timing only after all other build/test/proof/fuzz work is idle. On macOS use the AC guard; it also records available thermal/power settings:

```sh
rtk proxy python3 scripts/ac_power_run.py --output /tmp/base58-study-power.jsonl -- python3 scripts/resource_report.py run --build-metadata /tmp/base58-study-build.json --output /tmp/base58-study-times --mode timing --repeats 3 --samples 3 --timeout 600
```

Build a distinct worker with `--count-allocations` and a new target/output/binary path, then use `--mode allocations`. Add `--cache` to the build for the separate feature cohort. Never use counted timings. The native allocation window contains one actual operation per cold/warm phase; the surrounding independent oracle and caller buffers remain outside it.

The normal public consumer study is a separate scope: it calls Auto/no-allocation/competitors directly, without `__internals`. The code-size probe exports checked calls and opaque/prevalidated repeat calls for that purpose. All-size ordinary ABI goldens precede timing. The public CSV reports three process medians; the internal worker aggregate uses all nine warm observations. Do not exchange their estimator labels.

## Linked deployment size

Use `scripts/code_size.py` with `--cpu generic`, an explicit target and `--kinds cdylib`. Its legacy default CPU is a model-specific setting, so do not omit the CPU flag. The frozen survey used 17 backend/control entries, three directions and matching empty probes, default/cache separately:

```sh
rtk proxy env RUSTUP_TOOLCHAIN=stable CARGO_BUILD_JOBS=1 RUSTFLAGS= CARGO_ENCODED_RUSTFLAGS= python3 scripts/code_size.py --source . --output /tmp/base58-linked --target aarch64-apple-darwin --cpu generic --kinds cdylib
```

Use `wasm32-unknown-unknown`, `x86_64-unknown-linux-gnu` or `aarch64-unknown-linux-gnu` for the other target cohorts, with a matching linker/strip tool and explicit compiler contract. The retained ARM ELF build used `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc`, GCC 14.2 and GNUld 2.44. It was not executed. Rosetta-hosted x86 code generation is not native GCP performance evidence.

Add `--cache` for its own cohort. Add `--wasm-simd128` only for the explicitly labeled targeted Wasm experiment. Leave `--count-allocations` off for both linked-size and timing artifacts. The default full backend set includes the forced-Matrix and equal-leaf NTT controls; their linked observer selectively enables `__internals`, while Auto/no-allocation/bs58/five8 do not.

## Wasm timing and allocation counts

Build ordinary Wasm modules first, then run the uncounted timing driver with Node's eager optimized compilation flags:

```sh
rtk proxy node --no-liftoff --no-wasm-lazy-compilation --no-wasm-async-compilation scripts/wasm_tradeoffs.mjs /tmp/base58-wasm-linked /tmp/base58-wasm-times --samples 3 --repeats 3
```

The development grid has five predeclared disjoint bands per policy in [wasm-development-plan.json](wasm-development-plan.json). Pass each band's comma-separated sizes via `--sizes` and `auto,karatsuba,toom3,malachite,matrix,matrix-portable` via `--backends`. Keep the worker's cold/second/three-warm-call protocol unchanged. Each band has its own quiet-window/AC receipt. No band is relabeled as one uninterrupted run. The frozen Wasm worker has no 120-second child timeout; the private supervisor's administrative watchdog stops only after 1,800 seconds without a completed process and never accepts a truncated case.

For requested layouts, build a separate `--count-allocations` cohort with `--directions both`. First qualify its ABI on disposable instances:

```sh
rtk proxy node scripts/wasm_allocations.mjs --self-test /tmp/base58-wasm-counted
```

The report-local [counted collector](tools/count_wasm.py) uses the same reviewed `openCounted` helper, but starts a fresh process/instance per backend/direction/size. Actual measurement instances never run the growth/self-check first. Generate the independent fixture files with the small Natural-only exporter:

```sh
rtk proxy cargo build --release --locked --manifest-path docs/benchmarks/2026-10-27/tools/fixture-export/Cargo.toml --target-dir /tmp/base58-fixture-target
```

```sh
rtk proxy python3 docs/benchmarks/2026-10-27/tools/make_fixtures.py --binary /tmp/base58-fixture-target/release/base58-fixture-export --output /tmp/base58-fixtures --manifest /tmp/base58-fixtures.json
```

```sh
rtk proxy python3 docs/benchmarks/2026-10-27/tools/count_wasm.py --modules /tmp/base58-wasm-counted --fixtures /tmp/base58-fixtures --manifest /tmp/base58-fixtures.json --output /tmp/base58-wasm-layouts
```

`--backends` and `--sizes` allow bounded subsets. The portable fixture exporter/collector passed 18 fresh control/Auto cases at 32, 64 and 8192 B, including positive allocation in the actual library and zero-allocation controls. This qualifies the portable collector, not a rerun of the frozen full dataset. Full allocation data uses the retained original instrumentation/module hashes.

## Figures and interpretation

```sh
rtk proxy python3 assets/graphs/render.py --check
```

This checks both historical output reproduction and current CSV/table/SVG hash/scoping. It regenerates numeric tables from their CSVs, parses SVG XML and binds the README's current table cells. It does not regenerate Matplotlib pixels in CI.

Optional current figure regeneration uses the separately pinned `assets/graphs/requirements.txt` environment:

```sh
rtk proxy python3 assets/graphs/render_current.py --output /tmp/base58-regenerated-graphs
```

The original renderer is archived privately because it contains workspace paths; its hash and metadata retain provenance. The public portable renderer consumes committed CSVs, fixes future SVG identifiers/date and refuses an existing destination. Its output has its own receipt, rather than silently replacing the original image hashes.

The dated all-backend research plots use the same pinned Matplotlib environment, with a separate portable renderer and the internal CSV rather than the ordinary public CSV:

```sh
rtk proxy python3 docs/benchmarks/2026-10-27/tools/render_backends.py --data docs/benchmarks/2026-10-27/timings-local.csv --output /tmp/base58-backend-graphs
```

The table renderer uses only the standard library unless its optional plotting mode is requested:

```sh
rtk proxy python3 docs/benchmarks/2026-10-27/tools/render_backend_tables.py --data docs/benchmarks/2026-10-27/timings-local.csv --output /tmp/base58-backend-tables
```

The GCP [observer controls](observer-controls/README.md) are separate protocols. Neither a newer observer nor the final304dc source can repair or retarget their older receipts. In particular, do not use two separate ELF files as a close causal comparison without qualifying the loading-context effect.

Requested heap, retained tables, output Vec capacity, Wasm pages, RSS and static stack frames answer different questions. Do not subtract process peaks or empty latency, count zeroed calls twice, pool temporal Auto controls, infer native GCP latency from Rosetta code generation, or extrapolate missing quadratic rows.

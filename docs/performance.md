# Performance and resource tradeoffs

The [README](../README.md#performance) summarizes the current native timings and
linked sizes. [DESIGN.md](../DESIGN.md) describes algorithms, dispatch and memory
ownership. [CORRECTNESS.md](../CORRECTNESS.md) describes validation and its limits.

## Current native results

Measured **2026-11-12–2026-11-13** (host-local dates). Frozen PR #68 source is
[`75144d578689c4ff699c8096fa39e9b0ee294c9e`](https://github.com/elichai/faster-base58/commit/75144d578689c4ff699c8096fa39e9b0ee294c9e);
its frozen main is
[`9127267513c55bdb7300b6618fa053fbc56ead89`](https://github.com/elichai/faster-base58/commit/9127267513c55bdb7300b6618fa053fbc56ead89).
Both timing and linked size include the prepared NTT scratch bound and all three
scratch-sizing inline hints. This comparison measures this PR against the merged
code in its base. The [counted heap cohort](#cache-and-memory) uses separate
instrumented workers and has its own source and measurement scope.

The PR is rebased onto
[`d6da5c29d170a84cbcff2c2e3c7480825eefb9d9`](https://github.com/elichai/faster-base58/commit/d6da5c29d170a84cbcff2c2e3c7480825eefb9d9),
with code at
[`9322a6728d3f0b0b7e1aeb50e22f2dc419923457`](https://github.com/elichai/faster-base58/commit/9322a6728d3f0b0b7e1aeb50e22f2dc419923457).
The rebase changes fuzz tooling, CI and documentation. Both frozen/rebased pairs
have byte-identical library sources, Cargo files, toolchain/configuration files,
benchmark scripts, consumer probes and support adapters. README inclusion is under `cfg(doctest)`
and does not enter these ordinary public builds. The measurements remain bound
to the actual frozen artifacts; the input-identity record is retained with them.

| Host | Target / ISA | OS | Compiler |
| --- | --- | --- | --- |
| Apple M5 Pro | aarch64-apple-darwin / AArch64 | macOS 26.7.3 | Rust 1.99.1 (58cf8f9c3), LLVM 23.1.1 |
| GCP AMD EPYC 9B45, 4 vCPU | x86_64-unknown-linux-gnu / generic x86-64, runtime SIMD dispatch | Linux | Rust 1.99.0 (b940084d7), LLVM 23.1.1 |

Both use release opt-level 3, 16 codegen units, empty RUSTFLAGS, default `alloc`
or `cache`, fat LTO or LTO off. No `check`, `__internals`, `compact`, allocation
counter or target-cpu=native flag is enabled. Each artifact exposes public Auto
encode and decode. Linked size uses that exact timing artifact and its matching
empty consumer. LTO is selected with `CARGO_PROFILE_RELEASE_LTO=fat` or `off`;
the generated metadata's profile text alone says fat and does not record this
effective environment override. Cargo fingerprints confirm empty Rust flags,
the intended library features and distinct fat/off profiles.

GCP work uses `/mnt/bench/pr68-native`, waits for the shared VM lock, and pins
timing to vCPU 1. M5 starts and finishes each warm/cold phase on AC with no
reported thermal warning. On each host, timing phases contain no builds/tests.
The shared lock coordinates participating jobs; it does not attest that unrelated VM work
or host scheduling interference was absent.

### Warm measurement contract

Each of the eight host/LTO/policy cohorts uses **9 fresh processes × 3 fresh file
copies per label × 3 interleaved batches**, calibrated to at least 5 ms per batch.
The copy set is created once per cohort and reused across its processes.
`main` and `ctrl` are separate copies of the **same main artifact**. `head` is
PR68. The loader verifies that all exported function addresses are distinct.
Worker order visits all nine cyclic loader starting positions. Calibration
follows that order; batches rotate by 0, 1 and 2. Loader and timed-call orders
remain linked, and artifacts are not permuted across neutral file slots. Full
cyclic rotation does not independently isolate physical placement from source.
The two source builds use equal-length `main`/`head` path labels. Build paths are
not counterbalanced, and the identical-artifact control does not measure
independent rebuild variation.

Fixtures are deterministic size-seeded random bytes with the leading byte
nonzero. Inputs and caller output buffers are 64-byte aligned and reused.
The first label in each worker's rotated order produces the encoded fixture.
Before timing, every label must match that output and decode it back to the input.
Expected output is poisoned with its bytewise complement before each check. These checks establish
cross-revision agreement and round trips, not an independent encoding oracle.
Internal workspace allocation remains inside timing. Library loading, fixtures,
caller-buffer allocation and correctness checks are outside timing. The repeat
decode export uses the encoder's ASCII output through `from_utf8_unchecked`, so
warm timing excludes UTF-8 validation. Base58 digit validation remains inside the
public decoder.

The clock is process CPU time around repeated public calls in the cdylib;
the Python/FFI boundary occurs once per batch. A process/copy cell retains the
median of its three batches. Absolute times take each copy's median over
processes, then the median over copies. Ratios first take the median over copies
within each process, divide PR68 by main, then take the median of those paired
ratios. A paired ratio need not equal the ratio of the two absolute medians.
Ranges are observed process minima/maxima, **not confidence intervals**.
Individual batch values are not retained; the raw audit starts from process/copy
medians. No observations are dropped or adjusted using the control.

### Results and limits

M5 fat-LTO 1 MiB/4 MiB median ratios span **0.994–1.009×** across encode/decode and default/cache; duplicate-main control medians span **0.998–1.004×**.
GCP fat-LTO 1 MiB/4 MiB median ratios span **0.980–1.001×** across encode/decode and default/cache; duplicate-main control medians span **0.981–1.000×**.

Small-input slowdowns remain: M5 no-LTO/cache 512-byte encode is **1.046 [1.043, 1.048]×**, with identical-main control **1.000 [0.999, 1.005]×**. GCP no-LTO/cache 32-byte encode shifts in both PR68 and control (median **1.036× / 1.034×**). The process ranges and build/order limitations matter for these small effects.
The full process ranges and small-input cells matter. A near-unity median is not
proof of equivalent latency, and an unchanged-main control shift is non-source.

The following table selects the **largest median PR68/main ratio in each cohort**
over the complete 32 B–4 MiB set. It is a systematic view of the largest observed
slowdowns, with the matching identical-main control. Empty and one-byte cells
remain in the complete tables below. No focused repeat replaces these primary
observations.

| Host / LTO / policy | Payload / operation | PR68/main [range] | Control/main [range] |
| --- | --- | ---: | ---: |
| m5 / fat / default | 52 B / encode | 1.035 [1.003, 1.055] | 0.998 [0.993, 1.025] |
| m5 / fat / cache | 4 KiB / decode | 1.005 [0.995, 1.012] | 1.000 [0.993, 1.007] |
| m5 / off / default | 128 B / decode | 1.014 [0.982, 1.048] | 1.000 [0.997, 1.039] |
| m5 / off / cache | 512 B / encode | 1.046 [1.043, 1.048] | 1.000 [0.999, 1.005] |
| gcp / fat / default | 2 KiB / decode | 1.004 [1.001, 1.006] | 1.000 [0.998, 1.009] |
| gcp / fat / cache | 8 KiB / encode | 1.004 [0.989, 1.017] | 0.996 [0.981, 1.014] |
| gcp / off / default | 128 KiB / encode | 1.003 [1.002, 1.005] | 0.999 [0.998, 1.001] |
| gcp / off / cache | 32 B / encode | 1.036 [1.034, 1.038] | 1.034 [1.034, 1.036] |

![Current M5 PR68 paired ratios](../assets/graphs/m5-public.svg)

![Current GCP PR68 paired ratios](../assets/graphs/gcp-public.svg)

### Complete primary warm ratios

Each cell is PR68/main median [minimum, maximum] of paired process ratios.
The complete 27-size set includes zero and one byte; log-scale figures start at
32 bytes. Selected absolute fat-LTO timings are in the README.

<details>
<summary>Apple M5 Pro / fat: all paired ratios</summary>

| Decoded B | Default encode | Default decode | Cache encode | Cache decode |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 1.004 [0.978, 1.033] | 1.020 [0.957, 1.073] | 0.998 [0.966, 1.010] | 1.006 [0.925, 1.069] |
| 1 | 0.994 [0.941, 1.071] | 0.997 [0.989, 1.018] | 0.997 [0.894, 1.017] | 1.002 [0.999, 1.027] |
| 32 | 1.000 [0.921, 1.004] | 1.008 [0.996, 1.049] | 1.000 [0.995, 1.008] | 1.001 [0.996, 1.008] |
| 52 | 1.035 [1.003, 1.055] | 1.001 [0.958, 1.014] | 1.002 [0.991, 1.012] | 1.000 [0.993, 1.009] |
| 64 | 1.001 [0.998, 1.013] | 1.001 [0.991, 1.006] | 0.999 [0.996, 1.037] | 1.000 [0.998, 1.006] |
| 65 | 1.018 [0.990, 1.042] | 0.994 [0.970, 1.037] | 1.000 [0.989, 1.009] | 0.998 [0.987, 1.005] |
| 128 | 1.004 [0.992, 1.020] | 1.002 [0.984, 1.035] | 0.996 [0.985, 1.007] | 0.997 [0.988, 1.003] |
| 256 | 0.999 [0.993, 1.011] | 0.999 [0.996, 1.006] | 0.999 [0.993, 1.005] | 1.000 [0.983, 1.004] |
| 512 | 1.002 [0.996, 1.010] | 1.001 [0.995, 1.007] | 0.998 [0.993, 1.001] | 1.002 [0.996, 1.009] |
| 1,024 | 1.002 [1.000, 1.047] | 1.000 [0.989, 1.008] | 0.999 [0.991, 1.010] | 1.004 [0.998, 1.017] |
| 1,504 | 1.003 [1.000, 1.006] | 1.002 [0.997, 1.005] | 1.002 [0.998, 1.024] | 1.001 [0.990, 1.003] |
| 1,875 | 1.003 [1.001, 1.025] | 0.998 [0.987, 1.009] | 1.000 [0.992, 1.016] | 1.000 [0.990, 1.002] |
| 2,048 | 1.002 [1.001, 1.006] | 0.998 [0.990, 1.003] | 1.004 [0.994, 1.033] | 1.000 [0.990, 1.015] |
| 4,096 | 0.999 [0.994, 1.044] | 1.000 [0.993, 1.009] | 1.002 [0.995, 1.029] | 1.005 [0.995, 1.012] |
| 8,192 | 0.998 [0.979, 1.007] | 1.001 [0.995, 1.003] | 1.004 [0.980, 1.040] | 1.001 [0.995, 1.003] |
| 16,384 | 0.999 [0.996, 1.002] | 1.000 [0.982, 1.012] | 1.000 [0.997, 1.002] | 1.000 [0.996, 1.003] |
| 24,576 | 0.998 [0.994, 1.030] | 1.001 [0.994, 1.004] | 0.998 [0.987, 1.006] | 1.000 [0.985, 1.003] |
| 28,672 | 0.998 [0.993, 1.000] | 1.000 [0.998, 1.004] | 1.001 [0.997, 1.004] | 0.994 [0.990, 0.999] |
| 32,768 | 0.995 [0.982, 1.001] | 0.995 [0.982, 1.000] | 1.000 [0.987, 1.005] | 1.001 [0.997, 1.002] |
| 36,864 | 0.998 [0.968, 1.013] | 1.003 [0.995, 1.004] | 1.000 [0.990, 1.005] | 1.002 [0.999, 1.007] |
| 49,152 | 1.001 [0.996, 1.022] | 1.003 [0.997, 1.010] | 0.999 [0.987, 1.000] | 1.002 [0.972, 1.005] |
| 65,536 | 0.998 [0.994, 1.000] | 0.998 [0.981, 1.008] | 0.998 [0.991, 1.005] | 1.000 [0.957, 1.002] |
| 131,072 | 1.000 [0.983, 1.012] | 1.001 [0.915, 1.052] | 0.999 [0.989, 1.004] | 1.001 [0.998, 1.027] |
| 147,456 | 0.996 [0.985, 1.001] | 1.005 [0.988, 1.008] | 1.001 [0.997, 1.005] | 1.001 [0.995, 1.003] |
| 262,144 | 1.002 [0.995, 1.023] | 0.997 [0.973, 1.030] | 1.000 [0.996, 1.004] | 1.001 [0.997, 1.024] |
| 1,048,576 | 0.997 [0.988, 1.015] | 0.994 [0.979, 1.003] | 1.000 [0.995, 1.002] | 1.000 [0.994, 1.002] |
| 4,194,304 | 1.009 [0.982, 1.044] | 1.006 [0.977, 1.066] | 1.000 [0.998, 1.009] | 0.997 [0.993, 1.004] |

</details>

<details>
<summary>Apple M5 Pro / off: all paired ratios</summary>

| Decoded B | Default encode | Default decode | Cache encode | Cache decode |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.997 [0.797, 1.187] | 0.999 [0.950, 1.061] | 1.044 [0.955, 1.234] | 1.000 [0.999, 1.003] |
| 1 | 1.000 [0.873, 1.009] | 0.914 [0.820, 1.016] | 0.992 [0.880, 1.049] | 0.992 [0.980, 0.997] |
| 32 | 1.004 [0.967, 1.038] | 1.003 [0.995, 1.013] | 1.010 [0.989, 1.047] | 0.999 [0.995, 1.007] |
| 52 | 0.996 [0.927, 1.083] | 1.004 [0.975, 1.014] | 0.998 [0.982, 1.006] | 1.009 [0.997, 1.029] |
| 64 | 0.998 [0.982, 1.002] | 1.004 [0.996, 1.010] | 1.000 [0.999, 1.002] | 1.007 [0.984, 1.011] |
| 65 | 0.998 [0.934, 1.083] | 1.001 [0.924, 1.011] | 1.000 [0.985, 1.014] | 1.009 [1.003, 1.016] |
| 128 | 1.004 [0.944, 1.028] | 1.014 [0.982, 1.048] | 0.997 [0.972, 1.011] | 0.993 [0.975, 1.014] |
| 256 | 1.000 [0.950, 1.054] | 0.988 [0.948, 0.998] | 1.004 [0.999, 1.009] | 1.001 [0.994, 1.005] |
| 512 | 1.001 [0.979, 1.034] | 0.989 [0.954, 0.998] | 1.046 [1.043, 1.048] | 0.999 [0.992, 1.006] |
| 1,024 | 0.999 [0.978, 1.008] | 0.998 [0.964, 1.008] | 1.002 [0.996, 1.017] | 1.000 [0.992, 1.008] |
| 1,504 | 0.996 [0.976, 1.011] | 1.005 [0.978, 1.024] | 1.009 [0.994, 1.016] | 0.999 [0.997, 1.003] |
| 1,875 | 0.993 [0.982, 1.000] | 0.998 [0.989, 1.008] | 1.000 [0.988, 1.015] | 0.999 [0.997, 1.002] |
| 2,048 | 0.993 [0.982, 1.006] | 1.003 [0.990, 1.014] | 0.999 [0.977, 1.004] | 1.006 [0.986, 1.028] |
| 4,096 | 1.002 [0.995, 1.011] | 0.995 [0.994, 1.010] | 1.004 [0.997, 1.017] | 1.002 [0.959, 1.021] |
| 8,192 | 1.001 [0.989, 1.007] | 0.997 [0.970, 1.014] | 0.995 [0.984, 1.004] | 1.005 [1.003, 1.007] |
| 16,384 | 1.000 [0.976, 1.004] | 0.998 [0.979, 1.044] | 1.002 [0.995, 1.006] | 1.003 [0.999, 1.004] |
| 24,576 | 0.997 [0.974, 1.003] | 1.001 [0.995, 1.024] | 0.999 [0.997, 1.023] | 1.000 [0.971, 1.002] |
| 28,672 | 1.001 [0.996, 1.029] | 1.001 [0.996, 1.018] | 1.004 [1.000, 1.012] | 1.001 [0.980, 1.003] |
| 32,768 | 1.004 [0.981, 1.007] | 0.998 [0.985, 1.001] | 1.001 [0.996, 1.006] | 1.000 [1.000, 1.003] |
| 36,864 | 0.999 [0.994, 1.003] | 0.999 [0.966, 1.002] | 0.999 [0.991, 1.006] | 1.001 [0.990, 1.003] |
| 49,152 | 1.001 [0.997, 1.019] | 0.999 [0.985, 1.004] | 1.000 [0.997, 1.005] | 1.001 [1.000, 1.007] |
| 65,536 | 1.003 [0.995, 1.006] | 0.999 [0.973, 1.003] | 1.001 [0.998, 1.004] | 1.000 [0.976, 1.005] |
| 131,072 | 1.000 [0.992, 1.009] | 0.998 [0.989, 1.051] | 1.001 [0.910, 1.004] | 0.999 [0.995, 1.059] |
| 147,456 | 1.000 [0.963, 1.028] | 1.001 [0.997, 1.004] | 1.001 [0.997, 1.033] | 1.001 [0.999, 1.034] |
| 262,144 | 1.002 [0.989, 1.019] | 0.996 [0.975, 1.003] | 1.001 [0.996, 1.005] | 1.000 [0.998, 1.015] |
| 1,048,576 | 1.005 [0.980, 1.012] | 0.995 [0.968, 1.018] | 1.002 [0.984, 1.014] | 1.000 [0.959, 1.005] |
| 4,194,304 | 1.000 [0.987, 1.021] | 1.000 [0.982, 1.025] | 1.000 [0.999, 1.020] | 0.998 [0.974, 1.015] |

</details>

<details>
<summary>AMD EPYC 9B45 · GCP / fat: all paired ratios</summary>

| Decoded B | Default encode | Default decode | Cache encode | Cache decode |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 1.003 [0.992, 1.339] | 1.000 [0.604, 1.001] | 1.001 [0.983, 1.008] | 1.000 [1.000, 1.001] |
| 1 | 1.017 [0.965, 1.059] | 1.000 [0.913, 1.001] | 0.994 [0.990, 1.011] | 1.000 [0.979, 1.001] |
| 32 | 1.000 [0.939, 1.000] | 1.001 [0.999, 1.102] | 1.000 [1.000, 1.002] | 0.999 [0.971, 1.004] |
| 52 | 0.999 [0.998, 1.004] | 1.000 [0.997, 1.002] | 0.998 [0.997, 1.003] | 0.999 [0.985, 1.001] |
| 64 | 1.000 [1.000, 1.000] | 1.001 [1.000, 1.003] | 1.000 [0.997, 1.002] | 1.001 [0.988, 1.002] |
| 65 | 1.000 [0.999, 1.001] | 1.000 [0.997, 1.002] | 1.000 [0.997, 1.003] | 0.999 [0.989, 1.000] |
| 128 | 1.000 [0.970, 1.001] | 1.000 [0.998, 1.001] | 1.000 [0.998, 1.000] | 1.001 [0.998, 1.003] |
| 256 | 0.999 [0.993, 1.007] | 1.000 [0.999, 1.002] | 1.000 [0.979, 1.020] | 1.000 [0.995, 1.001] |
| 512 | 0.996 [0.992, 1.001] | 1.000 [0.998, 1.007] | 1.001 [0.997, 1.001] | 1.000 [0.996, 1.005] |
| 1,024 | 0.996 [0.991, 1.002] | 0.995 [0.988, 0.999] | 0.999 [0.997, 0.999] | 1.000 [0.999, 1.001] |
| 1,504 | 1.003 [0.996, 1.009] | 0.994 [0.991, 1.001] | 0.998 [0.997, 0.999] | 1.000 [0.997, 1.001] |
| 1,875 | 0.994 [0.990, 0.997] | 0.993 [0.989, 0.999] | 0.997 [0.981, 0.998] | 1.001 [1.000, 1.002] |
| 2,048 | 0.998 [0.993, 1.007] | 1.004 [1.001, 1.006] | 0.991 [0.984, 1.008] | 1.001 [0.999, 1.004] |
| 4,096 | 1.000 [0.993, 1.005] | 0.999 [0.994, 1.001] | 1.001 [0.989, 1.012] | 0.923 [0.894, 0.937] |
| 8,192 | 1.001 [0.988, 1.005] | 0.995 [0.993, 1.010] | 1.004 [0.989, 1.017] | 0.942 [0.918, 0.953] |
| 16,384 | 1.000 [0.987, 1.005] | 0.997 [0.990, 1.003] | 1.001 [0.997, 1.012] | 0.960 [0.945, 0.969] |
| 24,576 | 1.002 [0.994, 1.005] | 1.002 [1.000, 1.006] | 0.997 [0.994, 1.003] | 0.994 [0.988, 1.001] |
| 28,672 | 1.000 [0.999, 1.001] | 1.003 [0.998, 1.013] | 1.001 [0.993, 1.005] | 0.997 [0.989, 1.007] |
| 32,768 | 1.000 [0.999, 1.000] | 0.999 [0.997, 1.001] | 1.002 [0.993, 1.005] | 0.970 [0.961, 0.980] |
| 36,864 | 1.000 [0.996, 1.002] | 1.001 [0.998, 1.007] | 1.000 [0.998, 1.004] | 0.978 [0.966, 0.983] |
| 49,152 | 1.001 [0.999, 1.001] | 1.002 [0.998, 1.003] | 1.002 [0.994, 1.010] | 0.996 [0.988, 1.007] |
| 65,536 | 1.000 [0.999, 1.001] | 0.999 [0.996, 1.001] | 1.001 [0.997, 1.004] | 0.975 [0.965, 0.979] |
| 131,072 | 1.000 [0.999, 1.003] | 0.999 [0.997, 1.000] | 1.001 [0.997, 1.005] | 0.976 [0.967, 0.981] |
| 147,456 | 1.001 [0.999, 1.001] | 1.001 [0.997, 1.004] | 1.001 [0.998, 1.003] | 0.980 [0.977, 0.987] |
| 262,144 | 1.001 [1.000, 1.001] | 0.923 [0.922, 0.925] | 1.001 [0.995, 1.007] | 0.976 [0.972, 0.986] |
| 1,048,576 | 1.001 [0.999, 1.002] | 1.000 [0.998, 1.002] | 1.000 [0.998, 1.004] | 0.980 [0.976, 0.986] |
| 4,194,304 | 1.001 [0.997, 1.003] | 0.999 [0.997, 1.017] | 1.000 [0.997, 1.004] | 0.984 [0.974, 0.988] |

</details>

<details>
<summary>AMD EPYC 9B45 · GCP / off: all paired ratios</summary>

| Decoded B | Default encode | Default decode | Cache encode | Cache decode |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.909 [0.908, 0.910] | 1.000 [0.999, 1.001] | 1.000 [1.000, 1.001] | 1.000 [1.000, 1.001] |
| 1 | 1.000 [0.998, 1.001] | 0.957 [0.957, 1.005] | 0.917 [0.916, 0.918] | 1.000 [0.999, 1.000] |
| 32 | 1.000 [1.000, 1.001] | 0.984 [0.976, 0.988] | 1.036 [1.034, 1.038] | 0.992 [0.988, 1.000] |
| 52 | 1.000 [0.998, 1.005] | 0.966 [0.929, 0.983] | 1.004 [0.996, 1.007] | 1.002 [0.983, 1.006] |
| 64 | 1.000 [0.998, 1.001] | 0.976 [0.935, 0.986] | 1.002 [1.001, 1.003] | 0.994 [0.983, 1.004] |
| 65 | 1.000 [0.999, 1.001] | 0.979 [0.967, 0.983] | 1.004 [1.001, 1.010] | 0.990 [0.984, 0.999] |
| 128 | 0.998 [0.993, 1.001] | 0.981 [0.977, 0.985] | 1.008 [1.006, 1.019] | 0.996 [0.983, 1.003] |
| 256 | 1.000 [0.997, 1.001] | 0.974 [0.968, 0.982] | 0.996 [0.975, 1.017] | 1.013 [0.996, 1.018] |
| 512 | 0.998 [0.990, 1.003] | 0.992 [0.988, 0.993] | 1.000 [0.994, 1.004] | 1.000 [0.986, 1.001] |
| 1,024 | 1.001 [0.997, 1.004] | 0.996 [0.992, 1.001] | 0.999 [0.998, 1.001] | 0.999 [0.996, 1.001] |
| 1,504 | 1.000 [0.994, 1.004] | 0.984 [0.980, 0.990] | 0.998 [0.997, 1.001] | 0.997 [0.995, 1.000] |
| 1,875 | 0.999 [0.994, 1.007] | 0.989 [0.985, 0.993] | 0.999 [0.989, 1.002] | 0.997 [0.995, 0.999] |
| 2,048 | 1.000 [0.993, 1.004] | 0.997 [0.991, 1.002] | 0.995 [0.981, 1.003] | 0.996 [0.993, 0.998] |
| 4,096 | 0.997 [0.987, 1.000] | 0.992 [0.986, 0.994] | 1.003 [0.994, 1.007] | 1.010 [1.003, 1.013] |
| 8,192 | 1.000 [0.995, 1.006] | 0.995 [0.991, 0.996] | 1.013 [0.995, 1.017] | 1.010 [1.000, 1.013] |
| 16,384 | 1.001 [0.995, 1.004] | 0.996 [0.991, 0.999] | 1.010 [1.000, 1.015] | 1.005 [1.002, 1.010] |
| 24,576 | 1.002 [1.001, 1.006] | 0.997 [0.994, 0.998] | 1.001 [1.000, 1.003] | 1.003 [0.999, 1.004] |
| 28,672 | 1.001 [1.001, 1.003] | 0.997 [0.993, 1.007] | 1.004 [0.996, 1.010] | 1.000 [0.996, 1.003] |
| 32,768 | 1.001 [0.999, 1.003] | 1.000 [0.986, 1.010] | 1.005 [0.998, 1.009] | 1.002 [0.999, 1.008] |
| 36,864 | 0.997 [0.995, 1.000] | 0.998 [0.995, 1.001] | 1.002 [0.998, 1.005] | 1.001 [0.998, 1.006] |
| 49,152 | 1.001 [0.997, 1.004] | 0.997 [0.996, 0.999] | 1.000 [0.993, 1.003] | 1.000 [0.999, 1.005] |
| 65,536 | 1.002 [1.000, 1.002] | 0.998 [0.996, 1.003] | 1.003 [0.997, 1.009] | 1.001 [0.998, 1.003] |
| 131,072 | 1.003 [1.002, 1.005] | 0.996 [0.995, 1.001] | 1.003 [0.999, 1.007] | 1.003 [1.000, 1.005] |
| 147,456 | 0.999 [0.996, 1.001] | 0.999 [0.997, 1.001] | 0.997 [0.993, 1.002] | 0.999 [0.995, 1.004] |
| 262,144 | 1.002 [1.000, 1.003] | 0.927 [0.923, 0.937] | 1.003 [0.997, 1.040] | 1.002 [0.998, 1.006] |
| 1,048,576 | 1.002 [1.001, 1.004] | 0.998 [0.996, 1.000] | 1.002 [0.999, 1.007] | 0.999 [0.998, 1.003] |
| 4,194,304 | 1.002 [1.000, 1.004] | 0.998 [0.994, 1.003] | 1.001 [0.998, 1.003] | 1.001 [0.990, 1.004] |

</details>


## First-call decode

Each input is `"2"` repeated 1,408, 2,816, 5,632 or 32,768 times. Expected bytes
are independently derived with Python integer arithmetic. Each cell uses
**18 fresh processes × 2 file copies per label**, with one first call per copy.
Order covers all six cyclic starting positions three times. The output is
poisoned before the call. The Python process CPU clock surrounds one FFI call;
UTF-8 validation is inside this single-call timer. Ratios use within-process
two-copy medians. Absolute time is the median over copies of each copy's median
over processes. Short calls have coarse resolution and wide ranges. These data
do not establish a general first-call win or first-call latency at 1 MiB/4 MiB.

| Host / LTO / policy | Input chars | PR68 time | PR68/main [range] | Control/main [range] |
| --- | ---: | ---: | ---: | ---: |
| m5 / fat / default | 1,408 | 5 µs | 1.056 [0.562, 2.000] | 0.944 [0.688, 1.778] |
| m5 / fat / default | 2,816 | 10 µs | 1.000 [0.708, 1.316] | 0.976 [0.741, 1.353] |
| m5 / fat / default | 5,632 | 29.2 µs | 1.017 [0.887, 1.323] | 0.992 [0.885, 1.119] |
| m5 / fat / default | 32,768 | 320 µs | 1.009 [0.953, 1.045] | 0.992 [0.949, 1.062] |
| m5 / fat / cache | 1,408 | 55 µs | 1.009 [0.685, 1.131] | 1.008 [0.901, 1.087] |
| m5 / fat / cache | 2,816 | 188 µs | 0.996 [0.932, 1.089] | 0.997 [0.955, 1.122] |
| m5 / fat / cache | 5,632 | 730 µs | 1.006 [0.978, 1.048] | 1.008 [0.972, 1.028] |
| m5 / fat / cache | 32,768 | 493 µs | 1.006 [0.960, 1.192] | 1.004 [0.958, 1.094] |
| m5 / off / default | 1,408 | 8.5 µs | 1.067 [0.760, 1.312] | 0.906 [0.750, 1.438] |
| m5 / off / default | 2,816 | 14 µs | 1.055 [0.867, 1.185] | 0.963 [0.812, 1.192] |
| m5 / off / default | 5,632 | 33.2 µs | 1.030 [0.929, 1.092] | 0.977 [0.928, 1.111] |
| m5 / off / default | 32,768 | 334 µs | 0.996 [0.957, 1.049] | 1.003 [0.943, 1.037] |
| m5 / off / cache | 1,408 | 60.2 µs | 1.009 [0.584, 1.074] | 1.004 [0.671, 1.066] |
| m5 / off / cache | 2,816 | 209 µs | 1.040 [0.934, 1.152] | 0.989 [0.914, 1.090] |
| m5 / off / cache | 5,632 | 828 µs | 0.989 [0.949, 1.042] | 0.997 [0.955, 1.025] |
| m5 / off / cache | 32,768 | 528 µs | 0.997 [0.958, 1.040] | 0.995 [0.949, 1.043] |
| gcp / fat / default | 1,408 | 16.8 µs | 1.059 [0.910, 1.233] | 1.005 [0.844, 1.234] |
| gcp / fat / default | 2,816 | 38.9 µs | 1.028 [0.919, 1.100] | 1.013 [0.856, 1.095] |
| gcp / fat / default | 5,632 | 65.9 µs | 1.040 [0.989, 1.108] | 1.066 [0.986, 1.280] |
| gcp / fat / default | 32,768 | 324 µs | 1.030 [0.961, 1.069] | 1.007 [0.939, 1.095] |
| gcp / fat / cache | 1,408 | 87.7 µs | 1.006 [0.830, 1.216] | 0.983 [0.885, 1.143] |
| gcp / fat / cache | 2,816 | 319 µs | 1.006 [0.967, 1.052] | 1.001 [0.972, 1.058] |
| gcp / fat / cache | 5,632 | 47.4 µs | 0.994 [0.904, 1.048] | 1.009 [0.915, 1.233] |
| gcp / fat / cache | 32,768 | 310 µs | 1.015 [0.934, 1.075] | 0.992 [0.938, 1.051] |
| gcp / off / default | 1,408 | 29.2 µs | 0.968 [0.763, 1.087] | 0.959 [0.837, 1.140] |
| gcp / off / default | 2,816 | 52.7 µs | 0.985 [0.910, 1.068] | 1.032 [0.957, 1.074] |
| gcp / off / default | 5,632 | 76.2 µs | 1.013 [0.820, 1.034] | 1.029 [0.833, 1.142] |
| gcp / off / default | 32,768 | 367 µs | 1.013 [0.921, 1.059] | 1.001 [0.949, 1.083] |
| gcp / off / cache | 1,408 | 99.8 µs | 1.003 [0.859, 1.097] | 0.993 [0.897, 1.169] |
| gcp / off / cache | 2,816 | 324 µs | 1.003 [0.966, 1.030] | 1.005 [0.986, 1.065] |
| gcp / off / cache | 5,632 | 76.7 µs | 0.994 [0.942, 1.068] | 1.019 [0.976, 1.113] |
| gcp / off / cache | 32,768 | 332 µs | 1.022 [0.953, 1.063] | 0.985 [0.946, 1.051] |

## Linked size and compact

All PR68 columns below are **Auto-both minus the matching empty-both consumer**
from the timing build; all Δ columns compare that PR68 difference with main's
corresponding difference. Code, readonly, initialized data and BSS are section
totals; stripped file bytes include layout and metadata. Mach-O and ELF section
categories are not directly interchangeable. Signed BSS differences can be
negative after subtracting the scaffold. BSS movement is not retained-heap
measurement. `.rlib` size is not deployment size. Path labels have equal lengths.

| Host / LTO / policy | PR68 code B | PR68 readonly B | Code Δ B | Readonly Δ B | PR68 data B | Data Δ B | PR68 BSS B | BSS Δ B | PR68 stripped B | Stripped Δ B |
| --- |  ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| m5 / fat / default | 68,592 | 25,552 | +832 | +80 | 16 | +0 | 0 | +0 | 82,784 | +0 |
| m5 / fat / cache | 77,000 | 28,449 | +832 | +80 | 2,192 | +0 | 0 | +0 | 99,328 | +0 |
| m5 / off / default | 81,752 | 50,698 | +1,024 | +104 | 8 | +0 | 0 | +0 | 132,432 | +0 |
| m5 / off / cache | 85,784 | 58,732 | +1,024 | +104 | 2,184 | +0 | 0 | +0 | 148,992 | +16,528 |
| gcp / fat / default | 310,109 | 38,216 | +1,952 | +60 | 6,248 | +0 | -2,336 | -2,016 | 354,600 | +2,016 |
| gcp / fat / cache | 305,805 | 39,036 | +2,016 | +76 | 9,136 | +0 | -3,650 | -2,080 | 354,000 | +2,080 |
| gcp / off / default | 333,971 | 75,820 | +1,184 | +112 | 10,328 | +0 | 1,768 | +2,800 | 420,120 | +1,296 |
| gcp / off / cache | 325,859 | 71,608 | +1,152 | -208 | 11,464 | +0 | -1,230 | -944 | 408,936 | +944 |

`compact` selects eight rather than twelve x86 IFMA widths. It trades linked code
for more chunks on some multiplication shapes. This cohort measures the default
twelve-width policy only. No new compact, no-allocation or Wasm size claim is made.

## Correctness boundary

Every timing worker checks its fixtures before measurement. The source commit's
[Verification workflow](https://github.com/elichai/faster-base58/actions/runs/43297208704)
also passed, including custom Miri, sanitizers, proofs, normal tests and SIMD
inlining checks. Timing is not a substitute for those checks or for emitted-code
inspection. This cohort makes no invalid-input, owned-output, Base58Check,
allocation-counting, native-CPU, no-allocation or Wasm latency claim.

## Cache and memory

`cache` changes dispatch and leaf policy as well as retention. Cached first calls
can build tables, so warm results are not cold-start predictions. Code-derived
retained capacities and ownership are in
[DESIGN.md](../DESIGN.md#heap-stack-and-cache-costs). Per-call scratch, owned tables,
allocator overhead and concurrent calls add to retention. It is not an active-heap
bound. Wasm linear-memory pages need not return to the host.

### Counted heap usage

Measured **2026-11-12** at
[`1df40babe367`](https://github.com/elichai/faster-base58/commit/1df40babe367aea8978a5c99d67657e53e1abf5e)
on the Apple M5 Pro, macOS 26.7.3, native `aarch64-apple-darwin` with NEON,
Rust 1.99.1 (58cf8f9c3) / LLVM 23.1.1. Builds use release opt-level 3, 16 codegen
units, fat LTO and empty `RUSTFLAGS`. The internal Auto workers enable
`__internals,count-allocations`, with separate default and `cache` builds; `check`
and `compact` are disabled. These are separate, untimed allocation measurements,
with no timing clock; they are not ordinary public-consumer latency measurements.

`scripts/resource_report.py` uses deterministic size-seeded byte fixtures with a
nonzero leading byte, caller output buffers, and independent Malachite fixture
conversion. Each size/operation/policy uses three fresh worker processes. Each
process measures its first conversion and then one warm conversion, checking the
returned length and output bytes. All three processes produced identical counts
and byte totals for each cell.

Requests count allocations plus reallocations. Peaks are extra live requested Rust
layout bytes above the live-byte count immediately before each conversion; caller
buffers are excluded. First-call peaks include tables initialized during that
call. Warm peaks exclude tables retained from the first call. First-call retained
bytes remain live after that conversion; every measured warm call retains zero
additional bytes. These values are not RSS, allocator usable sizes, stack high
water or a bound for other inputs and call histories.

| Payload | Policy | Operation | First-call requests | First-call peak B | Warm requests | Warm peak B | First-call retained B |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 1 MiB | default | encode | 7 | 17,787,780 | 7 | 17,787,780 | 0 |
| 1 MiB | default | decode | 16 | 13,864,448 | 16 | 13,864,448 | 0 |
| 1 MiB | cache | encode | 7 | 17,787,780 | 7 | 17,787,780 | 0 |
| 1 MiB | cache | decode | 19 | 14,030,784 | 13 | 9,932,288 | 4,090,048 |
| 4 MiB | default | encode | 7 | 69,574,884 | 7 | 69,574,884 | 0 |
| 4 MiB | default | decode | 18 | 53,113,080 | 18 | 53,113,080 | 0 |
| 4 MiB | cache | encode | 7 | 69,574,884 | 7 | 69,574,884 | 0 |
| 4 MiB | cache | decode | 21 | 53,746,680 | 18 | 53,113,080 | 616,704 |

Reproduce with `scripts/resource_report.py build --count-allocations` and
`run --mode allocations --backends auto --sizes 1048576,4194304 --repeats 3 --samples 1`,
supplying fresh metadata/output paths. Repeat the build with `--cache`. Source and
binary hashes, environment and raw allocation records are retained locally under
ignored `target/measurements/pr68-heap/`.

## Measure and update

Use an idle host, AC power for laptops, and ignored `target/measurements/` output.
Freeze each source by commit and build all comparison artifacts before timing.
On the shared VM, use `/mnt/bench` and wait for its shared lock. Do not overlap
our timing with builds or tests. Example public-consumer builds (repeat for the
second source and cache):

```sh
CARGO_PROFILE_RELEASE_LTO=fat python3 -B scripts/code_size.py --source . --output target/measurements/native-fat --target aarch64-apple-darwin --cpu generic --backends empty,auto --directions both --kinds cdylib
CARGO_PROFILE_RELEASE_LTO=off python3 -B scripts/code_size.py --source . --output target/measurements/native-off --target aarch64-apple-darwin --cpu generic --backends empty,auto --directions both --kinds cdylib
```

Use `x86_64-unknown-linux-gnu` and `--strip-tool strip` on GCP; pin timing with
`--cpu 1`. Pass `--cache` for that policy. The ordinary harness is
`scripts/public_bench.py`, with `LABEL=PATH` probes and `--baseline main` for
paired ratios. This study's retained `public_bench_verified.py` derivative adds
output poison before untimed checks and retains process/copy medians. Its timed
loops are unchanged. Use `--processes 9 --copies 3 --batches 3 --min-ms 5` for
the warm contract above. All nine cyclic starts still leave load and call order
linked. Independent placement/order experiments are needed to isolate small
source effects when identical-main controls also shift.

Raw files, source snapshots, artifacts, commands, environment, script/source/
binary hashes and the study-specific exporter/renderer remain under ignored
`target/measurements/pr68-native/`; GCP also retains `/mnt/bench/pr68-native`.
The exporter validates all 864 warm/control rows, 64 cold/control rows and 8
coupled size rows against complete cell sets, checks 64 actual artifact hashes,
and verifies source/harness hashes and Cargo fingerprints. These checks validate
recorded provenance and aggregation, not an independent rebuild, individual
batch data, or the absence of unrelated host activity. Generated CSV/JSON and run
records stay outside commits.

With the pinned `assets/graphs/requirements.txt` environment, render the ignored
exports into a **new** directory and check the maintained documents:

```sh
python3 -B target/measurements/pr68-native/render.py --data target/measurements/pr68-native/published-data --output target/measurements/pr68-native/NEW_FIGURES
python3 -B assets/graphs/render.py --check
python3 -B -m unittest discover -s assets/graphs -p test_render.py
```

The figure/link check validates local links and SVG syntax. It does not verify
numbers or reproduce pixels. The retained renderer validates dataset keys and
source bindings and emits the 27-size tables with the figures. The Wasm figure
marks the current measurement gap.

## Other measurement scopes

Wasm scalar/SIMD128, comparator, internal backend, dispatch-threshold, allocation,
Base58Check and compact experiments need separate, source-bound cohorts. Wasm
timing uses Node/V8 elapsed time, not native process CPU time. Build with
`--target wasm32-unknown-unknown`, optionally `--wasm-simd128`, and use
`scripts/public_bench.py --wasm`. Allocation-counting modules are excluded from
ordinary timing. Criterion benches use wall time and may use owned comparator
results while Auto reuses caller output; report that difference.

For traversal changes, inspect both fat/off LTO and default/cache. Compare linked
hot loops as well as symbol lists with
`scripts/check_simd_inlining.py --compare OLD NEW`. Iterator forms can remove
bounds checks while retaining constructor calls or changing vectorization and
spills. Fixed-address code layout, dynamic mapping, file-page placement and
background load can affect small cells. Assembly is evidence of emitted work,
not proof of a latency improvement. Remeasure affected tables and SVGs together
when their source changes, or withdraw the results.

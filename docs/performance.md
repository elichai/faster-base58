# Performance and resource tradeoffs

The [README](../README.md#performance) summarizes the current native timings and
linked sizes. [DESIGN.md](../DESIGN.md) describes algorithms, dispatch and memory
ownership. [CORRECTNESS.md](../CORRECTNESS.md) describes validation and its limits.

## Current native results

Measured **2026-11-12**. Main is
[`65ea1581aae4d5da128abd70dcac3ead50cb7f65`](https://github.com/elichai/faster-base58/commit/65ea1581aae4d5da128abd70dcac3ead50cb7f65);
the full #53 → #55 → #57 → #59 → #67 stack is
[`3fcfbaff62f3257ddc6de357f4856cf6225eabf8`](https://github.com/elichai/faster-base58/commit/3fcfbaff62f3257ddc6de357f4856cf6225eabf8).
The stack contains #67's debug-only redundant Horner validity assertion and cold
invalid-byte helper, plus the direct Power-array construction. This comparison
measures the whole stack against main, not each layer or #67 alone. Subsequent
documentation changes leave the measured Rust, Cargo and consumer inputs unchanged.

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
effective environment override.

GCP work uses `/mnt/bench/pr67-stack-gcp-20261112`, waits for the shared VM lock,
and pins timing to vCPU 1. GCP timing was launched first; M5 timing followed the
user's later local authorization. M5 starts and finishes each warm/cold cohort on
AC with no reported thermal warning. Our timing phases contain no builds/tests.
The retained launch scripts use the shared lock; the primary archive alone is
not an independent attestation that no unrelated VM work occurred.

### Warm measurement contract

Each of the eight host/LTO/policy cohorts uses **5 fresh processes × 3 fresh file
copies per label × 3 interleaved batches**, calibrated to at least 5 ms per batch.
`main` and `ctrl` are separate copies of the **same main artifact**. `head` is the
stack. The loader verifies that exported function addresses are distinct.
Worker order rotates through five of nine possible starting positions. Inside
each worker, calibration follows that order and batches rotate by 0, 1 and 2.
This partially balances order; it does not independently counterbalance loader
position, call order and physical file placement.

Fixtures are deterministic random bytes with the leading byte nonzero. Inputs
and caller output buffers are 64-byte aligned and reused. Every label's encode
and decode result is checked before timing; expected output is poisoned with its
bytewise complement first. Internal workspace allocation remains inside timing.
Library loading, fixtures, caller-buffer allocation and correctness checks are
outside timing. The repeat-decode export uses a prevalidated UTF-8 fixture, so
UTF-8 validation is outside this warm timer. Base58 digit validation remains in
the public decoder. Warm results do not measure invalid-input latency.

The clock is process CPU time around repeated public calls inside the cdylib;
the Python/FFI boundary occurs once per batch. A retained process/copy cell is the
median of its three batches. Absolute times take each copy's median over processes,
then the median over copies. Ratios first take the median over copies within each
process, divide candidate by main, then report the median of those paired ratios.
These aggregation orders differ: a reported paired ratio need not equal the ratio
of the two absolute medians. Ranges are observed process minima/maxima, **not
confidence intervals**. Individual batch values are not retained, so the raw audit
starts from process/copy medians. No observations were dropped.

### Results and limits

The matrix does not reproduce the large encoder regression. Fat/default 1 MiB
and 4 MiB encode ratios are about 0.986 on M5 and 0.994 on GCP. M5 no-LTO/default
4 KiB decode is 0.861 [0.841, 0.869]. GCP no-LTO/cache encode at 256–1,024 bytes
is about 0.932–0.945. These are specific artifact measurements, not universal wins.

M5 fat/cache 512-byte decode is **1.027 [1.022, 1.029]**, with control
**1.002 [0.996, 1.004]**. A separate 7-process × 5-copy × 3-batch repeat with
15 ms batches gives **1.016 [1.011, 1.020]**, control
**0.999 [0.997, 1.004]**. This slowdown remains. Other small slowdowns and wide
ranges remain in the tables. Removing redundant arithmetic does not establish
that the changed whole binary cannot regress through layout or register allocation.

GCP no-LTO/default 32-byte encode is **1.047 [1.045, 1.049]**, but the identical-main
control is **1.048 [1.048, 1.048]**. The 7-process/5-copy repeat gives head
**1.083 [1.081, 1.100]**, control **1.048 [1.043, 1.048]**. Copy medians fall into
stable fast/slow bands; their mixtures select different label medians. A separate
9-process/3-copy repeat covers all nine cyclic loader starting positions, records
load index and encode-export address, and gives head **1.000 [0.998, 1.004]** and
control **1.000 [0.999, 1.002]**. This closes neither physical placement nor
loader-versus-timed-order causality: those orders remain linked, and artifacts
were not permuted across neutral slots. These cohorts do not isolate the source
contribution to the earlier head shifts. The identical-main shift is non-source.
No control adjustment is applied. All three cohorts remain separate.

![Current M5 whole-stack paired ratios](../assets/graphs/m5-public.svg)

![Current GCP whole-stack paired ratios](../assets/graphs/gcp-public.svg)

### Complete primary warm ratios

Each cell is stack/main median [minimum, maximum] of paired process ratios.
The complete 27-size set includes zero and one byte; log-scale figures start at
32 bytes. Absolute fat-LTO timings for selected sizes are in the README.

<details>
<summary>Apple M5 Pro / fat: all paired ratios</summary>

| Decoded B | Default encode | Default decode | Cache encode | Cache decode |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 1.000 [0.964, 1.020] | 0.813 [0.784, 0.856] | 0.990 [0.965, 1.012] | 0.985 [0.957, 1.009] |
| 1 | 0.991 [0.982, 1.033] | 0.842 [0.838, 0.843] | 0.988 [0.959, 1.038] | 0.999 [0.994, 1.002] |
| 32 | 0.998 [0.994, 1.001] | 0.993 [0.989, 1.007] | 1.007 [1.004, 1.008] | 0.977 [0.966, 0.983] |
| 52 | 0.984 [0.963, 1.007] | 0.983 [0.982, 0.996] | 1.006 [0.998, 1.012] | 0.987 [0.980, 0.997] |
| 64 | 1.001 [1.000, 1.005] | 0.990 [0.982, 1.000] | 1.004 [0.964, 1.009] | 0.977 [0.968, 0.979] |
| 65 | 0.989 [0.982, 1.016] | 0.997 [0.978, 0.998] | 0.894 [0.875, 0.921] | 0.959 [0.948, 0.970] |
| 128 | 0.979 [0.972, 0.993] | 0.990 [0.983, 0.994] | 0.950 [0.891, 0.967] | 0.988 [0.983, 1.000] |
| 256 | 0.932 [0.926, 0.954] | 0.972 [0.966, 0.975] | 0.923 [0.909, 0.925] | 0.976 [0.971, 0.980] |
| 512 | 0.951 [0.947, 0.956] | 0.974 [0.971, 0.975] | 0.981 [0.962, 0.987] | 1.027 [1.022, 1.029] |
| 1,024 | 0.965 [0.964, 0.968] | 0.970 [0.964, 0.972] | 1.004 [1.001, 1.007] | 0.999 [0.996, 1.003] |
| 1,504 | 0.956 [0.953, 0.959] | 0.965 [0.957, 0.982] | 1.002 [0.996, 1.007] | 0.997 [0.994, 1.026] |
| 1,875 | 0.952 [0.948, 0.955] | 0.940 [0.936, 0.952] | 1.005 [1.000, 1.021] | 0.997 [0.994, 1.004] |
| 2,048 | 0.951 [0.949, 0.952] | 0.944 [0.933, 0.951] | 1.001 [0.995, 1.029] | 0.991 [0.940, 1.010] |
| 4,096 | 0.986 [0.984, 0.991] | 0.946 [0.904, 0.955] | 1.002 [0.991, 1.004] | 0.999 [0.997, 1.007] |
| 8,192 | 0.986 [0.985, 0.989] | 0.989 [0.970, 0.995] | 1.001 [0.988, 1.004] | 1.005 [1.004, 1.010] |
| 16,384 | 0.986 [0.981, 0.992] | 0.985 [0.980, 0.988] | 1.005 [1.001, 1.006] | 1.006 [1.006, 1.007] |
| 24,576 | 0.982 [0.979, 0.984] | 0.982 [0.981, 0.985] | 0.999 [0.989, 1.001] | 1.003 [1.001, 1.004] |
| 28,672 | 0.979 [0.975, 0.987] | 0.978 [0.975, 0.981] | 1.000 [0.953, 1.006] | 1.005 [1.002, 1.007] |
| 32,768 | 0.981 [0.980, 0.984] | 0.987 [0.983, 0.992] | 1.005 [0.996, 1.008] | 1.009 [1.006, 1.015] |
| 36,864 | 0.989 [0.983, 0.995] | 0.988 [0.977, 0.990] | 1.004 [1.001, 1.008] | 1.002 [1.000, 1.009] |
| 49,152 | 0.985 [0.974, 0.986] | 0.982 [0.972, 0.991] | 0.999 [0.995, 1.003] | 1.005 [1.002, 1.006] |
| 65,536 | 0.986 [0.982, 0.989] | 0.984 [0.979, 0.986] | 1.002 [0.997, 1.007] | 1.009 [1.008, 1.011] |
| 131,072 | 0.983 [0.981, 0.987] | 0.986 [0.983, 0.991] | 1.002 [1.000, 1.005] | 1.011 [1.007, 1.015] |
| 147,456 | 0.985 [0.982, 0.993] | 0.984 [0.980, 0.987] | 1.009 [1.004, 1.009] | 1.006 [1.004, 1.070] |
| 262,144 | 0.983 [0.982, 0.985] | 0.987 [0.981, 0.991] | 1.002 [1.000, 1.007] | 1.008 [1.008, 1.010] |
| 1,048,576 | 0.986 [0.982, 0.987] | 0.988 [0.920, 0.991] | 1.002 [0.996, 1.003] | 1.009 [1.007, 1.010] |
| 4,194,304 | 0.986 [0.984, 0.996] | 0.989 [0.987, 0.991] | 1.003 [1.001, 1.007] | 1.004 [1.002, 1.007] |

</details>

<details>
<summary>Apple M5 Pro / off: all paired ratios</summary>

| Decoded B | Default encode | Default decode | Cache encode | Cache decode |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.970 [0.868, 1.109] | 0.996 [0.983, 1.007] | 0.991 [0.876, 1.143] | 1.017 [1.009, 1.023] |
| 1 | 0.913 [0.826, 1.009] | 0.996 [0.993, 1.007] | 0.937 [0.888, 1.116] | 1.006 [0.999, 1.015] |
| 32 | 1.015 [0.986, 1.027] | 0.951 [0.929, 0.960] | 1.001 [0.989, 1.022] | 0.965 [0.938, 0.974] |
| 52 | 1.002 [0.936, 1.050] | 0.974 [0.956, 0.982] | 0.991 [0.971, 0.999] | 0.986 [0.979, 0.994] |
| 64 | 1.001 [0.990, 1.010] | 0.974 [0.965, 0.983] | 0.989 [0.985, 0.990] | 0.974 [0.972, 0.978] |
| 65 | 0.998 [0.953, 1.044] | 0.970 [0.965, 0.975] | 0.993 [0.983, 1.000] | 0.977 [0.970, 0.983] |
| 128 | 0.989 [0.973, 1.049] | 0.986 [0.973, 0.997] | 0.996 [0.985, 1.004] | 1.001 [0.987, 1.004] |
| 256 | 0.957 [0.938, 0.968] | 0.986 [0.960, 1.007] | 1.001 [0.976, 1.012] | 0.998 [0.986, 1.002] |
| 512 | 0.972 [0.966, 1.000] | 1.018 [1.004, 1.021] | 0.993 [0.990, 0.995] | 1.016 [1.013, 1.019] |
| 1,024 | 0.969 [0.964, 0.980] | 0.971 [0.962, 0.973] | 0.994 [0.991, 1.003] | 1.013 [1.007, 1.015] |
| 1,504 | 0.969 [0.966, 0.974] | 0.923 [0.917, 0.933] | 1.004 [1.000, 1.008] | 1.005 [1.002, 1.007] |
| 1,875 | 0.969 [0.962, 0.983] | 0.896 [0.891, 0.902] | 1.002 [0.991, 1.007] | 1.008 [1.001, 1.010] |
| 2,048 | 0.962 [0.935, 0.964] | 0.887 [0.882, 0.898] | 1.001 [0.984, 1.022] | 1.004 [1.003, 1.009] |
| 4,096 | 1.020 [1.005, 1.031] | 0.861 [0.841, 0.869] | 0.990 [0.986, 1.000] | 1.003 [0.956, 1.010] |
| 8,192 | 1.008 [0.991, 1.019] | 0.999 [0.971, 1.017] | 0.999 [0.993, 1.004] | 0.999 [0.990, 1.000] |
| 16,384 | 1.006 [0.996, 1.009] | 0.989 [0.986, 1.012] | 1.011 [1.004, 1.014] | 1.005 [1.003, 1.006] |
| 24,576 | 1.001 [0.991, 1.003] | 0.987 [0.964, 0.994] | 1.001 [0.999, 1.009] | 1.006 [1.003, 1.011] |
| 28,672 | 1.009 [1.001, 1.012] | 0.985 [0.964, 0.997] | 1.007 [1.004, 1.012] | 1.008 [1.006, 1.011] |
| 32,768 | 0.996 [0.995, 1.005] | 0.991 [0.981, 1.014] | 1.004 [0.990, 1.015] | 1.004 [1.001, 1.005] |
| 36,864 | 1.000 [0.993, 1.010] | 0.994 [0.990, 1.011] | 1.000 [0.997, 1.005] | 1.008 [0.991, 1.010] |
| 49,152 | 1.007 [1.006, 1.016] | 0.984 [0.972, 0.998] | 1.001 [0.998, 1.002] | 1.003 [1.001, 1.005] |
| 65,536 | 1.001 [0.997, 1.013] | 0.998 [0.992, 1.003] | 1.012 [0.948, 1.042] | 1.003 [1.001, 1.006] |
| 131,072 | 1.003 [0.996, 1.006] | 0.998 [0.992, 1.004] | 1.008 [1.003, 1.086] | 1.003 [1.000, 1.005] |
| 147,456 | 0.999 [0.996, 1.009] | 0.988 [0.983, 0.992] | 1.000 [0.996, 1.003] | 1.007 [0.999, 1.009] |
| 262,144 | 1.005 [1.002, 1.010] | 0.998 [0.992, 0.999] | 1.006 [1.003, 1.059] | 1.001 [1.001, 1.006] |
| 1,048,576 | 1.006 [1.001, 1.010] | 0.997 [0.993, 0.999] | 1.005 [1.002, 1.009] | 1.004 [0.999, 1.011] |
| 4,194,304 | 1.002 [0.995, 1.004] | 0.996 [0.994, 1.001] | 1.003 [1.002, 1.003] | 1.003 [1.001, 1.025] |

</details>

<details>
<summary>AMD EPYC 9B45 · GCP / fat: all paired ratios</summary>

| Decoded B | Default encode | Default decode | Cache encode | Cache decode |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.985 [0.983, 0.990] | 0.935 [0.933, 0.936] | 1.010 [1.006, 1.011] | 0.888 [0.887, 0.889] |
| 1 | 0.970 [0.930, 0.975] | 0.960 [0.950, 0.980] | 1.001 [0.989, 1.006] | 0.883 [0.880, 0.884] |
| 32 | 1.001 [0.999, 1.003] | 1.003 [0.999, 1.008] | 1.001 [1.000, 1.001] | 0.987 [0.984, 1.047] |
| 52 | 1.016 [1.010, 1.016] | 1.000 [1.000, 1.017] | 0.999 [0.997, 1.003] | 0.984 [0.982, 1.028] |
| 64 | 0.995 [0.994, 0.997] | 0.988 [0.986, 0.999] | 1.002 [1.000, 1.007] | 1.013 [1.012, 1.060] |
| 65 | 1.008 [1.008, 1.053] | 1.003 [0.982, 1.026] | 0.999 [0.997, 1.002] | 1.014 [1.013, 1.053] |
| 128 | 0.992 [0.992, 1.014] | 0.995 [0.994, 0.998] | 1.000 [0.998, 1.002] | 1.000 [0.999, 1.029] |
| 256 | 0.981 [0.974, 0.987] | 0.985 [0.975, 0.995] | 1.001 [0.983, 1.019] | 0.972 [0.971, 1.001] |
| 512 | 0.991 [0.987, 1.000] | 0.971 [0.968, 0.972] | 1.000 [1.000, 1.004] | 0.985 [0.982, 0.999] |
| 1,024 | 0.993 [0.985, 0.995] | 0.975 [0.970, 0.983] | 1.001 [0.998, 1.002] | 1.007 [1.004, 1.008] |
| 1,504 | 0.986 [0.984, 0.991] | 0.970 [0.967, 0.972] | 1.000 [0.999, 1.002] | 1.002 [1.001, 1.005] |
| 1,875 | 0.985 [0.983, 0.990] | 0.979 [0.967, 0.983] | 1.000 [0.999, 1.001] | 1.000 [1.000, 1.003] |
| 2,048 | 0.983 [0.980, 0.992] | 0.978 [0.967, 0.983] | 1.000 [0.999, 1.003] | 1.000 [1.000, 1.001] |
| 4,096 | 0.988 [0.985, 0.993] | 0.986 [0.974, 1.003] | 0.997 [0.988, 1.000] | 0.992 [0.971, 1.029] |
| 8,192 | 0.994 [0.990, 0.996] | 0.987 [0.982, 0.990] | 0.992 [0.986, 1.006] | 0.987 [0.980, 0.992] |
| 16,384 | 0.995 [0.991, 1.000] | 0.986 [0.980, 0.991] | 1.007 [0.993, 1.089] | 0.993 [0.976, 1.021] |
| 24,576 | 0.995 [0.991, 0.999] | 0.989 [0.979, 0.989] | 1.000 [0.998, 1.003] | 0.993 [0.989, 0.997] |
| 28,672 | 0.992 [0.992, 0.994] | 0.995 [0.992, 0.996] | 0.994 [0.991, 0.995] | 0.989 [0.984, 0.994] |
| 32,768 | 0.992 [0.984, 0.993] | 0.991 [0.988, 0.994] | 0.995 [0.993, 0.996] | 0.989 [0.972, 0.993] |
| 36,864 | 0.995 [0.992, 0.996] | 0.991 [0.988, 0.994] | 0.990 [0.986, 0.994] | 0.997 [0.986, 0.999] |
| 49,152 | 0.994 [0.994, 0.997] | 0.993 [0.992, 0.994] | 1.001 [0.998, 1.004] | 0.994 [0.986, 0.997] |
| 65,536 | 0.993 [0.992, 0.994] | 0.993 [0.992, 0.999] | 0.993 [0.991, 0.996] | 0.993 [0.990, 0.999] |
| 131,072 | 0.993 [0.992, 0.995] | 0.996 [0.993, 1.000] | 0.993 [0.992, 0.998] | 0.993 [0.991, 1.026] |
| 147,456 | 0.997 [0.995, 0.998] | 0.994 [0.991, 0.995] | 0.997 [0.994, 0.999] | 0.994 [0.992, 1.002] |
| 262,144 | 0.992 [0.991, 0.993] | 0.997 [0.992, 0.998] | 0.995 [0.987, 0.997] | 0.992 [0.991, 0.996] |
| 1,048,576 | 0.994 [0.992, 0.996] | 0.997 [0.993, 0.997] | 0.994 [0.993, 0.997] | 0.995 [0.990, 0.999] |
| 4,194,304 | 0.994 [0.993, 0.997] | 0.998 [0.997, 0.999] | 0.994 [0.985, 0.999] | 0.996 [0.994, 0.999] |

</details>

<details>
<summary>AMD EPYC 9B45 · GCP / off: all paired ratios</summary>

| Decoded B | Default encode | Default decode | Cache encode | Cache decode |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.916 [0.915, 0.917] | 1.085 [1.085, 1.086] | 0.834 [0.833, 0.834] | 1.000 [1.000, 1.002] |
| 1 | 0.923 [0.923, 0.924] | 1.044 [1.001, 1.048] | 0.923 [0.922, 0.925] | 1.000 [0.997, 1.001] |
| 32 | 1.047 [1.045, 1.049] | 1.009 [0.998, 1.014] | 0.995 [0.987, 0.999] | 1.008 [1.000, 1.018] |
| 52 | 0.989 [0.981, 0.997] | 0.996 [0.994, 1.052] | 0.986 [0.981, 0.988] | 1.008 [1.004, 1.015] |
| 64 | 0.998 [0.995, 1.010] | 1.008 [1.002, 1.073] | 1.002 [0.999, 1.007] | 1.011 [1.008, 1.018] |
| 65 | 0.988 [0.979, 0.993] | 1.008 [0.999, 1.052] | 0.988 [0.982, 0.994] | 1.014 [1.007, 1.021] |
| 128 | 0.977 [0.974, 0.982] | 0.988 [0.982, 0.993] | 0.955 [0.946, 0.959] | 0.992 [0.984, 0.993] |
| 256 | 0.975 [0.974, 0.975] | 0.968 [0.964, 0.980] | 0.932 [0.925, 0.951] | 0.975 [0.970, 0.978] |
| 512 | 0.989 [0.978, 0.994] | 0.949 [0.946, 0.960] | 0.944 [0.941, 0.952] | 0.955 [0.954, 0.955] |
| 1,024 | 0.979 [0.977, 0.987] | 0.998 [0.992, 1.004] | 0.945 [0.944, 0.948] | 0.999 [0.997, 1.001] |
| 1,504 | 0.988 [0.982, 0.988] | 0.993 [0.991, 0.999] | 0.965 [0.965, 0.966] | 1.004 [1.001, 1.007] |
| 1,875 | 0.992 [0.984, 0.996] | 1.003 [1.001, 1.010] | 0.974 [0.972, 0.976] | 1.005 [1.003, 1.007] |
| 2,048 | 0.974 [0.970, 0.980] | 1.000 [0.995, 1.002] | 0.976 [0.970, 0.982] | 1.004 [1.002, 1.006] |
| 4,096 | 0.981 [0.978, 0.985] | 1.001 [0.998, 1.004] | 1.000 [0.996, 1.010] | 0.998 [0.991, 1.016] |
| 8,192 | 0.988 [0.987, 0.994] | 1.002 [1.000, 1.005] | 1.000 [0.993, 1.001] | 0.998 [0.995, 1.013] |
| 16,384 | 0.994 [0.992, 1.003] | 1.003 [1.000, 1.004] | 0.996 [0.988, 0.998] | 1.000 [0.995, 1.010] |
| 24,576 | 0.993 [0.991, 0.999] | 0.997 [0.995, 1.000] | 0.997 [0.993, 1.004] | 1.000 [0.998, 1.000] |
| 28,672 | 1.003 [1.002, 1.006] | 0.997 [0.995, 1.018] | 1.005 [1.000, 1.011] | 1.000 [0.999, 1.002] |
| 32,768 | 1.001 [0.999, 1.004] | 1.000 [0.998, 1.007] | 1.004 [1.001, 1.006] | 0.994 [0.992, 1.001] |
| 36,864 | 0.994 [0.994, 0.998] | 1.001 [0.999, 1.004] | 1.001 [0.996, 1.002] | 1.003 [1.000, 1.005] |
| 49,152 | 1.005 [1.002, 1.006] | 0.998 [0.996, 1.001] | 0.999 [0.996, 1.001] | 1.003 [0.999, 1.004] |
| 65,536 | 0.999 [0.999, 1.000] | 0.999 [0.998, 1.002] | 1.003 [1.001, 1.004] | 1.000 [0.999, 1.003] |
| 131,072 | 0.998 [0.998, 1.001] | 0.999 [0.998, 1.004] | 1.003 [0.998, 1.005] | 1.001 [0.997, 1.005] |
| 147,456 | 0.996 [0.994, 0.999] | 0.999 [0.988, 1.003] | 1.005 [1.001, 1.009] | 1.005 [0.996, 1.016] |
| 262,144 | 0.999 [0.997, 1.000] | 1.000 [0.997, 1.002] | 1.000 [0.997, 1.003] | 1.001 [1.000, 1.003] |
| 1,048,576 | 0.997 [0.994, 0.999] | 1.000 [0.999, 1.003] | 0.997 [0.996, 1.002] | 0.999 [0.992, 1.003] |
| 4,194,304 | 0.992 [0.987, 0.998] | 1.001 [0.997, 1.004] | 0.995 [0.994, 1.001] | 1.001 [0.999, 1.004] |

</details>


### Focused controls

Primary points above are not replaced by repeats. The focused cohorts use the
same linked artifacts, fixtures, process CPU clock and 3 timed batches at least
15 ms long. M5 repeats fat/cache at 512, 1,024, 2,048 and 32,768 bytes; GCP repeats
off/default at 32, 52, 64 and 65 bytes. The first repeat has 7 processes and 5 copies
(seven of fifteen cyclic starts); the GCP full-rotation supplement has 9 processes
and 3 copies. Each fresh copy visits every loader rank in that supplement.

#### m5 / fat / cache

| Bytes / operation | Primary stack | Primary control | 7-process stack | 7-process control | Full-rotation stack | Full-rotation control |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 512 / encode | 0.981 [0.962, 0.987] | 1.001 [0.970, 1.016] | 0.984 [0.980, 0.990] | 1.000 [0.995, 1.013] | — | — |
| 512 / decode | 1.027 [1.022, 1.029] | 1.002 [0.996, 1.004] | 1.016 [1.011, 1.020] | 0.999 [0.997, 1.004] | — | — |
| 1,024 / encode | 1.004 [1.001, 1.007] | 1.000 [0.991, 1.002] | 1.002 [0.999, 1.010] | 0.999 [0.996, 1.006] | — | — |
| 1,024 / decode | 0.999 [0.996, 1.003] | 1.000 [0.994, 1.005] | 0.997 [0.995, 1.005] | 1.003 [0.996, 1.006] | — | — |
| 2,048 / encode | 1.001 [0.995, 1.029] | 1.002 [0.994, 1.029] | 1.001 [0.993, 1.006] | 0.998 [0.995, 1.005] | — | — |
| 2,048 / decode | 0.991 [0.940, 1.010] | 0.991 [0.978, 1.009] | 0.999 [0.997, 1.005] | 1.000 [0.996, 1.006] | — | — |
| 32,768 / encode | 1.005 [0.996, 1.008] | 0.999 [0.991, 1.002] | 1.003 [1.001, 1.006] | 1.000 [0.999, 1.002] | — | — |
| 32,768 / decode | 1.009 [1.006, 1.015] | 0.999 [0.997, 1.006] | 1.008 [1.007, 1.011] | 1.000 [0.999, 1.001] | — | — |

#### gcp / off / default

| Bytes / operation | Primary stack | Primary control | 7-process stack | 7-process control | Full-rotation stack | Full-rotation control |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 32 / encode | 1.047 [1.045, 1.049] | 1.048 [1.048, 1.048] | 1.083 [1.081, 1.100] | 1.048 [1.043, 1.048] | 1.000 [0.998, 1.004] | 1.000 [0.999, 1.002] |
| 32 / decode | 1.009 [0.998, 1.014] | 1.040 [1.035, 1.050] | 1.006 [1.001, 1.016] | 1.000 [0.996, 1.022] | 1.002 [0.996, 1.015] | 0.997 [0.995, 1.001] |
| 52 / encode | 0.989 [0.981, 0.997] | 1.000 [0.990, 1.004] | 0.990 [0.983, 0.999] | 1.000 [0.998, 1.003] | 0.989 [0.981, 0.995] | 1.002 [0.996, 1.004] |
| 52 / decode | 0.996 [0.994, 1.052] | 1.021 [1.016, 1.027] | 1.014 [1.000, 1.048] | 1.000 [0.967, 1.020] | 0.999 [0.972, 1.004] | 0.999 [0.971, 1.001] |
| 64 / encode | 0.998 [0.995, 1.010] | 1.000 [0.998, 1.001] | 1.001 [0.995, 1.208] | 1.000 [1.000, 1.001] | 1.000 [0.995, 1.005] | 1.000 [0.998, 1.004] |
| 64 / decode | 1.008 [1.002, 1.073] | 1.029 [1.028, 1.097] | 1.004 [0.999, 1.009] | 1.001 [0.989, 1.014] | 1.000 [0.897, 1.009] | 1.000 [0.899, 1.002] |
| 65 / encode | 0.988 [0.979, 0.993] | 1.000 [0.995, 1.004] | 0.990 [0.987, 0.993] | 1.000 [0.998, 1.005] | 0.988 [0.973, 0.994] | 1.002 [0.994, 1.009] |
| 65 / decode | 1.008 [0.999, 1.052] | 1.003 [0.996, 1.013] | 1.007 [0.996, 1.011] | 1.000 [0.997, 1.004] | 1.003 [0.998, 1.011] | 1.000 [0.996, 1.002] |

## First-call decode

Separate fresh-process calls use 15 processes × 2 copies per label at 1,408,
2,816, 5,632 and 32,768 Base58 characters, input `"2"` repeated that many times.
An independent Python bigint oracle checks exact bytes and returned length.
Each output is poisoned before its clock starts. Loading, oracle construction,
caller buffers and poison writes are outside timing; first codec setup and internal
allocation remain inside. Unlike the warm prevalidated export, `size_decode`
performs its UTF-8 check inside this single-call timer. Call order rotates.
Ratios use within-process two-copy medians. Absolute time is the median over copies
of each copy's median over processes. Short calls have coarse resolution and wide
process ranges. These data do not establish a general first-call win.

| Host / LTO / policy | Input chars | Stack time | Stack/main [range] | Control/main [range] |
| --- | ---: | ---: | ---: | ---: |
| m5 / fat / default | 1,408 | 5.5 µs | 1.000 [0.588, 1.636] | 0.846 [0.588, 1.875] |
| m5 / fat / default | 2,816 | 10.5 µs | 0.955 [0.714, 1.182] | 0.917 [0.714, 1.238] |
| m5 / fat / default | 5,632 | 30 µs | 0.935 [0.797, 1.017] | 0.968 [0.838, 1.133] |
| m5 / fat / default | 32,768 | 322 µs | 0.974 [0.890, 1.065] | 0.985 [0.864, 1.104] |
| m5 / fat / cache | 1,408 | 57.5 µs | 0.991 [0.569, 1.165] | 1.018 [0.567, 1.127] |
| m5 / fat / cache | 2,816 | 203 µs | 1.010 [0.888, 1.081] | 1.013 [0.917, 1.066] |
| m5 / fat / cache | 5,632 | 786 µs | 1.001 [0.968, 1.029] | 1.010 [0.955, 1.038] |
| m5 / fat / cache | 32,768 | 522 µs | 0.999 [0.958, 1.076] | 0.998 [0.954, 1.055] |
| m5 / off / default | 1,408 | 8.5 µs | 1.000 [0.708, 1.333] | 0.947 [0.667, 1.278] |
| m5 / off / default | 2,816 | 15 µs | 0.967 [0.784, 1.161] | 1.056 [0.821, 1.226] |
| m5 / off / default | 5,632 | 35 µs | 0.875 [0.784, 0.987] | 0.963 [0.875, 1.089] |
| m5 / off / default | 32,768 | 341 µs | 0.985 [0.935, 1.031] | 0.988 [0.936, 1.067] |
| m5 / off / cache | 1,408 | 64.5 µs | 1.000 [0.716, 1.110] | 1.000 [0.685, 1.093] |
| m5 / off / cache | 2,816 | 220 µs | 1.033 [0.984, 1.087] | 1.029 [0.972, 1.087] |
| m5 / off / cache | 5,632 | 886 µs | 0.993 [0.966, 1.043] | 1.003 [0.964, 1.045] |
| m5 / off / cache | 32,768 | 549 µs | 0.998 [0.974, 1.038] | 0.990 [0.965, 1.033] |
| gcp / fat / default | 1,408 | 28.3 µs | 0.943 [0.846, 1.454] | 0.988 [0.831, 1.171] |
| gcp / fat / default | 2,816 | 48.2 µs | 0.923 [0.863, 0.991] | 0.911 [0.776, 0.979] |
| gcp / fat / default | 5,632 | 66.3 µs | 0.933 [0.720, 1.150] | 0.918 [0.702, 1.143] |
| gcp / fat / default | 32,768 | 321 µs | 0.974 [0.931, 1.039] | 0.998 [0.947, 1.039] |
| gcp / fat / cache | 1,408 | 93.2 µs | 1.001 [0.873, 1.128] | 0.979 [0.849, 1.265] |
| gcp / fat / cache | 2,816 | 318 µs | 0.996 [0.923, 1.024] | 0.994 [0.925, 1.055] |
| gcp / fat / cache | 5,632 | 69.3 µs | 0.967 [0.908, 1.029] | 0.977 [0.954, 1.032] |
| gcp / fat / cache | 32,768 | 326 µs | 1.002 [0.927, 1.058] | 1.090 [1.020, 1.152] |
| gcp / off / default | 1,408 | 36.3 µs | 0.895 [0.804, 0.989] | 0.887 [0.773, 1.054] |
| gcp / off / default | 2,816 | 54.5 µs | 0.890 [0.805, 1.001] | 0.946 [0.877, 1.147] |
| gcp / off / default | 5,632 | 76.8 µs | 0.926 [0.883, 0.987] | 0.960 [0.914, 1.065] |
| gcp / off / default | 32,768 | 366 µs | 0.983 [0.921, 1.037] | 0.996 [0.947, 1.035] |
| gcp / off / cache | 1,408 | 106 µs | 0.997 [0.851, 1.093] | 0.987 [0.887, 1.168] |
| gcp / off / cache | 2,816 | 326 µs | 0.987 [0.962, 1.004] | 0.995 [0.960, 1.066] |
| gcp / off / cache | 5,632 | 78.6 µs | 0.939 [0.865, 1.094] | 0.939 [0.913, 0.990] |
| gcp / off / cache | 32,768 | 360 µs | 0.950 [0.899, 1.005] | 0.957 [0.920, 0.982] |

## Linked size and compact

All stack columns below are **Auto-both minus the matching empty-both consumer**
from the timing build; all Δ columns compare that stack difference with main's
corresponding difference. Code, readonly, initialized data and BSS are section
totals; stripped file bytes also include layout and metadata. Mach-O and ELF
section categories are not directly interchangeable. Signed BSS differences can
be negative after subtracting the scaffold. BSS movement is not retained-heap
measurement. `.rlib` size is not deployment size. Path labels have equal lengths.

| Host / LTO / policy | Stack code B | Stack readonly B | Code Δ B | Readonly Δ B | Stack data B | Data Δ B | Stack BSS B | BSS Δ B | Stack stripped B | Stripped Δ B |
| --- |  ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| m5 / fat / default | 67,760 | 24,950 | -1,664 | -961 | 16 | +0 | 0 | +0 | 82,784 | +0 |
| m5 / fat / cache | 76,232 | 27,768 | -1,600 | -985 | 2,192 | +0 | 0 | +0 | 99,328 | +0 |
| m5 / off / default | 80,728 | 50,006 | +0 | -569 | 8 | +0 | 0 | +0 | 132,432 | +16 |
| m5 / off / cache | 84,760 | 58,019 | +0 | -465 | 2,184 | +0 | 0 | +0 | 132,464 | +0 |
| gcp / fat / default | 308,157 | 38,444 | -1,120 | -536 | 6,248 | -672 | -608 | +2,368 | 352,872 | -2,368 |
| gcp / fat / cache | 303,789 | 39,280 | -1,056 | -584 | 9,136 | -624 | -1,890 | -1,856 | 352,240 | -2,240 |
| gcp / off / default | 332,787 | 75,964 | -8,848 | -696 | 10,328 | -736 | -1,288 | -2,000 | 419,080 | -10,288 |
| gcp / off / cache | 324,707 | 72,072 | -8,240 | -296 | 11,464 | -704 | -542 | +1,104 | 408,248 | -9,296 |

`compact` selects eight rather than twelve x86 IFMA widths. It trades linked code
for more chunks on some multiplication shapes. This cohort measures the default
twelve-width policy only. No new compact, no-allocation or Wasm size claim is made.

## Emitted code and correctness

The four default/cache × fat/off configurations were inspected on both hosts.
The private Horner leaf loses the redundant bad-digit OR chain and release validity
assertion. Its output-shape checks remain. The public divide-and-conquer entry
still validates the entire significant suffix, including vector/scalar tails,
before arithmetic. Leading Base58 zeroes are valid; the cold error helper adds
their prefix offset once to the first invalid byte's position. Cached Matrix
leaves retain their own fallible validation. The sole production `values` caller
and connected public/owned/check entries preserve this prevalidation contract;
a future unvalidated caller would violate it in release builds.

The Power array is built directly in return storage: no extra 2,048-byte return
copy and no new per-Power index guard in the inspected bodies. The existing
upfront level bound and table/scratch capacity checks remain. Current main already
returns directly; the stack preserves that property. Four independent radix carry
chains remain in registers. The twelve GCP IFMA kernel bodies match after bounded
address-relocation/padding normalization. This does not prove equal constants,
all dispatcher clones, cache behavior or universal non-regression.

Concrete whole-stack costs remain relative to main: three no-LTO quarter-Zip
constructor calls per selected carry specialization; extra pointer reloads in
the fat-LTO CRT loop; 16 extra local stack bytes in GCP fat NTT powers and M5
fat/cache NTT powers; and M5
off/cache matrix decode's frame increase from 0x70 to 0xd0 plus setup and per-panel
helper calls. Actual GCP off/default ELFs also show extra reached work in the
32-byte encoder: the stack calls `as_flattened`, uses a counted leading-zero scan,
and calls two checked slice-copy helpers
that tail-call imported `memcpy` for 32 and 16 bytes, where main emits inline
flattening, unrolled leading-zero checks and stores. Both fixed32 source files
are unchanged; this is a whole-build inlining/constant-propagation difference.
All eight GCP digit-pair tables match byte for byte, and actual relocations bind
the inspected no-LTO validation/error calls to `all_valid` and offset-aware `invalid`.
The extra reached work does not explain the unchanged
main control shift, and its net latency cost is not established by the repeats.
These costs come from the complete stack, not all from #67. Instruction
counts alone cannot decide their latency cost. This is bounded emitted-path
inspection, not proof that every changed machine-code clone is cheaper.

Default/cache debug/release correctness with `check,__internals` and 512 property
cases passes on M5 and GCP, as do no-default-features/check debug/release suites.
Formatting, all-target/all-feature Clippy and six Linux x86-64/AArch64 ×
default/cache/compact inlining manifests pass. These checks do not substitute for
unrun CI jobs or other toolchains. No new invalid-input, owned-output, checked,
allocation-counting, native-CPU, no-allocation or Wasm performance claim is made.

## Cache and memory

`cache` changes dispatch and leaf policy as well as retention. Cached first calls
can build tables, so warm results are not cold-start predictions. Code-derived
retained capacities and ownership are in
[DESIGN.md](../DESIGN.md#heap-stack-and-cache-costs). Per-call scratch, owned tables,
allocator overhead and concurrent calls add to retention. It is not an active-heap
bound. Wasm linear-memory pages need not return to the host.

Count allocations in separate, untimed builds with
`scripts/resource_report.py build --count-allocations`, then `run`; these use internal
workers and are not ordinary public consumers. Requests count allocations plus
reallocations; requested-layout peaks are not RSS, allocator usable sizes or stack
high water. No counted-allocation cohort was refreshed with these timings.

## Measure and update

Use an idle host, AC power for laptops, and ignored `target/measurements/` output.
Build all comparison artifacts before timing. On the shared VM, use `/mnt/bench`
and wait for its shared lock. Do not overlap our timing with builds or tests.
Example generic public-consumer builds (repeat for the second source and cache):

```sh
CARGO_PROFILE_RELEASE_LTO=fat python3 -B scripts/code_size.py --source . --output target/measurements/native-fat --target aarch64-apple-darwin --cpu generic --backends empty,auto --directions both --kinds cdylib
CARGO_PROFILE_RELEASE_LTO=off python3 -B scripts/code_size.py --source . --output target/measurements/native-off --target aarch64-apple-darwin --cpu generic --backends empty,auto --directions both --kinds cdylib
```

Use `x86_64-unknown-linux-gnu` and `--strip-tool strip` on GCP; pin timing with
`--cpu 1`. Pass `--cache` for that policy. The ordinary harness is
`scripts/public_bench.py`, with `LABEL=PATH` probes and `--baseline main` for paired
ratios. For this study the retained derivative `public_bench_verified.py` adds
output poison before its untimed checks; its timed loops are unchanged. Preserve
the documented rotation limits when reproducing it. Use independent controls and
complete rotation blocks for a small effect; do not infer causality from an
unchanged main control that shifts in the same direction.

Raw files, artifacts, commands, environment, script/source/binary hashes and
assembly are retained locally under
`target/measurements/pr67-stack-gcp/` and on GCP under
`/mnt/bench/pr67-stack-gcp-20261112`. Generated CSV/JSON and run records stay
outside commits. The primary export covers 864 warm/control rows, 64 cold/control
rows and 8 size rows; an independent audit recomputed all 4,720 measurement fields,
checked 64 actual artifact hashes, parsed the 32 stripped images' sections, and
compared 136 frozen input files with Git blobs. This validates arithmetic and
recorded provenance, not an independent rebuild or absent individual batch data.

The study-specific renderer is retained with the ignored measurements, rather
than maintained as a PR-specific repository tool. With pinned
`assets/graphs/requirements.txt` installed, render the retained local exports
into a **new** directory:

```sh
python3 -B target/measurements/pr67-stack-gcp/render_stack_pr67.py --data target/measurements/pr67-stack-gcp/published-data --output target/measurements/pr67-stack-gcp/NEW_FIGURES
python3 -B assets/graphs/render.py --check
python3 -B -m unittest discover -s assets/graphs -p test_render.py
```

The figure/link check validates local links and SVG syntax. It does not verify
numbers or reproduce pixels. The retained local renderer validates dataset keys/source
bindings, preserves primary and supplemental cohorts separately, and emits the
27-size tables with the figures. The Wasm figure marks the current measurement gap.

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

# base58-bench

Rust encoding and decoding for the Bitcoin Base58 alphabet. The default API
selects conversion kernels by input size and CPU support. The library uses
`std` and has no normal dependencies.

Use the root API for automatic dispatch. Use `no_alloc` when conversions must
use no internal heap allocation, including the first call. Enable `cache` to
retain reusable tables between automatic conversions; caching is off by default.

## Automatic conversion

Arrays and reusable slices work with the borrowed-output helpers. Owned-output
helpers return a `String` or `Vec<u8>`. The length-returning functions support
callers that manage the written prefix themselves.

```rust
use base58_bench as b58;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = [0, 1, 2, 3];
    let mut text_buffer = [0; b58::max_encoded_len(4)];
    let mut byte_buffer = [0; 4];

    let text = b58::encode_into(input, &mut text_buffer)?;
    assert_eq!(b58::decode_array::<4>(text)?, input);
    assert_eq!(b58::decode_into(text, &mut byte_buffer)?, input);

    let owned_text = b58::encode_string(input);
    assert_eq!(b58::decode_vec(&owned_text)?, input);

    let written = b58::encode(&input, &mut text_buffer)?;
    let text = std::str::from_utf8(&text_buffer[..written])?;
    let written = b58::decode(text, &mut byte_buffer)?;
    assert_eq!(&byte_buffer[..written], &input);

    let short: Result<usize, b58::BufferTooSmall> = b58::encode(&[0], &mut []);
    assert_eq!(short, Err(b58::BufferTooSmall));
    assert_eq!(
        b58::decode("0", &mut [0; 1]),
        Err(b58::Error::InvalidCharacter { index: 0 }),
    );
    Ok(())
}
```

`max_encoded_len(n)` is a constant upper bound for encoding `n` input bytes.
An exactly sized result buffer also suffices. Decode needs at most as many
output bytes as input text bytes. Auto may allocate internal workspace even
when the caller supplies output storage.

Leading zero bytes round-trip as leading `'1'` characters. `decode_array::<N>`
requires exactly `N` decoded bytes: shorter values return
`Error::InvalidLength { expected, actual }`; longer values return
`Error::BufferTooSmall`. It does not pad or discard zeros.

Invalid-character indices are UTF-8 byte offsets. Both error types implement
`Display` and `std::error::Error`. Successful calls leave the unused output tail
untouched. Output contents after an error are unspecified. If invalid input and
insufficient capacity both apply, their error precedence is unspecified.
Conversions have input-dependent timing.

## Conversion without internal allocation

`no_alloc` offers `encode`, `decode`, `encode_into`, `decode_into` and
`decode_array`. It uses caller storage and local state, with compile-time tables
for fixed 32/64-byte encoding. Other encoding sizes and decoding use batched
conversion, whose work is quadratic at large sizes. It still requires `std`.

```rust
use base58_bench::{max_encoded_len, no_alloc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = [0, 1, 2, 3];
    let mut text_buffer = [0; max_encoded_len(4)];
    let mut byte_buffer = [0; 4];

    let text = no_alloc::encode_into(&input, &mut text_buffer)?;
    assert_eq!(no_alloc::decode_array::<4>(text)?, input);
    assert_eq!(no_alloc::decode_into(text, &mut byte_buffer)?, input);

    let written = no_alloc::encode(&input, &mut text_buffer)?;
    let text = std::str::from_utf8(&text_buffer[..written])?;
    assert_eq!(no_alloc::decode(text, &mut byte_buffer)?, input.len());
    Ok(())
}
```

The module shares the root error types and output contracts. It has no owned
`String` or `Vec` helpers. Enabling `cache` does not change its allocation guarantee.

## Optional table retention

For a consumer using a local checkout, opt in with:

```toml
[dependencies]
base58-bench = { path = "../base58-bench", features = ["cache"] }
```

Without `cache`, Auto avoids general matrix setup and uses batched/divide-and-
conquer conversion, with owned NTT tables for large products. With `cache`, it
also enables general matrix routes/leaves and retains matrix/bounded NTT tables
for the process lifetime. First use still builds them; history affects later
calls. Large NTT tables remain owned by each conversion. This changes dispatch
as well as retention, trading setup time, idle memory and concurrent peak costs.

Cached table data are allocated lazily on the **heap**. On the recorded 64-bit
layouts, all matrix classes total 18,617,760 bytes and production NTT logs 5–18
can retain 25,164,288 bytes: together 43,782,048 bytes (41.8 MiB). Actual retention
depends on call history. Their `OnceLock`/vector headers add 6,280 bytes of
**static storage** when all three cache families are linked; these stores are
absent without `cache`. Fixed 32/64-byte tables are separate compile-time
constants: 2,208 bytes of reference layout, not a guarantee of emitted size.
These are requested capacities, excluding allocator overhead, not RSS measurements.
Construction, workspace, prepared powers, large owned tables and concurrent calls
add active memory. See
[the resource details](DESIGN.md#heap-stack-and-cache-costs).

The `__internals` feature exposes backend selectors and measurement hooks under
`base58_bench::internals`. Those APIs are unstable; the old root selectors and
`bench` module are absent. It does not enable `cache`. Normal callers need neither
named backends nor this feature. See [DESIGN.md](DESIGN.md) for routes and costs.

The tables retain earlier measurement snapshots. The
[allocation report](docs/tradeoffs/2026-10-25/allocation-layout/README.md) records
the newer storage changes, paired timings and linked-size comparisons. Source
identities and limits are retained with each report.
The separate [fixed-table reuse experiment](docs/tradeoffs/2026-10-25/fixed-table-reuse/README.md)
tests nearby lengths on M5, GCP and Wasm and records why broad padding was rejected.

<!-- PUBLIC-API-TABLES:BEGIN -->

## Performance

AC-qualified laptop rerun. GCP timings, linked sizes and allocations are prior snapshots.
The user reports the earlier laptop was unplugged; historical per-run telemetry is absent.
This does not establish every old run's power state or a controlled battery/AC causal effect.

Warm time per conversion; lower is better. Native rows use process CPU time;
Wasm/V8 uses elapsed time. Bold marks the lowest unrounded median, without a
statistical claim. These time raw `encode`/`decode` into reusable caller buffers,
excluding owned-result allocations and convenience-wrapper UTF-8 checks.
Payload sizes mean input bytes for encoding and decoded bytes
for decoding. See the [full report](docs/tradeoffs/2026-10-25/ac-power/README.md)
for first calls, process ranges, allocations, controls and exact build identities.

M5 uses `apple-m4` / Rust 1.99.1. The AMD EPYC 9B45 VM on GCP is a
`c4d-highcpu-4` with 4 vCPUs and 7 GiB RAM. Generic targets `x86-64`;
native uses `-C target-cpu=native`, both with Rust 1.99.0.
Wasm runs in Node 26.12.1 / V8 14.6 on the M5.
Native workers link internal backends and a fixture oracle; standalone consumer
layout/inlining can differ. Wasm selects one public mode per module.

`cache` is not uniformly faster: GCP generic 4 KiB decoding takes 33.39 µs
versus 27.46 µs by default (+21.6% CPU time). This case needs future tuning.

M5 1 KiB encoding changes from 7.55 µs to 1.94 µs warm CPU time,
while first-call wall time rises from 13.08 µs to 109.79 µs with `cache`.

Encoding:

| Configuration / payload bytes | Auto | Auto + `cache` | `no_alloc` | bs58 |
| --- | ---: | ---: | ---: | ---: |
| AC M5 Pro (apple-m4) / 32 | 13.94 ns | **13.30 ns** | 13.92 ns | 881.84 ns |
| AC M5 Pro (apple-m4) / 64 | 23.52 ns | **22.89 ns** | 23.10 ns | 3.18 µs |
| AC M5 Pro (apple-m4) / 256 | 623.29 ns | **225.34 ns** | 634.64 ns | 57.49 µs |
| AC M5 Pro (apple-m4) / 1,024 | 7.55 µs | **1.94 µs** | 7.65 µs | 975.00 µs |
| AC M5 Pro (apple-m4) / 4,096 | 105.31 µs | **28.68 µs** | 106.48 µs | 15.23 ms |
| AC M5 Pro (apple-m4) / 65,536 | 2.67 ms | **2.25 ms** | 27.15 ms | 3.88 s |
| AMD EPYC 9B45 VM on GCP (generic) / 32 | 31.55 ns | **30.85 ns** | 31.37 ns | 887.69 ns |
| AMD EPYC 9B45 VM on GCP (generic) / 64 | 42.35 ns | **40.88 ns** | 42.46 ns | 3.91 µs |
| AMD EPYC 9B45 VM on GCP (generic) / 256 | 1.19 µs | **403.29 ns** | 1.19 µs | 69.39 µs |
| AMD EPYC 9B45 VM on GCP (generic) / 1,024 | 8.51 µs | **3.39 µs** | 16.28 µs | 1.13 ms |
| AMD EPYC 9B45 VM on GCP (generic) / 4,096 | 51.45 µs | **41.27 µs** | 247.16 µs | 18.05 ms |
| AMD EPYC 9B45 VM on GCP (generic) / 65,536 | 3.62 ms | **2.78 ms** | 62.13 ms | 4.63 s |
| AMD EPYC 9B45 VM on GCP (native) / 32 | 32.32 ns | 34.96 ns | **32.09 ns** | 765.95 ns |
| AMD EPYC 9B45 VM on GCP (native) / 64 | 32.60 ns | **32.31 ns** | 32.71 ns | 2.97 µs |
| AMD EPYC 9B45 VM on GCP (native) / 256 | 1.16 µs | **400.97 ns** | 1.16 µs | 54.16 µs |
| AMD EPYC 9B45 VM on GCP (native) / 1,024 | 8.23 µs | **3.38 µs** | 15.08 µs | 920.65 µs |
| AMD EPYC 9B45 VM on GCP (native) / 4,096 | 49.47 µs | **38.88 µs** | 228.19 µs | 14.90 ms |
| AMD EPYC 9B45 VM on GCP (native) / 65,536 | 3.59 ms | **2.78 ms** | 57.07 ms | 3.83 s |
| AC M5 Pro Wasm/V8 / 32 | **26.10 ns** | 26.43 ns | 26.40 ns | 636.16 ns |
| AC M5 Pro Wasm/V8 / 64 | 101.81 ns | 101.75 ns | **101.10 ns** | 2.74 µs |
| AC M5 Pro Wasm/V8 / 256 | 2.09 µs | **871.69 ns** | 2.13 µs | 54.52 µs |
| AC M5 Pro Wasm/V8 / 1,024 | 29.71 µs | **10.07 µs** | 29.91 µs | 931.20 µs |
| AC M5 Pro Wasm/V8 / 4,096 | 458.00 µs | **149.54 µs** | 465.10 µs | 15.75 ms |
| AC M5 Pro Wasm/V8 / 65,536 | **14.27 ms** | 14.36 ms | 116.77 ms | 3.88 s |

Decoding:

| Configuration / payload bytes | Auto | Auto + `cache` | `no_alloc` | bs58 |
| --- | ---: | ---: | ---: | ---: |
| AC M5 Pro (apple-m4) / 32 | 16.61 ns | 16.91 ns | **15.92 ns** | 321.61 ns |
| AC M5 Pro (apple-m4) / 64 | 28.86 ns | 29.05 ns | **27.92 ns** | 1.16 µs |
| AC M5 Pro (apple-m4) / 256 | 186.18 ns | 183.50 ns | **182.52 ns** | 18.60 µs |
| AC M5 Pro (apple-m4) / 1,024 | 2.57 µs | **1.85 µs** | 2.64 µs | 309.64 µs |
| AC M5 Pro (apple-m4) / 4,096 | 38.37 µs | **24.89 µs** | 37.29 µs | 5.01 ms |
| AC M5 Pro (apple-m4) / 65,536 | 1.12 ms | **1.02 ms** | 8.90 ms | 1.29 s |
| AMD EPYC 9B45 VM on GCP (generic) / 32 | **26.29 ns** | 26.86 ns | 26.49 ns | 322.39 ns |
| AMD EPYC 9B45 VM on GCP (generic) / 64 | 48.02 ns | 48.46 ns | **47.98 ns** | 1.26 µs |
| AMD EPYC 9B45 VM on GCP (generic) / 256 | 348.68 ns | **302.52 ns** | 346.54 ns | 21.80 µs |
| AMD EPYC 9B45 VM on GCP (generic) / 1,024 | 4.80 µs | **2.95 µs** | 4.81 µs | 368.74 µs |
| AMD EPYC 9B45 VM on GCP (generic) / 4,096 | **27.46 µs** | 33.39 µs | 72.40 µs | 5.87 ms |
| AMD EPYC 9B45 VM on GCP (generic) / 65,536 | 1.30 ms | **1.29 ms** | 17.86 ms | 1.49 s |
| AMD EPYC 9B45 VM on GCP (native) / 32 | 27.98 ns | **25.86 ns** | 26.24 ns | 317.94 ns |
| AMD EPYC 9B45 VM on GCP (native) / 64 | 47.01 ns | 47.28 ns | **46.87 ns** | 1.26 µs |
| AMD EPYC 9B45 VM on GCP (native) / 256 | **263.08 ns** | 265.45 ns | 263.36 ns | 21.99 µs |
| AMD EPYC 9B45 VM on GCP (native) / 1,024 | 2.71 µs | **2.71 µs** | 2.71 µs | 375.82 µs |
| AMD EPYC 9B45 VM on GCP (native) / 4,096 | **22.03 µs** | 22.37 µs | 38.56 µs | 5.97 ms |
| AMD EPYC 9B45 VM on GCP (native) / 65,536 | 1.23 ms | **1.17 ms** | 9.24 ms | 1.52 s |
| AC M5 Pro Wasm/V8 / 32 | 51.62 ns | 50.61 ns | **50.32 ns** | 212.15 ns |
| AC M5 Pro Wasm/V8 / 64 | 126.00 ns | **125.55 ns** | 127.04 ns | 851.47 ns |
| AC M5 Pro Wasm/V8 / 256 | **1.35 µs** | 1.39 µs | 1.38 µs | 15.83 µs |
| AC M5 Pro Wasm/V8 / 1,024 | 17.99 µs | **9.08 µs** | 18.21 µs | 325.83 µs |
| AC M5 Pro Wasm/V8 / 4,096 | 214.61 µs | **125.10 µs** | 293.05 µs | 5.06 ms |
| AC M5 Pro Wasm/V8 / 65,536 | 6.62 ms | **6.62 ms** | 68.08 ms | 1.31 s |

Warm retained-table results omit construction on the first call. Cache-off Auto
still owns temporary tables/workspace. `cache` also changes matrix routes/leaves,
so the feature comparison includes dispatch changes. `no_alloc` is quadratic at general sizes.
`no_alloc` and bs58 measurements stop at 64 KiB by measurement policy, not an API
or format limit. Auto is measured through 4 MiB in the full report.

Unchanged-source controls shift by up to 24.7% across feature artifacts.
Compiler/layout effects or measurement variation may contribute; source equality
does not establish identical machine code. Small gaps need the recorded ranges.

## Linked size — prior snapshot

Matched encode+decode consumers, stripped, with an empty scaffold per feature cohort.
Native padding can hide section growth. The report retains code/read-only/data/BSS,
Wasm CODE/DATA, scaffold deltas, gzip and artifact hashes.

| Configuration / consumer | File bytes | Δ empty | Δ initialized sections |
| --- | ---: | ---: | ---: |
| AC M5 Pro (apple-m4) / Empty | 302,752 | 0 | 0 |
| AC M5 Pro (apple-m4) / Empty + cache | 302,768 | 0 | 0 |
| AC M5 Pro (apple-m4) / bs58 | 302,768 | 16 | 1,814 |
| AC M5 Pro (apple-m4) / Auto | 369,024 | 66,272 | 83,925 |
| AC M5 Pro (apple-m4) / Auto + cache | 385,584 | 82,816 | 93,886 |
| AC M5 Pro (apple-m4) / no_alloc | 302,864 | 112 | 13,033 |
| AMD EPYC 9B45 VM on GCP (generic) / Empty | 288,280 | 0 | 0 |
| AMD EPYC 9B45 VM on GCP (generic) / Empty + cache | 288,280 | 0 | 0 |
| AMD EPYC 9B45 VM on GCP (generic) / bs58 | 291,160 | 2,880 | 2,884 |
| AMD EPYC 9B45 VM on GCP (generic) / Auto | 616,440 | 328,160 | 328,132 |
| AMD EPYC 9B45 VM on GCP (generic) / Auto + cache | 613,968 | 325,688 | 325,676 |
| AMD EPYC 9B45 VM on GCP (generic) / no_alloc | 325,032 | 36,752 | 36,723 |
| AMD EPYC 9B45 VM on GCP (native) / Empty | 289,032 | 0 | 0 |
| AMD EPYC 9B45 VM on GCP (native) / Empty + cache | 289,032 | 0 | 0 |
| AMD EPYC 9B45 VM on GCP (native) / bs58 | 292,504 | 3,472 | 3,456 |
| AMD EPYC 9B45 VM on GCP (native) / Auto | 596,736 | 307,704 | 307,705 |
| AMD EPYC 9B45 VM on GCP (native) / Auto + cache | 585,104 | 296,072 | 296,109 |
| AMD EPYC 9B45 VM on GCP (native) / no_alloc | 315,112 | 26,080 | 26,112 |

| Configuration / consumer | Module bytes | Δ empty | Gzip bytes |
| --- | ---: | ---: | ---: |
| AC M5 Pro Wasm/V8 / Empty | 16,452 | 0 | 6,480 |
| AC M5 Pro Wasm/V8 / Empty + cache | 16,452 | 0 | 6,480 |
| AC M5 Pro Wasm/V8 / bs58 | 20,058 | 3,606 | 8,073 |
| AC M5 Pro Wasm/V8 / Auto | 105,752 | 89,300 | 45,208 |
| AC M5 Pro Wasm/V8 / Auto + cache | 108,412 | 91,960 | 46,370 |
| AC M5 Pro Wasm/V8 / no_alloc | 40,091 | 23,639 | 19,525 |

<!-- PUBLIC-API-TABLES:END -->

## Tests, benchmarks and reference notes

The code uses `std::cfg_select!` (Rust 1.95+); the complete crate MSRV is not yet established.

Run `cargo test` for the default API and `cargo test --features cache` for table
retention. Backend tests and benchmarks use `--features __internals`; add
`,cache` to measure retention. Benchmark targets include bs58, fixed-length
five8 and a development-only Malachite adapter.

[Verification notes](docs/review/verification.md) describe differential/property
tests, sanitizer fuzzing, supported Miri paths and scoped proof obligations.
These are bounded checks, not proofs of every backend or SIMD instruction.

The [historical README](docs/historical-readme.md) preserves the full previous
derivations, experimental notes and older measurements. The
[full backend grids](docs/tradeoffs/2026-10-24/benchmark-results/README.md) and
[resource/dispatch experiments](docs/tradeoffs/2026-10-24/resource-dispatch/README.md)
remain available with their original API, source and measurement scope.

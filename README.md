# faster-base58

Bitcoin Base58 encoding and decoding in Rust. The default API selects algorithms
by input size and CPU support. It is `no_std` by default, using `alloc` for its
faster general conversions. On x86-64, runtime SIMD detection uses the standard
library when `std` is enabled, and `cpufeatures` otherwise.

```toml
[dependencies]
faster-base58 = { git = "https://github.com/elichai/faster-base58" }
```

Requires Rust 1.95 or newer. This crate has not been published to crates.io yet.

## Encode and decode

Use borrowed output with arrays or reusable buffers, or return owned strings and vectors:

```rust
use faster_base58 as b58;

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
    assert_eq!(b58::decode("0", &mut [0; 1]),
               Err(b58::Error::InvalidCharacter { index: 0 }));
    Ok(())
}
```

`max_encoded_len(n)` bounds the encoded output; an exactly sized result buffer
also works. Decoding needs at most one output byte per input text byte. Leading
zero bytes round-trip as leading `'1'` characters. `decode_array::<N>` requires
exactly `N` bytes: shorter results return `InvalidLength`, longer ones `BufferTooSmall`.

Invalid-character indices are UTF-8 byte offsets. Errors implement `Display` and
the standard error trait. Successful calls leave the unused output tail untouched;
on errors, output contents and error precedence are unspecified. Conversion timing
depends on input values and length; these functions are not constant-time.

## No internal allocation

`no_alloc` makes no heap allocations, including first calls and error paths.
It uses fixed tables for 32/64-byte encoding and batched conversion elsewhere.
General large inputs take quadratic time; the default API can allocate workspace
for faster algorithms. The examples are ordinary `std` programs; the library need not be.

```rust
use faster_base58::{max_encoded_len, no_alloc};

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

This module shares the root error and buffer contracts. It has no owned-result
helpers. Enabling `cache` does not change its allocation guarantee.

Auto trades workspace for large-input speed. At 4 MiB, the M5 measurements with
default features peak at about 192 MiB of extra requested heap for encoding and
105 MiB for decoding, excluding caller buffers. [Allocation details](DESIGN.md#heap-stack-and-cache-costs).

## Optional caching

Add `features = ["cache"]` to the dependency to retain reusable tables. This
feature enables `std`. Caching is off by default; it changes dispatch as well as
retention. First use builds tables; the warm timings below exclude that setup.
[Cold calls and cache tradeoffs](docs/benchmarks/2026-10-27/README.md) help assess
whether reuse repays the setup and memory cost for your workload.

Tables are lazy **heap** allocations. Recorded capacity bounds are 18,617,760 bytes
for matrix tables and 25,164,288 bytes for production NTT twiddles: **41.8 MiB**
together, depending on call history. All cache families also use **6,280 bytes of
static headers** in the recorded 64-bit layout; these stores are absent by default.
Dead stripping can remove headers for cache families a consumer does not use.
Fixed 32/64-byte constants are separate (2,208 bytes of reference layout).
These are payload capacities, not RSS. Allocator overhead, construction scratch,
large per-call tables and concurrent conversions add memory; this is not a bound
on active heap. [Memory and algorithm details](DESIGN.md#heap-stack-and-cache-costs).

## Performance

**Measured 2026-10-27, Rust 1.99.1, default target flags.** Warm calls reuse output
buffers and include Auto's workspace allocations. Owned helpers are measured
separately. Native uses process CPU time; Wasm uses V8 elapsed time.
[Methods, ranges, source snapshots and earlier GCP results](docs/performance.md).

### Apple M5 Pro

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 11.9 ns / 15.2 ns | 11.5 ns / **14.1 ns** | **9.79 ns** / 14.4 ns | 779 ns / 324 ns | 33.3 ns / 31.2 ns |
| 64 B | 24.4 ns / 27.2 ns | 24.3 ns / **27.2 ns** | **24 ns** / 28 ns | 3.08 µs / 1.17 µs | 76.9 ns / 94.1 ns |
| 512 B | 2.16 µs / 671 ns | **566 ns** / **595 ns** | 2.18 µs / 668 ns | 239 µs / 75.8 µs | — / — |
| 4 KiB | 110 µs / 39.9 µs | **28.3 µs** / **25.2 µs** | 109 µs / 38.6 µs | 15.1 ms / 5.34 ms | — / — |
| 64 KiB | 2.77 ms / 1.15 ms | **2.31 ms** / **1.01 ms** | 24.9 ms / 8.89 ms | 3.9 s / 1.37 s | — / — |
| 1 MiB | 62.9 ms / 28.3 ms | **58 ms** / **27.4 ms** | — / — | — / — | — / — |
| 4 MiB | 298 ms / 142 ms | **280 ms** / **139 ms** | — / — | — / — | — / — |

### WebAssembly on Apple M5 Pro (Node/V8)

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 25.7 ns / 54.7 ns | 25.6 ns / 53.8 ns | **25.5 ns** / 50.6 ns | 635 ns / 212 ns | 44.8 ns / **41.2 ns** |
| 64 B | 99.1 ns / 133 ns | 99.6 ns / 133 ns | 101 ns / **126 ns** | 2.87 µs / 833 ns | **94 ns** / 127 ns |
| 512 B | 7.76 µs / 5.04 µs | **2.84 µs** / **2.65 µs** | 7.71 µs / 5 µs | 237 µs / 72.1 µs | — / — |
| 4 KiB | 489 µs / 217 µs | **149 µs** / **124 µs** | 486 µs / 281 µs | 15.4 ms / 5.35 ms | — / — |
| 64 KiB | 14.1 ms / 6.58 ms | **14 ms** / **6.45 ms** | 116 ms / 70.6 ms | 3.9 s / 1.31 s | — / — |
| 1 MiB | 339 ms / 168 ms | **339 ms** / **167 ms** | — / — | — / — | — / — |
| 4 MiB | **1.61 s** / 819 ms | 1.62 s / **818 ms** | — / — | — / — | — / — |

Each cell is **encode / decode**. Bold marks the lowest observed median, not
statistical significance. `no_alloc` and bs58 stop at 64 KiB by benchmark policy;
five8 supports the measured 32/64-byte cases. All sizes remain in the data report.

![Native public API conversion time on Apple M5 Pro](assets/graphs/m5-public.svg)

![WebAssembly public API conversion time on Apple M5 Pro](assets/graphs/wasm-public.svg)

## Linked size

Code and data linked **above an empty consumer**, in KiB. Both API directions,
opt3/fat LTO. These deltas include associated code and metadata. Full files also
contain runtime, scaffolding and platform padding.

| Consumer | M5 code / readonly | Wasm code / data section |
|---|---:|---:|
| Auto | 58.5 KiB / 22.6 KiB | 70.1 KiB / 14.4 KiB |
| Auto + cache | 60.8 KiB / 25.0 KiB | 71.8 KiB / 15.2 KiB |
| No allocation | 9.6 KiB / 4.1 KiB | 13.5 KiB / 9.6 KiB |
| bs58 | 1.3 KiB / 0.9 KiB | 2.7 KiB / 0.8 KiB |

[Per-backend and platform sizes, allocations, RSS and source bindings](docs/benchmarks/2026-10-27/README.md)
give the full cost comparison. Cache headers and retained heap are additional
costs described above. `no_alloc` supplies the smallest general implementation.

On x86-64, `features = ["compact"]` keeps eight IFMA kernel widths instead of
twelve. This reduces linked code and uses more chunks for some multiplications.
It preserves the API and allocation contracts; the speed tradeoff depends on
operand size. Other production targets do not use these IFMA kernels.

## Development and license

Run `cargo test` and `cargo test --features cache`. The `__internals` feature
exposes unstable backend controls for tests and benchmarks; applications need neither.
Benchmark targets include bs58, fixed-length five8, and a development-only Malachite adapter.
[CORRECTNESS.md](CORRECTNESS.md) describes differential/property tests, sanitizer
fuzzing, Miri, scoped proofs and remaining coverage gaps.
[DESIGN.md](DESIGN.md) explains algorithms, dispatch and resource costs.

Implementation influences, algorithm references and upstream licenses are listed
in [REFERENCES.md](REFERENCES.md).

Licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at your option.

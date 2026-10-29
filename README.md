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
default features peak at about 126 MiB of extra requested heap for encoding and
105 MiB for decoding, excluding caller buffers. [Allocation details](DESIGN.md#heap-stack-and-cache-costs).

## Optional caching

Add `features = ["cache"]` to the dependency to retain reusable tables. This
feature enables `std`. Caching is off by default; it changes dispatch as well as
retention. First use builds tables; the warm timings below exclude that setup.
[Cold calls and cache tradeoffs](docs/performance.md#cache-and-memory) help assess
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

**Measured 2026-10-29 from this branch at `daf6403b2c45`.** These results describe
this PR alone; they do not combine the independent small- and large-input PRs.
This snapshot predates further scratch pooling. Its allocation savings and
latency-neutral paired result are in [the performance guide](docs/performance.md#pooled-radix-scratch).
Warm calls reuse output buffers and include Auto's workspace allocations. Native
uses process CPU time; Wasm uses V8 elapsed time. Each cell is **encode / decode**.
Bold marks the lowest observed median, without a statistical significance claim.
[Methods, cold calls, observed ranges and source identities](docs/performance.md).

Ordinary public consumers have no `__internals` feature. Each cell uses three
fresh processes, each with three warm batches of at least 10 ms. The reported
value is the median of the three process medians; figure bars show their range.
No-allocation and bs58 stop at 64 KiB by benchmark policy; five8 covers 32/64 B.
The figures show all supported sizes from 1 B through 4 MiB.

### Apple M5 Pro

Rust 1.99.1 / LLVM 23.1.1; compiler-default target flags.

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 11.6 ns / 15.1 ns | 11.7 ns / 14.3 ns | **9.6 ns** / **14.2 ns** | 756 ns / 322 ns | 31.6 ns / 31.5 ns |
| 64 B | 23.4 ns / 28.2 ns | 23.3 ns / 27.3 ns | **23 ns** / **27.3 ns** | 2.96 µs / 1.17 µs | 76.3 ns / 94.3 ns |
| 512 B | 2.07 µs / 660 ns | **569 ns** / **591 ns** | 2.12 µs / 705 ns | 231 µs / 76.8 µs | — / — |
| 4 KiB | 105 µs / 39.4 µs | **28.6 µs** / **25.2 µs** | 106 µs / 37.2 µs | 15.1 ms / 5.02 ms | — / — |
| 64 KiB | 2.24 ms / 1.13 ms | **2.22 ms** / **1.02 ms** | 25.6 ms / 9.03 ms | 3.94 s / 1.31 s | — / — |
| 1 MiB | 52.8 ms / 28.6 ms | **52 ms** / **27.1 ms** | — / — | — / — | — / — |
| 4 MiB | **250 ms** / 140 ms | 252 ms / **135 ms** | — / — | — / — | — / — |

![Native public API conversion time on Apple M5 Pro](assets/graphs/m5-public.svg)

### AMD EPYC 9B45 (GCP, generic x86-64)

Rust 1.99.0 / LLVM 23.1.1; generic target, runtime CPU selection enabled, CPU 0
pinned. These are ordinary public consumers. The older internal-worker native
comparison is separate in the [performance guide](docs/performance.md#generic-and-native-x86-64).

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 30.5 ns / 25.6 ns | 30.3 ns / 25.5 ns | **30.2 ns** / **25.1 ns** | 889 ns / 319 ns | 52.9 ns / 48.7 ns |
| 64 B | 31.3 ns / 47.4 ns | 31.3 ns / 47.1 ns | **31.2 ns** / **46.6 ns** | 3.91 µs / 1.25 µs | 116 ns / 123 ns |
| 512 B | 4.26 µs / 1 µs | **1.1 µs** / 1.23 µs | 4.51 µs / **991 ns** | 281 µs / 91.9 µs | — / — |
| 4 KiB | 51 µs / 27.5 µs | **40.8 µs** / **27.4 µs** | 276 µs / 55.4 µs | 18.1 ms / 5.85 ms | — / — |
| 64 KiB | 3.65 ms / 1.44 ms | **2.81 ms** / **1.21 ms** | 69.9 ms / 13.4 ms | 4.62 s / 1.49 s | — / — |
| 1 MiB | 80.8 ms / 34.2 ms | **72.8 ms** / **32.2 ms** | — / — | — / — | — / — |
| 4 MiB | 375 ms / 170 ms | **347 ms** / **170 ms** | — / — | — / — | — / — |

![Native public API conversion time on AMD EPYC 9B45](assets/graphs/gcp-public.svg)

### WebAssembly on Apple M5 Pro (Node/V8)

Rust 1.99.1 / LLVM 23.1.1; scalar Wasm; Node 26.13.0 / V8 14.6.202.34-node.40.
Host variation was substantial: byte-identical competitor modules differed
between source cohorts. These observations do not establish a source-caused
speedup. [Observed ranges and controls](docs/performance.md#timing-limits).

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 26 ns / 53.8 ns | 26.2 ns / 54.5 ns | **25.9 ns** / 52.2 ns | 616 ns / 216 ns | 45.9 ns / **42.1 ns** |
| 64 B | 104 ns / 136 ns | 105 ns / 156 ns | 102 ns / 130 ns | 2.95 µs / 843 ns | **94.9 ns** / **127 ns** |
| 512 B | 7.87 µs / 4.97 µs | **2.8 µs** / **3.04 µs** | 7.95 µs / 4.93 µs | 249 µs / 76.1 µs | — / — |
| 4 KiB | 474 µs / 216 µs | **151 µs** / **127 µs** | 482 µs / 287 µs | 16.6 ms / 5.56 ms | — / — |
| 64 KiB | 14.3 ms / 6.71 ms | **14.3 ms** / **6.67 ms** | 120 ms / 73.7 ms | 4.24 s / 1.41 s | — / — |
| 1 MiB | **345 ms** / **172 ms** | 353 ms / 176 ms | — / — | — / — | — / — |
| 4 MiB | **1.65 s** / **836 ms** | 1.66 s / 844 ms | — / — | — / — | — / — |

![WebAssembly public API conversion time on Apple M5 Pro](assets/graphs/wasm-public.svg)

## Linked size

Code and data linked **above a matching empty consumer**, in KiB, measured
2026-10-29 from the same branch snapshot as the timing tables. Both API
directions are reachable: opt-level 3, fat LTO, 16 codegen units, generic target
flags. These are linked cdylib section deltas, including associated code and
metadata. Full files also contain runtime, scaffolding and platform padding.

| Consumer | M5 code / readonly | x86-64 code / readonly | Wasm code / data section |
|---|---:|---:|---:|
| Auto | 68.0 KiB / 24.6 KiB | 279.7 KiB / 34.8 KiB | 70.1 KiB / 14.3 KiB |
| Auto + cache | 70.9 KiB / 26.8 KiB | 270.6 KiB / 35.5 KiB | 71.8 KiB / 15.1 KiB |
| No allocation | 9.6 KiB / 4.1 KiB | 20.5 KiB / 14.4 KiB | 13.5 KiB / 9.5 KiB |
| bs58 | 1.3 KiB / 0.9 KiB | 2.0 KiB / 0.9 KiB | 2.7 KiB / 0.8 KiB |

Readonly constants, initialized data, BSS, cache headers, retained heap and Wasm
linear-memory pages are different costs. The pair lookup table has 3,364 entries
of two bytes (6,728 B); whether it survives linking depends on reachable paths.
[Section accounting, cache memory and historical compact results](docs/performance.md).

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

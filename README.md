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

**Measured 2026-10-29 from this branch at `8ca392a84c48`.** These results describe
this PR alone; they do not combine the independent small- and large-input PRs.
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
| 32 B | 11.6 ns / 15.1 ns | 11.7 ns / **14.2 ns** | **9.63 ns** / 14.2 ns | 760 ns / 321 ns | 31.1 ns / 31.2 ns |
| 64 B | 23.8 ns / **27.2 ns** | 23.4 ns / 27.7 ns | **23 ns** / 27.5 ns | 2.96 µs / 1.15 µs | 72.1 ns / 94.5 ns |
| 512 B | 1.97 µs / 657 ns | **577 ns** / **578 ns** | 2.04 µs / 708 ns | 230 µs / 76.2 µs | — / — |
| 4 KiB | 103 µs / 39 µs | **28.6 µs** / **25.3 µs** | 104 µs / 36.9 µs | 15.1 ms / 5 ms | — / — |
| 64 KiB | 2.71 ms / 1.12 ms | **2.23 ms** / **1.02 ms** | 25.8 ms / 9.08 ms | 3.92 s / 1.3 s | — / — |
| 1 MiB | 63 ms / 28.6 ms | **58.2 ms** / **26.8 ms** | — / — | — / — | — / — |
| 4 MiB | 301 ms / 140 ms | **276 ms** / **135 ms** | — / — | — / — | — / — |

![Native public API conversion time on Apple M5 Pro](assets/graphs/m5-public.svg)

### AMD EPYC 9B45 (GCP, generic x86-64)

Rust 1.99.0 / LLVM 23.1.1; generic target, runtime CPU selection enabled, CPU 0
pinned. These are ordinary public consumers. The older internal-worker native
comparison is separate in the [performance guide](docs/performance.md#generic-and-native-x86-64).

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 30.6 ns / 25.7 ns | **30.2 ns** / 25.4 ns | 30.3 ns / **25.1 ns** | 886 ns / 320 ns | 52.9 ns / 49 ns |
| 64 B | **31.2 ns** / 47.5 ns | 31.2 ns / 46.9 ns | 31.3 ns / **46.6 ns** | 3.9 µs / 1.26 µs | 116 ns / 122 ns |
| 512 B | 4.07 µs / 1.23 µs | **1.1 µs** / 999 ns | 4.3 µs / **997 ns** | 282 µs / 91.8 µs | — / — |
| 4 KiB | 49.6 µs / **27.4 µs** | **41.6 µs** / 33.2 µs | 265 µs / 55.3 µs | 18 ms / 5.85 ms | — / — |
| 64 KiB | 3.66 ms / 1.43 ms | **2.82 ms** / **1.3 ms** | 67.4 ms / 13.5 ms | 4.62 s / 1.49 s | — / — |
| 1 MiB | 81.1 ms / 34.2 ms | **73.6 ms** / **33.1 ms** | — / — | — / — | — / — |
| 4 MiB | 373 ms / **171 ms** | **345 ms** / 175 ms | — / — | — / — | — / — |

![Native public API conversion time on AMD EPYC 9B45](assets/graphs/gcp-public.svg)

### WebAssembly on Apple M5 Pro (Node/V8)

Rust 1.99.1 / LLVM 23.1.1; scalar Wasm; Node 26.13.0 / V8 14.6.202.34-node.40.
Host variation was substantial: byte-identical competitor modules differed
between source cohorts. These observations do not establish a source-caused
speedup. [Observed ranges and controls](docs/performance.md#timing-limits).

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | **26 ns** / 58.7 ns | 26 ns / 56.4 ns | 26.4 ns / 53.8 ns | 695 ns / 221 ns | 46.4 ns / **42.9 ns** |
| 64 B | 103 ns / 133 ns | 105 ns / 171 ns | 110 ns / 131 ns | 3.03 µs / 844 ns | **99.5 ns** / **127 ns** |
| 512 B | 8.85 µs / 6.21 µs | **3.81 µs** / **2.77 µs** | 8.95 µs / 4.92 µs | 250 µs / 76.1 µs | — / — |
| 4 KiB | 466 µs / 224 µs | **158 µs** / **134 µs** | 468 µs / 285 µs | 16.2 ms / 5.42 ms | — / — |
| 64 KiB | **14.3 ms** / **6.71 ms** | 15.6 ms / 10.3 ms | 119 ms / 71.8 ms | 4.24 s / 1.39 s | — / — |
| 1 MiB | **370 ms** / **173 ms** | 431 ms / 174 ms | — / — | — / — | — / — |
| 4 MiB | 2.33 s / **887 ms** | **1.71 s** / 889 ms | — / — | — / — | — / — |

![WebAssembly public API conversion time on Apple M5 Pro](assets/graphs/wasm-public.svg)

## Linked size

Code and data linked **above a matching empty consumer**, in KiB, measured
2026-10-29 from the same branch snapshot as the timing tables. Both API
directions are reachable: opt-level 3, fat LTO, 16 codegen units, generic target
flags. These are linked cdylib section deltas, including associated code and
metadata. Full files also contain runtime, scaffolding and platform padding.

| Consumer | M5 code / readonly | x86-64 code / readonly | Wasm code / data section |
|---|---:|---:|---:|
| Auto | 58.0 KiB / 22.5 KiB | 279.5 KiB / 34.8 KiB | 69.9 KiB / 14.3 KiB |
| Auto + cache | 60.4 KiB / 25.0 KiB | 270.2 KiB / 35.5 KiB | 71.6 KiB / 15.1 KiB |
| No allocation | 9.4 KiB / 10.7 KiB | 20.3 KiB / 14.5 KiB | 13.5 KiB / 9.6 KiB |
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

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

Reusable configurations can be constants. Conversion starts at the terminal method:

```rust
# #[cfg(feature = "alloc")]
# {
use faster_base58::{Encoder, Decoder};

const ENCODER: Encoder = Encoder::bitcoin();
const DECODER: Decoder = Decoder::bitcoin();

let data = [0u8; 21];
let text: String = ENCODER.encode(&data).into_string();
assert_eq!(DECODER.decode(&text).into_array::<21>()?, data);
let mut buffer = [0; ENCODER.max_encoded_len(21)];
assert_eq!(ENCODER.encode(&data).onto(&mut buffer)?, text);
# }
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Encoder` and `Decoder` are small `Copy` values without type parameters.
Constructors, setters, `.encode()` and `.decode()` are `const fn` and always inlined,
so constant configurations fold away; runtime-selected settings still dispatch.

Encoding provides `.into_string()`, `.into_vec()` (ASCII bytes), and `.onto()`
(borrowed `&str`). Decoding provides `.into_vec()`, `.into_array::<N>()`, and
`.onto()` (borrowed `&mut [u8]`). Input can be borrowed or owned. Decoded input
implements `AsRef<[u8]>`: strings and raw bytes work, and bytes need no UTF-8
validation first. Owned output methods require `alloc` (enabled by
default). Borrowed output and exact arrays are available with either allocation policy.

```rust
use faster_base58::{Encoder, Decoder};

const ENCODER: Encoder = Encoder::bitcoin();
const DECODER: Decoder = Decoder::bitcoin();
let input = [0, 1, 2, 3];
let mut text_buffer = [0; ENCODER.max_encoded_len(4)];
let mut byte_buffer = [0; 4];
let text = ENCODER.encode(input).onto(&mut text_buffer)?;
assert_eq!(DECODER.decode(text).into_array::<4>()?, input);
assert_eq!(DECODER.decode(text).onto(&mut byte_buffer)?, input);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`ENCODER.max_encoded_len(n)` bounds encoded output; exact result capacity also
works. Plain decoding needs at most one output byte per input text byte. Leading
zero bytes round-trip as leading `'1'` characters. `.into_array::<N>()` requires
exactly `N` bytes: shorter results return `InvalidLength`, longer ones `BufferTooSmall`
(decoding stops when the array is full, so the actual length is not known).

Invalid-character indices are byte offsets into the input. `Error` is `#[non_exhaustive]`;
downstream matches need a wildcard arm. Errors implement `Display` and
the standard error trait. Successful calls leave the unused output tail untouched;
on errors, output contents and error precedence are unspecified. Conversion timing
depends on input values and length; these functions are not constant-time.

## Optional Base58Check

Enable `features = ["check"]` to add Bitcoin's four-byte double-SHA-256 checksum.
This enables an optional `sha2` dependency without requiring `std`. With the
feature disabled, normal dependencies do not include SHA-256. Select checking
on a configuration:

```rust
# #[cfg(all(feature = "check", feature = "alloc"))]
# fn main() -> Result<(), faster_base58::Error> {
use faster_base58::{Encoder, Decoder};

const ENCODER: Encoder = Encoder::bitcoin().with_check();
const DECODER: Decoder = Decoder::bitcoin().with_check();

let data = [0u8; 21];
let text: String = ENCODER.encode(&data).into_string();
assert_eq!(text, "1111111111111111111114oLvT2");
assert_eq!(DECODER.decode(&text).into_array::<21>()?, data);
let mut bytes = [0u8; 21];
assert_eq!(DECODER.decode(&text).onto(&mut bytes)?, data);

const VERSIONED_ENCODER: Encoder = Encoder::bitcoin().with_check_version(0);
const VERSIONED_DECODER: Decoder = Decoder::bitcoin().with_check_version(0);
let text = VERSIONED_ENCODER.encode([0u8; 20]).into_string();
assert_eq!(VERSIONED_DECODER.decode(&text).into_array::<21>()?, data);
# Ok(())
# }
# #[cfg(not(all(feature = "check", feature = "alloc")))]
# fn main() {}
```

`.with_check()` hashes the input as supplied. `.with_check_version(v)` prepends
`v` on encode and requires that first data byte on decode. The version is included
in the checksum and **retained** in decoded output. Setters replace the previous
setting; `.with_check()` clears any configured version. Versions are one byte;
multi-byte prefixes, such as BIP32's four-byte extended-key version, are part of
the payload: use `.with_check()` and keep them in the data.

Checked decoding removes the checksum. Errors distinguish `MissingChecksum`,
`InvalidChecksum`, and `InvalidVersion { expected, actual }`; `actual: None`
means valid empty data has no version byte. These variants and setters are gated
by `check`. Checksum verification precedes version validation.

`ENCODER.max_encoded_len(n)` includes checksum and any inserted version. Checked
decoding with `.onto()` or `.into_array::<N>()` needs room only for the returned
data (including a retained version): the checksum is held on the stack, and the
unused output tail stays untouched. Neither direction allocates a temporary frame;
normal codec workspace still applies to large inputs, and hashing adds work.
Enabling the feature does not select checking for plain configurations.

## No internal allocation

Disable default features to exclude allocating backends and owned output methods:

```toml
faster-base58 = { git = "https://github.com/elichai/faster-base58", default-features = false }
```

The same `Encoder` / `Decoder` buffer and array methods then make no heap
allocations, including first calls and error paths. Encoding uses fixed tables for
32/64-byte frames and batched conversion elsewhere; decoding uses tiny/batched
conversion. General large inputs take quadratic time. The optional `check` feature
works with either policy. `cache` and development-only `__internals` enable `alloc`.
Cargo features are additive: any dependency that enables `alloc` enables it for
all consumers of that crate instance. Dropping an owned input can still call its
allocator; borrow inputs when counting all allocator operations.

Auto trades workspace for large-input speed. At 4 MiB on the M5 Pro, default
encoding peaks at about 66 MiB of extra requested heap in 7 allocations and
decoding at about 54 MiB in 18, excluding caller buffers (2026-11-07; [allocation
details](docs/performance.md#cache-and-memory)).

## Optional caching

Add `features = ["cache"]` to the dependency to retain reusable tables. This
feature enables `std`. Caching is off by default; it changes dispatch as well as
retention. First use builds tables; the warm timings below exclude that setup.
[Cache and memory costs](docs/performance.md#cache-and-memory) help assess
whether reuse repays the setup and memory cost for your workload.

Enabling `cache` alone allocates **zero heap bytes**. Conversions allocate tables
on first use and retain them for the process lifetime. Only needed table classes
are initialized; retained heap depends on input sizes, CPU and call history.

| Cache family | Maximum retained heap payload across production classes |
| --- | ---: |
| Encoding and decoding Matrix tables | 18,617,760 B (17.8 MiB) |
| NTT twiddles | 9,436,416 B (9.0 MiB) |
| **Combined** | **28,054,176 B (26.8 MiB)** |

This is retained table capacity, **not a limit on total active heap**: per-call
scratch, larger owned tables, allocator overhead and concurrent calls add to it.
The static headers of the cache families take 2,176 bytes when present. None of
these figures measures RSS. [Memory and algorithm details](DESIGN.md#heap-stack-and-cache-costs).

## Performance

Measured **2026-11-07** on this revision with ordinary public consumers (no
`__internals`), release opt-level 3, fat LTO and generic target flags. Each cell is
**encode / decode**: the median over fresh processes, each timing all libraries
interleaved in batches of at least 10 ms. On the EPYC, where byte-identical library
files can run at different speeds, each library ran as four fresh file copies and a
cell is the median over copies. Native cells use process CPU time, Wasm V8 elapsed
time. Workspace allocation is included; caller buffers are reused. Bold marks the
lowest median, without a significance claim. No-allocation and bs58 stop at 64 KiB;
five8 covers 32/64 B. [Method, ranges and limits](docs/performance.md).

### Apple M5 Pro

Rust 1.99.1 / LLVM 23.1.1, `aarch64-apple-darwin`, five processes.

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 11.9 ns / **13.8 ns** | 11.9 ns / 14 ns | **9.84 ns** / 13.9 ns | 831 ns / 342 ns | 33.3 ns / 32.4 ns |
| 64 B | **23.9 ns** / 26.6 ns | 24.1 ns / **26.6 ns** | 23.9 ns / 26.8 ns | 3.15 µs / 1.38 µs | 78.4 ns / 96.2 ns |
| 512 B | 1.91 µs / **534 ns** | **594 ns** / 589 ns | 1.97 µs / 535 ns | 246 µs / 81.2 µs | — / — |
| 4 KiB | 110 µs / 36.1 µs | **29.9 µs** / **26.1 µs** | 114 µs / 27.4 µs | 16.1 ms / 5.36 ms | — / — |
| 64 KiB | 1.98 ms / 1.06 ms | **1.93 ms** / **1.02 ms** | 27 ms / 6.51 ms | 4.15 s / 1.38 s | — / — |
| 1 MiB | 46.2 ms / 27.7 ms | **46.1 ms** / **26.9 ms** | — / — | — / — | — / — |
| 4 MiB | 220 ms / 137 ms | **218 ms** / **135 ms** | — / — | — / — | — / — |

![Native public API conversion time on Apple M5 Pro](assets/graphs/m5-public.svg)

### AMD EPYC 9B45 (GCP, generic x86-64)

Rust 1.99.0 / LLVM 23.1.1, generic `x86_64-unknown-linux-gnu` with runtime CPU
selection, CPU 0 pinned, three processes, four fresh copies of each library file.
In 17 default Auto cells from 1 KiB to 4 MiB, one copy took 5–37% longer than the
fastest ([list](docs/performance.md#results)); the median over copies excludes it.

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 30 ns / **24.1 ns** | **30 ns** / 25.2 ns | 30 ns / 24.9 ns | 903 ns / 322 ns | 53.1 ns / 48.5 ns |
| 64 B | **31.4 ns** / **45.5 ns** | 31.6 ns / 46.4 ns | 31.4 ns / 46.1 ns | 3.88 µs / 1.24 µs | 115 ns / 122 ns |
| 512 B | 4.03 µs / 878 ns | **1.08 µs** / **869 ns** | 4.28 µs / 871 ns | 282 µs / 92.3 µs | — / — |
| 4 KiB | 49.8 µs / 22.6 µs | **40.3 µs** / **22.4 µs** | 267 µs / 43.5 µs | 18.1 ms / 5.85 ms | — / — |
| 64 KiB | 3.46 ms / 1.2 ms | **2.79 ms** / **1.15 ms** | 67.5 ms / 10.5 ms | 4.62 s / 1.49 s | — / — |
| 1 MiB | 71.3 ms / 29.5 ms | **65.7 ms** / **28.7 ms** | — / — | — / — | — / — |
| 4 MiB | 333 ms / 144 ms | **310 ms** / **144 ms** | — / — | — / — | — / — |

![Native public API conversion time on AMD EPYC 9B45](assets/graphs/gcp-public.svg)

### WebAssembly on Apple M5 Pro (Node/V8)

Scalar Wasm, Rust 1.99.1; Node 26.13.0 / V8 14.6.202.34; three processes.

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 25.6 ns / 49.6 ns | 25.5 ns / 52.5 ns | **25.3 ns** / 49 ns | 651 ns / 215 ns | 45 ns / **41 ns** |
| 64 B | 108 ns / 138 ns | 104 ns / 131 ns | 106 ns / **125 ns** | 2.97 µs / 817 ns | **98.4 ns** / 125 ns |
| 512 B | 7.48 µs / 5.12 µs | **2.81 µs** / **2.57 µs** | 7.59 µs / 5.01 µs | 232 µs / 72.9 µs | — / — |
| 4 KiB | 466 µs / 215 µs | **149 µs** / **124 µs** | 449 µs / 296 µs | 16.2 ms / 5.34 ms | — / — |
| 64 KiB | 14.8 ms / 6.68 ms | **14.8 ms** / **6.61 ms** | 121 ms / 76.1 ms | 4.15 s / 1.4 s | — / — |
| 1 MiB | **360 ms** / 172 ms | 360 ms / **172 ms** | — / — | — / — | — / — |
| 4 MiB | **1.72 s** / 842 ms | **1.72 s** / **838 ms** | — / — | — / — | — / — |

![WebAssembly public API conversion time on Apple M5 Pro](assets/graphs/wasm-public.svg)

A separate [basexx comparison](docs/performance.md#basexx-comparison) includes a
general-purpose comparator that is much faster than bs58.

## Linked size

Code and data linked **above an empty consumer** with the same scaffold, both
directions reachable, from the same builds as the timing tables. These are cdylib
section deltas, including associated code and metadata; readonly data, BSS, cache
headers, retained heap and Wasm memory pages are separate costs.

| Consumer | M5 code / readonly | x86-64 code / readonly | Wasm code / data section |
|---|---:|---:|---:|
| Auto | 71.7 KiB / 24.5 KiB | 280.1 KiB / 35.4 KiB | 71.9 KiB / 14.1 KiB |
| Auto + cache | 74.8 KiB / 26.7 KiB | 274.5 KiB / 35.7 KiB | 73.6 KiB / 14.9 KiB |
| No allocation | 10.2 KiB / 10.6 KiB | 20.9 KiB / 14.3 KiB | 14.0 KiB / 9.5 KiB |
| bs58 | 1.4 KiB / 0.9 KiB | 2.0 KiB / 0.9 KiB | 2.7 KiB / 0.7 KiB |

On x86-64, `features = ["compact"]` keeps eight IFMA kernel widths instead of
twelve. This reduces linked code and uses more chunks for some multiplications.
It preserves the API and allocation contracts; the speed tradeoff depends on
operand size. Other production targets do not use these IFMA kernels.

## Development and license

[CI.md](CI.md) lists the CI jobs and their local commands (most suites need
`--features __internals`; checksum suites need `check`).
[CORRECTNESS.md](CORRECTNESS.md) describes the differential/property tests,
sanitizer fuzzing, Miri, scoped proofs and remaining gaps.
[DESIGN.md](DESIGN.md) explains algorithms, dispatch and resource costs. The
`__internals` feature exposes unstable backend controls for tests and benchmarks;
applications need neither. Benchmarks compare bs58, basexx, fixed-length five8 and
a development-only Malachite adapter.

Implementation influences, algorithm references and upstream licenses are listed
in [REFERENCES.md](REFERENCES.md).

Licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at your option.

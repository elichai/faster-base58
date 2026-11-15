# faster-base58

Bitcoin Base58 encoding and decoding in Rust, with automatic algorithm and CPU
selection. The crate is `no_std` by default. Disable the default `alloc` feature
for heap-free conversion.

```toml
[dependencies]
faster-base58 = "0.1.0"
```

Requires Rust 1.95 or newer.

## Performance

Timings measured **2026-11-15**, source
[`8f7e439dcee9`](https://github.com/elichai/faster-base58/commit/8f7e439dcee94ca4e31ae46c5ea1e28c6c81d5d7).
Warm public-API **encode / decode** times with reused caller output, generic CPU
flags, release opt-level 3, 16 codegen units and fat LTO. Internal workspace
allocation is timed.
`Auto` uses default features, `Auto + cache` enables caching, and `No allocation`
disables default features. Comparators are bs58 0.5.1 and five8 1.0.0.
Each process interleaves all libraries in three batches calibrated to at least 10 ms.
Bold marks the lowest observed median without a significance claim. No-allocation
and bs58 cover through 64 KiB; five8 covers 32/64 B.
[Methods, ranges, both LTO settings, first calls and resource costs](docs/performance.md).

In the Auto columns, the second line is **encode / decode peak internal heap**
for a warm call, including retained cache tables. **Input and output storage are
excluded.** Heap counts use three fresh-process repeats and a separate fixture
at each payload size; they are requested Rust layout bytes, not RSS. The
no-allocation build uses zero internal heap. Heap measurements are dated
**2026-11-15**, source
[`d5c89dfcd675`](https://github.com/elichai/faster-base58/commit/d5c89dfcd67502d6067a5acf9594e565f837ede8).
[Heap measurement scope and first-call costs](docs/performance.md#cache-and-memory).

### Apple M5 Pro

Rust 1.99.1 / LLVM 23.1.1, AArch64; process CPU time, 5 processes × 3 file copies.

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 11.9 ns / 13.8 ns<br>heap 0 B / 0 B | 12.1 ns / **13.8 ns**<br>heap 0 B / 0 B | **9.93 ns** / 13.9 ns | 836 ns / 342 ns | 34.1 ns / 32.7 ns |
| 64 B | 24.4 ns / 26.3 ns<br>heap 0 B / 0 B | 24.3 ns / 26.4 ns<br>heap 0 B / 0 B | **24.2 ns** / **25.9 ns** | 3.17 µs / 1.38 µs | 80.7 ns / 96.8 ns |
| 512 B | 1.84 µs / 529 ns<br>heap 0 B / 0 B | **587 ns** / **518 ns**<br>heap 47 KiB / 40.2 KiB | 1.95 µs / 529 ns | 248 µs / 82.5 µs | - / - |
| 4 KiB | 73.6 µs / 26.2 µs<br>heap 82.5 KiB / 0 B | **29.5 µs** / **26.2 µs**<br>heap 2.76 MiB / 2.32 MiB | 108 µs / 26.9 µs | 16.1 ms / 5.44 ms | - / - |
| 64 KiB | 1.88 ms / 994 µs<br>heap 1.28 MiB / 1.02 MiB | **1.86 ms** / **969 µs**<br>heap 1.53 MiB / 1.17 MiB | 25.9 ms / 6.33 ms | 4 s / 1.36 s | - / - |
| 1 MiB | **44.4 ms** / 26.2 ms<br>heap 17 MiB / 14.1 MiB | 44.8 ms / **26 ms**<br>heap 17 MiB / 14.2 MiB | - / - | - / - | - / - |
| 4 MiB | **208 ms** / **127 ms**<br>heap 66.4 MiB / 54.2 MiB | 210 ms / 128 ms<br>heap 66.4 MiB / 54.7 MiB | - / - | - / - | - / - |

![Public encode and decode times on Apple M5 Pro](assets/graphs/m5-public.svg)

### AMD EPYC 9B45 (generic x86-64)

Rust 1.99.0 / LLVM 23.1.1, generic x86-64 with runtime CPU selection.
Process CPU time, 3 processes × 4 file copies, pinned to CPU 1.

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | 30.3 ns / 24.4 ns<br>heap 0 B / 0 B | **30.2 ns** / **23.8 ns**<br>heap 0 B / 0 B | 30.2 ns / 23.9 ns | 911 ns / 327 ns | 55.8 ns / 49.4 ns |
| 64 B | 31.7 ns / **45.1 ns**<br>heap 0 B / 0 B | 31.8 ns / 45.2 ns<br>heap 0 B / 0 B | **31.7 ns** / 45.5 ns | 3.9 µs / 1.25 µs | 116 ns / 129 ns |
| 512 B | 3.24 µs / **856 ns**<br>heap 5.2 KiB / 0 B | **1.09 µs** / 864 ns<br>heap 47 KiB / 0 B | 4.08 µs / 866 ns | 283 µs / 93.3 µs | - / - |
| 4 KiB | 48.2 µs / 16 µs<br>heap 38.4 KiB / 21.3 KiB | **40 µs** / **15.7 µs**<br>heap 91.8 KiB / 30.3 KiB | 244 µs / 41.5 µs | 18.2 ms / 5.86 ms | - / - |
| 64 KiB | 2.23 ms / 1.03 ms<br>heap 1.28 MiB / 952 KiB | **2.11 ms** / **829 µs**<br>heap 1.53 MiB / 1000 KiB | 61.4 ms / 9.96 ms | 4.64 s / 1.49 s | - / - |
| 1 MiB | **48.5 ms** / 24.3 ms<br>heap 17 MiB / 14 MiB | 48.8 ms / **22.9 ms**<br>heap 17 MiB / 14.1 MiB | - / - | - / - | - / - |
| 4 MiB | **230 ms** / 119 ms<br>heap 66.4 MiB / 54.1 MiB | **230 ms** / **116 ms**<br>heap 66.4 MiB / 54.1 MiB | - / - | - / - | - / - |

![Public encode and decode times on AMD EPYC 9B45 (GCP, generic x86-64)](assets/graphs/gcp-public.svg)

### WebAssembly on Apple M5 Pro (Node/V8)

Rust 1.99.1; Node 26.13.1 / V8 14.6.202.34; scalar Wasm, V8 elapsed time, 5 processes.

| Payload | Auto | Auto + cache | No allocation | bs58 | five8 |
|---|---:|---:|---:|---:|---:|
| 32 B | **25.1 ns** / 51.9 ns<br>heap 0 B / 0 B | 25.2 ns / 50 ns<br>heap 0 B / 0 B | 25.3 ns / 52.5 ns | 598 ns / 216 ns | 44.5 ns / **40.8 ns** |
| 64 B | 98.8 ns / 136 ns<br>heap 0 B / 0 B | 99.3 ns / 131 ns<br>heap 0 B / 0 B | 100 ns / 136 ns | 2.8 µs / 819 ns | **91.5 ns** / **124 ns** |
| 512 B | 7.15 µs / 6.06 µs<br>heap 0 B / 0 B | **2.77 µs** / **2.49 µs**<br>heap 46.9 KiB / 40 KiB | 7.36 µs / 5.87 µs | 232 µs / 73 µs | - / - |
| 4 KiB | 286 µs / 189 µs<br>heap 82.3 KiB / 60.3 KiB | **146 µs** / **122 µs**<br>heap 2.76 MiB / 2.32 MiB | 445 µs / 336 µs | 15.3 ms / 5.18 ms | - / - |
| 64 KiB | 8.58 ms / 5.98 ms<br>heap 1.28 MiB / 1.02 MiB | **8.51 ms** / **5.93 ms**<br>heap 1.53 MiB / 1.02 MiB | 111 ms / 81.3 ms | 3.93 s / 1.33 s | - / - |
| 1 MiB | **220 ms** / **155 ms**<br>heap 17 MiB / 14.1 MiB | **220 ms** / **155 ms**<br>heap 17 MiB / 14.1 MiB | - / - | - / - | - / - |
| 4 MiB | **1.07 s** / 766 ms<br>heap 66.4 MiB / 54.2 MiB | 1.07 s / **765 ms**<br>heap 66.4 MiB / 54.2 MiB | - / - | - / - | - / - |

![Public encode and decode times on Scalar WebAssembly on Apple M5 Pro (Node/V8)](assets/graphs/wasm-public.svg)

[Wasm SIMD128 results](docs/performance.md#wasm-simd128) use separate builds.

## Encode and decode

Configure an encoder or decoder, then choose owned or borrowed output:

```rust
use faster_base58::{Encoder, Decoder};

const ENCODER: Encoder = Encoder::bitcoin();
const DECODER: Decoder = Decoder::bitcoin();

let data = [0u8; 21];
let text: String = ENCODER.encode(&data).into_string();
assert_eq!(DECODER.decode(&text).into_array::<21>().unwrap(), data);
let mut buffer = [0; ENCODER.max_encoded_len(21)];
assert_eq!(ENCODER.encode(&data).onto(&mut buffer).unwrap(), text);
```

`Encoder` and `Decoder` are small `Copy` values and can be constants.
These examples use `unwrap()` for brevity; handle errors in application code.

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
let text = ENCODER.encode(input).onto(&mut text_buffer).unwrap();
assert_eq!(DECODER.decode(text).into_array::<4>().unwrap(), input);
assert_eq!(DECODER.decode(text).onto(&mut byte_buffer).unwrap(), input);
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
use faster_base58::{Encoder, Decoder};

const ENCODER: Encoder = Encoder::bitcoin().with_check();
const DECODER: Decoder = Decoder::bitcoin().with_check();

let data = [0u8; 21];
let text: String = ENCODER.encode(&data).into_string();
assert_eq!(text, "1111111111111111111114oLvT2");
assert_eq!(DECODER.decode(&text).into_array::<21>().unwrap(), data);
let mut bytes = [0u8; 21];
assert_eq!(DECODER.decode(&text).onto(&mut bytes).unwrap(), data);

const VERSIONED_ENCODER: Encoder = Encoder::bitcoin().with_check_version(0);
const VERSIONED_DECODER: Decoder = Decoder::bitcoin().with_check_version(0);
let text = VERSIONED_ENCODER.encode([0u8; 20]).into_string();
assert_eq!(VERSIONED_DECODER.decode(&text).into_array::<21>().unwrap(), data);
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
faster-base58 = { version = "0.1.0", default-features = false }
```

The same `Encoder` / `Decoder` buffer and array methods then make no heap
allocations, including first calls and error paths. Encoding uses fixed tables for
32/64-byte frames and batched conversion elsewhere; decoding uses tiny/batched
conversion. General large inputs take quadratic time. The optional `check` feature
works with either policy. `cache` and development-only `__internals` enable `alloc`.
Cargo features are additive: any dependency that enables `alloc` enables it for
all consumers of that crate instance. Dropping an owned input can still call its
allocator; borrow inputs when counting all allocator operations.

## Optional caching

Add `features = ["cache"]` to retain reusable tables and enable `std`. Caching is
off by default and changes algorithm selection. Enabling it allocates no heap
storage until a conversion needs tables. Those tables remain for the process
lifetime; retention depends on input sizes, CPU and call history. Warm timings
exclude first-use setup. See [cache and memory costs](docs/performance.md#cache-and-memory).

| Cache family | Maximum retained heap payload across production classes |
| --- | ---: |
| Encoding and decoding Matrix tables | 18,617,760 B (17.8 MiB) |
| NTT twiddles | 9,436,416 B (9.0 MiB) |
| **Combined** | **28,054,176 B (26.8 MiB)** |

This is retained table capacity, **not a limit on total active heap**: per-call
scratch, larger owned tables, allocator overhead and concurrent calls add to it.
The static headers of the cache families take 2,176 bytes when present. None of
these figures measures RSS. [Memory and algorithm details](DESIGN.md#heap-stack-and-cache-costs).

## Linked size

Code and readonly/data section bytes **above a matching empty consumer**, from
the same fat-LTO builds as the timing figures, with both directions reachable.
These linked-consumer sizes include associated code and metadata. BSS, retained
heap, stack and Wasm linear memory are separate costs.
[Full size accounting and LTO-off results](docs/performance.md#linked-size-and-compact).

| Consumer | M5 code / readonly | x86-64 code / readonly | Wasm code / data section |
|---|---:|---:|---:|
| Auto | 66.2 KiB / 24.4 KiB | 300.9 KiB / 37.7 KiB | 84.2 KiB / 15.8 KiB |
| Auto + cache | 74.6 KiB / 27.2 KiB | 296.9 KiB / 38.6 KiB | 89.9 KiB / 16.7 KiB |
| No allocation | 10.4 KiB / 10.9 KiB | 21.3 KiB / 14.5 KiB | 14.8 KiB / 9.9 KiB |
| bs58 | 1.4 KiB / 0.9 KiB | 2.0 KiB / 0.9 KiB | 2.7 KiB / 0.8 KiB |
| five8 | 11.8 KiB / 2.0 KiB | 11.8 KiB / 1.7 KiB | 10.0 KiB / 1.6 KiB |

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

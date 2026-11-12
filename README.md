# faster-base58

Bitcoin Base58 encoding and decoding in Rust. The default API selects algorithms
by input size and CPU support. It is `no_std` by default, using `alloc` for its
faster general conversions. On x86-64, runtime SIMD detection uses the standard
library when `std` is enabled, and `cpufeatures` otherwise.

```toml
[dependencies]
faster-base58 = "0.1.0"
```

Requires Rust 1.95 or newer.

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

Auto trades workspace for large-input speed. At 4 MiB on the M5 Pro, default
encoding peaks at about 66 MiB of extra requested heap in 7 allocations and
decoding at about 54 MiB in 18, excluding caller buffers (2026-11-05; [allocation
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

Measured **2026-11-12**: the complete #53 → #55 → #57 → #59 → #67 stack,
[`3fcfbaff62f3`](https://github.com/elichai/faster-base58/commit/3fcfbaff62f3257ddc6de357f4856cf6225eabf8),
against main [`65ea1581aae4`](https://github.com/elichai/faster-base58/commit/65ea1581aae4d5da128abd70dcac3ead50cb7f65).
These are whole-stack results; they do not isolate #67's contribution.

Ordinary public Auto with caller buffers, generic CPU flags, release opt-level 3,
16 codegen units, default/cache and fat/off LTO. Tables give **stack warm
encode / decode medians with fat LTO**. Figures show paired stack/main ratios;
below 1 is faster. Bars span observed process ratios, not confidence intervals.
The [performance guide](docs/performance.md#current-native-results) gives both
LTO settings, all 27 sizes, first calls, linked sizes, controls and limitations.

The large encoder slowdown is absent from this matrix. Results are mixed:
M5 cached 512-byte decode is 2.7% slower in the primary fat-LTO run and 1.6%
slower in a focused repeat. GCP no-LTO 32-byte encode shifts in both stack and
identical-main controls, then returns to about 1.000× in a fully rotated repeat.
This is not evidence that every changed path is neutral or faster.

### Apple M5 Pro

| Payload | Default encode / decode | Cache encode / decode |
| --- | ---: | ---: |
| 32 B | 11.8 ns / 13.7 ns | 11.9 ns / 13.6 ns |
| 64 B | 24.1 ns / 26 ns | 23.9 ns / 25.8 ns |
| 512 B | 1.83 µs / 519 ns | 582 ns / 505 ns |
| 4 KiB | 73.9 µs / 26.1 µs | 29.1 µs / 24.6 µs |
| 1 MiB | 43.6 ms / 25.8 ms | 43.5 ms / 25.3 ms |
| 4 MiB | 206 ms / 126 ms | 206 ms / 126 ms |

![Current whole-stack paired latency ratios on Apple M5 Pro](assets/graphs/m5-public.svg)

### AMD EPYC 9B45 (GCP)

| Payload | Default encode / decode | Cache encode / decode |
| --- | ---: | ---: |
| 32 B | 30.1 ns / 24.4 ns | 30 ns / 23.2 ns |
| 64 B | 31.1 ns / 44.8 ns | 31.5 ns / 45.1 ns |
| 512 B | 3.18 µs / 854 ns | 1.08 µs / 857 ns |
| 4 KiB | 47.5 µs / 15.7 µs | 39.6 µs / 15.8 µs |
| 1 MiB | 47.5 ms / 24 ms | 47.6 ms / 22.6 ms |
| 4 MiB | 221 ms / 120 ms | 220 ms / 116 ms |

![Current whole-stack paired latency ratios on AMD EPYC 9B45](assets/graphs/gcp-public.svg)

### WebAssembly

Current-stack Wasm timings and linked sizes have not been measured. The figure
marks that gap. No-allocation, comparator, Base58Check, owned-output, native-CPU
and counted-allocation performance are outside this native cohort.

![Wasm performance awaits current-source measurements](assets/graphs/wasm-public.svg)

## Linked size

The **same native timing artifacts**, with both directions reachable, link the
following bytes above a matching empty consumer. Δ compares stack with main.
Code and readonly data are separate cdylib section differences. They are not
crate-only instruction counts. File layout, BSS, stack and heap are separate costs.
The [complete size table](docs/performance.md#linked-size-and-compact) includes
initialized data, signed BSS and stripped-file differences.

| Host / LTO / policy | Stack code B | Stack readonly B | Code Δ B | Readonly Δ B |
| --- |  ---: | ---: | ---: | ---: |
| m5 / fat / default | 67,760 | 24,950 | -1,664 | -961 |
| m5 / fat / cache | 76,232 | 27,768 | -1,600 | -985 |
| m5 / off / default | 80,728 | 50,006 | +0 | -569 |
| m5 / off / cache | 84,760 | 58,019 | +0 | -465 |
| gcp / fat / default | 308,157 | 38,444 | -1,120 | -536 |
| gcp / fat / cache | 303,789 | 39,280 | -1,056 | -584 |
| gcp / off / default | 332,787 | 75,964 | -8,848 | -696 |
| gcp / off / cache | 324,707 | 72,072 | -8,240 | -296 |

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

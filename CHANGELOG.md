# Changelog

## [Unreleased]

## [0.1.0](https://github.com/elichai/faster-base58/releases/tag/v0.1.0) - 2026-11-12

Initial release:

- Bitcoin Base58 encoding and decoding with reusable `Encoder` and `Decoder`
  configurations, caller-provided buffers, exact arrays and owned results.
- Optional Base58Check with checksum verification and one-byte version prefixes.
- `no_std` support. Disable the default `alloc` feature for heap-free conversion.
- Automatic algorithm and CPU selection, including NEON, AVX2, AVX-512 and Wasm
  SIMD128 kernels on supported targets.
- Optional table caching and smaller x86 IFMA kernels through `cache` and `compact`.

See [README.md](README.md) for usage and [CORRECTNESS.md](CORRECTNESS.md) for
verification scope and remaining gaps.

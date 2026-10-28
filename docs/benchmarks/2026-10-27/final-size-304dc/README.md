# Final-source compact size supplement

These are **32 ordinary Auto/empty artifacts in 16 matched build configurations** for resource source `304dc4c5dbf18be9c6e1eab7a62b5dabb98022ff6532c1e5ba4fe5e6df000add`. They measure the actual `compact` feature wiring. They do not replace the earlier README sizes, timing curves or hardcoded NW8 prototype. No timings or runtime-heap measurements were taken in this supplement.

Each artifact exports both borrowed encode and decode. The table gives **Auto minus its matching empty probe**, in bytes. Native code is executable-section size; Wasm CODE is a different section metric. These are linked deployment deltas, including reachable runtime/scaffold effects.

| Target and CPU flags | cache | Wide code / CODE B | Compact code / CODE B | Saved B | Stripped file delta, wide → compact B |
|---|---|---:|---:|---:|---:|
| M5 Mach-O, `apple-m4` | off | 60,388 | 60,388 | 0 | 82,752 → 82,752 |
| M5 Mach-O, `apple-m4` | on | 62,524 | 62,524 | 0 | 82,800 → 82,800 |
| Scalar Wasm, generic | off | 71,769 | 71,769 | 0 | 86,451 → 86,451 |
| Scalar Wasm, generic | on | 73,519 | 73,519 | 0 | 88,981 → 88,981 |
| Linux x86 ELF, generic | off | 286,445 | **191,005** | 95,440 | 327,792 → 232,128 |
| Linux x86 ELF, generic | on | 277,101 | **181,661** | 95,440 | 326,040 → 230,328 |
| Linux x86 ELF, `znver5` | off | 274,029 | **178,589** | 95,440 | 311,472 → 215,824 |
| Linux x86 ELF, `znver5` | on | 255,309 | **159,885** | 95,424 | 297,240 → 201,608 |

The x86 savings are about **93.2 KiB**, or 33.3–37.4% of these Auto-minus-empty code deltas. Baseline adjustment matters: the full generic/cache Auto artifact saves 95,424 code bytes while its empty probe grows 16 B. The full `znver5`/cache Auto artifact saves 95,440 B while its empty probe shrinks 16 B. [Raw artifacts](artifacts.csv) retain both values; [matched deltas](deltas.csv) retain the subtraction.

M5 and Wasm code/data section sizes and stripped lengths are unchanged by `compact` in these builds. A few gzip sizes differ: M5/cache's matched delta grows 5 B, and Wasm/default's grows 26 B. Compression differences and equal section lengths do not establish semantic equivalence or a semantic change. No gzip files are committed.

## Writable storage is separate from cache heap

For completeness, these are the signed Auto-minus-empty ELF storage deltas. Negative BSS is a subtraction between two real linked programs, each with nonnegative storage.

| x86 flags | cache | Readonly B, wide → compact | Initialized writable B, wide → compact | BSS B, wide → compact |
|---|---|---:|---:|---:|
| generic | off | 35,616 → 35,400 | 5,696 → 5,664 | −104 → −2,744 |
| generic | on | 36,276 → 36,060 | 12,616 → 12,584 | −258 → −2,850 |
| `znver5` | off | 31,844 → 31,628 | 5,616 → 5,584 | −176 → −2,832 |
| `znver5` | on | 30,080 → 29,864 | 11,848 → 11,816 | −368 → −3,040 |

ELF writable-section accounting includes relocation-only `.data.rel.ro`; linker placement and alignment also affect file/storage deltas. These values do not measure retained cache tables or imply a change to their runtime heap usage. Wasm DATA and initial linear-memory pages have their own CSV columns. Requested allocations, retained heap and process RSS remain separate measurements in the [resource survey](../README.md#heap-cache-rss-and-stack).

## Source, compiler and feature contracts

The [receipt](receipt.json) preserves the complete per-configuration compiler text, target cfg, profile, selected Cargo features, actual compiled-library feature fingerprints, producer hashes, lock hashes and artifact hashes. Its SHA-256 is `cfbf46b54765e772e4c292ad8cbf926cef7a58de8570c81b90b34f786db0d73d`.

- Resource fingerprint: `304dc4c5…000add`; code-size source hash: `d4de08ee…1a63`. These hash namespaces differ. [Final source metadata](../final-source-304dc.json) identifies the measured files.
- Rust 1.99.1 (`58cf8f9c35b355199727627d07ae34a4e2846d67`), LLVM 23.1.1; opt3, fat LTO, 16 codegen units, debug/incremental off, stripping after link.
- M5 size builds use `-C target-cpu=apple-m4`. The ordinary M5 timing curves use compiler-default target flags and an older source epoch. Wasm is scalar `wasm32-unknown-unknown`, with SIMD128 and allocation instrumentation off.
- Linux x86 builds ran in the local amd64 container under Rosetta, with generic flags or explicit `-C target-cpu=znver5`. They are code-generation results, not GCP/native timing. The emitted ELF comments identify actual LLD 23.1.1. M5 stripping used LLVM 23.1.4; Linux/Wasm stripping used the Rust LLVM-tools 23.1.1 build.
- Selected public features are only `cache` and/or `compact`. Actual library fingerprints include `default`, add `std` with `cache`, and contain no `__internals`. Both Auto and empty use the same selected features in each configuration.

The CSV column `profile_sha256` binds the **normalized full probe manifest**, not just its release-profile subsection. That displayed manifest uses `<frozen-source>`; its hash differs from `manifest_sha256`, which binds the retained original manifest. [Offline verification](verification.json) independently checked 64 retained raw/stripped file hashes, all 32 section/compression metrics, 16 compiled-library fingerprints and every matched delta. The copied receipt/CSVs remain byte-for-byte unchanged.

## Qualification and reproduction

The existing `scripts/code_size.py` performs silent wrapper/allocator smoke assertions before writing matching-host CSV rows. For M5 and translated x86 these check zeroed allocator lengths 0/1/32/65,537, the two-byte `00 01` ↔ `12` vector, short-capacity error returns, and prevalidated decode repeat counts 3/0. Whole build drivers exited successfully. There is no separate smoke receipt and no native x86 ISA/IFMA qualification claim from these tiny calls. Wasm is codegen-only in this supplement. Broader correctness/native checks have their own evidence.

Use the matching source/producer hashes, Rust 1.99.1 and the existing code-size tool with fresh output directories. For example:

```sh
python3 scripts/code_size.py --source . --output /tmp/base58-size-wide \
  --target x86_64-unknown-linux-gnu --cpu generic \
  --backends empty,auto --directions both --kinds cdylib \
  --strip-tool /path/to/llvm-strip
```

Repeat with a new output path and `--compact`; add `--cache` to both members of the cache pair. The other contracts select `--cpu znver5`, or the matching Apple/Wasm target and CPU flags. Preserve raw and stripped outputs and metadata. Final feature-wired GCP timings remain separately gated by the qualified observer controls; this static saving alone makes no speed claim.

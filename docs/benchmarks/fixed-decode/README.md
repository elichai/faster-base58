# Fixed-size decoding investigation

Keep the existing decoder. Two portable fixed-matrix prototypes were slower
and added code and tables on the measured Apple M5 Pro. No prototype was added
to the library. This does not rule out a future architecture-specific decoder.

The baseline is `f1a710f4e774f946e59a433c0b210bd8ae6a90db`, before the
[dispatch refactor](../dispatch/README.md). Rust 1.99.1 / LLVM 23.1.1,
`aarch64-apple-darwin` compiler defaults, opt-level 3, fat LTO and 16 codegen
units were used. The benchmark has neither `cache` nor `__internals` enabled.

## Contracts and method

A 32-byte value can encode to 32–44 characters; a 64-byte value to 64–88.
Text length alone does not establish decoded length. Numeric values at the
44/88-character boundary can require 33/65 bytes, and each leading `'1'`
represents a zero byte. An exact-array API must also reject a short result
instead of silently padding it.

The safe-Rust prototypes group four or five Base58 digits, multiply by const
power tables in base `2^32`, then carry. Their coefficients were generated
independently. Compile-time `u128` checks establish that each column plus carry
fits `u64`; [bounds](table-bounds.json) retain the exact values. Tables for both
sizes total 1,760 bytes for four-digit groups or 1,440 for five-digit groups.
They occupy read-only storage and require no heap cache.

## Timing and size

These are warm process-CPU times for the first deterministic random fixture.
Each case has three fresh AB/BA pairs and three batches of at least 10 ms per
process. Output buffers are reused; fixture setup and checks are outside timers.
Positive changes mean slower. The [complete table](timings.csv) includes all
72 cases, pair ranges, controls and the secondary elapsed clock.

| API | Decoded bytes | Group digits | Baseline ns | Prototype ns | Paired change |
| --- | ---: | ---: | ---: | ---: | ---: |
| Slice | 32 | 4 | **15.60** | 20.48 | +30.69% |
| Slice | 32 | 5 | **15.58** | 20.78 | +32.87% |
| Array | 32 | 4 | **21.27** | 22.75 | +6.97% |
| Array | 32 | 5 | **21.47** | 23.58 | +9.85% |
| Slice | 64 | 4 | **27.76** | 67.18 | +142.75% |
| Slice | 64 | 5 | **27.96** | 59.84 | +114.60% |
| Array | 64 | 4 | **30.86** | 68.18 | +120.96% |
| Array | 64 | 5 | **30.70** | 62.13 | +102.66% |

Other random, all-`0xff`, half-zero and one-nonzero-tail fixtures also regress.
All-zero early returns improve; that case does not require a fixed matrix.
Five8's corresponding valid conversions take 32.27/100.20 ns and bs58 takes
340.09/1,273.07 ns in this observer. Adapters and error contracts differ.
These values do not replace the broader public-mode comparison.

| Linked observer | Group digits | Added code B | Added read-only / Wasm DATA B | Added stripped file B |
| --- | ---: | ---: | ---: | ---: |
| M5 | 4 | 4,736 | 1,859 | 0 |
| M5 | 5 | 4,416 | 1,603 | 0 |
| Wasm | 4 | 7,412 | 1,783 | 9,196 |
| Wasm | 5 | 6,590 | 1,527 | 8,118 |

[All linked sizes](linked-sizes.csv) come from separate ordinary consumers.
Both exact-array paths and their conservative general fallback are reachable.
Mach-O padding hides section growth in the file total. Wasm was compiled for
size only. Neither candidate allocates on cold, repeated or checked error paths.
Requested heap counters do not measure stack or RSS.

Default and cache builds each pass 2,928 valid and 4,795 invalid-index fixtures,
including independent byte-Horner and bs58 oracles, leading zeros, overflow,
Unicode indices, short buffers and canaries. All 432 timing processes completed;
both clock summaries replay from raw events. No new Miri, Kani, fuzz, x86 or
Wasm execution claim is made for these rejected prototypes.

The [receipt](receipt.json) binds tables, artifacts, compiler and archived raw
inputs. Full prototype source and raw logs remain in the local research backup.
Five8 1.0.0 / five8_core 1.0.0 were inspected as [MIT-licensed references](../../../REFERENCES.md) at
revision `311279fb81461df36e730458a0463a6a322042d7`; code and tables were not copied.

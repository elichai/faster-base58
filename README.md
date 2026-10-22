# base58-bench

Benchmarks base58 (Bitcoin alphabet) decoders, plus faster decoders in `src/`:

- `decode_batched`: quadratic, but 14–17× (31–32 B) to 146× (32 KiB) faster than
  `bs58`. No allocation.
- `decode`: divide-and-conquer radix conversion with Karatsuba multiplication,
  O(n^1.585). It uses `decode_batched` up to about 3.7 KB of output, where that is as
  fast or faster.
- `decode_toom3`: `decode` with Toom-3 multiplication for products of at least 128
  limbs. 11% faster than `decode` at 32 KiB, 19% at 128 KiB, 30% at 512 KiB.
- `decode_ntt`, `decode_ntt_cached`, `decode_ntt_portable`: `decode` with
  number-theoretic-transform multiplication.
- `decode_malachite`: `decode` with malachite-nz's multiplication.
- `decode_matrix`, `decode_matrix_portable`: a matrix of powers with SIMD multiply-adds,
  for about 0.4 KiB to 4 KiB. The speedup was measured with NEON (aarch64). On AMD
  Zen 5 the AVX2 kernel was faster from about 1 KiB with default flags, but only tied
  `decode_batched` at 2–4 KiB with `-C target-cpu=native`. Elsewhere the plain-Rust
  kernels are slower than `decode_batched`.

All return the same bytes as `bs58`. On invalid input they report the same index,
the first invalid byte. If the input is invalid and the buffer is also too small for
it, which of the two errors is reported is unspecified: it can differ from `bs58` and
between decoders.

## Build configuration

- **Fat LTO** for release builds and `cargo bench`, set in `Cargo.toml`. The Apple M5
  Pro numbers were measured before the repo set it, with the same fat LTO from the
  author's `~/.cargo/config.toml`.
- **`-C target-cpu=apple-m4`** for every Apple M5 Pro number, also from that config.
  It is specific to the host, so the repo does not set it. Elsewhere, builds target the
  default CPU unless you pass `-C target-cpu`.
- **AMD Zen 5 numbers** (GCP `c4d-highcpu-4`, in the `decode_matrix` section) are
  release builds from before the repo set LTO: 16 codegen units, no LTO. The "default"
  columns target the default CPU, the "native" columns `-C target-cpu=native`
  (znver5). So a comparison between the two machines also compares LTO with no LTO.
- **LTO's effect depends on the machine.** From 4 KiB to 1 MiB:
  - Zen 5, `-C target-cpu=native`: fat LTO made `dc` 14–26% and `dc-toom3` 13–22%
    slower (1 MiB: 326 to 411 ms and 207 to 254 ms). `decode_batched`, the NTT
    decoders, `dc-malachite` and the matrix kernel moved by 5% or less. These are
    separate runs, one per build, on a shared VM, without pinned code alignment. On
    that VM a 16-byte shift of `decode_batched`, which made its hot loop cross a
    64-byte line, alone cost 24–31%, and 64-byte alignment removed it. So part or all
    of the LTO loss may be placement, not LTO.
  - M5 Pro, `-C target-cpu=apple-m4`: fat LTO took 0.94–1.04× the time of Cargo's
    default (`lto = false`, thin LTO inside each crate): `dc` 0.97–1.01, `dc-toom3`
    0.96–1.03, `ntt-cached` 0.98–1.03, `dc-malachite` 0.94–1.04. That is from
    back-to-back `cargo bench` runs, two rounds per build. Two sizes in one run had a
    load spike and are left out. So the M5 numbers would be about the same without fat
    LTO.
  - `lto = "off"` also drops the thin LTO inside each crate. On the M5 it made
    `dc-malachite` 1.5–1.8× slower and the others up to 11% slower, so it is not a
    fair "no LTO" baseline.
  - Not measured: fat LTO with the default x86 CPU.

## Layout

Each algorithm has its own file, so separate work does not touch the same code:

| file                | contents                                                        |
|---------------------|-----------------------------------------------------------------|
| `src/lib.rs`        | `Error`, module list, public re-exports                         |
| `src/common.rs`     | alphabet table, chunk parsing, validity check, error index (shared) |
| `src/batched.rs`    | `decode_batched`                                                |
| `src/dc.rs`         | `decode`: divide and conquer, power table, scratch bounds       |
| `src/mul.rs`        | Karatsuba and schoolbook multiplication                         |
| `src/toom3.rs`      | `decode_toom3`: Toom-3 multiplication on top of `mul.rs`        |
| `src/ntt/`          | `decode_ntt*`: NTT multiplication (tables, CRT, portable/NEON kernels) |
| `src/malachite.rs`  | `decode_malachite`: `dc.rs` with malachite-nz multiplication    |
| `src/matrix/`       | `decode_matrix*`: a matrix of powers (tables, portable/NEON/x86 kernels) |
| `src/tests.rs`      | every decoder against `bs58`                                    |
| `benches/decode.rs` | criterion benchmark of all decoders                             |

To add a decoder:
1. Put it in a new module and re-export it from `lib.rs`.
2. Add one line to `DECODERS` in `src/tests.rs`.
3. Add one line to `DECODERS` in `benches/decode.rs`.

Tests specific to an algorithm live in its own module.

## `decode_batched`: ten characters per 64-bit limb update

`bs58` updates the whole output once per character, one byte at a time:
`x ← x·58 + d` over `u8` limbs.

`decode_batched` parses ten characters into one `u64` chunk `c` (58^10 < 2^64). It then
runs `x ← x·58^10 + c` over `u64` limbs, with a 128-bit product per limb
(`u64::carrying_mul`). That is ten characters per pass and eight bytes per limb, so
80× fewer limb updates. It is still O(n²). With one carry chain, the measured speedup
at 32 KiB was 81×: a limb update cost about as much as a `bs58` byte update. Running
two chains at once (see [Two carry chains per loop](#two-carry-chains-per-loop))
raised it to 146×.

Details:

- **In place, no allocation.** The value is kept little-endian in `output`. The top
  limb stays in a register, so an exactly sized `output` is enough. At the end, the
  top limb's bytes are appended and the result is reversed to big-endian.
- **Validation is almost free.** Invalid bytes map to `0xFF` and valid digits are
  below `0x80`, so each chunk ORs its digits and tests one bit. Only on error does
  a rescan find the exact index.

## `decode`: divide and conquer

Split the chunk sequence into `hi` and `lo`, where `lo` is the last `e` chunks:

```
value(chunks) = value(hi) · 58^(10e) + value(lo)
```

and recurse on both halves. `decode` does this above `DC_THRESHOLD = 512` chunks.
Leaves of up to `LEAF = 256` chunks use the batched loop on `u64` limbs. With
Karatsuba multiplication, the cost is dominated by the top multiplications:
O(n^log2(3)). Measured time grows about 3.0× per doubling of size.

Design choices that mattered:

- **Balanced splits.** The split exponents are `unit · 2^i`, with
  `unit = ceil(n / 2^L) ≤ LEAF`. All powers come from one small power by
  repeated squaring, and every split is close to half. A first version split at
  the largest power of two below `n`. For 4470 chunks (32 KiB) that gives a
  374-chunk `hi` and a 4096-chunk `lo`, and computes a large power for one small
  product. Balancing took 32 KiB from 2.20 ms to 1.82 ms.
- **Trailing zero limbs in powers.** 58 = 2·29, so 58^k ends in k zero bits.
  Powers are stored without their low zero limbs plus a limb shift. This shortens
  the power operand of every product by about 1/6 (10 of every 58.6 bits).
- **Karatsuba** below `KARATSUBA_THRESHOLD = 32` limbs falls back to schoolbook
  (`u64::carrying_mul_add`, two rows per pass). Unbalanced products are cut into
  balanced blocks.
- **Checks before any work.** The first digit is not `'1'`, so the value is at least
  `58^(d−1)` for `d` digits, which takes more than `(d − 1) · 0.732` bytes. A shorter
  output fails with `BufferTooSmall`, then one vectorized pass checks every character.
  Only then does `decode` build the power table or allocate. Before this, a 1-byte buffer
  or a bad last character cost a full decode: 45 ms for 400k characters into 1 byte,
  and 3.7 s building powers for 24.7M invalid characters. The pass costs about 0.075 ns
  per character: 1–2.5% of a decode at 4 KiB (measured), under 1% from 8 KiB.
- **One allocation.** All memory is sized from the chunk count `n` before any work.
  The result (`n` limbs), the power table (about `0.77n`) and all scratch (about `3n`)
  come from one buffer. The power index is a fixed array on the stack. The table
  bound is within about 1% of the actual size: `58^10 < 2^59`, and `58^(10e)` ends in
  exactly `10e` zero bits. Each scratch bound is proved by induction in its doc
  comment. The tests run `mul` with exactly the documented scratch length, pre-filled
  with garbage.

Thresholds were re-tuned on an Apple M5 Pro after the two-chain kernels, with fat LTO
and `-C target-cpu=apple-m4` (see [Build configuration](#build-configuration)). A build
without LTO or for another CPU may favour other values:
- **Karatsuba:** 32/40/48/64 tried; 32 is best by 1.5–2.5%.
- **Leaf size:** 128/192/256/384 tried; 128 and 256 are within 1%.
- **`DC_THRESHOLD`:** 512 chunks (about 3.7 KB decoded), where `decode_batched` and
  divide and conquer tie.

## `decode_toom3`: Toom-3 multiplication

`dc.rs`'s `decode_with` takes the multiplication as a parameter. `decode_toom3` passes
`toom3::mul` instead of `mul::mul`, so `dc` and `dc-toom3` differ only in how they
multiply.

Toom-3 cuts each operand into three parts, `a = a2·X² + a1·X + a0` with `X = 2^(64k)`
and `k = ⌈n/3⌉`. It evaluates the product polynomial `r` at 0, 1, −1, 2 and ∞. That is
five products of at most `k + 1` limbs, against Karatsuba's three of about `n/2`: O(n^1.465)
instead of O(n^1.585).

- **No signed numbers.** Only `r(−1)` can be negative. It is kept as a magnitude and a
  sign. The interpolation order keeps every intermediate value a sum of non-negative
  coefficients of `r`, so no subtraction wraps. `debug_assert!`s check every carry and
  borrow.
- **Exact division by 3** multiplies each limb by 3⁻¹ mod 2^64, from the low end.
- **Scratch.** A step uses `6k + 6` limbs. The operands at 1 stay in the unused middle
  of `out` until `r(0)` and `r(∞)` are written there. This fits in `mul::scratch_len`
  by induction, so `dc`'s buffer does not grow. A test checks the induction step for
  every `n` up to 2^20 limbs.
- **Shapes.** Below 128 limbs, `toom3::mul` calls `mul::mul`. If the shorter operand
  has at most `2k` limbs, it cuts the longer one into blocks, as `mul::mul` does.

Tuning: Toom-3 for one level against Karatsuba for one level gives no clean crossover.
The winner alternates between 64 and 384 limbs, because the first step changes the
size at which the recursion reaches schoolbook. Toom-3 also does about `12n` more limb
operations in linear passes per step. So the threshold was chosen on whole decodes,
with Toom-3 at every level from the threshold up (criterion medians, M5 Pro, with
other work running, so several percent of noise):

| decoded bytes | Karatsuba | T = 96  | T = 128 | T = 192 | T = 256 |
|--------------:|----------:|--------:|--------:|--------:|--------:|
| 4 KiB         | 69.2 µs   | 68.7 µs | 66.0 µs | 67.8 µs | 68.3 µs |
| 8 KiB         | 199 µs    | 204 µs  | 203 µs  | 205 µs  | 211 µs  |
| 16 KiB        | 598 µs    | 564 µs  | 544 µs  | 576 µs  | 570 µs  |
| 32 KiB        | 1.79 ms   | 1.65 ms | 1.59 ms | 1.66 ms | 1.63 ms |
| 128 KiB       | 16.7 ms   | 13.4 ms | 13.0 ms | 13.6 ms | 13.4 ms |

`cargo bench`, `dc` and `dc-toom3` in the same run:

| decoded bytes | dc      | dc-toom3 | change |
|--------------:|--------:|---------:|-------:|
| 31 – 1024     |         |          | same code (`decode_batched`) |
| 32768         | 1.76 ms | 1.53 ms  | −13%   |
| 32769         | 1.78 ms | 1.55 ms  | −13%   |

Other sizes, from the tuning harness with the final code:

| decoded bytes | dc      | dc-toom3 | change |
|--------------:|--------:|---------:|-------:|
| 4 KiB         | 62.2 µs | 64.3 µs  | +3% (noise level) |
| 8 KiB         | 191 µs  | 186 µs   | −2%    |
| 16 KiB        | 584 µs  | 536 µs   | −8%    |
| 32 KiB        | 1.78 ms | 1.60 ms  | −10%   |
| 128 KiB       | 16.6 ms | 12.9 ms  | −22%   |

These tables predate the two-chain kernels below. With those kernels, Toom-3
thresholds of 96, 128 and 192 are within 1% of each other, so 128 stays. Current
numbers are in [Results](#results).

## Two carry chains per loop

Each inner loop is a serial carry chain: every limb waits for the previous limb's
carry. This covers the batched pass, the D&C leaves and the schoolbook rows. The CPU
can run independent chains at the same time, but only if they are close together in
the instruction stream. Running `hi` and `lo` one after the other is too far apart.
So each loop now runs two chains, the second one step behind the first:

- **`decode_batched` and the D&C leaves** take chunks in pairs. The second pass
  updates limb `i − 1` right after the first pass updates limb `i`. Each limb is
  loaded and stored once per pair.
- **Schoolbook multiplication** takes rows in pairs. Row `i + 1` adds into column
  `j − 1` right after row `i` adds into column `j`.

A standalone microbenchmark of the quadratic kernel, in ns per limb update over 16 to
4096 limbs. This and the A/B below ran on the Apple M5 Pro with fat LTO and
`-C target-cpu=apple-m4` (see [Build configuration](#build-configuration)):

| one chain | `hi` and `lo` in one loop | two passes over one number | ×58^20, one chain | four chains |
|----------:|--------------------------:|---------------------------:|------------------:|------------:|
| 0.47–0.50 | 0.21–0.25                 | 0.22–0.26                  | 0.30–0.37         | 0.18–0.19   |

- **Pairing passes is as good as pairing `hi` with `lo`.** It runs at the same speed
  and needs no change to the recursion.
- **Multiplying by 58^20** halves the passes but keeps one chain, so it gains less.
- **Four chains** would be about 1.3× faster again, with more complex tail handling.
  That is not done.
- **Schoolbook with two rows per pass** takes 0.35–0.39 ns per limb product,
  against 0.47–0.70 ns with one.

The result, timed against the previous version in the same binary, includes the
re-tuned thresholds. New time over old:

| decoded bytes | batched | dc   | dc-toom3 |
|--------------:|--------:|-----:|---------:|
| 8             | 0.95    |      |          |
| 32            | 0.83    |      |          |
| 128           | 0.73    |      |          |
| 1 KiB         | 0.58    |      |          |
| 2 KiB         | 0.57    | 0.53 | 0.52     |
| 4 KiB         | 0.55    | 0.66 | 0.64     |
| 8 KiB         | 0.54    | 0.69 | 0.69     |
| 32 KiB        | 0.55    | 0.72 | 0.73     |
| 128 KiB       | –       | 0.74 | 0.78     |

`dc` gains most at 2 KiB because it now runs `decode_batched` there.

## `decode_ntt`: number-theoretic-transform multiplication

`decode_ntt`, `decode_ntt_cached` and `decode_ntt_portable` run the same
divide-and-conquer as `decode`. The difference is that they multiply products of at
least `THRESHOLD` limbs with NTTs instead of Karatsuba.

- **Three primes below 2^30** (998244353, 897581057, 880803841), each `1 mod 2^23`.
  Every limb gives two 32-bit coefficients. A product coefficient is below
  2^22 · (2^32 − 1)^2 < 2^86 < q1·q2·q3 ≈ 2^89.35, so Garner's CRT recovers it
  exactly.
- **Signed Montgomery arithmetic in `i32` lanes.** On NEON this is `sqdmulh` + `vhsub`
  per product and `sqdmulh` + `srshr` + `mls` per reduction. Every stored value stays
  in (−q, q). The NEON kernels give the same bits as the plain-Rust ones, and the tests
  check that.
- **Backends.** NEON on aarch64. On x86_64, the plain-Rust kernels are compiled for
  AVX-512 or AVX2 and chosen at runtime. Under Rosetta the tests pass with the AVX2
  kernels (`ROSETTA_ADVERTISE_AVX=1`, release build, where they use 256-bit vectors).
  The AVX-512 kernels compile but have not run. Rosetta translates x86 code, so real
  x86 hardware is still the better test.
- **Twiddle tables.** `decode_ntt` builds them on every call; `decode_ntt_cached` keeps
  them in a process-wide cache. `decode_ntt_portable` uses the cache with the
  plain-Rust kernels. On aarch64 LLVM vectorizes those with NEON itself, so it compares
  hand-written NEON with compiler-generated NEON, not SIMD with scalar code.
- **Memory.** The tables take 48·L bytes for L-point transforms, sized for the longest
  product of the decode. That is 1.5 MiB at 128 KiB decoded and 12 MiB at 1 MiB; in
  general 11–22× the output size, depending on where that product falls between
  powers of two. A smaller table is a prefix of a larger one, so `decode_ntt_cached`
  serves a request from any larger table it already holds, and builds a size only when
  it holds none at least that large. It keeps what it builds until the process exits:
  after a 16 MiB decode, 192 MiB, and up to about twice that if smaller sizes came
  first. The NTT scratch is 16 limbs per operand limb. The stored transforms of the
  powers ([Prepared powers](#prepared-powers)) add less than 12·L bytes per decode
  while the longest product fits the tables. Above about 34 MiB decoded, where the
  tables stop at 2^23 points, they can reach up to 24·L bytes.
- **Threshold.** The sweep over 64, 128, 256 and 512 limbs chose `THRESHOLD = 128`.
  64 was within noise at 32 and 128 KiB but 3.7% slower at 1 MiB; 512 was 19% slower
  at 32 KiB. Products with a prepared power use transforms from
  `PREPARED_THRESHOLD = 96` limbs.

Apple M5 Pro, rustc 1.99.0, `cargo bench` (criterion medians from one run). Like every
M5 number in this section, these were built with fat LTO and `-C target-cpu=apple-m4`
(see [Build configuration](#build-configuration)). 36864, 49152 and 147456
bytes need products just above a power of two; see
[Power-of-two padding](#power-of-two-padding).

| decoded bytes | dc      | dc-toom3 | ntt     | ntt-cached | ntt-portable |
|--------------:|--------:|---------:|--------:|-----------:|-------------:|
| 32768         | 1.24 ms | 1.11 ms  | 522 µs  | 487 µs     | 796 µs       |
| 36864         | 1.50 ms | 1.33 ms  | 644 µs  | 594 µs     | 946 µs       |
| 49152         | 2.43 ms | 2.09 ms  | 868 µs  | 815 µs     | 1.30 ms      |
| 131072        | 11.9 ms | 9.45 ms  | 2.61 ms | 2.48 ms    | 4.23 ms      |
| 147456        | 14.1 ms | 11.2 ms  | 3.19 ms | 3.01 ms    | 4.99 ms      |
| 1048576       | 327 ms  | 217 ms   | 28.9 ms | 28.4 ms    | 47.7 ms      |

Building the tables on every call (`ntt`) costs 2–8% over `ntt-cached` at these
sizes, about as much as the noise between runs.

These runs predate [Scratch alignment](#scratch-alignment), which made `ntt-cached`
4.5%, 3.4% and 2.3% faster at 32, 36 and 48 KiB in a same-binary A/B, and left the
other sizes of the table unchanged. Criterion was not run again.

Each level of the divide-and-conquer costs about one top-level product, so with
O(n log n) products the decode is O(n log² n).

### Schönhage–Strassen: measured, not used

A throwaway probe (not in the repo) timed GMP 6.3's Schönhage–Strassen
(`mpn_nussbaumer_mul`) against `ntt::mul_with` in one binary: n × n limbs, random
operands, every product checked against `mpn_mul_n`, best of repeated calls, one run on
the M5 Pro. The top product of an X-byte decode has operands of about X/16 limbs. The
`ntt` column is the code before the padding change below, so the 3072 and 12288 rows
show the NTT at its worst padding. That change makes the NTT faster at such sizes and
leaves GMP as it is, so it can only widen the gap.

| limbs                 | ntt     | GMP SSA | SSA / ntt |
|----------------------:|--------:|--------:|----------:|
| 2048 (32 KiB decoded) | 84.5 µs | 125 µs  | 1.48      |
| 3072                  | 176 µs  | 198 µs  | 1.12      |
| 4096                  | 180 µs  | 256 µs  | 1.42      |
| 8192 (128 KiB)        | 399 µs  | 581 µs  | 1.46      |
| 12288                 | 832 µs  | 988 µs  | 1.19      |
| 32768                 | 1.83 ms | 3.24 ms | 1.77      |
| 65536 (1 MiB)         | 3.88 ms | 7.72 ms | 1.99      |
| 262144                | 17.5 ms | 42.3 ms | 2.42      |

- **Why SSA loses.** Its butterflies add, subtract and shift numbers of many limbs:
  serial carry chains in scalar registers. Most of its time goes to 2^k pointwise
  products of numbers of about √(2N) bits. Here those would run on `mul.rs`, which is
  about 2× slower than GMP at 128–256 limbs, so an SSA in this crate would likely be
  slower than GMP's.
- **Asymptotics.** Up to 2^23 points the NTT is O(n log n) and SSA is
  O(n log n log log n). The ratio grows with n.
- **What SSA would add:** no twiddle tables and no length cap. Both have cheaper fixes
  in the NTT: twiddles computed on the fly, and a fourth prime.
- **Caveats.** Homebrew's GMP is built for generic `armv8-a`, so its thresholds are not
  tuned for this CPU. GMP's SSA has no √2 trick and no truncated FFT, so a stronger SSA
  could do better.

### Power-of-two padding

A cyclic transform of `L` points multiplies modulo `x^L − 1`, so a product of `P`
coefficients needs `L ≥ P`, a power of two. The top product of a decode is about 0.92×
the output limbs, and every level of the divide and conquer has about the same fill. So
the 32 KiB, 128 KiB and 1 MiB benchmark sizes use about 92% of their transforms, and
sizes just above them about 52%. With one cyclic transform per product, `ntt-cached`
grew 63% from 32 to 36 KiB and 76% from 128 to 144 KiB, against 20% for `dc-toom3`.

So `transform` computes a product modulo up to three coprime polynomials whose degrees
add up to about `P` (`Plan` in `src/ntt/mod.rs`). With `p = P` rounded up to a multiple
of 8 and `t` the largest power of two not above `p`:

| `p`         | moduli                                      | points                |
|-------------|---------------------------------------------|-----------------------|
| `= t`       | `x^t − 1`                                   | `t`                   |
| `≤ 3t/2`    | `x^t + 1`, `x^c − 1`                        | `t + c`, `c < t`      |
| `≤ 7t/4`    | `x^t + 1`, `x^(t/2) + 1`, `x^c − 1`         | `1.5 t + c`, `c < t/2` |
| otherwise   | `x^(2t) − 1`                                | `2t`                  |

`c` is the smallest power of two (at least 64, see
[Small cyclic pieces](#small-cyclic-pieces)) that covers the rest of `p`. The
transforms then fill about 5/6 of their points at worst, and usually more than 90%.

- **Negacyclic pieces.** Modulo `x^m + 1` is an `m`-point cyclic transform after
  coefficient `i` is multiplied by `w^i`, for `w` a primitive `2m`-th root of unity.
  Those powers are the twiddles of the top stage of a `2m`-point transform, so the
  existing tables serve them and their size does not change. The multiplication is
  fused into the load. An operand longer than `m` coefficients is first folded with
  alternating signs.
- **Coprime moduli.** `x^m + 1` vanishes exactly at the elements of order `2m`, and
  `x^c − 1` at elements of order at most `c < m`.
- **Polynomial CRT** (`crt`). Let `m` be the largest piece and `d` the total of the
  others, and write the product as `lo + x^d·mid + x^m·hi`. Modulo `x^m + 1` it is
  `lo − hi`, then `mid`. Modulo each smaller piece `x^m = 1`, so it is
  `lo + hi + x^d·mid`. Subtracting `x^d·mid` from the smaller residues leaves those of
  `lo + hi`, which the same steps rebuild. Then `lo` and `hi` are half the sum and the
  difference of `lo + hi` and `lo − hi`. All of it is linear passes, exact modulo each
  prime. For two pieces, one fused kernel undoes the twist, subtracts and halves.
- **Exactness and scratch.** A product still has at most `L ≤ 2^23` coefficients, so
  the Garner bound above holds as before. The pieces of total length `w < L` need
  `3w + L/2 < 3.5L` `i32`s of scratch, inside the cyclic path's `4L`.
- **At most three pieces.** A first version rounded `p` to three set bits at every size.
  It added small third pieces where two already fit, and was 12% slower at 36 KiB. Each
  piece costs passes over the largest one: folding the operands, undoing the twist,
  subtracting. Each pass costs roughly one butterfly stage. A third piece is
  used only where two would fall back to `2t`. Plain-Rust instead of NEON versions of
  the new linear kernels cost 7% at 36 KiB.

`ntt-cached` before and after this change alone (before NTT squares), in one binary
with interleaved runs, best of repeated runs (M5 Pro; probe outside the repo):

| decoded bytes | before  | after   | change | dc-toom3 |
|--------------:|--------:|--------:|-------:|---------:|
| 32 KiB        | 611 µs  | 590 µs  | −3%    | 1.16 ms  |
| 36 KiB        | 997 µs  | 684 µs  | −31%   | 1.39 ms  |
| 40 KiB        | 1.22 ms | 825 µs  | −32%   | 1.69 ms  |
| 48 KiB        | 1.27 ms | 986 µs  | −23%   | 2.21 ms  |
| 56 KiB        | 1.37 ms | 1.21 ms | −11%   | 2.78 ms  |
| 64 KiB        | 1.46 ms | 1.43 ms | −2%    | 3.44 ms  |
| 72 KiB        | 2.43 ms | 1.62 ms | −33%   | 4.11 ms  |
| 96 KiB        | 3.03 ms | 2.30 ms | −24%   | 6.34 ms  |
| 108 KiB       | 3.14 ms | 2.65 ms | −16%   | 7.76 ms  |
| 128 KiB       | 3.39 ms | 3.31 ms | −2%    | 9.88 ms  |
| 144 KiB       | 5.95 ms | 3.87 ms | −35%   | 11.8 ms  |
| 1 MiB         | 42.3 ms | 40.9 ms | −3%    |          |

From 32 to 36 KiB the decode now grows 16%, and from 128 to 144 KiB 17%. `dc-toom3`
grows 20% over both steps. Sizes that already filled their transforms gained 2–3%,
probably because the power table's squares (about 84% of a power of two) now use three
pieces.

Tried and not kept:

- **Choosing `dc`'s unit per decode** so that products fill their transforms better.
  Measured in a copy of the crate with the unit forced at run time (a probe, not in the
  repo). At the default number of levels, `dc` accepts units from the default `u0` up to
  `min(256, 2·u0 − 1)`. A unit `u ≤ 128` with one more level gives the same tree as
  `2u`, because nodes of at most 256 chunks are leaves.
  - **Cost model.** Each operation of the tree costs a linear formula: per transform
    piece of `m` points, `m·log2 m` per transform, passes per piece and a fixed cost;
    plus the product length. Karatsuba costs `b·a^0.585`, schoolbook `a·b`, a leaf `m²`.
    Fitted to about 5,200 operations timed alone: median error 2–3%, 90th percentile
    4.5–6.5%. Summed over the tree, plus a constant per size, it is within about 2% of
    the measured decode for 90% of the units from 32 to 144 KiB.
  - **Result.** Measured after the [scratch alignment](#scratch-alignment) fix. The
    model's unit against the default in one binary. Each size has three arms: the
    default, the model's unit, and the default again (an A/A control). An arm is a block
    of about 50 ms of decodes, timed as the best decode of the block (one decode at
    1 MiB). The order of the arms rotates every round. The table gives the median of 31
    per-round ratios. A second process with 15 rounds agreed within 0.8 points at every
    size. The A/A medians were 0.995–1.006. The cost of the search is not included.
    "Best tried": a scan timed every unit that `dc` accepts (best of 21 interleaved
    decodes, 7 at 1 MiB). Then the four or five fastest ran against the default as above,
    with 21 rounds (41 at 1 MiB). `L + 1` marks one more level than the default. Where
    the best tried is the model's unit, the two changes come from different runs and
    differ by up to 0.5 points.

    | decoded bytes | default unit | model's unit | change | best tried | change | 64-pt rest |
    |--------------:|-------------:|-------------:|-------:|-----------:|-------:|-----------:|
    | 32 KiB        | 140          | 150          | −1.2%  | 150        | −0.7%  | −0.7%      |
    | 36 KiB        | 158          | 162          | −0.6%  | 162        | −0.6%  | −0.8%      |
    | 44 KiB        | 193          | 210          | −4.0%  |            |        | −4.0%      |
    | 48 KiB        | 210          | 227          | −0.3%  | 113, L + 1 | −0.4%  | −0.2%      |
    | 88 KiB        | 193          | 228          | −5.5%  |            |        | −4.8%      |
    | 128 KiB       | 140          | 151          | −2.2%  | 150        | −2.4%  | −2.1%      |
    | 140 KiB       | 153          | 154          | −7.4%  |            |        | 162: −2.2% |
    | 144 KiB       | 158          | 162          | −0.7%  | 84, L + 1  | −1.0%  | −0.8%      |
    | 1 MiB         | 140          | 151          | −2.7%  | 151        | −2.8%  | −2.9%      |

    Over 34 sizes (32 to 160 KiB in steps of 4 KiB, and 1 MiB) the model keeps the
    default at 4. At the other 30 its unit was faster in both runs, except at 68 KiB:
    −0.3% and +0.2%, inside the A/A spread. The geometric mean over the 34 was −1.4% in
    both runs. Both runs gained 2–8% at 44, 60, 88, 120, 124, 140 and 160 KiB and at
    1 MiB. At the six sizes scanned, the model's unit was within 1% of the best unit
    tried. The same A/B before the alignment fix gave −1.2%, with 64, 112, 116, 128 and
    152 KiB and 1 MiB 1.5–3.6% slower.

    "64-pt rest": the same A/B again after the smallest cyclic piece became 64 points
    ([Small cyclic pieces](#small-cyclic-pieces)), 31 rounds; a second process with 15
    rounds agreed within 0.6 points at every size. The model picks the same unit as
    before at every size except 140 KiB, where it now picks 162. The geometric mean over
    the 34 sizes was −1.2% in both runs (it was −1.4%). All 30 sizes with a changed unit
    were faster in both runs, 68 KiB included; the A/A medians were 0.997–1.005. So
    about 0.2 points of the gain came from avoiding small pieces, nearly all at 140 KiB.
  - **Why the model misses: products are slower inside a decode.** Now about equally
    for every unit. At 144 KiB the top product takes about 400 µs inside the decode with
    units 158, 159 and 162, and about 285 µs alone. Before the fix it took 402 µs with
    158 and 440 µs with 159 or 163. At 1 MiB all multiplier calls together take 3.1%
    (unit 140) and 3.5% (unit 151) longer inside than alone; before the fix, about 0
    and 10–15%. A probable cause is the cache: timed alone, the prepared power and the
    operands are warm. Not checked.
  - **Why the model missed: small cyclic pieces were slow.** Fixed since; see
    [Small cyclic pieces](#small-cyclic-pieces). At 140 KiB the model expected −1.1%,
    and the A/B measured −7.4%. With unit 153 most products of the second and third
    levels are 8,193 to 8,199 and 4,100 limbs, just above a power of two, so `Plan`
    added an 8- or 16-point cyclic piece to a 16,384- or 8,192-point negacyclic one. The
    top product, 16,391 limbs, got a 16-point piece. With unit 154 the top product,
    16,381 limbs, fits one cyclic transform, and the products of the next two levels get
    pieces of 64 or 128 points, or none. The model charges each piece by its own length,
    so a small piece looked almost free.
  - **A ±3.5% effect that was not the split: scratch alignment.** Before the fix, at
    36 KiB, unit 79 with one more level was 3.6% faster than the default 158. It makes
    the same multiplier calls, plus a 61-limb power and its square. At 72 KiB 79 was
    3.1% faster, at 144 KiB 3.3% slower. The cause was the address of the NTT's scratch,
    which follows the power table in one allocation: the tables of 79 and 158 differ by
    43 limbs, an odd number, so one of the two put the scratch 8 bytes off a 16-byte
    boundary (see [Scratch alignment](#scratch-alignment)). With the fix, 79 against
    158 is −0.2% at 36 KiB and −0.3% at 144 KiB, and one limb of padding before the
    scratch changes the decode by 0.3% or less (it was 3.4%).

  Not built. The model's unit now gains reliably, but the search as probed costs more
  than it saves. It walks every node of the tree for each unit it tries: 46 µs at
  32 KiB (10% of the decode), 166 µs at 128 KiB (7%) and 1.4 ms at 1 MiB (5%). What a
  real implementation needs is in [Possible next steps](#possible-next-steps).
- **A truncated FFT.** The pieces are a coarse truncated FFT. On the same points they
  do fewer butterflies than one, because each piece's transform is shorter than `L`.
  After them a truncated FFT could save at most about 17% on the worst products and
  less on average, before its own overhead. It would also need a new inverse kernel in
  NEON. Modelled, not built.
- **Mixed radix `3·2^k`.** It needs new primes: of the three, only q3 = 105·2^23 + 1 is
  1 mod 3·2^23. Not tried.

#### Small cyclic pieces

The cyclic piece of a plan has at least 64 points. It used to go down to 8. A product a
few limbs above a power of two then got an 8- or 16-point piece next to a negacyclic
piece of thousands of points, and took up to 12% longer than with a 64-point piece,
although the larger piece does more work. The change affects only products whose rest
is at most 32 coefficients.

Two linear passes cost one step per block of the small piece, and every step updates
the same `c` sums:

- **Folding.** `fold` reduces an operand modulo `x^c − 1` by adding it to the piece in
  blocks of `c/2` limbs, one `load_add` call per block. Each call loads the sums that
  the previous call stored, adds, reduces and stores them. For `c = 8`, an operand of
  4,500 limbs takes about 1,100 calls of two vectors each.
- **Recombining.** `pair` subtracts each `c`-point block of the negacyclic residue from
  the same `c` sums: 2,048 dependent steps for `c = 8` next to a 16,384-point piece.
  With three pieces, `fold_sub` in `crt` does the same.

So both passes run at the latency of one load–add–reduce–store step, not at the
throughput of the vector unit. Timed alone, one prime, next to a 16,384-point
negacyclic piece (best of 200 runs):

| `c` | fold, 4,508 limbs | fold, 3,688 limbs | pair   |
|----:|------------------:|------------------:|-------:|
| 8   | 3.1 µs            | 2.5 µs            | 5.2 µs |
| 16  | 1.7 µs            | 1.3 µs            | 2.7 µs |
| 32  | 1.3 µs            | 1.1 µs            | 2.7 µs |
| 64  | 1.3 µs            | 1.0 µs            | 2.3 µs |
| 128 | 1.3 µs            | 1.0 µs            | 2.1 µs |

With each phase timed inside a 4,508 × 3,688-limb product (three primes, mean of 200
runs), folding and recombining took 26 µs more with `c = 8` than with `c = 64`: 21 µs
against 7 µs for folding, 20 µs against 7 µs for `pair`. The negacyclic transforms took
3–5 µs more too, although their length and work did not change. Timed per prime in an
8,200-limb product (16-point rest), all of that was for the second prime. The cause
there is where the scratch layout puts that prime's residues, which moves with the
total length of the pieces. With unused `i32`s between the primes' residues (plain
product, mean of 300 runs), the second prime's negacyclic product took 42.8 µs with a
16-point rest and none, 39.1 µs with 48; with a 64-point rest 38.9 µs with none, and
36.5 µs with 976, as fast as for the other two primes. Why those addresses are slow, and
why decodes do not meet them: [Residue placement](#residue-placement-measured-not-changed).

Products alone, `2^13` limbs plus a few, with the smallest piece as a run-time knob in a
copy of the crate (a probe outside the repo). Operands of 0.55 and 0.45 of the length,
best of 60 runs, the four settings back to back, M5 Pro:

| limbs | rest | smallest 8 (before) | 32     | 64 (after) | 128    |
|------:|-----:|--------------------:|-------:|-----------:|-------:|
| 8,192 | none | 182 µs              | 182 µs | 182 µs     | 182 µs |
| 8,196 | 8    | 220 µs              | 200 µs | 199 µs     | 199 µs |
| 8,200 | 16   | 203 µs              | 200 µs | 199 µs     | 199 µs |
| 8,208 | 32   | 200 µs              | 200 µs | 199 µs     | 199 µs |
| 8,224 | 64   | 199 µs              | 199 µs | 199 µs     | 198 µs |

With an 8-point rest (`2^k + 4` limbs), 64 points saved 8–10% for `k` = 9 to 14, and
8–11% with a prepared power. 32 points saved almost as much; 64 was 0.1–1.2% faster again
for rests of 8 to 32 at `k` = 12 to 14. 128 was within 0.4% of 64. At `k` = 8
(512-point pieces, products of 3.5–5 µs) the settings differed by at most 0.1 µs either
way, near the timer's 0.04 µs resolution. The 145 × 121-limb products of 36, 72 and
144 KiB are of that size; their rest goes from 32 to 64 points.

Decodes: `ntt-cached` with the smallest piece 8 (before), 32, 64 (after) and 8 again (an
A/A control), in one binary. An arm is a block of about 50 ms of decodes, timed as the
best decode of the block; the order of the arms rotates every round. Changes are the
median of the per-round ratios to the first arm. Run 1: 31 rounds (62 at 1 MiB); run 2:
21 rounds (42 at 1 MiB), over the 34 sizes of the unit choice above. "Changed" counts the
multiplier calls whose plan changes, of all multiplier calls of the decode.

| decoded bytes | changed      | before  | after   | run 1  | run 2  |
|--------------:|-------------:|--------:|--------:|-------:|-------:|
| 32 KiB        | 0 of 39      | 473 µs  | 473 µs  | 0.0%   | 0.0%   |
| 36 KiB        | 16 of 39     | 574 µs  | 575 µs  | +0.1%  | +0.1%  |
| 128 KiB       | 0 of 139     | 2.49 ms | 2.49 ms | +0.1%  | 0.0%   |
| 140 KiB       | 122 of 139   | 3.16 ms | 2.94 ms | −7.6%  | −7.8%  |
| 144 KiB       | 63 of 138    | 3.01 ms | 3.01 ms | +0.2%  | +0.1%  |
| 1 MiB         | 0 of 1,040   | 28.3 ms | 28.3 ms | −0.1%  | −0.3%  |

The A/A medians were 0.996–1.000 in run 1 and 0.998–1.003 in run 2. 15 of the 34 sizes
have changed calls. At 140 KiB the default unit 153 gives an 8-point piece to 120 of the
139 calls, at every level from 512 to 16,384 points, and a 16-point piece to two,
among them the top product. Every other size was between −0.3% and +0.2%, inside the
A/A spread; the geometric mean over the 34 was −0.25%, all of it from 140 KiB. 32
points gave the same within 0.25 points at every size except 1 MiB (−0.6%, with no
changed call: noise).

### Squares

`dc` squares each power by passing one slice twice. `transform` and `pieces` see this
(`ptr::eq`). They load and transform the operand once per prime instead of twice, then
square pointwise (the `square` kernel). In `split`, which a decode never reaches, the
two diagonal products of halves are squares too.

Before prepared powers, the power table took about 9–12% of an `ntt-cached` decode (a
probe timing `dc::powers()`). A square skips one load and one of the three transforms
per prime, so the expected gain was about 3–4%. Measured on the Apple M5 Pro (NEON
kernels, fat LTO and `-C target-cpu=apple-m4`, see
[Build configuration](#build-configuration); not measured on x86-64), in one binary
against the same code with the square path turned off, with
[prepared powers](#prepared-powers) and the [lazy inverse](#lazy-inverse) in place (a
probe, not in the repo: 41 interleaved rounds of about 100 ms per size, median of the
per-round ratios, two runs, times from the second):

| decoded bytes | squares | no squares | change, run 1 | change, run 2 |
|--------------:|--------:|-----------:|--------------:|--------------:|
| 32 KiB        | 484 µs  | 504 µs     | −3.3%         | −3.7%         |
| 36 KiB        | 586 µs  | 606 µs     | −3.3%         | −3.2%         |
| 48 KiB        | 805 µs  | 833 µs     | −3.4%         | −3.3%         |
| 128 KiB       | 2.47 ms | 2.55 ms    | −3.5%         | −3.4%         |
| 144 KiB       | 2.98 ms | 3.07 ms    | −3.1%         | −3.0%         |
| 1 MiB         | 28.0 ms | 29.0 ms    | −2.9%         | −3.0%         |

In both runs the interquartile range of the per-round ratios is below 1 at every size.
An A/A control, both arms squaring, gave ratios of 0.999–1.002. Before prepared powers
and the lazy inverse, the same probe gave −2.3% to −3.5%: those changes make the other
products cheaper, so the squares are a larger share. Two pairs of `cargo bench` runs (before, then after) could not resolve this,
because other benchmarks shared the machine. In the quieter pair `ntt-cached` came out
4–14% faster, but `dc-malachite`, whose code did not change, also moved by 1–10%.

### Prepared powers

All products at one level of the divide and conquer multiply by the same power, and
every power but the largest is used at least twice. So `dc` hands each power but the
last to `Multiplier::prepare_power`, and multiplies by it with `mul_power`. `Ntt` keeps
one cyclic forward transform of each power of at least 96 limbs, at the length of the
longest product it will take (`Prepared` in `src/ntt/mod.rs`). A product then
transforms only `hi`: one forward transform per prime instead of two.

- **One transform serves every plan.** In bit-reversed order, the first `d` values of
  the stored transform are the `d`-point transform of the power modulo `x^d − 1`, and
  values `d..2d` the twisted one modulo `x^d + 1`: the frequencies that are multiples of
  `L/d`, and the odd multiples of `L/(2d)`. So every piece of a `Plan` of at most `L`
  points is a slice of it, and a shorter product takes a prefix. A test checks this
  against folding and transforming directly, for every piece size.
- **Cost.** One forward transform per prepared power. The power's square still
  transforms it again, as its own pieces.
- **Threshold.** Without the power's forward transform, the products of the last level
  (about 130 × 109 limbs at the benchmark sizes) are cheaper by transform than by
  Karatsuba: `PREPARED_THRESHOLD = 96`.

M5 Pro. The first three columns are criterion medians from back-to-back runs, before and
after. The last is a same-binary interleaved A/B of thresholds 128 and 96, read at run
time in a copy of the crate (a probe outside the repo: median of 31 rounds of 200 ms
blocks; A/A control 0.9996). Separate criterion runs of the same code differed by up to
5% at 32 KiB, more than the threshold's effect.

| decoded bytes | before  | prepared | change | threshold 96 against 128 |
|--------------:|--------:|---------:|-------:|-------------------------:|
| 32 KiB        | 586 µs  | 528 µs   | −9.8%  | −1.6%                    |
| 36 KiB        | 722 µs  | 635 µs   | −12.0% | −1.0%                    |
| 48 KiB        | 947 µs  | 861 µs   | −9.0%  |                          |
| 128 KiB       | 3.09 ms | 2.72 ms  | −12.0% | −1.9%                    |
| 144 KiB       | 3.85 ms | 3.24 ms  | −15.7% |                          |
| 1 MiB         | 37.9 ms | 30.7 ms  | −18.9% | −1.5%                    |

A profile at 32 KiB shows where the gain comes from. Forward-transform samples at depths
2–4 fell to about half, and the power table's rose by the new transforms. Depth 1 did
not change: the top power is used once and is not prepared.

### Lazy inverse

The inverse transform (Cooley–Tukey butterflies) adds `mont(y, w)`, below 0.75q for any
`y` below 2^31, to a value. So the stages where log2(h) is odd skip both reductions,
except the last stage:

- after a reducing stage every value is at most q/2 + 32;
- a lazy stage then stores less than 1.25q + 32;
- the next stage's sums stay below 2q + 32 < 2^31, and it reduces.

The h = 1 stage and the last stage reduce, so the inverse still takes and returns values
in (−q, q). Half of the stages save 6 of their 12 vector operations. NEON and plain Rust
skip the same reductions (`portable::lazy`), so they stay bit-identical. The forward
transform (Gentleman–Sande) cannot do the same: its unreduced sums double every stage,
and two `mont` outputs summed twice reach 3q > 2^31.

Criterion medians, back-to-back before and after: `ntt-cached` −2.0% at 32 KiB, −6.0% at
36 KiB, −3.3% at 48 KiB, −4.8% at 128 KiB, −4.1% at 144 KiB and −4.9% at 1 MiB. In a
32 KiB profile the inverse samples per depth fell 10–28%.

### Scratch alignment

`as_i32` (`src/ntt/mod.rs`) starts the transforms' `i32` scratch at the next 64-byte
boundary. It skips at most 7 limbs, which the `+ 64` of `scratch_len` covers. Before,
the scratch started where `dc` cut it from its one allocation: after `n` limbs and the
power table. So its address mod 16 depended on the parity of `n` plus the table length.
When it was 8 bytes off a 16-byte boundary, a quarter of the NEON 16-byte loads and
stores crossed a 64-byte cache line, and every transform product took 3–8% longer.

- **Found** from the 36 KiB case in [Power-of-two padding](#power-of-two-padding). In a
  probe, moving the scratch by an odd number of limbs swapped the fast and the slow
  unit, at 36 and at 144 KiB. Moves of 2, 4, 6 and 8 limbs changed nothing, so 16-byte
  alignment is enough on the M5 Pro. 64 bytes also covers wider vectors; x86-64 was not
  measured.
- **Counted** with Instruments (`xctrace`, CPU Counters, guided mode "L1D Request
  Characteristics"), 550 decodes of 36 KiB with unit 158: 629 million µops crossed a
  64-byte line with the scratch 8 bytes off, 73 million with it aligned. µops crossing
  a 16 KiB page: 2.7 million against 8.
- **Which sizes.** The table's slots after the first have even lengths, so the scratch
  was off when `n + unit` was odd. That holds for every size below that got faster, and
  for none of those that did not. At 44 and 56 KiB the unit is odd, so the lowest
  level, whose scratch starts `unit` limbs further in, was aligned already: smaller
  gains.

`ntt-cached` before and after, in one binary (a probe outside the repo: 11 rounds of
about 50 ms per arm and size, order alternating, median of the per-round ratios). An
A/A control gave 0.997–1.000.

| decoded bytes | before  | after   | change |
|--------------:|--------:|--------:|-------:|
| 32 KiB        | 462 µs  | 441 µs  | −4.5%  |
| 36 KiB        | 555 µs  | 536 µs  | −3.4%  |
| 44 KiB        | 724 µs  | 706 µs  | −2.4%  |
| 48 KiB        | 765 µs  | 747 µs  | −2.3%  |
| 56 KiB        | 925 µs  | 909 µs  | −1.7%  |
| 64 KiB        | 1.02 ms | 1.02 ms | 0      |
| 72 KiB        | 1.27 ms | 1.23 ms | −3.1%  |
| 108 KiB       | 2.05 ms | 2.00 ms | −2.4%  |
| 128 KiB       | 2.33 ms | 2.32 ms | −0.1%  |
| 144 KiB       | 2.80 ms | 2.79 ms | −0.2%  |
| 1 MiB         | 27.3 ms | 27.2 ms | +0.3%  |

### Residue placement: measured, not changed

In a product timed alone, one prime's negacyclic product could take up to 16% longer
than the other two (see [Small cyclic pieces](#small-cyclic-pieces)). The cause is an
address match, modulo 4 MiB, between the residue a transform writes and the twiddle
table it reads. Decodes almost never meet it, and two layout rules gained nothing
there, so `pieces` still puts the three primes' residues back to back. All on the M5
Pro, in a copy of the crate with run-time knobs (a probe outside the repo).

- **Which transform.** Timed per prime and kernel inside a 4,510 × 3,690-limb product
  with a prepared operand and a 16-point rest (min over 15 rounds of the mean of 40
  runs): forward 16.4 µs and pointwise 2.1 µs for each prime, inverse 17.6, 23.5 and
  17.5 µs. Another processing order left the second prime slow. Swapping its part of
  the scratch with the first or the third prime's made all three fast. A gap of 16 or
  32 KiB between the residues left that inverse at 18.9 µs; of 48 or 64 KiB, 17.4 µs.
  So the address within a 16 KiB page did not decide it alone.
- **Which addresses.** There the residue started `0x804040` bytes after the second
  prime's `iw` table. A probe copies the tables to chosen offsets in one 64 MiB block
  and times a 16,384-point inverse (16.3–17.4 µs at clean offsets). It is slow when
  the residue starts a little after a table, by 16 to about 336 bytes beyond a
  multiple of 512 bytes, 4 KiB, 16 KiB or 32 KiB, modulo 4 MiB. With such offsets, in
  some blocks of some stages, each twiddle load has the same address modulo 4 MiB as a
  store to the residue 1 to 21 loop iterations earlier. The residue 16 bytes after
  `iw + 8 MiB`: 41.3 µs; 64 bytes after: 22.9 µs; at exactly `iw + 8 MiB`: not slow.
  `+4 MiB` and `+12 MiB` act like `+8 MiB` (21.1 and 20.6 µs against 24.4 µs at 64
  bytes after); offsets more than about 64 KiB from a multiple of 4 MiB were never
  slow. The forward transform against `fw`: the same (30.7 µs 16 bytes after
  `fw + 8 MiB`, 15.3 µs at it).
- **Counted** with Instruments (`xctrace`, CPU Counters, guided modes "L1D Cache
  Metrics", "Discarded Sampling" and "Instruction Processing Bottlenecks"), 50,000
  inverses at a slow and at a clean offset: 30% more cycles; fewer L1D misses (loads
  434 against 471 million, stores 151 against 266 million); work discarded for memory
  dependences about zero in both; twice the share of cycles in which the window holds
  µops but issues none while no L1D miss is pending. So not cache-set conflicts and
  not flushes: loads wait. That fits a check of loads against older, still-buffered
  stores on the low 22 virtual-address bits, like the 4 KiB aliasing of Intel cores.
  This is an inference; Apple does not document it. Some milder slow offsets (64 bytes
  past a 4 KiB multiple, up to 48 KiB from a multiple of 4 MiB, 1–2 µs) do not fit
  it: there the nearest store with a matching address is 260 or more loop iterations
  (4 KiB of residue) back.
  Their cause was not found.
- **Why decodes miss it.** An `n`-point transform is exposed only when its residue
  starts within about `4 n` bytes of a table position modulo 4 MiB, and a little after
  it. In the probe product the scratch, the tables and the prepared transforms were
  fresh page-aligned allocations close together, which made that likely. A decode cuts
  its scratch from `dc`'s buffer. A model of the exposed butterflies (the store window
  only, without the milder cases above) counted, in one decode each, 0.08% of the
  butterflies at 32 KiB, none at 36 and 128 KiB, about 0.002% at 140 KiB, 144 KiB and
  1 MiB. Over 74 sizes from 4 KiB to 2 MiB the most was 0.55%, at 4 KiB.
- **Rules tried.** (a) Start the `i32` scratch at a 4 KiB boundary and the primes'
  residues a multiple of 4 KiB apart (up to 16 KiB more scratch). Twiddle tables
  of `2^12` points or more are 16 KiB-aligned from macOS `malloc`, so every residue
  then starts a multiple of 4 KiB from them. No such offset was slow in the probe, and
  it made the three primes equal in the product above. (b) The 976 unused `i32`s
  between the primes' residues that made that product fast. Products alone with (a),
  60 lengths from 258 to 235,929 limbs, each at 8 scratch offsets: geometric mean of
  the ratios 1.0002 with a prepared operand and 1.0027 without; A/A 1.0054 and 1.0013.

`ntt-cached` decodes in one binary: 31 rounds (62 at 1 MiB) of about 50 ms per arm,
order rotated, median of the per-round ratios. Rule (a) in two runs, (b) in one, each
with its own A/A control:

| decoded bytes | time    | (a) run 1 | (a) run 2 | (b)    | A/A            |
|--------------:|--------:|----------:|----------:|-------:|---------------:|
| 32 KiB        | 444 µs  | +0.13%    | +0.10%    | −0.03% | −0.05…−0.02%   |
| 36 KiB        | 540 µs  | +0.12%    | +0.15%    | +0.09% | −0.10…+0.13%   |
| 128 KiB       | 2.34 ms | +0.06%    | −0.06%    | +0.08% | −0.50…+0.08%   |
| 140 KiB       | 2.76 ms | +0.07%    | +0.14%    | +0.01% | −0.07…+0.15%   |
| 144 KiB       | 2.82 ms | +0.10%    | −0.15%    | −0.38% | −0.25…+0.17%   |
| 1 MiB         | 27.3 ms | +0.15%    | −0.06%    | +0.12% | −0.14…+0.01%   |

### Leaf size: measured, not changed

`LEAF` (256 chunks) bounds the leaves of every `dc` decoder. A same-binary A/B, with the
leaf size read at run time in a copy of the crate, found:

- 192: no change at these sizes. Their units are at most 158 chunks, so the tree is the
  same.
- 128: 0.5% slower at 32 KiB, 128 KiB and 1 MiB, where it halves a unit of 140, and 2.8%
  faster at 36 KiB, where it halves a unit of 158. That was probably not the leaf size:
  halving 158 to 79 makes the power table 43 limbs longer, which aligned the scratch
  (see [Scratch alignment](#scratch-alignment)). Not measured again since that fix.

Measured again with the matrix leaf on aarch64: 256 stays (see
[`decode_matrix`](#decode_matrix-a-matrix-of-powers)).

## `decode_malachite`: malachite-nz multiplication

`decode_malachite` runs the divide and conquer of `decode`, with every product from
malachite-nz 0.14. malachite selects the algorithm from the operand lengths. For equal
lengths on 64-bit targets: schoolbook below 16 limbs, then Toom-22, Toom-33 from 105,
Toom-6h from 300, Toom-8h from 389 and an FFT from 1000. Unequal lengths go to the FFT
from a mean length of 389, if the shorter operand has at least 130 limbs. The FFT is a port of FLINT's small-prime FFT (`fft_small`,
`f64` arithmetic). It pads only to a block multiple, not to a power of two.

- **Divide and conquer, not `decode_batched`.** `decode_batched` multiplies only by one
  limb, `BASE`, so there is no algorithm to select. Inputs up to `DC_THRESHOLD` and the
  leaves still use the two-chain loops.
- **Public API only.** malachite's limb-slice functions (`limbs_mul_to_out` and its
  scratch length) are `pub` only with its `test_build` feature. That feature depends on
  rug (which builds GMP), pyo3 and serde, and those functions are not a stable API. So
  each product goes through `Natural`:
  - The operands are copied into two buffers that the multiplier keeps, so they
    allocate only when they grow. The exception is an operand of at most one
    significant limb, such as a zero `hi`: malachite stores it inline and drops the
    buffer, so the next product allocates it again. That is rare.
  - malachite allocates each product. In the Toom range that is one allocation, with
    the scratch in the same block, and one `shrink_to_fit`. The FFT makes 10–16
    allocations per product, and at 64k limbs also 24–30 reallocations.
  - The multiplier uses no scratch from `dc`. Its `scratch_len(n) = n` only covers
    `dc`'s own temporaries.
- **Squares.** `dc` squares the powers by passing one slice twice. The multiplier sees
  this (`ptr::eq`) and computes `&x * &x`, which malachite sends to its squaring code.
  The NTT decoders square too (see [Squares](#squares)); `dc` and `dc-toom3` run a
  full multiplication there.
- **Keep malachite's `std` feature.** Without it, the FFT rounds with `libm`'s software
  `roundeven` and `fma`.

Allocations in one decode (a counting global allocator; peak heap above the input and
output, after one warm-up decode, so `ntt-cached` does not count its tables):

| decoded bytes | dc: allocations, peak | ntt-cached     | dc-malachite: allocations, reallocations, peak |
|--------------:|----------------------:|---------------:|-----------------------------------------------:|
| 32 KiB        | 1, 166 KiB            | 1, 359 KiB     | 119, 35, 359 KiB                               |
| 128 KiB       | 1, 667 KiB            | 1, 1.4 MiB     | 457, 123, 2.2 MiB                              |
| 1 MiB         | 1, 5.2 MiB            | 1, 11.2 MiB    | 3500, 1068, 17.2 MiB                           |

At 1 MiB the decode makes about 1030 products, so about 3.4 allocations each. All but
O(levels) of them are malachite's own.

Apple M5 Pro, rustc 1.99.0, `cargo bench` with fat LTO and `-C target-cpu=apple-m4`
(see [Build configuration](#build-configuration)); criterion medians
from one run, with other programs running. The `ntt-cached` column predates the later NTT
changes, squares among them. The probes below used the same build settings.

| decoded bytes | dc      | dc-toom3 | ntt-cached | dc-malachite |
|--------------:|--------:|---------:|-----------:|-------------:|
| 32768         | 1.29 ms | 1.14 ms  | 600 µs     | 545 µs       |
| 36864         | 1.55 ms | 1.40 ms  | 769 µs     | 620 µs       |
| 49152         | 2.44 ms | 2.22 ms  | 1.03 ms    | 976 µs       |
| 131072        | 12.3 ms | 9.69 ms  | 3.31 ms    | 2.90 ms      |
| 147456        | 14.3 ms | 11.5 ms  | 4.05 ms    | 3.35 ms      |
| 1048576       | 330 ms  | 223 ms   | 40.2 ms    | 35.9 ms      |

Against this older `ntt-cached`, `dc-malachite` was 5–19% faster. Up to 1 KiB all
four run `decode_batched` and are equal within noise.

The current `ntt-cached` is faster than `dc-malachite` on the M5 from 32 KiB up, and
slower on AMD Zen 5. Criterion medians, one run per machine. The M5 used this README's
build settings. Zen 5 used a default release build, without
LTO and without pinned code alignment (see [Build configuration](#build-configuration)):

| decoded bytes | M5: ntt-cached | M5: dc-malachite | Zen 5: ntt-cached | Zen 5: dc-malachite |
|--------------:|---------------:|-----------------:|------------------:|--------------------:|
| 4 KiB         | 35.9 µs        | 33.5 µs          | 61.5 µs           | 46.5 µs             |
| 32 KiB        | 482 µs         | 537 µs           | 923 µs            | 686 µs              |
| 1 MiB         | 28.4 ms        | 36.9 ms          | 53.6 ms           | 42.4 ms             |

On the M5, `dc-malachite` takes 1.11× as long as `ntt-cached` at 32 KiB and 1.30× at
1 MiB. Back-to-back M5 runs with the same build settings agree at 36–144 KiB
(1.04–1.21×). On Zen 5, `ntt-cached` takes 1.35× as long at 32 KiB and 1.26× at 1 MiB.

The squaring path, measured in one binary against a copy without it (best of 7
interleaved rounds per size; a probe, not in the repo):

| decoded bytes | squares: decode, power table | no squares: decode, power table | change |
|--------------:|-----------------------------:|--------------------------------:|-------:|
| 32 KiB        | 550 µs, 54.7 µs              | 589 µs, 94.4 µs                 | +7.0%  |
| 36 KiB        | 641 µs, 68.2 µs              | 690 µs, 115 µs                  | +7.6%  |
| 128 KiB       | 3.12 ms, 246 µs              | 3.31 ms, 356 µs                 | +6.0%  |
| 144 KiB       | 3.49 ms, 279 µs              | 3.64 ms, 397 µs                 | +4.3%  |
| 1 MiB         | 38.3 ms, 2.52 ms             | 39.4 ms, 3.35 ms                | +2.9%  |

- So squares save 3–8% of a decode, and most of the lead over the older `ntt-cached`
  remains without them. A first estimate from two separate `cargo bench` runs said 7–17%; that
  was run-to-run noise.
- Part of the gain at the small sizes is a malachite threshold, not squaring itself.
  Equal-length products of 389–999 limbs use Toom-6h or Toom-8h, while an n × (n − 1)
  product of the same size uses the FFT: 77.8 µs against 50.8 µs at 999 limbs. Squares
  go to the FFT from 389 limbs. In the FFT range a square costs about 0.72× a product,
  and at 243–300 limbs (Toom) about 0.95×.

## Results

Apple M5 Pro, rustc 1.99.0, `cargo bench` with fat LTO and `-C target-cpu=apple-m4`
(see [Build configuration](#build-configuration)); criterion medians
from one run, expect several percent of noise between runs. The ad-hoc harness below
used the same build settings.

| decoded bytes | bs58    | basexx  | basexx-b | batched | dc      | dc-toom3 |
|--------------:|--------:|--------:|---------:|--------:|--------:|---------:|
| 31            | 304 ns  | 104 ns  | 410 ns   | 22.1 ns | 23.1 ns | 23.3 ns  |
| 32            | 333 ns  | 108 ns  | 422 ns   | 19.9 ns | 20.9 ns | 20.8 ns  |
| 128           | 4.84 µs | 284 ns  | 3.74 µs  | 110 ns  | 106 ns  | 96 ns    |
| 1024          | 335 µs  | 5.37 µs | 305 µs   | 2.94 µs | 2.95 µs | 2.97 µs  |
| 32768         | 339 ms  | 4.20 ms | 315 ms   | 2.33 ms | 1.25 ms | 1.12 ms  |

Up to 1024 bytes, `dc` and `dc-toom3` run `decode_batched`. Their differences there,
up to 13% at 128 B, are code layout and noise, not algorithm.

Larger inputs, from an ad-hoc harness: best of repeated runs, random input, same
machine (`dc-toom3` from a second run):

| decoded bytes | batched | dc      | dc-toom3 | dc allocations | dc peak heap |
|--------------:|--------:|--------:|---------:|---------------:|-------------:|
| 2 KiB         | 9.75 µs | 9.75 µs | (batched) | 0 (batched)   | –            |
| 4 KiB         | 36.3 µs | 36.6 µs | 36.7 µs  | 1              | 20 KiB       |
| 32 KiB        | 2.19 ms | 1.18 ms | 1.06 ms  | 1              | 166 KiB      |
| 128 KiB       | 36.3 ms | 11.5 ms | 9.24 ms  | 1              | 667 KiB      |
| 256 KiB       | 146 ms  | 35.6 ms | 26.6 ms  | 1              | 1.3 MiB      |
| 512 KiB       | –       | 108 ms  | 76.3 ms  | 1              | 2.6 MiB      |

`decode_batched` makes no heap allocations at any size. Before the allocation work,
`decode` made 1,110 allocations at 32 KiB and 95,629 at 512 KiB, and its peak heap was
about 30% higher. An interleaved A/B against that version, both in the same binary:

- `batched`: 14–26% faster below 128 B, 6% faster at 1 KiB, the same at 32 KiB.
- `dc`: 3–5% faster from 4 KiB to 128 KiB. At 2 KiB the difference is within
  build-to-build noise: the unchanged old code itself moved 7% between two builds.

## Tests

`cargo test` (about 7 s; the dev profile uses `opt-level = 3` but keeps debug
assertions and overflow checks):

- **Every decoder in `DECODERS` against `bs58`:** output, first-invalid-character index, exactly
  sized buffer, one-byte-short buffer (`BufferTooSmall`), and no writes past the
  result.
  - Byte patterns: random, leading zero bytes, all `0xFF`, all zero.
  - Lengths: every length 0–300, 31 lengths around `DC_THRESHOLD` (about 3.75 KB),
    1/4/8 KiB, 32 KiB and 32 KiB + 1, plus `58^n − 1` at about 32 KiB. The larger
    benchmark sizes are too slow for `bs58`; the next item covers them.
  - Invalid characters, ASCII and non-ASCII, at several positions on both paths.
  - `2^(64k)`, a one and `k` zero limbs, for 11 values of `k` from 1 to 1000.
  - 300 random strings, some with an invalid character, into buffers of random and
    boundary sizes: the result fits exactly when the buffer holds it, and an invalid
    string reports its character, or `BufferTooSmall` only when the buffer is too
    small.
- **Every decoder against `decode_batched` at 300k characters.** It shares only the
  chunk parsing with the others. Inputs: random, maximum-carry `zzz…`, leading `'1'`s,
  and an inner run of 100k `'1'`s that makes whole subtrees zero.
- **`decode` against `decode_batched`** (`src/dc.rs`) with each leaf, up to 164k
  characters, including the maximum-carry string `zzz…` and a first leaf of 2 chunks.
  Short buffers and invalid characters fail before any multiplier call, and the early
  length check accepts the shortest value of each length into an exact buffer. A
  checking multiplier sees every power prepared in order, and every product with a
  power within the bound it was prepared for.
- **Karatsuba and the two-row schoolbook against a one-row reference**
  (`src/mul.rs`) for operand lengths around the thresholds, with random and all-ones
  limbs.
- **Toom-3 against Karatsuba** (`src/toom3.rs`): lengths around both thresholds, with
  Toom-3 also forced down to 32 limbs; operands that make `r(−1)` positive or
  negative; exact division by 3; the scratch bound; and `decode_toom3` against
  `decode` at 100k and 300k characters.
- **NTT** (`src/ntt/`): the primes and roots; Montgomery and Barrett bounds on edge
  values; twiddles against direct powers; the forward transform against a naive DFT;
  inverse round trips; Garner's CRT at coefficients up to 2^86 − 1; the NEON kernels
  bit-identical to the plain-Rust ones; `square` bit-identical to `pointwise` of a
  vector with itself; `mul_with` against Karatsuba for every shape up to 64×64 limbs,
  around the threshold, unbalanced and split, with squares (one slice twice) of those
  lengths, and an all-ones 2^18-limb square and product with exact scratch; all-ones
  squares and products through two and three pieces at 2^20 + 2^18 and
  2^20 + 2^19 + 2^16 points; squares that never call `pointwise`, and products with a
  copy that never call `square`; the plans (coverage, distinct
  powers of two, table size) for every product up to 2^14 limbs; products and squares
  of every length from 4 limbs below to 36 above 2^k and 1.5·2^k limbs (k = 7, 10, 12),
  so every small cyclic piece with two and three pieces; the polynomial CRT
  and the fused two-piece kernel against direct reduction; the twisted load against
  direct powers of the root; the three decoders against `decode` up to 1M
  characters; 8 threads racing to build the cached tables; smaller tables being
  prefixes of larger ones; tables sized for the longest product of a decode; products
  with prepared powers against Karatsuba (one cyclic transform, a prefix of a longer
  one, one and two negacyclic pieces, operands wrapping modulo the cyclic piece, and
  every fallback); a count of forward transforms showing that such a product transforms
  only `hi`; every piece transform being a slice of one cyclic transform; the inverse at
  the largest input magnitudes for every length from 8 to 2^14; a real decode through
  `split` with capped tables; and the scratch bound covering Karatsuba's.
  `cargo test -- --ignored` also runs 2^23-point products and squares (the largest) of
  every kind of plan, all-ones operands against the closed form `(X^a − 1)(X^b − 1)`, with both
  kernel sets: about 5 s and 1 GB. The dev profile's overflow checks keep the plain-Rust
  kernels scalar, so `cargo test --release` and, on Apple Silicon,
  `ROSETTA_ADVERTISE_AVX=1 cargo test --release --target x86_64-apple-darwin` run the
  vectorized and AVX2 kernels.
- **malachite** (`src/malachite.rs`): the multiplier against Karatsuba around each
  malachite threshold up to 2500 limbs, with squares (one slice twice) and all-ones
  limbs, through one multiplier whose buffers grow and shrink, and with no scratch; and
  `decode_malachite` against `decode` up to 1M characters.

## Possible next steps

- **Toom-3** could still save work with a squaring variant for the power table and by
  fusing its linear passes.
- **Four carry chains** in the quadratic loop: about 1.3× more in the microbenchmark,
  for more complex tail handling.
- **Choose `dc`'s unit per decode** (see
  [Power-of-two padding](#power-of-two-padding)). Measured after the alignment fix and
  the 64-point smallest cyclic piece, the fitted model's unit was 1.2% faster on average
  over 34 sizes, up to 5% at some, and slower at none beyond the noise. A real
  implementation needs:
  - A cheaper search. The probe walks every node for each unit: 46 µs at 32 KiB, 10% of
    the decode. Full nodes of one level have the same shape, so their cost is needed
    once per level: about `4·levels` formula evaluations per unit, about 2,300 at
    32 KiB. At the probe's rate (about 5 ns each) that is about 13 µs, still more than
    the 1% it gains there (an estimate, not measured). So search only above some size,
    or use a table made in advance: the unit depends only on `n`.
  - The choice only where it was measured. The formula models `ntt-cached`'s
    multiplier; the other `dc` decoders were not measured with other units.
  - Coefficients for each target, or a check that these transfer. They were fitted on
    the M5 Pro; x86-64 was not measured.
  - Tests that every unit the search can return decodes correctly at many lengths. The
    probe checked every accepted unit at six sizes.
- **Align the other NTT buffers** to 64 bytes. The prepared transforms and the twiddle
  tables are `Vec`s, 16-byte aligned from macOS `malloc`, and the operands' limbs start
  at any 8-byte offset. With the scratch aligned, 550 decodes of 36 KiB still count 73
  million line-crossing µops (against 629 million before); where they come from was not
  looked at. On the M5 Pro 16-byte alignment was enough for the scratch. A 64-byte
  AVX-512 access that is not 64-byte aligned always spans two lines, so x86-64 may gain
  more. Not measured.
- **Residues on 4 KiB boundaries on x86-64.** Intel documents "4K aliasing": a load
  waits for an older store whose address matches in the low 12 bits. There the
  twiddle loads of [Residue placement](#residue-placement-measured-not-changed) would
  wait whenever a residue starts a little after a table position modulo 4 KiB, with no
  4 MiB condition: much more often than on the M5 Pro. Rule (a) of that section avoids
  it unless a table starts a few hundred bytes or less before a 4 KiB boundary (macOS
  puts large blocks on one, glibc 16 bytes past one). Not measured; how AMD Zen 5, the
  remote x86-64 machine, behaves here is not known.
- **Squares from prepared transforms**: a power's square could take its pieces from the
  power's stored transform instead of transforming it again. That is about 2–3% at
  32 KiB, going by the profile.

## `decode_matrix`: a matrix of powers

Firedancer's decode, for any length up to 5632 characters (about 4.0 KiB decoded):

- Group the digits in fours: `d_i < 58^4`, and the value is `Σ d_i · 58^(4i)`.
- A table holds `58^(4i)` in base 2^32. Each output word is a column sum
  `Σ d_i · T[i][j]`, kept in `u64` (NEON `umlal`). A final carry pass turns the sums
  into 64-bit limbs.
- The products are independent, so this runs at SIMD multiply-add throughput instead
  of along the Horner loop's carry chain. A 32-bit carry pass every 376 rows keeps the
  sums below 2^64; 379 rows is the bound.

The table is built on first use and kept, in four size classes: 40 KiB (704
characters), 154 KiB (1408), 601 KiB (2816) and 2.3 MiB (5632). A smaller input uses a
larger class's table if one is built already. The first call in a class took about
38 µs, 76 µs, 234 µs and 0.96 ms longer than later calls (the build plus cold caches).
Below 576 characters (about 420 B) `decode_matrix` calls `decode_batched`, and above
5632 it calls `decode`. `decode_matrix_portable` is the same algorithm with plain-Rust
kernels.

Apple M5 Pro, criterion medians from one run, with other benchmarks running on the
machine:

| decoded bytes | batched | matrix  | matrix-portable | speedup |
|--------------:|--------:|--------:|----------------:|--------:|
| 31            | 19.5 ns | 18.9 ns | 18.8 ns         | –       |
| 32            | 15.2 ns | 15.0 ns | 16.2 ns         | –       |
| 128           | 72.6 ns | 71.9 ns | 73.0 ns         | –       |
| 256           | 214 ns  | 203 ns  | 198 ns          | –       |
| 512           | 714 ns  | 605 ns  | 1.21 µs         | 1.18×   |
| 1024          | 2.75 µs | 1.92 µs | 4.36 µs         | 1.44×   |
| 1875          | 9.34 µs | 6.25 µs | 17.0 µs         | 1.49×   |
| 4096          | 40.8 µs | 26.0 µs | 65.0 µs         | 1.57×   |

Up to 256 B, both matrix decoders run `decode_batched`; the differences there are
noise. The plain-Rust kernels are 1.6–1.8× slower than `decode_batched`: the SIMD
multiply-adds are what makes the matrix pay. `decode_batched` is a fast baseline here:
it runs two carry chains at once, at about 1.1–1.3 cycles per 64×64 limb step, close to
the scalar multiplier's throughput.

**Cold caches.** The speedup needs the table in cache: in a benchmark loop, or across
the leaves of one `dc` decode. Single calls after writing 64 MiB to evict the caches
(ad-hoc harness; medians of three runs, each the median of 31 calls):

| decoded bytes | batched | matrix  | speedup |
|--------------:|--------:|--------:|--------:|
| 512           | 0.88 µs | 1.29 µs | 0.68×   |
| 1024          | 2.96 µs | 3.38 µs | 0.88×   |
| 1875          | 8.71 µs | 9.17 µs | 0.95×   |
| 4096          | 38.5 µs | 39.5 µs | 0.97×   |

For one-off decodes of up to 4 KiB, `decode_matrix` is slower than `decode_batched`.

`MATRIX_MIN` was tuned with the minimum of 31 interleaved rounds per size, because
other benchmarks were running. Against the current `decode_batched`, `decode_matrix`
was 0.94–0.97× at 330–384 B and 1.07× at 416 B.

**Matrix leaves in `dc`.** On aarch64 `decode_ntt_cached` and `decode_ntt_portable`
convert their leaves with `matrix::leaf` instead of the chunk parse and Horner loop. Its
table is built on first use and kept, like their twiddle tables. `decode`,
`decode_toom3`, `decode_malachite` and `decode_ntt` keep the Horner loop and no state
between calls. Other targets use the Horner loop everywhere: x86_64 is not measured yet.
An A/B with both leaves in one binary, in interleaved rounds (Apple M5 Pro on AC power,
matrix time / Horner time):

| decoded bytes | ntt-cached, warm | ntt-cached, first decode in a process |
|--------------:|-----------------:|--------------------------------------:|
| 4 KiB         | 0.87             | 2.37                                  |
| 32 KiB        | 0.95             | 1.10                                  |
| 48 KiB        | 0.93             | 1.18                                  |
| 128 KiB       | 0.95             | 0.98                                  |
| 1 MiB         | 0.96             | 0.97                                  |

The first decode in a process builds the table: 50–85 µs more at 4–32 KiB, whose leaves
have 140 chunks (154 KiB class), and 145–190 µs at 48 KiB, whose leaves have 210 (601 KiB
class). The other `dc` decoders were faster warm too (0.87–0.99 up to 128 KiB). Without
a cache they would build the table in every call, at about that first-decode cost (an
estimate: it also includes a new process's cold caches), so the leaf would only pay from
about 128 KiB. `LEAF` and `DC_THRESHOLD` were measured again with this leaf and stay.
`LEAF = 512` was slower for the NTT decoders at 4 and 48 KiB. From 400 to 560 chunks,
`dc` with matrix leaves beat `decode_batched` by up to 7%, but `decode_matrix` was 1.5×
faster than both.

On x86_64 the panel kernel uses AVX2 or SSE4.1 (`pmuludq` and `paddq`), chosen at run
time, and parsing is plain Rust. Both kernels pass the tests under Rosetta on Apple
Silicon, AVX2 with `ROSETTA_ADVERTISE_AVX=1`. On AMD Zen 5 (GCP `c4d-highcpu-4`,
criterion medians, one run per build) the AVX2 kernel was 1.9–3.9× faster than the
plain-Rust one. Against `decode_batched`:

| decoded bytes | batched, default | matrix, default | batched, native | matrix, native |
|--------------:|-----------------:|----------------:|----------------:|---------------:|
| 512           | 0.99 µs          | 1.00 µs         | 793 ns          | 1.01 µs        |
| 1024          | 3.82 µs          | 2.97 µs         | 2.73 µs         | 3.02 µs        |
| 1875          | 12.1 µs          | 8.90 µs         | 8.51 µs         | 8.77 µs        |
| 4096          | 55.0 µs          | 37.8 µs         | 38.9 µs         | 37.4 µs        |

"Native" is `-C target-cpu=native` (znver5). It makes `decode_batched` 20–30% faster
and changes the matrix kernel by at most 2%, so with it `decode_matrix` only ties from
about 2 KiB. The "batched, default" column is from an earlier run on the same VM: in
the run with the matrix kernel, default-flags `decode_batched` was 21–31% slower with
the same instructions. The cause is code placement: the function started 16 bytes past
a 64-byte boundary, so its 49-byte inner loop crossed into the next line. With
`-C llvm-args=-align-all-functions=6` it ran at the column's speed again. Whether the
native builds placed `decode_batched` well was not checked.

# base58-bench

Benchmarks base58 (Bitcoin alphabet) decoders and encoders, plus faster ones in `src/`.
Encoders are described in [Encoding](#encoding). The decoders:

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
  for about 0.4 KiB to 4 KiB. The speedup was measured with NEON (aarch64). x86_64 has
  AVX2 and SSE4.1 kernels, not measured; elsewhere the plain-Rust kernels are slower
  than `decode_batched`.

All return the same bytes as `bs58`. On invalid input they report the same index,
the first invalid byte. If the input is invalid and the buffer is also too small for
it, which of the two errors is reported is unspecified: it can differ from `bs58` and
between decoders.

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
| `src/encode/`       | encoders: `batched.rs`, `dc.rs` (divide and conquer), `tests.rs` |
| `benches/decode.rs` | criterion benchmark of all decoders                             |
| `benches/encode.rs` | criterion benchmark of all encoders                             |

To add a decoder:
1. Put it in a new module and re-export it from `lib.rs`.
2. Add one line to `DECODERS` in `src/tests.rs`.
3. Add one line to `DECODERS` in `benches/decode.rs`.

Encoders work the same way, with `ENCODERS` in `src/encode/tests.rs` and
`benches/encode.rs`. A backend file (`toom3.rs`, `ntt/`, `malachite.rs`) holds both its
`decode_*` and its `encode_*`.

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

Thresholds were re-tuned on an Apple M5 Pro after the two-chain kernels:
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
4096 limbs:

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

Apple M5 Pro, rustc 1.99.0, `cargo bench` (criterion medians from one run). 36864,
49152 and 147456 bytes need products just above a power of two; see
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

`c` is the smallest power of two (at least 8) that covers the rest of `p`. The
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

- **Aligning `dc`'s splits** so products land just below a power of two. A scan of every
  `unit` that `dc` accepts near its default gave the upper bound below, with the best
  unit for each size, confirmed in a second interleaved run:

  | decoded bytes | default unit | best unit | change |
  |--------------:|-------------:|----------:|-------:|
  | 32 KiB        | 140          | 146       | −0.9%  |
  | 36 KiB        | 158          | 162       | −0.6%  |
  | 44 KiB        | 193          | 225       | −5.5%  |
  | 64 KiB        | 140          | 149       | −3.3%  |
  | 128 KiB       | 140          | 145       | −2.6%  |
  | 144 KiB       | 158          | 159       | −2.0%  |

  Run-to-run noise is about 3%, so only 44 KiB is clearly better. The best unit
  differs by size. A real version would need a cost method on `Multiplier`, and a
  search for the unit on every decode, which costs about as much as the gain at
  32 KiB.
- **A truncated FFT.** The pieces are a coarse truncated FFT. On the same points they
  do fewer butterflies than one, because each piece's transform is shorter than `L`.
  After them a truncated FFT could save at most about 17% on the worst products and
  less on average, before its own overhead. It would also need a new inverse kernel in
  NEON. Modelled, not built.
- **Mixed radix `3·2^k`.** It needs new primes: of the three, only q3 = 105·2^23 + 1 is
  1 mod 3·2^23. Not tried.

### Squares

`dc` squares each power by passing one slice twice. `transform` and `pieces` see this
(`ptr::eq`). They load and transform the operand once per prime instead of twice, then
square pointwise (the `square` kernel). In `split`, which a decode never reaches, the
two diagonal products of halves are squares too.

Before prepared powers, the power table took about 9–12% of an `ntt-cached` decode (a
probe timing `dc::powers()`). A square skips one load and one of the three transforms
per prime, so the expected gain was about 3–4%. Measured on the Apple M5 Pro (NEON
kernels; not measured on x86-64), in one binary against the same code with the square
path turned off, with [prepared powers](#prepared-powers) and the
[lazy inverse](#lazy-inverse) in place (a probe, not in the repo: 41 interleaved rounds
of about 100 ms per size, median of the per-round ratios, two runs, times from the
second):

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

### Leaf size: measured, not changed

`LEAF` (256 chunks) bounds the leaves of every `dc` decoder. A same-binary A/B, with the
leaf size read at run time in a copy of the crate, found:

- 192: no change at these sizes. Their units are at most 158 chunks, so the tree is the
  same.
- 128: 0.5% slower at 32 KiB, 128 KiB and 1 MiB, where it halves a unit of 140, and 2.8%
  faster at 36 KiB, where it halves a unit of 158.

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

Apple M5 Pro, rustc 1.99.0, `cargo bench` (criterion medians from one run, with other
programs running):

| decoded bytes | dc      | dc-toom3 | ntt-cached | dc-malachite |
|--------------:|--------:|---------:|-----------:|-------------:|
| 32768         | 1.29 ms | 1.14 ms  | 600 µs     | 545 µs       |
| 36864         | 1.55 ms | 1.40 ms  | 769 µs     | 620 µs       |
| 49152         | 2.44 ms | 2.22 ms  | 1.03 ms    | 976 µs       |
| 131072        | 12.3 ms | 9.69 ms  | 3.31 ms    | 2.90 ms      |
| 147456        | 14.3 ms | 11.5 ms  | 4.05 ms    | 3.35 ms      |
| 1048576       | 330 ms  | 223 ms   | 40.2 ms    | 35.9 ms      |

`dc-malachite` is 5–19% faster than `ntt-cached`. Up to 1 KiB all four run
`decode_batched` and are equal within noise.

The squaring path, measured in one binary against a copy without it (best of 7
interleaved rounds per size; a probe, not in the repo):

| decoded bytes | squares: decode, power table | no squares: decode, power table | change |
|--------------:|-----------------------------:|--------------------------------:|-------:|
| 32 KiB        | 550 µs, 54.7 µs              | 589 µs, 94.4 µs                 | +7.0%  |
| 36 KiB        | 641 µs, 68.2 µs              | 690 µs, 115 µs                  | +7.6%  |
| 128 KiB       | 3.12 ms, 246 µs              | 3.31 ms, 356 µs                 | +6.0%  |
| 144 KiB       | 3.49 ms, 279 µs              | 3.64 ms, 397 µs                 | +4.3%  |
| 1 MiB         | 38.3 ms, 2.52 ms             | 39.4 ms, 3.35 ms                | +2.9%  |

- So squares save 3–8% of a decode, and most of the lead over `ntt-cached` remains
  without them. A first estimate from two separate `cargo bench` runs said 7–17%; that
  was run-to-run noise.
- Part of the gain at the small sizes is a malachite threshold, not squaring itself.
  Equal-length products of 389–999 limbs use Toom-6h or Toom-8h, while an n × (n − 1)
  product of the same size uses the FFT: 77.8 µs against 50.8 µs at 999 limbs. Squares
  go to the FFT from 389 limbs. In the FFT range a square costs about 0.72× a product,
  and at 243–300 limbs (Toom) about 0.95×.

## Results

Apple M5 Pro, rustc 1.99.0, `cargo bench` (criterion medians from one run; expect
several percent of noise between runs):

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

`cargo test` (about 5 s; the dev profile uses `opt-level = 3` but keeps debug
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
  powers of two, table size) for every product up to 2^14 limbs; the polynomial CRT
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
- **`dc` split alignment for the NTT**: up to 5.5% at some sizes (see
  [Power-of-two padding](#power-of-two-padding)), if choosing the unit costs less than
  it saves.
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
time, and parsing is plain Rust. Both kernels pass the tests under
Rosetta on Apple Silicon, AVX2 with `ROSETTA_ADVERTISE_AVX=1`; there are no x86
performance numbers.

## Encoding

`encode*(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall>` write the
Bitcoin-alphabet string and return its length. Every encoder returns the same bytes as
`bs58`, accepts an exactly sized buffer, fails with `BufferTooSmall` one byte short, and
writes nothing past the result. `max_encoded_len(n)` bounds the length for `n` bytes.

- `encode_batched`: quadratic, in place in `output`, no allocation.
- `encode`: divide and conquer with Karatsuba; `encode_toom3`, `encode_ntt`,
  `encode_ntt_cached` and `encode_malachite` use the other multipliers, and
  `encode_ntt_portable` the NTT with the plain-Rust kernels (as `decode_ntt_portable`).
  All run `encode_batched` up to their threshold.
- `encode_matrix`, `encode_matrix_portable`: a matrix of powers with the decoder's SIMD
  multiply-adds, up to 8 KiB; see [below](#encode_matrix-a-matrix-of-powers). Exactly 32
  or 64 bytes take fixed-size code ([Fixed sizes](#fixed-sizes-32-and-64-bytes)).

Encoding is the harder direction. Decoding only multiplies: `x·58^10 + c` keeps the
multiply off the carry chain, and its divide-and-conquer does one product per node.
Encoding divides by powers of 58, so both the quadratic kernel and the tree cost more.

### `encode_batched`: one division by 58^10 split off the carry chain

The value is kept in base `BASE = 58^10` (limbs below 2^58.6, ten characters each).
Each pass takes 7 input bytes, `c < 2^56`, and computes `y ← y·2^56 + c` over the
limbs: per limb `t = y·2^56 + carry`, new limb `t mod BASE`, carry `t div BASE`.

- **The carry is below 2^56.** `t < BASE·2^56`, so the quotient is too. That is why
  passes take 56 bits, not 64.
- **The division is split.** `(q, r) = divmod(y·2^56, BASE)` depends only on the old
  limb, so it runs off the chain: `q = umulh(y, floor(2^120 / BASE))` is exact or one
  less, and the remainder is fixed with one compare. Then `r + carry < 2·BASE`, so one
  compare finishes the division on the chain, and the carry out is `q` or `q + 1`.
- **Two passes per sweep**, the second one limb behind, as in `decode_batched`.
- **In place.** A limb takes 8 bytes and becomes 10 characters. The limbs sit
  right-aligned below `min_digits`, a lower bound on the digit count that is at most two
  below it (`floor(log58(2)·2^64)` in fixed point), and the top limb stays in a register.
  The text is written top limb first. The text of limb `i` lands at or above the bytes of
  limb `i`, so it only overwrites limbs already read. So the result never needs more
  room than its own length, and nothing past it is written.

### `encode`: divide and conquer with Barrett division

The tree is `decode`'s, run the other way: `x = hi·58^(10e) + lo`, with the same power
table (balanced splits `unit·2^i`, powers stored without their low zero limbs), and
`lo` written as exactly `10e` characters, zero digits included.

- **Division by a power `P`.** `58^k = 2^k·29^k`, so the low `shift` zero limbs of `P`
  only pass `x`'s low limbs through to the remainder. The rest is shifted to set the top
  bit (`pn`) and divided with Barrett's method: the quotient estimate is the high part
  of `floor(A/β^dn)·inv`, at most 6 below the quotient; the remainder takes one product
  with `pn` and up to 6 subtractions. `x < P^2` bounds every length.
- **Reciprocals once per power.** `inv = floor(β^(2qn) / (pn·β^shift))`, or up to two
  less, comes from Newton's method (Brent–Zimmermann, Algorithm 3.5): the reciprocal of
  the top half, then one Newton step; schoolbook division (Knuth's Algorithm D) below
  17 limbs. Each power's reciprocal serves every division by it, as in malachite's
  port of GMP's `mpn_get_str` (GMP itself recomputes it per division).
- **Prepared operands.** Every power but the top one divides at least two nodes, so
  `pn` and `inv` go to `Multiplier::prepare_power`: the NTT keeps their transforms.
- **A wrapped `q·pn`.** The remainder `A - q·pn` is below `7·pn < β^(dn+1)`, so only
  its residue modulo `β^L - 1` for some `L >= dn + 2` is needed. `Multiplier::wrap_len`
  says whether a multiplier has such a product cheaper than the full one: the NTT, when
  a cyclic transform of `2L` points (`L` the next power of two) is shorter than the
  pieces of the full product. Its stored transform of `pn` serves as is: the first `2L`
  values are those of `pn mod (x^(2L) - 1)`. A cyclic coefficient sums at most
  `2L <= 2^23` products of 32-bit coefficients, below `2^87`, so Garner's CRT still
  recovers it; the carry out of the top limb wraps to the bottom.
- **Leaves** convert with `encode_batched`'s kernel on stack buffers, or in the caching
  NTT encoders with `encode_matrix`'s conversion ([Matrix leaves](#matrix-leaves)).
- So each node costs two products (`A1 · inv` and `q · pn`, maybe wrapped) against
  `decode`'s one, plus a reciprocal per level.

### Encoding results

Apple M5 Pro, AC power, commit `49fb679`, `cargo bench --bench encode -- --warm-up-time 1
--measurement-time 3`: criterion medians from one run of every encoder. A watcher saw no
build or other benchmark during the run. Random input with a non-zero first byte.
`malachite` is malachite-nz's own conversion (`Natural::to_digits_desc`, a port of GMP's
`mpn_get_str`) plus the alphabet; `five8` is Firedancer's encoder for 32 and 64 bytes only
(scalar on aarch64); `basexx` is num-bigint's conversion. The divide-and-conquer encoders
run `encode_batched` up to their threshold, so they are timed from 4 KiB; `encode_matrix`
converts up to 8 KiB itself and calls `encode_ntt_cached` above. The 32 and 64 byte rows
are from a later run, with `encode_matrix`'s fixed-size path, of every encoder at those
sizes ([Fixed sizes](#fixed-sizes-32-and-64-bytes)).

Against the other crates (`× bs58`: our fastest against `bs58`; `× best other`: against
the fastest other crate at that size):

| bytes | bs58 | basexx | five8 | malachite | fastest here | × bs58 | × best other |
|---:|---:|---:|---:|---:|:---|---:|---:|
| 31 | 864 ns | 147 ns |  | 140 ns | **32 ns** matrix | 27× | 4.38× |
| 32 | 960 ns | 150 ns | 31.1 ns | 135 ns | **12.7 ns** matrix | 75× | 2.44× |
| 64 | 3.1 µs | 308 ns | 74.8 ns | 241 ns | **28.8 ns** matrix | 108× | 2.60× |
| 128 | 13.8 µs | 946 ns |  | 530 ns | **116 ns** matrix | 119× | 4.56× |
| 256 | 56.9 µs | 3.27 µs |  | 1.5 µs | **231 ns** matrix | 246× | 6.51× |
| 512 | 235 µs | 8.08 µs |  | 3.69 µs | **585 ns** matrix | 402× | 6.31× |
| 1024 | 943 µs | 22.1 µs |  | 10 µs | **1.95 µs** matrix | 485× | 5.16× |
| 1875 | 3.2 ms | 56.1 µs |  | 23.9 µs | **5.91 µs** matrix | 542× | 4.04× |
| 4096 | 15.4 ms | 175 µs |  | 76 µs | **28.9 µs** matrix | 533× | 2.63× |
| 6144 | 35.5 ms | 309 µs |  | 142 µs | **65.4 µs** matrix | 543× | 2.16× |
| 8192 | 61.1 ms | 524 µs |  | 204 µs | **119 µs** matrix | 513× | 1.71× |
| 12288 | 137 ms | 878 µs |  | 380 µs | **335 µs** ntt-cached | 409× | 1.13× |
| 16384 | 243 ms | 1.51 ms |  | 566 µs | **489 µs** ntt-cached | 497× | 1.16× |
| 24576 | 550 ms | 2.56 ms |  | 1.03 ms | **826 µs** ntt-cached | 666× | 1.25× |
| 32768 | 973 ms | 4.64 ms |  | 1.54 ms | **1.2 ms** ntt-cached | 812× | 1.28× |
| 36864 |  |  |  | 2.09 ms | **1.33 ms** ntt-cached |  | 1.57× |
| 49152 |  |  |  | 2.79 ms | **1.95 ms** ntt-cached |  | 1.43× |
| 131072 |  |  |  | 9.04 ms | **6.2 ms** ntt-cached |  | 1.46× |
| 147456 |  |  |  | 10.6 ms | **6.88 ms** ntt-cached |  | 1.53× |
| 1048576 |  |  |  | 108 ms | **67.6 ms** ntt-cached |  | 1.59× |

Every encoder:

| bytes | bs58 | basexx | basexx-b | malachite | batched | matrix | matrix-portable | five8 | dc | dc-toom3 | ntt | ntt-cached | ntt-portable | dc-malachite |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 31 | 864 ns | 147 ns | 838 ns | 140 ns | 36 ns | **32 ns** | 38.7 ns | | | | | | | |
| 32 | 960 ns | 150 ns | 869 ns | 135 ns | 40 ns | **12.7 ns** | 19 ns | 31.1 ns | | | | | | |
| 64 | 3.1 µs | 308 ns | 3.07 µs | 241 ns | 88.6 ns | **28.8 ns** | 76.4 ns | 74.8 ns | | | | | | |
| 128 | 13.8 µs | 946 ns | 11.7 µs | 530 ns | 211 ns | **116 ns** | 159 ns | | | | | | | |
| 256 | 56.9 µs | 3.27 µs | 47.5 µs | 1.5 µs | 634 ns | **231 ns** | 418 ns | | | | | | | |
| 512 | 235 µs | 8.08 µs | 194 µs | 3.69 µs | 2.13 µs | **585 ns** | 1.28 µs | | | | | | | |
| 1024 | 943 µs | 22.1 µs | 786 µs | 10 µs | 8.14 µs | **1.95 µs** | 4.55 µs | | | | | | | |
| 1875 | 3.2 ms | 56.1 µs | 2.65 ms | 23.9 µs | 24.2 µs | **5.91 µs** | 14.5 µs | | | | | | | |
| 4096 | 15.4 ms | 175 µs | 12.6 ms | 76 µs | 107 µs | **28.9 µs** | 68.1 µs | | 107 µs | 107 µs | 107 µs | 106 µs | 107 µs | 92.5 µs |
| 6144 | 35.5 ms | 309 µs | 28.7 ms | 142 µs | 241 µs | **65.4 µs** | 148 µs | | 222 µs | 222 µs | 186 µs | 124 µs | 165 µs | 173 µs |
| 8192 | 61.1 ms | 524 µs | 50.5 ms | 204 µs | 414 µs | **119 µs** | 265 µs | | 351 µs | 345 µs | 264 µs | 190 µs | 254 µs | 251 µs |
| 12288 | 137 ms | 878 µs | 114 ms | 380 µs | 934 µs | | | | 680 µs | 657 µs | 441 µs | **335 µs** | 514 µs | 443 µs |
| 16384 | 243 ms | 1.51 ms | 203 ms | 566 µs | 1.63 ms | | | | 1.09 ms | 1.04 ms | 617 µs | **489 µs** | 735 µs | 621 µs |
| 24576 | 550 ms | 2.56 ms | 454 ms | 1.03 ms | 3.67 ms | | | | 2.07 ms | 1.95 ms | 1.01 ms | **826 µs** | 1.33 ms | 1.1 ms |
| 32768 | 973 ms | 4.64 ms | 865 ms | 1.54 ms | 6.95 ms | | | | 3.55 ms | 3.32 ms | 1.47 ms | **1.2 ms** | 1.96 ms | 1.69 ms |
| 36864 | | | | 2.09 ms | | | | | 4.35 ms | 3.97 ms | 1.7 ms | **1.33 ms** | 2.13 ms | 1.85 ms |
| 49152 | | | | 2.79 ms | | | | | 6.52 ms | 5.92 ms | 2.37 ms | **1.95 ms** | 3.16 ms | 2.55 ms |
| 131072 | | | | 9.04 ms | | | | | 32.1 ms | 26.2 ms | 6.95 ms | **6.2 ms** | 10.1 ms | 8.28 ms |
| 147456 | | | | 10.6 ms | | | | | 38.3 ms | 31.2 ms | 8.16 ms | **6.88 ms** | 11.5 ms | 9.7 ms |
| 1048576 | | | | 108 ms | | | | | 874 ms | 587 ms | 74.2 ms | **67.6 ms** | 110 ms | 94.1 ms |

- `encode_matrix` is the fastest encoder from 31 B to 8 KiB, 1.7–6.5× faster than the
  fastest other crate. With its fixed-size path it is 2.4× faster than `five8`'s
  fixed-size code at 32 bytes and 2.6× at 64.
- From 12 KiB `encode_ntt_cached` (768-chunk matrix leaves) leads: 1.13× faster than
  malachite's own conversion at 12 KiB, 1.28× at 32 KiB, 1.46–1.53× at 128–144 KiB and
  1.59× at 1 MiB.
- Against `bs58` our fastest is 27× faster at 31 B and up to 812× at 32 KiB;
  `encode_batched` alone is 24× (31 B) to 140× (32 KiB) faster.
- Karatsuba (`dc`) is 3.0× slower than the NTT at 32 KiB and 12.9× at 1 MiB.

**Code placement.** The same NTT code ran 6.6–7.5% faster in the binary before the
matrix encoder's merge (`a80b4fb`) than after it (back-to-back criterion runs at 32 KiB, 128 KiB and 1 MiB; malachite
and `dc-malachite`, as controls, within ±1%). With every loop aligned to 64 bytes
(`-C llvm-args=-align-loops=64`) both builds took the same time, at the slower level:
1.51 against 1.48 ms at 32 KiB and 77.6 against 77.4 ms at 1 MiB (128 KiB was noisy:
7.26 against 7.71 ms, with the second's interval 7.51–7.92). So `ntt-cached`'s numbers
depend on where its loops land by up to about 7%. The merge changed no NTT code.

### Encoding results on Zen 5

AMD EPYC 9B45 (Zen 5, AVX-512 IFMA) on a GCP VM, commit `49fb679`, rustc 1.99.0, default
flags (no `target-cpu`), the same command and sizes as above. A watcher on the VM saw only
the benchmark during the run. On IFMA CPUs `encode_matrix` converts up to 4 KiB itself and
calls `encode_ntt_cached` above. `five8` selects its AVX2 code at compile time, so with
default flags it ran its scalar code; our encoders detect AVX-512 at run time. The 32 and
64 byte rows are from a later run of every encoder at those sizes (commit `b2ffc77`,
default and native flags back to back; [Fixed sizes](#fixed-sizes-32-and-64-bytes)). Its
64-byte `matrix` cell is the kernels `encode_matrix` uses now, timed in that run as
`fixed-avx2-sums`.

| bytes | bs58 | basexx | five8 | malachite | fastest here | × bs58 | × best other |
|---:|---:|---:|---:|---:|:---|---:|---:|
| 31 | 817 ns | 190 ns |  | 161 ns | **54 ns** matrix | 15× | 2.98× |
| 32 | 901 ns | 192 ns | 56.5 ns | 162 ns | **25.5 ns** matrix | 35× | 2.22× |
| 64 | 3.91 µs | 437 ns | 121 ns | 312 ns | **73.3 ns** matrix | 53× | 1.65× |
| 128 | 16.6 µs | 1.01 µs |  | 714 ns | **168 ns** matrix | 99× | 4.25× |
| 256 | 69.7 µs | 3.63 µs |  | 2.06 µs | **396 ns** matrix | 176× | 5.20× |
| 512 | 282 µs | 8.89 µs |  | 4.72 µs | **1.1 µs** matrix | 257× | 4.31× |
| 1024 | 1.13 ms | 24.3 µs |  | 12.9 µs | **3.38 µs** matrix | 334× | 3.82× |
| 1875 | 3.79 ms | 61.8 µs |  | 31.8 µs | **10.3 µs** matrix | 370× | 3.10× |
| 4096 | 18.1 ms | 188 µs |  | 100 µs | **43.5 µs** matrix | 416× | 2.31× |
| 6144 | 40.7 ms | 320 µs |  | 186 µs | **82.2 µs** ntt-cached | 495× | 2.26× |
| 8192 | 72.4 ms | 548 µs |  | 283 µs | **123 µs** ntt-portable | 588× | 2.30× |
| 12288 | 163 ms | 943 µs |  | 522 µs | **238 µs** ntt-cached | 684× | 2.19× |
| 16384 | 292 ms | 1.64 ms |  | 804 µs | **365 µs** ntt-portable | 800× | 2.21× |
| 24576 | 652 ms | 2.75 ms |  | 1.54 ms | **713 µs** ntt-cached | 914× | 2.17× |
| 32768 | 1.16 s | 4.57 ms |  | 2.24 ms | **1.1 ms** ntt-cached | 1049× | 2.03× |
| 36864 |  |  |  | 2.6 ms | **1.41 ms** ntt-portable |  | 1.84× |
| 49152 |  |  |  | 3.81 ms | **2.17 ms** ntt-cached |  | 1.75× |
| 131072 |  |  |  | 12.1 ms | **9.17 ms** ntt-cached |  | 1.32× |
| 147456 |  |  |  | 14.1 ms | **10.8 ms** ntt-cached |  | 1.30× |
| 1048576 |  |  |  | 132 ms | **111 ms** dc-malachite |  | 1.19× |

Every encoder:

| bytes | bs58 | basexx | basexx-b | malachite | batched | matrix | matrix-portable | five8 | dc | dc-toom3 | ntt | ntt-cached | ntt-portable | dc-malachite |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 31 | 817 ns | 190 ns | 786 ns | 161 ns | 65.9 ns | **54 ns** | 62.1 ns | | | | | | | |
| 32 | 901 ns | 192 ns | 838 ns | 162 ns | 65.1 ns | **25.5 ns** | 25.5 ns | 56.5 ns | | | | | | |
| 64 | 3.91 µs | 437 ns | 3.42 µs | 312 ns | 138 ns | **73.3 ns** | 121 ns | 121 ns | | | | | | |
| 128 | 16.6 µs | 1.01 µs | 14 µs | 714 ns | 377 ns | **168 ns** | 255 ns | | | | | | | |
| 256 | 69.7 µs | 3.63 µs | 56.3 µs | 2.06 µs | 1.19 µs | **396 ns** | 697 ns | | | | | | | |
| 512 | 282 µs | 8.89 µs | 225 µs | 4.72 µs | 4.34 µs | **1.1 µs** | 2.24 µs | | | | | | | |
| 1024 | 1.13 ms | 24.3 µs | 901 µs | 12.9 µs | 16.2 µs | **3.38 µs** | 7.58 µs | | | | | | | |
| 1875 | 3.79 ms | 61.8 µs | 3.01 ms | 31.8 µs | 52.9 µs | **10.3 µs** | 24 µs | | | | | | | |
| 4096 | 18.1 ms | 188 µs | 14.4 ms | 100 µs | 247 µs | **43.5 µs** | 107 µs | | 56.2 µs | 56.3 µs | 68 µs | 44.7 µs | 44.6 µs | 140 µs |
| 6144 | 40.7 ms | 320 µs | 32.4 ms | 186 µs | 552 µs | 82.3 µs | 82.3 µs | | 101 µs | 100 µs | 112 µs | **82.2 µs** | 82.3 µs | 250 µs |
| 8192 | 72.4 ms | 548 µs | 57.6 ms | 283 µs | 979 µs | 124 µs | 123 µs | | 151 µs | 151 µs | 174 µs | 123 µs | **123 µs** | 364 µs |
| 12288 | 163 ms | 943 µs | 130 ms | 522 µs | 2.2 ms | | | | 286 µs | 285 µs | 310 µs | **238 µs** | 240 µs | 625 µs |
| 16384 | 292 ms | 1.64 ms | 233 ms | 804 µs | 3.94 ms | | | | 440 µs | 440 µs | 485 µs | 366 µs | **365 µs** | 876 µs |
| 24576 | 652 ms | 2.75 ms | 518 ms | 1.54 ms | 8.78 ms | | | | 842 µs | 841 µs | 892 µs | **713 µs** | 713 µs | 1.44 ms |
| 32768 | 1.16 s | 4.57 ms | 921 ms | 2.24 ms | 15.6 ms | | | | 1.29 ms | 1.29 ms | 1.48 ms | **1.1 ms** | 1.1 ms | 2.05 ms |
| 36864 | | | | 2.6 ms | | | | | 1.47 ms | 1.47 ms | 1.67 ms | 1.42 ms | **1.41 ms** | 2.4 ms |
| 49152 | | | | 3.81 ms | | | | | 2.53 ms | 2.48 ms | 2.76 ms | **2.17 ms** | 2.18 ms | 3.4 ms |
| 131072 | | | | 12.1 ms | | | | | 11.6 ms | 11 ms | 10.6 ms | **9.17 ms** | 27.3 ms | 10.4 ms |
| 147456 | | | | 14.1 ms | | | | | 13.2 ms | 12.8 ms | 11.7 ms | **10.8 ms** | 32.6 ms | 12.2 ms |
| 1048576 | | | | 132 ms | | | | | 316 ms | 252 ms | 124 ms | 111 ms | 484 ms | **111 ms** |

- `encode_matrix` is the fastest up to 4 KiB: 2.2× faster than scalar `five8` at 32
  bytes, 1.65× at 64, and 2.3–5.2× faster than the fastest other crate from 128 bytes.
- From 6 KiB `encode_ntt_cached` leads: about 2.2× faster than malachite's own conversion
  from 6 to 24 KiB, 2.03× at 32 KiB, 1.75× at 48 KiB, 1.30–1.32× at 128–144 KiB.
- Up to 48 KiB no transform runs on this CPU. Every product there is below
  `IFMA_THRESHOLD` (3072 limbs, or 2048 for a prepared power), so the NTT multiplier
  passes it to Karatsuba with the IFMA leaf. Counted with the IFMA thresholds and
  parameters forced on the M5: no forward transform up to 48 KiB, 93 at 128 KiB, 801 at
  1 MiB. So up to 48 KiB `ntt-cached` is Karatsuba with 96-chunk matrix leaves.
- For the same reason `ntt-cached` and `ntt-portable` run the same code up to 48 KiB and
  took the same time within 1%, as did `matrix` at 6–8 KiB (it calls `ntt-cached`). The
  bold among them is noise. From 128 KiB the transforms run, and `ntt-portable`'s kernels
  (baseline x86-64: no AVX2 or AVX-512) take 3.0× as long at 128–144 KiB and 4.3× at
  1 MiB.
- At 1 MiB `dc-malachite` (111.05 ms) and `ntt-cached` (111.40 ms) tie, 1.19× faster than
  malachite's own conversion. On the M5 `ntt-cached` was 1.39× faster than `dc-malachite`
  there. On x86_64 the NTT kernels are the portable code compiled for AVX-512; aarch64 has
  hand-written NEON ones. Hand-written AVX-512 kernels might close this gap; not tried.
- Against `bs58` our fastest is 15× faster at 31 B, 334× at 1 KiB and 1049× at 32 KiB.
- `dc` (plain `encode`: Karatsuba with 32-chunk Horner leaves) takes 1.17–1.26× as long
  as `ntt-cached` from 4 to 32 KiB. No transform runs there, so that gap is the leaves
  alone. At 1 MiB it takes 2.8× as long. On the M5, where transforms start at 128 limbs,
  it took 3.0× as long at 32 KiB and 12.9× at 1 MiB.

### Native CPU on Zen 5

The same VM ran the encode bench twice, back to back in one session: first with default
flags, then with `-C target-cpu=native` (znver5: AVX2, AVX-512F, AVX-512 IFMA). The code
was that of `49fb679`, with no LTO and no pinning. A watcher saw only the benchmark in
both windows (steal 0). The default run matched the run above within −2.0% to +3.3% in
all 176 cells, so the default tables above stand. The M5 numbers above are already
native: the local cargo config sets `target-cpu=apple-m4`.

With native flags (`× best other` is against the other crates built with native flags
too; the 32 and 64 byte rows are from the `b2ffc77` run, as above):

| bytes | bs58 | basexx | five8 | malachite | fastest here | × bs58 | × best other |
|---:|---:|---:|---:|---:|:---|---:|---:|
| 31 | 693 ns | 193 ns |  | 151 ns | **52.7 ns** matrix | 13× | 2.87× |
| 32 | 760 ns | 188 ns | 33.6 ns | 157 ns | **24.8 ns** matrix | 31× | 1.35× |
| 64 | 3.26 µs | 419 ns | 50.7 ns | 312 ns | **75.0 ns** matrix | 43× | 0.68× |
| 128 | 13.7 µs | 1.01 µs |  | 715 ns | **175 ns** matrix | 78× | 4.09× |
| 256 | 59.7 µs | 3.94 µs |  | 2.13 µs | **420 ns** matrix | 142× | 5.07× |
| 512 | 242 µs | 9.44 µs |  | 4.71 µs | **1.16 µs** matrix | 208× | 4.05× |
| 1024 | 974 µs | 25.6 µs |  | 13.8 µs | **3.42 µs** matrix | 285× | 4.04× |
| 1875 | 3.38 ms | 65.5 µs |  | 37.1 µs | **10.7 µs** matrix | 317× | 3.48× |
| 4096 | 15.7 ms | 191 µs |  | 125 µs | **44.3 µs** ntt-cached | 354× | 2.83× |
| 6144 | 34.4 ms | 332 µs |  | 224 µs | **78.8 µs** ntt-portable | 437× | 2.84× |
| 8192 | 59.9 ms | 537 µs |  | 354 µs | **118 µs** matrix-portable | 509× | 3.01× |
| 12288 | 135 ms | 929 µs |  | 642 µs | **225 µs** ntt-cached | 598× | 2.85× |
| 16384 | 239 ms | 1.59 ms |  | 998 µs | **344 µs** ntt-cached | 697× | 2.90× |
| 24576 | 541 ms | 2.72 ms |  | 1.85 ms | **665 µs** ntt-cached | 813× | 2.78× |
| 32768 | 958 ms | 4.5 ms |  | 2.72 ms | **1.03 ms** ntt-portable | 932× | 2.65× |
| 36864 |  |  |  | 3.18 ms | **1.32 ms** ntt-cached |  | 2.42× |
| 49152 |  |  |  | 4.49 ms | **1.99 ms** ntt-cached |  | 2.26× |
| 131072 |  |  |  | 14.1 ms | **8.63 ms** ntt-portable |  | 1.64× |
| 147456 |  |  |  | 16.8 ms | **10.7 ms** ntt-cached |  | 1.58× |
| 1048576 |  |  |  | 148 ms | **107 ms** dc-malachite |  | 1.38× |

Every encoder, native flags:

| bytes | bs58 | basexx | basexx-b | malachite | batched | matrix | matrix-portable | five8 | dc | dc-toom3 | ntt | ntt-cached | ntt-portable | dc-malachite |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 31 | 693 ns | 193 ns | 765 ns | 151 ns | 65.8 ns | **52.7 ns** | 52.7 ns | | | | | | | |
| 32 | 760 ns | 188 ns | 833 ns | 157 ns | 65.7 ns | **24.8 ns** | 24.8 ns | 33.6 ns | | | | | | |
| 64 | 3.26 µs | 419 ns | 3.35 µs | 312 ns | 138 ns | 75 ns | 127 ns | **50.7 ns** | | | | | | |
| 128 | 13.7 µs | 1.01 µs | 14 µs | 715 ns | 372 ns | **175 ns** | 277 ns | | | | | | | |
| 256 | 59.7 µs | 3.94 µs | 58.5 µs | 2.13 µs | 1.22 µs | **420 ns** | 852 ns | | | | | | | |
| 512 | 242 µs | 9.44 µs | 230 µs | 4.71 µs | 4.2 µs | **1.16 µs** | 3.18 µs | | | | | | | |
| 1024 | 974 µs | 25.6 µs | 927 µs | 13.8 µs | 15.2 µs | **3.42 µs** | 11.4 µs | | | | | | | |
| 1875 | 3.38 ms | 65.5 µs | 3.15 ms | 37.1 µs | 50.8 µs | **10.7 µs** | 40.4 µs | | | | | | | |
| 4096 | 15.7 ms | 191 µs | 14.7 ms | 125 µs | 237 µs | 47.4 µs | 204 µs | | 56.3 µs | 56.2 µs | 67.6 µs | **44.3 µs** | 44.8 µs | 172 µs |
| 6144 | 34.4 ms | 332 µs | 32.8 ms | 224 µs | 514 µs | 79.3 µs | 79 µs | | 97.6 µs | 97.5 µs | 107 µs | 79 µs | **78.8 µs** | 296 µs |
| 8192 | 59.9 ms | 537 µs | 56.4 ms | 354 µs | 905 µs | 118 µs | **118 µs** | | 145 µs | 145 µs | 165 µs | 118 µs | 118 µs | 421 µs |
| 12288 | 135 ms | 929 µs | 127 ms | 642 µs | 2.04 ms | | | | 271 µs | 271 µs | 290 µs | **225 µs** | 225 µs | 684 µs |
| 16384 | 239 ms | 1.59 ms | 225 ms | 998 µs | 3.6 ms | | | | 417 µs | 415 µs | 456 µs | **344 µs** | 344 µs | 959 µs |
| 24576 | 541 ms | 2.72 ms | 508 ms | 1.85 ms | 8.12 ms | | | | 786 µs | 789 µs | 835 µs | **665 µs** | 665 µs | 1.53 ms |
| 32768 | 958 ms | 4.5 ms | 902 ms | 2.72 ms | 14.4 ms | | | | 1.22 ms | 1.21 ms | 1.4 ms | 1.03 ms | **1.03 ms** | 2.14 ms |
| 36864 | | | | 3.18 ms | | | | | 1.38 ms | 1.36 ms | 1.55 ms | **1.32 ms** | 1.32 ms | 2.46 ms |
| 49152 | | | | 4.49 ms | | | | | 2.34 ms | 2.28 ms | 2.54 ms | **1.99 ms** | 1.99 ms | 3.45 ms |
| 131072 | | | | 14.1 ms | | | | | 10.8 ms | 10.1 ms | 10.1 ms | 8.63 ms | **8.63 ms** | 10.5 ms |
| 147456 | | | | 16.8 ms | | | | | 12.3 ms | 11.6 ms | 11.1 ms | **10.7 ms** | 10.9 ms | 12.4 ms |
| 1048576 | | | | 148 ms | | | | | 292 ms | 226 ms | 123 ms | 110 ms | 108 ms | **107 ms** |

Native time over default time, per encoder over its sizes (below 1: faster with native
flags):

| encoder | sizes | min | median | max |
|:---|---:|---:|---:|---:|
| five8 | 2 | 0.438 | 0.508 | 0.579 |
| bs58 | 15 | 0.823 | 0.844 | 0.892 |
| dc | 12 | 0.910 | 0.934 | 0.983 |
| dc-toom3 | 12 | 0.895 | 0.936 | 0.985 |
| ntt | 12 | 0.907 | 0.935 | 1.011 |
| ntt-cached | 12 | 0.910 | 0.941 | 0.996 |
| batched | 15 | 0.920 | 0.954 | 1.015 |
| basexx | 15 | 0.976 | 0.997 | 1.070 |
| basexx-b | 15 | 0.974 | 0.998 | 1.044 |
| matrix | 11 | 0.942 | 1.011 | 1.060 |
| dc-malachite | 12 | 0.963 | 1.044 | 1.226 |
| malachite | 20 | 0.937 | 1.177 | 1.249 |
| matrix-portable | 11 | 0.715 | 1.075 | 1.885 |
| ntt-portable | 12 | 0.222 | 0.928 | 1.003 |

- `five8` selects its AVX2 code at compile time, so only the native build uses it. With
  its fixed-size path `encode_matrix` is still 1.35× faster at 32 bytes (24.8 against
  33.6 ns), but `five8` is 1.48× faster at 64 (50.7 against 75.0 ns). Before that path
  (`49fb679`) `five8` won both: 33.6 and 52.5 ns against 51.2 and 82.6.
- `encode_matrix` does not change (median +1%, within ±6%). Its panel kernels are
  selected at run time (AVX2 here) in both builds. It stays the fastest at 31 bytes and
  from 128 to 1875 bytes. At 4 KiB, its hand-off size on IFMA CPUs, `ntt-cached` was 7% faster with
  native flags (44.3 against 47.4 µs) and tied it with default flags (44.5 against
  44.9 µs). The cap is unchanged: this is one run, and `matrix` at 4 KiB moved 3.3%
  between the two default runs.
- The divide-and-conquer encoders and `encode_batched` take about 5–7% less time with
  native flags.
- malachite's own conversion takes 18% more time with native flags (median; up to 25%),
  and `dc-malachite`, which multiplies with malachite, 4%. Not investigated. So the
  `× best other` column is higher with native flags partly because malachite got slower.
  At 12 KiB native `ntt-cached` (225 µs) is 2.85× faster than native malachite (642 µs)
  and 2.33× faster than default-flag malachite (524 µs). With default flags on both sides
  it was 2.18×.
- `ntt-portable`'s kernels compile for AVX-512 with native flags, and it then matches
  `ntt-cached` (8.63 against 8.63 ms at 128 KiB, 108 against 110 ms at 1 MiB). With
  default flags it took 3.0–4.4× as long from 128 KiB.
- `matrix-portable` (the plain-Rust matrix kernel) takes 15–28% less time at 31–64 bytes
  with native flags, but 1.24–1.88× as long from 256 bytes to 4 KiB. `encode_matrix`
  does not use it on x86_64.
- At 1 MiB `dc-malachite` (107 ms), `ntt-portable` (108 ms) and `ntt-cached` (110 ms)
  are within 3%: a tie.

### Tuning

All in one binary (`benches/encode_tune.rs`: interleaved rounds, the minimum time per
call of each configuration), M5 Pro on AC power.

- **Threshold.** Divide and conquer beats `encode_batched` from about 3 KiB with
  malachite's multiplication (57.4 against 58.6 µs at 3 KiB), 4 KiB with the NTT (a tie;
  117.6 against 130.6 µs at 4.5 KiB) and 5 KiB with Karatsuba (a tie; 207.9 against
  224.2 µs at 6 KiB). So each backend has its own `ENCODE_THRESHOLD`.
- **Leaf size** matters little: within about 5% from 32 to 256 chunks. 96 is best with
  Karatsuba and malachite; with the NTT, 32 is up to 4% faster from 8 KiB up and 1–2%
  slower at 4–6 KiB, so the NTT uses 32.
- **Where the time goes** (temporary timers, not in the repo; `ntt-cached` and
  `dc-malachite`):

  | input   | reciprocals | divisions | leaves | power table |
  |--------:|------------:|----------:|-------:|------------:|
  | 4 KiB   | 27%         | 47%       | 19%    | 4%          |
  | 32 KiB  | 27–28%      | 58–59%    | 5–9%   | 3%          |
  | 1 MiB   | 20–23%      | 66–72%    | 3–5%   | 3%          |

  The top division, and its reciprocal, serve one node but cost about as much as a whole
  level below. (Measured before the wrapped `q·pn`.)
- **Wrapped products.** `encode_ntt_cached` with the plain products, with a wrapped
  `q·pn`, and with also a wrapped `a·x_h` in each Newton step (`t = β^(n+h) - a·x_h`
  lies in `(-2 β^n, 2 β^n]`, so its residue modulo `β^L - 1` for `L >= n + 2` gives it
  with its sign). One binary with a temporary switch, 21 interleaved rounds (minimum
  times; the ratios are medians of the per-round ratios). No other benchmark ran; a
  backup process used one core:

  | input   | plain    | wrapped `q·pn` | ratio | also `a·x_h` | ratio |
  |--------:|---------:|---------------:|------:|-------------:|------:|
  | 8 KiB   | 269 µs   | 254 µs         | 0.945 | 255 µs       | 0.945 |
  | 16 KiB  | 644 µs   | 603 µs         | 0.937 | 603 µs       | 0.936 |
  | 32 KiB  | 1.46 ms  | 1.35 ms        | 0.925 | 1.35 ms      | 0.924 |
  | 48 KiB  | 2.39 ms  | 2.27 ms        | 0.949 | 2.21 ms      | 0.922 |
  | 128 KiB | 7.39 ms  | 6.72 ms        | 0.908 | 6.71 ms      | 0.908 |
  | 144 KiB | 8.64 ms  | 7.73 ms        | 0.896 | 7.74 ms      | 0.895 |
  | 1 MiB   | 81.5 ms  | 72.6 ms        | 0.891 | 72.6 ms      | 0.890 |

  An earlier run, with another benchmark on the machine, gave the same ratios within
  0.01. The wrapped `q·pn` is kept. The wrapped `a·x_h` gained 3% at 48 KiB and nothing
  elsewhere: at the reciprocals' lengths, the next power of two is rarely shorter than
  the pieces of the full product. It is not kept.

Tried and not kept:

- **A sign test in `reduce`** (`(t - BASE) as i64 < 0` instead of `overflowing_sub`):
  23–30% slower.
- **Schoolbook division for small powers** (Knuth's Algorithm D with a Möller–Granlund
  3-by-2 quotient digit, no reciprocal), as malachite does below 50 limbs, with
  `dc-malachite`: below 256 limbs it was 6% faster at 3 KiB, within 1% at 4 KiB and 2–5%
  slower from 8 KiB; smaller cutoffs were within 2% everywhere. The reciprocals that
  matter belong to the large powers.

### Possible next steps

- **Wrapped products of other lengths.** `L` is a power of two, so a `q·pn` just above
  one (and every Newton `a·x_h`) gains little. Residues modulo `β^a - 1` and `β^b + 1`
  (the negacyclic transforms the plans already use) put together by an integer CRT would
  allow lengths like `3·2^k`. GMP's `mpn_mulmod_bnm1` splits `β^(2n) - 1` into
  `(β^n - 1)(β^n + 1)` in a similar way. Not tried.
- **Scaled remainder tree** (Bouvier–Zimmermann): one reciprocal for the whole number,
  then one product per node (a middle product, which a cyclic NTT gives) instead of two,
  and no reciprocal per level. Needs a proved error bound and an exact fix-up.
- **Four passes per sweep** in `encode_batched`'s kernel.

### `encode_matrix`: a matrix of powers

Firedancer's encode, for any length up to 8192 bytes. It mirrors `decode_matrix`:

- Split the bytes into `u32` words `w_i`, least significant first: the value is
  `Σ w_i · 2^(32i)`.
- A table holds `2^(32i)` in base 58^4. Output column `j` (four characters) is the column
  sum `Σ w_i · T[i][j]`, kept in `u64`. The decoder's panel kernels compute it (NEON
  `umlal`; AVX2 or SSE4.1 on x86_64): `u32` words times entries below 58^4 have the
  decoder's product bound, `(2^32 - 1)(58^4 - 1)`.
- `2^(32i)` is not a multiple of 58, so every row starts at column 0. The decoder's rows
  start further up, so for the same byte count the encoder's table, and its work, is
  about 1.2× the decoder's.

Carries:

- **A pass every 47 panels** (376 rows) moves each column's multiples of 58^4 into the
  next column: `c_j ← c_j mod 58^4 + ⌊c_(j-1) / 58^4⌋`. Each carry depends only on its own
  column, so the pass is not a carry chain. Afterwards a column is at most
  `58^4 - 1 + 2^40.57`, and 376 more products leave it 2^57.25 below 2^64; 384 could
  overflow. `const` asserts in `encode/matrix/table.rs` check both. On all-0xFF input at
  8 KiB, which maximizes every column at once, the largest sum is 2^63.15.
- **The final pass** does that step and an exact carry in one loop. After the step, the
  quotient by 58^4 is below 2^18, so the remainder plus the incoming carry is below
  `2 · 58^4`: one compare finishes the division, and only that compare and an increment
  are on the carry chain.
- Every column is a non-negative part of the value, so columns at or above its length
  stay zero. The final pass stops at `⌈max_encoded_len / 4⌉ + 1` columns, not at the last
  panel's padded width, which matters only for small inputs.

Characters: four per column. On aarch64 a NEON kernel converts four columns per step:
`c div 3364` and `x div 58` by fixed-point multiplies (`sqdmulh`; both constants were
checked over all inputs), one `tbl` into the alphabet and one `tbl` to put the columns
top first. Elsewhere a scalar loop uses a table of the 3364 character pairs.

The table is built on first use and kept, in five size classes: 47 KiB (512 bytes),
181 KiB (1 KiB), 712 KiB (2 KiB), 2.8 MiB (4 KiB) and 11 MiB (8 KiB). A smaller class is
a prefix of a larger one, so an input uses the largest class already built, and only one
table needs to stay in cache. Below `MATRIX_MIN = 22` bytes (after leading zeros)
`encode_matrix` calls `encode_batched`, and above 8192 it calls `encode_ntt_cached` (see
**The cap** below). `encode_matrix_portable` is the same algorithm with plain-Rust
kernels. Up to 8 KiB a call takes about 30 KiB of stack and allocates nothing.

Apple M5 Pro on AC power, criterion medians from one run (`cargo bench --bench encode --
--warm-up-time 1 --measurement-time 3 'encode/(batched|matrix|matrix-portable|malachite|five8)/'`,
same inputs as above, up to 8 KiB; other applications ran, a load average of about 4.5):

| bytes | batched | matrix       | matrix-portable | malachite | five8   | speedup |
|------:|--------:|-------------:|----------------:|----------:|--------:|--------:|
| 31    | 34.7 ns | **29.4 ns**  | 35.5 ns         | 138 ns    |         | 1.18×   |
| 32    | 36.4 ns | **30.2 ns**  | 36.8 ns         | 134 ns    | 31.1 ns | 1.21×   |
| 64    | 86.4 ns | **51.0 ns**  | 69.4 ns         | 231 ns    | 83.4 ns | 1.69×   |
| 128   | 211 ns  | **112 ns**   | 154 ns          | 487 ns    |         | 1.89×   |
| 256   | 622 ns  | **248 ns**   | 414 ns          | 1.48 µs   |         | 2.51×   |
| 512   | 2.10 µs | **604 ns**   | 1.28 µs         | 3.60 µs   |         | 3.48×   |
| 1024  | 7.48 µs | **1.98 µs**  | 4.59 µs         | 9.86 µs   |         | 3.77×   |
| 1875  | 24.1 µs | **5.96 µs**  | 14.4 µs         | 23.7 µs   |         | 4.04×   |
| 4096  | 107 µs  | **29.1 µs**  | 65.9 µs         | 74.7 µs   |         | 3.68×   |
| 6144  | 237 µs  | **65.2 µs**  | 148 µs          | 144 µs    |         | 3.64×   |
| 8192  | 421 µs  | **119 µs**   | 262 µs          | 207 µs    |         | 3.55×   |

(speedup: `encode_matrix` against `encode_batched`.)

- `encode_matrix` is the fastest general encoder here from 22 bytes to 8 KiB: 1.2× faster
  than `encode_batched` at 31–32 B, 1.7–2.5× at 64–256 B and 3.5–4.0× from 512 B. It is
  2.6× faster than malachite's own conversion at 4 KiB, 2.2× at 6 KiB and 1.7× at
  8 KiB.
- At 32 bytes `encode_matrix` and `five8`'s fixed-size code tie (3% apart; 29.9 against
  30.4 ns in a second run); at 64 bytes `encode_matrix` is 1.6× faster. (Before the
  fixed-size path: see [Fixed sizes](#fixed-sizes-32-and-64-bytes).)
- Unlike decoding, the plain-Rust kernels already beat the quadratic loop (1.25–1.67×
  from 64 B): `encode_batched` divides by 58^10 in each limb step, where the matrix only
  multiplies and adds. NEON adds another 1.7–2.4× from 256 B.
- Above 8 KiB `encode_matrix` calls `encode_ntt_cached`: see **The cap** below.

Where the time goes (temporary timers, not in the repo): the panels, with parsing and
setup, take about 95% at 4 KiB, 83% at 1 KiB and 55% at 256 B. Most of the rest is the
final carry pass (1.05 µs, 0.27 µs and 80 ns); the characters take 0.28 µs, 70 ns and
18 ns.

**Cold caches.** Single calls after touching every line of 64 MiB
(`SIZES=64,256,512,1024,1875,4096,6144,8192 cargo bench --bench encode_matrix_tune --
cold`; medians of three runs, six up to 512 B, each the median of 31 calls; the timer
ticks every 41.7 ns; µs):

| bytes | batched | dc   | ntt-cached | malachite | matrix   | speedup |
|------:|--------:|-----:|-----------:|----------:|---------:|--------:|
| 64    | **0.33** |      |            |           | 0.42     | 0.80×   |
| 256   | **0.88** |      |            |           | 0.98     | 0.89×   |
| 512   | 2.36    |      |            |           | **1.73** | 1.36×   |
| 1024  | 7.58    |      |            |           | **4.13** | 1.84×   |
| 1875  | 23.8    |      |            |           | **10.8** | 2.20×   |
| 4096  | 104     | 103  | 104        | 101       | **46.2** | 2.18×   |
| 6144  | 226     | 230  | 198        | 164       | **101**  | 1.63×   |
| 8192  | 401     | 363  | 284        | 239       | **176**  | 1.36×   |

(speedup: `encode_matrix` against the fastest other column.)

Cold, a call at 4 KiB takes about 18 µs longer and one at 8 KiB about 60 µs longer, as
the table (2.8 and 11 MiB) comes from memory. The matrix still wins from about 512 B,
1.4× ahead of malachite at 8 KiB and 2.1× ahead of `encode`, which `encode_matrix`
called there before: `encode_batched` and the divide-and-conquer encoders are slow
enough that loading the table pays. (`decode_matrix` lost cold at every size.) For
one-off encodes of 256 B or less, `encode_batched` ties or wins; single calls that
short vary by up to ±50% between runs (0.72–1.19× at 256 B). The first call in a class
also builds the table: about 48 µs, 0.13 ms, 0.40 ms, 1.7 ms and 6.1 ms for the five
classes (`-- first`, medians of three runs). So against `encode`, a process gains at
4–8 KiB only after about 30 (8 KiB) to 65 (4.5 KiB) warm calls.

**Threshold.** `benches/encode_matrix_tune.rs` runs the matrix path at every size
against `encode_batched` (the minimum of 31 interleaved rounds; two runs). Below 22 bytes
the matrix ties or loses (0.67–1.01×). From 22 to 32 bytes it wins by 16–27%, as
`encode_batched` needs a fourth 7-byte step there. At 33–35 bytes it loses 3–6%: the
second panel costs about 10 ns. It wins at every size measured from 36 bytes.

**The cap.** Each class doubles the bytes and quadruples the table and the work. With
four classes (4 KiB), `encode_matrix` called `encode` above the cap. Each build in one
binary, the two builds run back to back (`SIZES=4096,4097,… cargo bench --bench
encode_matrix_tune`; the minimum of two runs, each the minimum of 31 interleaved rounds;
µs). `4 KiB cap` is the previous build; the other columns are this build. `dc` and
`ntt-cached` run `encode_batched` up to 5120 and 4096 bytes.

| bytes | dc   | ntt-cached | dc-malachite | malachite | 4 KiB cap | 8 KiB cap | speedup |
|------:|-----:|-----------:|-------------:|----------:|----------:|----------:|--------:|
| 4096  | 102  | 102        | 88.4         | 70.9      | **27.4**  | 27.6      | 0.99×   |
| 4097  | 102  | 101        | 88.3         | 71.1      | 101       | **28.0**  | 3.62×   |
| 4608  | 128  | 120        | 105          | 87.7      | 128       | **35.4**  | 3.61×   |
| 5120  | 157  | 137        | 127          | 100       | 157       | **43.9**  | 3.58×   |
| 6144  | 207  | 176        | 165          | 132       | 207       | **63.2**  | 3.28×   |
| 7168  | 272  | 208        | 199          | 166       | 269       | **85.8**  | 3.14×   |
| 8192  | 334  | 255        | 239          | 196       | 334       | **112**   | 2.97×   |
| 8193  | 335  | 255        | 237          | **196**   | 334       | 255       | 1.31×   |
| 8704  | 370  | 272        | 261          | **220**   | 366       | 274       | 1.34×   |
| 12288 | 653  | 426        | 429          | **369**   | 649       | 426       | 1.52×   |
| 16384 | 1042 | 612        | 600          | **555**   | 1039      | 618       | 1.68×   |

(speedup: the 8 KiB cap against the 4 KiB cap.)

- The 8 KiB class, 11 MiB, fits the 16 MiB L2 of the performance cores: from 4 to 8 KiB
  the time grows 4.07×, as the work does (4×). The matrix stays the fastest: 2.5×
  faster than malachite's own conversion at 4.5 KiB, 2.1× at 6 KiB, 1.75× at 8 KiB.
- The cliff moves to 8 KiB and shrinks from 3.7× (27.4 to 101 µs at 4097 bytes) to 2.3×
  (112 to 255 µs at 8193).
- Above the cap `encode_ntt_cached` replaces `encode`: 1.3–1.7× faster than `encode`
  from 8 to 16 KiB, and ahead of this crate's other encoders from 48 KiB
  ([Encoding results](#encoding-results)). malachite's own conversion is 1.1–1.3×
  faster than it from 8 to 16 KiB, but it is not this crate's code.
- **The fallback depends on code placement.** Built with loops aligned to 64 bytes
  (`-C llvm-args=-align-loops=64`, two runs, back to back with this build),
  `ntt-cached` took about 10% longer from 8 to 32 KiB (279 µs at 8193 B, a 2.5× cliff),
  while `dc-malachite` moved by at most 1.5%. So from 8 to 32 KiB `dc-malachite` is
  2–17% faster than `ntt-cached` in that build, and between 5% slower and 7% faster in
  this one.
- A sixth class (16 KiB) would take 44 MiB, beyond the L2. Even at L2 speed it would
  beat malachite at 16 KiB by at most 1.2× (4 × 112 against 555 µs). Not tried.
- The price: 11 MiB more memory, a 6 ms first call at 4–8 KiB (see **Cold caches**), and
  about 30 KiB of stack in place of 15.

**Stack alignment.** With the fifth class the scratch on the stack doubled, and the
panels took 7–8% longer at 1 KiB and 12% at 1875 B, at all of 32 stack depths (a caller
padded in 16-byte steps; four builds back to back). The new frame put both arrays at
`sp + 8`, so the column sums, loaded and stored in 16-byte vectors, were 8 bytes off a
16-byte boundary. `Scratch` is now `#[repr(C, align(64))]`, which gives the old times
back (within 1%). The offset within a 64-byte line made no difference. With loops
aligned to 64 bytes in both builds, which rules out code placement, the misaligned
scratch still cost 7% at 1 KiB, 10% at 1875 B, nothing at 4 KiB and 2–3% at 8 KiB.
`decode_matrix`'s `Scratch` is not aligned either: it gets whatever its frame gives.

Tried and not kept (interleaved A/B in one binary, AC power, another session building):

- **A NEON kernel for two panels at once** (16 rows per accumulator load and store): 2–3%
  slower end to end from 256 B to 4 KiB, although a loop of panels alone ran 6–23%
  faster with it.
- **Two accumulator chains per block** (rows 0–3 and 4–7, added at the end): 9% less
  panel time at 256 B, none at 1 KiB and 2% at 4 KiB. The kernel is limited by
  throughput, not by the latency of its `umlal` chains.

Since then:

- **Matrix leaves** are in the divide-and-conquer encoders: see
  [Matrix leaves](#matrix-leaves).
- **The fallback above 8 KiB** stays `encode_ntt_cached`, now with matrix leaves. In one
  binary with loops aligned, `encode_malachite` with matrix leaves won at 16 KiB by 4%,
  lost at 24 KiB by 4%, and tied at 12 and 32 KiB.
- **x86_64** has timings (Zen 5, [Matrix leaves](#matrix-leaves)). The characters are
  still scalar there.

Possible next steps:

- **SME** on M4 and later: outer products into the ZA tiles could raise the
  multiply-add throughput that bounds the panels.

### Matrix leaves

The divide-and-conquer encoders can convert their leaves with `encode_matrix`'s
conversion instead of `encode_batched`'s kernel (`Leaves::Matrix`). A leaf of `n` chunks
must write exactly `10n` characters, which four-character columns do not divide. So the
top column gives only its last few characters (the others must be zero digits), or gets
`'1'`s before it. As in the decoder, only the encoders that cache tables anyway use
them: `encode_ntt_cached` and `encode_ntt_portable`. The others keep Horner leaves and
no state between calls. An input that fits in one leaf is converted as a whole. Matrix
leaves can be up to `MATRIX_LEAF = 1024` chunks (8 KiB, the matrix's largest table),
where Horner leaves stop at 256.

On the M5 Pro (one binary with loops aligned to 64 bytes, minimum of 11 interleaved
rounds, µs; `h` Horner leaves, `m` matrix leaves, the number the leaf size in chunks):

| bytes   | ntt-cached /32h | /256h | /256m | /768m      | /1024m     |
|--------:|----------------:|------:|------:|-----------:|-----------:|
| 8 KiB   | 271.9           | 279.7 | 240.3 | 198.2      | 197.6      |
| 16 KiB  | 648.6           | 659.1 | 577.1 | 519.0      | 517.8      |
| 32 KiB  | 1447            | 1489  | 1331  | **1227**   | 1230       |
| 128 KiB | 7047            | 7260  | 6819  | 6400       | **6376**   |
| 1 MiB   |                 |       | 72371 | **69380**  | 69634      |

(Two aligned sweeps: the Horner columns from the first, the matrix columns from a second
after the change that allowed leaves above 256. `/256m` was in both: 238.4 against
240.3 µs at 8 KiB, 1325 against 1331 µs at 32 KiB.)
So `encode_ntt_cached` uses 768-chunk matrix leaves: 4–8% faster than 256 from 32 KiB
to 1 MiB, and 15% faster than 32-chunk Horner leaves at 32 KiB. 1024 was within 1% at
the large sizes but 3–4% slower at 12 and 24 KiB.

On Zen 5 (EPYC 9B45, the remote VM; default flags, a single sweep of leaves up to 256),
matrix leaves took less time than Horner leaves for every backend and leaf size: from 2%
(32-chunk leaves at 128 KiB) to 59% (256-chunk leaves at 2 KiB, 17.4 against 42.8 µs),
so `Leaves::BEST` is Matrix on x86_64 too. With the AVX-512 IFMA leaf, Karatsuba is
cheap enough that plain `encode` wants 32-chunk leaves there (96 took 13–29% longer at
2–8 KiB). The NTT encoder with matrix leaves tied `encode_matrix` at 4 KiB and was 33%
faster at 8 KiB, so on IFMA CPUs `encode_matrix` hands over above 4 KiB.

A second Zen 5 run (default flags, minimum of interleaved rounds, µs) swept the larger
matrix leaves and the thresholds:

| bytes | ntt-cached /96m | /256m  | /768m  | dc /96m   | batched | dc /32h | malachite (`dc-malachite`) /96h |
|------:|----------------:|-------:|-------:|----------:|--------:|--------:|--------------------------------:|
| 512   |                 |        |        |           | 4.3     | 4.1     | 4.3                             |
| 768   |                 |        |        |           | 9.3     | 6.7     | 10.3                            |
| 1024  |                 |        |        |           | 16.3    | 9.4     | 16.8                            |
| 1536  |                 |        |        |           | 35.9    | 15.3    | 30.5                            |
| 6144  | 89.1            | 93.3   | 123.6  | **81.2**  |         |         |                                 |
| 8192  | 123.6           | 124.8  | 195.2  | **123.5** |         |         |                                 |
| 32768 | 1104.5          | 1105.0 | 1277.3 | **1099.0**|         |         |                                 |
| 1 MiB | 111270          | **111029** | 116000 | 284653 |      |         |                                 |

- On x86 the caching NTT encoders use 96-chunk matrix leaves: 768 took 39% longer at
  6 KiB and 58% at 8 KiB, where the whole input is one leaf (the x86 matrix kernels are
  slower than NEON's). So `CACHED_LEAF` is 768 on aarch64 and 96 on x86_64.
- With the IFMA leaf, divide and conquer with 32-chunk Horner leaves ties
  `encode_batched` at 512 bytes and takes 26–28% less time at 768, so the IFMA
  thresholds are 512 for `encode` and the NTT encoders, and 1280 for `encode_malachite`
  (3% slower at 1 KiB, 15% faster at 1.5 KiB).
- Karatsuba with matrix leaves (`dc /96m`) was fastest from 6 to 32 KiB, by up to 9%,
  but 2.5× slower than the NTT at 1 MiB. Plain `encode` keeps Horner leaves, and the
  fallback above `encode_matrix`'s cap stays the NTT encoder. Up to 48 KiB no product
  reaches the transforms on IFMA CPUs (see "Encoding results on Zen 5"), so there
  `ntt-cached /96m` and `dc /96m` compute the same thing: the 9% at 6 KiB is not the
  multiplier.
- `MATRIX_MIN = 22` holds: the matrix lost at 8 and 12 bytes (0.74–0.84×), was within
  6% at 16, 20 and 28, and won at 24 and 32 (1.10× and 1.21×). The differences are a few
  nanoseconds.

### Fixed sizes: 32 and 64 bytes

`encode_matrix` sends exactly 32 or 64 bytes (keys, hashes, signatures) to fixed-size
code (`encode/matrix/fixed.rs`): the same matrix, fixed at compile time. Firedancer's
encoder (`five8`) works the same way in base 58^5; this one keeps base 58^4 columns.

- **The table is a constant**: `2^(32k)` for 8 or 16 words in 12 or 24 columns (44 and
  88 characters, padded to groups of four). No table to build or look up, no scratch to
  clear, no result to trim. Const asserts check that a column sum of 16 words plus a
  carry fits in `u64` (the sums stay below 2^60, so unlike `five8` at 64 bytes nothing is
  reduced before the carry pass) and that every value fits in 11 or 22 columns.
- **Column sums**: NEON `umlal`, four columns per block, from the first row that is not
  zero there; a row that is zero in a block's upper two columns adds to the lower two
  only. On x86_64 AVX2 `vpmuludq` for 64 bytes, picked at run time with
  `is_x86_feature_detected!`, and plain Rust for 32 bytes (see Zen 5 below); other
  targets run plain Rust. As constants, every pair of table entries took its own
  load (`adrp`, `ldr d`), one per `umlal`, so the NEON kernel reads the table through
  `black_box`:
  11% faster at 64 bytes (35.4 against 39.6 ns), 4% slower at 32 (16.5 against 15.9;
  both with the general path's carry); skipping the zero half-rows then took off 1–2%.
- **The carry is serial.** About half of a call is the carry pass: with it removed
  (wrong output, timing only) a call took 5.8 against 11.4 ns at 32 bytes and 12.7
  against 27.1 ns at 64. The general path's `normalize` (two divisions per column, off
  the chain) took 1.33× as long as a plain serial carry at 32 bytes (16.1 against
  12.1 ns) and 1.15–1.24× at 64 (33.7–35.8 against 28.9–29.2 ns, two runs), though its
  chain is shorter. The chain keeps only the quotients: `t += c; c = t / 58^4`. The
  column, `t - 58^4·c`, is below 2^32, so the digit kernels compute it modulo 2^32 from
  the low halves (NEON four columns per instruction): 5–7% faster again on the M5 (11.4
  against 12.1 ns, 27.1 against 28.9–29.2).
- **Characters**: on aarch64 the general path's NEON kernel with that subtraction in
  front. Elsewhere, x86_64 included, the scalar kernel with the table of 3364 character
  pairs: an AVX2 kernel (`c div 3364` by `vpmuludq`, the pairs by `vpmulhuw`, the
  alphabet by compares) was slower on Zen 5 and was removed.
- **Output**: the leading `'1'`s of the characters are dropped and one per leading zero
  byte put back. A result is 32–44 or 64–88 characters, so two overlapping fixed-size
  copies write exactly it.
- The NEON path has no branch or table lookup that depends on the input except through
  the result's length (its leading zero bytes and digit count). The x86_64 and plain-Rust
  paths, like the general one, index the table of character pairs by digit pairs, so
  which cache lines they touch depends on the input. Not checked with tools.

Apple M5 Pro, AC power, `cargo bench --bench encode -- --warm-up-time 1
--measurement-time 3 'encode/[^/]+/(31|32|64)$'`: the base commit (`e1a9f13`), `be91f3a`
(the aarch64 code has not changed since), then the base commit again, back to back. During the second base run's 64-byte
row the machine was busier: every encoder there took 4–22% longer than in the first
(`matrix` 22%).
`matrix-general` is `encode_matrix` without the fixed-size path, in the new binary. ns:

| bytes | five8 | matrix, base | matrix | matrix-general | matrix-portable, base | matrix-portable | speedup |
|------:|------:|-------------:|-------:|---------------:|----------------------:|----------------:|--------:|
| 31    |       | 31.5, 31.8   | 31.6   |                | 39.0, 39.4            | 39.8            | 1.00×   |
| 32    | 31.1  | 31.4, 32.0   | **12.7** | 32.1         | 42.1, 41.8            | 19.0            | 2.47×   |
| 64    | 74.8  | 55.2, 67.5   | **28.8** | 55.7         | 72.8, 76.2            | 76.4            | 1.92×   |

(speedup: `matrix` against the first base run; 31 bytes is a control and does not take
the new path.)

- `encode_matrix` is 2.5× faster at 32 bytes and 1.9× at 64 (2.5× and 1.9× against
  `matrix-general` in the same binary), and 2.4× and 2.6× faster than `five8`.
- `matrix-portable` gains 2.2× at 32 bytes. At 64 bytes its column sums take 173 scalar
  multiply-adds (43 at 32), and it is 2–5% slower than the general path's plain-Rust
  kernels (two runs). Kept for one rule for both sizes; no target here uses it.
- A previous build of the same kernels measured 12.7 and 29.2 ns (base 31.6–31.8 and
  54.8–55.9 ns, `five8` 31.1–31.9 and 72.9–78.0 ns).

Tried and not kept (M5):

- **Two accumulator chains per block** indexed by row parity: LLVM kept them in memory,
  2.3× slower at 64 bytes (one binary, interleaved).
- **Stack buffers aligned to 64 bytes**: slower, from an extra copy of the column sums.
- **One fused NEON pass per block** (its sums, its four carry steps and its characters,
  all in registers, so program order follows the carry chain): 12.3 ns at 32 bytes but
  44 ns at 64; with two accumulator chains per block 11.8 and 43.9 ns. The staged kernels
  took 12.6 and 28.4 ns in the next build.
- **Zipped iterators** in the row loops of the staged NEON kernel (for clippy) instead of
  indexing: 13.2 and 47.0 ns, against 12.9 and 29.0 ns indexed in the next build. The two
  functions differ by under 2% in instruction mix (881 against 894) and differ mostly in
  order. So at 64 bytes this code has a 1.6× cliff that depends on how LLVM schedules
  it. The cause is not found; moving the stack buffers by 2 KiB in a slow build changed
  nothing. Other builds may land on the slow side.

**Zen 5.** AMD EPYC 9B45 on the GCP VM, rustc 1.99.0, the same bench command with
default flags and then `-C target-cpu=native`, back to back; no LTO, no pinning; one
criterion run per build, steal 0. Two runs. The first (`be91f3a`) had AVX2 column sums
and AVX2 characters at both sizes. ns:

| bytes | flags   | five8 | matrix (AVX2) | matrix-general | matrix-portable |
|------:|:--------|------:|--------------:|---------------:|----------------:|
| 32    | default | 56.3  | 38.6          | 53.0           | **25.5**        |
| 32    | native  | 36.6  | 51.8          | 54.0           | **26.5**        |
| 64    | default | 127   | **86.0**      | 92.3           | 128             |
| 64    | native  | **55.1** | 91.6       | 89.3           | 135             |

The second (`b2ffc77`) ran the plain-Rust kernels at 32 bytes in `encode_matrix` and timed
four AVX2 combinations beside it: sums and characters (`encode_matrix`'s 64-byte path in
this run, timed a second time), sums only, characters only, and sums and characters with
`normalize`'s carry. ns:

| bytes | flags   | five8 | matrix | matrix-general | matrix-portable | sums, chars | sums | chars | sums, chars, normalize |
|------:|:--------|------:|-------:|---------------:|----------------:|------------:|-----:|------:|-----------------------:|
| 32    | default | 56.5  | **25.5** | 52.6         | 25.5            | 53.1        | 36.8 | 30.2  | 40.4                   |
| 32    | native  | 33.6  | **24.8** | 50.7         | 24.8            | 35.6        | 37.9 | 25.1  | 41.6                   |
| 64    | default | 121   | 98.2   | 85.6           | 121             | 97.3        | **73.3** | 130 | 101                  |
| 64    | native  | **50.7** | 88.2 | 82.1          | 127             | 82.0        | 75.0 | 123   | 99.3                   |

- **32 bytes: plain Rust.** It took 24.8–26.5 ns in all four builds, against 35.6–53.1 ns
  with AVX2 sums. It is 2.2× faster than `five8` with default flags and 1.35× faster
  than its AVX2 code with native flags.
- **64 bytes: AVX2 sums, plain-Rust characters**, now `encode_matrix`'s path on x86_64:
  73.3 and 75.0 ns, against 82–101 ns with AVX2 characters and 82–86 ns for the general
  path. That is 1.65× faster than `five8` with default flags, but `five8`'s AVX2 code is
  1.48× faster with native flags (50.7 ns). It was timed as the `sums` column, not
  through `encode_matrix`.
- The AVX2 character kernel cost time with AVX2 sums at both sizes, and with plain-Rust
  sums it was slower than plain Rust or within 3% of it. It is removed.
- **Placement.** The same AVX2 32-byte code took 38.6, then 53.1 ns with default flags
  and 51.8, then 35.6 ns with native flags, while `bs58`, `encode_batched` and `five8`
  moved by at most 8%. The 64-byte AVX2 path timed twice in one binary: 97.3 and 98.2 ns,
  82.0 and 88.2 ns. So gaps under about 10% here are not reliable.
- Not tried on x86: `five8`'s AVX2 code keeps its characters in registers through to
  masked stores, where ours goes through a stack buffer and the table of pairs, and its
  carry chain has 17 steps (base 58^5) against our 22. Either may explain its lead at
  64 bytes with native flags; neither is measured.

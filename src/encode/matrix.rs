//! Encoding by a matrix of powers (Firedancer's encode, for any length): split the bytes
//! into `u32` words, multiply the word vector by a table of `2^(32i)` in base 58^4, sum
//! columns in `u64`, then carry. The multiply-adds use the decoder's panel kernels
//! (`crate::matrix`).

use std::mem::MaybeUninit;

use super::{encode_batched, max_encoded_len, split_zeros};
use crate::common::ALPHABET;
use crate::{BufferTooSmall, encode_ntt_cached};
use crate::matrix::{PANEL, PANEL_PORTABLE, Panel, zeroed};
use table::{CARRY_EVERY, COL_BASE, MAX_BYTES, MAX_COLS, MAX_WORDS, PANEL_BYTES, ROWS, WORD};

mod fixed;
#[cfg(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon"))]
mod neon;
mod table;
#[cfg(target_arch = "x86_64")]
mod x86;
#[cfg(target_arch = "x86_64")]
pub use x86::sweep as fixed_sweep;

/// Inputs with fewer bytes than this, after leading zeros, use [`encode_batched`], which
/// is faster there. Tuned by measurement; README.md has the numbers.
const MATRIX_MIN: usize = 22;

/// One backend's kernels: the decoder's panel kernel, and [`digits`]'s.
struct Kernels {
    panel: Panel,
    digits: fn(&[u64], &mut [[u8; 4]]),
}

const PORTABLE: Kernels = Kernels { panel: PANEL_PORTABLE, digits };

#[cfg(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon"))]
const BEST: Kernels = Kernels { panel: PANEL, digits: neon::digits };
#[cfg(not(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon")))]
const BEST: Kernels = Kernels { panel: PANEL, digits };

/// Like [`encode_batched`], but faster from 22 bytes to 8 KiB when the table of powers is
/// in cache. Fewer than `MATRIX_MIN` bytes use [`encode_batched`]; more than 8192 (4096 on
/// CPUs with the AVX-512 IFMA leaf, see [`IFMA_CAP`]) use [`encode_ntt_cached`].
///
/// NEON on aarch64; the decoder's AVX2 or SSE4.1 kernels on x86_64, where on Zen 5 it was
/// 1.2x (32 B) to 3.9x (512 B) faster than [`encode_batched`]. Other targets run the
/// plain-Rust kernels.
///
/// Exactly 32 or 64 bytes take a fixed-size path (`matrix/fixed.rs`) with its table in
/// the binary: NEON on aarch64; on x86_64 plain Rust, with AVX2 kernels for 64 bytes if
/// the CPU has AVX2.
///
/// No allocation per call up to 8 KiB, and about 30 KiB of stack. Each size class of the
/// table is built once, on first use, and kept: 47 KiB, 181 KiB, 712 KiB, 2.8 MiB or
/// 11 MiB.
pub fn encode_matrix(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    if let Ok(x) = input.try_into() {
        return fixed::encode_32(x, output);
    }
    if let Ok(x) = input.try_into() {
        return fixed::encode_64(x, output);
    }
    encode_with(&BEST, input, output, MATRIX_MIN)
}

/// [`encode_matrix`] with the plain-Rust kernels, to show what the SIMD ones add.
pub fn encode_matrix_portable(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    if let Ok(x) = input.try_into() {
        return fixed::encode_32_portable(x, output);
    }
    if let Ok(x) = input.try_into() {
        return fixed::encode_64_portable(x, output);
    }
    encode_with(&PORTABLE, input, output, MATRIX_MIN)
}

/// [`encode_matrix`] with [`encode_batched`] below `min` bytes and without the fixed-size
/// path, for tuning.
pub fn encode_matrix_with(
    input: &[u8],
    output: &mut [u8],
    min: usize,
) -> Result<usize, BufferTooSmall> {
    encode_with(&BEST, input, output, min)
}

/// [`encode_matrix`] with kernels `k`, using [`encode_batched`] below `min` bytes.
/// Inlined, so small inputs pay only a length check.
#[inline(always)]
fn encode_with(
    k: &Kernels,
    input: &[u8],
    output: &mut [u8],
    min: usize,
) -> Result<usize, BufferTooSmall> {
    // The input length bounds the byte count, so small inputs skip `split_zeros` here.
    if input.len() < min {
        return encode_batched(input, output);
    }
    encode_large(k, input, output, min)
}

/// [`encode_with`] for inputs of at least `min` bytes. Not inlined: its stack frame holds
/// [`Scratch`], about 30 KiB, which small inputs should not set up.
#[inline(never)]
fn encode_large(
    k: &Kernels,
    input: &[u8],
    output: &mut [u8],
    min: usize,
) -> Result<usize, BufferTooSmall> {
    let (zeros, x) = split_zeros(input);
    if x.len() < min {
        return encode_batched(input, output);
    }
    let cap = if crate::mul::ifma_threshold() != 0 { IFMA_CAP } else { MAX_BYTES };
    if x.len() > cap {
        return encode_ntt_cached(input, output);
    }
    let mut scratch = Scratch::new();
    write(k, zeros, convert(k, x, &mut scratch), output)
}

/// The largest input `encode_matrix` converts itself when Karatsuba has the AVX-512 IFMA
/// leaf. On Zen 5 the NTT encoder with matrix leaves tied the matrix at 4 KiB (43.7
/// against 44.4 µs) and was 33% faster at 8 KiB.
const IFMA_CAP: usize = 4096;

/// [`encode_matrix`]'s conversion with the fastest kernels, without its thresholds: the
/// divide-and-conquer encoders' single-leaf case. Requires at most `MAX_BYTES` bytes after
/// the leading zeros.
#[inline(never)]
pub(super) fn encode_whole(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    let (zeros, x) = split_zeros(input);
    assert!(x.len() <= MAX_BYTES, "{} bytes for one matrix", x.len());
    let mut scratch = Scratch::new();
    write(&BEST, zeros, convert(&BEST, x, &mut scratch), output)
}

/// Stack space for one conversion, about 30 KiB. Only the part an input needs is written.
///
/// Aligned, with `words` first so that `cols` is aligned too. Otherwise the frame decides:
/// with `cols` 8 bytes off a 16-byte boundary the panels took 7-12% longer at 1-2 KiB
/// (M5 Pro).
#[repr(C, align(64))]
pub(super) struct Scratch {
    words: [MaybeUninit<u32>; MAX_WORDS],
    cols: [MaybeUninit<u64>; MAX_COLS],
}
const _: () = assert!(std::mem::offset_of!(Scratch, cols) % 64 == 0);

impl Scratch {
    pub(super) fn new() -> Scratch {
        Scratch {
            words: [MaybeUninit::uninit(); MAX_WORDS],
            cols: [MaybeUninit::uninit(); MAX_COLS],
        }
    }
}

/// [`convert`] with the fastest kernels: the divide-and-conquer encoders' matrix leaves.
pub(super) fn columns<'a>(x: &[u8], scratch: &'a mut Scratch) -> &'a [u64] {
    // A leaf has at most `MATRIX_LEAF` chunks, so at most `8 MATRIX_LEAF` bytes.
    const _: () = assert!(8 * super::dc::MATRIX_LEAF <= MAX_BYTES);
    convert(&BEST, x, scratch)
}

/// [`digits`] with the fastest kernel: `out[i] = four_digits(cols[cols.len() - 1 - i])`.
pub(super) fn column_digits(cols: &[u64], out: &mut [[u8; 4]]) {
    (BEST.digits)(cols, out);
}

/// The value of the big-endian `x` in base 58^4, least significant column first and
/// trimmed, in `scratch`. Requires `x.len() <= MAX_BYTES`.
fn convert<'a>(k: &Kernels, x: &[u8], scratch: &'a mut Scratch) -> &'a [u64] {
    if x.is_empty() {
        return &[];
    }
    let panels = x.len().div_ceil(PANEL_BYTES);
    let words = zeroed(&mut scratch.words, ROWS * panels);
    let (head, body) = x.as_rchunks::<WORD>();
    for (w, c) in words.iter_mut().zip(body.iter().rev()) {
        *w = u32::from_be_bytes(*c);
    }
    if !head.is_empty() {
        words[body.len()] = head.iter().fold(0, |v, &b| v << 8 | b as u32);
    }
    let table = table::get(x.len());
    let cols = zeroed(&mut scratch.cols, table.cols(panels));
    for (p, d) in words.as_chunks::<ROWS>().0.iter().enumerate() {
        let end = 4 * table.panels[p].nb;
        (k.panel)(&mut cols[..end], table.words(p), d);
        if (p + 1).is_multiple_of(CARRY_EVERY) {
            // `cols[end]` is the carry column: panel ends increase, so neither a panel
            // nor an earlier pass has written it.
            spread(&mut cols[..=end]);
        }
    }
    // Every column is a non-negative part of the value, so those at or above its length
    // stay zero. The value has at most `ceil(max_encoded_len / 4)` columns; one more keeps
    // the last column processed at zero.
    let used = cols.len().min(max_encoded_len(x.len()).div_ceil(4) + 1);
    let len = normalize(&mut cols[..used]);
    &cols[..len]
}

/// Moves each column's multiples of 58^4 into the next one, all at once. Afterwards each
/// column is at most `MAX_AFTER_CARRY`: a remainder plus one carry. Requires the last
/// column below 58^4, so nothing leaves the slice.
///
/// Each carry depends only on its own column, so this is not a carry chain.
fn spread(cols: &mut [u64]) {
    let mut carry = 0;
    for c in cols.iter_mut() {
        let v = *c;
        *c = v % COL_BASE + carry;
        carry = v / COL_BASE;
    }
    debug_assert_eq!(carry, 0, "the last column was not below 58^4");
}

/// Reduces every column below 58^4 and returns the trimmed length. Requires the last
/// column below 58^4.
///
/// One pass: [`spread`]'s step, then an exact carry. A spread column `s` is at most
/// `MAX_AFTER_CARRY`, so `q = s / 58^4 < 2^18` and the carry in is at most `q + 1`: the
/// remainder plus the carry is below `2 * 58^4` (asserted in `table.rs`), and one
/// compare finishes the division. Only that compare and the increment are on the carry
/// chain.
fn normalize(cols: &mut [u64]) -> usize {
    let (mut spill, mut carry) = (0, 0);
    for c in cols.iter_mut() {
        let v = *c;
        let s = v % COL_BASE + spill;
        spill = v / COL_BASE;
        let (q, r) = (s / COL_BASE, s % COL_BASE);
        let k = carry >= COL_BASE - r;
        *c = r + carry - if k { COL_BASE } else { 0 };
        carry = q + k as u64;
    }
    debug_assert_eq!(spill, 0, "the last column was not below 58^4");
    debug_assert_eq!(carry, 0, "the value does not fit the columns");
    cols.iter().rposition(|&c| c != 0).map_or(0, |i| i + 1)
}

/// Writes `zeros` '1's and the digits of the trimmed columns `cols` to `output`, and
/// returns the length. Writes nothing past it.
fn write(
    k: &Kernels,
    zeros: usize,
    cols: &[u64],
    output: &mut [u8],
) -> Result<usize, BufferTooSmall> {
    let Some((&top, rest)) = cols.split_last() else {
        output.get_mut(..zeros).ok_or(BufferTooSmall)?.fill(b'1');
        return Ok(zeros);
    };
    let top = four_digits(top);
    let skip = top.iter().take_while(|&&c| c == b'1').count();
    let n = zeros + 4 - skip + 4 * rest.len();
    let out = output.get_mut(..n).ok_or(BufferTooSmall)?;
    let (head, body) = out.split_at_mut(zeros + 4 - skip);
    head[..zeros].fill(b'1');
    head[zeros..].copy_from_slice(&top[skip..]);
    (k.digits)(rest, body.as_chunks_mut::<4>().0);
    Ok(n)
}

/// The characters of `cols`, each below 58^4, top column first:
/// `out[i] = four_digits(cols[cols.len() - 1 - i])`. Requires `out.len() == cols.len()`.
fn digits(cols: &[u64], out: &mut [[u8; 4]]) {
    assert_eq!(cols.len(), out.len());
    for (o, &c) in out.iter_mut().zip(cols.iter().rev()) {
        *o = four_digits(c);
    }
}

/// The four characters of a column below 58^4, most significant first.
#[inline(always)]
pub(super) fn four_digits(c: u64) -> [u8; 4] {
    let c = c as u32;
    let [a, b] = CHAR_PAIRS[(c / 3364) as usize];
    let [d, e] = CHAR_PAIRS[(c % 3364) as usize];
    [a, b, d, e]
}

/// The two characters of each value below 58^2, most significant first.
static CHAR_PAIRS: [[u8; 2]; 3364] = {
    let mut t = [[0; 2]; 3364];
    let mut i = 0;
    while i < t.len() {
        t[i] = [ALPHABET[i / 58], ALPHABET[i % 58]];
        i += 1;
    }
    t
};

#[cfg(test)]
mod tests {
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::SeedableRng;

    use super::table::{MAX_AFTER_CARRY, MAX_CARRY};
    use super::*;
    use crate::encode::tests::{Encode, encode_vec, random_bytes};

    /// The matrix path at every size up to `MAX_BYTES`.
    fn forced_best(data: &[u8], out: &mut [u8]) -> Result<usize, BufferTooSmall> {
        encode_with(&BEST, data, out, 0)
    }

    fn forced_portable(data: &[u8], out: &mut [u8]) -> Result<usize, BufferTooSmall> {
        encode_with(&PORTABLE, data, out, 0)
    }

    const FORCED: [(&str, Encode); 2] = [("best", forced_best), ("portable", forced_portable)];

    /// Checks both forced kernels against `encode_batched`, into an ample buffer, an
    /// exact one and one a byte short.
    fn check(data: &[u8]) {
        let want = encode_vec(encode_batched, data);
        for (name, f) in FORCED {
            assert!(encode_vec(f, data) == want, "{name} on {} bytes", data.len());
            let mut exact = vec![0xAA; want.len()];
            assert_eq!(f(data, &mut exact), Ok(want.len()), "{name}: exact, {} bytes", data.len());
            assert!(exact == want, "{name}: exact, {} bytes", data.len());
            if let Some(short) = exact.get_mut(1..) {
                assert_eq!(f(data, short), Err(BufferTooSmall), "{name}: short");
            }
        }
    }

    /// Random bytes at every size up to `MAX_BYTES`, some with leading zeros.
    #[test]
    fn every_size() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(30);
        for len in 0..=MAX_BYTES {
            let mut data = random_bytes(&mut rng, len);
            if len % 7 == 0 {
                data[..len.min(5)].fill(0);
            }
            check(&data);
        }
    }

    /// All 0xFF gives the largest column sums, at every size near a panel or class edge.
    /// Also a single top bit, and inner runs of zero bytes, which give zero columns.
    #[test]
    fn patterns() {
        // Class edges, and every 47 panels (1504 bytes) a carry pass.
        let edges = [1, 31, 32, 33, 512, 1024, 1504, 2048, 3008, 4096, 4512, 6016, 7520, MAX_BYTES];
        let around =
            edges.iter().flat_map(|&e| e.saturating_sub(4).max(1)..=(e + 4).min(MAX_BYTES));
        for n in around.chain([MATRIX_MIN]) {
            check(&vec![0xFF; n]);
            let mut pow2 = vec![0; n];
            pow2[0] = 0x80;
            check(&pow2);
            let mut runs = vec![0xFF; n];
            runs[n / 4..n / 2].fill(0);
            check(&runs);
            // Leading zeros move the byte count, not the table class.
            check(&[vec![0; 3], vec![0xFF; n]].concat());
        }
    }

    #[test]
    fn best_matches_portable() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(31);
        for n in [1, 33, 512, 1000, 1505, 2048, 4097, MAX_BYTES] {
            for data in [random_bytes(&mut rng, n), vec![0xFF; n]] {
                let (mut a, mut b) = (Scratch::new(), Scratch::new());
                let want = convert(&PORTABLE, &data, &mut b).to_vec();
                assert_eq!(convert(&BEST, &data, &mut a), want, "{n} bytes");
            }
        }
    }

    /// The carry schedule on all 0xFF at `MAX_BYTES`, in `u128`: every column sum stays
    /// below 2^64 before a pass and at most `MAX_AFTER_CARRY` after it, and the result
    /// matches `convert`. All 0xFF maximizes every column sum at once, since the table's
    /// entries are not negative.
    #[test]
    fn carry_bound() {
        let words = vec![u32::MAX; MAX_WORDS];
        let table = table::get(MAX_BYTES);
        let panels = MAX_WORDS / ROWS;
        let mut cols = vec![0u128; table.cols(panels)];
        let mut peak = 0;
        let pass = |cols: &mut [u128], peak: &mut u128| {
            let mut carry = 0;
            for c in cols.iter_mut() {
                *peak = (*peak).max(*c);
                assert!(*c <= u64::MAX as u128);
                let v = *c;
                *c = v % COL_BASE as u128 + carry;
                carry = v / COL_BASE as u128;
                assert!(carry <= MAX_CARRY as u128 && *c <= MAX_AFTER_CARRY as u128);
            }
            assert_eq!(carry, 0);
        };
        for p in 0..panels {
            let (nb, t) = (table.panels[p].nb, table.words(p));
            for (j, c) in cols[..4 * nb].iter_mut().enumerate() {
                for r in 0..ROWS {
                    *c += words[ROWS * p + r] as u128 * t[(ROWS * (j / 4) + r) * 4 + j % 4] as u128;
                }
            }
            if (p + 1).is_multiple_of(CARRY_EVERY) {
                pass(&mut cols[..=4 * nb], &mut peak);
            }
        }
        pass(&mut cols, &mut peak);
        // Exact normalization, then the same columns as `convert`.
        let mut carry = 0;
        for c in cols.iter_mut() {
            let v = *c + carry;
            (*c, carry) = (v % COL_BASE as u128, v / COL_BASE as u128);
        }
        assert_eq!(carry, 0);
        let len = cols.iter().rposition(|&c| c != 0).map_or(0, |i| i + 1);
        let want: Vec<u64> = cols[..len].iter().map(|&c| c as u64).collect();
        for k in [&BEST, &PORTABLE] {
            assert_eq!(convert(k, &vec![0xFF; MAX_BYTES], &mut Scratch::new()), want);
        }
        // The bound has room: the peak is about 2^63.
        let room = (u64::MAX as u128 - peak) as f64;
        eprintln!("peak column sum 2^{:.2}, room 2^{:.2}", (peak as f64).log2(), room.log2());
    }

    #[test]
    fn four_digits_matches_division() {
        for c in (0..COL_BASE).step_by(997).chain([0, 57, 58, 3363, 3364, COL_BASE - 1]) {
            let mut want = [0; 4];
            let mut v = c;
            for d in want.iter_mut().rev() {
                *d = ALPHABET[(v % 58) as usize];
                v /= 58;
            }
            assert_eq!(four_digits(c), want, "{c}");
        }
    }

    /// Below `MATRIX_MIN` bytes after the leading zeros, and around it.
    #[test]
    fn leading_zeros_around_threshold() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(32);
        let real: [(&str, Encode); 2] =
            [("best", encode_matrix), ("portable", encode_matrix_portable)];
        for bytes in [MATRIX_MIN - 1, MATRIX_MIN, MATRIX_MIN + 1, MAX_BYTES, MAX_BYTES + 1] {
            for zeros in [0, 1, 2, 100, MATRIX_MIN] {
                let mut x = random_bytes(&mut rng, bytes);
                x[0] |= 1;
                let data = [vec![0; zeros], x].concat();
                let want = encode_vec(encode_batched, &data);
                for (name, f) in real {
                    let at = format!("{name}: {zeros} zeros, {bytes} bytes");
                    assert!(encode_vec(f, &data) == want, "{at}");
                    let mut exact = vec![0; want.len()];
                    assert_eq!(f(&data, &mut exact), Ok(want.len()), "{at}");
                    assert!(exact == want, "{at}");
                    assert_eq!(f(&data, &mut exact[1..]), Err(BufferTooSmall), "{at}");
                }
            }
        }
    }

    /// Several threads encode in every size class at once.
    #[test]
    fn threads() {
        let inputs: Vec<Vec<u8>> =
            [500, 1000, 2000, 4000, 8000].iter().map(|&n| vec![0xFF; n]).collect();
        let want: Vec<_> = inputs.iter().map(|d| encode_vec(encode_batched, d)).collect();
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for (d, w) in inputs.iter().zip(&want) {
                        assert!(&encode_vec(encode_matrix, d) == w);
                    }
                });
            }
        });
    }
}

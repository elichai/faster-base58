//! Encoding by a matrix of powers (Firedancer's encode, for any length): split the bytes
//! into `u32` words, multiply the word vector by a table of `2^(32i)` in base 58^4, sum
//! columns in `u64`, then carry. The multiply-adds use the decoder's panel kernels
//! (`crate::matrix`).

use core::mem::MaybeUninit;

use super::max_encoded_len;
#[cfg(any(feature = "cache", test, feature = "__internals", kani))]
use super::{bits, min_digits};
#[cfg(any(feature = "cache", test, feature = "__internals", kani))]
use crate::BufferTooSmall;
use crate::common::ALPHABET;
use crate::matrix::{PANEL, Panel, zeroed};
use table::{CARRY_EVERY, COL_BASE, MAX_BYTES, MAX_COLS, MAX_WORDS, PANEL_BYTES, ROWS, WORD};

pub(crate) mod fixed;
#[cfg(all(
    target_arch = "aarch64",
    target_endian = "little",
    target_feature = "neon"
))]
mod neon;
mod table;
#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
mod x86;

/// One backend's kernels: the decoder's panel kernel, and [`digits`]'s.
struct Kernels {
    panel: Panel,
    digits: fn(&[u64], &mut [[u8; 4]]),
}

const BEST: Kernels = core::cfg_select! {
    all(target_arch = "aarch64", target_endian = "little", target_feature = "neon") => {
        Kernels { panel: PANEL, digits: neon::digits }
    }
    _ => Kernels { panel: PANEL, digits },
};

/// Matrix conversion after automatic length selection.
#[cfg(any(feature = "cache", test, feature = "__internals"))]
pub(crate) mod entry {
    use super::*;

    /// Inputs with fewer bytes than this, after leading zeros, use the batched encoder, which
    /// is faster there. Tuned by measurement; see `docs/performance.md`.
    pub(crate) const MATRIX_MIN: usize = 22;

    /// Converts a normalized value of `MATRIX_MIN..=MAX_BYTES` bytes.
    /// The caller selects the backend before entering this 30 KiB stack frame.
    #[cfg(feature = "cache")]
    #[inline]
    pub(crate) fn encode(
        zeros: usize,
        x: &[u8],
        output: &mut [u8],
    ) -> Result<usize, BufferTooSmall> {
        assert!(
            x.len() >= MATRIX_MIN,
            "value is too short for the matrix encoder"
        );
        encode_parts(&BEST, zeros, x, output)
    }

    /// The largest input `encode_matrix` converts itself when Karatsuba has the AVX-512 IFMA
    /// leaf. On Zen 5, in one binary, the NTT encoder with matrix leaves took 3% more time
    /// than the matrix at 3584 bytes with default flags and 3.5–6% less at 3840 and 4096
    /// (historically 7–11% less from 3584 with native flags).
    pub(crate) const IFMA_CAP: usize = 3584;

    pub(crate) const MAX_INPUT: usize = MAX_BYTES;
}

/// Comparison and tuning hooks; production conversions keep using the core below.
#[cfg(any(test, feature = "__internals"))]
pub(crate) mod internals {
    use super::entry::{IFMA_CAP, MATRIX_MIN, MAX_INPUT};
    use super::*;
    use crate::matrix::internals::PANEL_PORTABLE;

    pub(super) const PORTABLE: Kernels = Kernels {
        panel: PANEL_PORTABLE,
        digits,
    };

    /// General matrix encoding, with batched and NTT fallbacks outside its useful range.
    /// Fixed 32/64-byte encoders belong to Auto and `no_alloc`, not this comparison hook.
    ///
    /// # Errors
    /// Returns [`BufferTooSmall`] if `output` cannot hold the encoded string.
    pub fn encode_matrix(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
        encode_with(&BEST, input, output, MATRIX_MIN)
    }

    /// [`encode_matrix`] with plain-Rust kernels.
    ///
    /// # Errors
    /// Returns [`BufferTooSmall`] if `output` cannot hold the encoded string.
    pub fn encode_matrix_portable(
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, BufferTooSmall> {
        encode_with(&PORTABLE, input, output, MATRIX_MIN)
    }

    /// [`encode_matrix`] with [`crate::internals::encode_batched`] below
    /// `min` bytes and without the fixed-size path, for tuning.
    ///
    /// # Errors
    /// Returns [`BufferTooSmall`] if `output` cannot hold the encoded string.
    pub fn encode_matrix_with(
        input: &[u8],
        output: &mut [u8],
        min: usize,
    ) -> Result<usize, BufferTooSmall> {
        encode_with(&BEST, input, output, min)
    }

    pub(super) fn encode_with(
        k: &Kernels,
        input: &[u8],
        output: &mut [u8],
        min: usize,
    ) -> Result<usize, BufferTooSmall> {
        let (zeros, x) = super::super::split_zeros(input);
        if x.len() < min {
            return super::super::batched::encode_parts(zeros, x, output);
        }
        let cap = if crate::mul::ifma_threshold() != 0 {
            IFMA_CAP
        } else {
            MAX_INPUT
        };
        if x.len() > cap {
            return crate::ntt::internals::cached::encode_ntt_cached(input, output);
        }
        encode_parts(k, zeros, x, output)
    }

    #[cfg(feature = "__internals")]
    pub(crate) mod feature_only {
        use super::super::fixed;

        pub use super::super::table::cache_memory as encode_matrix_cache_memory;

        /// Type-layout bytes for the two fixed tables, not an emitted-binary size.
        #[must_use]
        pub fn fixed_matrix_table_bytes() -> [usize; 2] {
            [
                size_of::<fixed::Table<8, 12>>(),
                size_of::<fixed::Table<16, 24>>(),
            ]
        }
    }
}

/// Matrix conversion with the fastest kernels, without its thresholds: the
/// divide-and-conquer encoders' single-leaf case. Requires at most `MAX_BYTES` bytes after
/// the leading zeros.
///
/// # Errors
/// Returns [`BufferTooSmall`] if `output` cannot hold the encoded string.
#[inline(never)]
#[cfg(any(test, feature = "__internals", kani))]
pub fn encode_whole(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    let (zeros, x) = super::split_zeros(input);
    encode_parts(&BEST, zeros, x, output)
}

#[cfg(any(feature = "cache", test, feature = "__internals", kani))]
#[inline(never)]
fn encode_parts(
    k: &Kernels,
    zeros: usize,
    x: &[u8],
    output: &mut [u8],
) -> Result<usize, BufferTooSmall> {
    assert!(x.len() <= MAX_BYTES, "{} bytes for one matrix", x.len());
    debug_assert_ne!(x.first(), Some(&0));
    if zeros > output.len() || (!x.is_empty() && output.len() - zeros < min_digits(bits(x))) {
        return Err(BufferTooSmall);
    }
    let mut scratch = Scratch::new();
    write(k, zeros, convert(k, x, &mut scratch), output)
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
const _: () = assert!(core::mem::offset_of!(Scratch, cols) % 64 == 0);

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
        words[body.len()] = head.iter().fold(0, |v, &b| (v << 8) | b as u32);
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
#[cfg(any(feature = "cache", test, feature = "__internals", kani))]
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
#[cfg(any(
    test,
    feature = "__internals",
    not(all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ))
))]
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
const CHAR_PAIRS: [[u8; 2]; 3364] = {
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

    use super::entry::{IFMA_CAP, MATRIX_MIN};
    use super::internals::{PORTABLE, encode_matrix, encode_matrix_portable, encode_with};
    use super::table::{MAX_AFTER_CARRY, MAX_CARRY};
    use super::*;
    use crate::encode::batched::encode_batched;
    use crate::encode::tests::{Encode, encode_vec, random_bytes};

    #[cfg(feature = "cache")]
    #[test]
    #[should_panic(expected = "value is too short for the matrix encoder")]
    fn selected_encoder_rejects_short_values() {
        entry::encode(0, &[1], &mut [0; 2]).unwrap();
    }

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
            assert!(
                encode_vec(f, data) == want,
                "{name} on {} bytes",
                data.len()
            );
            let mut exact = vec![0xAA; want.len()];
            assert_eq!(
                f(data, &mut exact),
                Ok(want.len()),
                "{name}: exact, {} bytes",
                data.len()
            );
            assert!(exact == want, "{name}: exact, {} bytes", data.len());
            if let Some(short) = exact.get_mut(1..) {
                assert_eq!(f(data, short), Err(BufferTooSmall), "{name}: short");
            }
        }
    }

    /// Random bytes at every size up to `MAX_BYTES`, some with leading zeros.
    #[test]
    #[cfg_attr(
        not(feature = "cache"),
        ignore = "exhaustive identical kernels run with cache in CI; cache-off ownership and boundary tests remain active"
    )]
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
        let edges = [
            1, 31, 32, 33, 512, 1024, 1504, 2048, 3008, 4096, 4512, 6016, 7520, MAX_BYTES,
        ];
        let around = edges
            .iter()
            .flat_map(|&e| e.saturating_sub(4).max(1)..=(e + 4).min(MAX_BYTES));
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
            assert_eq!(
                convert(k, &vec![0xFF; MAX_BYTES], &mut Scratch::new()),
                want
            );
        }
        // The bound has room: the peak is about 2^63.
        let room = (u64::MAX as u128 - peak) as f64;
        eprintln!(
            "peak column sum 2^{:.2}, room 2^{:.2}",
            (peak as f64).log2(),
            room.log2()
        );
    }

    #[test]
    fn four_digits_matches_division() {
        for c in (0..COL_BASE)
            .step_by(997)
            .chain([0, 57, 58, 3363, 3364, COL_BASE - 1])
        {
            let mut want = [0; 4];
            let mut v = c;
            for d in want.iter_mut().rev() {
                *d = ALPHABET[(v % 58) as usize];
                v /= 58;
            }
            assert_eq!(four_digits(c), want, "{c}");
        }
    }

    /// Below `MATRIX_MIN` bytes after the leading zeros, and around it and the caps.
    #[test]
    fn leading_zeros_around_threshold() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(32);
        let real: [(&str, Encode); 2] = [
            ("best", encode_matrix),
            ("portable", encode_matrix_portable),
        ];
        let caps = [IFMA_CAP, IFMA_CAP + 1, MAX_BYTES, MAX_BYTES + 1];
        for bytes in [MATRIX_MIN - 1, MATRIX_MIN, MATRIX_MIN + 1]
            .into_iter()
            .chain(caps)
        {
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
        let inputs: Vec<Vec<u8>> = [500, 1000, 2000, 4000, 8000]
            .iter()
            .map(|&n| vec![0xFF; n])
            .collect();
        let want: Vec<_> = inputs
            .iter()
            .map(|d| encode_vec(encode_batched, d))
            .collect();
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for (d, w) in inputs.iter().zip(&want) {
                        assert_eq!(&encode_vec(encode_matrix, d), w);
                    }
                });
            }
        });
    }
}

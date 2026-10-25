//! Decoding by a matrix of powers (Firedancer's decode, for any length): group the digits
//! in base 58^4, multiply the group vector by a table of `58^(4i)` in base 2^32, sum
//! columns in `u64`, then carry once. The products are independent, so this runs at SIMD
//! multiply-add throughput instead of along a carry chain. See the spec,
//! `docs/superpowers/specs/2026-10-14-simd-front-end-design.md`.

use std::mem::MaybeUninit;

use crate::common::{CHUNK, DIGITS};
use table::{CARRY_EVERY, MAX_COLS, MAX_GROUPS, PANEL_CHARS, ROWS};

#[cfg(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon"))]
mod neon;
#[cfg(any(test, feature = "__internals", kani, not(all(
    target_arch = "aarch64", target_endian = "little", target_feature = "neon"
))))]
mod portable;
#[cfg(kani)]
mod proofs;
mod table;
#[cfg(target_arch = "x86_64")]
mod x86;

/// A panel kernel: see `portable::panel`.
pub(crate) type Panel = fn(&mut [u64], &[u32], &[u32; 8]);

/// One backend's kernels. See `portable.rs` for their contracts.
struct Kernels {
    parse: fn(&[u8], &mut [u32]) -> bool,
    panel: Panel,
}

#[cfg(any(test, feature = "__internals", not(any(
    all(target_arch = "aarch64", target_endian = "little", target_feature = "neon"),
    target_arch = "x86_64"
))))]
const PORTABLE: Kernels = Kernels { parse: portable::parse, panel: portable::panel };

const BEST: Kernels = std::cfg_select! {
    all(target_arch = "aarch64", target_endian = "little", target_feature = "neon") => {
        Kernels { parse: neon::parse, panel: neon::panel }
    }
    target_arch = "x86_64" => {
        Kernels { parse: portable::parse, panel: x86::panel }
    }
    _ => PORTABLE,
};

/// The panel kernels, for the matrix encoder (`encode/matrix.rs`).
pub(crate) const PANEL: Panel = BEST.panel;

/// Thresholded entry points and their shared wrappers.
#[cfg(any(feature = "cache", test, feature = "__internals"))]
pub(crate) mod entry {
    use super::*;
    use super::table::MAX_CHARS;
    use crate::{
        Error, batched::decode_batched, common::{invalid, split_zeros}, dc::{decode, write_bytes},
    };

    /// Inputs with fewer digits than this use [`decode_batched`], which is faster there.
    /// Tuned by measurement; README.md has the numbers.
    pub(super) const MATRIX_MIN: usize = 576;

    /// Like [`decode_batched`], but faster from about 0.4 KiB to 4 KiB when the table of
    /// powers is in cache. Fewer than `MATRIX_MIN` digits use [`decode_batched`]; more than
    /// 5632 use [`decode`].
    ///
    /// The speedup was measured with NEON (aarch64). x86_64 uses AVX2 or SSE4.1 kernels: on
    /// AMD Zen 5 they were faster from about 1 KiB with default flags, but with
    /// `-C target-cpu=native` they only tied [`decode_batched`] at 2–4 KiB. Other targets run
    /// the plain-Rust kernels; their advantage depends on the target and input size.
    ///
    /// With `cache`, warm matrix calls allocate no heap; first use builds a retained table.
    /// Without `cache`, each matrix invocation builds and drops its own table.
    /// Data is reserved at its final length: about 40 KiB, 154 KiB, 601 KiB or 2.3 MiB, plus
    /// panel metadata. With `cache`, initialized classes accumulate. The recursive
    /// fallback can allocate workspace. See `docs/tradeoffs/README.md` for measurements.
    pub fn decode_matrix(input: &str, output: &mut [u8]) -> Result<usize, Error> {
        decode_with(&BEST, input, output, MATRIX_MIN)
    }

    /// [`decode_matrix`] with kernels `k`, using [`decode_batched`] below `min` digits.
    /// Inlined, so small inputs pay only a length check.
    #[inline(always)]
    pub(super) fn decode_with(k: &Kernels, input: &str, output: &mut [u8], min: usize) -> Result<usize, Error> {
        // The input length bounds the digit count, so small inputs skip `split_zeros` here.
        if input.len() < min {
            return decode_batched(input, output);
        }
        decode_large(k, input, output, min)
    }

    /// [`decode_with`] for inputs of at least `min` bytes. Not inlined: its stack frame holds
    /// [`Scratch`], about 14 KiB, which small inputs should not set up.
    #[inline(never)]
    fn decode_large(k: &Kernels, input: &str, output: &mut [u8], min: usize) -> Result<usize, Error> {
        let (zeros, digits) = split_zeros(input.as_bytes());
        if digits.len() < min {
            return decode_batched(input, output);
        }
        if digits.len() > MAX_CHARS {
            return decode(input, output);
        }
        // A nonzero first digit gives at least 58^(d - 1), as in the D&C preflight.
        let min_len = if digits.is_empty() {
            0
        } else {
            ((digits.len() - 1) as u128 * 732 / 1000) as usize + 1
        };
        if output.len() < zeros + min_len {
            return Err(Error::BufferTooSmall);
        }
        let mut scratch = Scratch::new();
        let limbs = convert(k, digits, &mut scratch).ok_or_else(|| invalid(input))?;
        write_bytes(zeros, limbs, output)
    }
}

/// Comparison hooks; production conversions keep using the core below.
#[cfg(any(test, feature = "__internals"))]
pub(crate) mod internals {
    use super::*;
    use super::entry::{MATRIX_MIN, decode_with};
    use crate::Error;

    pub(crate) const PANEL_PORTABLE: Panel = PORTABLE.panel;

    /// [`super::entry::decode_matrix`] with the plain-Rust kernels, to show what the SIMD ones add.
    pub fn decode_matrix_portable(input: &str, output: &mut [u8]) -> Result<usize, Error> {
        decode_with(&PORTABLE, input, output, MATRIX_MIN)
    }

    #[cfg(feature = "__internals")]
    pub(crate) mod feature_only {
        use super::super::BEST;
        use super::super::entry::decode_with;
        use crate::Error;

        pub use super::super::table::cache_memory as decode_matrix_cache_memory;

        /// Force the general matrix kernel within its size limit, bypassing the small fallback.
        pub fn decode_matrix_forced(input: &str, output: &mut [u8]) -> Result<usize, Error> {
            decode_with(&BEST, input, output, 0)
        }
    }
}

/// Writes the value of `digits` to `out` as little-endian limbs and returns its trimmed
/// length, or `None` on an invalid character. Requires `digits.len() <= 5632` and
/// `out.len() >= digits.len().div_ceil(10)`. `out` may hold anything on entry.
///
/// `dc`'s leaf in the cached NTT decoders on aarch64. Not inlined: its stack frame holds
/// [`Scratch`], which every level of `dc`'s recursion would otherwise carry.
#[inline(never)]
pub(crate) fn leaf(out: &mut [u64], digits: &[u8]) -> Option<usize> {
    let mut scratch = Scratch::new();
    let limbs = convert(&BEST, digits, &mut scratch)?;
    out[..limbs.len()].copy_from_slice(limbs);
    Some(limbs.len())
}

/// Stack space for one conversion, about 14 KiB. Only the part an input needs is written.
struct Scratch {
    groups: [MaybeUninit<u32>; MAX_GROUPS],
    cols: [MaybeUninit<u64>; MAX_COLS],
}

impl Scratch {
    fn new() -> Scratch {
        Scratch {
            groups: [MaybeUninit::uninit(); MAX_GROUPS],
            cols: [MaybeUninit::uninit(); MAX_COLS],
        }
    }
}

/// The first `len` elements of `buf`, set to zero. Zeroing all of [`Scratch`] would cost
/// about 300 cycles per call.
pub(crate) fn zeroed<T: Copy + Default>(buf: &mut [MaybeUninit<T>], len: usize) -> &mut [T] {
    let buf = &mut buf[..len];
    buf.fill(MaybeUninit::new(T::default()));
    // SAFETY: every element was just written, and `MaybeUninit<T>` has `T`'s layout.
    unsafe { &mut *(buf as *mut [MaybeUninit<T>] as *mut [T]) }
}

/// The value of `digits` as trimmed little-endian limbs, in `scratch`, or `None` if a
/// byte is not a base58 digit. Requires `digits.len() <= MAX_CHARS`.
fn convert<'a>(k: &Kernels, digits: &[u8], scratch: &'a mut Scratch) -> Option<&'a [u64]> {
    if digits.is_empty() {
        return Some(&[]);
    }
    let panels = digits.len().div_ceil(PANEL_CHARS);
    // 8 groups per panel is at least the 4 per 16 characters that `parse` writes.
    let groups = zeroed(&mut scratch.groups, ROWS * panels);
    if !(k.parse)(digits, groups) {
        return None;
    }
    // After `parse`, so invalid input builds no table.
    let table = table::get(digits.len());
    let cols = zeroed(&mut scratch.cols, table.cols(panels));
    // Columns below `from` get no more products: later panels start further up.
    let mut from = 0;
    for (p, d) in groups.as_chunks::<ROWS>().0.iter().enumerate() {
        let end = p + 4 * table.panels[p].nb;
        (k.panel)(&mut cols[p..end], table.words(p), d);
        if (p + 1).is_multiple_of(CARRY_EVERY) {
            carry32(&mut cols[from..=end]);
            from = p + 1;
        }
    }
    let len = carry64(cols);
    let cols: &'a [u64] = cols;
    Some(&cols[..len])
}

/// Reduces every column but the last below 2^32, carrying into the last.
///
/// Afterwards each of these columns is at most `2 (2^32 - 1)`: the last one was zero
/// before (no panel reaches it yet) and gets one carry of at most `2^32 - 1`.
fn carry32(cols: &mut [u64]) {
    let (last, body) = cols.split_last_mut().expect("the carry column");
    let mut carry = 0;
    for c in body {
        let v = *c + carry;
        *c = v & u32::MAX as u64;
        carry = v >> 32;
    }
    *last += carry;
}

/// Turns 32-bit column sums into `u64` limbs in place, limb `k` from columns `2k` and
/// `2k + 1`, and returns the trimmed limb count. Requires an even length.
///
/// Each sum is below `2^64 + 2^96 + carry`, so it fits in `u128`, and the carry stays
/// below `2^34`.
fn carry64(cols: &mut [u64]) -> usize {
    let mut carry = 0u128;
    for k in 0..cols.len() / 2 {
        let v = cols[2 * k] as u128 + ((cols[2 * k + 1] as u128) << 32) + carry;
        cols[k] = v as u64;
        carry = v >> 64;
    }
    debug_assert_eq!(carry, 0, "the value does not fit the columns");
    cols[..cols.len() / 2].iter().rposition(|&l| l != 0).map_or(0, |i| i + 1)
}

// `leaf`'s `out` bound: `58^n < 2^(64 * ceil(n / 10))` because `58^10 < 2^64`.
const _: () = assert!(58u128.pow(CHUNK as u32) < 1u128 << 64);

#[cfg(test)]
mod tests {
    use super::*;
    use super::entry::{MATRIX_MIN, decode_matrix, decode_with};
    use super::internals::decode_matrix_portable;
    use super::table::MAX_CHARS;
    use crate::{Error, batched::decode_batched};
    use crate::common::{ALPHABET, BASE, chunks};
    use crate::tests::{Decode, decode_vec, random_string};
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    /// The matrix path at every size up to `MAX_CHARS`.
    fn forced_best(s: &str, out: &mut [u8]) -> Result<usize, Error> {
        decode_with(&BEST, s, out, 0)
    }

    fn forced_portable(s: &str, out: &mut [u8]) -> Result<usize, Error> {
        decode_with(&PORTABLE, s, out, 0)
    }

    const FORCED: [(&str, Decode); 2] = [("best", forced_best), ("portable", forced_portable)];

    /// Class boundaries and the dispatch threshold, each with its neighbours.
    fn boundaries() -> impl Iterator<Item = usize> {
        [704, 1408, 2816, MAX_CHARS, MATRIX_MIN].into_iter().flat_map(|b| [b - 1, b, b + 1])
    }

    fn check(s: &str) {
        let want = decode_vec(decode_batched, s);
        for (name, f) in FORCED {
            assert_eq!(decode_vec(f, s), want, "{name} on {} chars", s.len());
        }
    }

    #[test]
    fn forced_matches_batched() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(5);
        let random = (0..20).map(|_| 1 + rng.next_u32() as usize % MAX_CHARS).collect::<Vec<_>>();
        for len in (1..=1500).chain(boundaries()).chain(random) {
            check(&random_string(&mut rng, len));
        }
    }

    #[test]
    fn patterns() {
        for n in [2, 15, 16, 17, 31, 32, 33, 703, 704, 705, 1408, 2816, MAX_CHARS] {
            // All 'z' gives the largest column sums (about 2^63.1 at `MAX_CHARS`), so the
            // carry schedule suffices for every input. This does not show that the bound
            // is tight: the `const` asserts in `table.rs` guard it.
            check(&"z".repeat(n));
            check(&format!("2{}", "1".repeat(n - 1)));
            // A run of zero digits inside, and leading '1's.
            check(&format!("z{}z", "1".repeat(n - 2)));
            check(&format!("{}{}", "1".repeat(n / 2), "z".repeat(n - n / 2)));
        }
    }

    #[test]
    fn best_matches_portable() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(6);
        for n in [1, 33, 1000, 704, 1408, 2816, MAX_CHARS] {
            for s in [random_string(&mut rng, n), "z".repeat(n)] {
                let (mut a, mut b) = (Scratch::new(), Scratch::new());
                let want = convert(&PORTABLE, s.as_bytes(), &mut b);
                assert_eq!(convert(&BEST, s.as_bytes(), &mut a), want, "{n} chars");
            }
        }
    }

    /// Little-endian limbs of `digits` by the Horner loop, trimmed.
    fn horner(digits: &[u8]) -> Vec<u64> {
        let mut limbs: Vec<u64> = Vec::new();
        for c in chunks(digits) {
            let mut carry = c.expect("valid digits");
            for l in &mut limbs {
                (*l, carry) = l.carrying_mul(BASE, carry);
            }
            if carry != 0 {
                limbs.push(carry);
            }
        }
        limbs
    }

    #[test]
    #[cfg_attr(not(feature = "cache"), ignore = "exhaustive identical kernels run with cache in CI; cache-off ownership and boundary tests remain active")]
    fn leaf_matches_horner() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(7);
        let all_z = ["z".repeat(MAX_CHARS), "z".repeat(2816), "z".repeat(2560)];
        let inputs = (1..=MAX_CHARS).map(|n| random_string(&mut rng, n)).chain(all_z);
        for s in inputs {
            // Exactly one limb per chunk, holding garbage.
            let mut out = vec![0xDEAD_BEEF; s.len().div_ceil(CHUNK)];
            let len = leaf(&mut out, s.as_bytes()).expect("valid");
            assert_eq!(out[..len], horner(s.as_bytes())[..], "{} chars", s.len());
        }
        // Zero digits only: the value and the length are zero.
        assert_eq!(leaf(&mut [7; 2], b"11111111111"), Some(0));
        assert_eq!(leaf(&mut [7; 1], b"12O4"), None);
    }

    /// Each byte value in a valid input, as raw bytes: a `&str` cannot hold 0x80 as its
    /// only invalid byte, and NEON `parse` looks 0x80 up in its second table.
    #[test]
    fn leaf_raw_bytes() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(12);
        for n in [5, 16, 17, 40] {
            let valid = random_string(&mut rng, n).into_bytes();
            for c in 0..=u8::MAX {
                // In the partial first block and in full blocks.
                for pos in [0, n / 2, n - 1] {
                    let mut s = valid.clone();
                    s[pos] = c;
                    let ok = ALPHABET.contains(&c);
                    assert_eq!(leaf(&mut [0; 4], &s).is_some(), ok, "{c:#04x} at {pos} of {n}");
                    let portable = convert(&PORTABLE, &s, &mut Scratch::new()).is_some();
                    assert_eq!(portable, ok, "portable: {c:#04x} at {pos} of {n}");
                }
            }
        }
    }

    #[test]
    fn invalid_bytes() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(8);
        for n in [5, 16, 17, 40, 1000, 1401] {
            let valid = random_string(&mut rng, n);
            // In the partial first block, at block edges, in the middle and at the end.
            for pos in [0, 1, n % 16, n / 2, n.saturating_sub(16), n - 1, n] {
                for bad in ["0", "O", "I", "l", "{", "\0", "\u{80}", "é"] {
                    let s = format!("{}{bad}{}", &valid[..pos], &valid[pos..]);
                    let want = decode_vec(decode_batched, &s);
                    assert!(want.is_err());
                    for (name, f) in FORCED {
                        assert_eq!(decode_vec(f, &s), want, "{name}: {s:?}");
                    }
                }
            }
        }
    }

    /// Invalid input builds no table. A table is built once per process, so the test runs
    /// itself again in a child process, where none is built yet.
    #[cfg(feature = "cache")]
    #[test]
    fn invalid_input_builds_no_table() {
        const FRESH: &str = "MATRIX_FRESH_PROCESS";
        if std::env::var_os(FRESH).is_none() {
            let exe = std::env::current_exe().expect("the test binary");
            let child = std::process::Command::new(exe)
                .args(["matrix::tests::invalid_input_builds_no_table", "--exact"])
                .env(FRESH, "1")
                .output()
                .expect("the child runs");
            let ran = String::from_utf8_lossy(&child.stdout).contains("1 passed");
            assert!(child.status.success() && ran, "{child:?}");
            return;
        }
        let built = || table::cached::TABLES.iter().filter(|t| t.get().is_some()).count();
        for n in [MATRIX_MIN, 705, 1409, 2817, MAX_CHARS] {
            let s = format!("{}0", "z".repeat(n - 1));
            let index = n - 1;
            assert_eq!(decode_matrix(&s, &mut [0; 5000]), Err(Error::InvalidCharacter { index }));
            assert_eq!(leaf(&mut [0; 564], s.as_bytes()), None);
        }
        assert_eq!(built(), 0);
        // The check can fail: a valid input builds a table.
        decode_matrix(&"z".repeat(MATRIX_MIN), &mut [0; 5000]).expect("valid");
        assert_eq!(built(), 1);
    }

    #[test]
    fn buffers() {
        for (name, f) in FORCED {
            assert_eq!(f("", &mut []), Ok(0), "{name}");
            assert_eq!(decode_vec(f, "111"), Ok(vec![0, 0, 0]), "{name}");
            assert_eq!(f("5Q", &mut []), Err(Error::BufferTooSmall), "{name}");
            assert_eq!(f("15Q", &mut [0; 1]), Err(Error::BufferTooSmall), "{name}");
            assert_eq!(f("15Q", &mut [0; 2]), Ok(2), "{name}");
            let s = format!("11{}", "z".repeat(3000));
            let want = decode_vec(decode_batched, &s).expect("valid");
            let mut exact = vec![0; want.len()];
            assert_eq!(f(&s, &mut exact), Ok(want.len()), "{name}");
            assert_eq!(exact, want, "{name}");
            assert_eq!(f(&s, &mut exact[1..]), Err(Error::BufferTooSmall), "{name}");
        }
    }

    /// Leading '1's: inputs of at least `MATRIX_MIN` bytes, with digit counts around it.
    #[test]
    fn leading_ones_around_threshold() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(9);
        let real: [(&str, Decode); 2] =
            [("best", decode_matrix), ("portable", decode_matrix_portable)];
        for digits in [MATRIX_MIN - 1, MATRIX_MIN, MATRIX_MIN + 1] {
            for ones in [1, 2, 100, MATRIX_MIN] {
                // 'z' first, so the digit count is exact.
                let s = format!("{}z{}", "1".repeat(ones), random_string(&mut rng, digits - 1));
                let want = decode_vec(decode_batched, &s).expect("valid");
                for (name, f) in real {
                    let at = format!("{name}: {ones} ones, {digits} digits");
                    assert_eq!(decode_vec(f, &s).as_ref(), Ok(&want), "{at}");
                    let mut exact = vec![0; want.len()];
                    assert_eq!(f(&s, &mut exact), Ok(want.len()), "{at}");
                    assert_eq!(exact, want, "{at}");
                    assert_eq!(f(&s, &mut exact[1..]), Err(Error::BufferTooSmall), "{at}");
                }
            }
        }
    }

    /// Several threads decode in every size class at once. Other tests in this process
    /// may have built some tables already, so this does not always hit a cold class;
    /// `tests/matrix_cold.rs` does, in its own process.
    #[test]
    fn threads() {
        let inputs: Vec<String> = [700, 1400, 2800, 5600].iter().map(|&n| "z".repeat(n)).collect();
        let want: Vec<_> = inputs.iter().map(|s| decode_vec(decode_batched, s)).collect();
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for (s, w) in inputs.iter().zip(&want) {
                        assert_eq!(&decode_vec(decode_matrix, s), w);
                    }
                });
            }
        });
    }
}

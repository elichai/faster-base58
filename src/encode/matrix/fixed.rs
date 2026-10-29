//! Fixed matrix encoding for exactly 32 or 64 bytes: the sizes of keys,
//! hashes and signatures.
//!
//! The same algorithm, fixed at compile time: the table is a constant, the column sums
//! are locals, and there is no scratch to clear, table to look up or result to trim.
//! Firedancer's encoder (the `five8` crate) works the same way in base 58^5. The columns
//! here stay in base 58^4: 44 and 88 characters are whole columns, and a sum of 16
//! products stays below 2^60, so nothing is reduced before the final carry pass.
//!
//! The carry pass is serial, unlike the general path's (`super::normalize`): for at most
//! 22 columns, one scalar multiply per column on the chain beat `normalize`'s four off it.
//! The `x86_64` (AVX2, AVX-512 VBMI) and NEON paths for 64 bytes cut the chain into four
//! segments (`carry_split`).

use super::table::COL_BASE;
use crate::BufferTooSmall;
use crate::encode::max_encoded_len;

#[cfg(any(
    test,
    feature = "__internals",
    not(all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ))
))]
pub(crate) mod portable;

/// The powers `2^(32k)`, `k < W`, in base 58^4, in `C` columns.
#[repr(C, align(16))]
pub(super) struct Table<const W: usize, const C: usize> {
    /// `rows[k][j]`: column `j` of `2^(32k)`, least significant first.
    pub(super) rows: [[u32; C]; W],
    /// `start[j]`: rows below it are zero from column `j` on.
    pub(super) start: [usize; C],
}

impl<const W: usize, const C: usize> Table<W, C> {
    const fn new() -> Self {
        let mut rows = [[0; C]; W];
        rows[0][0] = 1;
        let mut k = 1;
        while k < W {
            let mut carry = 0;
            let mut j = 0;
            while j < C {
                let v = ((rows[k - 1][j] as u64) << 32) + carry;
                rows[k][j] = (v % COL_BASE) as u32;
                carry = v / COL_BASE;
                j += 1;
            }
            assert!(carry == 0, "2^(32k) does not fit in the columns");
            k += 1;
        }
        let mut start = [W; C];
        let mut j = 0;
        while j < C {
            // The first row longer than `j` columns.
            let mut k = 0;
            while k < W && len(&rows[k]) <= j {
                k += 1;
            }
            start[j] = k;
            j += 1;
        }
        Table { rows, start }
    }

    /// Checks the bounds the encoder relies on for `N`-byte inputs.
    const fn check(&self, n: usize) {
        assert!(n == 4 * W && C.is_multiple_of(4));
        // Each column sum of `W` words, plus `carry`'s incoming carry (a `u64` divided by
        // 58^4), fits in `u64`.
        let mut j = 0;
        while j < C {
            let mut sum = (u64::MAX / COL_BASE) as u128;
            let mut k = 0;
            while k < W {
                sum += u32::MAX as u128 * self.rows[k][j] as u128;
                k += 1;
            }
            assert!(sum <= u64::MAX as u128);
            j += 1;
        }
        // Every value fits in the columns `carry` reduces; the rest are zero.
        let mut k = 0;
        while k < W {
            assert!(len(&self.rows[k]) <= max_encoded_len(n).div_ceil(4));
            k += 1;
        }
        // Every result fits in the characters, and `encode`'s two copies cover it.
        assert!(max_encoded_len(n) <= 4 * C && max_encoded_len(n) <= n + n / 2);
    }
}

/// Columns up to the last non-zero one.
const fn len(row: &[u32]) -> usize {
    let mut n = row.len();
    while n > 0 && row[n - 1] == 0 {
        n -= 1;
    }
    n
}

/// 44 characters, plus 4 so that the digit kernels take whole groups of 4 columns.
pub(super) const TABLE_32: Table<8, 12> = Table::new();
/// 88 characters, plus 8.
pub(super) const TABLE_64: Table<16, 24> = Table::new();
const _: () = TABLE_32.check(32);
const _: () = TABLE_64.check(64);

// Select the architecture wrappers here; the scalar implementation stays common.
core::cfg_select! {
    all(target_arch = "x86_64", target_feature = "sse2") => {
        pub(crate) use super::x86::{encode_fixed_32 as encode_32, encode_fixed_64 as encode_64};
    }
    all(target_arch = "aarch64", target_endian = "little", target_feature = "neon") => {
        pub(crate) use super::neon::{encode_fixed_32 as encode_32, encode_fixed_64 as encode_64};
    }
    _ => {
        pub(crate) use self::portable::{encode_32_portable as encode_32, encode_64_portable as encode_64};
    }
}

/// Encodes `x`, whose characters in `4 C` digits `chars` gives (see [`chars`]).
#[inline(always)]
pub(super) fn encode<const N: usize, const C: usize>(
    x: &[u8; N],
    output: &mut [u8],
    chars: impl FnOnce(&[u8; N], &mut [[u8; 4]; C]),
) -> Result<usize, BufferTooSmall> {
    let mut c = [[0; 4]; C];
    chars(x, &mut c);
    let chars = c.as_flattened();
    // '1' is the zero digit: drop the leading ones, then add one per leading zero byte.
    let n = leading_zeros(x) + chars.len() - leading_ones(chars);
    let out = output.get_mut(..n).ok_or(BufferTooSmall)?;
    let src = &chars[chars.len() - n..];
    // `N <= n <= N + N / 2` (`check`; leading zero bytes take one character each, other
    // bytes at least one), so two fixed-size copies cover the result.
    out[..N].copy_from_slice(&src[..N]);
    let tail = n - N / 2;
    out[tail..].copy_from_slice(&src[tail..]);
    Ok(n)
}

/// The words of `x`, least significant first: `x = Σ w[k] 2^(32k)`.
#[inline(always)]
pub(super) fn words<const N: usize, const W: usize>(x: &[u8; N]) -> [u32; W] {
    const { assert!(N == 4 * W) };
    let c = x.as_chunks::<4>().0;
    core::array::from_fn(|k| u32::from_be_bytes(c[W - 1 - k]))
}

/// The characters of `x`'s value in `4 C` digits, top first, from separate kernels: `cols`
/// for the column sums, then [`carry`], then `digits`.
#[inline(always)]
pub(super) fn chars<const N: usize, const C: usize>(
    x: &[u8; N],
    out: &mut [[u8; 4]; C],
    cols: impl FnOnce(&[u8; N]) -> [u64; C],
    digits: impl FnOnce(&[u64], &[u32], &mut [[u8; 4]]),
) {
    let mut t = cols(x);
    let q = carry(&mut t, max_encoded_len(N).div_ceil(4));
    digits(&t, &q, out);
}

/// Carries the column sums `t[..r]`, from column 0 up: `t[j] += c`, then
/// `c = q[j] = t[j] / 58^4`. Returns `q`, truncated to 32 bits. Requires the value to fit
/// in `r` columns, and the columns from `r` on to be zero.
///
/// Column `j` is then `t[j] - 58^4 q[j]`, below 58^4, so the digit kernels can compute it
/// modulo 2^32 (NEON four columns per instruction); see [`column`]. Only the division is on
/// the chain.
#[inline(always)]
fn carry<const C: usize>(t: &mut [u64; C], r: usize) -> [u32; C] {
    let mut q = [0; C];
    let mut c = 0;
    for (t, q) in t[..r].iter_mut().zip(&mut q) {
        *t += c;
        c = *t / COL_BASE;
        *q = c as u32;
    }
    debug_assert_eq!(c, 0, "the value does not fit in the columns");
    q
}

/// A carried column, `t - 58^4 q` (see [`carry`]).
#[inline(always)]
fn column(t: u64, q: u32) -> u32 {
    (t as u32).wrapping_sub(q.wrapping_mul(COL_BASE as u32))
}

/// [`carry`], returning the carried columns.
#[cfg(any(
    all(target_arch = "x86_64", target_feature = "sse2"),
    all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ),
    test
))]
#[inline(always)]
pub(super) fn carry_serial<const C: usize>(t: &[u64; C], r: usize) -> [u32; C] {
    let mut out = [0; C];
    let mut c = 0;
    for (t, o) in t[..r].iter().zip(&mut out) {
        let v = t + c;
        c = v / COL_BASE;
        *o = column(v, c as u32);
    }
    debug_assert_eq!(c, 0, "the value does not fit in the columns");
    out
}

/// [`carry_serial`] with the chain cut into `K` segments that run in parallel, each from
/// a carry of zero. Then each segment's first column takes the carry out of the segment
/// below (one more division) and passes the difference, `d`, to its second column. That
/// column's carry changes only if `d` takes it to 58^4 or more: then `None`, and the
/// caller carries serially. `d` is below 2^14; at 64 bytes this happened for 0 to 6 of
/// 20000 random inputs (2 to 6 segments).
///
/// The chain is about `r / K` divisions instead of `r`. Requires at least two columns per
/// segment.
#[cfg(any(
    all(target_arch = "x86_64", target_feature = "sse2"),
    all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ),
    test
))]
#[inline(always)]
pub(super) fn carry_split<const C: usize, const K: usize>(
    t: &[u64; C],
    r: usize,
) -> Option<[u32; C]> {
    assert!(K >= 2 && 2 * K <= r && r <= C);
    let start = |s: usize| s * r / K;
    let mut out = [0; C];
    // Each segment's running carry, and its first column's quotient.
    let (mut c, mut first) = ([0; K], [0; K]);
    // Step `i` of every segment together, so that the chains interleave.
    for i in 0..r.div_ceil(K) {
        for s in 0..K {
            let j = start(s) + i;
            if j < start(s + 1) {
                let v = t[j] + c[s];
                c[s] = v / COL_BASE;
                out[j] = column(v, c[s] as u32);
                if i == 0 {
                    first[s] = c[s];
                }
            }
        }
    }
    let mut serial = false;
    for s in 1..K {
        let j = start(s);
        let v = t[j] + c[s - 1];
        let q = v / COL_BASE;
        out[j] = column(v, q as u32);
        // `d <= c[s - 1] / 58^4 + 1`, about 2^14 at most.
        let d = (q - first[s]) as u32;
        out[j + 1] += d;
        serial |= out[j + 1] >= COL_BASE as u32;
    }
    // From a carry of zero the top segment's carry is at most the true one, zero.
    debug_assert_eq!(c[K - 1], 0, "the value does not fit in the columns");
    (!serial).then_some(out)
}

/// The number of leading zero bytes of `x`.
#[inline(always)]
pub(super) fn leading_zeros<const N: usize>(x: &[u8; N]) -> usize {
    for (i, c) in x.as_chunks::<8>().0.iter().enumerate() {
        let w = u64::from_be_bytes(*c);
        if w != 0 {
            return 8 * i + w.leading_zeros() as usize / 8;
        }
    }
    N
}

/// The number of leading `'1'`s of `chars`. Requires a multiple of 8 bytes.
#[inline(always)]
fn leading_ones(chars: &[u8]) -> usize {
    for (i, c) in chars.as_chunks::<8>().0.iter().enumerate() {
        let w = u64::from_le_bytes(*c) ^ u64::from_le_bytes([b'1'; 8]);
        if w != 0 {
            return 8 * i + w.trailing_zeros() as usize / 8;
        }
    }
    chars.len()
}

#[cfg(kani)]
mod verification {
    use super::*;

    fn bounded_columns() -> ([u64; 4], u128) {
        let t: [u64; 4] = kani::any();
        for &v in &t[..3] {
            kani::assume(v < 1 << 58);
        }
        kani::assume(t[3] < COL_BASE);
        let value = t
            .iter()
            .rev()
            .fold(0u128, |v, &c| v * COL_BASE as u128 + c as u128);
        kani::assume(value < (COL_BASE as u128).pow(4));
        (t, value)
    }

    #[kani::proof]
    #[kani::unwind(5)]
    fn fixed_scalar_carry_preserves_value() {
        let (t, value) = bounded_columns();
        let want = carry_serial(&t, 4);
        let mut actual = 0u128;
        for &c in want.iter().rev() {
            assert!((c as u64) < COL_BASE);
            actual = actual * COL_BASE as u128 + c as u128;
        }
        assert_eq!(actual, value);
        let mut carried = t;
        let q = carry(&mut carried, 4);
        for i in 0..4 {
            assert_eq!(column(carried[i], q[i]), want[i]);
        }
    }

    #[kani::proof]
    #[kani::unwind(7)]
    fn fixed_split_carry_matches_serial() {
        let mut t: [u64; 6] = kani::any();
        for &v in &t[..4] {
            kani::assume(v < 1 << 58);
        }
        t[4..].fill(0);
        if let Some(out) = carry_split::<6, 3>(&t, 6) {
            let serial = carry_serial(&t, 6);
            for i in 0..6 {
                assert_eq!(out[i], serial[i]);
            }
        }
    }
}

/// `count` random 64-byte inputs for each segment but the first of [`carry_split`] with `k`
/// segments, on which it falls back: their base-58^4 digit above the segment's first
/// column is zero, so the carry from below takes that column to 58^4.
#[cfg(test)]
pub(super) fn split_fallbacks(
    rng: &mut rand_xoshiro::Xoshiro256PlusPlus,
    k: usize,
    count: usize,
) -> Vec<[u8; 64]> {
    use malachite_base::num::conversion::traits::{Digits, PowerOf2Digits};
    use malachite_nz::natural::Natural;

    let (base, r) = (Natural::from(COL_BASE), max_encoded_len(64).div_ceil(4));
    let mut out = vec![];
    for s in 1..k {
        for _ in 0..count {
            let x = crate::encode::tests::random_bytes(rng, 64).into_iter();
            let x = Natural::from_power_of_2_digits_desc(8, x).unwrap();
            let mut digits: Vec<Natural> = x.to_digits_asc(&base);
            digits.resize(r, Natural::from(0u32));
            digits[s * r / k + 1] = Natural::from(0u32);
            let y = Natural::from_digits_asc(&base, digits.into_iter()).unwrap();
            let bytes: Vec<u8> = y.to_power_of_2_digits_desc(8);
            let mut x = [0; 64];
            x[64 - bytes.len()..].copy_from_slice(&bytes);
            out.push(x);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use malachite_base::num::arithmetic::traits::{Pow, PowerOf2};
    use malachite_base::num::conversion::traits::Digits;
    use malachite_nz::natural::Natural;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    use super::portable::{cols, digits, encode_32_portable, encode_64_portable};
    use super::*;
    use crate::encode::tests::{Encode, encode_vec, random_bytes};
    use crate::internals::encode_batched;

    #[test]
    fn tables_hold_the_powers() {
        fn check<const W: usize, const C: usize>(t: &Table<W, C>) {
            let base = Natural::from(COL_BASE);
            for (k, row) in t.rows.iter().enumerate() {
                let v = Natural::from_digits_asc(&base, row.iter().map(|&d| Natural::from(d)));
                assert_eq!(v, Some(Natural::power_of_2(32 * k as u64)), "row {k}");
                for j in 0..C {
                    let first = t.rows.iter().position(|r| r[j..].iter().any(|&d| d != 0));
                    assert_eq!(t.start[j], first.unwrap_or(W), "start {j}");
                }
            }
        }
        check(&TABLE_32);
        check(&TABLE_64);
        assert_eq!(Natural::from(COL_BASE), Natural::from(58u32).pow(4));
    }

    /// Both sizes through each entry point. `best` runs the AVX-512 VBMI or AVX2 kernels
    /// when the CPU has them (the tests in `x86.rs` say when it has not), NEON on aarch64.
    const ENCODERS: [(&str, Encode); 4] = [
        ("auto", crate::encode),
        ("no_alloc", crate::no_alloc::encode),
        ("best", |d, o| match d.len() {
            32 => encode_32(d.try_into().unwrap(), o),
            _ => encode_64(d.try_into().unwrap(), o),
        }),
        ("portable", |d, o| match d.len() {
            32 => encode_32_portable(d.try_into().unwrap(), o),
            _ => encode_64_portable(d.try_into().unwrap(), o),
        }),
    ];

    /// Each encoder against bs58 into an ample buffer, an exact one, one a byte short and
    /// an empty one.
    fn check(data: &[u8]) {
        let want = bs58::encode(data).into_vec();
        assert_eq!(encode_vec(encode_batched, data), want, "batched, {data:?}");
        for (name, f) in ENCODERS {
            let at = format!("{name}: {} bytes, {data:?}", data.len());
            assert!(encode_vec(f, data) == want, "{at}");
            let mut exact = vec![0xAA; want.len()];
            assert_eq!(f(data, &mut exact), Ok(want.len()), "{at}: exact");
            assert!(exact == want, "{at}: exact");
            let mut short = vec![0xAA; want.len() - 1];
            assert_eq!(f(data, &mut short), Err(BufferTooSmall), "{at}: short");
            assert!(
                short.iter().all(|&b| b == 0xAA),
                "{at}: wrote into a short buffer"
            );
            assert_eq!(f(data, &mut []), Err(BufferTooSmall), "{at}: empty");
        }
    }

    #[test]
    fn random() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(40);
        for n in [32, 64] {
            for _ in 0..2000 {
                check(&random_bytes(&mut rng, n));
            }
        }
    }

    /// Every count of leading zero bytes, each before a small, a random and a large byte.
    #[test]
    fn leading_zeros() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(41);
        for n in [32, 64] {
            for zeros in 0..=n {
                for first in [1, 0x80, 0xFF, (rng.next_u32() as u8).max(1)] {
                    let mut data = random_bytes(&mut rng, n);
                    data[..zeros].fill(0);
                    if zeros < n {
                        data[zeros] = first;
                    }
                    check(&data);
                }
                // The smallest value after this many zero bytes: the shortest result.
                let mut low = vec![0; n];
                if zeros < n {
                    low[zeros] = 1;
                }
                check(&low);
            }
        }
    }

    /// All 0xFF gives the largest column sums and the longest result; also each byte value
    /// repeated, and powers of two at word and column edges.
    #[test]
    fn patterns() {
        for n in [32, 64] {
            for b in 0..=255 {
                check(&vec![b; n]);
            }
            for bit in 0..8 * n {
                let mut pow2 = vec![0; n];
                pow2[n - 1 - bit / 8] = 1 << (bit % 8);
                check(&pow2);
                let mut below = vec![0xFF; n];
                below[..n - 1 - bit / 8].fill(0);
                below[n - 1 - bit / 8] = ((1u16 << (bit % 8 + 1)) - 1) as u8;
                check(&below);
            }
        }
    }

    /// Columns `t - 58^4 q` with `t` past 2^32, against the general path's kernel.
    #[test]
    fn digits_of_carried_columns() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(43);
        for n in [1, 4, 12, 24, 100] {
            let cols: Vec<u64> = (0..n).map(|_| rng.next_u64() % COL_BASE).collect();
            let q: Vec<u32> = (0..n)
                .map(|i| if i % 3 == 0 { 0 } else { rng.next_u32() })
                .collect();
            let t: Vec<u64> = cols
                .iter()
                .zip(&q)
                .map(|(&c, &q)| c + COL_BASE * q as u64)
                .collect();
            let mut want = vec![[0; 4]; n];
            super::super::digits(&cols, &mut want);
            let mut got = vec![[0; 4]; n];
            digits(&t, &q, &mut got);
            assert_eq!(got, want, "{n} columns");
            #[cfg(all(
                target_arch = "aarch64",
                target_endian = "little",
                target_feature = "neon"
            ))]
            if n % 4 == 0 {
                let mut got = vec![[0; 4]; n];
                super::super::neon::fixed_digits(&t, &q, &mut got);
                assert_eq!(got, want, "{n} columns, NEON");
            }
        }
    }

    /// `carry_split`, when it returns, against `carry_serial`; returns whether it did.
    fn split_matches<const K: usize>(x: &[u8; 64]) -> bool {
        let (t, r) = (cols(x, &TABLE_64), max_encoded_len(64).div_ceil(4));
        let got = carry_split::<24, K>(&t, r);
        if let Some(got) = got {
            assert_eq!(got, carry_serial(&t, r), "{K} segments, {x:?}");
        }
        got.is_some()
    }

    fn split_matches_all(x: &[u8; 64]) -> [bool; 5] {
        [
            split_matches::<2>(x),
            split_matches::<3>(x),
            split_matches::<4>(x),
            split_matches::<5>(x),
            split_matches::<6>(x),
        ]
    }

    /// Random inputs, then inputs whose base-58^4 digit above a segment's first column is
    /// zero: those must fall back.
    #[test]
    fn split_carry_matches_serial() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(47);
        let mut fallbacks = [0; 5];
        let tries = 20000;
        for _ in 0..tries {
            let x: [u8; 64] = random_bytes(&mut rng, 64).try_into().unwrap();
            for (f, ok) in fallbacks.iter_mut().zip(split_matches_all(&x)) {
                *f += !ok as usize;
            }
        }
        eprintln!("fallbacks for 2 to 6 segments, of {tries} random inputs: {fallbacks:?}");
        for k in 2..=6 {
            for x in split_fallbacks(&mut rng, k, 20) {
                assert!(!split_matches_all(&x)[k - 2], "{k} segments, {x:?}");
                check(&x);
            }
        }
    }

    /// `benches/encode.rs`'s 64-byte input takes the split carry, so the bench times it
    /// and not the fallback.
    #[test]
    fn bench_input_takes_the_split_carry() {
        let mut x = [0; 64];
        Xoshiro256PlusPlus::seed_from_u64(64).fill_bytes(&mut x);
        x[0] |= 1;
        assert_eq!(split_matches_all(&x), [true; 5]);
    }

    /// The NEON column kernel against the plain-Rust one.
    #[cfg(all(
        target_arch = "aarch64",
        target_endian = "little",
        target_feature = "neon"
    ))]
    #[test]
    fn neon_cols_match_portable() {
        use super::super::neon;
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
        let inputs = (0..1000)
            .map(|_| random_bytes(&mut rng, 64))
            .chain([vec![0xFF; 64]]);
        for x in inputs {
            let (x64, x32): (&[u8; 64], &[u8; 32]) =
                (x[..].try_into().unwrap(), x[..32].try_into().unwrap());
            assert_eq!(neon::cols(x64, &TABLE_64), cols(x64, &TABLE_64), "{x:?}");
            assert_eq!(neon::cols(x32, &TABLE_32), cols(x32, &TABLE_32), "{x:?}");
        }
    }
}

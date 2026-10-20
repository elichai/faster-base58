//! [`encode_matrix`](super::encode_matrix) for exactly 32 or 64 bytes: the sizes of keys,
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

use super::four_digits;
use super::table::COL_BASE;
use crate::BufferTooSmall;
use crate::encode::max_encoded_len;

/// The powers `2^(32k)`, `k < W`, in base 58^4, in `C` columns.
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

/// [`encode_matrix`](super::encode_matrix) for 32 bytes. On x86_64 with the plain-Rust
/// kernels, which beat the AVX2 ones on Zen 5 (README, "Fixed sizes").
pub(super) fn encode_32(x: &[u8; 32], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    #[cfg(target_arch = "x86_64")]
    return encode_32_portable(x, output);
    #[cfg(not(target_arch = "x86_64"))]
    encode_best(x, output, &TABLE_32)
}

/// [`encode_matrix`](super::encode_matrix) for 64 bytes.
pub(super) fn encode_64(x: &[u8; 64], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    encode_best(x, output, &TABLE_64)
}

/// [`encode_32`] with the plain-Rust kernels.
pub(super) fn encode_32_portable(x: &[u8; 32], out: &mut [u8]) -> Result<usize, BufferTooSmall> {
    encode(x, out, |x, o| chars(x, o, |x| cols(x, &TABLE_32), digits))
}

/// [`encode_64`] with the plain-Rust kernels.
pub(super) fn encode_64_portable(x: &[u8; 64], out: &mut [u8]) -> Result<usize, BufferTooSmall> {
    encode(x, out, |x, o| chars(x, o, |x| cols(x, &TABLE_64), digits))
}

/// [`encode`] with the fastest kernels: AVX2 if the CPU has it on x86_64, NEON on
/// aarch64, plain Rust otherwise.
#[inline(always)]
fn encode_best<const N: usize, const W: usize, const C: usize>(
    x: &[u8; N],
    output: &mut [u8],
    table: &Table<W, C>,
) -> Result<usize, BufferTooSmall> {
    #[cfg(target_arch = "x86_64")]
    if is_x86_feature_detected!("avx2") {
        // SAFETY: the CPU has AVX2.
        return unsafe { super::x86::encode_fixed(x, output, table) };
    }
    #[cfg(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon"))]
    return encode(x, output, |x, o| {
        chars(x, o, |x| super::neon::cols(x, table), super::neon::fixed_digits)
    });
    #[allow(unreachable_code)]
    encode(x, output, |x, o| chars(x, o, |x| cols(x, table), digits))
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
    std::array::from_fn(|k| u32::from_be_bytes(c[W - 1 - k]))
}

/// The column sums `Σ w[k] rows[k][j]` of `x`'s words.
#[inline(always)]
pub(super) fn cols<const N: usize, const W: usize, const C: usize>(
    x: &[u8; N],
    t: &Table<W, C>,
) -> [u64; C] {
    let w = words::<N, W>(x);
    std::array::from_fn(|j| (t.start[j]..W).map(|k| w[k] as u64 * t.rows[k][j] as u64).sum())
}

/// The characters of `x`'s value in `4 C` digits, top first, from separate kernels: `cols`
/// for the column sums, then [`carry`], then `digits` (see [`digits`]).
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
/// modulo 2^32, four columns per instruction with SIMD; see [`column`]. Only the division
/// is on the chain.
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

/// The characters of the carried columns (see [`carry`]), top first: `out[i]` holds those
/// of column `n - 1 - i`. Requires `t`, `q` and `out` of one length.
#[inline(always)]
pub(super) fn digits(t: &[u64], q: &[u32], out: &mut [[u8; 4]]) {
    assert!(t.len() == q.len() && t.len() == out.len());
    for ((o, &t), &q) in out.iter_mut().zip(t.iter().rev()).zip(q.iter().rev()) {
        *o = four_digits(column(t, q) as u64);
    }
}

/// The number of leading zero bytes of `x`.
#[inline(always)]
fn leading_zeros<const N: usize>(x: &[u8; N]) -> usize {
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

#[cfg(test)]
mod tests {
    use malachite_base::num::arithmetic::traits::{Pow, PowerOf2};
    use malachite_base::num::conversion::traits::Digits;
    use malachite_nz::natural::Natural;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    use super::*;
    use crate::encode::tests::{Encode, encode_vec, random_bytes};
    use crate::{encode_batched, encode_matrix, encode_matrix_portable};

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

    /// Both sizes through each entry point. `best` runs the AVX2 kernels when the CPU has
    /// them (the tests in `x86.rs` say when it has not), NEON on aarch64.
    const ENCODERS: [(&str, Encode); 4] = [
        ("matrix", encode_matrix),
        ("matrix-portable", encode_matrix_portable),
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
            assert!(short.iter().all(|&b| b == 0xAA), "{at}: wrote into a short buffer");
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
            let q: Vec<u32> = (0..n).map(|i| if i % 3 == 0 { 0 } else { rng.next_u32() }).collect();
            let t: Vec<u64> = cols.iter().zip(&q).map(|(&c, &q)| c + COL_BASE * q as u64).collect();
            let mut want = vec![[0; 4]; n];
            super::super::digits(&cols, &mut want);
            let mut got = vec![[0; 4]; n];
            digits(&t, &q, &mut got);
            assert_eq!(got, want, "{n} columns");
            #[cfg(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon"))]
            if n % 4 == 0 {
                let mut got = vec![[0; 4]; n];
                super::super::neon::fixed_digits(&t, &q, &mut got);
                assert_eq!(got, want, "{n} columns, NEON");
            }
        }
    }

    /// The NEON column kernel against the plain-Rust one.
    #[cfg(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon"))]
    #[test]
    fn neon_cols_match_portable() {
        use super::super::neon;
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
        let inputs = (0..1000).map(|_| random_bytes(&mut rng, 64)).chain([vec![0xFF; 64]]);
        for x in inputs {
            let (x64, x32): (&[u8; 64], &[u8; 32]) =
                (x[..].try_into().unwrap(), x[..32].try_into().unwrap());
            assert_eq!(neon::cols(x64, &TABLE_64), cols(x64, &TABLE_64), "{x:?}");
            assert_eq!(neon::cols(x32, &TABLE_32), cols(x32, &TABLE_32), "{x:?}");
        }
    }
}

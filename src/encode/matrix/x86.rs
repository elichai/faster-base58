//! AVX2 column sums for the fixed sizes, with the contract of `fixed::cols`. `fixed.rs`
//! picks them at run time for 64 bytes.

use core::arch::x86_64::*;

use super::fixed::{self, TABLE_64, Table, words};
use super::four_digits;
use crate::BufferTooSmall;
use crate::encode::max_encoded_len;

/// `fixed::encode` with AVX2 column sums and the plain-Rust carry and digits, compiled for
/// AVX2. On Zen 5 an AVX2 digit kernel took longer than the plain-Rust one, at both sizes
/// (README, "Fixed sizes").
#[target_feature(enable = "avx2")]
pub(super) fn encode_fixed<const N: usize, const W: usize, const C: usize>(
    x: &[u8; N],
    output: &mut [u8],
    t: &Table<W, C>,
) -> Result<usize, BufferTooSmall> {
    fixed::encode(x, output, |x, o| fixed::chars(x, o, |x| cols(x, t), fixed::digits))
}

/// Kernel combinations for 64 bytes, for one Zen 5 sweep: `benches/encode.rs` times each.
/// Temporary: the winner replaces `encode_fixed` at 64 bytes.
#[doc(hidden)]
pub mod sweep {
    use super::*;

    type Encode = fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall>;

    macro_rules! variant {
        ($name:ident, $f:expr) => {
            fn $name(d: &[u8], o: &mut [u8]) -> Result<usize, BufferTooSmall> {
                assert!(is_x86_feature_detected!("avx2"), "the sweep needs AVX2");
                // SAFETY: the CPU has AVX2.
                unsafe { $f(d.try_into().expect("the sweep takes 64 bytes"), o) }
            }
        };
    }
    variant!(sums, encode_64_sums);
    variant!(split4, encode_64_stack::<4>);
    variant!(regs_split4, encode_64_regs::<4>);
    variant!(regs_split6, encode_64_regs::<6>);
    variant!(inline_split4, encode_64_inline::<4, false, 0>);
    variant!(count_split3, encode_64_inline::<3, true, 0>);
    variant!(count_split4, encode_64_inline::<4, true, 0>);
    variant!(count_split4_b, encode_64_inline::<4, true, 1>);
    variant!(count_split6, encode_64_inline::<6, true, 0>);

    /// `fixed-avx2-sums` is `encode_matrix`'s 64-byte path. `split` cuts the carry chain
    /// (`fixed::carry_split`); `regs` keeps the characters in registers; `inline` also
    /// writes them without a call; `count` also counts the leading ones from the carried
    /// columns. `-b` is a second copy of the code.
    pub const VARIANTS: [(&str, Encode); 9] = [
        ("fixed-avx2-sums", sums),
        ("fixed-split4", split4),
        ("fixed-regs-split4", regs_split4),
        ("fixed-regs-split6", regs_split6),
        ("fixed-inline-split4", inline_split4),
        ("fixed-count-split3", count_split3),
        ("fixed-count-split4", count_split4),
        ("fixed-count-split4-b", count_split4_b),
        ("fixed-count-split6", count_split6),
    ];
}

/// Columns that hold the value at 64 bytes: 88 characters.
const COLS_64: usize = max_encoded_len(64).div_ceil(4);

/// The carried columns of the sums `t`, with the chain in `K` segments (`K = 1`: serial);
/// `None` where `fixed::carry_split` falls back.
#[inline(always)]
fn carry<const K: usize>(t: &[u64; 24]) -> Option<[u32; 24]> {
    if K == 1 {
        return Some(fixed::carry_serial(t, COLS_64));
    }
    fixed::carry_split::<24, K>(t, COLS_64)
}

// The fallbacks below each carry serially and then write, so that the hot path's columns
// go from registers to the digit kernels and not through a stack array both paths write.

/// [`encode_fixed`] at 64 bytes.
#[target_feature(enable = "avx2")]
fn encode_64_sums(x: &[u8; 64], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    encode_fixed(x, output, &TABLE_64)
}

/// 64 bytes with AVX2 sums, the carry in `K` segments, and the plain-Rust digits through a
/// stack buffer (`fixed::encode`).
#[target_feature(enable = "avx2")]
fn encode_64_stack<const K: usize>(
    x: &[u8; 64],
    output: &mut [u8],
) -> Result<usize, BufferTooSmall> {
    let t = cols(x, &TABLE_64);
    match carry::<K>(&t) {
        Some(r) => write_stack(x, &r, output),
        None => stack_cold(x, &t, output),
    }
}

#[cold]
#[inline(never)]
#[target_feature(enable = "avx2")]
fn stack_cold(x: &[u8; 64], t: &[u64; 24], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    write_stack(x, &fixed::carry_serial(t, COLS_64), output)
}

#[inline]
#[target_feature(enable = "avx2")]
fn write_stack(x: &[u8; 64], r: &[u32; 24], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    fixed::encode(x, output, |_, o: &mut [[u8; 4]; 24]| {
        for (o, &r) in o.iter_mut().zip(r.iter().rev()) {
            *o = four_digits(r as u64);
        }
    })
}

/// 64 bytes with AVX2 sums, the carry in `K` segments, and AVX2 digits that stay in
/// registers until three stores write the result.
#[target_feature(enable = "avx2")]
fn encode_64_regs<const K: usize>(
    x: &[u8; 64],
    output: &mut [u8],
) -> Result<usize, BufferTooSmall> {
    let t = cols(x, &TABLE_64);
    // Vectors, not the array: an array both paths write would go through the stack, and
    // the vector loads from it would not get the columns forwarded from the stores.
    let c = match carry::<K>(&t) {
        Some(r) => lanes(&r),
        None => lanes_cold(&t),
    };
    write_regs(x, &c, output)
}

/// [`encode_64_regs`] with the writing inlined: no call, and nothing staged on the stack.
/// `SCALAR`: the leading zero digits from the carried columns, off the vector path, and not
/// from the digit vectors. `COPY = 1` makes the last store first (it does not overlap the
/// first), only so that this copy of the code is not merged with the other: a probe of
/// what placement alone changes.
#[target_feature(enable = "avx2")]
fn encode_64_inline<const K: usize, const SCALAR: bool, const COPY: usize>(
    x: &[u8; 64],
    output: &mut [u8],
) -> Result<usize, BufferTooSmall> {
    let t = cols(x, &TABLE_64);
    let (c, zeros) = match carry::<K>(&t) {
        Some(r) => (lanes(&r), zero_digits(&r)),
        None => lanes_zeros_cold(&t),
    };
    let d = [group(c[0], c[1]), group(c[2], c[3]), group(c[4], c[5])];
    let zeros = if SCALAR { zeros } else { zero_digits_v(&d) };
    // `write`, here so that it is inlined: LLVM keeps it out of line where it has several
    // callers, and `#[inline(always)]` is not allowed with `#[target_feature]`.
    let n = fixed::leading_zeros(x) + 96 - zeros;
    let out = output.get_mut(..n).ok_or(BufferTooSmall)?;
    let skip = 96 - n;
    let [c0, c1, c2] = d.map(|d| alphabet(d));
    if COPY == 1 {
        store(out[64 - skip..].first_chunk_mut().unwrap(), c2);
    }
    store(out.first_chunk_mut().unwrap(), shift(c0, skip));
    store(out[32 - skip..].first_chunk_mut().unwrap(), c1);
    if COPY != 1 {
        store(out[64 - skip..].first_chunk_mut().unwrap(), c2);
    }
    Ok(n)
}

#[cold]
#[inline(never)]
#[target_feature(enable = "avx2")]
fn lanes_zeros_cold(t: &[u64; 24]) -> ([__m256i; 6], usize) {
    let r = fixed::carry_serial(t, COLS_64);
    (lanes(&r), zero_digits(&r))
}

/// The leading zero digits of the 96 digits of the carried columns `r`.
#[inline(always)]
fn zero_digits(r: &[u32; 24]) -> usize {
    let Some(j) = r.iter().rposition(|&c| c != 0) else {
        return 96;
    };
    let c = r[j];
    let digits = 1 + (c >= 58) as usize + (c >= 58 * 58) as usize + (c >= 58 * 58 * 58) as usize;
    4 * (24 - j) - digits
}

/// The leading zero digits of the 96 digits `d`, top first.
#[target_feature(enable = "avx2")]
fn zero_digits_v(d: &[__m256i; 3]) -> usize {
    let zero = d.map(|d| _mm256_movemask_epi8(_mm256_cmpeq_epi8(d, _mm256_setzero_si256())));
    let zero = zero.map(|z| z as u32);
    let top = zero[0] as u64 | (zero[1] as u64) << 32;
    let n = if top != u64::MAX { top.trailing_ones() } else { 64 + zero[2].trailing_ones() };
    n as usize
}

/// Writes `x`'s value from its 96 digits `d`, top first, of which `zeros` lead.
#[target_feature(enable = "avx2")]
fn write(
    x: &[u8; 64],
    d: [__m256i; 3],
    zeros: usize,
    output: &mut [u8],
) -> Result<usize, BufferTooSmall> {
    // '1' is the zero digit: drop the leading ones, then add one per leading zero byte.
    let n = fixed::leading_zeros(x) + 96 - zeros;
    let out = output.get_mut(..n).ok_or(BufferTooSmall)?;
    // `64 <= n <= 88` (`fixed::Table::check`), so the stores below are in `out` and the
    // last ends at `n`. The second overwrites the first one's excess.
    let skip = 96 - n;
    let [c0, c1, c2] = d.map(|d| alphabet(d));
    store(out.first_chunk_mut().unwrap(), shift(c0, skip));
    store(out[32 - skip..].first_chunk_mut().unwrap(), c1);
    store(out[64 - skip..].first_chunk_mut().unwrap(), c2);
    Ok(n)
}

#[cold]
#[inline(never)]
#[target_feature(enable = "avx2")]
fn lanes_cold(t: &[u64; 24]) -> [__m256i; 6] {
    lanes(&fixed::carry_serial(t, COLS_64))
}

/// The carried columns `r` in the lanes [`group`] takes, top first.
#[target_feature(enable = "avx2")]
fn lanes(r: &[u32; 24]) -> [__m256i; 6] {
    let v = |a: usize, b, c, d| {
        _mm256_setr_epi64x(r[a] as i64, r[b] as i64, r[c] as i64, r[d] as i64)
    };
    [
        v(23, 22, 19, 18),
        v(21, 20, 17, 16),
        v(15, 14, 11, 10),
        v(13, 12, 9, 8),
        v(7, 6, 3, 2),
        v(5, 4, 1, 0),
    ]
}

/// Writes `x`'s value from its carried columns `c` (see [`lanes`]) with AVX2.
#[target_feature(enable = "avx2")]
fn write_regs(
    x: &[u8; 64],
    c: &[__m256i; 6],
    output: &mut [u8],
) -> Result<usize, BufferTooSmall> {
    // The 96 digits, top first: columns 23 to 16, 15 to 8, 7 to 0.
    let d = [group(c[0], c[1]), group(c[2], c[3]), group(c[4], c[5])];
    write(x, d, zero_digits_v(&d), output)
}

#[target_feature(enable = "avx2")]
fn store(out: &mut [u8; 32], v: __m256i) {
    // SAFETY: `out` holds 32 bytes; the store is unaligned.
    unsafe { _mm256_storeu_si256(out.as_mut_ptr().cast(), v) };
}

/// The bytes of `v` from `n` on, `n <= 32`, followed by unspecified bytes.
#[target_feature(enable = "avx2")]
fn shift(v: __m256i, n: usize) -> __m256i {
    // Dword `i` is bytes `n..n + 4` of dwords `i + n / 4` and `i + n / 4 + 1`. Indices
    // past 7 wrap, into the unspecified bytes.
    let i = _mm256_set1_epi32(n as i32 / 4);
    let i = _mm256_add_epi32(_mm256_setr_epi32(0, 1, 2, 3, 4, 5, 6, 7), i);
    let lo = _mm256_permutevar8x32_epi32(v, i);
    let hi = _mm256_permutevar8x32_epi32(v, _mm256_add_epi32(i, _mm256_set1_epi32(1)));
    let bits = _mm256_set1_epi32(8 * (n % 4) as i32);
    // Shifts by 32 bits give zero.
    let hi = _mm256_sllv_epi32(hi, _mm256_sub_epi32(_mm256_set1_epi32(32), bits));
    _mm256_or_si256(_mm256_srlv_epi32(lo, bits), hi)
}

/// `ceil(2^36 / 3364)`: `(c * M) >> 36 = c / 3364` for every `c < 58^4`.
const DIV_3364: i64 = 20427907;
/// `ceil(2^18 / 58)`: `(x * M) >> 18 = x / 58` for every `x < 3364`.
const DIV_58: i16 = 4520;

/// The 32 digits, top first, of eight carried columns `j..j + 8`: `a` holds columns
/// `j + 7, j + 6, j + 3, j + 2` and `b` columns `j + 5, j + 4, j + 1, j`, one per 64-bit
/// lane. So 128-bit lane 0 has columns `j + 7` to `j + 4` and lane 1 the rest, and no step
/// crosses lanes.
#[target_feature(enable = "avx2")]
fn group(a: __m256i, b: __m256i) -> __m256i {
    let (a, b) = (digits(a), digits(b));
    // `b`'s digits to bytes 2, 3, 6 and 7 of each 64-bit lane, beside `a`'s.
    let d = _mm256_or_si256(a, _mm256_slli_epi64::<16>(b));
    #[rustfmt::skip]
    let order = _mm256_setr_epi8(
        0, 1, 4, 5, 8, 9, 12, 13, 2, 3, 6, 7, 10, 11, 14, 15,
        0, 1, 4, 5, 8, 9, 12, 13, 2, 3, 6, 7, 10, 11, 14, 15,
    );
    _mm256_shuffle_epi8(d, order)
}

/// The digits of four columns below 58^4, one per 64-bit lane: bytes 0, 1, 4 and 5 of
/// each lane, top first; the other bytes are zero.
#[target_feature(enable = "avx2")]
fn digits(c: __m256i) -> __m256i {
    let q = _mm256_srli_epi64::<36>(_mm256_mul_epu32(c, _mm256_set1_epi64x(DIV_3364)));
    let r = _mm256_sub_epi64(c, _mm256_mul_epu32(q, _mm256_set1_epi64x(3364)));
    // u16 lanes `q, 0, r, 0` per column, then `x / 58` and `x % 58` of each.
    let x = _mm256_or_si256(q, _mm256_slli_epi64::<32>(r));
    let a = _mm256_srli_epi16::<2>(_mm256_mulhi_epu16(x, _mm256_set1_epi16(DIV_58)));
    let b = _mm256_sub_epi16(x, _mm256_mullo_epi16(a, _mm256_set1_epi16(58)));
    _mm256_or_si256(a, _mm256_slli_epi16::<8>(b))
}

/// The characters of 32 digits below 58: `'1' + d`, plus the alphabet's gaps before
/// 'A' (7), 'J', 'P', 'a' (6) and 'm'.
#[target_feature(enable = "avx2")]
fn alphabet(d: __m256i) -> __m256i {
    // -1 where `d > n`.
    let gt = |n: i8| _mm256_cmpgt_epi8(d, _mm256_set1_epi8(n));
    let wide = _mm256_add_epi8(
        _mm256_and_si256(gt(8), _mm256_set1_epi8(7)),
        _mm256_and_si256(gt(32), _mm256_set1_epi8(6)),
    );
    let narrow = _mm256_add_epi8(_mm256_add_epi8(gt(16), gt(21)), gt(43));
    let base = _mm256_add_epi8(d, _mm256_set1_epi8(b'1' as i8));
    _mm256_sub_epi8(_mm256_add_epi8(base, wide), narrow)
}

/// The column sums of `x`'s words, four columns per block.
#[target_feature(enable = "avx2")]
fn cols<const N: usize, const W: usize, const C: usize>(x: &[u8; N], t: &Table<W, C>) -> [u64; C] {
    let w = words::<N, W>(x);
    let mut out = [0; C];
    for (b, o) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let mut s = _mm256_setzero_si256();
        let first = t.start[4 * b];
        for (&w, row) in w[first..].iter().zip(&t.rows[first..]) {
            let e = &row[4 * b..][..4];
            // SAFETY: `e` holds 16 bytes; the load is unaligned.
            let e = _mm256_cvtepu32_epi64(unsafe { _mm_loadu_si128(e.as_ptr().cast()) });
            // `vpmuludq` multiplies the low 32 bits of each lane.
            s = _mm256_add_epi64(s, _mm256_mul_epu32(e, _mm256_set1_epi64x(w as i64)));
        }
        // SAFETY: `o` holds 32 bytes; the store is unaligned.
        unsafe { _mm256_storeu_si256(o.as_mut_ptr().cast(), s) };
    }
    out
}

#[cfg(test)]
mod tests {
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    use super::super::fixed::TABLE_32;
    use super::super::table::COL_BASE;
    use super::*;
    use crate::common::ALPHABET;

    fn avx2() -> bool {
        let yes = is_x86_feature_detected!("avx2");
        if !yes {
            eprintln!("no AVX2 on this CPU: AVX2 kernels not run");
        }
        yes
    }

    #[test]
    fn cols_match_portable() {
        if !avx2() {
            return;
        }
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(45);
        let mut inputs: Vec<[u8; 64]> = [0, 1, 0x80, 0xFF].map(|b| [b; 64]).to_vec();
        inputs.extend((0..1000).map(|_| std::array::from_fn(|_| rng.next_u32() as u8)));
        for x in &inputs {
            let half: &[u8; 32] = x[..32].try_into().unwrap();
            // SAFETY: the CPU has AVX2.
            unsafe {
                assert_eq!(cols(x, &TABLE_64), fixed::cols(x, &TABLE_64), "{x:?}");
                assert_eq!(cols(half, &TABLE_32), fixed::cols(half, &TABLE_32), "{x:?}");
            }
        }
    }

    /// Every column value below 58^4, in all four lanes, against the scalar kernel.
    #[test]
    fn digits_every_value() {
        if !avx2() {
            return;
        }
        let mut lanes = [0u64; 4];
        for c in 0..COL_BASE {
            lanes[c as usize % 4] = c;
            if c % 4 != 3 && c != COL_BASE - 1 {
                continue;
            }
            // SAFETY: the CPU has AVX2; the loads and stores are unaligned, 32 bytes.
            let got: [[u8; 8]; 4] = unsafe {
                let d = digits(_mm256_loadu_si256(lanes.as_ptr().cast()));
                std::mem::transmute(d)
            };
            for (l, (&c, g)) in lanes.iter().zip(got).enumerate() {
                let digit = |ch| ALPHABET.iter().position(|&a| a == ch).unwrap() as u8;
                let want = four_digits(c).map(digit);
                assert_eq!([g[0], g[1], g[4], g[5]], want, "{c} in lane {l}");
                assert_eq!([g[2], g[3], g[6], g[7]], [0; 4], "{c} in lane {l}");
            }
        }
    }

    /// Every digit through `alphabet`, at every byte.
    #[test]
    fn alphabet_every_digit() {
        if !avx2() {
            return;
        }
        for first in 0..58u8 {
            let d: [u8; 32] = std::array::from_fn(|i| (first + i as u8) % 58);
            // SAFETY: the CPU has AVX2.
            let got: [u8; 32] = unsafe {
                std::mem::transmute(alphabet(std::mem::transmute::<[u8; 32], __m256i>(d)))
            };
            let want = d.map(|d| ALPHABET[d as usize]);
            assert_eq!(got, want);
        }
    }

    #[test]
    fn shift_every_count() {
        if !avx2() {
            return;
        }
        let v: [u8; 32] = std::array::from_fn(|i| i as u8 + 100);
        for n in 0..=32 {
            // SAFETY: the CPU has AVX2.
            let got: [u8; 32] = unsafe {
                std::mem::transmute(shift(std::mem::transmute::<[u8; 32], __m256i>(v), n))
            };
            assert_eq!(got[..32 - n], v[n..], "shift by {n}");
        }
    }

    /// Each variant against bs58: random inputs, every count of leading zero bytes, all
    /// 0xFF; into an ample buffer (nothing written past the result), an exact one and one
    /// a byte short (nothing written).
    #[test]
    fn sweep_matches_bs58() {
        if !avx2() {
            return;
        }
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(46);
        let mut random = || -> [u8; 64] { std::array::from_fn(|_| rng.next_u32() as u8) };
        let mut inputs: Vec<[u8; 64]> = (0..3000).map(|_| random()).collect();
        for zeros in 0..=64 {
            let mut x = random();
            x[..zeros].fill(0);
            if zeros < 64 {
                x[zeros] = x[zeros].max(1);
                inputs.push(x);
                x[zeros] = 1;
                x[zeros + 1..].fill(0);
            }
            inputs.push(x);
        }
        inputs.extend([0x80, 0xFF].map(|b| [b; 64]));
        // The serial fallbacks.
        for k in [2, 3, 4, 6] {
            inputs.extend(fixed::split_fallbacks(&mut rng, k, 5));
        }
        for x in &inputs {
            let want = bs58::encode(x).into_vec();
            for (name, f) in sweep::VARIANTS {
                let at = format!("{name}: {x:?}");
                let mut out = vec![0xAA; want.len() + 40];
                assert_eq!(f(x, &mut out), Ok(want.len()), "{at}");
                assert!(out[..want.len()] == want[..], "{at}");
                let past = out[want.len()..].iter().any(|&b| b != 0xAA);
                assert!(!past, "{at}: wrote past the result");
                assert_eq!(f(x, &mut out[..want.len()]), Ok(want.len()), "{at}: exact");
                let mut short = vec![0xAA; want.len() - 1];
                assert_eq!(f(x, &mut short), Err(BufferTooSmall), "{at}: short");
                assert!(short.iter().all(|&b| b == 0xAA), "{at}: wrote into a short buffer");
            }
        }
    }
}

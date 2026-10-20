//! Divide-and-conquer radix conversion: `value(hi) * BASE^e + value(lo)`, with balanced
//! splits. [`decode`] multiplies with Karatsuba; [`decode_with`] takes the multiplication.

use crate::batched::decode_batched;
use crate::common::{BASE, CHUNK, all_valid, chunks, invalid, split_zeros};
use crate::{Error, matrix, mul};

/// Inputs with more chunks than this use divide-and-conquer. Below it,
/// [`decode_batched`] is faster.
pub(crate) const DC_THRESHOLD: usize = 512;

/// Leaves have at most this many chunks.
pub(crate) const LEAF: usize = 256;
const _: () = assert!(LEAF <= DC_THRESHOLD);

/// Like [`decode_batched`], but subquadratic for large inputs.
///
/// Makes one allocation, for the result, the power table and all scratch space.
pub fn decode(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    decode_with(input, output, &mut Karatsuba)
}

/// A multiplication for [`decode_with`].
pub(crate) trait Multiplier {
    /// Called once per decode, before any [`Multiplier::mul`]. No product will be longer
    /// than `len` limbs: `a.len() + b.len() <= len`.
    fn prepare(&mut self, _len: usize) {}

    /// Scratch limbs [`Multiplier::mul`] needs when no operand is longer than `n` limbs.
    /// [`dc_scratch_len`] relies on `n / 2 + scratch_len(n / 2) <= scratch_len(n)`,
    /// which holds for any `c * n + d` with `c >= 1`.
    fn scratch_len(&self, n: usize) -> usize;

    /// `out = a * b`. Requires `out.len() == a.len() + b.len()` and
    /// `scratch.len() >= scratch_len(max(a.len(), b.len()))`. `scratch` may hold anything.
    fn mul(&mut self, out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]);

    /// Power `i` of the table is `b`, and its products through [`Multiplier::mul_power`]
    /// will be at most `len >= b.len()` limbs long, so the multiplier may keep work on `b`
    /// for them. Called after [`Multiplier::prepare`] for each power but the last, in
    /// increasing `i`.
    fn prepare_power(&mut self, _i: usize, _b: &[u64], _len: usize) {}

    /// `out = a * b`, where `b` is power `i` of the table: the same slice that
    /// [`Multiplier::prepare_power`] got, if that power was prepared. Otherwise as
    /// [`Multiplier::mul`].
    fn mul_power(&mut self, out: &mut [u64], a: &[u64], _i: usize, b: &[u64], s: &mut [u64]) {
        self.mul(out, a, b, s);
    }

    /// A length `L >= n` for which [`Multiplier::mul_wrapped`] is cheaper than a product of
    /// `full` limbs, if there is one. By default there is none.
    fn wrap_len(&self, _n: usize, _full: usize) -> Option<usize> {
        None
    }

    /// Scratch limbs [`Multiplier::mul_wrapped`] needs for `L` limbs when no operand is
    /// longer than `n`.
    fn wrapped_scratch_len(&self, _l: usize, n: usize) -> usize {
        2 * n + self.scratch_len(n)
    }

    /// `out = a * b mod (β^L - 1)` for `L = out.len() > 0`, as some value in
    /// `[0, β^L - 1]` (both ends stand for zero). Needs `scratch.len() >=
    /// wrapped_scratch_len(L, max(a.len(), b.len()))`. By default, the full product
    /// folded.
    fn mul_wrapped(&mut self, out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]) {
        let (p, s) = scratch.split_at_mut(a.len() + b.len());
        self.mul(p, a, b, s);
        mul::fold_wrapped(out, p);
    }

    /// [`Multiplier::mul_wrapped`] where `b` is power `i` of the table, as in
    /// [`Multiplier::mul_power`].
    fn mul_power_wrapped(&mut self, out: &mut [u64], a: &[u64], _i: usize, b: &[u64], s: &mut [u64]) {
        self.mul_wrapped(out, a, b, s);
    }
}

/// [`mul::mul`] as a [`Multiplier`].
pub(crate) struct Karatsuba;

impl Multiplier for Karatsuba {
    fn scratch_len(&self, n: usize) -> usize {
        mul::scratch_len(n)
    }

    fn mul(&mut self, out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]) {
        mul::mul(out, a, b, scratch);
    }
}

/// How [`dc`] converts its leaves.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Leaf {
    /// Chunk parse and [`mul_add_chunks`].
    Horner,
    /// [`matrix::leaf`]. Faster with its table in cache, but the table is built on first
    /// use and kept for the life of the process, so only decoders that cache anyway use it.
    Matrix,
}

impl Leaf {
    /// [`Leaf::Matrix`] where it was measured faster: NEON. x86_64 is not measured yet.
    pub(crate) const BEST: Leaf =
        if cfg!(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon")) {
            Leaf::Matrix
        } else {
            Leaf::Horner
        };
}

/// [`decode`], multiplying with `m`.
pub(crate) fn decode_with(
    input: &str,
    output: &mut [u8],
    m: &mut impl Multiplier,
) -> Result<usize, Error> {
    decode_with_leaf(input, output, m, Leaf::Horner)
}

/// [`decode_with`], with leaves converted by `leaf`.
pub(crate) fn decode_with_leaf(
    input: &str,
    output: &mut [u8],
    m: &mut impl Multiplier,
    leaf: Leaf,
) -> Result<usize, Error> {
    let (zeros, digits) = split_zeros(input.as_bytes());
    let n = digits.len().div_ceil(CHUNK);
    if n <= DC_THRESHOLD {
        return decode_batched(input, output);
    }
    // Fail before allocating or computing anything. The first digit is not '1', so the
    // value is at least 58^(d - 1), which takes more than (d - 1) * 0.732 bytes.
    let min_len = ((digits.len() - 1) as u128 * 732 / 1000) as usize + 1;
    if output.len() < zeros + min_len {
        return Err(Error::BufferTooSmall);
    }
    if !all_valid(digits) {
        return Err(invalid(input));
    }
    let (unit, levels) = split_unit(n);
    // Powers have at most `e` limbs, and `hi` at most `n - e <= e`.
    let e = unit << (levels - 1);
    // Each product is `hi * BASE^e'` with `hi < BASE^e` (`BASE < 2^59`) and `e' <= e`,
    // or the square of a smaller power.
    m.prepare(product_len(e, stored(e)));
    let table_len = table_len(unit, levels);
    let mut buf = vec![0; n + table_len + dc_scratch_len(m, e)];
    let (limbs, rest) = buf.split_at_mut(n);
    let (table, scratch) = rest.split_at_mut(table_len);
    let powers = powers(unit, levels, table, scratch, m, true);
    let len = dc(limbs, digits, &powers[..levels], scratch, m, leaf);
    write_bytes(zeros, &limbs[..len], output)
}

/// `limbs[..len] = limbs[..len] * BASE + c`. Returns the new length.
///
/// This and [`mul_add_base2`] repeat `batched.rs`'s kernel, which works on output
/// bytes, for `u64` limbs.
fn mul_add_base(limbs: &mut [u64], len: usize, c: u64) -> usize {
    let mut carry = c;
    for l in &mut limbs[..len] {
        (*l, carry) = l.carrying_mul(BASE, carry);
    }
    if carry == 0 {
        return len;
    }
    limbs[len] = carry;
    len + 1
}

/// `limbs[..len] = (limbs[..len] * BASE + c1) * BASE + c2`. Returns the new length.
///
/// The second pass runs one limb behind the first, so the CPU overlaps their carry
/// chains: about twice the throughput of two [`mul_add_base`] calls.
fn mul_add_base2(limbs: &mut [u64], len: usize, c1: u64, c2: u64) -> usize {
    let l = &mut limbs[..len];
    let Some(&first) = l.first() else {
        let len = mul_add_base(limbs, 0, c1);
        return mul_add_base(limbs, len, c2);
    };
    // `p` is the previous limb after the first pass only.
    let (mut p, mut k1) = first.carrying_mul(BASE, c1);
    let mut k2 = c2;
    for i in 1..l.len() {
        let (a, h1) = l[i].carrying_mul(BASE, k1);
        let (b, h2) = p.carrying_mul(BASE, k2);
        l[i - 1] = b;
        (p, k1, k2) = (a, h1, h2);
    }
    (l[len - 1], k2) = p.carrying_mul(BASE, k2);
    // The first pass's carry, if non-zero, is its new top limb.
    let mut len = len;
    if k1 != 0 {
        (limbs[len], k2) = k1.carrying_mul(BASE, k2);
        len += 1;
    }
    if k2 != 0 {
        limbs[len] = k2;
        len += 1;
    }
    len
}

/// Folds `chunks` (most significant first) into `limbs[..len]`. Returns the new length.
fn mul_add_chunks(limbs: &mut [u64], len: usize, chunks: &[u64]) -> usize {
    let (pairs, last) = chunks.as_chunks::<2>();
    let len = pairs.iter().fold(len, |len, &[c1, c2]| mul_add_base2(limbs, len, c1, c2));
    last.iter().fold(len, |len, &c| mul_add_base(limbs, len, c))
}

/// Writes `zeros` zero bytes, then `limbs` (trimmed, little-endian) as big-endian bytes.
pub(crate) fn write_bytes(zeros: usize, limbs: &[u64], output: &mut [u8]) -> Result<usize, Error> {
    let top = limbs.last().map_or(0, |l| l.leading_zeros() as usize / 8);
    let total = zeros + limbs.len() * 8 - top;
    let out = output.get_mut(..total).ok_or(Error::BufferTooSmall)?;
    let (z, rest) = out.split_at_mut(zeros);
    z.fill(0);
    for (dst, l) in rest.rchunks_mut(8).zip(limbs) {
        dst.copy_from_slice(&l.to_be_bytes()[8 - dst.len()..]);
    }
    Ok(total)
}

/// `BASE^exp`, stored as `limbs` shifted left by `shift` limbs.
/// `58 = 2 * 29`, so large powers end in zero limbs; dropping them shortens products.
#[derive(Clone, Copy, Default)]
pub(crate) struct Power<'a> {
    pub(crate) exp: usize,
    pub(crate) limbs: &'a [u64],
    pub(crate) shift: usize,
}

impl<'a> Power<'a> {
    fn new(exp: usize, limbs: &'a [u64], shift: usize) -> Self {
        let limbs = &limbs[..trimmed_len(limbs)];
        let zeros = limbs.iter().take_while(|&&x| x == 0).count();
        Power { exp, limbs: &limbs[zeros..], shift: shift + zeros }
    }
}

/// More than [`split_unit`] can return for any `usize`.
pub(crate) const MAX_LEVELS: usize = usize::BITS as usize;

/// The smallest `levels` with `unit = ceil(n / 2^levels) <= LEAF`, and that `unit`.
///
/// [`dc`] splits at `unit * 2^i` chunks for `i < levels`, so the top split is at about
/// `n / 2` and every level is balanced.
pub(crate) fn split_unit(n: usize) -> (usize, usize) {
    split_unit_leaf(n, LEAF)
}

/// [`split_unit`] with leaves of at most `leaf` chunks.
pub(crate) fn split_unit_leaf(n: usize, leaf: usize) -> (usize, usize) {
    let (mut unit, mut levels) = (n, 0);
    while unit > leaf {
        unit = unit.div_ceil(2);
        levels += 1;
    }
    (unit, levels)
}

/// Limbs of `table` that [`powers`] uses.
///
/// Slot 0 holds `BASE^unit`, at most `unit` limbs; slot `i` holds the square of power
/// `i - 1`, so twice its [`stored`] length.
pub(crate) fn table_len(unit: usize, levels: usize) -> usize {
    (1..levels).map(|i| 2 * stored(unit << (i - 1))).sum::<usize>() + unit
}

/// At most the limbs a [`Power`] keeps of `BASE^e`. `BASE < 2^59`, and
/// `BASE^e = 2^(10e) * 29^(10e)` ends in exactly `10e` zero bits, so dropping its low
/// zero limbs leaves at most `ceil(59e/64) - floor(10e/64)`.
fn stored(e: usize) -> usize {
    (59 * e).div_ceil(64) - 10 * e / 64
}

/// The longest product with the power `BASE^e` when it is stored in `stored` limbs: the
/// other operand is below `BASE^e < 2^(59 e)` (see [`dc`]).
fn product_len(e: usize, stored: usize) -> usize {
    (59 * e).div_ceil(64) + stored
}

/// `BASE^(unit * 2^i)` for `i < levels`, stored in `table`, which needs
/// [`table_len`] limbs. With `prepare`, hands each power but the last to
/// [`Multiplier::prepare_power`] for [`dc`]'s products.
pub(crate) fn powers<'a>(
    unit: usize,
    levels: usize,
    mut table: &'a mut [u64],
    scratch: &mut [u64],
    m: &mut impl Multiplier,
    prepare: bool,
) -> [Power<'a>; MAX_LEVELS] {
    let mut powers = [Power::default(); MAX_LEVELS];
    let p;
    (p, table) = table.split_at_mut(unit);
    p[0] = 1;
    // BASE^unit, at most LEAF chunks at a time: the encoders' units can be larger.
    let zeros = [0; LEAF];
    let len = (0..unit).step_by(LEAF).fold(1, |len, i| {
        mul_add_chunks(p, len, &zeros[..LEAF.min(unit - i)])
    });
    powers[0] = Power::new(unit, &p[..len], 0);
    for i in 1..levels {
        let last = powers[i - 1];
        if prepare {
            m.prepare_power(i - 1, last.limbs, product_len(last.exp, last.limbs.len()));
        }
        let sq;
        (sq, table) = table.split_at_mut(2 * last.limbs.len());
        m.mul(sq, last.limbs, last.limbs, scratch);
        powers[i] = Power::new(2 * last.exp, sq, 2 * last.shift);
    }
    powers
}

/// Upper bound on the scratch limbs [`dc`] (and [`powers`]) use when the top split is
/// at `e` chunks.
///
/// A node splitting at `e` chunks holds `e` limbs while it multiplies operands of at
/// most `e` limbs or recurses into children that split at most at `e / 2`. By
/// induction, using `e / 2 + m.scratch_len(e / 2) <= m.scratch_len(e)`, it uses at most
/// `e + m.scratch_len(e)` limbs.
fn dc_scratch_len(m: &impl Multiplier, e: usize) -> usize {
    e + m.scratch_len(e)
}

/// Writes the value of `digits`, which must be valid, to `out` as little-endian limbs
/// and returns its trimmed length. Requires one `out` limb per chunk of `digits`, and
/// `dc_scratch_len(m, e)` limbs of `scratch` for the top split `e`. `out` and `scratch`
/// may hold anything on entry.
fn dc(
    out: &mut [u64],
    digits: &[u8],
    powers: &[Power<'_>],
    scratch: &mut [u64],
    m: &mut impl Multiplier,
    leaf: Leaf,
) -> usize {
    let n = out.len();
    if n <= LEAF {
        if leaf == Leaf::Matrix {
            return matrix::leaf(out, digits).expect("`decode_with` validates the digits");
        }
        let mut parsed = [0u64; LEAF];
        for (p, chunk) in parsed.iter_mut().zip(chunks(digits)) {
            *p = chunk.expect("`decode_with` validates the digits");
        }
        return mul_add_chunks(out, 0, &parsed[..n]);
    }
    // value = hi * BASE^p.exp + lo. `powers[0].exp <= LEAF`, so `p` exists,
    // and the next power is at least `n`, so `hi` is no longer than `lo`.
    let i = powers.partition_point(|p| p.exp < n) - 1;
    let p = &powers[i];
    let (hi, lo) = digits.split_at(digits.len() - p.exp * CHUNK);
    // `t` holds `hi`, then `lo`. `BASE^p.exp < 2^(64 * p.exp)`, so `end <= n`.
    let (t, scratch) = scratch.split_at_mut(p.exp);
    let hn = dc(&mut t[..n - p.exp], hi, powers, scratch, m, leaf);
    let end = p.shift + p.limbs.len() + hn;
    out[..p.shift].fill(0);
    m.mul_power(&mut out[p.shift..end], &t[..hn], i, p.limbs, scratch);
    let ln = dc(t, lo, powers, scratch, m, leaf);
    // lo < BASE^p.exp, so it fits below `end` and the sum does not overflow.
    let carry = mul::add_assign(&mut out[..end], &t[..ln]);
    debug_assert!(!carry);
    trimmed_len(&out[..end])
}

/// Length of `limbs` without its high zero limbs.
pub(crate) fn trimmed_len(limbs: &[u64]) -> usize {
    limbs.iter().rposition(|&l| l != 0).map_or(0, |i| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{Decode, decode_vec, random_string};
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::SeedableRng;

    /// [`decode`] with each leaf.
    const LEAVES: [(&str, Decode); 2] = [
        ("horner", |s, out| decode_with_leaf(s, out, &mut Karatsuba, Leaf::Horner)),
        ("matrix", |s, out| decode_with_leaf(s, out, &mut Karatsuba, Leaf::Matrix)),
    ];

    /// Large inputs, where bs58 is too slow to serve as the reference.
    #[test]
    fn matches_batched() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(3);
        let t = DC_THRESHOLD * CHUNK;
        // 163_850 digits are 16385 chunks, whose first leaf has 2 chunks.
        for len in [t + 1, 2 * t + 1, 10_000, 40_960, 100_001, 163_850] {
            // 58^len - 1 maximises carries.
            let max = "z".repeat(len);
            let min = format!("2{}", "1".repeat(len - 1));
            let zeros = format!("111{max}");
            for s in [random_string(&mut rng, len), max, min, zeros] {
                let want = decode_vec(decode_batched, &s);
                for (name, f) in LEAVES {
                    assert_eq!(decode_vec(f, &s), want, "{name}, len {len}");
                }
            }
        }
    }

    /// A [`Multiplier`] that checks what [`decode_with`] promises: `prepare` comes once
    /// and first, its `len` bounds every product, `mul` gets the scratch that
    /// `scratch_len` asks for, powers are prepared in order, and `mul_power` gets the
    /// power prepared under its index, in products no longer than the `len` given then.
    struct Checked {
        len: Option<usize>,
        calls: usize,
        /// Products with a prepared power.
        power_calls: usize,
        /// Address, length and product bound of each prepared power.
        powers: Vec<(usize, usize, usize)>,
    }

    impl Checked {
        fn new() -> Self {
            Checked { len: None, calls: 0, power_calls: 0, powers: Vec::new() }
        }
    }

    impl Multiplier for Checked {
        fn prepare(&mut self, len: usize) {
            assert_eq!(self.len.replace(len), None, "prepare called twice");
        }

        // More than Karatsuba needs, so a slice cut too short would show.
        fn scratch_len(&self, n: usize) -> usize {
            16 * n + 64
        }

        fn mul(&mut self, out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]) {
            let len = self.len.expect("mul before prepare");
            assert!(a.len() + b.len() <= len, "product longer than len = {len}");
            assert!(scratch.len() >= self.scratch_len(a.len().max(b.len())), "scratch");
            self.calls += 1;
            mul::mul(out, a, b, scratch);
        }

        fn prepare_power(&mut self, i: usize, b: &[u64], len: usize) {
            assert!(self.len.is_some(), "prepare_power before prepare");
            assert_eq!(i, self.powers.len(), "powers prepared out of order");
            assert!(b.len() <= len, "power {i} longer than len = {len}");
            self.powers.push((b.as_ptr().addr(), b.len(), len));
        }

        fn mul_power(&mut self, out: &mut [u64], a: &[u64], i: usize, b: &[u64], s: &mut [u64]) {
            if let Some(&(addr, n, len)) = self.powers.get(i) {
                let power = (b.as_ptr().addr(), b.len());
                assert_eq!((addr, n), power, "power {i} is not the one prepared");
                assert!(a.len() + b.len() <= len, "product with power {i} longer than {len}");
                self.power_calls += 1;
            }
            self.mul(out, a, b, s);
        }
    }

    #[test]
    fn multiplier_contract() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(5);
        for len in [DC_THRESHOLD * CHUNK + 1, 10_000, 40_960] {
            let s = random_string(&mut rng, len);
            let mut m = Checked::new();
            let mut out = vec![0; len];
            let n = decode_with(&s, &mut out, &mut m).unwrap();
            assert_eq!(Ok(out[..n].to_vec()), decode_vec(decode, &s), "len {len}");
            assert!(m.calls > 0, "len {len}: no multiplications");
            assert!(m.power_calls > 0, "len {len}: no products with a prepared power");
        }
    }

    /// Short buffers and invalid characters fail before `prepare` or any product.
    #[test]
    fn fails_early() {
        let s = "z".repeat(1 << 20);
        let invalid = format!("{s}0");
        let mut m = Checked::new();
        assert_eq!(decode_with(&s, &mut [0; 1], &mut m), Err(Error::BufferTooSmall));
        let at = s.len();
        assert_eq!(
            decode_with(&invalid, &mut vec![0; at], &mut m),
            Err(Error::InvalidCharacter { index: at })
        );
        assert_eq!((m.len, m.calls, m.powers.len()), (None, 0, 0), "work before the checks");
    }

    /// The early length check never rejects a buffer the result fits in: the smallest
    /// value of `d` digits, `58^(d - 1)`, into a buffer of exactly its length.
    #[test]
    fn early_check_is_a_lower_bound() {
        for d in DC_THRESHOLD * CHUNK + 1..DC_THRESHOLD * CHUNK + 300 {
            let s = format!("2{}", "1".repeat(d - 1));
            let want = decode_vec(decode_batched, &s).unwrap();
            let mut out = vec![0; want.len()];
            assert_eq!(decode(&s, &mut out), Ok(want.len()), "{d} digits");
            assert_eq!(out, want);
        }
    }
}

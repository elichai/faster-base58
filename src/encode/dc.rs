//! Divide-and-conquer encoding: `x = hi * BASE^e + lo` with `lo < BASE^e`, from one
//! division by a power of the table, with balanced splits as in `crate::dc`. `lo` is
//! written as exactly `10 e` digits; `hi` takes the leading ones.
//!
//! Each division is Barrett's: a quotient estimate from one product with a reciprocal of
//! the power, the remainder from a second product, then at most a few corrections. Each
//! power's reciprocal comes from Newton's method once per call and serves every division
//! by it. All products go through a [`Multiplier`].

use std::cmp::Ordering;

use super::batched::Limbs;
use super::{bits, matrix, min_digits, split_zeros, ten_digits};
use crate::common::CHUNK;
use crate::dc::{
    Karatsuba, LEAF, Multiplier, Power, powers, split_unit_leaf, table_len, trimmed_len,
};
use crate::mul::{add_assign, fold_wrapped, sub_assign};
use crate::{BufferTooSmall, encode_batched};

/// Inputs of more than this many bytes use divide and conquer, with leaves of at most
/// [`DC_LEAF`] chunks, when multiplying with Karatsuba or Toom-3. Tuned on an M5 Pro
/// against [`encode_batched`], in one binary; the NTT and malachite have their own.
pub(crate) const DC_THRESHOLD: usize = 5120;
pub(crate) const DC_LEAF: usize = 96;

/// Divisors of at most this many limbs get their reciprocal by schoolbook division.
const RECIP_BASECASE: usize = 16;

/// The largest matrix leaf in chunks: at most `8 MATRIX_LEAF` bytes, `encode_matrix`'s
/// cap. Horner leaves keep `LEAF`, which sizes their stack buffers.
pub(super) const MATRIX_LEAF: usize = 1024;

/// Like [`crate::encode_batched`], but subquadratic for large inputs.
pub fn encode(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    encode_with(input, output, &mut Karatsuba)
}

/// [`encode`] with `threshold` and `leaf` in place of [`DC_THRESHOLD`] and [`DC_LEAF`],
/// and leaves converted by `leaves`.
pub fn encode_karatsuba_with(
    input: &[u8],
    output: &mut [u8],
    threshold: usize,
    leaf: usize,
    leaves: Leaves,
) -> Result<usize, BufferTooSmall> {
    encode_tuned(input, output, &mut Karatsuba, threshold, leaf, leaves)
}

/// [`encode`], multiplying with `m`.
pub(crate) fn encode_with(
    input: &[u8],
    output: &mut [u8],
    m: &mut impl Multiplier,
) -> Result<usize, BufferTooSmall> {
    encode_tuned(input, output, m, DC_THRESHOLD, DC_LEAF, Leaves::Horner)
}

/// How the divide-and-conquer encoders convert their leaves.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Leaves {
    /// [`encode_batched`]'s kernel.
    Horner,
    /// `encode_matrix`'s conversion. Faster with its table in cache, but the table is
    /// built on first use and kept for the life of the process, so only encoders that
    /// cache anyway use it.
    Matrix,
}

impl Leaves {
    /// [`Leaves::Matrix`] where `encode_matrix` has its NEON kernels, else
    /// [`Leaves::Horner`]. Provisional on x86_64, where it is not measured yet.
    pub(crate) const BEST: Leaves =
        if cfg!(all(target_arch = "aarch64", target_endian = "little", target_feature = "neon")) {
            Leaves::Matrix
        } else {
            Leaves::Horner
        };
}

/// [`encode_with`] with divide and conquer above `threshold` bytes, leaves of at most
/// `leaf <= LEAF` chunks, converted by `leaves`.
pub(crate) fn encode_tuned(
    input: &[u8],
    output: &mut [u8],
    m: &mut impl Multiplier,
    threshold: usize,
    leaf: usize,
    leaves: Leaves,
) -> Result<usize, BufferTooSmall> {
    let max = if leaves == Leaves::Matrix { MATRIX_LEAF } else { LEAF };
    assert!((1..=max).contains(&leaf), "{leaves:?} leaf of {leaf} chunks");
    let (zeros, x) = split_zeros(input);
    if x.len() <= threshold {
        return encode_batched(input, output);
    }
    let min = min_digits(bits(x));
    let (ones, out) = output.split_at_mut_checked(zeros).ok_or(BufferTooSmall)?;
    if out.len() < min {
        return Err(BufferTooSmall);
    }
    // `x < 58^(min + 2) <= BASE^n`.
    let n = (min + 2).div_ceil(CHUNK);
    let (unit, levels) = split_unit_leaf(n, leaf);
    if levels == 0 {
        return encode_batched(input, output);
    }
    let e = unit << (levels - 1);
    // No divisor, quotient or remainder is longer than BASE^e, at most `full(e)` limbs, and
    // no product longer than twice that plus one.
    m.prepare(2 * full(e) + 2);

    let x: Vec<u64> = x.rchunks(8).map(|c| c.iter().fold(0, |v, &b| v << 8 | b as u64)).collect();
    let mut table = vec![0; table_len(unit, levels)];
    let mut scratch = vec![0; m.scratch_len(e) + recip_scratch_len(full(e) + 1, m)];
    let powers = powers(unit, levels, &mut table, &mut scratch, m, false);
    let divs: Vec<Divisor> = powers[..levels].iter().map(|p| Divisor::new(p, &mut scratch, m)).collect();
    // The top divisor serves one division; the others at least two, so their products
    // may keep work on them.
    for (i, d) in divs[..levels - 1].iter().enumerate() {
        let (dn, qn) = (d.pn.len(), d.qn());
        m.prepare_power(2 * i, &d.pn, qn + dn);
        m.prepare_power(2 * i + 1, &d.inv, 2 * qn + 1);
    }
    let mut scratch = vec![0; node_scratch_len(&divs, m)];
    let mut out = Out { buf: out, pos: 0 };
    node(&x, n, false, &Tree { divs: &divs, leaf, leaves }, &mut out, &mut scratch, m)?;
    ones.fill(b'1');
    Ok(zeros + out.pos)
}

/// At least the limbs of `BASE^e`: `BASE < 2^59`.
fn full(e: usize) -> usize {
    (59 * e).div_ceil(64)
}

/// The power `BASE^exp`, ready for division: `pn << (64 shift)` is the power shifted left
/// by `bits`, so that the top bit of `pn` is set, and `inv` is a reciprocal for quotients
/// of up to `qn = pn.len() + shift` limbs.
struct Divisor {
    exp: usize,
    shift: usize,
    bits: u32,
    pn: Vec<u64>,
    /// `floor(β^(2 qn) / (pn β^shift))`, or up to two less, in `qn + 1` limbs.
    inv: Vec<u64>,
}

impl Divisor {
    fn new(p: &Power<'_>, scratch: &mut [u64], m: &mut impl Multiplier) -> Divisor {
        let bits = p.limbs.last().expect("powers are not zero").leading_zeros();
        let mut pn = vec![0; p.limbs.len()];
        shl(&mut pn, p.limbs, bits);
        let qn = pn.len() + p.shift;
        let mut padded = vec![0; qn];
        padded[p.shift..].copy_from_slice(&pn);
        let mut inv = vec![0; qn + 1];
        reciprocal(&mut inv, &padded, scratch, m);
        Divisor { exp: p.exp, shift: p.shift, bits, pn, inv }
    }

    fn qn(&self) -> usize {
        self.pn.len() + self.shift
    }
}

/// Splits `x < BASE^(2 exp)` by the power `d` (number `i` of the table): writes
/// `x / BASE^exp` to `hi` (`qn + 1` limbs) and `x % BASE^exp` to `lo` (`qn` limbs), and
/// returns their trimmed lengths. Needs [`divide_scratch_len`] limbs of `s`.
///
/// With `P = (pn >> bits) β^shift` the power and `A = (x >> 64 shift) << bits`, the
/// quotient is `floor(A / pn)`: the low `shift` limbs of `x` only reach the remainder.
/// `x < P^2` gives `A < β^(dn + qn)` and a quotient below `β^qn`. Barrett with `inv` gives
/// an estimate at most 6 below it.
fn divide(
    x: &[u64],
    d: &Divisor,
    i: usize,
    hi: &mut [u64],
    lo: &mut [u64],
    s: &mut [u64],
    m: &mut impl Multiplier,
) -> (usize, usize) {
    let (dn, sh, qn) = (d.pn.len(), d.shift, d.qn());
    let (a, s) = s.split_at_mut(dn + qn);
    shl(a, x.get(sh..).unwrap_or(&[]), d.bits);
    hi.fill(0);
    let a1 = &a[dn..];
    let a1 = &a1[..trimmed_len(a1)];
    if !a1.is_empty() {
        // q ≈ floor(a1 * inv / β^qn).
        let (p, s) = s.split_at_mut(a1.len() + qn + 1);
        m.mul_power(p, a1, 2 * i + 1, &d.inv, s);
        let q = &p[qn..];
        hi[..q.len()].copy_from_slice(q);
        let ql = trimmed_len(hi);
        // The remainder `A - q pn` is below `7 pn < β^(dn + 1)`, so it is also its own
        // residue modulo `β^L - 1` for `L >= dn + 2`. Same arguments as in
        // `divide_scratch_len`.
        match m.wrap_len(dn + 2, qn + dn) {
            Some(l) if ql > 0 => {
                let (w, s) = s.split_at_mut(l);
                let (r, s) = s.split_at_mut(l);
                m.mul_power_wrapped(w, &hi[..ql], 2 * i, &d.pn, s);
                fold_wrapped(r, a);
                sub_wrapped(r, w);
                canonical(r);
                let (keep, rest) = r.split_at(l.min(a.len()));
                debug_assert!(rest.iter().all(|&v| v == 0), "the remainder is too large");
                a.fill(0);
                a[..keep.len()].copy_from_slice(keep);
            }
            None if ql > 0 => {
                let (qp, s) = s.split_at_mut(ql + dn);
                m.mul_power(qp, &hi[..ql], 2 * i, &d.pn, s);
                let borrow = sub_assign(a, qp);
                debug_assert!(!borrow, "the quotient estimate is too large");
            }
            _ => {}
        }
    }
    let mut fixes = 0;
    while ge(a, &d.pn) {
        fixes += 1;
        debug_assert!(fixes <= 6, "more than 6 corrections");
        sub_assign(a, &d.pn);
        add_assign(hi, &[1]);
    }
    // lo = (A >> bits) β^shift + x mod β^shift. The remainder is below `pn`.
    let low = &x[..sh.min(x.len())];
    lo[..low.len()].copy_from_slice(low);
    lo[low.len()..sh].fill(0);
    shr(&mut lo[sh..], &a[..dn], d.bits);
    (trimmed_len(hi), trimmed_len(lo))
}

/// Scratch limbs [`divide`] needs.
fn divide_scratch_len(d: &Divisor, m: &impl Multiplier) -> usize {
    let (dn, qn) = (d.pn.len(), d.qn());
    let full = (qn + dn) + m.scratch_len(qn + 1);
    let wrapped = m.wrap_len(dn + 2, qn + dn).map_or(0, |l| 2 * l + m.wrapped_scratch_len(l, qn));
    (dn + qn) + (2 * qn + 1) + full.max(wrapped)
}

/// `a = (a - b) mod (β^L - 1)`, for `a` and `b` of `L` limbs in `[0, β^L - 1]`.
fn sub_wrapped(a: &mut [u64], b: &[u64]) {
    // On a borrow the difference wrapped by `β^L`, one more than the modulus; it is at
    // least one, so taking that one back does not borrow again.
    if sub_assign(a, b) {
        let borrow = sub_assign(a, &[1]);
        debug_assert!(!borrow);
    }
}

/// Replaces `β^L - 1`, the other representative of zero modulo `β^L - 1`, by zero.
fn canonical(a: &mut [u64]) {
    if a.iter().all(|&v| v == u64::MAX) {
        a.fill(0);
    }
}

/// Where the digits go: `buf[pos..]`.
struct Out<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl Out<'_> {
    fn take(&mut self, n: usize) -> Result<&mut [u8], BufferTooSmall> {
        let s = self.buf.get_mut(self.pos..self.pos + n).ok_or(BufferTooSmall)?;
        self.pos += n;
        Ok(s)
    }
}

/// What every [`node`] of one encode shares: the divisors by level, and the largest leaf.
struct Tree<'a> {
    divs: &'a [Divisor],
    leaf: usize,
    leaves: Leaves,
}

/// Writes the digits of `x < BASE^n`: exactly `10 n` of them if `pad`, else without
/// leading zeros. Needs [`node_scratch_len`] limbs of `s`.
fn node(
    x: &[u64],
    n: usize,
    pad: bool,
    t: &Tree<'_>,
    out: &mut Out<'_>,
    s: &mut [u64],
    m: &mut impl Multiplier,
) -> Result<(), BufferTooSmall> {
    if n <= t.leaf {
        return match t.leaves {
            Leaves::Horner => leaf(x, n, pad, out),
            Leaves::Matrix => leaf_matrix(x, n, pad, out),
        };
    }
    if x.is_empty() {
        if pad {
            out.take(10 * n)?.fill(b'1');
        }
        return Ok(());
    }
    // `divs[0].exp <= leaf < n`, so `i` exists, and the next power is at least `n`, so
    // `x < BASE^(2 exp)`.
    let i = t.divs.partition_point(|d| d.exp < n) - 1;
    let d = &t.divs[i];
    let qn = d.qn();
    let (hi, s) = s.split_at_mut(qn + 1);
    let (lo, s) = s.split_at_mut(qn);
    let (hn, ln) = divide(x, d, i, hi, lo, s, m);
    if !pad && hn == 0 {
        return node(&lo[..ln], d.exp, false, t, out, s, m);
    }
    node(&hi[..hn], n - d.exp, pad, t, out, s, m)?;
    node(&lo[..ln], d.exp, true, t, out, s, m)
}

/// Scratch limbs [`node`] needs for any node, by induction on the levels: a node at level
/// `i` holds `hi` and `lo` while it divides or while its children, at lower levels, run.
fn node_scratch_len(divs: &[Divisor], m: &impl Multiplier) -> usize {
    divs.iter().fold(0, |below, d| 2 * d.qn() + 1 + divide_scratch_len(d, m).max(below))
}

/// The big-endian bytes of `x < BASE^n` without leading zeros, in `buf`, which needs
/// `8 n` bytes: `x < 2^(64 n)`, so it has at most `n` limbs.
fn be_bytes<'a>(x: &[u64], buf: &'a mut [u8]) -> &'a [u8] {
    let bytes = &mut buf[..8 * x.len()];
    for (b, l) in bytes.rchunks_exact_mut(8).zip(x) {
        b.copy_from_slice(&l.to_be_bytes());
    }
    split_zeros(bytes).1
}

/// [`node`] for `n <= MATRIX_LEAF` by `encode_matrix`'s conversion. Not inlined: its
/// frame holds the matrix's scratch and the bytes, about 38 KiB.
#[inline(never)]
fn leaf_matrix(x: &[u64], n: usize, pad: bool, out: &mut Out<'_>) -> Result<(), BufferTooSmall> {
    let mut bytes = [0u8; 8 * MATRIX_LEAF];
    let mut scratch = matrix::Scratch::new();
    let cols = matrix::columns(be_bytes(x, &mut bytes), &mut scratch);
    let Some((&top, rest)) = cols.split_last() else {
        if pad {
            out.take(10 * n)?.fill(b'1');
        }
        return Ok(());
    };
    let top = matrix::four_digits(top);
    // Characters the top column gives: without its leading zero digits, or with `pad` as
    // many as make `10 n` in all. `x < 58^(10 n)`, so the other columns hold fewer than
    // `10 n` characters, and any top digits past `10 n` are zero.
    let lead = match pad {
        true => 10 * n - 4 * rest.len(),
        false => 4 - top.iter().take_while(|&&c| c == b'1').count(),
    };
    let (head, body) = out.take(lead + 4 * rest.len())?.split_at_mut(lead);
    if lead <= 4 {
        debug_assert!(top[..4 - lead].iter().all(|&c| c == b'1'), "x >= BASE^n");
        head.copy_from_slice(&top[4 - lead..]);
    } else {
        head[..lead - 4].fill(b'1');
        head[lead - 4..].copy_from_slice(&top);
    }
    matrix::column_digits(rest, body.as_chunks_mut::<4>().0);
    Ok(())
}

/// [`node`] for `n <= LEAF`, with [`Limbs`] on the stack.
fn leaf(x: &[u64], n: usize, pad: bool, out: &mut Out<'_>) -> Result<(), BufferTooSmall> {
    let mut bytes = [0u8; 8 * LEAF];
    let mut slots = [[0u8; 8]; LEAF];
    let mut limbs = Limbs { slots: &mut slots, len: 0, top: 0 };
    limbs.feed(be_bytes(x, &mut bytes));
    let Limbs { slots, len, top } = limbs;
    let stored = &slots[slots.len() - len..];
    let top_digits = ten_digits(top);
    let o = if pad {
        let o = out.take(10 * n)?;
        let used = if top == 0 { 0 } else { len + 1 };
        let (ones, o) = o.split_at_mut(10 * (n - used));
        ones.fill(b'1');
        if top == 0 {
            return Ok(());
        }
        o[..10].copy_from_slice(&top_digits);
        &mut o[10..]
    } else {
        if top == 0 {
            return Ok(());
        }
        let skip = top_digits.iter().take_while(|&&c| c == b'1').count();
        let o = out.take(10 - skip + 10 * len)?;
        o[..10 - skip].copy_from_slice(&top_digits[skip..]);
        &mut o[10 - skip..]
    };
    for (o, s) in o.chunks_exact_mut(10).zip(stored) {
        o.copy_from_slice(&ten_digits(u64::from_ne_bytes(*s)));
    }
    Ok(())
}

/// Writes to `x` (`n + 1` limbs) a value with `a x < β^(2n) <= a (x + 2)`, for `a` of `n`
/// limbs with its top bit set. Needs [`recip_scratch_len`]`(n)` limbs of `s`.
///
/// Brent and Zimmermann, Modern Computer Arithmetic, Algorithm 3.5 (ApproximateReciprocal):
/// the reciprocal `x_h` of the top `h` limbs, then one Newton step from it.
fn reciprocal(x: &mut [u64], a: &[u64], s: &mut [u64], m: &mut impl Multiplier) {
    let n = a.len();
    debug_assert!(x.len() == n + 1 && a[n - 1] >> 63 == 1);
    if n == 1 {
        let v = u128::MAX / a[0] as u128;
        (x[0], x[1]) = (v as u64, (v >> 64) as u64);
        return;
    }
    if n <= RECIP_BASECASE {
        // floor((β^(2n) - 1) / a), exactly.
        let u = &mut s[..2 * n + 1];
        u[..2 * n].fill(u64::MAX);
        u[2 * n] = 0;
        div_schoolbook(x, u, a);
        return;
    }
    let l = (n - 1) / 2;
    let h = n - l;
    let (xh, s) = s.split_at_mut(h + 1);
    reciprocal(xh, &a[l..], s, m);
    // t = a x_h. Padded divisors end in zero limbs; skip them.
    // t = a x_h, in (β^(n+h) - 2 β^n, β^(n+h) + 2 β^n). Padded divisors end in zero
    // limbs; skip them.
    let (t, s) = s.split_at_mut(n + h + 1);
    let z = a.iter().take_while(|&&v| v == 0).count();
    t[..z].fill(0);
    m.mul(&mut t[z..], &a[z..], xh, s);
    // Each step takes `a >= β^n / 2` off: at most four.
    let mut steps = 0;
    while t[n + h] != 0 {
        steps += 1;
        debug_assert!(steps <= 4, "a x_h was not below β^(n+h) + 2 β^n");
        sub_assign(xh, &[1]);
        sub_assign(t, a);
    }
    // t = β^(n+h) - a x_h, in (0, 2 β^n], so `tm = t / β^l` has at most h + 1 limbs.
    negate(&mut t[..n + h]);
    let tm = &t[l..n + h];
    let tm = &tm[..trimmed_len(tm)];
    debug_assert!(tm.len() <= h + 1);
    // x = x_h β^l + floor(tm x_h / β^(2h - l)).
    x.fill(0);
    x[l..].copy_from_slice(xh);
    if !tm.is_empty() {
        let (u, s) = s.split_at_mut(tm.len() + h + 1);
        m.mul(u, tm, xh, s);
        let carry = add_assign(x, u.get(2 * h - l..).unwrap_or(&[]));
        debug_assert!(!carry);
    }
}

/// Scratch limbs [`reciprocal`] needs for `n` limbs.
fn recip_scratch_len(n: usize, m: &impl Multiplier) -> usize {
    if n <= RECIP_BASECASE {
        return 2 * n + 1;
    }
    let h = n - (n - 1) / 2;
    let step = (n + h + 1) + (2 * h + 2) + m.scratch_len(n);
    (h + 1) + recip_scratch_len(h, m).max(step)
}

/// Knuth's Algorithm D: `q = u / v` and `u = u % v`, for `v` of `n >= 2` limbs with its
/// top bit set, `u` of `k + n + 1` limbs whose top limb is zero, and `q` of `k + 1` limbs.
fn div_schoolbook(q: &mut [u64], u: &mut [u64], v: &[u64]) {
    let n = v.len();
    let (v1, v0) = (v[n - 1] as u128, v[n - 2] as u128);
    for j in (0..u.len() - n).rev() {
        let top = (u[j + n] as u128) << 64 | u[j + n - 1] as u128;
        let (mut qhat, mut rhat) = (top / v1, top % v1);
        // At most two steps make `qhat` exact or one too large.
        while qhat >> 64 != 0 || qhat * v0 > (rhat << 64 | u[j + n - 2] as u128) {
            qhat -= 1;
            rhat += v1;
            if rhat >> 64 != 0 {
                break;
            }
        }
        // u[j..=j+n] -= qhat v.
        let (mut carry, mut borrow) = (0u64, false);
        for i in 0..n {
            let (lo, hi) = (qhat as u64).carrying_mul(v[i], carry);
            (u[j + i], borrow) = u[j + i].borrowing_sub(lo, borrow);
            carry = hi;
        }
        let (top, b1) = u[j + n].overflowing_sub(carry);
        let (top, b2) = top.overflowing_sub(borrow as u64);
        u[j + n] = top;
        if b1 | b2 {
            // One too large: add `v` back.
            qhat -= 1;
            let mut c = false;
            for i in 0..n {
                (u[j + i], c) = u[j + i].carrying_add(v[i], c);
            }
            u[j + n] = u[j + n].wrapping_add(c as u64);
        }
        q[j] = qhat as u64;
    }
}

/// `dst = src << bits` for `bits < 64`, zero-extended. The bits shifted out of `dst` must
/// be zero.
fn shl(dst: &mut [u64], src: &[u64], bits: u32) {
    dst.fill(0);
    if bits == 0 {
        dst[..src.len()].copy_from_slice(src);
        return;
    }
    let mut carry = 0;
    for (d, &v) in dst.iter_mut().zip(src) {
        *d = v << bits | carry;
        carry = v >> (64 - bits);
    }
    match dst.get_mut(src.len()) {
        Some(d) => *d = carry,
        None => debug_assert_eq!(carry, 0, "shifted out set bits"),
    }
}

/// `dst = src >> bits` for `bits < 64`, over `dst.len() == src.len()` limbs.
fn shr(dst: &mut [u64], src: &[u64], bits: u32) {
    for (j, d) in dst.iter_mut().enumerate() {
        let up = match (bits, src.get(j + 1)) {
            (0, _) | (_, None) => 0,
            (_, Some(&v)) => v << (64 - bits),
        };
        *d = src[j] >> bits | up;
    }
}

/// `a >= b` for `a.len() >= b.len()`.
fn ge(a: &[u64], b: &[u64]) -> bool {
    let (lo, hi) = a.split_at(b.len());
    hi.iter().any(|&v| v != 0) || lo.iter().rev().cmp(b.iter().rev()) != Ordering::Less
}

/// `t = β^len - t` for `t != 0`.
fn negate(t: &mut [u64]) {
    let mut carry = true;
    for v in t {
        (*v, carry) = (!*v).overflowing_add(carry as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mul;
    use crate::ntt::{Kernels, Ntt};
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    fn product(a: &[u64], b: &[u64]) -> Vec<u64> {
        let mut out = vec![0; a.len() + b.len()];
        mul::mul(&mut out, a, b, &mut vec![0; mul::scratch_len(a.len().max(b.len()))]);
        out
    }

    /// Karatsuba, with every product that may wrap wrapped modulo `β^(n + extra) - 1` by
    /// the default `mul_wrapped` (the full product, folded): lengths that are not powers
    /// of two, down to the least the callers allow.
    struct Wrap(usize);

    impl Multiplier for Wrap {
        fn scratch_len(&self, n: usize) -> usize {
            mul::scratch_len(n)
        }

        fn mul(&mut self, out: &mut [u64], a: &[u64], b: &[u64], scratch: &mut [u64]) {
            mul::mul(out, a, b, scratch);
        }

        fn wrap_len(&self, n: usize, _full: usize) -> Option<usize> {
            Some(n + self.0)
        }
    }

    /// The NTT with tables for products of up to `len` limbs. It wraps the products
    /// where a power-of-two cyclic transform is shorter.
    fn ntt(len: usize) -> Ntt {
        let mut m = Ntt::new(false, Kernels::best());
        m.prepare(len);
        m
    }

    /// `a x < β^(2n) <= a (x + 2)` for random and extreme divisors, around the base case,
    /// with Karatsuba and the NTT.
    #[test]
    fn reciprocal_bounds() {
        let mut lens: Vec<usize> = (1..=70).collect();
        lens.extend([127, 128, 129, 255, 256, 500, 1000, 2049]);
        check_reciprocals(&lens, &mut Karatsuba);
        check_reciprocals(&lens, &mut ntt(8192));
    }

    fn check_reciprocals(lens: &[usize], m: &mut impl Multiplier) {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(30);
        for &n in lens {
            let mut random: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
            random[n - 1] |= 1 << 63;
            let mut min = vec![0; n];
            min[n - 1] = 1 << 63;
            let mut padded = random.clone();
            padded[..n / 3].fill(0);
            for a in [random, min, vec![u64::MAX; n], padded] {
                let mut x = vec![0xA5; n + 1];
                let mut s = vec![0x5A; recip_scratch_len(n, m)];
                reciprocal(&mut x, &a, &mut s, m);
                let ax = product(&a, &x);
                // β^(2n) is a one at limb 2n.
                assert!(ax[2 * n..].iter().all(|&v| v == 0), "{n}: a x >= β^(2n)");
                let mut x2 = x.clone();
                x2.push(0);
                add_assign(&mut x2, &[2]);
                let ax2 = product(&a, &x2);
                assert!(ax2[2 * n..].iter().any(|&v| v != 0), "{n}: a (x + 2) < β^(2n)");
            }
        }
    }

    /// `x = hi P + lo` with `lo < P`, for every level of a real table and random,
    /// all-ones and short `x`, with plain and wrapped products.
    #[test]
    fn divide_splits() {
        check_divisions(&mut Karatsuba);
        check_divisions(&mut Wrap(0));
        check_divisions(&mut Wrap(3));
        check_divisions(&mut ntt(1 << 15));
    }

    fn check_divisions(m: &mut impl Multiplier) {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(31);
        let (unit, levels) = split_unit_leaf(20_000, LEAF);
        let mut table = vec![0; table_len(unit, levels)];
        let e = unit << (levels - 1);
        let mut s = vec![0; m.scratch_len(e) + recip_scratch_len(full(e) + 1, m)];
        let powers = powers(unit, levels, &mut table, &mut s, m, false);
        for (i, p) in powers[..levels].iter().enumerate() {
            let d = Divisor::new(p, &mut s, m);
            let mut power = vec![0; p.shift];
            power.extend_from_slice(p.limbs);
            // Below `β^(2 len - 2) <= P^2`.
            let len = 2 * power.len() - 2;
            let random: Vec<u64> = (0..len).map(|_| rng.next_u64()).collect();
            let mut max = product(&power, &power);
            sub_assign(&mut max, &[1]);
            let max = max[..trimmed_len(&max)].to_vec();
            for x in [random, max, vec![7], vec![], p.limbs.to_vec(), power.clone()] {
                let (mut hi, mut lo) = (vec![0; d.qn() + 1], vec![0; d.qn()]);
                let mut ds = vec![0; divide_scratch_len(&d, m)];
                let (hn, ln) = divide(&x, &d, i, &mut hi, &mut lo, &mut ds, m);
                assert!(ln < power.len() || !ge(&lo[..ln], &power), "level {i}: lo >= P");
                let mut back = product(&hi[..hn], &power);
                back.resize(back.len().max(ln) + 1, 0);
                add_assign(&mut back, &lo[..ln]);
                let bl = trimmed_len(&back);
                assert!(back[..bl] == x[..trimmed_len(&x)], "level {i}: hi P + lo != x");
            }
        }
    }

    /// Whole encodes with every product that may wrap wrapped, against Karatsuba.
    #[test]
    fn wrapped_encodes() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(33);
        for len in [6000, 40_000, 150_001] {
            let mut data = vec![0u8; len];
            rng.fill_bytes(&mut data);
            for data in [data, vec![0xFF; len]] {
                let mut want = vec![0; crate::max_encoded_len(len)];
                let n = encode(&data, &mut want).unwrap();
                for extra in [0, 1, 9] {
                    let mut got = vec![0; want.len()];
                    let m = &mut Wrap(extra);
                    assert_eq!(encode_tuned(&data, &mut got, m, 0, 32, Leaves::Horner), Ok(n), "{len}, {extra}");
                    assert!(got[..n] == want[..n], "{len} bytes, L = n + {extra}");
                }
            }
        }
    }

    /// Whole encodes with Horner leaves of 1 to `LEAF` chunks and matrix leaves of 1 to
    /// `MATRIX_LEAF` against `encode_batched`: padded leaves whose top column holds fewer
    /// than four of their digits, or all of them with '1's before, unpadded top leaves,
    /// and zero leaves. Both buffers start as 0xAA, so this also checks that nothing is
    /// written past the result.
    #[test]
    fn leaf_kinds_and_sizes() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(34);
        for len in [5000, 30_001, 120_000] {
            let mut random = vec![0u8; len];
            rng.fill_bytes(&mut random);
            let mut runs = random.clone();
            runs[len / 5..len / 2].fill(0);
            for data in [random, vec![0xFF; len], runs, [vec![0; 2], vec![0xFF; len]].concat()] {
                let mut want = vec![0xAA; crate::max_encoded_len(data.len())];
                let n = encode_batched(&data, &mut want);
                let both = [1, 2, 3, 7, 32, 96, 255, LEAF].into_iter();
                let both = both.flat_map(|l| [(l, Leaves::Horner), (l, Leaves::Matrix)]);
                let matrix = [LEAF + 1, 512, MATRIX_LEAF].map(|l| (l, Leaves::Matrix));
                for (leaf, kind) in both.chain(matrix) {
                    let mut got = vec![0xAA; want.len()];
                    let r = encode_tuned(&data, &mut got, &mut Karatsuba, 0, leaf, kind);
                    assert_eq!(r, n, "{len} bytes, leaf {leaf}, {kind:?}");
                    assert!(got == want, "{len} bytes, leaf {leaf}, {kind:?}");
                }
            }
        }
    }
}

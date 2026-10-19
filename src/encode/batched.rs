//! Quadratic encoding with one base-58^10 limb update per seven input bytes.

use super::{bits, min_digits, split_zeros, ten_digits};
use crate::BufferTooSmall;
use crate::common::BASE;

/// Input bytes per limb update. `2^56 < BASE`, which [`Limbs::step`] relies on.
const STEP: usize = 7;
const _: () = assert!(1 << (8 * STEP) < BASE);

/// `floor(2^120 / BASE)`, for [`split`]. Below 2^62.
const RECIP: u64 = ((1u128 << 120) / BASE as u128) as u64;

/// Encodes `input` into `output` and returns the number of bytes written.
///
/// Works in place in `output`: no allocation. Writes nothing past the result.
pub fn encode_batched(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    let (zeros, x) = split_zeros(input);
    if x.is_empty() {
        output.get_mut(..zeros).ok_or(BufferTooSmall)?.fill(b'1');
        return Ok(zeros);
    }
    // The limbs are kept right-aligned below `anchor`, a lower bound on the digit count,
    // so they lie inside the result. See `write`.
    let anchor = min_digits(bits(x));
    let (ones, rest) = output.split_at_mut_checked(zeros).ok_or(BufferTooSmall)?;
    let region = rest.get_mut(..anchor).ok_or(BufferTooSmall)?;
    let mut limbs = Limbs { slots: region.as_rchunks_mut::<8>().1, len: 0, top: 0 };
    limbs.feed(x);
    let Limbs { len, top, .. } = limbs;
    let n = write(rest, anchor, len, top)?;
    ones.fill(b'1');
    Ok(zeros + n)
}

/// Writes the digits of the value with `len` limbs right-aligned below `anchor` in `buf`
/// and the top limb `top > 0`. Returns their count.
///
/// The text of a limb is written after the limb is read, top first. With `n <= anchor + 2`
/// digits, limb `i` (least significant first) sits at `anchor - 8 (i + 1)` and its text at
/// `n - 10 (i + 1)`, which can reach only limbs at or above `i`: already read.
fn write(buf: &mut [u8], anchor: usize, len: usize, top: u64) -> Result<usize, BufferTooSmall> {
    let top_digits = ten_digits(top);
    let skip = top_digits.iter().take_while(|&&c| c == b'1').count();
    let n = 10 - skip + 10 * len;
    assert!(n <= anchor + 2, "min_digits is too low");
    let out = buf.get_mut(..n).ok_or(BufferTooSmall)?;
    let mut pos = 10 - skip;
    out[..pos].copy_from_slice(&top_digits[skip..]);
    for i in (0..len).rev() {
        let at = anchor - 8 * (i + 1);
        let limb = u64::from_ne_bytes(out[at..at + 8].try_into().unwrap());
        out[pos..pos + 10].copy_from_slice(&ten_digits(limb));
        pos += 10;
    }
    Ok(n)
}

/// `(y * 2^56 / BASE, y * 2^56 % BASE)` for `y < BASE`. Depends only on `y`, so it is off
/// the carry chain of a pass.
#[inline(always)]
pub(super) fn split(y: u64) -> (u64, u64) {
    // `RECIP` is below `2^120 / BASE` by less than one, and `y < 2^64`, so this is the
    // quotient or one less.
    let q = ((y as u128 * RECIP as u128) >> 64) as u64;
    // The remainder for `q` is below `2 BASE < 2^64`, so wrapping arithmetic gives it.
    let r = (y << 56).wrapping_sub(q.wrapping_mul(BASE));
    let (d, borrow) = r.overflowing_sub(BASE);
    if borrow { (q, r) } else { (q + 1, d) }
}

/// `(t % BASE, t / BASE)` for `t < 2 BASE`.
#[inline(always)]
fn reduce(t: u64) -> (u64, u64) {
    let (d, borrow) = t.overflowing_sub(BASE);
    if borrow { (t, 0) } else { (d, 1) }
}

/// A value in base `BASE`: limb `i` (least significant first) in
/// `slots[slots.len() - 1 - i]` for `i < len`, and the top limb in `top`, non-zero unless
/// the value is zero. Holding the top limb back keeps the stored limbs inside the result.
pub(super) struct Limbs<'a> {
    pub slots: &'a mut [[u8; 8]],
    pub len: usize,
    pub top: u64,
}

impl Limbs<'_> {
    /// `value = value * 2^(8 x.len()) + x` for big-endian `x`.
    pub fn feed(&mut self, x: &[u8]) {
        let (head, body) = x.as_rchunks::<STEP>();
        let be = |c: &[u8]| c.iter().fold(0u64, |v, &b| v << 8 | b as u64);
        if !head.is_empty() {
            self.step(be(head));
        }
        let (pairs, last) = body.as_chunks::<2>();
        for [c1, c2] in pairs {
            self.step2(be(c1), be(c2));
        }
        if let [c] = last {
            self.step(be(c));
        }
    }

    /// `value = value * 2^56 + c` for `c < 2^56`.
    ///
    /// Each limb gives `t = y * 2^56 + carry`, new limb `t % BASE` and carry `t / BASE`.
    /// The carry is below `2^56` (as `t < BASE * 2^56`), so with `(q, r)` from [`split`],
    /// `r + carry < 2 BASE`: one compare finishes the division.
    fn step(&mut self, c: u64) {
        let cap = self.slots.len();
        // The carry into the next limb is `q + k`.
        let (mut q, mut k) = (c, 0);
        for s in self.slots[cap - self.len..].iter_mut().rev() {
            let (qy, r) = split(u64::from_ne_bytes(*s));
            // `r + q` does not wait for the previous limb's compare.
            let (y, ky) = reduce(r + q + k);
            *s = y.to_ne_bytes();
            (q, k) = (qy, ky);
        }
        self.step_top(q + k);
    }

    /// `value = (value * 2^56 + c1) * 2^56 + c2`.
    ///
    /// The second pass runs one limb behind the first, so the CPU overlaps their carry
    /// chains.
    fn step2(&mut self, c1: u64, c2: u64) {
        let cap = self.slots.len();
        let l = &mut self.slots[cap - self.len..];
        let n = l.len();
        if n == 0 {
            self.step(c1);
            return self.step(c2);
        }
        // `p` is the previous limb after the first pass only.
        let (q, r) = split(u64::from_ne_bytes(l[n - 1]));
        let (mut p, mut k1) = reduce(r + c1);
        let (mut q1, mut q2, mut k2) = (q, c2, 0);
        for i in (0..n - 1).rev() {
            let (qa, ra) = split(u64::from_ne_bytes(l[i]));
            let (a, ka) = reduce(ra + q1 + k1);
            let (qb, rb) = split(p);
            let (b, kb) = reduce(rb + q2 + k2);
            l[i + 1] = b.to_ne_bytes();
            (p, q1, k1, q2, k2) = (a, qa, ka, qb, kb);
        }
        let (qb, rb) = split(p);
        let (b, kb) = reduce(rb + q2 + k2);
        l[0] = b.to_ne_bytes();
        let mut c2 = qb + kb;
        // The first pass on `top`. If it pushes a limb, the second pass handles it too.
        let (t, ct) = step_limb(self.top, q1 + k1);
        if ct == 0 {
            self.top = t;
        } else {
            let (b, cb) = step_limb(t, c2);
            self.push(b);
            (self.top, c2) = (ct, cb);
        }
        self.step_top(c2);
    }

    /// `top = top * 2^56 + c`, pushing the low limb if it overflows.
    fn step_top(&mut self, c: u64) {
        let (t, ct) = step_limb(self.top, c);
        self.top = t;
        if ct != 0 {
            self.push(t);
            self.top = ct;
        }
    }

    fn push(&mut self, limb: u64) {
        let i = self.slots.len() - 1 - self.len;
        self.slots[i] = limb.to_ne_bytes();
        self.len += 1;
    }
}

/// `(y * 2^56 + c) % BASE` and the carry `(y * 2^56 + c) / BASE < 2^56`.
#[inline(always)]
fn step_limb(y: u64, c: u64) -> (u64, u64) {
    let (q, r) = split(y);
    let (t, k) = reduce(r + c);
    (t, q + k)
}

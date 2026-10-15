//! Quadratic decoding with one 64-bit limb update per ten characters.

use crate::Error;
use crate::common::{BASE, chunks, invalid, split_zeros};

/// Decodes `input` into `output` and returns the number of bytes written.
///
/// Works in place in `output`: no allocation.
pub fn decode_batched(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    let (zeros, digits) = split_zeros(input.as_bytes());
    let (zero_bytes, buf) = output.split_at_mut_checked(zeros).ok_or(Error::BufferTooSmall)?;
    let mut chunks = chunks(digits).map(|c| c.ok_or_else(|| invalid(input)));
    // The first chunk is the starting value; the rest come in pairs.
    let top = chunks.next().transpose()?.unwrap_or(0);
    let mut value = Value { limbs: buf.as_chunks_mut::<8>().0, len: 0, top };
    while let Some(c1) = chunks.next() {
        match chunks.next() {
            Some(c2) => value.mul_add2(c1?, c2?)?,
            None => value.mul_add(c1?)?,
        }
    }
    let Value { len, top, .. } = value;
    // `top` is non-zero unless the value is zero, so `n` is the exact byte length.
    let n = 8 * len + 8 - top.leading_zeros() as usize / 8;
    let tail = buf.get_mut(8 * len..n).ok_or(Error::BufferTooSmall)?;
    tail.copy_from_slice(&top.to_le_bytes()[..tail.len()]);
    buf[..n].reverse();
    zero_bytes.fill(0);
    Ok(zeros + n)
}

/// A value kept little-endian in the output buffer: full limbs in `limbs[..len]`, the
/// top limb in `top`. Holding the top limb back lets an exactly sized output suffice.
struct Value<'a> {
    limbs: &'a mut [[u8; 8]],
    len: usize,
    top: u64,
}

impl Value<'_> {
    /// `value = value * BASE + c`.
    fn mul_add(&mut self, c: u64) -> Result<(), Error> {
        let mut carry = c;
        for l in &mut self.limbs[..self.len] {
            let (lo, hi) = u64::from_le_bytes(*l).carrying_mul(BASE, carry);
            *l = lo.to_le_bytes();
            carry = hi;
        }
        self.mul_add_top(carry)
    }

    /// `value = (value * BASE + c1) * BASE + c2`.
    ///
    /// The second pass runs one limb behind the first, so the CPU overlaps their carry
    /// chains: about twice the throughput of two [`Value::mul_add`] calls.
    fn mul_add2(&mut self, c1: u64, c2: u64) -> Result<(), Error> {
        let l = &mut self.limbs[..self.len];
        let Some(first) = l.first() else {
            self.mul_add(c1)?;
            return self.mul_add(c2);
        };
        // `p` is the previous limb after the first pass only.
        let (mut p, mut k1) = u64::from_le_bytes(*first).carrying_mul(BASE, c1);
        let mut k2 = c2;
        for i in 1..l.len() {
            let (a, h1) = u64::from_le_bytes(l[i]).carrying_mul(BASE, k1);
            let (b, h2) = p.carrying_mul(BASE, k2);
            l[i - 1] = b.to_le_bytes();
            (p, k1, k2) = (a, h1, h2);
        }
        let (b, h2) = p.carrying_mul(BASE, k2);
        l[l.len() - 1] = b.to_le_bytes();
        k2 = h2;
        // The first pass on `top`. If it pushes a limb, the second pass handles it too.
        let (t, h1) = self.top.carrying_mul(BASE, k1);
        if h1 == 0 {
            self.top = t;
        } else {
            let (b, h2) = t.carrying_mul(BASE, k2);
            self.push(b)?;
            (self.top, k2) = (h1, h2);
        }
        self.mul_add_top(k2)
    }

    /// `top = top * BASE + carry`, pushing the low limb if it overflows.
    fn mul_add_top(&mut self, carry: u64) -> Result<(), Error> {
        let (lo, hi) = self.top.carrying_mul(BASE, carry);
        self.top = lo;
        if hi != 0 {
            self.push(lo)?;
            self.top = hi;
        }
        Ok(())
    }

    fn push(&mut self, limb: u64) -> Result<(), Error> {
        *self.limbs.get_mut(self.len).ok_or(Error::BufferTooSmall)? = limb.to_le_bytes();
        self.len += 1;
        Ok(())
    }
}

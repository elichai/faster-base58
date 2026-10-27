//! Quadratic decoding with one 64-bit limb update per ten characters.

use crate::Error;
use crate::common::{BASE, DIGITS, chunks, invalid, split_zeros};

/// Decodes `input` into `output` and returns the number of bytes written.
///
/// Works in place in `output`: no allocation.
pub fn decode_batched(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    let (zeros, _) = split_zeros(input.as_bytes());
    decode_parts(input, zeros, output)
}

/// Batched conversion with the number of leading `'1'` bytes already known.
pub(crate) fn decode_parts(input: &str, zeros: usize, output: &mut [u8]) -> Result<usize, Error> {
    let digits = &input.as_bytes()[zeros..];
    debug_assert!(digits.first() != Some(&b'1'));
    let (zero_bytes, buf) = output
        .split_at_mut_checked(zeros)
        .ok_or(Error::BufferTooSmall)?;
    let mut chunks = chunks(digits).map(|c| c.ok_or_else(|| invalid(input)));
    // The first chunk is the starting value; the rest come in pairs.
    let top = chunks.next().transpose()?.unwrap_or(0);
    let mut value = Value {
        limbs: buf.as_chunks_mut::<8>().0,
        len: 0,
        top,
    };
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

pub(crate) fn decode_short(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    let bytes = input.as_bytes();
    let Some(&first) = bytes.first() else {
        return Ok(0);
    };
    let first = DIGITS[first as usize];
    let second = bytes.get(1).map(|&b| DIGITS[b as usize]);
    if first == 0xFF || second == Some(0xFF) {
        return Err(invalid(input));
    }
    let Some(second) = second else {
        *output.first_mut().ok_or(Error::BufferTooSmall)? = first;
        return Ok(1);
    };
    if first == 0 {
        let out = output.get_mut(..2).ok_or(Error::BufferTooSmall)?;
        out[0] = 0;
        out[1] = second;
        return Ok(2);
    }
    let value = first as u16 * 58 + second as u16;
    let len = 1 + usize::from(value >= 256);
    let out = output.get_mut(..len).ok_or(Error::BufferTooSmall)?;
    if len == 1 {
        out[0] = value as u8;
    } else {
        out[0] = (value >> 8) as u8;
        out[1] = value as u8;
    }
    Ok(len)
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    /// The interleaved passes must propagate the same carries and limb growth as two
    /// independent passes, including either pass creating a new top limb.
    #[test]
    fn paired_update_matches_sequential() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(71);
        for len in 0..=4 {
            for top in [1, BASE - 1, u64::MAX / BASE, u64::MAX] {
                for c1 in [0, 1, BASE - 1, rng.next_u64() % BASE] {
                    for c2 in [0, 1, BASE - 1, rng.next_u64() % BASE] {
                        let mut paired =
                            core::array::from_fn::<_, 6, _>(|_| rng.next_u64().to_le_bytes());
                        let mut sequential = paired;
                        let mut p = Value {
                            limbs: &mut paired,
                            len,
                            top,
                        };
                        let mut s = Value {
                            limbs: &mut sequential,
                            len,
                            top,
                        };
                        p.mul_add2(c1, c2).unwrap();
                        s.mul_add(c1).unwrap();
                        s.mul_add(c2).unwrap();
                        assert_eq!((p.len, p.top), (s.len, s.top));
                        assert_eq!(paired, sequential);
                    }
                }
            }
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    // Open obligation: SAT timed out; Z3 hit a CBMC SMT-lowering error (see review).
    #[kani::proof]
    #[kani::unwind(5)]
    fn paired_update_matches_sequential() {
        let low: u64 = kani::any();
        let top: u64 = kani::any();
        let c1: u64 = kani::any();
        let c2: u64 = kani::any();
        kani::assume(top != 0 && c1 < BASE && c2 < BASE);
        let mut paired = [low.to_le_bytes(), [0xA5; 8], [0xA5; 8]];
        let mut sequential = paired;
        let mut p = Value {
            limbs: &mut paired,
            len: 1,
            top,
        };
        let mut s = Value {
            limbs: &mut sequential,
            len: 1,
            top,
        };
        assert!(p.mul_add2(c1, c2).is_ok());
        assert!(s.mul_add(c1).is_ok());
        assert!(s.mul_add(c2).is_ok());
        assert_eq!((p.len, p.top), (s.len, s.top));
        for i in 0..3 {
            assert_eq!(
                u64::from_le_bytes(paired[i]),
                u64::from_le_bytes(sequential[i])
            );
        }
    }

    // Status/buffer-safety conjunct only; numeric equivalence remains separate.
    #[kani::proof]
    #[kani::unwind(5)]
    fn paired_update_all_return_success() {
        let low: u64 = kani::any();
        let top: u64 = kani::any();
        let c1: u64 = kani::any();
        let c2: u64 = kani::any();
        kani::assume(top != 0 && c1 < BASE && c2 < BASE);
        let mut paired = [low.to_le_bytes(), [0xA5; 8], [0xA5; 8]];
        let mut sequential = paired;
        let mut p = Value {
            limbs: &mut paired,
            len: 1,
            top,
        };
        let mut s = Value {
            limbs: &mut sequential,
            len: 1,
            top,
        };
        assert!(p.mul_add2(c1, c2).is_ok());
        assert!(s.mul_add(c1).is_ok());
        assert!(s.mul_add(c2).is_ok());
    }

    #[kani::proof]
    #[kani::unwind(3)]
    #[kani::solver(z3)]
    fn single_update_matches_wide_reference() {
        let low: u64 = kani::any();
        let top: u64 = kani::any();
        let c: u64 = kani::any();
        kani::assume(top != 0 && c < BASE);
        let first = low as u128 * BASE as u128 + c as u128;
        let last = top as u128 * BASE as u128 + (first >> 64);
        let mut limbs = [low.to_le_bytes(), [0xA5; 8]];
        let mut value = Value {
            limbs: &mut limbs,
            len: 1,
            top,
        };
        assert!(value.mul_add(c).is_ok());
        let carry = (last >> 64) as u64;
        assert_eq!(value.len, if carry == 0 { 1 } else { 2 });
        assert_eq!(value.top, if carry == 0 { last as u64 } else { carry });
        assert_eq!(u64::from_le_bytes(limbs[0]), first as u64);
        let second = if carry == 0 {
            0xA5A5_A5A5_A5A5_A5A5
        } else {
            last as u64
        };
        assert_eq!(u64::from_le_bytes(limbs[1]), second);
    }
}

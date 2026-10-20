//! [`decode_malachite`]: divide and conquer (`dc.rs`) with malachite-nz's multiplication,
//! which picks schoolbook, Toom-Cook or FFT by operand size.
//!
//! malachite's limb-slice functions are public only with its `test_build` feature, which
//! pulls in GMP, pyo3 and more. So products go through [`Natural`]: the operands are
//! copied into reused buffers, and malachite allocates each product (and the FFT its own
//! buffers).

use std::{mem, ptr};

use malachite_nz::natural::Natural;

use crate::dc::{self, Multiplier};
use crate::encode::Leaves;
use crate::{BufferTooSmall, Error, encode};

/// Like [`crate::decode`], but multiplies with malachite-nz.
pub fn decode_malachite(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    dc::decode_with(input, output, &mut Malachite::default())
}

/// The encoder's divide-and-conquer threshold in bytes and leaf size in chunks, tuned on an
/// M5 Pro against `encode_batched` in one binary.
pub(crate) const ENCODE_THRESHOLD: usize = 3072;
const ENCODE_LEAF: usize = 96;

/// Like [`crate::encode`], but multiplies with malachite-nz.
pub fn encode_malachite(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    let m = &mut Malachite::default();
    encode::encode_tuned(input, output, m, ENCODE_THRESHOLD, ENCODE_LEAF, Leaves::Horner)
}

/// [`encode_malachite`] with divide and conquer above `threshold` bytes and leaves of at
/// most `leaf` chunks, converted by `leaves`.
pub fn encode_malachite_with(
    input: &[u8],
    output: &mut [u8],
    threshold: usize,
    leaf: usize,
    leaves: Leaves,
) -> Result<usize, BufferTooSmall> {
    encode::encode_tuned(input, output, &mut Malachite::default(), threshold, leaf, leaves)
}

/// The [`Multiplier`] of [`decode_malachite`].
#[derive(Default)]
struct Malachite {
    /// Operand buffers, reused so that building a [`Natural`] does not allocate.
    bufs: [Vec<u64>; 2],
}

impl Multiplier for Malachite {
    // malachite allocates its own scratch. `n` is the least that `dc` needs for its own
    // temporaries (see `Multiplier::scratch_len`).
    fn scratch_len(&self, n: usize) -> usize {
        n
    }

    fn mul(&mut self, out: &mut [u64], a: &[u64], b: &[u64], _: &mut [u64]) {
        let x = natural(&mut self.bufs[0], a);
        // malachite squares when both operands are the same `Natural`. `dc` squares its
        // powers by passing one slice twice.
        let p = if ptr::eq(a, b) {
            &x * &x
        } else {
            let y = natural(&mut self.bufs[1], b);
            let p = &x * &y;
            self.bufs[1] = y.into_limbs_asc();
            p
        };
        self.bufs[0] = x.into_limbs_asc();
        let p = p.into_limbs_asc();
        let (lo, hi) = out.split_at_mut(p.len());
        lo.copy_from_slice(&p);
        hi.fill(0);
    }
}

/// `limbs` as a [`Natural`] that owns `buf`'s vector. [`Natural::into_limbs_asc`] gives the
/// vector back.
fn natural(buf: &mut Vec<u64>, limbs: &[u64]) -> Natural {
    let mut v = mem::take(buf);
    v.clear();
    v.extend_from_slice(limbs);
    Natural::from_owned_limbs_asc(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::CHUNK;
    use crate::dc::DC_THRESHOLD;
    use crate::mul;
    use crate::tests::{decode_vec, random_string};
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    /// `m.mul(a, b)` against Karatsuba, with garbage in `out` and no scratch at all.
    fn check(m: &mut Malachite, a: &[u64], b: &[u64]) {
        let mut want = vec![0; a.len() + b.len()];
        mul::mul(&mut want, a, b, &mut vec![0; mul::scratch_len(a.len().max(b.len()))]);
        let mut got = vec![0x5A5A_5A5A_5A5A_5A5A; want.len()];
        m.mul(&mut got, a, b, &mut []);
        assert!(got == want, "{} x {}", a.len(), b.len());
    }

    /// Lengths around malachite's thresholds on 64-bit targets: Toom-22 at 16 limbs,
    /// Toom-33 at 105, Toom-6h at 300, Toom-8h and unbalanced FFT at 389, balanced FFT at
    /// 1000. One [`Malachite`] serves every product, so its buffers grow and shrink.
    #[test]
    fn mul_matches_karatsuba() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(40);
        let mut m = Malachite::default();
        let lens = [0, 1, 2, 15, 16, 17, 104, 105, 300, 389, 390, 999, 1000, 1001, 2500];
        let random = |rng: &mut Xoshiro256PlusPlus, n| (0..n).map(|_| rng.next_u64()).collect();
        for na in lens {
            let a: Vec<u64> = random(&mut rng, na);
            // The same slice twice takes malachite's squaring path.
            check(&mut m, &a, &a);
            check(&mut m, &a, &a.clone());
            for nb in lens {
                check(&mut m, &a, &random(&mut rng, nb));
                // All-ones limbs maximise every carry.
                check(&mut m, &vec![u64::MAX; na], &vec![u64::MAX; nb]);
            }
        }
    }

    #[test]
    fn matches_karatsuba_decode() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(41);
        let t = DC_THRESHOLD * CHUNK;
        for len in [t + 1, 2 * t + 1, 10_000, 40_960, 100_001, 1_000_000] {
            // 58^len - 1 maximises carries.
            let max = "z".repeat(len);
            let min = format!("2{}", "1".repeat(len - 1));
            let zeros = format!("111{max}");
            for s in [random_string(&mut rng, len), max, min, zeros] {
                assert!(decode_vec(decode_malachite, &s) == decode_vec(crate::decode, &s), "{len}");
            }
        }
    }
}

//! Test and benchmark adapter for divide and conquer with malachite-nz's multiplication,
//! which picks schoolbook, Toom-Cook or FFT by operand size.
//!
//! malachite's limb-slice functions are public only with its `test_build` feature, which
//! pulls in GMP, pyo3 and more. So products go through [`Natural`]: the operands are
//! copied into reused buffers, and malachite allocates each product (and the FFT its own
//! buffers).

#![allow(dead_code)]

use std::{mem, ptr};

use malachite_nz::{natural::Natural, platform::Limb};

use faster_base58::internals::{Leaves, Multiplier, decode_with, encode_tuned, ifma_threshold};
use faster_base58::{BufferTooSmall, Error};

/// Like [`faster_base58::internals::decode_karatsuba`], but multiplies with malachite-nz.
pub fn decode_malachite(input: &str, output: &mut [u8]) -> Result<usize, Error> {
    decode_with(input, output, &mut Malachite::default())
}

/// The encoder's divide-and-conquer threshold in bytes and leaf size in chunks, tuned on an
/// M5 Pro against `encode_batched` in one binary.
pub(crate) const ENCODE_THRESHOLD: usize = 3072;
const ENCODE_LEAF: usize = 96;

/// [`ENCODE_THRESHOLD`] on CPUs with the IFMA leaf, where `encode_batched` is slower
/// relative to malachite. On Zen 5 this encoder (96-chunk leaves) took 3% longer than
/// `encode_batched` at 1 KiB and 15% less time at 1.5 KiB.
pub(crate) const ENCODE_IFMA_THRESHOLD: usize = 1280;

/// Like [`faster_base58::internals::encode_karatsuba`], but multiplies with malachite-nz.
pub fn encode_malachite(input: &[u8], output: &mut [u8]) -> Result<usize, BufferTooSmall> {
    let m = &mut Malachite::default();
    let threshold = match ifma_threshold() {
        0 => ENCODE_THRESHOLD,
        _ => ENCODE_IFMA_THRESHOLD,
    };
    encode_tuned(input, output, m, threshold, ENCODE_LEAF, Leaves::Horner)
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
    encode_tuned(
        input,
        output,
        &mut Malachite::default(),
        threshold,
        leaf,
        leaves,
    )
}

/// The [`Multiplier`] of [`decode_malachite`].
#[derive(Default)]
pub(crate) struct Malachite {
    /// Operand buffers, reused while [`Natural`] retains its multi-limb representation.
    bufs: [Vec<Limb>; 2],
}

impl Multiplier for Malachite {
    // malachite allocates its own scratch. `n` is the least that `dc` needs for its own
    // temporaries (see `Multiplier::scratch_len`).
    fn scratch_len(&self, n: usize) -> usize {
        n
    }

    #[allow(
        clippy::unnecessary_cast,
        reason = "Malachite Limb can be u32 after dependency feature unification"
    )]
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
        let per_word = 64 / Limb::BITS as usize;
        let (lo, hi) = out.split_at_mut(p.len().div_ceil(per_word));
        for (o, c) in lo.iter_mut().zip(p.chunks(per_word)) {
            *o = c[0] as u64;
            if Limb::BITS == 32 {
                *o |= (c.get(1).copied().unwrap_or(0) as u64) << 32;
            }
        }
        hi.fill(0);
    }
}

/// `limbs` as a [`Natural`] that owns `buf`'s vector. Multi-limb values return that vector
/// through [`Natural::into_limbs_asc`]; zero and one-limb values can replace it.
fn natural(buf: &mut Vec<Limb>, limbs: &[u64]) -> Natural {
    let mut v = mem::take(buf);
    v.clear();
    if Limb::BITS == 64 {
        v.extend(limbs.iter().map(|&l| l as Limb));
    } else {
        v.extend(limbs.iter().flat_map(|&l| [l as Limb, (l >> 32) as Limb]));
    }
    Natural::from_owned_limbs_asc(v)
}

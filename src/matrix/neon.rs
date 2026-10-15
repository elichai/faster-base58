//! NEON kernels. Same contracts as `portable.rs`.

use core::arch::aarch64::*;

use super::DIGITS;

/// Digits of the bytes `'1'..='p'`, indexed by `c - '1'`.
const LOW: [u8; 64] = table::<64>(b'1');
/// Digits of the bytes `'q'..=0x80`, indexed by `c - 'q'`.
const HIGH: [u8; 16] = table::<16>(b'q');

const fn table<const N: usize>(first: u8) -> [u8; N] {
    let mut t = [0; N];
    let mut i = 0;
    while i < N {
        t[i] = DIGITS[first as usize + i];
        i += 1;
    }
    t
}

/// See `portable::parse`. Writes all `4 * digits.len().div_ceil(16)` groups.
pub(super) fn parse(digits: &[u8], groups: &mut [u32]) -> bool {
    let n = digits.len();
    let (full, rem) = (n / 16, n % 16);
    assert!(groups.len() >= 4 * n.div_ceil(16));
    // SAFETY: NEON is enabled at compile time (cfg on the module). Block `k < full`
    // loads `digits[n - 16 (k + 1)..n - 16 k]`, and the head loads a local array. Block
    // `k` stores `groups[4 k..4 k + 4]`, with `k < n.div_ceil(16)`: in bounds by the assert.
    unsafe {
        let low = vld1q_u8_x4(LOW.as_ptr());
        let high = vld1q_u8(HIGH.as_ptr());
        // Reverses the bytes, so each u32 lane holds one group, least significant digit first.
        const REV: [u8; 16] = [15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
        let rev = vld1q_u8(REV.as_ptr());
        const W1: [u8; 16] = [1, 58, 1, 58, 1, 58, 1, 58, 1, 58, 1, 58, 1, 58, 1, 58];
        let w1 = vld1q_u8(W1.as_ptr());
        const W2: [u16; 8] = [1, 3364, 1, 3364, 1, 3364, 1, 3364];
        let w2 = vld1q_u16(W2.as_ptr());
        let mut bad = vdupq_n_u8(0);
        let out = groups.as_mut_ptr();
        let mut block = |v: uint8x16_t, k: usize| {
            // Bytes below '1' wrap to large indices; non-ASCII bytes give idx >= 80. Both
            // keep 0xFF.
            let idx = vsubq_u8(v, vdupq_n_u8(b'1'));
            let x = vqtbx4q_u8(vdupq_n_u8(0xFF), low, idx);
            let x = vqtbx1q_u8(x, high, vsubq_u8(idx, vdupq_n_u8(64)));
            bad = vorrq_u8(bad, x);
            let x = vqtbl1q_u8(x, rev);
            // Pairs `x0 + 58 x1` (at most 255 * 59 < 2^16), then `p0 + 3364 p1`.
            let p = vpaddq_u16(vmull_u8(vget_low_u8(x), vget_low_u8(w1)), vmull_high_u8(x, w1));
            let q = vpaddq_u32(vmull_u16(vget_low_u16(p), vget_low_u16(w2)), vmull_high_u16(p, w2));
            vst1q_u32(out.add(4 * k), q);
        };
        for k in 0..full {
            block(vld1q_u8(digits.as_ptr().add(n - 16 * (k + 1))), k);
        }
        if rem != 0 {
            // '1' is digit 0, so padding on the left keeps the value.
            let mut head = [b'1'; 16];
            head[16 - rem..].copy_from_slice(&digits[..rem]);
            block(vld1q_u8(head.as_ptr()), full);
        }
        vmaxvq_u8(bad) < 0x80
    }
}

/// See `portable::panel`.
pub(super) fn panel(acc: &mut [u64], words: &[u32], d: &[u32; 8]) {
    let nb = acc.len() / 4;
    assert!(acc.len() == 4 * nb && words.len() == 32 * nb);
    // SAFETY: NEON is enabled at compile time. Block `b < nb` reads and writes
    // `acc[4 b..4 b + 4]` and reads `words[32 b..32 b + 32]`: in bounds by the assert.
    unsafe {
        let d0 = vld1q_u32(d.as_ptr());
        let d1 = vld1q_u32(d.as_ptr().add(4));
        macro_rules! row {
            ($a0:ident, $a1:ident, $t:expr, $d:expr, $lane:literal) => {{
                let t = vld1q_u32($t);
                $a0 = vmlal_laneq_u32::<$lane>($a0, vget_low_u32(t), $d);
                $a1 = vmlal_high_laneq_u32::<$lane>($a1, t, $d);
            }};
        }
        for b in 0..nb {
            let a = acc.as_mut_ptr().add(4 * b);
            let t = words.as_ptr().add(32 * b);
            let mut a0 = vld1q_u64(a);
            let mut a1 = vld1q_u64(a.add(2));
            row!(a0, a1, t, d0, 0);
            row!(a0, a1, t.add(4), d0, 1);
            row!(a0, a1, t.add(8), d0, 2);
            row!(a0, a1, t.add(12), d0, 3);
            row!(a0, a1, t.add(16), d1, 0);
            row!(a0, a1, t.add(20), d1, 1);
            row!(a0, a1, t.add(24), d1, 2);
            row!(a0, a1, t.add(28), d1, 3);
            vst1q_u64(a, a0);
            vst1q_u64(a.add(2), a1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::portable;
    use super::*;
    use crate::tests::random_string;
    use rand_xoshiro::Xoshiro256PlusPlus;
    use rand_xoshiro::rand_core::{Rng, SeedableRng};

    #[test]
    fn parse_matches_portable() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(10);
        for n in (0..=100).chain([255, 256, 257, 1000]) {
            let valid = random_string(&mut rng, n);
            let mut inputs = vec![valid.clone()];
            if n > 0 {
                // One invalid byte, anywhere: only the result flag must agree.
                let pos = rng.next_u32() as usize % n;
                inputs.push(format!("{}0{}", &valid[..pos], &valid[pos + 1..]));
            }
            for s in inputs {
                let len = 4 * s.len().div_ceil(16);
                let (mut a, mut b) = (vec![0; len], vec![0; len]);
                let ok = portable::parse(s.as_bytes(), &mut b);
                assert_eq!(parse(s.as_bytes(), &mut a), ok, "{s:?}");
                if ok {
                    assert_eq!(a, b, "{s:?}");
                }
            }
        }
    }

    #[test]
    fn panel_matches_portable() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(11);
        for nb in [0, 1, 2, 7, 50] {
            let acc: Vec<u64> = (0..4 * nb).map(|_| rng.next_u64() >> 24).collect();
            let words: Vec<u32> = (0..32 * nb).map(|_| rng.next_u32()).collect();
            let d: [u32; 8] = std::array::from_fn(|_| rng.next_u32() % 58u32.pow(4));
            let (mut a, mut b) = (acc.clone(), acc);
            panel(&mut a, &words, &d);
            portable::panel(&mut b, &words, &d);
            assert_eq!(a, b, "nb {nb}");
        }
    }
}

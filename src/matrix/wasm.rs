//! Wasm SIMD128 kernels. Same contracts as `portable.rs`.

use core::arch::wasm32::*;

use super::DIGITS;

/// Digits of the bytes `'1'..=0x80`, indexed by `c - '1'`, in five 16-byte tables.
const TABLES: [[u8; 16]; 5] = {
    let mut t = [[0; 16]; 5];
    let mut i = 0;
    while i < 80 {
        t[i / 16][i % 16] = DIGITS[b'1' as usize + i];
        i += 1;
    }
    t
};

/// See `portable::parse`. Writes all `4 * digits.len().div_ceil(16)` groups.
pub(super) fn parse(digits: &[u8], groups: &mut [u32]) -> bool {
    let n = digits.len();
    let (full, rem) = (n / 16, n % 16);
    assert!(groups.len() >= 4 * n.div_ceil(16));
    let tables = TABLES.map(|t| {
        u8x16(
            t[0], t[1], t[2], t[3], t[4], t[5], t[6], t[7], t[8], t[9], t[10], t[11], t[12], t[13],
            t[14], t[15],
        )
    });
    let mut bad = u8x16_splat(0);
    let out = groups.as_mut_ptr();
    let mut block = |v: v128, k: usize| {
        // Bytes below '1' wrap to large indices; non-ASCII bytes give idx >= 80. Both
        // get 0xFF; `swizzle` gives 0 for indices of 16 or more.
        let idx = u8x16_sub(v, u8x16_splat(b'1'));
        let mut x = u8x16_ge(idx, u8x16_splat(80));
        for (i, &t) in tables.iter().enumerate() {
            x = v128_or(
                x,
                u8x16_swizzle(t, u8x16_sub(idx, u8x16_splat(16 * i as u8))),
            );
        }
        bad = v128_or(bad, x);
        // u16 lanes hold two digits, the more significant in the low byte: `58 x0 + x1`
        // (at most 255 * 59 < 2^15). Then u32 lanes, the more significant pair first.
        let p = i16x8_add(
            i16x8_mul(v128_and(x, u16x8_splat(0xFF)), u16x8_splat(58)),
            u16x8_shr(x, 8),
        );
        let g = i32x4_dot_i16x8(p, i32x4_splat(3364 | (1 << 16)));
        // The last group in memory is the least significant.
        // SAFETY: block `k` stores `groups[4 k..4 k + 4]`, with `k < n.div_ceil(16)`: in
        // bounds by the assert.
        unsafe { v128_store(out.add(4 * k).cast(), i32x4_shuffle::<3, 2, 1, 0>(g, g)) };
    };
    for k in 0..full {
        // SAFETY: block `k < full` loads `digits[n - 16 (k + 1)..n - 16 k]`.
        block(
            unsafe { v128_load(digits.as_ptr().add(n - 16 * (k + 1)).cast()) },
            k,
        );
    }
    if rem != 0 {
        // '1' is digit 0, so padding on the left keeps the value.
        let mut head = [b'1'; 16];
        head[16 - rem..].copy_from_slice(&digits[..rem]);
        // SAFETY: `head` has 16 bytes.
        block(unsafe { v128_load(head.as_ptr().cast()) }, full);
    }
    u8x16_bitmask(bad) == 0
}

/// See `portable::panel`.
pub(super) fn panel(acc: &mut [u64], words: &[u32], d: &[u32; 8]) {
    let nb = acc.len() / 4;
    assert!(acc.len() == 4 * nb && words.len() == 32 * nb);
    let splats = d.map(|d| u32x4_splat(d));
    for b in 0..nb {
        // From memory: LLVM would otherwise hoist the splats' zero extension out of the
        // loop, and the products become `i64x2.mul`, which V8 expands to about seven
        // instructions on `AArch64`.
        let d = *core::hint::black_box(&splats);
        // SAFETY: block `b < nb` reads and writes `acc[4 b..4 b + 4]` and reads
        // `words[32 b..32 b + 32]`: in bounds by the assert.
        unsafe {
            let a = acc.as_mut_ptr().add(4 * b);
            let t = words.as_ptr().add(32 * b);
            let mut a0 = v128_load(a.cast());
            let mut a1 = v128_load(a.add(2).cast());
            for (r, &d) in d.iter().enumerate() {
                let w = v128_load(t.add(4 * r).cast());
                a0 = i64x2_add(a0, u64x2_extmul_low_u32x4(w, d));
                a1 = i64x2_add(a1, u64x2_extmul_high_u32x4(w, d));
            }
            v128_store(a.cast(), a0);
            v128_store(a.add(2).cast(), a1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::portable;
    use super::*;

    #[test]
    fn parse_matches_portable() {
        super::super::tests::check_parse_matches_portable(parse);
    }

    /// Every byte value at every position of a block, against the scalar flag.
    #[test]
    fn parse_flags_every_byte() {
        for c in 0..=255u8 {
            for pos in 0..17 {
                let mut s = vec![b'z'; 17];
                s[pos] = c;
                let mut g = vec![0; 8];
                let ok = parse(&s, &mut g);
                assert_eq!(ok, DIGITS[c as usize] < 58, "byte {c} at {pos}");
                if ok {
                    let mut want = vec![0; 8];
                    assert!(portable::parse(&s, &mut want));
                    assert_eq!(g, want, "byte {c} at {pos}");
                }
            }
        }
    }

    #[test]
    fn panel_matches_portable() {
        super::super::tests::check_panel_matches_portable(panel);
    }
}

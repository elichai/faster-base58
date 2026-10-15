//! Plain-Rust kernels: the fallback without NEON, and the reference in tests.

use super::DIGITS;

/// Groups `digits` in fours from the end, least significant group first. A short first
/// group is padded with zero digits. Returns whether every byte is a base58 digit.
///
/// Requires `groups` to be zero, with room for `4 * digits.len().div_ceil(16)` groups.
/// Groups past `digits.len().div_ceil(4)` stay zero.
pub(super) fn parse(digits: &[u8], groups: &mut [u32]) -> bool {
    debug_assert!(groups.len() >= 4 * digits.len().div_ceil(16));
    let (head, tail) = digits.as_rchunks::<4>();
    let head = Some(head).filter(|h| !h.is_empty());
    let mut bad = 0;
    let sources = tail.iter().rev().map(|c| c.as_slice()).chain(head);
    for (g, s) in groups.iter_mut().zip(sources) {
        // At most 255 * (58^3 + 58^2 + 58 + 1) < 2^26, even for invalid bytes.
        *g = s.iter().fold(0, |v, &c| {
            let d = DIGITS[c as usize];
            bad |= d;
            v * 58 + d as u32
        });
    }
    bad < 0x80
}

/// Adds group `d[r]` times row `r` of one panel to its columns:
/// `acc[4 b + w] += d[r] * words[(8 b + r) * 4 + w]`.
///
/// Requires `words.len() == 8 * acc.len()`, a multiple of 32.
pub(super) fn panel(acc: &mut [u64], words: &[u32], d: &[u32; 8]) {
    let (blocks, rest) = acc.as_chunks_mut::<4>();
    let (rows, words_rest) = words.as_chunks::<32>();
    assert!(rest.is_empty() && words_rest.is_empty() && blocks.len() == rows.len());
    let d = d.map(u64::from);
    for (a, t) in blocks.iter_mut().zip(rows) {
        let mut s = *a;
        for r in 0..8 {
            for w in 0..4 {
                s[w] += d[r] * t[4 * r + w] as u64;
            }
        }
        *a = s;
    }
}

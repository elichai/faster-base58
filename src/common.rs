//! Pieces shared by the decoders: the alphabet, chunk parsing and error reporting.

use core::iter;

use crate::{BufferTooSmall, Error};

/// A decoding failure inside the crate. Only the public terminal methods convert it
/// to [`Error`], so internal result layouts never depend on optional public variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecodeError {
    InvalidCharacter { index: usize },
    BufferTooSmall,
}

impl From<BufferTooSmall> for DecodeError {
    #[inline]
    fn from(BufferTooSmall: BufferTooSmall) -> Self {
        Self::BufferTooSmall
    }
}

impl From<DecodeError> for Error {
    #[inline]
    fn from(error: DecodeError) -> Self {
        match error {
            DecodeError::InvalidCharacter { index } => Self::InvalidCharacter { index },
            DecodeError::BufferTooSmall => Self::BufferTooSmall,
        }
    }
}

pub(crate) const ALPHABET: &[u8; 58] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Digit value of each byte. `0xFF` marks an invalid character.
pub(crate) const DIGITS: [u8; 256] = {
    let mut t = [0xFF; 256];
    let mut i = 0;
    while i < ALPHABET.len() {
        t[ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    t
};

/// Characters per chunk. `58^10 < 2^64`.
pub(crate) const CHUNK: usize = 10;
pub(crate) const BASE: u64 = 58u64.pow(CHUNK as u32);

/// Whether the paired multiply-add loops in [`crate::batched`] and [`crate::dc`] take four
/// limbs per iteration.
///
/// With one limb, the x86-64 loop was 49 bytes and its closing branch straddled a 64-byte
/// line whenever the loop started at 16 mod 64; there Zen 5 decoded 512 B-2 KiB 1.2-1.3x
/// slower. Four limbs varied by at most 5% across 16-byte code shifts and took 6-22% less
/// time on Zen 5 (best placements compared) and 4-31% on the M5. V8 ran unrolled Wasm
/// loops 4-30% slower.
pub(crate) const UNROLL_PAIRED: bool = !cfg!(target_family = "wasm");

/// Splits off the leading '1's, each of which decodes to a zero byte.
// A slice-pattern loop: no bounds check, and LLVM keeps the digit count within the input
// length, so a bounded input folds the decoder selection. `take_while().count()` with
// `&input[zeros..]` kept the check and lost the bound (every backend stayed linked).
pub(crate) fn split_zeros(input: &[u8]) -> (usize, &[u8]) {
    let mut digits = input;
    while let [b'1', rest @ ..] = digits {
        digits = rest;
    }
    (input.len() - digits.len(), digits)
}

/// Base-`BASE` digits of `digits`, most significant first.
/// `None` marks a chunk with an invalid character.
// An x86 AVX2 block parser for these chunks (nibble-table map, `vpmaddubsw`/`vpmaddwd`,
// two `vpmuludq` per chunk, 32 chunks per block) was not adopted. On Zen 5 (generic
// flags, alignment pinned) decoding took 1.15-1.21x as long at 32 B, 0.91-0.93x at 256 B and
// 0.97-0.99x at 1-64 KiB; the kernel was slower than this loop below 60 characters.
pub(crate) fn chunks(digits: &[u8]) -> impl Iterator<Item = Option<u64>> {
    let valid = |(v, bad): (u64, u8)| (bad < 0x80).then_some(v);
    let (head, tail) = digits.as_rchunks::<CHUNK>();
    let head = iter::once(head)
        .filter(|h| !h.is_empty())
        .map(move |s| valid(short_chunk(s)));
    head.chain(tail.iter().map(move |s| valid(full_chunk(s))))
}

/// [`chunks`] of valid `digits` into `out`, one value per chunk, in one loop: a caller
/// that zipped the iterator into its buffer got its `next` as a call per chunk.
#[cfg(any(feature = "alloc", test, kani))]
pub(crate) fn values(digits: &[u8], out: &mut [u64]) {
    let (head, tail) = digits.as_rchunks::<CHUNK>();
    let short = usize::from(!head.is_empty());
    assert_eq!(out.len(), short + tail.len(), "one value per chunk");
    let (first, rest) = out.split_at_mut(short);
    let mut bad = 0;
    if let [o] = first {
        (*o, bad) = short_chunk(head);
    }
    for (o, s) in rest.iter_mut().zip(tail) {
        let (v, b) = full_chunk(s);
        *o = v;
        bad |= b;
    }
    assert!(bad < 0x80, "caller validates the digits");
}

/// The value of up to `CHUNK` digits and the OR of their digit values, which reaches
/// `0x80` for an invalid character.
#[inline(always)]
fn short_chunk(s: &[u8]) -> (u64, u8) {
    let (mut v, mut bad) = (0u64, 0u8);
    for &c in s {
        let d = DIGITS[c as usize];
        bad |= d;
        // Cannot overflow even with 0xFF digits: 255 * (58^10 - 1) / 57 < 2^61.
        v = v * 58 + d as u64;
    }
    (v, bad)
}

/// [`short_chunk`] of a full chunk.
#[inline(always)]
fn full_chunk(s: &[u8; CHUNK]) -> (u64, u8) {
    let d = s.map(|c| DIGITS[c as usize]);
    let bad = d.iter().fold(0, |a, &x| a | x);
    // Two independent chains of 5, joined: same value as the chain of 10.
    let half = |h: &[u8]| h.iter().fold(0u64, |v, &x| v * 58 + x as u64);
    (half(&d[..5]) * 58u64.pow(5) + half(&d[5..]), bad)
}

/// The first invalid byte in `input`, reported relative to a prefix of `offset` bytes.
pub(crate) fn invalid(input: &[u8], offset: usize) -> DecodeError {
    let index = input.iter().position(|&c| DIGITS[c as usize] == 0xFF);
    DecodeError::InvalidCharacter {
        index: offset + index.expect("caller saw an invalid character"),
    }
}

/// Stores the low `out.len()` (at most 8) bytes of `value`, least significant first.
///
/// Fixed-width stores, like the loads in `core::hash::sip::u8to64_le`: a
/// variable-length `copy_from_slice` compiles to a stack spill and a `memcpy` call. One
/// overlapping 8-byte store ending at the length measured faster at 21-25 bytes but
/// slower at 32-82 bytes (Zen 5, M5), so it is not used.
#[inline]
pub(crate) fn store_low_le(out: &mut [u8], value: u64) {
    let len = out.len();
    debug_assert!(len <= 8);
    if len == 8 {
        out.copy_from_slice(&value.to_le_bytes());
        return;
    }
    let mut i = 0;
    if i + 3 < len {
        out[i..i + 4].copy_from_slice(&(value as u32).to_le_bytes());
        i += 4;
    }
    if i + 1 < len {
        out[i..i + 2].copy_from_slice(&((value >> (8 * i)) as u16).to_le_bytes());
        i += 2;
    }
    if i < len {
        out[i] = (value >> (8 * i)) as u8;
    }
}

/// Whether every byte of `digits` is in the alphabet.
///
/// Branch-free range checks with no early exit, so the loop vectorizes: on an M5 Pro
/// 0.075 ns per byte, against 0.16 for [`DIGITS`] lookups and 1.8 for `matches!`, whose
/// branches mispredict on random input.
#[cfg(any(feature = "alloc", test, kani))]
pub(crate) fn all_valid(digits: &[u8]) -> bool {
    let valid = |c: u8| {
        (c.wrapping_sub(b'1') < 9)
            | (c.wrapping_sub(b'A') < 8)
            | (c.wrapping_sub(b'J') < 5)
            | (c.wrapping_sub(b'P') < 11)
            | (c.wrapping_sub(b'a') < 11)
            | (c.wrapping_sub(b'm') < 14)
    };
    digits.iter().fold(0u8, |bad, &c| bad | !valid(c) as u8) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_low_le_writes_only_its_bytes() {
        let value = 0x0807_0605_0403_0201;
        for len in 0..=8 {
            let mut buf = [0xA5; 10];
            store_low_le(&mut buf[1..1 + len], value);
            assert_eq!(buf[1..1 + len], value.to_le_bytes()[..len], "len {len}");
            assert_eq!(buf[0], 0xA5);
            assert!(buf[1 + len..].iter().all(|&b| b == 0xA5), "len {len}");
        }
    }

    #[test]
    fn values_match_chunks() {
        // No head, every head length, and a full leaf of 256 chunks.
        for len in (0..=21).chain([2559, 2560]) {
            let digits: Vec<u8> = (0..len).map(|i| ALPHABET[(i * 37 + 11) % 58]).collect();
            let want: Vec<u64> = chunks(&digits).map(Option::unwrap).collect();
            let mut got = vec![0; want.len()];
            values(&digits, &mut got);
            assert_eq!(got, want, "len {len}");
        }
    }

    #[test]
    #[should_panic(expected = "caller validates the digits")]
    fn values_reject_an_invalid_head() {
        values(b"01111111111", &mut [0; 2]);
    }

    #[test]
    #[should_panic(expected = "caller validates the digits")]
    fn values_reject_an_invalid_chunk() {
        values(b"11111111110", &mut [0; 2]);
    }

    #[test]
    #[should_panic(expected = "one value per chunk")]
    fn values_need_one_value_per_chunk() {
        values(b"11111111111", &mut [0; 1]);
    }
}

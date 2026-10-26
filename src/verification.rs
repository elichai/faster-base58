//! Bounded proofs of production parser and shared arithmetic. Backend-local proofs
//! live next to their private kernels. These do not prove unbounded conversions or SIMD.

use crate::common::{self, BASE, CHUNK, DIGITS};
use crate::mul;

#[kani::proof]
#[kani::unwind(5)]
fn validity_matches_digit_table() {
    let input: [u8; 4] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= input.len());
    let input = &input[..len];
    let expected = input.iter().all(|&byte| DIGITS[byte as usize] != u8::MAX);
    assert_eq!(common::all_valid(input), expected);
}

#[kani::proof]
#[kani::unwind(9)]
fn leading_zero_split_is_exact() {
    let input: [u8; 8] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= input.len());
    let (zeros, rest) = common::split_zeros(&input[..len]);
    assert!(zeros <= len);
    assert!(input[..zeros].iter().all(|&byte| byte == b'1'));
    assert_eq!(rest, &input[zeros..len]);
    assert!(rest.is_empty() || rest[0] != b'1');
}

#[kani::proof]
#[kani::unwind(12)]
fn full_chunk_matches_horner_and_rejects_invalid() {
    let input: [u8; CHUNK] = kani::any();
    let mut expected = 0u64;
    let mut valid = true;
    for &byte in &input {
        let digit = DIGITS[byte as usize];
        valid &= digit != u8::MAX;
        expected = expected * 58 + digit as u64;
    }
    let mut got = common::chunks(&input);
    assert_eq!(got.next(), Some(valid.then_some(expected)));
    assert!(got.next().is_none());
    assert!(!valid || expected < BASE);
}

// Together these two cases cover every input of the original full-chunk harness.
// Keep that harness above as the monolithic reference obligation.
#[kani::proof]
fn digit_table_has_only_valid_values_or_sentinel() {
    let byte: u8 = kani::any();
    let digit = DIGITS[byte as usize];
    assert!(digit < 58 || digit == u8::MAX);
}

// Prove the table definitions separately from parser control flow.
#[kani::proof]
fn alphabet_character_roundtrips() {
    let digit: u8 = kani::any();
    kani::assume(digit < 58);
    let character = common::ALPHABET[digit as usize];
    assert_eq!(DIGITS[character as usize], digit);
    assert!(character < 128);
}

#[kani::proof]
#[kani::unwind(12)]
fn full_chunk_valid_numeric_identity() {
    let input: [u8; CHUNK] = kani::any();
    let mut expected = 0u64;
    for &byte in &input {
        let digit = DIGITS[byte as usize];
        // The separate table-range obligation proves that this is exactly the
        // complement of the sentinel case, rather than a narrower valid domain.
        kani::assume(digit < 58);
        expected = expected * 58 + digit as u64;
    }
    let mut got = common::chunks(&input);
    assert_eq!(got.next(), Some(Some(expected)));
    assert!(got.next().is_none());
    assert!(expected < BASE);
}

#[kani::proof]
#[kani::unwind(12)]
fn full_chunk_invalid_rejection() {
    let input: [u8; CHUNK] = kani::any();
    kani::assume(input.iter().any(|&byte| DIGITS[byte as usize] == u8::MAX));
    let mut got = common::chunks(&input);
    assert_eq!(got.next(), Some(None));
    assert!(got.next().is_none());
}

#[kani::proof]
#[kani::unwind(12)]
fn partial_chunk_matches_horner_and_rejects_invalid() {
    let input: [u8; CHUNK - 1] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= input.len());
    let mut expected = 0u64;
    let mut valid = true;
    for &byte in &input[..len] {
        let digit = DIGITS[byte as usize];
        valid &= digit != u8::MAX;
        expected = expected * 58 + digit as u64;
    }
    let mut got = common::chunks(&input[..len]);
    let expected = (len != 0).then_some(valid.then_some(expected));
    assert_eq!(got.next(), expected);
    assert!(got.next().is_none());
}

fn value(limbs: &[u64; 2]) -> u128 {
    limbs[0] as u128 | (limbs[1] as u128) << 64
}

#[kani::proof]
#[kani::unwind(4)]
fn two_limb_add_matches_u128_with_carry() {
    let mut a: [u64; 2] = kani::any();
    let mut b: [u64; 2] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= b.len());
    b[len..].fill(0);
    let (expected, carry) = value(&a).overflowing_add(value(&b));
    assert_eq!(mul::add_assign(&mut a, &b[..len]), carry);
    assert_eq!(value(&a), expected);
}

#[kani::proof]
#[kani::unwind(4)]
fn two_limb_sub_matches_u128_with_borrow() {
    let mut a: [u64; 2] = kani::any();
    let mut b: [u64; 2] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= b.len());
    b[len..].fill(0);
    let (expected, borrow) = value(&a).overflowing_sub(value(&b));
    assert_eq!(mul::sub_assign(&mut a, &b[..len]), borrow);
    assert_eq!(value(&a), expected);
}

#[kani::proof]
#[kani::unwind(5)]
fn wrapped_fold_preserves_residue() {
    let input: [u64; 3] = kani::any();
    let mut out = [kani::any::<u64>()];
    mul::fold_wrapped(&mut out, &input);
    let modulus = u64::MAX as u128;
    let expected = input.iter().map(|&limb| limb as u128).sum::<u128>() % modulus;
    assert_eq!(out[0] as u128 % modulus, expected);
}

#[kani::proof]
#[kani::unwind(4)]
fn one_limb_production_multiply_matches_u128() {
    let a: [u64; 1] = kani::any();
    let b: [u64; 1] = kani::any();
    let mut out = [kani::any::<u64>(), kani::any::<u64>()];
    let mut scratch = [0; mul::scratch_len(1)];
    mul::mul_with(&mut out, &a, &b, &mut scratch, 0);
    assert_eq!(value(&out), a[0] as u128 * b[0] as u128);
}

#[kani::proof]
#[kani::unwind(12)]
fn one_byte_batched_encoding_matches_radix_and_capacity() {
    let byte: u8 = kani::any();
    let capacity: usize = kani::any();
    kani::assume(capacity <= 3);
    let mut out = [0xA5; 5];
    let result = crate::encode::encode_batched(&[byte], &mut out[1..1 + capacity]);
    let digits = if byte < 58 { 1 } else { 2 };
    assert_eq!(out[0], 0xA5);
    assert!(out[1 + capacity..].iter().all(|&byte| byte == 0xA5));
    if capacity < digits {
        assert_eq!(result, Err(crate::BufferTooSmall));
    } else {
        assert_eq!(result, Ok(digits));
        if digits == 2 {
            assert_eq!(out[1], common::ALPHABET[(byte / 58) as usize]);
        }
        assert_eq!(out[digits], common::ALPHABET[(byte % 58) as usize]);
        assert!(out[1 + digits..].iter().all(|&byte| byte == 0xA5));
    }
}

#[kani::proof]
#[kani::unwind(12)]
fn two_digit_batched_decoding_matches_radix_and_capacity() {
    let digits: [u8; 2] = kani::any();
    kani::assume(digits[0] < 58 && digits[1] < 58);
    let text = [
        common::ALPHABET[digits[0] as usize],
        common::ALPHABET[digits[1] as usize],
    ];
    let input = core::str::from_utf8(&text).unwrap();
    let capacity: usize = kani::any();
    kani::assume(capacity <= 3);
    let mut out = [0xA5; 5];
    let result = crate::batched::decode_batched(input, &mut out[1..1 + capacity]);
    let value = digits[0] as u16 * 58 + digits[1] as u16;
    let zeros = if digits[0] == 0 {
        if digits[1] == 0 { 2 } else { 1 }
    } else {
        0
    };
    let bytes = zeros
        + if value == 0 {
            0
        } else if value <= 255 {
            1
        } else {
            2
        };
    assert_eq!(out[0], 0xA5);
    assert!(out[1 + capacity..].iter().all(|&byte| byte == 0xA5));
    if capacity < bytes {
        assert_eq!(result, Err(crate::Error::BufferTooSmall));
    } else {
        assert_eq!(result, Ok(bytes));
        assert!(out[1..1 + zeros].iter().all(|&byte| byte == 0));
        if value > 255 {
            assert_eq!(out[1 + zeros], (value >> 8) as u8);
        }
        if value != 0 {
            assert_eq!(out[bytes], value as u8);
        }
        assert!(out[1 + bytes..].iter().all(|&byte| byte == 0xA5));
    }
}

#[kani::proof]
fn valid_digit_table_entries_roundtrip_to_character() {
    let byte: u8 = kani::any();
    let digit = DIGITS[byte as usize];
    if digit != u8::MAX {
        assert!(digit < 58);
        assert_eq!(common::ALPHABET[digit as usize], byte);
        assert!(byte < 128);
    }
}

// Actual tiny-input decoder used by the public API. Every valid two-byte UTF-8
// string is covered, including invalid base58 characters and byte error offsets.
#[kani::proof]
#[kani::unwind(6)]
fn two_byte_short_decode_preserves_radix_errors_and_canaries() {
    let text: [u8; 2] = kani::any();
    let Ok(input) = core::str::from_utf8(&text) else {
        return;
    };
    let capacity: usize = kani::any();
    kani::assume(capacity <= 3);
    let mut out = [0xA5; 5];
    let result = crate::batched::decode_short(input, &mut out[1..1 + capacity]);
    assert_eq!(out[0], 0xA5);
    assert!(out[1 + capacity..].iter().all(|&byte| byte == 0xA5));
    let first = DIGITS[text[0] as usize];
    let second = DIGITS[text[1] as usize];
    if first == u8::MAX || second == u8::MAX {
        let index = usize::from(first != u8::MAX);
        assert_eq!(result, Err(crate::Error::InvalidCharacter { index }));
        return;
    }
    assert!(first < 58 && second < 58);
    let value = first as u16 * 58 + second as u16;
    let bytes = if first == 0 {
        2
    } else {
        1 + usize::from(value >= 256)
    };
    if capacity < bytes {
        assert_eq!(result, Err(crate::Error::BufferTooSmall));
    } else {
        assert_eq!(result, Ok(bytes));
        if first == 0 {
            assert_eq!(out[1], 0);
            assert_eq!(out[2], second);
        } else if value >= 256 {
            assert_eq!(out[1], (value >> 8) as u8);
            assert_eq!(out[2], value as u8);
        } else {
            assert_eq!(out[1], value as u8);
        }
        assert!(out[1 + bytes..].iter().all(|&byte| byte == 0xA5));
    }
}

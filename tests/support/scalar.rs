//! Small, independent radix oracle. No converter or bigint backend is called.
//!
//! Numeric values must fit in u128; an arbitrary leading-zero prefix is allowed.

pub const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub fn encode(input: &[u8]) -> Option<Vec<u8>> {
    let zeros = input.iter().take_while(|&&byte| byte == 0).count();
    let significant = &input[zeros..];
    if significant.len() > 16 {
        return None;
    }
    let mut value = significant
        .iter()
        .fold(0u128, |n, &byte| (n << 8) | byte as u128);
    let mut digits = Vec::new();
    while value != 0 {
        digits.push(ALPHABET[(value % 58) as usize]);
        value /= 58;
    }
    let mut result = vec![b'1'; zeros];
    result.extend(digits.into_iter().rev());
    Some(result)
}

pub fn decode(input: &str) -> Option<Result<Vec<u8>, usize>> {
    let mut value = Some(0u128);
    for (index, byte) in input.bytes().enumerate() {
        let Some(digit) = ALPHABET.iter().position(|&candidate| candidate == byte) else {
            return Some(Err(index));
        };
        value = value.and_then(|n| n.checked_mul(58)?.checked_add(digit as u128));
    }
    // Still scan for invalid bytes after overflow: validation and numeric range
    // are separate properties, and error indices count UTF-8 bytes.
    let value = value?;
    let zeros = input.bytes().take_while(|&byte| byte == b'1').count();
    let bytes = value.to_be_bytes();
    let first = bytes
        .iter()
        .position(|&byte| byte != 0)
        .unwrap_or(bytes.len());
    let mut result = vec![0; zeros];
    result.extend_from_slice(&bytes[first..]);
    Some(Ok(result))
}

/// Deterministic fuzz control independent of pointer width. Long inputs can
/// select positions after byte 255; their bytes remain conversion input too.
pub fn selector(input: &[u8]) -> usize {
    input
        .iter()
        .take(4)
        .chain(input.iter().rev().take(4))
        .fold(0u32, |value, &byte| value.rotate_left(8) ^ byte as u32) as usize
}

#[cfg(test)]
mod tests {
    #[allow(
        unused_imports,
        reason = "Benchmarks compile this module without the test harness"
    )]
    use super::*;

    #[test]
    fn vectors_and_range_are_explicit() {
        for (bytes, text) in [
            (&b""[..], ""),
            (&[0][..], "1"),
            (&[0, 0, 1][..], "112"),
            (&[57][..], "z"),
            (&[58][..], "21"),
        ] {
            assert_eq!(encode(bytes), Some(text.as_bytes().to_vec()));
            assert_eq!(decode(text), Some(Ok(bytes.to_vec())));
        }
        assert!(encode(&[1; 17]).is_none());
        assert!(decode(&"z".repeat(23)).is_none());
        assert_eq!(decode(&format!("{}é", "z".repeat(23))), Some(Err(23)));
        assert_eq!(encode(&[0; 64]), Some(vec![b'1'; 64]));
    }

    #[test]
    fn selector_can_inject_after_byte255() {
        let mut input = vec![0; 1024];
        input[1021] = 1;
        assert_eq!(selector(&input) % (input.len() + 1), 256);
    }
}

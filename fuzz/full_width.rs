//! Structured inputs keep native IFMA hosts above their NTT thresholds.

#![allow(
    dead_code,
    reason = "Each structured target uses a subset of these generators"
)]

pub const WIDTH: usize = 65_537;

pub fn expand(raw: &[u8]) -> Vec<u8> {
    expand_to(raw, WIDTH)
}

/// Keep constant/periodic cases, or select a nonperiodic tail with bit 6 of a
/// multi-byte input. The stream is deterministic; it adds no entropy to the seed.
pub fn expand_to(raw: &[u8], width: usize) -> Vec<u8> {
    assert!(width > 0);
    let mut input = vec![0xA5; width];
    if raw.len() > 1 && raw[0] & 0x40 != 0 {
        let mut state = raw.iter().fold(0xcbf2_9ce4_8422_2325u64, |state, &byte| {
            (state ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
        });
        for chunk in input.chunks_mut(8) {
            // SplitMix64: use wrapping arithmetic even in the checked fuzz build.
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut word = state;
            word = (word ^ (word >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            word = (word ^ (word >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            let bytes = (word ^ (word >> 31)).to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        let n = raw.len().min(width);
        input[..n].copy_from_slice(&raw[..n]);
    } else if !raw.is_empty() {
        for chunk in input.chunks_mut(raw.len()) {
            chunk.copy_from_slice(&raw[..chunk.len()]);
        }
    }
    // Full width keeps the current native IFMA thresholds from avoiding transforms.
    input[0] |= 0x80;
    input
}

/// A large valid Base58 string, with an optional quarter-width leading-zero run.
pub fn decode_input(raw: &[u8]) -> Vec<u8> {
    const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let control = raw.first().copied().unwrap_or(0);
    let width = if control & 0x80 != 0 {
        1 << 21
    } else {
        1 << 17
    };
    let mut input = expand_to(raw, width);
    for byte in &mut input {
        *byte = ALPHABET[usize::from(*byte) % 58];
    }
    let zeros = if control & 0x20 != 0 { width / 4 } else { 0 };
    input[..zeros].fill(b'1');
    input[zeros] = b'z';
    input
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_zero_periodic_and_oversized_raw_inputs_stay_full_width() {
        for raw in [
            vec![],
            vec![0],
            vec![1, 2, 255],
            vec![0x1A; 8193],
            (0..WIDTH + 1).map(|i| i as u8).collect(),
        ] {
            let input = expand(&raw);
            assert_eq!(input.len(), WIDTH);
            assert_eq!(input[0].leading_zeros(), 0);
            for (i, &actual) in input.iter().enumerate() {
                let expected = if raw.is_empty() {
                    0xA5
                } else {
                    raw[i % raw.len()]
                };
                assert_eq!(actual, if i == 0 { expected | 0x80 } else { expected });
            }
        }
    }

    #[test]
    fn pseudorandom_mode_is_reproducible_nonperiodic_and_preserves_raw_bytes() {
        let raw = [0x40, 1, 2, 3];
        let input = expand(&raw);
        assert_eq!(input, expand(&raw));
        assert_eq!(&input[..raw.len()], &[0xC0, 1, 2, 3]);
        assert_ne!(&input[8..16], &input[16..24]);
        assert_ne!(&input[8..24], &expand(&[0x40, 1, 2, 4])[8..24]);
        assert_eq!(input.len(), WIDTH);
        assert_ne!(input[0] & 0x80, 0);
    }

    #[test]
    fn expansion_handles_short_tails_and_raw_inputs_larger_than_the_output() {
        for width in [1, 7, 8, 9, 128] {
            for raw in [vec![], vec![0x40, 1], vec![0x40; 129]] {
                let input = expand_to(&raw, width);
                assert_eq!(input.len(), width);
                assert_ne!(input[0] & 0x80, 0);
                assert_eq!(input, expand_to(&raw, width));
            }
        }
    }

    #[test]
    fn large_decode_inputs_preserve_width_alphabet_and_significant_suffix() {
        for control in [0, 0x20, 0x40, 0x80, 0xA0, 0xC0, 0xE0] {
            let input = decode_input(&[control, 1, 2]);
            let width = if control & 0x80 != 0 {
                1 << 21
            } else {
                1 << 17
            };
            assert_eq!(input.len(), width);
            assert!(input.iter().all(|b| {
                b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(b)
            }));
            let zeros = input.iter().take_while(|&&b| b == b'1').count();
            assert_eq!(zeros, if control & 0x20 != 0 { width / 4 } else { 0 });
        }
    }
}

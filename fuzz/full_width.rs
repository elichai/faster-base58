//! Structured inputs keep native IFMA hosts above their NTT thresholds.

pub const WIDTH: usize = 65_537;

pub fn expand(raw: &[u8]) -> Vec<u8> {
    let mut input = vec![0xA5; WIDTH];
    if !raw.is_empty() {
        for chunk in input.chunks_mut(raw.len()) {
            chunk.copy_from_slice(&raw[..chunk.len()]);
        }
    }
    // Full width keeps the current native IFMA thresholds from avoiding transforms.
    input[0] |= 0x80;
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
            vec![0x5A; 8193],
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
}

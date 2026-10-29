//! Base58 (Bitcoin alphabet) encoding. See DESIGN.md for the design.
//!
//! - [`encode_batched`] (`batched.rs`): quadratic, one base-58^10 limb update per seven
//!   input bytes, in place in the output.
//! - `dc.rs`: divide and conquer by powers of 58^10,
//!   each division Barrett's with a Newton reciprocal, over any `Multiplier`.
//! - [`matrix`] (`matrix.rs`): a matrix of powers with the decoder's SIMD
//!   multiply-adds, up to 8 KiB, and a fixed-size path for 32 and 64 bytes
//!   (`matrix/fixed.rs`).

pub(crate) mod batched;
pub(crate) mod dc;
pub(crate) mod matrix;
#[cfg(test)]
mod tests;

pub use batched::encode_batched;
pub use dc::Leaves;
#[cfg(any(test, feature = "__internals", kani))]
pub use dc::encode_tuned;

use crate::common::BASE;

/// `floor(log_58(2) * 2^64)`.
const LOG58_2: u128 = 3148993499475808360;

/// An upper bound on the encoded length of `len` bytes, saturating at `usize::MAX`.
pub const fn max_encoded_len(len: usize) -> usize {
    // Leading zero bytes take one character each, fewer than any other byte, so `len`
    // bytes encode to at most the digits of 2^(8 len) - 1.
    // Shift by 61 instead of multiplying by 8, so even `usize::MAX` fits in `u128`.
    let bound = ((len as u128 * (LOG58_2 + 1)) >> 61) + 1;
    if bound > usize::MAX as u128 {
        usize::MAX
    } else {
        bound as usize
    }
}

/// A lower bound on the digits of a value of `bits` significant bits, at most two below
/// the actual count: the value is at least 2^(bits - 1), and below 2^bits, whose digit
/// count is at most one more.
fn min_digits(bits: u64) -> usize {
    // `LOG58_2` rounds down, which loses at most one digit while `bits < 2^64`.
    (((bits - 1) as u128 * LOG58_2) >> 64) as usize + 1
}

/// Significant bits of the big-endian `x`, whose first byte is not zero.
fn bits(x: &[u8]) -> u64 {
    8 * x.len() as u64 - x[0].leading_zeros() as u64
}

/// Splits off the leading zero bytes, each of which encodes to a '1'.
pub(crate) fn split_zeros(input: &[u8]) -> (usize, &[u8]) {
    let zeros = input.iter().take_while(|&&b| b == 0).count();
    (zeros, &input[zeros..])
}

/// The ten characters of a limb below [`BASE`], most significant first.
fn ten_digits(y: u64) -> [u8; 10] {
    debug_assert!(y < BASE);
    const TWO: u64 = 58u64.pow(2);
    let top = matrix::CHAR_PAIRS[(y / TWO.pow(4)) as usize];
    let quotients = [y / TWO.pow(3), y / TWO.pow(2), y / TWO, y];
    let mut d = [0; 10];
    d[..2].copy_from_slice(&top);
    for (pair, q) in d[2..].as_chunks_mut::<2>().0.iter_mut().zip(quotients) {
        pair.copy_from_slice(&matrix::CHAR_PAIRS[(q % TWO) as usize]);
    }
    d
}

#[cfg(kani)]
mod verification {
    use super::max_encoded_len;

    #[kani::proof]
    fn encoded_capacity_never_panics() {
        let len: usize = kani::any();
        let _ = max_encoded_len(len);
    }

    #[kani::proof]
    fn encoded_capacity_is_monotone_and_saturating() {
        let len: usize = kani::any();
        let bound = max_encoded_len(len);
        assert!(bound >= len);
        if let Some(next) = len.checked_add(1) {
            assert!(max_encoded_len(next) >= bound);
        }
        assert_eq!(max_encoded_len(usize::MAX), usize::MAX);
    }

    // These conjuncts have exactly the domain and claims of the monolithic
    // capacity harness above. Splitting them gives each its own solver query.
    #[kani::proof]
    fn encoded_capacity_covers_input() {
        let len: usize = kani::any();
        assert!(max_encoded_len(len) >= len);
    }

    #[kani::proof]
    fn encoded_capacity_adjacent_monotonicity() {
        let len: usize = kani::any();
        kani::assume(len < usize::MAX);
        assert!(max_encoded_len(len + 1) >= max_encoded_len(len));
    }

    #[kani::proof]
    fn encoded_capacity_endpoint_saturates() {
        assert_eq!(max_encoded_len(usize::MAX), usize::MAX);
    }

    // A mathematical lemma, proved for every usize before it is used as a
    // verified stub. It does not replace max_encoded_len or its multiplication.
    #[kani::ensures(|_: &()|
        (len as u128 + 1) * (super::LOG58_2 + 1)
            == len as u128 * (super::LOG58_2 + 1) + (super::LOG58_2 + 1))]
    #[kani::ensures(|_: &()| len == usize::MAX
        || (len + 1) as u128 * (super::LOG58_2 + 1)
            == len as u128 * (super::LOG58_2 + 1) + (super::LOG58_2 + 1))]
    fn capacity_affine_step(len: usize) {
        let _ = len;
    }

    #[kani::proof_for_contract(capacity_affine_step)]
    fn capacity_affine_step_lemma() {
        capacity_affine_step(kani::any());
    }

    fn capacity_clamp(product: u128) -> usize {
        let bound = (product >> 61) + 1;
        if bound > usize::MAX as u128 {
            usize::MAX
        } else {
            bound as usize
        }
    }

    #[kani::ensures(|result: &usize|
        *result == capacity_clamp(len as u128 * (super::LOG58_2 + 1)))]
    fn capacity_formula(len: usize) -> usize {
        max_encoded_len(len)
    }

    #[kani::proof_for_contract(capacity_formula)]
    fn capacity_formula_lemma() {
        let _ = capacity_formula(kani::any());
    }

    #[kani::requires(product <= u128::MAX - (super::LOG58_2 + 1))]
    #[kani::ensures(|_: &()|
        capacity_clamp(product + (super::LOG58_2 + 1)) >= capacity_clamp(product))]
    fn capacity_shift_clamp_step(product: u128) {
        let _ = product;
    }

    #[kani::proof_for_contract(capacity_shift_clamp_step)]
    fn capacity_shift_clamp_step_lemma() {
        capacity_shift_clamp_step(kani::any());
    }

    #[kani::proof]
    #[kani::stub_verified(capacity_affine_step)]
    #[kani::stub_verified(capacity_shift_clamp_step)]
    #[kani::stub_verified(capacity_formula)]
    fn encoded_capacity_adjacent_monotonicity_composed() {
        let len: usize = kani::any();
        kani::assume(len < usize::MAX);
        capacity_affine_step(len);
        capacity_shift_clamp_step(len as u128 * (super::LOG58_2 + 1));
        assert!(capacity_formula(len + 1) >= capacity_formula(len));
    }

    #[kani::requires(product == len as u128 * (super::LOG58_2 + 1))]
    #[kani::ensures(|result: &usize| *result == capacity_clamp(product))]
    fn capacity_at_product(len: usize, product: u128) -> usize {
        let _ = product;
        max_encoded_len(len)
    }

    #[kani::proof_for_contract(capacity_at_product)]
    fn capacity_at_product_lemma() {
        let _ = capacity_at_product(kani::any(), kani::any());
    }

    #[kani::proof]
    #[kani::stub_verified(capacity_affine_step)]
    #[kani::stub_verified(capacity_shift_clamp_step)]
    #[kani::stub_verified(capacity_at_product)]
    fn encoded_capacity_adjacent_monotonicity_at_products() {
        let len: usize = kani::any();
        kani::assume(len < usize::MAX);
        let product = len as u128 * (super::LOG58_2 + 1);
        capacity_affine_step(len);
        capacity_shift_clamp_step(product);
        let here = capacity_at_product(len, product);
        let next = capacity_at_product(len + 1, product + (super::LOG58_2 + 1));
        assert!(next >= here);
    }

    #[kani::proof]
    #[kani::unwind(12)]
    fn one_byte_borrowed_api_preserves_exact_array_contracts() {
        let byte: u8 = kani::any();
        let capacity: usize = kani::any();
        kani::assume(capacity <= 3);
        let mut output = [0xA5; 5];
        let result = crate::encode_into([byte], &mut output[1..1 + capacity]);
        let digits = if byte < 58 { 1 } else { 2 };
        if capacity < digits {
            assert!(matches!(result, Err(crate::BufferTooSmall)));
        } else {
            let text = result.unwrap();
            assert_eq!(text.len(), digits);
            assert_eq!(
                text.as_bytes()[digits - 1],
                crate::common::ALPHABET[(byte % 58) as usize]
            );
            if digits == 2 {
                assert_eq!(
                    text.as_bytes()[0],
                    crate::common::ALPHABET[(byte / 58) as usize]
                );
            }
            assert_eq!(crate::decode_array::<1>(text).unwrap()[0], byte);
            assert!(matches!(
                crate::decode_array::<0>(text),
                Err(crate::Error::BufferTooSmall)
            ));
            assert!(matches!(
                crate::decode_array::<2>(text),
                Err(crate::Error::InvalidLength {
                    expected: 2,
                    actual: 1
                })
            ));
        }
        assert_eq!(output[0], 0xA5);
        assert!(output[1 + capacity..].iter().all(|&c| c == 0xA5));
        if capacity >= digits {
            assert!(output[1 + digits..].iter().all(|&c| c == 0xA5));
        }
    }
}

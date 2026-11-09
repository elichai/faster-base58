//! Base58 by malachite-nz's subquadratic radix conversion, GMP's `mpn_get_str` and
//! `mpn_set_str` algorithms. Nothing from this crate is called.

use malachite_base::num::conversion::traits::{Digits, PowerOf2Digits};
use malachite_nz::natural::Natural;

use super::scalar::ALPHABET;

pub fn encode(data: &[u8]) -> Vec<u8> {
    let zeros = data.iter().take_while(|&&byte| byte == 0).count();
    let value = Natural::from_power_of_2_digits_desc(8, data[zeros..].iter().copied()).unwrap();
    let digits: Vec<u8> = value.to_digits_desc(&58);
    let mut text = vec![b'1'; zeros];
    text.extend(digits.iter().map(|&digit| ALPHABET[digit as usize]));
    text
}

/// The decoded bytes or the first invalid byte offset.
pub fn decode(text: &[u8]) -> Result<Vec<u8>, usize> {
    let digits = text
        .iter()
        .enumerate()
        .map(|(index, byte)| ALPHABET.iter().position(|c| c == byte).ok_or(index))
        .collect::<Result<Vec<_>, _>>()?;
    let zeros = digits.iter().take_while(|&&digit| digit == 0).count();
    let value = Natural::from_digits_desc(&58, digits[zeros..].iter().map(|&d| d as u8)).unwrap();
    let mut data = vec![0; zeros];
    data.extend(PowerOf2Digits::<u8>::to_power_of_2_digits_desc(&value, 8));
    Ok(data)
}

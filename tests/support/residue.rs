//! Linear-time check that base58 text encodes given bytes. It compares the zero prefixes and
//! the values modulo fixed primes, so it calls no converter, bigint library or code from
//! this crate.
//!
//! The primes are 61-bit safe primes `p = 2q + 1` searched from hashed starting points,
//! unrelated to the NTT moduli. Modulo each, 2, 58 and 256 have order at least `q`. A wrong
//! value passes only if its error is a multiple of all three primes, a product above 2^182.
//! An error `c * b^i * (b^t - 1)` with `0 < |c| < p` and `0 < t < q` never is.

use super::scalar::ALPHABET;

const PRIMES: [u64; 3] = [
    0x1E6C_D9F1_903F_19EF,
    0x16C2_C6BB_EB0C_661B,
    0x1F0D_285C_2DB9_D8AF,
];

/// Whether `text` is the canonical base58 encoding of `data`, up to residue collisions.
pub fn matches(data: &[u8], text: &[u8]) -> bool {
    let zeros = data.iter().take_while(|&&byte| byte == 0).count();
    let ones = text.iter().take_while(|&&c| c == b'1').count();
    // Both remainders start with a nonzero digit, so equal values mean equal encodings.
    let value = data[zeros..]
        .iter()
        .fold([0; 3], |r, &byte| step(r, 256, byte.into()));
    let digits = text[ones..].iter().try_fold([0; 3], |r, c| {
        let digit = ALPHABET.iter().position(|a| a == c)?;
        Some(step(r, 58, digit as u64))
    });
    zeros == ones && digits == Some(value)
}

fn step(residues: [u64; 3], base: u64, digit: u64) -> [u64; 3] {
    std::array::from_fn(|i| {
        let p = u128::from(PRIMES[i]);
        ((u128::from(residues[i]) * u128::from(base) + u128::from(digit)) % p) as u64
    })
}

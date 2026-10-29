use malachite_base::num::conversion::traits::{Digits, PowerOf2Digits};
use malachite_nz::natural::Natural;
use std::path::PathBuf;

fn data(size: usize) -> Vec<u8> {
    let mut result = vec![0; size];
    let mut state = size as u64 + 20_261_023;
    for byte in &mut result {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = state as u8;
    }
    if let Some(first) = result.first_mut() { *first |= 1; }
    result
}

fn main() {
    let mut args = std::env::args().skip(1);
    let destination = PathBuf::from(args.next().expect("fresh output directory"));
    std::fs::create_dir(&destination).unwrap();
    for argument in args {
        let size: usize = argument.parse().unwrap();
        assert!(size <= 4_194_304);
        let input = data(size);
        let zeros = input.iter().take_while(|&&byte| byte == 0).count();
        let value = Natural::from_power_of_2_digits_desc(8, input[zeros..].iter().copied()).unwrap();
        let digits: Vec<u8> = value.to_digits_desc(&58);
        let alphabet = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        let encoded: Vec<u8> = std::iter::repeat_n(b'1', zeros)
            .chain(digits.into_iter().map(|digit| alphabet[digit as usize])).collect();
        std::fs::write(destination.join(format!("{size}.bin")), &input).unwrap();
        std::fs::write(destination.join(format!("{size}.txt")), &encoded).unwrap();
        println!("{size},{}", encoded.len());
    }
}

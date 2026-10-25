#![allow(dead_code)]

pub mod malachite;

use base58_bench::{BufferTooSmall, Error};

pub const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
pub type Encode = fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall>;
pub type Decode = fn(&str, &mut [u8]) -> Result<usize, Error>;

pub const ENCODERS: &[(&str, Encode)] = &[
    ("auto", base58_bench::encode),
    ("no-alloc", base58_bench::no_alloc::encode),
    ("batched", base58_bench::internals::encode_batched),
    ("dc", base58_bench::internals::encode_karatsuba),
    ("toom3", base58_bench::internals::encode_toom3),
    ("ntt", base58_bench::internals::encode_ntt),
    ("ntt-cached", base58_bench::internals::encode_ntt_cached),
    ("ntt-portable", base58_bench::internals::encode_ntt_portable),
    ("malachite", malachite::encode_malachite),
    ("matrix", base58_bench::internals::encode_matrix),
    ("matrix-portable", base58_bench::internals::encode_matrix_portable),
];

pub const DECODERS: &[(&str, Decode)] = &[
    ("auto", base58_bench::decode),
    ("no-alloc", base58_bench::no_alloc::decode),
    ("batched", base58_bench::internals::decode_batched),
    ("dc", base58_bench::internals::decode_karatsuba),
    ("toom3", base58_bench::internals::decode_toom3),
    ("ntt", base58_bench::internals::decode_ntt),
    ("ntt-cached", base58_bench::internals::decode_ntt_cached),
    ("ntt-portable", base58_bench::internals::decode_ntt_portable),
    ("malachite", malachite::decode_malachite),
    ("matrix", base58_bench::internals::decode_matrix),
    ("matrix-portable", base58_bench::internals::decode_matrix_portable),
];

const GUARD: usize = 16;
const CANARY: u8 = 0xA5;

pub fn check_encoder(name: &str, encode: Encode, data: &[u8], want: &[u8], selector: usize) {
    let n = want.len();
    for capacity in [0, n.saturating_sub(1), n, n + 17, selector % (n + 18)] {
        let mut storage = vec![CANARY; GUARD + capacity + GUARD];
        let result = encode(data, &mut storage[GUARD..GUARD + capacity]);
        assert!(
            storage[..GUARD].iter().all(|&x| x == CANARY),
            "{name}: prefix canary"
        );
        assert!(
            storage[GUARD + capacity..].iter().all(|&x| x == CANARY),
            "{name}: suffix canary"
        );
        if capacity < n {
            assert_eq!(
                result,
                Err(BufferTooSmall),
                "{name}: {} bytes into {capacity}",
                data.len()
            );
        } else {
            assert_eq!(
                result,
                Ok(n),
                "{name}: {} bytes into {capacity}",
                data.len()
            );
            assert_eq!(&storage[GUARD..GUARD + n], want, "{name}: output");
            assert!(
                storage[GUARD + n..].iter().all(|&x| x == CANARY),
                "{name}: wrote past result"
            );
        }
    }
}

pub fn check_encode(data: &[u8], selector: usize) {
    let want = bs58::encode(data).into_vec();
    assert!(want.len() <= base58_bench::max_encoded_len(data.len()));
    for &(name, encode) in ENCODERS {
        check_encoder(name, encode, data, &want, selector);
    }
}

pub fn oracle_decode(input: &str) -> Result<Vec<u8>, usize> {
    bs58::decode(input.as_bytes())
        .into_vec()
        .map_err(|error| match error {
            bs58::decode::Error::InvalidCharacter { index, .. }
            | bs58::decode::Error::NonAsciiCharacter { index } => index,
            error => panic!("unexpected bs58 error: {error:?}"),
        })
}

pub fn check_decoder(
    name: &str,
    decode: Decode,
    input: &str,
    want: &Result<Vec<u8>, usize>,
    selector: usize,
) {
    let n = want.as_ref().map_or(input.len(), Vec::len);
    for capacity in [0, n.saturating_sub(1), n, n + 17, selector % (n + 18)] {
        let mut storage = vec![CANARY; GUARD + capacity + GUARD];
        let result = decode(input, &mut storage[GUARD..GUARD + capacity]);
        assert!(
            storage[..GUARD].iter().all(|&x| x == CANARY),
            "{name}: prefix canary"
        );
        assert!(
            storage[GUARD + capacity..].iter().all(|&x| x == CANARY),
            "{name}: suffix canary"
        );
        match want {
            Ok(bytes) if capacity >= n => {
                assert_eq!(
                    result,
                    Ok(n),
                    "{name}: {} characters into {capacity}",
                    input.len()
                );
                assert_eq!(&storage[GUARD..GUARD + n], bytes, "{name}: output");
                assert!(
                    storage[GUARD + n..].iter().all(|&x| x == CANARY),
                    "{name}: wrote past result"
                );
            }
            Ok(_) => assert_eq!(result, Err(Error::BufferTooSmall), "{name}: short buffer"),
            Err(index) => {
                let index = *index;
                let invalid = Err(Error::InvalidCharacter { index });
                // Error precedence is unspecified when the buffer is also too small.
                assert!(
                    result == invalid
                        || (capacity < input.len() && result == Err(Error::BufferTooSmall)),
                    "{name}: {result:?}, expected invalid byte {index}, capacity {capacity}"
                );
            }
        }
    }
}

pub fn check_decode(input: &str, selector: usize) {
    let want = oracle_decode(input);
    for &(name, decode) in DECODERS {
        check_decoder(name, decode, input, &want, selector);
    }
}

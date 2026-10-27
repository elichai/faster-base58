#![allow(dead_code)]

pub mod malachite;
pub mod scalar;

use faster_base58::{BufferTooSmall, Error};

pub const ALPHABET: &[u8; 58] = scalar::ALPHABET;
pub type Encode = fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall>;
pub type Decode = fn(&str, &mut [u8]) -> Result<usize, Error>;

pub const ENCODERS: &[(&str, Encode)] = &[
    ("auto", faster_base58::encode),
    ("no-alloc", faster_base58::no_alloc::encode),
    ("batched", faster_base58::internals::encode_batched),
    ("dc", faster_base58::internals::encode_karatsuba),
    ("toom3", faster_base58::internals::encode_toom3),
    ("ntt", faster_base58::internals::encode_ntt),
    ("ntt-cached", faster_base58::internals::encode_ntt_cached),
    (
        "ntt-portable",
        faster_base58::internals::encode_ntt_portable,
    ),
    ("malachite", malachite::encode_malachite),
    ("matrix", faster_base58::internals::encode_matrix),
    (
        "matrix-portable",
        faster_base58::internals::encode_matrix_portable,
    ),
];

pub const DECODERS: &[(&str, Decode)] = &[
    ("auto", faster_base58::decode),
    ("no-alloc", faster_base58::no_alloc::decode),
    ("batched", faster_base58::internals::decode_batched),
    ("dc", faster_base58::internals::decode_karatsuba),
    ("toom3", faster_base58::internals::decode_toom3),
    ("ntt", faster_base58::internals::decode_ntt),
    ("ntt-cached", faster_base58::internals::decode_ntt_cached),
    (
        "ntt-portable",
        faster_base58::internals::decode_ntt_portable,
    ),
    ("malachite", malachite::decode_malachite),
    ("matrix", faster_base58::internals::decode_matrix),
    (
        "matrix-portable",
        faster_base58::internals::decode_matrix_portable,
    ),
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
    if let Some(scalar) = scalar::encode(data) {
        assert_eq!(want, scalar, "independent u128 encode oracle");
    }
    assert!(want.len() <= faster_base58::max_encoded_len(data.len()));
    assert_eq!(faster_base58::encode_string(data).as_bytes(), want);
    for &(name, encode) in ENCODERS {
        check_encoder(name, encode, data, &want, selector);
    }
    // Matrix comparison hooks no longer select fixed-size kernels.
    let fixed: Option<Encode> = match data.len() {
        32 => Some(|data, out| {
            faster_base58::internals::encode_32_portable(data.try_into().unwrap(), out)
        }),
        64 => Some(|data, out| {
            faster_base58::internals::encode_64_portable(data.try_into().unwrap(), out)
        }),
        _ => None,
    };
    if let Some(encode) = fixed {
        check_encoder("fixed-portable", encode, data, &want, selector);
    }
}

pub fn oracle_decode(input: &str) -> Result<Vec<u8>, usize> {
    let result = bs58::decode(input.as_bytes())
        .into_vec()
        .map_err(|error| match error {
            bs58::decode::Error::InvalidCharacter { index, .. }
            | bs58::decode::Error::NonAsciiCharacter { index } => index,
            error => panic!("unexpected bs58 error: {error:?}"),
        });
    if let Some(scalar) = scalar::decode(input) {
        assert_eq!(
            result, scalar,
            "independent u128 decode/first-invalid oracle"
        );
    }
    result
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
    assert_eq!(
        faster_base58::decode_vec(input),
        want.map_err(|index| Error::InvalidCharacter { index })
    );
}

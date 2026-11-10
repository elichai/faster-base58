#![allow(
    dead_code,
    reason = "Each test and benchmark uses a subset of these shared helpers"
)]

pub mod malachite;
pub mod reference;
pub mod residue;
pub mod scalar;

use faster_base58::{BufferTooSmall, Error};

pub const ALPHABET: &[u8; 58] = scalar::ALPHABET;
pub type Encode = fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall>;
pub type Decode = fn(&str, &mut [u8]) -> Result<usize, Error>;

pub const ENCODERS: &[(&str, Encode)] = &[
    (
        "auto",
        (|input, output| {
            faster_base58::Encoder::bitcoin()
                .encode(input)
                .onto(output)
                .map(str::len)
        }),
    ),
    ("no-alloc", faster_base58::internals::encode_no_alloc),
    ("batched", faster_base58::internals::encode_batched),
    ("dc", faster_base58::internals::encode_karatsuba),
    ("ntt", faster_base58::internals::encode_ntt),
    ("ntt-cached", faster_base58::internals::encode_ntt_cached),
    ("radix", faster_base58::internals::encode_radix),
    (
        "radix-portable",
        faster_base58::internals::encode_radix_portable,
    ),
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
    (
        "auto",
        (|input, output| {
            faster_base58::Decoder::bitcoin()
                .decode(input)
                .onto(output)
                .map(|bytes| bytes.len())
        }),
    ),
    ("no-alloc", faster_base58::internals::decode_no_alloc),
    ("batched", faster_base58::internals::decode_batched),
    ("dc", faster_base58::internals::decode_karatsuba),
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

/// Output capacities for a result of `n` bytes or characters: empty, one short, exact, with
/// slack, and one picked by `selector`. Large backends reject capacities below about
/// `n - 2` before converting, so one short, exact and slack each cost a whole conversion.
/// Above [`QUADRATIC_MAX_LEN`], exact and slack alternate by input.
fn capacities(n: usize, selector: usize) -> impl Iterator<Item = usize> {
    let skip = match (n > QUADRATIC_MAX_LEN, selector & 1) {
        (false, _) => None,
        (true, 0) => Some(n + 17),
        (true, _) => Some(n),
    };
    [0, n.saturating_sub(1), n, n + 17, selector % (n + 18)]
        .into_iter()
        .filter(move |&capacity| Some(capacity) != skip)
}

pub fn check_encoder(name: &str, encode: Encode, data: &[u8], want: &[u8], selector: usize) {
    let n = want.len();
    for capacity in capacities(n, selector) {
        check_encoder_capacity(name, encode, data, want, capacity);
    }
}

pub fn check_encoder_capacity(
    name: &str,
    encode: Encode,
    data: &[u8],
    want: &[u8],
    capacity: usize,
) {
    let n = want.len();
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

/// Longest input, in bytes or characters, for the quadratic `batched` and `no-alloc`
/// backends (`no-alloc` calls `batched` at these sizes). Their carry bounds hold per limb
/// whatever the length; `large_inputs_round_trip` and `large_inputs_match_batched` check
/// them at 200-300 K. Above this they took about half of the full-width replay time.
pub const QUADRATIC_MAX_LEN: usize = 8 * 1024 + 1;

fn skip_quadratic(name: &str, len: usize) -> bool {
    len > QUADRATIC_MAX_LEN && matches!(name, "batched" | "no-alloc")
}

/// Longest input for the bs58 oracle, whose quadratic conversion takes seconds at 64 KiB.
/// Longer inputs use malachite-nz's subquadratic conversion; `properties` checks that the two
/// agree across this boundary.
pub const BS58_MAX_LEN: usize = 2048;

/// The expected encoding of `data`, from bs58 or [`reference`] by length, confirmed by
/// [`residue`].
pub fn oracle_encode(data: &[u8]) -> Vec<u8> {
    let want = if data.len() > BS58_MAX_LEN {
        reference::encode(data)
    } else {
        bs58::encode(data).into_vec()
    };
    if let Some(scalar) = scalar::encode(data) {
        assert_eq!(want, scalar, "independent u128 encode oracle");
    }
    assert!(residue::matches(data, &want), "residue encode oracle");
    want
}

/// Checks every encoder against [`oracle_encode`] and returns the expected text.
pub fn check_encode(data: &[u8], selector: usize) -> String {
    let want = oracle_encode(data);
    assert!(want.len() <= faster_base58::Encoder::bitcoin().max_encoded_len(data.len()));
    assert_eq!(
        faster_base58::Encoder::bitcoin()
            .encode(data)
            .into_string()
            .as_bytes(),
        want
    );
    for &(name, encode) in ENCODERS {
        if !skip_quadratic(name, data.len()) {
            check_encoder(name, encode, data, &want, selector);
        }
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
    String::from_utf8(want).unwrap()
}

/// Checks every encoder on `data`, then every decoder on the expected text. The expected
/// decoding is `data` itself, so no decode oracle runs.
pub fn check_round_trip(data: &[u8], selector: usize) {
    let text = check_encode(data, selector);
    check_decode_as(&text, &Ok(data.to_vec()), selector);
}

/// The decoded bytes or the first invalid byte offset, from bs58 or [`reference`] by length.
/// Decoded bytes are confirmed by [`residue`].
pub fn oracle_decode_bytes(input: &[u8]) -> Result<Vec<u8>, usize> {
    let want = if input.len() > BS58_MAX_LEN {
        reference::decode(input)
    } else {
        bs58_decode(input)
    };
    if let Ok(data) = &want {
        assert!(residue::matches(data, input), "residue decode oracle");
    }
    want
}

/// bs58's result for raw bytes: the decoded bytes or the first invalid byte offset.
pub fn bs58_decode(input: &[u8]) -> Result<Vec<u8>, usize> {
    bs58::decode(input).into_vec().map_err(|error| match error {
        bs58::decode::Error::InvalidCharacter { index, .. }
        | bs58::decode::Error::NonAsciiCharacter { index } => index,
        error => panic!("unexpected bs58 error: {error:?}"),
    })
}

pub fn oracle_decode(input: &str) -> Result<Vec<u8>, usize> {
    let result = oracle_decode_bytes(input.as_bytes());
    if let Some(scalar) = scalar::decode(input) {
        assert_eq!(
            result, scalar,
            "independent u128 decode/first-invalid oracle"
        );
    }
    result
}

pub fn check_decoder<T: AsRef<[u8]> + ?Sized>(
    name: &str,
    decode: impl Fn(&T, &mut [u8]) -> Result<usize, Error>,
    input: &T,
    want: &Result<Vec<u8>, usize>,
    selector: usize,
) {
    let len = input.as_ref().len();
    let n = want.as_ref().map_or(len, Vec::len);
    for capacity in capacities(n, selector) {
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
                assert_eq!(result, Ok(n), "{name}: {len} characters into {capacity}");
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
                    result == invalid || (capacity < len && result == Err(Error::BufferTooSmall)),
                    "{name}: {result:?}, expected invalid byte {index}, capacity {capacity}"
                );
            }
        }
    }
}

pub fn check_decode(input: &str, selector: usize) {
    check_decode_as(input, &oracle_decode(input), selector);
}

/// The encoders whose division tree writes zero blocks for runs of the digit `1`.
const TREE_ENCODERS: [&str; 4] = ["auto", "dc", "ntt", "ntt-cached"];

/// [`check_decode`], then, for valid text, the division-tree encoders on the decoded bytes,
/// which must give `input` back: each byte string has exactly one encoding. Text reaches
/// encoder inputs that random bytes almost never do, such as long runs of the zero digit.
pub fn check_decode_round_trip(input: &str, selector: usize) {
    let want = oracle_decode(input);
    check_decode_as(input, &want, selector);
    let Ok(data) = want else { return };
    // Exact output slices only: the encode targets check the others, and each costs a
    // whole conversion. `decode_all` replays of the stored corpus (M5, ASan, 2026-11-10)
    // took 1.27 times as long with these encoders on its mapped text; every encoder at every
    // capacity on both of its texts took 2.7 times, and every encoder on the mapped text 2.2.
    for &(name, encode) in ENCODERS {
        if TREE_ENCODERS.contains(&name) {
            check_encoder_capacity(name, encode, &data, input.as_bytes(), input.len());
        }
    }
}

fn check_decode_as(input: &str, want: &Result<Vec<u8>, usize>, selector: usize) {
    for &(name, decode) in DECODERS {
        if !skip_quadratic(name, input.len()) {
            check_decoder(name, decode, input, want, selector);
        }
    }
    assert_eq!(
        faster_base58::Decoder::bitcoin().decode(input).into_vec(),
        want.clone()
            .map_err(|index| Error::InvalidCharacter { index })
    );
}

/// The public decoder on raw bytes, which need not be UTF-8, against bs58.
pub fn check_decode_bytes(input: &[u8], selector: usize) {
    let want = oracle_decode_bytes(input);
    let public = |input: &[u8], output: &mut [u8]| {
        faster_base58::Decoder::bitcoin()
            .decode(input)
            .onto(output)
            .map(|bytes| bytes.len())
    };
    check_decoder("public-bytes", public, input, &want, selector);
    assert_eq!(
        faster_base58::Decoder::bitcoin().decode(input).into_vec(),
        want.map_err(|index| Error::InvalidCharacter { index })
    );
}

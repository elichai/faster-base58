//! Checks every decoder against `bs58`.

use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

use crate::common::{ALPHABET, CHUNK, DIGITS, all_valid};
use crate::dc::DC_THRESHOLD;
use crate::{
    Error, decode, decode_batched, decode_malachite, decode_matrix, decode_matrix_portable,
    decode_ntt, decode_ntt_cached, decode_ntt_portable, decode_toom3,
};

pub(crate) type Decode = fn(&str, &mut [u8]) -> Result<usize, Error>;

/// Every decoder under test. Add new ones here.
const DECODERS: &[(&str, Decode)] = &[
    ("batched", decode_batched),
    ("dc", decode),
    ("dc-toom3", decode_toom3),
    ("ntt", decode_ntt),
    ("ntt-cached", decode_ntt_cached),
    ("ntt-portable", decode_ntt_portable),
    ("dc-malachite", decode_malachite),
    ("matrix", decode_matrix),
    ("matrix-portable", decode_matrix_portable),
];

/// Decodes into an ample buffer and checks nothing past the result was written.
pub(crate) fn decode_vec(f: Decode, s: &str) -> Result<Vec<u8>, Error> {
    let mut out = vec![0xAA; s.len()];
    let n = f(s, &mut out)?;
    assert!(out[n..].iter().all(|&b| b == 0xAA), "wrote past the result");
    out.truncate(n);
    Ok(out)
}

pub(crate) fn random_string(rng: &mut Xoshiro256PlusPlus, len: usize) -> String {
    (0..len).map(|_| ALPHABET[(rng.next_u32() % 58) as usize] as char).collect()
}

/// Checks every decoder against bs58 on `s`: the output, or the index of the first
/// invalid character; then exactly sized and one byte short output buffers.
/// Returns bs58's output.
fn check(s: &str) -> Result<Vec<u8>, usize> {
    let want = bs58::decode(s).into_vec().map_err(|e| match e {
        bs58::decode::Error::InvalidCharacter { index, .. } => index,
        bs58::decode::Error::NonAsciiCharacter { index } => index,
        e => panic!("bs58: {e:?}"),
    });
    for &(name, f) in DECODERS {
        let got = decode_vec(f, s).map_err(|e| match e {
            Error::InvalidCharacter { index } => index,
            e => panic!("{name}: {e:?}"),
        });
        assert_eq!(got, want, "{name} on {} chars", s.len());
        if let Ok(want) = &want {
            let mut exact = vec![0xAA; want.len()];
            assert_eq!(f(s, &mut exact), Ok(want.len()), "{name}: exact buffer");
            assert_eq!(&exact, want, "{name}: exact buffer");
            if let Some(short) = exact.get_mut(1..) {
                assert_eq!(f(s, short), Err(Error::BufferTooSmall), "{name}: short buffer");
            }
        }
    }
    want
}

/// bs58's encodings of several byte patterns, across all small lengths, the
/// divide-and-conquer threshold and the benchmark sizes.
#[test]
fn matches_bs58() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(2);
    let random_bytes = |rng: &mut Xoshiro256PlusPlus, len| {
        let mut v = vec![0u8; len];
        rng.fill_bytes(&mut v);
        v
    };
    // Bytes that encode to about `DC_THRESHOLD` chunks: log(58) / log(256) ≈ 0.732.
    let t = DC_THRESHOLD * CHUNK * 732 / 1000;
    for len in (0..=300).chain(t - 15..=t + 15).chain([1024, 4096, 8192]) {
        let random = random_bytes(&mut rng, len);
        let mut leading_zeros = random.clone();
        leading_zeros[..len.min(3)].fill(0);
        for data in [random, leading_zeros, vec![0xFF; len], vec![0; len]] {
            let s = bs58::encode(&data).into_string();
            assert_eq!(check(&s), Ok(data), "{len} bytes");
        }
    }
    // bs58 is quadratic, so the benchmark sizes get one random input each,
    // plus 58^n - 1, which maximises carries.
    for len in [32 * 1024, 32 * 1024 + 1] {
        let data = random_bytes(&mut rng, len);
        assert_eq!(check(&bs58::encode(&data).into_string()), Ok(data), "{len} bytes");
    }
    assert!(check(&"z".repeat(44_761)).is_ok());
}

/// The range checks of `all_valid` against the alphabet table, for every byte.
#[test]
fn all_valid_matches_digits() {
    for c in 0..=u8::MAX {
        assert_eq!(all_valid(&[c]), DIGITS[c as usize] != 0xFF, "{c:#04x}");
    }
    assert!(all_valid(ALPHABET) && all_valid(b""));
}

/// `2^(64 k)`, a one followed by `k` zero limbs, on both decoding paths.
#[test]
fn powers_of_two_64() {
    for k in [1, 2, 63, 64, 65, 300, 468, 469, 470, 600, 1000] {
        let mut data = vec![0u8; 8 * k + 1];
        data[0] = 1;
        assert_eq!(check(&bs58::encode(&data).into_string()), Ok(data), "2^(64 * {k})");
    }
}

/// Inputs too large for bs58, against `decode_batched`, which shares no code with the
/// divide-and-conquer decoders beyond parsing. A long inner run of '1's gives nodes
/// whose high part is zero.
#[test]
fn large_inputs_match_batched() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(7);
    let len = 300_001;
    let random = random_string(&mut rng, len);
    let zero_run = format!("{}{}{}", &random[..1000], "1".repeat(100_000), &random[1000..]);
    for s in [random, "z".repeat(len), zero_run, format!("111{}", "z".repeat(len))] {
        let want = decode_vec(decode_batched, &s);
        for &(name, f) in DECODERS {
            assert!(decode_vec(f, &s) == want, "{name}, {} chars", s.len());
        }
    }
}

/// Random strings, valid or with one invalid character, into buffers of random and
/// boundary sizes. A valid string decodes exactly when the buffer holds the result. An
/// invalid one reports its first invalid character, or `BufferTooSmall` when the buffer
/// is also too small: which of the two is unspecified.
#[test]
fn random_buffer_sizes() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(6);
    let t = DC_THRESHOLD * CHUNK;
    for round in 0..300 {
        let len = rng.next_u32() as usize % if round % 3 == 0 { 2 * t } else { 1500 };
        let mut s = random_string(&mut rng, len);
        if round % 4 == 0 {
            s.insert_str(0, "111");
        }
        let bad = (round % 5 == 0).then(|| rng.next_u32() as usize % (s.len() + 1));
        if let Some(at) = bad {
            s.insert(at, '0');
        }
        let want = bs58::decode(&s).into_vec().ok();
        let n = want.as_ref().map_or(s.len(), Vec::len);
        let sizes = [0, n.saturating_sub(1), n, n + 1, rng.next_u32() as usize % (n + 2)];
        for &(name, f) in DECODERS {
            for size in sizes {
                let mut out = vec![0xAA; size];
                let got = f(&s, &mut out);
                match (&want, bad) {
                    (Some(want), _) if size >= want.len() => {
                        assert_eq!(got, Ok(want.len()), "{name}, {} into {size}", s.len());
                        assert_eq!(out[..want.len()], want[..], "{name}");
                        assert!(out[want.len()..].iter().all(|&b| b == 0xAA), "{name}");
                    }
                    (Some(_), _) => assert_eq!(got, Err(Error::BufferTooSmall), "{name}"),
                    (None, Some(index)) => {
                        let invalid = Err(Error::InvalidCharacter { index });
                        let ok = got == invalid || got == Err(Error::BufferTooSmall) && size < n;
                        assert!(ok, "{name}: {got:?} for {} chars into {size}", s.len());
                    }
                    (None, None) => unreachable!("bs58 rejected a valid string"),
                }
            }
        }
    }
}

/// Invalid characters at several positions, on both decoding paths.
#[test]
fn errors_match_bs58() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(4);
    let t = DC_THRESHOLD * CHUNK;
    for len in [0, 1, 9, 10, 11, 100, t + 1, 2 * t] {
        // Leading '1's take a separate path.
        let valid = format!("11{}", random_string(&mut rng, len));
        for bad in ['0', 'O', 'I', 'l', '+', ' ', '\0', '\x7f', 'é'] {
            for pos in [0, 1, 2, valid.len() / 2, valid.len()] {
                let s = format!("{}{bad}{}", &valid[..pos], &valid[pos..]);
                assert_eq!(check(&s), Err(pos), "{s:?}");
            }
        }
    }
}

#[test]
fn edge_cases() {
    for &(_, f) in DECODERS {
        assert_eq!(f("", &mut []), Ok(0));
        assert_eq!(decode_vec(f, "111"), Ok(vec![0, 0, 0]));
        assert_eq!(decode_vec(f, "1112"), Ok(vec![0, 0, 0, 1]));
        assert_eq!(decode_vec(f, "5Q"), Ok(vec![0xFF]));
        assert_eq!(f("5Q", &mut []), Err(Error::BufferTooSmall));
        assert_eq!(f("15Q", &mut [0; 1]), Err(Error::BufferTooSmall));
        assert_eq!(f("15Q", &mut [0; 2]), Ok(2));
    }
}

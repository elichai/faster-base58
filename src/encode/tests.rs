//! Checks every encoder against `bs58`, and large inputs against round trips through
//! `decode` and against malachite-nz's conversion.

use malachite_base::num::conversion::traits::{Digits, PowerOf2Digits};
use malachite_nz::natural::Natural;
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

use super::dc::karatsuba::DC_THRESHOLD;
use super::{max_encoded_len, min_digits};
use crate::common::ALPHABET;
use crate::internals::{
    encode_batched, encode_karatsuba, encode_matrix, encode_matrix_portable, encode_ntt,
    encode_ntt_cached, encode_ntt_portable, encode_radix, encode_radix_portable, encode_toom3,
};
use crate::{BufferTooSmall, decode, encode, encode_malachite};

pub(crate) type Encode = fn(&[u8], &mut [u8]) -> Result<usize, BufferTooSmall>;

/// Every encoder under test. Add new ones here.
const ENCODERS: &[(&str, Encode)] = &[
    ("batched", encode_batched),
    ("auto", encode),
    ("dc", encode_karatsuba),
    ("dc-toom3", encode_toom3),
    ("ntt", encode_ntt),
    ("ntt-cached", encode_ntt_cached),
    ("ntt-portable", encode_ntt_portable),
    ("radix", encode_radix),
    ("radix-portable", encode_radix_portable),
    ("dc-malachite", encode_malachite),
    ("matrix", encode_matrix),
    ("matrix-portable", encode_matrix_portable),
];

/// Encodes into an ample buffer and checks nothing past the result was written.
pub(crate) fn encode_vec(f: Encode, data: &[u8]) -> Vec<u8> {
    let mut out = vec![0xAA; max_encoded_len(data.len())];
    let n = f(data, &mut out).expect("max_encoded_len is enough");
    assert!(out[n..].iter().all(|&b| b == 0xAA), "wrote past the result");
    out.truncate(n);
    out
}

pub(crate) fn random_bytes(rng: &mut Xoshiro256PlusPlus, len: usize) -> Vec<u8> {
    let mut v = vec![0; len];
    rng.fill_bytes(&mut v);
    v
}

#[test]
fn ten_digits_matches_division() {
    const BASE: u64 = 58u64.pow(10);

    fn check(x: u64) {
        let mut want = [0; 10];
        let mut value = x;
        for digit in want.iter_mut().rev() {
            *digit = ALPHABET[(value % 58) as usize];
            value /= 58;
        }
        assert_eq!(super::ten_digits(x), want, "value {x}");
    }

    // Exhaust every three-digit suffix at both ends of the ten-digit range.
    for x in 0..58u64.pow(3) {
        check(x);
        check(BASE - 1 - x);
    }
    for exponent in 0..10 {
        for digit in 1..58 {
            let boundary = digit * 58u64.pow(exponent);
            for x in [boundary - 1, boundary, boundary + 1] {
                check(x);
            }
        }
    }
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(58);
    for _ in 0..10_000 {
        check(rng.next_u64() % BASE);
    }
}

/// Checks every encoder against bs58 on `data`: into an ample buffer, an exactly sized
/// one and one a byte short.
fn check(data: &[u8]) {
    let want = bs58::encode(data).into_vec();
    for &(name, f) in ENCODERS {
        assert!(encode_vec(f, data) == want, "{name}: {} bytes", data.len());
        let mut exact = vec![0xAA; want.len()];
        assert_eq!(f(data, &mut exact), Ok(want.len()), "{name}: exact buffer");
        assert!(exact == want, "{name}: exact buffer, {} bytes", data.len());
        if let Some(short) = exact.get_mut(1..) {
            assert_eq!(f(data, short), Err(BufferTooSmall), "{name}: short buffer");
        }
    }
}

/// Several byte patterns across all small lengths and the benchmark sizes up to 32 KiB.
#[test]
fn matches_bs58() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(20);
    // Those of CPUs with and without the IFMA leaf, whichever runs here.
    let thresholds = [
        DC_THRESHOLD,
        super::dc::karatsuba::DC_IFMA_THRESHOLD,
        crate::ntt::ENCODE_THRESHOLD,
        crate::ntt::internals::ENCODE_IFMA_THRESHOLD,
        crate::malachite::ENCODE_THRESHOLD,
        crate::malachite::ENCODE_IFMA_THRESHOLD,
    ];
    let around = thresholds.into_iter().flat_map(|t| t - 8..=t + 8);
    for len in (0..=300).chain(around).chain([1024, 4096, 8192]) {
        let random = random_bytes(&mut rng, len);
        let mut leading_zeros = random.clone();
        leading_zeros[..len.min(3)].fill(0);
        for data in [random, leading_zeros, vec![0xFF; len], vec![0; len]] {
            check(&data);
        }
    }
    // bs58 is quadratic, so the largest sizes get one random input each.
    for len in [32 * 1024, 32 * 1024 + 1] {
        check(&random_bytes(&mut rng, len));
    }
}

/// Values at limb and digit boundaries: `2^(64 k)`, `2^(64 k) - 1`, `58^k` and
/// `58^k - 1`.
#[test]
fn boundaries() {
    for k in [1, 2, 7, 8, 63, 64, 65, 300, 468, 469, 470, 1000] {
        let mut pow2 = vec![0u8; 8 * k + 1];
        pow2[0] = 1;
        check(&pow2);
        check(&vec![0xFF; 8 * k]);
    }
    for k in [1, 9, 10, 11, 19, 20, 21, 100, 1000, 5000] {
        for s in ["z".repeat(k), format!("2{}", "1".repeat(k))] {
            let mut data = vec![0; s.len()];
            let n = decode(&s, &mut data).unwrap();
            check(&data[..n]);
        }
    }
}

/// Random data into buffers of random and boundary sizes: the result fits exactly when
/// the buffer holds it.
#[test]
fn random_buffer_sizes() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(21);
    for round in 0..300 {
        let len = rng.next_u32() as usize % if round % 3 == 0 { 6000 } else { 600 };
        let mut data = random_bytes(&mut rng, len);
        if round % 4 == 0 {
            data[..len.min(3)].fill(0);
        }
        let want = bs58::encode(&data).into_vec();
        let n = want.len();
        let sizes = [
            0,
            n.saturating_sub(2),
            n.saturating_sub(1),
            n,
            n + 1,
            rng.next_u32() as usize % (n + 2),
        ];
        for &(name, f) in ENCODERS {
            for size in sizes {
                let mut out = vec![0xAA; size];
                let got = f(&data, &mut out);
                if size < n {
                    assert_eq!(got, Err(BufferTooSmall), "{name}: {len} bytes into {size}");
                    continue;
                }
                assert_eq!(got, Ok(n), "{name}: {len} bytes into {size}");
                assert!(out[..n] == want[..], "{name}: {len} bytes into {size}");
                assert!(
                    out[n..].iter().all(|&b| b == 0xAA),
                    "{name}: wrote past the result"
                );
            }
        }
    }
}

#[test]
fn edge_cases() {
    for &(name, f) in ENCODERS {
        assert_eq!(f(&[], &mut []), Ok(0), "{name}");
        assert_eq!(encode_vec(f, &[0, 0, 0]), b"111", "{name}");
        assert_eq!(encode_vec(f, &[0, 0, 0, 1]), b"1112", "{name}");
        assert_eq!(encode_vec(f, &[0xFF]), b"5Q", "{name}");
        assert_eq!(f(&[0xFF], &mut [0; 1]), Err(BufferTooSmall), "{name}");
        assert_eq!(f(&[0, 0xFF], &mut [0; 2]), Err(BufferTooSmall), "{name}");
        assert_eq!(f(&[0, 0xFF], &mut [0; 3]), Ok(3), "{name}");
        assert_eq!(f(&[0, 0], &mut [0; 1]), Err(BufferTooSmall), "{name}");
    }
}

/// Inputs too large for bs58: every encoder round trips through `decode`, which is
/// checked against bs58, and agrees with the first encoder. A long inner run of zero bytes
/// gives long runs of '1' inside the result.
#[test]
fn large_inputs_round_trip() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(22);
    let len = 200_001;
    let random = random_bytes(&mut rng, len);
    let mut zero_run = random.clone();
    zero_run[1000..80_000].fill(0);
    for data in [
        random,
        vec![0xFF; len],
        zero_run,
        [vec![0; 3], vec![0xFF; len]].concat(),
    ] {
        let want = encode_vec(ENCODERS[0].1, &data);
        let mut back = vec![0; data.len()];
        let n = decode(core::str::from_utf8(&want).unwrap(), &mut back).unwrap();
        assert!(back[..n] == data[..], "round trip, {} bytes", data.len());
        for &(name, f) in &ENCODERS[1..] {
            assert!(encode_vec(f, &data) == want, "{name}, {} bytes", data.len());
        }
    }
}

/// malachite-nz's conversion (a port of GMP's `mpn_get_str`): an oracle that shares no
/// code with the encoders here.
fn malachite_encode(data: &[u8]) -> Vec<u8> {
    let zeros = data.iter().take_while(|&&b| b == 0).count();
    let x = Natural::from_power_of_2_digits_desc(8, data[zeros..].iter().copied()).unwrap();
    let digits: Vec<u8> = x.to_digits_desc(&58);
    let ones = core::iter::repeat_n(b'1', zeros);
    ones.chain(digits.iter().map(|&d| ALPHABET[d as usize]))
        .collect()
}

/// The subquadratic encoders at 1 MiB, where even `encode_batched` is slow. Inner runs of
/// zero and 0xFF bytes give zero subtrees and long carries.
#[test]
fn huge_inputs_match_malachite() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(23);
    let len = 1 << 20;
    let random = random_bytes(&mut rng, len);
    let mut runs = random.clone();
    runs[100_000..400_000].fill(0);
    runs[600_000..900_000].fill(0xFF);
    for data in [random, vec![0xFF; len], runs] {
        let want = malachite_encode(&data);
        for &(name, f) in &ENCODERS[1..] {
            assert!(encode_vec(f, &data) == want, "{name}, {} bytes", data.len());
        }
    }
}

/// `min_digits` against the digit counts of the smallest and largest values of each bit
/// length, and `max_encoded_len` against the largest value of each byte length.
#[test]
fn digit_bounds() {
    let digits = |data: &[u8]| bs58::encode(data).into_vec().len();
    for bits in 1..=2500u64 {
        let bytes = bits.div_ceil(8) as usize;
        let mut lo = vec![0u8; bytes];
        lo[0] = 1 << ((bits - 1) % 8);
        let mut hi = vec![0xFFu8; bytes];
        hi[0] = (1u16 << ((bits - 1) % 8 + 1)).wrapping_sub(1) as u8;
        let (dl, dh) = (digits(&lo), digits(&hi));
        let m = min_digits(bits);
        assert!(m <= dl && dh <= m + 2, "{bits} bits: {m}, {dl}..={dh}");
    }
    for len in 0..=400 {
        assert!(digits(&vec![0xFF; len]) <= max_encoded_len(len), "{len}");
        assert!(len <= max_encoded_len(len), "{len}");
    }
}

/// Capacity calculations must not wrap when their result is larger than `usize`.
#[test]
fn encoded_length_saturates_without_wrapping() {
    assert_eq!(max_encoded_len(usize::MAX), usize::MAX);
    assert_eq!(max_encoded_len(usize::MAX - usize::MAX / 4), usize::MAX);
    let mut previous = 0;
    for len in [
        0,
        1,
        7,
        8,
        32,
        64,
        512,
        8192,
        1 << 20,
        usize::MAX / 2,
        usize::MAX,
    ] {
        let bound = max_encoded_len(len);
        assert!(bound >= len, "{len} bytes need at least {len} characters");
        assert!(bound >= previous, "capacity decreased at {len} bytes");
        previous = bound;
    }
}

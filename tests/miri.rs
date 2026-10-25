#![cfg(miri)]

mod support;

#[test]
fn all_backends_buffers() {
    for len in [0, 1, 7, 8, 21, 22, 31, 32, 33, 63, 64, 65] {
        let data: Vec<u8> = (0..len).map(|i| (i * 73 + 19) as u8).collect();
        support::check_encode(&data, len);
        support::check_decode(&bs58::encode(&data).into_string(), len);
    }
    for data in [[0; 64], [0xFF; 64]] {
        support::check_encode(&data, 3);
        support::check_decode(&bs58::encode(&data).into_string(), 3);
    }
}

#[test]
fn all_backends_malformed() {
    for input in ["0", "O", "1\0z", "é", "1111_"] {
        support::check_decode(input, 5);
    }
}

#[test]
fn forced_encoder_leaves() {
    use base58_bench::internals::{Leaves, encode_karatsuba_with};
    use support::malachite::encode_malachite_with;
    for leaves in [Leaves::Horner, Leaves::Matrix] {
        let encode = |input: &[u8], out: &mut [u8]| encode_karatsuba_with(input, out, 0, 1, leaves);
        for len in [64, 129] {
            let data: Vec<u8> = (0..len).map(|i| (i * 73 + 19) as u8).collect();
            let want = bs58::encode(&data).into_vec();
            let mut out = vec![0xA5; want.len() + 1];
            assert_eq!(encode(&data, &mut out), Ok(want.len()));
            assert_eq!(&out[..want.len()], want);
            assert_eq!(out[want.len()], 0xA5);
            out.fill(0xA5);
            assert_eq!(
                encode_malachite_with(&data, &mut out, 0, 1, leaves),
                Ok(want.len())
            );
            assert_eq!(&out[..want.len()], want);
            assert_eq!(out[want.len()], 0xA5);
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512ifma"))]
#[test]
fn actual_ifma_shapes() {
    use base58_bench::internals::{ifma_mul, ifma_scratch_len, ifma_threshold, mul_with, scratch_len};
    assert_ne!(ifma_threshold(), 0, "the requested IFMA path must execute");
    for (na, nb) in [(8, 13), (13, 17), (17, 97)] {
        let a = vec![u64::MAX; na];
        let b: Vec<u64> = (0..nb).map(|i| u64::MAX - i as u64).collect();
        let mut want = vec![0; na + nb];
        mul_with(&mut want, &a, &b, &mut vec![0; scratch_len(nb)], 0);
        let mut got = vec![0xA5A5_A5A5_A5A5_A5A5; na + nb];
        ifma_mul(&mut got, &a, &b, &mut vec![0; ifma_scratch_len(na, nb)]);
        assert_eq!(got, want, "{na} x {nb}");
    }
}

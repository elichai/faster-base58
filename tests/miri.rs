#![cfg(miri)]

mod support;

fn check_buffer_case(data: &[u8], selector: usize) {
    support::check_encode(data, selector);
    support::check_decode(&bs58::encode(data).into_string(), selector);
}

fn check_sized_buffer_case(len: usize) {
    let data: Vec<u8> = (0..len).map(|i| (i * 73 + 19) as u8).collect();
    check_buffer_case(&data, len);
}

// These extra entry points are ignored in the ordinary suite, which retains the
// full sequence below. The custom runner selects one per interpreter process.
mod buffer_cases {
    macro_rules! sizes {
        ($($name:ident: $len:literal),* $(,)?) => {
            pub(super) const LENGTHS: &[usize] = &[$($len),*];

            $(
                #[test]
                #[ignore = "run in a separate interpreter process with the custom Miri runner"]
                fn $name() {
                    super::check_sized_buffer_case($len);
                }
            )*
        };
    }

    sizes! {
        bytes_0: 0,
        bytes_1: 1,
        bytes_7: 7,
        bytes_8: 8,
        bytes_21: 21,
        bytes_22: 22,
        bytes_31: 31,
        bytes_32: 32,
        bytes_33: 33,
        bytes_63: 63,
        bytes_64: 64,
        bytes_65: 65,
    }

    #[test]
    #[ignore = "run in a separate interpreter process with the custom Miri runner"]
    fn zeros_64() {
        super::check_buffer_case(&[0; 64], 3);
    }

    #[test]
    #[ignore = "run in a separate interpreter process with the custom Miri runner"]
    fn max_64() {
        super::check_buffer_case(&[0xFF; 64], 3);
    }
}

#[test]
fn all_backends_buffers() {
    for &len in buffer_cases::LENGTHS {
        check_sized_buffer_case(len);
    }
    for data in [[0; 64], [0xFF; 64]] {
        check_buffer_case(&data, 3);
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
    use faster_base58::internals::{Leaves, encode_karatsuba_with};
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
    use faster_base58::internals::{
        ifma_mul, ifma_scratch_len, ifma_threshold, mul_with, scratch_len,
    };
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

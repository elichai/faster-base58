use crate::common::CHUNK;
use crate::dc::{DC_THRESHOLD, Multiplier};
use crate::malachite::{Malachite, decode_malachite};
use crate::mul;
use crate::tests::{decode_vec, random_string};
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{Rng, SeedableRng};

/// `m.mul(a, b)` against Karatsuba, with garbage in `out` and no scratch at all.
fn check(m: &mut Malachite, a: &[u64], b: &[u64]) {
    let mut want = vec![0; a.len() + b.len()];
    mul::mul(
        &mut want,
        a,
        b,
        &mut vec![0; mul::scratch_len(a.len().max(b.len()))],
    );
    let mut got = vec![0x5A5A_5A5A_5A5A_5A5A; want.len()];
    m.mul(&mut got, a, b, &mut []);
    assert!(got == want, "{} x {}", a.len(), b.len());
}

/// Lengths around malachite's thresholds on 64-bit targets: Toom-22 at 16 limbs,
/// Toom-33 at 105, Toom-6h at 300, Toom-8h and unbalanced FFT at 389, balanced FFT at
/// 1000. One [`Malachite`] serves every product, so its buffers grow and shrink.
#[test]
fn mul_matches_karatsuba() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(40);
    let mut m = Malachite::default();
    let lens = [
        0, 1, 2, 15, 16, 17, 104, 105, 300, 389, 390, 999, 1000, 1001, 2500,
    ];
    let random = |rng: &mut Xoshiro256PlusPlus, n| (0..n).map(|_| rng.next_u64()).collect();
    for na in lens {
        let a: Vec<u64> = random(&mut rng, na);
        // The same slice twice takes malachite's squaring path.
        check(&mut m, &a, &a);
        check(&mut m, &a, &a.clone());
        for nb in lens {
            check(&mut m, &a, &random(&mut rng, nb));
            // All-ones limbs maximise every carry.
            check(&mut m, &vec![u64::MAX; na], &vec![u64::MAX; nb]);
        }
    }
}

#[test]
fn high_zero_limbs_and_buffer_reuse() {
    let mut m = Malachite::default();
    for n in [0, 1, 2, 15, 16, 17, 104, 105] {
        for low in [0, 1, u64::MAX] {
            let mut padded = vec![0; n + 3];
            padded[0] = low;
            check(&mut m, &padded, &padded);
            check(&mut m, &padded, &[u64::MAX, u64::MAX, 0]);
            check(&mut m, &[u64::MAX; 16], &padded);
        }
    }
}

#[test]
fn limb_width_adapter_preserves_word_boundaries() {
    let mut m = Malachite::default();
    for (a, b, want) in [
        (0, u64::MAX, [0, 0]),
        (1 << 32, 1 << 32, [0, 1]),
        (0xFFFF_FFFF, 0x1_0000_0001, [u64::MAX, 0]),
        (u64::MAX, u64::MAX, [1, u64::MAX - 1]),
    ] {
        let mut out = [0xA5A5_A5A5_A5A5_A5A5; 2];
        m.mul(&mut out, &[a], &[b], &mut []);
        assert_eq!(out, want);
    }
}

#[test]
fn matches_karatsuba_decode() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(41);
    let t = DC_THRESHOLD * CHUNK;
    for len in [t + 1, 2 * t + 1, 10_000, 40_960, 100_001, 1_000_000] {
        // 58^len - 1 maximises carries.
        let max = "z".repeat(len);
        let min = format!("2{}", "1".repeat(len - 1));
        let zeros = format!("111{max}");
        for s in [random_string(&mut rng, len), max, min, zeros] {
            assert!(
                decode_vec(decode_malachite, &s) == decode_vec(crate::dc::decode, &s),
                "{len}"
            );
        }
    }
}

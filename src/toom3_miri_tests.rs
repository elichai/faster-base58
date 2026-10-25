use super::*;

#[test]
fn interpolation_and_blocks() {
    for (na, nb) in [(32usize, 32usize), (33, 35), (32, 65)] {
        for cancel in [false, true] {
            let mut a = vec![u64::MAX; na];
            if cancel {
                let k = na.div_ceil(3);
                a[2 * k..].fill(0);
            }
            let b: Vec<u64> = (0..nb).map(|i| u64::MAX - i as u64).collect();
            let want = crate::mul::tests::reference(&a, &b);
            let mut got = vec![0xA5A5_A5A5_A5A5_A5A5; na + nb];
            let mut scratch = vec![0x5A5A_5A5A_5A5A_5A5A; mul::scratch_len(nb)];
            mul_with(&mut got, &a, &b, &mut scratch, MIN_THRESHOLD);
            assert_eq!(got, want, "{na} x {nb}, cancel {cancel}");
        }
    }
}

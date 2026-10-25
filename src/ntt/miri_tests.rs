use super::*;

fn check(kernels: Kernels) {
    let tables = Tables::new(5);
    for (na, nb) in [(1, 1), (5, 7), (9, 9), (17, 18)] {
        let a = vec![u64::MAX; na];
        let b: Vec<u64> = (0..nb).map(|i| u64::MAX - i as u64).collect();
        for b in [&b[..], &a[..]] {
            let want = crate::mul::tests::reference(&a, b);
            let n = na.max(b.len());
            for skip in [0, 1] {
                let mut storage = vec![0xA5A5_A5A5_A5A5_A5A5; scratch_len(n) + 2];
                let mut out = vec![0; want.len()];
                mul_with(
                    &mut out,
                    &a,
                    b,
                    &mut storage[skip..skip + scratch_len(n)],
                    &tables,
                    &kernels,
                    1,
                );
                assert_eq!(out, want, "{na} x {}, offset {skip}", b.len());
                assert!(storage[..skip].iter().all(|&x| x == 0xA5A5_A5A5_A5A5_A5A5));
                assert!(
                    storage[skip + scratch_len(n)..]
                        .iter()
                        .all(|&x| x == 0xA5A5_A5A5_A5A5_A5A5)
                );
            }
        }
    }

    let mut ntt = Ntt {
        threshold: 1,
        prepared_threshold: 1,
        ..Ntt::new_owned(kernels)
    };
    let (a, b) = (vec![u64::MAX; 9], vec![u64::MAX - 1; 7]);
    ntt.prepare(32);
    ntt.prepare_power(0, &b, 16);
    assert!(ntt.powers[0].is_some());
    let mut out = vec![0; 16];
    ntt.mul_power(&mut out, &a, 0, &b, &mut vec![0; scratch_len(9)]);
    assert_eq!(out, crate::mul::tests::reference(&a, &b));

    let mut want = vec![0; 8];
    crate::mul::fold_wrapped(&mut want, &out);
    let mut wrapped = vec![0; 8];
    let mut scratch = vec![0; ntt.wrapped_scratch_len(8, 9)];
    ntt.mul_wrapped(&mut wrapped, &a, &b, &mut scratch);
    assert_eq!(wrapped, want);
    ntt.mul_power_wrapped(&mut wrapped, &a, 0, &b, &mut scratch);
    assert_eq!(wrapped, want);
}

#[test]
fn portable_transform_paths() {
    check(Kernels::portable());
}

#[test]
fn selected_simd_transform_paths() {
    #[cfg(target_arch = "x86_64")]
    assert!(
        is_x86_feature_detected!("avx2"),
        "run this with explicit SIMD target features"
    );
    check(Kernels::best());
}

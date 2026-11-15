//! Validate the compile-time ceiling before any codec or mutator initializes caches.

#[cfg(any(fuzzing, test))]
fn validate(requested: &str, features: [bool; 5]) {
    match requested {
        "host" => {}
        "avx2" => {
            assert!(features[1], "AVX2 fuzzing requires AVX2 on this host");
            assert!(
                !features[2..].iter().any(|&f| f),
                "AVX-512 leaked through the AVX2 ceiling"
            );
        }
        "sse41" => {
            assert!(features[0], "SSE4.1 fuzzing requires SSE4.1 on this host");
            assert!(
                !features[1..].iter().any(|&f| f),
                "AVX leaked through the SSE4.1 ceiling"
            );
        }
        _ => panic!("FASTER_BASE58_FUZZ_MAX_ISA must be host, avx2 or sse41"),
    }
}

pub fn init() {
    #[cfg(fuzzing)]
    {
        let requested = option_env!("FASTER_BASE58_FUZZ_MAX_ISA").unwrap_or("host");
        #[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
        {
            let features = faster_base58::internals::fuzz_cpu_features();
            validate(requested, features);
            eprintln!(
                "fuzz max ISA={requested}; effective [sse41,avx2,avx512f,ifma,vbmi]={features:?}"
            );
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse2")))]
        {
            validate(requested, [false; 5]);
            eprintln!("fuzz max ISA={requested}; non-x86 host dispatch");
        }
    }
    #[cfg(not(fuzzing))]
    assert!(
        option_env!("FASTER_BASE58_FUZZ_MAX_ISA").is_none(),
        "FASTER_BASE58_FUZZ_MAX_ISA requires cargo fuzz (cfg(fuzzing))"
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn valid_receipts() {
        super::validate("host", [true; 5]);
        super::validate("host", [false; 5]);
        super::validate("avx2", [true, true, false, false, false]);
        super::validate("sse41", [true, false, false, false, false]);
        super::init();
    }

    #[test]
    fn rejects_missing_hardware_and_leaked_features() {
        for mode in ["avx2", "sse41", "invalid"] {
            assert!(std::panic::catch_unwind(|| super::validate(mode, [false; 5])).is_err());
        }
        for bit in 2..5 {
            let mut features = [true, true, false, false, false];
            features[bit] = true;
            assert!(std::panic::catch_unwind(|| super::validate("avx2", features)).is_err());
        }
        for bit in 1..5 {
            let mut features = [true, false, false, false, false];
            features[bit] = true;
            assert!(std::panic::catch_unwind(|| super::validate("sse41", features)).is_err());
        }
    }
}

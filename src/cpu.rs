//! OS-aware x86 feature checks, using std when enabled.

core::cfg_select! {
    feature = "std" => {
        #[cfg(any(feature = "alloc", test, kani))]
        #[inline]
        pub(crate) fn sse41() -> bool {
            std::is_x86_feature_detected!("sse4.1")
        }

        #[inline]
        pub(crate) fn avx2() -> bool {
            #[cfg(fuzzing)]
            if crate::fuzz_isa::LEVEL < 1 { return false; }
            std::is_x86_feature_detected!("avx2")
        }

        #[cfg(any(feature = "alloc", test, kani))]
        #[inline]
        pub(crate) fn avx512f() -> bool {
            #[cfg(fuzzing)]
            if crate::fuzz_isa::LEVEL < 2 { return false; }
            std::is_x86_feature_detected!("avx512f")
        }

        #[cfg(any(feature = "alloc", test, kani))]
        #[inline]
        pub(crate) fn ifma() -> bool {
            #[cfg(fuzzing)]
            if crate::fuzz_isa::LEVEL < 2 { return false; }
            std::is_x86_feature_detected!("avx512f")
                && std::is_x86_feature_detected!("avx512ifma")
        }

        #[inline]
        pub(crate) fn vbmi() -> bool {
            #[cfg(fuzzing)]
            if crate::fuzz_isa::LEVEL < 2 { return false; }
            std::is_x86_feature_detected!("avx2")
                && std::is_x86_feature_detected!("avx512f")
                && std::is_x86_feature_detected!("avx512bw")
                && std::is_x86_feature_detected!("avx512vl")
                && std::is_x86_feature_detected!("avx512vbmi")
        }
    }
    _ => {
        #[cfg(any(feature = "alloc", test, kani))]
        cpufeatures::new!(detect_sse41, "sse4.1");
        cpufeatures::new!(detect_avx2, "avx2");
        #[cfg(any(feature = "alloc", test, kani))]
        cpufeatures::new!(detect_avx512f, "avx512f");
        #[cfg(any(feature = "alloc", test, kani))]
        cpufeatures::new!(detect_ifma, "avx512f", "avx512ifma");
        cpufeatures::new!(
            detect_vbmi,
            "avx2",
            "avx512f",
            "avx512bw",
            "avx512vl",
            "avx512vbmi"
        );

        #[cfg(not(fuzzing))]
        pub(crate) use detect_avx2::get as avx2;
        #[cfg(all(not(fuzzing), any(feature = "alloc", test, kani)))]
        pub(crate) use detect_avx512f::get as avx512f;
        #[cfg(all(not(fuzzing), any(feature = "alloc", test, kani)))]
        pub(crate) use detect_ifma::get as ifma;
        #[cfg(any(feature = "alloc", test, kani))]
        pub(crate) use detect_sse41::get as sse41;
        #[cfg(not(fuzzing))]
        pub(crate) use detect_vbmi::get as vbmi;
    }
}

#[cfg(all(fuzzing, not(feature = "std")))]
#[inline]
pub(crate) fn avx2() -> bool {
    crate::fuzz_isa::LEVEL >= 1 && detect_avx2::get()
}

#[cfg(all(fuzzing, not(feature = "std"), any(feature = "alloc", test, kani)))]
#[inline]
pub(crate) fn avx512f() -> bool {
    crate::fuzz_isa::LEVEL >= 2 && detect_avx512f::get()
}

#[cfg(all(fuzzing, not(feature = "std"), any(feature = "alloc", test, kani)))]
#[inline]
pub(crate) fn ifma() -> bool {
    crate::fuzz_isa::LEVEL >= 2 && detect_ifma::get()
}

#[cfg(all(fuzzing, not(feature = "std")))]
#[inline]
pub(crate) fn vbmi() -> bool {
    crate::fuzz_isa::LEVEL >= 2 && detect_vbmi::get()
}

#[cfg(test)]
mod tests {
    #[test]
    fn detection_agrees_with_std() {
        #[cfg(fuzzing)]
        let level = crate::fuzz_isa::LEVEL;
        #[cfg(not(fuzzing))]
        let level = 2;
        assert_eq!(super::sse41(), std::is_x86_feature_detected!("sse4.1"));
        assert_eq!(
            super::avx2(),
            level >= 1 && std::is_x86_feature_detected!("avx2")
        );
        assert_eq!(
            super::avx512f(),
            level >= 2 && std::is_x86_feature_detected!("avx512f")
        );
        assert_eq!(
            super::ifma(),
            level >= 2
                && std::is_x86_feature_detected!("avx512f")
                && std::is_x86_feature_detected!("avx512ifma")
        );
        assert_eq!(
            super::vbmi(),
            level >= 2
                && std::is_x86_feature_detected!("avx2")
                && std::is_x86_feature_detected!("avx512f")
                && std::is_x86_feature_detected!("avx512bw")
                && std::is_x86_feature_detected!("avx512vl")
                && std::is_x86_feature_detected!("avx512vbmi")
        );
    }
}

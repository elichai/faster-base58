//! OS-aware x86 feature checks, available without std.

cpufeatures::new!(detect_sse41, "sse4.1");
cpufeatures::new!(detect_avx2, "avx2");
cpufeatures::new!(detect_avx512f, "avx512f");
cpufeatures::new!(detect_ifma, "avx512f", "avx512ifma");
cpufeatures::new!(
    detect_vbmi,
    "avx2",
    "avx512f",
    "avx512bw",
    "avx512vl",
    "avx512vbmi"
);

pub(crate) use detect_avx2::get as avx2;
pub(crate) use detect_avx512f::get as avx512f;
pub(crate) use detect_ifma::get as ifma;
pub(crate) use detect_sse41::get as sse41;
pub(crate) use detect_vbmi::get as vbmi;

#[cfg(test)]
mod tests {
    #[test]
    fn detection_agrees_with_std() {
        assert_eq!(super::sse41(), std::is_x86_feature_detected!("sse4.1"));
        assert_eq!(super::avx2(), std::is_x86_feature_detected!("avx2"));
        assert_eq!(super::avx512f(), std::is_x86_feature_detected!("avx512f"));
        assert_eq!(
            super::ifma(),
            std::is_x86_feature_detected!("avx512f") && std::is_x86_feature_detected!("avx512ifma")
        );
        assert_eq!(
            super::vbmi(),
            std::is_x86_feature_detected!("avx2")
                && std::is_x86_feature_detected!("avx512f")
                && std::is_x86_feature_detected!("avx512bw")
                && std::is_x86_feature_detected!("avx512vl")
                && std::is_x86_feature_detected!("avx512vbmi")
        );
    }
}

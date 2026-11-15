//! Compile-time ISA ceilings for cargo-fuzz; absent from ordinary consumers.

const fn parse(value: Option<&str>) -> u8 {
    match value {
        None => 2,
        Some(value) => match value.as_bytes() {
            b"host" => 2,
            b"avx2" => 1,
            b"sse41" => 0,
            _ => panic!("FASTER_BASE58_FUZZ_MAX_ISA must be host, avx2 or sse41"),
        },
    }
}

#[cfg(fuzzing)]
pub(crate) const LEVEL: u8 = parse(option_env!("FASTER_BASE58_FUZZ_MAX_ISA"));

#[cfg(fuzzing)]
const _: () = {
    assert!(
        cfg!(feature = "__internals"),
        "ISA fuzzing requires __internals"
    );
    if LEVEL < 2 {
        assert!(
            cfg!(all(target_arch = "x86_64", target_feature = "sse2")),
            "AVX2 and SSE4.1 fuzz ceilings require x86-64 with SSE2"
        );
        assert!(
            !cfg!(target_feature = "avx512f"),
            "capped fuzz builds require generic target flags; AVX-512 is globally enabled"
        );
    }
    if LEVEL == 0 {
        assert!(
            !cfg!(target_feature = "avx"),
            "SSE4.1 fuzz builds require generic target flags; AVX is globally enabled"
        );
    }
};

#[cfg(test)]
mod tests {
    #[test]
    fn accepts_only_named_ceilings() {
        assert_eq!(super::parse(None), 2);
        assert_eq!(super::parse(Some("host")), 2);
        assert_eq!(super::parse(Some("avx2")), 1);
        assert_eq!(super::parse(Some("sse41")), 0);
    }

    #[test]
    #[should_panic(expected = "FASTER_BASE58_FUZZ_MAX_ISA must be host, avx2 or sse41")]
    fn rejects_unknown_ceiling() {
        super::parse(Some("avx512"));
    }
}

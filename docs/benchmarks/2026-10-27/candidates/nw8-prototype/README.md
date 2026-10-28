# IFMA window-8 prototype

This bounded experiment changes `NW_MAX` from12 to8 and removes the unreachable9–12 picker arms. The comparison base is frozen4e5; the candidate resource identity is `7e297910abb1476d9031051d86dc31a3c5217724c1e598686c64ac297996ef87`. [The patch](candidate.patch) is not the later feature-wired implementation.

Verified static savings are95,440–95,456 linked code bytes in the four GCP CPU/cache cohorts. Kernels1–8 are byte-identical in the matched static review; specializations9–12 disappear. The arithmetic domain, production threshold384, maximum input limb count1664 and existing fallback remain unchanged. Normal public512-case configurations, required native IFMA/emulation/Shape checks and all29 guarded fixtures passed before timing.

[Public observations](public.csv), [micro observations](micro.csv), [sizes](sizes.csv) and [metadata](metadata.json) retain all1,776 first-run processes. Public results include mixed outcomes. Balanced47/64/128-limb micro shapes observed higher CPU costs, but short shapes below384 limbs lie outside the ordinary production IFMA threshold and are explicitly micro tests.

The [later observer controls](../../observer-controls/README.md) found systematic file-identity/loading-context effects. Therefore these ordinary two-file timings are descriptive observations, not established source-level speedups or slowdowns. Static size savings remain verified.

Wide12 remains the default. Final main offers opt-in `compact`, with separate exact-source qualification and a pending trustworthy final-source timing supplement. This prototype dataset is never relabeled as that feature build.

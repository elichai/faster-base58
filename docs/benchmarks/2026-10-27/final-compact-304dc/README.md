# Final-source compact tradeoff

Measured **2026-10-28 UTC**, within the broader campaign stored under `2026-10-27`.

Keep the wide12 default. The opt-in `compact` feature removes about 93.2 KiB of linked x86 code in these actual ordinary consumers. It also improves cache-off encoding at 4 KiB–4 MiB in this cohort, while large decoding and cache-mode conversions mostly cost 1–3%. These are whole-consumer results on one AMD EPYC 9B45 VM, including compiler and call-context effects.

This study uses final integrated source `304dc4c5…000add`, with the real feature wiring and constant-threshold cleanup. It is separate from the earlier hardcoded-NW8 prototype, the frozen 4e5 full-backend grid and the local final-source [size supplement](../final-size-304dc/README.md). No earlier row is relabeled as this source. Auto-only measurements do not decide which backend families to remove.

## Code size

Both-direction Auto cdylibs are compared with matching empty observer cdylibs. Values are executable-section bytes above that empty scaffold, not standalone crate-only bytes. Allocator/panic/runtime routines can be linked by a real consumer. The actual linker is LLD 23.1.1; GNU strip 2.40 is a separate file-size step.

| CPU settings | Policy | Wide code above empty | Compact code above empty | Saving |
|---|---|---:|---:|---:|
| Generic | Default |286,445 B|191,005 B|95,440 B /33.3%|
| Generic | Cache |277,101 B|181,661 B|95,440 B /34.4%|
| Native | Default |274,029 B|178,589 B|95,440 B /34.8%|
| Native | Cache |255,309 B|159,869 B|95,440 B /37.4%|

[Every artifact](linked-artifacts.csv), [matching empty deltas](linked-above-empty.csv) and [actual source/compiler/feature contracts](build-contracts.json) remain available. Native means actual `-C target-cpu=native` on this VM. The local size supplement's explicit `znver5` build is a different compiler contract: native/cache compact is 159,869 B above empty here versus 159,885 B there. M5 and scalar Wasm have no executable/data-section saving in that local supplement. Small gzip differences are not semantic differences. Signed DATA/BSS/RELRO and alignment deltas are not retained cache heap.

## Measured latency

Ratios below are compact/wide paired process-CPU latency. Lower favors compact. They are medians of five alternating fresh-process pairs. Each process has three warm batches of at least 100 ms process CPU. Absolute times in the linked tables are separate medians; their quotient can differ from the paired ratio and can even favor the other label near parity. Bold marks only the lower absolute median. The displayed results retain all observations and are not significance bounds.

| CPU / policy | Payload | Encode ratio | Decode ratio |
|---|---:|---:|---:|
| Generic / default |4 KiB|0.924284|1.010433|
| Generic / default |64 KiB|0.954280|1.011273|
| Generic / default |4 MiB|0.981802|1.015573|
| Native / default |4 KiB|0.909468|1.017231|
| Native / default |64 KiB|0.946250|1.013103|
| Native / default |4 MiB|0.974818|1.017026|
| Generic / cache |4 KiB|1.013158|1.025414|
| Generic / cache |64 KiB|1.015936|1.013358|
| Generic / cache |4 MiB|1.011741|1.017384|
| Native / cache |4 KiB|1.004279|1.024514|
| Native / cache |64 KiB|1.011637|1.014149|
| Native / cache |4 MiB|1.016850|1.010900|

Across all 4 KiB–4 MiB rows, cache-off encoding ratios are 0.9095–0.9819, default decoding 1.0104–1.0246, cache encoding 1.0043–1.0297 and cache decoding 1.0069–1.0290. Encoding's benefit and decoding/cache costs both matter. The feature trades linked code and emitted specialization for whole-program speed; it is not uniformly slower or faster.

All 29 sizes and absolute latencies are in the readable [generic/default](generic-default.md), [generic/cache](generic-cache.md), [native/default](native-default.md) and [native/cache](native-cache.md) tables. [feature-cells.csv](feature-cells.csv) retains 232 cells, [all-pairs.csv](all-pairs.csv) retains all 1,440 feature/control pairs, and [summary.json](summary.json) binds counts and raw hashes.

Tiny extrema need separate context. Native/default 22 B decoding is 29.413→22.026 ns; native/cache 23 B is 25.292→29.823 ns. Those sizes bypass IFMA. Their repeat wrappers directly call the library decoder, whose placement differs between linked images; [decode-layout.csv](decode-layout.csv) records exact module offsets, lengths and raw body hashes. Call/layout context is a plausible contributor, but its causal effect was not isolated. These cells do not measure the speed of an IFMA primitive.

## Observer qualification and limitations

An earlier two-file observer measured byte-identical libraries about 27% apart in tiny decoding. Same-file labels were near parity. That observation implicates file identity/loading context; its physical mechanism is unknown. The old observations remain in [observer controls](../observer-controls/README.md) and are not repaired or pooled into this epoch.

This new observer installs every immutable image into one fixed path/device/inode and global padded length, only between fully exited/reaped direct children. The image's original prefix and loadable segments are unchanged; padding is not counted as linked size. CPU 0, seed 0, equal-length labels and mod64 input/output residues 1/3 are fixed. Exact input bytes, output capacity, repeated export/pointer/signature/arguments, mapped text/readonly bytes, poisoned output, return value, full golden output, canaries and unchanged input are checked outside clocks. Independent Natural fixtures are unchanged.

The qualification sequence completed all 498 untimed ABI cases, then 10 known-identical-content timing processes, then the entire 2,880-process feature/control study. The known-sensitive five control ratios were 1.001979, 0.993525, 1.001335, 1.009853 and 1.037619. All 56 study control cells and every pair are retained in the [control table](identical-controls.md) and [CSV](identical-controls.csv): cell medians range 0.993936–1.018033; individual pairs range 0.812408–1.114856. No outlier or unfavorable cell was deleted. This does not establish sub-percent measurement stability.

General GOT targets and writable CPU/cache state are recorded for inspection; their presence is not a proof of all-target/state correctness. One narrow encoder-selfbinding predicate verified 466 qualifications; 32 with no such relocation were not applicable. The qualified cleanup helper signals the owned group and reaps its direct child. Declared workers have no asynchronous descendants. Closed controller/worker/group evidence is not an independent proof about arbitrary descendants.

[receipts.json](receipts.json) preserves source/image/raw/archive hashes and scope. [observer/](observer/) contains the exact executed source and its pure tests. Its `PROTOCOL.md` was sealed before execution and retains that historical pending status; completion comes from these source-bound receipts. The fixed historical deadline must be explicitly revised for a future campaign, and a fresh preparation/qualification is required. This export contains no raw binaries, raw logs or private build paths.

The [reverse source patch](source-304dc-to-07ec.patch) and [replay receipt](source-final-replay.json), followed by the earlier proof/API bridges, reconstruct exact frozen 4e5. This is source provenance, not a performance patch or retargeted measurement.

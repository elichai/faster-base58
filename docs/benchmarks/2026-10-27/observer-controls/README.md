# Observer controls and attribution limits

Byte-identical GCP Auto libraries produced a systematic tiny-decoder difference when loaded from two files. Pinning to guest CPU0, equal-length labels, fixed buffer alignment and longer batches did not remove it. A later same-file control was near parity. These observations implicate file identity or its loading context; they do not establish a physical mechanism.

This limits causal claims from the older ordinary-library comparisons. Their observed values remain published. No favorable condition is selected, no negative control is discarded, and no older dataset is relabeled with a newer observer.

## Full factorial calibration

The calibration used 14 operation/size/control cells, all 16 combinations of four factors, five alternating fresh pairs and three warm batches: 2,240 timed processes plus 448 untimed qualifications. Factors were guest affinity 0–3/CPU0, original/equal-length loader labels, original/fixed mod64 input/output residues, and 10/100 ms minimum CPU batches. Order was fixed before execution. CPU endpoints are samples, not proof that no unpinned migration occurred. Environment capture was whitelisted; no ASLR, frequency, security or host settings changed.

[Cells](calibration-cells.csv), [every pair](calibration-pairs.csv), [condition summaries](calibration-conditions.csv) and [factor contrasts](calibration-factor-contrasts.csv) retain the complete observations. The [metadata](calibration-metadata.json) binds the raw receipt and identical artifact hashes. No factor combination restored parity. In the strongest fixed condition, 23 B decode measured about 26.10/33.33 ns, median B/A 1.279389, with five ratios spanning 1.260786–1.315679.

The original observer recorded the checked-export pointer, while timing called a distinct repeated export. That metadata gap was found during review. A **separate untimed** inspection subsequently checked the actual repeated function, its primitive argument tuple, output capacity, iteration argument, complete mapped text/readonly bytes, inner call targets and relevant writable state. No mismatch was found in those new processes. This does not prove historical in-memory state or repair the old receipt retroactively.

## Same-inode discriminator

The next observer used only 23 B decoding, CPU0, fixed input/output residues 1/3 mod64 and 100 ms batches. Five pairs per mapping group were interleaved and A/B order was balanced. All labels had equal length. It poisoned output before repeat1 qualification and before every calibration/warm batch, outside the clock, then checked return value, complete output, canaries and unchanged input. It also recorded the actual repeated pointer/offset, mapped bytes and exact argument tuple.

Both groups used byte-identical ELF contents. The two-file group retained different file identities; the same-file group pointed both labels to the same inode. Every one of the 20 first-run processes passed. [All ten pairs](inode-pairs.csv) and [the summary](inode-summary.json) are retained.

| Mapping group | A median | B median | Median paired B/A | Five paired ratios |
|---|---:|---:|---:|---|
| Two files | 26.130 ns | 33.322 ns | 1.273584 | 1.292278, 1.269755, 1.221437, 1.306215, 1.273584 |
| Same file | 26.147 ns | 25.950 ns | 0.992318 | 0.998102, 0.992318, 0.992034, 0.998457, 0.953727 |

The actual repeat entry offset was 90,048 in both groups. This is evidence for a file-identity/loading-context effect, not a claim about physical-page placement, instruction caches, allocation or another hardware cause. The mechanism is unresolved.

## Single-worker backend grid

The native backend grid uses one frozen executable per CPU/cache cohort. Runtime backend names select typed Rust function pointers from a registry; every loop calls that selected function with opaque input/output, and every batch checks full output and length against independent Natural fixtures. There is no second DSO choice or ctypes binding. Build hashes, source identities, supported domains and counted/uncounted separation are checked before execution.

The old worker starts with zeroed output and does not re-poison it or add benchmark canaries between batches. Its post-batch checks are weaker than the discriminator's checks. Those stronger checks are not retroactively claimed for old data. The grid can still describe source-bound throughput with explicit sampling ranges, fallback aliases and temporal Auto controls. Close medians are not significance or causal claims.

## Capacity and crossover controls

[Capacity timings](capacity-timing.csv) and [allocation windows](capacity-allocations.csv) use one common helper that computes both bounds, then changes only requested Vec capacity before calling the same borrowed decoder. All prefix/bound work occurs in both policies. The native/cache ratios are 1.000263 at 4 KiB and 0.993562 at 4 MiB. This does not reproduce the actual old/current owned observations; capacity alone is not established as their cause. The probe does not control every ordinary wrapper or code-generation difference.

[GCP DC3072 observations](dc3072-gcp-timing.csv) and [linked sizes](dc3072-gcp-sizes.csv) retain all 308 cells, including byte-identical controls. [The local candidate review](../candidates/dc3072/dc3072-review.md) retains all M5/Wasm observations. The candidate was rejected for verified code growth and no demonstrated benefit, not a proven source-induced slowdown.

All clocks report process CPU primarily and wall secondarily. Empty results are not subtracted. Descriptive paired ranges are not confidence intervals. [Evidence index](evidence.json) binds these selected exports to their immutable source receipts.

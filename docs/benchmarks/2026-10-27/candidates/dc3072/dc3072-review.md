# DC3072 candidate: decision and evidence

Rejected: retain DC_THRESHOLD=5120. The candidate adds 10–11 KiB of linked code and demonstrates no speed benefit in its changed range. This decision is not a claim of a proven causal slowdown; unchanged controls were noisy. No further threshold tuning is planned.

## Completed local observations

Both frozen platforms completed without case cuts or selective retries. The candidate was rejected for lack of demonstrated benefit. These measurements do not establish a speed benefit. Byte-identical controls have material variability, so close causal rankings require calibration.

The only candidate source change is `src/encode/dc.rs`'s existing non-IFMA constant, 5120 → 3072. Baseline resource identity is `07ec12777e6065bd644baa75af4fd2cd0fe38fc39674957dc1eb220dc220f3b1`; candidate identity is `f0c7311366525a95539be9e53a12ec025d13ad46bdabc31de0563d70f39c0f88`. Compact and atomic-threshold proposals are absent from both sources.

| Platform | Cases | Fresh processes | Accepted warm batches | Retained calibrations | AC-guard duration |
|---|---:|---:|---:|---:|---:|
| M5 | 354 | 2,124 | 6,372 | 9,750 | 254.549 s |
| Wasm | 354 | 2,124 | 6,372 | 3,261 | 498.896 s |

AC guards accepted both runs. Each case used three alternating fresh pairs and three accepted warm batches per process. M5's primary clock is process CPU; Wasm's is elapsed time. The duration includes fresh-process validation and calibration. Every raw observation remains in the sealed files. Thermal queries were not consistently supported; temperature control is not established.

## Focused no-cache Auto encode

Each entry is the minimum / median / maximum percentage change across the three paired process medians, candidate / baseline minus one. Positive values mean longer time. These are descriptive ranges, not confidence intervals or causal effects.

| Significant input bytes | M5 CPU change | Wasm elapsed change |
|---:|---:|---:|
| 3,072 | −0.66 / −0.03 / +0.65% | −0.40 / +5.56 / +7.37% |
| 3,073 | +5.25 / +7.69 / +10.03% | +10.95 / +16.30 / +18.62% |
| 3,584 | +5.10 / +5.32 / +5.93% | +13.68 / +14.25 / +14.92% |
| 4,095 | −3.81 / +1.45 / +8.94% | −11.28 / +1.02 / +1.38% |
| 4,096 | +1.05 / +1.12 / +2.27% | +0.45 / +3.66 / +13.14% |
| 4,097 | −0.11 / +0.29 / +1.26% | −0.06 / +0.40 / +0.78% |

All 708 platform/case observations are in `all-local-observations.csv`. `positive-default-auto-observations.csv` preserves every no-cache Auto cell whose paired median exceeded one. It is an observation list, not a ranking of candidate effects. Other conspicuous positive medians include M5 encode 1 byte (+7.69%; range +6.45..+8.24%) and Wasm encode empty (+8.69%; +7.78..+17.80%). Source control flow alone does not assign these timing changes to the changed route.

## Exact byte-identical controls

All cache Auto artifacts, all empty artifacts, and all no-alloc artifacts are byte-identical between epochs, both raw and stripped. Artifact hashes are bound in `timing-plan-v2.json`, `sizes.json`, and each control receipt. There are 291 control cells per platform: 63 cache Auto, 126 empty, and 102 no-alloc.

| Platform / group | Individual paired changes | Cell paired-median changes |
|---|---:|---:|
| M5, all controls | −24.71..+15.00% | −9.74..+10.43% |
| M5, cache Auto | −10.04..+8.58% | −1.50..+5.58% |
| M5, empty | −14.23..+15.00% | −9.74..+8.85% |
| M5, no-alloc | −24.71..+11.42% | −3.91..+10.43% |
| Wasm, all controls | −18.90..+27.90% | −6.72..+9.13% |
| Wasm, cache Auto | −17.34..+8.54% | −5.77..+2.66% |
| Wasm, empty | −18.90..+14.91% | −6.70..+9.13% |
| Wasm, no-alloc | −16.04..+27.90% | −6.72..+6.87% |

M5's identical default-empty decode-16 control had three positive paired changes, +3.04 / +8.85 / +11.06%, using the identical artifact SHA `927b704da2d59330d1610686403df1a45e047b363f8ff202d41eba5343d00d8c`. Wasm's identical cache-Auto encode-32KiB control had three negative changes, −7.06 / −5.77 / −1.53%, using SHA `64680e48fcb8bf62acd640b3ce0b5f6ed0f6ff92c33bb9d52c680f6a373ee16e`. Thus three matching signs do not by themselves identify a candidate machine-code effect. The exact three pairs and extrema are retained in `control-variability-{m5,wasm}.json`. No control subtraction, uncertainty bound, or calibration timer was applied here.

## Verified cost and qualification

The matching ordinary both-direction no-cache Auto linked artifacts grow:

| Platform | Code | Other section change | Stripped whole artifact | Stripped gzip |
|---|---:|---:|---:|---:|
| M5 | +10,240 B (+4.08%) | readonly +848 B | +32 B | +6,380 B |
| Wasm | +11,032 B (+12.75%) | data +32 B | +11,072 B | +4,698 B |

Whole-artifact and section deltas have different scopes. Their difference is not given an unverified packaging explanation. Wasm initial memory is unchanged. Unchanged controls have zero linked-size deltas.

The newly active route covers 3,073..=4,096 significant input bytes on these non-IFMA consumers. Source review shows it uses per-call D&C heap buffers; the previous batched route uses the supplied output without allocation. Allocation counts and peak memory were not measured. The public no-alloc API is unchanged.

Before timing, all six normal public integration configurations passed with `PROPTEST_CASES=512`. All 24 guarded ordinary consumers passed the 35 independent Natural fixtures and exact/ample/short/zero-capacity cases. `qualification-evidence.json` binds those receipts. Timing preflight and final sealing rechecked source, fixture, plan, worker, and artifact hashes.

## Evidence

- Frozen plan: `timing-plan-v2.json`, SHA `c57b4633aafa238d3ebcd9e1629a3a8b3df14514d0b851f5863a2361c9f16c73`.
- M5 raw: `timing/m5/raw.jsonl`, SHA `b969080654cd973abc6d4b1f456f63672fb738ab80cb0c8163a92eb5685bed5a`; summary SHA `aa41b6206b732de15e8cd514331bb42f1dba75aa6e4a9800ce4ed554c1b2d914`.
- Wasm raw: `timing/wasm/raw.jsonl`, SHA `78b557df5e551ff82050e9b1bd69c414a0dd8ebb4b770e03b9671509d874b1f8`; summary SHA `0848044640a21d56f19408893b7c3f98998e116159a739d792a1299bc6afbb89`.
- Combined observations: `all-local-observations.csv`, SHA `34e5307a2c180e9a301d67247270c04f1c39dc3d761377b95e7a5beef2d0f5bb`.
- Complete receipt: `timing-complete.json`, SHA `64de149c901944d1a6051677a75015b2d626279729a5419b164d63951436188b`.
- Hash manifest: `timing-evidence.json`, SHA `2808c0b7e4884ed6eeff1a5d21779db99f602fa7f6d92c71de3a7391cd9c6101`.

Skipped / unchecked / risk: no new calibration, allocation count, peak-memory, hardware-counter, affinity, or cleanup experiment was run in this campaign. Main and prior sealed reports are unchanged. Causal attribution remains unresolved; the decision retains 5120 because the candidate adds code without a demonstrated benefit.

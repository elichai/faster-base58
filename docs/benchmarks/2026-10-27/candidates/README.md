# Measured candidates

The patches record bounded experiments against the source epochs in their metadata. They are not a set of changes to apply together. The thin owned empty wrapper was accepted. Final main also has separate opt-in compact wiring and threshold-state cleanup; their exact source qualification is not inferred from these older prototypes.

| Candidate | Outcome | Evidence |
|---|---|---|
| Direct tiny call to `decode_short` | No change in emitted owned binaries; not separately timed | LLVM already inlined and simplified the guarded call |
| Monolithic empty return | Rejected | Cache empty latency about 4.70 ns versus 3.71 ns current and 2.94 ns actualold in its paired campaign; frame grew |
| Thin inline empty wrapper / nonempty helper | Accepted | About 1.9 ns empty; default 32 B owned remains about 4.7%/~1.3 ns above actualold |
| Full output capacity through 128 B | Rejected | No 32 B gain; 64 B worsened, with 256 B additional code per configuration and up to 34 extra requested bytes |
| Closed no-cache Horner leaf | Completed, unmerged; no established benefit | M5 saves 4,864 code +8,236 readonly bytes; native GCP 22 B decode observed +31.11%, with causal attribution limited by identical-file controls |
| Medium decoder outline | Completed, unmerged; no established benefit | Smaller M5 frame; generic GCP 4 MiB decode observed +11.1%, with causal attribution unresolved |
| Combined leaf/outline | Completed, unmerged; no established benefit | M5 saves 4,800 code +8,144 readonly bytes; generic 512 B decode observed +20.64%, native 22 B +20.71%; no established source-level cause |
| IFMA maximum window12→8 prototype | Default rollout rejected; compact integrated separately | Verified GCP saving 95,440–95,456 code bytes, mixed observed speed. Prototype data is separate from feature-wired final source |
| Capacity-only owned observer | Complete; capacity alone not an established cause | One common prefix/bounds/borrowed decoder. Native/cache tight/full ratio 1.000263 at 4 KiB and 0.993562 at 4 MiB; it does not reproduce actual old/current differences |
| Non-IFMA encoder threshold5120→3072 | Rejected for code growth and no demonstrated benefit | Verified +10–11KiB local code, with no established speed benefit; unchanged controls prevent causal slowdown claims |
| IFMA detector snapshot | Recipe only | [Production loads and experiment requirements](../detector-hoist.md) |

The normal default/std/cache public integrations ran without `__internals`, with 512 property cases, so closed production specialization was exercised. Ordinary ABI goldens cover all 29 sizes through 4 MiB, leading-zero variants and invalid decoder input. These checks do not replace the native x86 measurements required before accepting architecture-sensitive tuning.

The [GCP owned/ablation review](../owned-and-ablation-review.md) retains adverse cells and exact-artifact comparisons. Smaller frames and identical normalized instruction bodies do not establish a performance cause; code placement is an inference, not a measured hardware explanation.

Later byte-identical controls exposed systematic observer/loading-context differences. The full 2,240-process calibration did not remove them; the 20-process discriminator measured a 1.273584 median ratio for two files and 0.992318 for one shared file. [All observer controls](../observer-controls/README.md) remain recorded. The physical cause is unknown. Leaf/outline observations remain descriptive and the candidates stay unmerged. Three matching paired signs alone do not prove a source effect.

Each CSV retains its paired process medians/ranges and source-specific controls. Do not construct a new paired claim from controls that occurred in separate candidate contexts. In particular, the short-policy experiment compares each control with that candidate; it does not create a directly paired old-versus-thin 64/128 B study.

`thin-wrapper.patch` contains the candidate's original helper comment. Main uses the more precise wording, “Keep the nonempty decoder's frame off the empty path.” That one comment change has an explicit source bridge and does not change executable bodies. All old datasets remain immutable.

# Differential fuzzing

Both targets compare Auto, `no_alloc`, and the backends exposed by `__internals`
with `bs58`. They check empty inputs, leading zeros, exact/short/oversized output
slices, result suffixes, and canaries around each slice. Decoding checks the first
invalid byte index. The backends take
`&str`, so malformed UTF-8 is tested after lossy conversion; arbitrary raw invalid
UTF-8 cannot be passed to those APIs.

`encode_all` accepts up to 65,537 bytes: one byte past 64 KiB. Full-width inputs
cross from 8,192 to 8,193 `u64` limbs. `decode_all` derives its raw-input budget from
`max_encoded_len(65_537)`, currently 89,502 bytes. It checks both lossy strings and
a valid alphabet mapping, then inserts an invalid NUL. This is a raw-byte budget;
lossy UTF-8 can expand it by up to 3 times, and NUL insertion adds one byte.

The earlier 89,493-byte decode budget was nine bytes below the encode bound. It
was a campaign limit, not a current NTT-size boundary. The generator keeps its old
seeds and adds all-`z` inputs at 89,499..89,502 characters. Such an input represents
`58^n - 1`: 89,499 characters fit 8,192 limbs, while 89,500 require 8,193. The final
89,502-character seed can decode to 65,538 bytes, so the decode budget does not
impose the encode target's binary-length limit.

These budgets reach divide-and-conquer, Toom-3, matrix and NTT paths, including
the larger IFMA multiplication thresholds on supported CPUs. They do not cover
all supported NTT sizes or force unsupported instructions. For nonzero valid
inputs near either decode cap, the current split/power bounds prepare log14
tables; moving the cap does not cross that boundary. The next prepared-table
transition is at 96,641 characters. Actual transform use also depends on operand
lengths and the selected multiplication thresholds.

Install `cargo-fuzz` and a nightly Rust toolchain, then run from the repository:

```sh
python3 fuzz/test_seed_corpus.py
python3 fuzz/seed_corpus.py
cargo +nightly fuzz run encode_all fuzz/corpus/encode_small -- -max_len=2048 -max_total_time=60 -timeout=30 -rss_limit_mb=4096
cargo +nightly fuzz run decode_all fuzz/corpus/decode_small -- -max_len=2048 -max_total_time=60 -timeout=30 -rss_limit_mb=4096
cargo +nightly fuzz run encode_all -- -runs=0 -max_len=65537 -timeout=120 -rss_limit_mb=4096
cargo +nightly fuzz run decode_all -- -runs=0 -max_len=89502 -timeout=120 -rss_limit_mb=4096
```

AddressSanitizer is enabled by default. The fuzz profile also enables integer
overflow checks. The short-input campaigns perform mutations quickly; replaying
the full boundary corpus exercises larger kernels without spending the entire
mutation budget on the quadratic oracle. For sustained large-input fuzzing,
replace `-runs=0` with `-max_total_time=3600`.

The generator preserves existing seeds and fails if a named seed was edited,
including under `python3 -O`. New
coverage seeds and crash artifacts are ignored by Git. Replay any crash with
`cargo +nightly fuzz run TARGET PATH_TO_ARTIFACT`.

See [verification evidence](../docs/review/verification.md) for actual run counts,
proof bounds and architecture limits. Command examples omit the local `rtk` proxy.

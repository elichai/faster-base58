# CI and repository scripts

This guide lists every GitHub Actions job and repository script. For each, it
gives what it checks, when it runs, where it runs and the shortest local command.
Detailed procedures stay next to their tools: [fuzzing](fuzz/README.md), the
[fuzz corpus store](docs/fuzzing-corpus.md), [development Miri](ci/miri/README.md),
the [linked-size probe](tools/code-size/README.md) and
[performance measurement](docs/performance.md). [CORRECTNESS.md](CORRECTNESS.md)
states what the checks establish and which gaps remain. A passing job establishes
only the checks and platforms that it executes.

The workflow files are the record. If a command here differs from a workflow step,
the workflow is correct. Run commands from the repository root, where rustup
selects the stable toolchain in [rust-toolchain.toml](rust-toolchain.toml). An
`export` line changes the later commands in the same shell. Placeholders in
capitals, such as `NEW_DIR`, are paths that you supply.

## Toolchains and tools

[rust-toolchain.toml](rust-toolchain.toml) pins stable Rust for CI and for local
builds in the repository. [ci/nightly/rust-toolchain.toml](ci/nightly/rust-toolchain.toml)
pins the nightly; jobs install it with the
[nightly-toolchain](.github/actions/nightly-toolchain/action.yml) action, which
exports its name as `RUST_NIGHTLY`. [Dependabot](.github/dependabot.yml) proposes
weekly updates to both files. The custom Miri job takes its Cargo toolchain from
[ci/miri/pins.json](ci/miri/pins.json) instead. The `env:` block of
[verification.yml](.github/workflows/verification.yml) pins the other versions.
[fuzz-corpus.yml](.github/workflows/fuzz-corpus.yml) and
[miri-nextest-benchmark.yml](.github/workflows/miri-nextest-benchmark.yml) repeat
the cargo-fuzz and Nextest pins.

| Tool | Version | Used by |
| --- | --- | --- |
| Rust stable | 1.99.1 ([rust-toolchain.toml](rust-toolchain.toml)) | Tests, builds, lint, property coverage, SIMD inlining |
| Rust MSRV | 1.95 | `msrv` |
| Rust nightly | `nightly-2026-11-08` ([ci/nightly](ci/nightly/rust-toolchain.toml)) | Miri, fuzzing, fuzz coverage, sanitizers |
| Custom Miri Cargo | `cargo_toolchain` in [pins.json](ci/miri/pins.json) | `miri_custom` |
| cargo-nextest | 0.9.150 | Test jobs, Nextest Miri comparison |
| cargo-hack | 0.6.45 | Feature-powerset builds and Clippy |
| cargo-llvm-cov | 0.9.1 | `proptest_coverage` |
| cargo-fuzz | 0.13.2 | Fuzz jobs |
| Kani (`kani-verifier`) | 0.68.0, with `z3` on `PATH` | `proofs`, `proofs_open` |
| cross | Git revision `74e99b2223de06166f8cdcc1af7f1308ec52baf4` | `big_endian` |
| wasmtime | 50.0.1 | `wasm_simd128` |
| rustup-toolchain-install-master | 1.12.2 | `miri_custom` |
| Clang/LLVM | 23 (`clang-23`, `libclang-rt-23-dev`) | Fuzzing and native sanitizers on Linux |
| Python | 3.11 or later (the scripts use `tomllib`) | Scripts |
| Node | Not pinned | Wasm checks |
| Matplotlib | [requirements.txt](assets/graphs/requirements.txt) | Figure rendering only |

Install the Rust toolchains, components and Cargo tools. The first `rustc` call
installs the pinned stable with clippy and rustfmt; later commands use `$nightly`:

```sh
rustc -vV
rustup component add llvm-tools
rustup target add wasm32-unknown-unknown wasm32-wasip1 aarch64-unknown-linux-gnu thumbv7em-none-eabi x86_64-unknown-none i686-unknown-linux-gnu
rustup toolchain install 1.95 --profile minimal
export nightly=$(sed -n 's/^channel = "\(.*\)"$/\1/p' ci/nightly/rust-toolchain.toml)
rustup toolchain install "$nightly" --profile minimal --component miri --component rust-src --component llvm-tools-preview
cargo install --locked cargo-nextest --version 0.9.150
cargo install --locked cargo-hack --version 0.6.45
cargo install --locked cargo-llvm-cov --version 0.9.1
cargo install --locked cargo-fuzz --version 0.13.2
cargo install --locked kani-verifier --version 0.68.0
cargo kani setup
```

On Linux, the [setup-clang23](.github/actions/setup-clang23/action.yml) action
installs `clang-23` and `libclang-rt-23-dev` from the signed `apt.llvm.org`
repository. It sets `CC`, `CXX`, `CC_<triple>` and `CXX_<triple>` to `clang-23`
and `clang++-23`. Set the same variables locally, so that the C++ code in these
builds matches the LLVM 23 runtime of the pinned nightly.

Do not run the scripts with `python3 -O`. Several of them depend on assertions and
refuse optimized mode.

## Host limits

| Limit | Effect |
| --- | --- |
| AVX-512 kernel tests (`mul_ifma::tests::avx512_matches_reference` needs AVX-512F and IFMA; `ntt::avx512::tests::avx512_matches_portable` needs AVX-512F) | Compile only for x86-64. Print `skipped:` and pass on CPUs without the feature. `BASE58_REQUIRE_IFMA=1` changes the skip into a failure. No native-execution CI job sets the variable, and GitHub-hosted x86-64 runners do not guarantee AVX-512, so a CI pass does not show that the kernels ran. |
| AVX2 and VBMI kernel tests | Print `no AVX2 on this CPU` or `no AVX-512 VBMI on this CPU` and pass without the feature. On Apple silicon, Rosetta 2 reports no AVX or AVX2 to x86-64 processes, so these kernels do not run under Rosetta. |
| NEON kernels | Run natively only on arm64 hosts. x86-64 runs do not establish NEON coverage. The Miri jobs interpret selected NEON paths. |
| `compact` | Changes only the x86-64 IFMA kernels. |
| Native sanitizers | The memory lane needs x86-64 Linux; the driver refuses other targets. The thread lane also accepts `--target aarch64-apple-darwin`. |
| 32-bit tests | i686 test binaries run on x86-64 Linux with 32-bit C libraries (`gcc-multilib`). |
| Big-endian tests | Need Linux, a container engine for `cross`, and QEMU `binfmt_misc` with the `F` flag so that cold-cache tests can start child processes. |
| Miri | Interprets the `--target` that it receives, so the host can differ from the target. |
| Clang 23 action | Accepts only the Ubuntu releases `noble` and `resolute`. |
| Kani and custom Miri drivers | Refuse to start with less than 2 GiB of free disk. `kani_guard.py` also needs `z3` on `PATH`. |
| SIMD inlining gate | Needs an unstripped library and an `nm` whose `-C` demangles Rust symbols. It reads symbols only: slow code inside an inlined body passes. |

## Verification workflow

[verification.yml](.github/workflows/verification.yml) runs on every pull request,
on each push to `main` and on manual dispatch. A new run for the same ref cancels
the run in progress. Three dispatch inputs enable jobs that are off by default:

| Input | Job | Expected result |
| --- | --- | --- |
| `experimental_proofs` | `proofs_open` | Fails while the obligations stay open |
| `extended_miri` | `miri_unsupported_simd` | Fails while the stock Miri shims are missing |
| `run_fuzz` | `fuzz` | Passes |

`proptest_coverage` does not run for pull requests from forks. It and
`fuzz_coverage` of the [fuzz corpus workflow](#fuzz-corpus-workflow) upload to
Codecov with an OIDC token. Jobs that upload records keep them as artifacts for
14 days.

| Job | Checks | Platforms | Trigger |
| --- | --- | --- | --- |
| `tests` | Unit, integration, property and documentation tests; public API without `__internals`; Base58Check; no-allocation lane; downstream API namespace; benchmark builds | x86-64 and arm64 Linux; debug and release; cache off and on | All |
| `big_endian` | Unit, integration, property and documentation tests on PowerPC64 | x86-64 Linux, cross/QEMU; cache off and on | All |
| `compact` | Tests with `compact` | x86-64 Linux; cache off and on | All |
| `portability` | Feature-powerset builds for Wasm, AArch64 Linux, Cortex-M and bare x86-64; library Clippy for Wasm and all-target Clippy for AArch64 Linux; Wasm ABI and counted-allocation contracts | x86-64 Linux | All |
| `wasm_simd128` | Clippy, the library unit tests and the public radix boundaries with `+simd128`, run under wasmtime | x86-64 Linux, `wasm32-wasip1` target | All |
| `maximum_transform` | The ignored `2^23`-point transform test | x86-64 Linux | All |
| `msrv` | Feature-powerset build and public tests on Rust 1.95 | x86-64 Linux | All |
| `pointer_width_32` | Tests on i686 | x86-64 Linux | All |
| `lint` | Script tests, public test lists, rustfmt, Clippy, rustdoc warnings, package contents | x86-64 Linux | All |
| `simd_inlining` | Compares every out-of-line crate function of the Auto consumer library with `scripts/check_simd_inlining.txt` | x86-64 Linux; x86-64 and arm64 targets, each default, `cache` and `compact` | All |
| `measurement_tools` | `resource-profile` tests and self-checks; script self-tests; documentation check | x86-64 Linux; cache off and on | All |
| `miri_supported` | Stock Miri on portable paths and AVX2 kernels | x86-64 Linux | All |
| `miri_arm_panel` | Stock Miri on a NEON panel and forced encoder leaves | x86-64 Linux, AArch64 target | All |
| `miri_custom` | Development Miri on all SIMD probes, kernels and buffer cases | x86-64 Linux, AArch64 and x86-64 targets; cache off and on; separate x86-64 `compact` rows | All |
| `miri_unsupported_simd` | Stock Miri on every unsupported SIMD probe | x86-64 Linux, AArch64 and x86-64 targets | Dispatch, `extended_miri` |
| `fuzz` | Careful ASan and LSan campaigns | x86-64 Linux | Dispatch, `run_fuzz` |
| `proptest_coverage` | Runs unit and integration suites, including properties, builders, cache diagnostics and no-allocation tests; uploads coverage | x86-64 and arm64 Linux, generic CPU, cache off and on; x86-64 native CPU with cache and `compact` | Not fork pull requests |
| `native_sanitizers` | MemorySanitizer and ThreadSanitizer on selected tests | x86-64 Linux | All |
| `proofs` | Required Kani harnesses | x86-64 Linux | All |
| `proofs_open` | Open Kani obligations | x86-64 Linux | Dispatch, `experimental_proofs` |

### Test jobs

These jobs need the pinned stable Rust and cargo-nextest. CI sets `PROPTEST_CASES=512`
(128 in `big_endian`). The
dev profile uses opt-level 3 and keeps overflow checks and debug assertions.

`tests` runs these commands in its debug, cache-off rows:

```sh
export PROPTEST_CASES=512
cargo nextest run --locked --features __internals --lib --tests
cargo nextest run --locked --test api --test builders --test no_alloc --test tiny_inputs --test public_properties --test bitcoin_core --test owned_output --test radix_boundaries
cargo test --locked --doc
cargo nextest run --locked --test builders --test no_alloc --features check
cargo nextest run --locked --no-default-features --features check --test no_alloc --test tiny_inputs --test public_properties --test bitcoin_core
cargo test --locked --doc --features check
cargo build --locked --features __internals,check --benches
python3 scripts/check_public_api.py --output NEW_DIR
```

Release rows add `--release`. Cache rows use `__internals,cache` and add
`--features cache` to the public lists and documentation tests; they skip the
no-allocation lane. On x86-64 debug rows, the first suite uses `cargo test`, so
the tests of one binary share a process:

```sh
cargo test --locked --features __internals --lib --tests
```

Arm64 release rows also run the ignored 16 MiB radix boundaries:

```sh
cargo test --locked --release --test radix_boundaries -- --ignored --exact full_transform_limit_and_fallback_match_malachite
```

`check_public_api.py` builds seven feature profiles and compiles downstream
fixtures against each. It runs only on debug, cache-off rows. Its `--output`
directory must not exist.

The other test jobs differ in features, toolchain or target:

```sh
cargo nextest run --locked --features __internals,compact --lib --tests
cargo +1.95 hack build --locked --lib --feature-powerset
cargo +1.95 nextest run --locked --test api --test builders --test no_alloc --test tiny_inputs --test public_properties --test bitcoin_core --test owned_output --test radix_boundaries --features cache,compact
cargo nextest run --locked --target i686-unknown-linux-gnu --features __internals --lib --tests
cargo test --locked --release --lib ntt::tests::largest_transforms -- --ignored --exact
```

- `compact` also runs release, cache, public-list, documentation and benchmark-build
  variants.
- `msrv` runs the public list only with `cache,compact`; the powerset build covers
  the other feature sets at compile time.
- `pointer_width_32` also runs with `cache`. It needs the i686 target and `gcc-multilib`.
- `maximum_transform` requires the summary `1 passed; 0 failed; 0 ignored`.

`wasm_simd128` builds the Wasm SIMD128 kernels, which exist only with `+simd128`,
and runs the library unit tests and the public radix boundaries (both cache
policies) under wasmtime. WASI preview 1 cannot spawn threads
or processes, so the job skips the tests that need them; `should_panic` tests are
ignored there because Wasm panics abort. Install wasmtime 50.0.1, then:

```sh
rustup target add wasm32-wasip1
export RUSTFLAGS="-C target-feature=+simd128" CARGO_TARGET_WASM32_WASIP1_RUNNER=wasmtime
cargo clippy --locked --lib --target wasm32-wasip1 --features cache -- -D warnings
cargo test --locked --target wasm32-wasip1 --features __internals,cache --lib -- --skip threads --skip invalid_input_builds_no_table --skip retention_is_bounded_across_histories
cargo test --locked --target wasm32-wasip1 --test radix_boundaries
cargo test --locked --target wasm32-wasip1 --test radix_boundaries --features cache
```

Unset `RUSTFLAGS` to run the same tests on scalar Wasm.

`big_endian` installs the pinned cross revision and runs the suite in a QEMU
container. On Ubuntu, first install `qemu-user-binfmt` and register `qemu-ppc64`
with the `F` flag, as the workflow step "Enable BE child processes" does. To use
the pinned image, also set `CROSS_TARGET_POWERPC64_UNKNOWN_LINUX_GNU_IMAGE` to the
digest in the workflow. CI also runs the suite with `__internals,cache`.

```sh
cargo install --locked --git https://github.com/cross-rs/cross --rev 74e99b2223de06166f8cdcc1af7f1308ec52baf4 cross
PROPTEST_CASES=128 CROSS_BUILD_ENV_PASSTHROUGH=PROPTEST_CASES cross test --locked --target powerpc64-unknown-linux-gnu --features __internals
```

### Build, lint and tool jobs

`portability` builds every feature combination in debug and release. Clippy runs
on the library for Wasm and on all targets for AArch64 Linux, which lints the
AArch64-only tests that `lint` on x86-64 does not compile. Criterion's `alloca`
dependency compiles C, so the AArch64 Clippy needs `gcc-aarch64-linux-gnu`. The
no-`std` targets exclude `std` and `cache`:

```sh
cargo hack build --locked --lib --feature-powerset --target wasm32-unknown-unknown
cargo hack clippy --locked --lib --feature-powerset --target wasm32-unknown-unknown -- -D warnings
cargo hack build --locked --lib --feature-powerset --exclude-features std,cache --target thumbv7em-none-eabi
```

Repeat with `--release`, with `aarch64-unknown-linux-gnu` (build, and Clippy with
`--all-targets` in place of `--lib`) and with `x86_64-unknown-none` (build without
`std,cache`). The job then builds Wasm
probes, checks their ABI and checks counted allocations:

```sh
python3 scripts/code_size.py --source . --output target/measurements/wasm-abi --target wasm32-unknown-unknown --cpu generic --kinds cdylib --backends auto,bs58,five8,matrix-forced,ntt-same-owned,ntt-same-cached
node scripts/code_size_wasm.mjs target/measurements/wasm-abi/*.wasm.stripped
python3 scripts/code_size.py --source . --output target/measurements/wasm-counted --target wasm32-unknown-unknown --cpu generic --backends empty,auto,no-alloc --directions both --kinds cdylib --count-allocations
node scripts/wasm_allocations.mjs --self-test target/measurements/wasm-counted
```

CI repeats the counted build with `--cache`. It also checks that
`public_bench.py --wasm` refuses a counted module with
`timing requires uninstrumented modules`. `--strip-tool` defaults to
`llvm-strip` on `PATH`; CI passes the `llvm-strip` of the pinned stable
`llvm-tools` component. Each `--output` directory must not exist.

`lint` runs on x86-64 Linux:

```sh
python3 scripts/check_ci_tests.py
python3 scripts/test_process_supervisor.py
python3 scripts/test_kani_guard.py
python3 -B -m unittest discover -s scripts -p test_custom_miri.py -v
cargo fmt --all -- --check
cargo fmt --manifest-path tools/resource-profile/Cargo.toml --all -- --check
cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check
cargo hack clippy --locked --all-targets --feature-powerset -- -D warnings
cargo hack clippy --locked --all-targets --feature-powerset --manifest-path tools/resource-profile/Cargo.toml -- -D warnings
cargo hack clippy --locked --all-targets --feature-powerset --manifest-path fuzz/Cargo.toml -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps --all-features
cargo package --locked
python3 scripts/check_package.py target/package
```

`cargo package` refuses uncommitted changes unless you add `--allow-dirty`.

`simd_inlining` builds the Auto consumer library with generic CPU flags for x86-64
and arm64 Linux, each default, `cache` and `compact`, and compares every out-of-line
function whose name mentions the crate or a `core::arch` intrinsic with
[check_simd_inlining.txt](scripts/check_simd_inlining.txt). It fails on an unlisted
function (`+`) and on a listed function that is no longer out of line (`-`); fix the
code or update the list in the same commit. On x86-64 or arm64 Linux, with
`gcc-ARCH-linux-gnu` for the other architecture and its Rust target:

```sh
python3 scripts/check_simd_inlining.py --build target/measurements/simd
```

On Apple silicon Macs, use an arm64 Linux container that cross-compiles x86-64:

```sh
docker run --rm -v "$PWD":/src:ro -w /src ubuntu:26.04 sh -euc 'apt-get update -qq; apt-get install -y -qq rustup gcc gcc-x86-64-linux-gnu python3 ca-certificates git > /dev/null; rustup target add x86_64-unknown-linux-gnu; python3 -B scripts/check_simd_inlining.py --build /tmp/simd'
```

`check_simd_inlining.py CONFIG=LIBRARY`
checks libraries built elsewhere (`--nm` selects the `nm`).
`check_simd_inlining.py --list LIBRARY` prints the out-of-line crate functions with
their sizes, and `--compare OLD NEW` lists what a change inlined, outlined or
resized. On macOS, pass the unstripped `.dylib` and `--nm` with an `llvm-nm`.

`measurement_tools` tests the [resource-profile](tools/resource-profile/Cargo.toml)
worker and runs its `self-check` command:

```sh
cargo nextest run --locked --release --manifest-path tools/resource-profile/Cargo.toml
cargo run --locked --release --manifest-path tools/resource-profile/Cargo.toml -- self-check
```

CI repeats both with `--features count-allocations`, `compact` and
`compact,count-allocations`. A second matrix row adds `--features cache` to each
command. The cache-off row also runs the self-tests of `resource_report.py`,
`code_size.py`, `check_simd_inlining.py`, `run_native_sanitizer.py`,
`check_ci_tests.py` and `check_public_api.py`, the graph tests, the documentation
check and the syntax
checks. [Checks without a Rust build](#checks-without-a-rust-build) lists them
together with the other checks that need no Rust build.

### Miri jobs

The stock jobs need the pinned nightly with the `miri` component. CI requires an
exact pass and ignore count for each command; the `grep` patterns in the workflow
give the counts.

`miri_supported` interprets x86-64 with AVX2:

```sh
export RUSTFLAGS='-C target-cpu=x86-64 -C target-feature=+avx2'
export MIRIFLAGS=-Zmiri-strict-provenance
cargo +"$nightly" miri setup --target x86_64-unknown-linux-gnu
cargo +"$nightly" miri test --target x86_64-unknown-linux-gnu --locked --features __internals --test miri
cargo +"$nightly" miri test --target x86_64-unknown-linux-gnu --locked --test tiny_inputs
cargo +"$nightly" miri test --target x86_64-unknown-linux-gnu --locked --lib miri_tests::
cargo +"$nightly" miri test --target x86_64-unknown-linux-gnu --locked --lib -- matrix::x86::tests::kernels_match_portable matrix::x86::tests::parse_matches_portable --exact
MIRIFLAGS='-Zmiri-strict-provenance -Zmiri-tree-borrows -Zmiri-seed=1' cargo +"$nightly" miri test --target x86_64-unknown-linux-gnu --locked --lib miri_tests::
```

`miri_arm_panel` interprets generic AArch64. CI also installs `gcc-aarch64-linux-gnu`.

```sh
export RUSTFLAGS='-C target-cpu=generic'
export MIRIFLAGS=-Zmiri-strict-provenance
cargo +"$nightly" miri setup --target aarch64-unknown-linux-gnu
cargo +"$nightly" miri test --target aarch64-unknown-linux-gnu --locked --lib matrix::neon::tests::panel_matches_portable -- --exact
cargo +"$nightly" miri test --target aarch64-unknown-linux-gnu --locked --features __internals,malachite-nz/32_bit_limbs --test miri forced_encoder_leaves -- --exact
```

`miri_custom` builds the [development interpreter](ci/miri/README.md) and runs
the exact selection in [coverage.json](ci/miri/coverage.json). The workflow
installs `rustup-toolchain-install-master` 1.12.2 and the pinned compiler into an
isolated Rustup home, then runs the two scripts below. `MIRI_CHECKOUT` is a clean
checkout of the pinned Miri commit. `COMPILER_SYSROOT` is the sysroot of the
pinned compiler (`rustc --print sysroot`).
`run_custom_miri.py` needs an output directory outside the crate; both output
directories must not exist.

```sh
python3 scripts/build_custom_miri.py --source MIRI_CHECKOUT --compiler COMPILER_SYSROOT --cargo "$(rustup which --toolchain "$(jq -r .cargo_toolchain ci/miri/pins.json)" cargo)" --output NEW_BUILD_DIR
python3 scripts/run_custom_miri.py --build-record NEW_BUILD_DIR/build.json --source . --output NEW_RUN_DIR --arch x86 --alias-model tree --seed 1 --suite all
```

The CI matrix uses `--arch arm` and `--arch x86`, each with and without `--cache`.
Each row then runs `--suite kernels` with `--alias-model stacked --seed 7`.
Separate x86 rows, with and without `--cache`, run only the kernels with
`--compact` under both alias models; the first of these builds the sysroot.
`build_custom_miri.py --prepare-only` validates the source and patch without a
build. `run_custom_miri.py --plan-only` writes the planned commands to `run.json`
without running them.

`miri_unsupported_simd` runs every stock-Miri probe that lacks a shim. A nonzero
exit is expected:

```sh
sh scripts/miri-extended.sh arm
sh scripts/miri-extended.sh x86
```

`BASE58_MIRI_TOOLCHAIN`, `BASE58_CARGO_MIRI`, `BASE58_MIRI_LOG_DIR` and
`MIRIFLAGS` override the toolchain, the `cargo-miri` binary, the log directory and
the Miri flags.

### Fuzzing and coverage jobs

The fuzz jobs need the pinned nightly with `rust-src`, cargo-fuzz 0.13.2 and, on
Linux, Clang 23. [fuzz/README.md](fuzz/README.md) has the campaign, replay and
crash-replay commands.

`fuzz` builds with `--careful --build-std --no-default-features`. Its rows use
ASan with default, `cache`, `compact` and `cache,compact` features, and LSan with
default and `cache` features. CI sets `CXXFLAGS`, `ASAN_OPTIONS` and `LSAN_OPTIONS`
in the job `env:`. Each mutation stage runs for 180 seconds. The full-width stages
require actual mutation attempts, which `fuzz/check_mutations.py` checks. The job
also compiles and runs the unit tests of the full-width helper:

```sh
mkdir -p target
rustc +"$nightly" --edition=2024 --test fuzz/full_width.rs -o target/full-width-tests
target/full-width-tests
```

`proptest_coverage` exports LCOV for all library unit tests and integration
suites, with `__internals` and `check`. This includes direct SIMD kernel tests,
builder and error contracts, and the separate cold-cache/preflight binaries.
CI adds `RUSTFLAGS=-C target-cpu=generic` or `native`, and `cache` in three of
the five rows. The native x86 row also enables `compact`. Each row exports a
second report for the public no-allocation suites with only `check` enabled.

```sh
export PROPTEST_CASES=512
cargo llvm-cov --locked --features __internals,check --lib --tests --lcov --output-path proptests.lcov --ignore-filename-regex '/(registry|rustc|rustlib)/|/fuzz/|/tests/|/ntt/internals/diagnostics\.rs$'
cargo llvm-cov --locked --no-default-features --features check --test no_alloc --test tiny_inputs --test public_properties --test bitcoin_core --lcov --output-path no-alloc.lcov --ignore-filename-regex '/(registry|rustc|rustlib)/|/fuzz/|/tests/'
```

Keep both reports outside the coverage target directory: the second command
cleans the first command's build artifacts. Both LCOV files are uploaded together
with the `tests` flag. The fuzz reports use the `fuzz` flag.
[codecov.yml](codecov.yml) waits for five `tests` uploads and twelve `fuzz` uploads.

Runtime feature detection still selects instructions supported by the host;
`target-cpu=generic` does not force a scalar or SSE4.1 fallback. Direct kernel
unit tests call each supported implementation even when dispatch prefers a newer
ISA. Unsupported ISA tests and ignored large-boundary tests remain outside these
reports. Compile-time-only functions and unused generic instantiations can also
have zero runtime counts. The file filter excludes external code, test-support
files and the measurement-only NTT diagnostics module; it does not exclude
Matrix or NTT table cache diagnostics.

### Native sanitizers

`native_sanitizers` runs [run_native_sanitizer.py](scripts/run_native_sanitizer.py)
in five rows: thread with cache on, and memory with cache off and on, each with and
without `--compact`. It needs the pinned nightly with `rust-src` and Clang 23.

```sh
python3 scripts/run_native_sanitizer.py memory --cache off
python3 scripts/run_native_sanitizer.py thread --cache on --log-dir target/native-sanitizers/thread-on/logs
```

The driver rebuilds `std` with the sanitizer. It lists the tests first, then runs
each selected unit test and integration binary as a separate command. It requires
the exact nonzero pass count and writes `complete.json` to the log directory. The
default log directory, `target/native-sanitizers/<sanitizer>/logs`, must not
exist; give each further run a new `--log-dir`. `--require-ifma` sets
`BASE58_REQUIRE_IFMA=1`. The CI job does not pass it, so on a runner without
AVX-512 IFMA the memory rows skip `avx512_matches_reference`, pass, and record
`pass_with_isa_skips` in `complete.json`.

### Proofs

`proofs` needs Kani 0.68.0 and `z3`. Each command runs under
[kani_guard.py](scripts/kani_guard.py), which stops Z3 when its verifier exits:

```sh
python3 scripts/kani_guard.py -- sh scripts/kani-production.sh
```

`kani-production.sh` covers only the tiny public conversion and alphabet
obligations. The workflow runs five more `cargo kani` selections. They use
different solvers, timeouts and `CARGO_PROFILE_DEV_OPT_LEVEL` values; the dev
profile otherwise uses opt-level 3. Copy each command with its step `env:`. One
step passes `--cbmc-args --external-smt2-solver scripts/kani-z3-polynomial.sh`.

`proofs_open` runs the open obligations. A nonzero exit is expected:

```sh
python3 scripts/kani_guard.py -- sh scripts/kani-open.sh
```

## Fuzz corpus workflow

[fuzz-corpus.yml](.github/workflows/fuzz-corpus.yml) runs on every pull request,
on each push to `main`, daily at 02:17 UTC and on manual dispatch. Only pull-request
runs cancel a run in progress. Dispatch inputs: `run_nightly` (boolean),
`publish` (boolean) and `fuzz_seconds` (seconds per target, default 300).
[The corpus guide](docs/fuzzing-corpus.md) describes the branch and trust model.

| Job | Checks | Platforms | Trigger |
| --- | --- | --- | --- |
| `corpus_snapshot` | Validates the head of `fuzz-corpus` and pins its revision | x86-64 Linux | All |
| `fuzz_coverage` | Replays seeds and the pinned corpus with ASan, without mutation, and exports source coverage; one job per row and target; uploads to Codecov on pushes and same-repository pull requests | x86-64 Linux, generic CPU, cache off and on; arm64 Linux, generic CPU, cache on; each of the four targets | All |
| `nightly` | Mutation campaigns seeded by the run ID, with debug assertions and overflow checks, and value profile in the cache rows; writes provenance and uploads the campaign | x86-64 and arm64 Linux, generic CPU, cache off and on | Schedule; dispatch with `run_nightly` |
| `minimize` | Keeps the seeds and the pinned corpus, and merges in the campaign inputs that add coverage beyond them in its build | Same as `nightly` | Schedule; dispatch with `run_nightly` |
| `candidate_smoke` | Replays the seeds and the union of the four merged sets with ASan | Same rows as `fuzz_coverage` | Schedule; dispatch with `run_nightly` |
| `publish` | Replaces `corpora/` with that union, which holds every stored input, and adds the provenance to `fuzz-corpus`; needs `fuzz_coverage`, `minimize` and `candidate_smoke` | x86-64 Linux | Schedule; dispatch with `run_nightly` and `publish` |

Nightly mutation starts from all deterministic seeds plus at most 16 stored
inputs per target. Selection ranks SHA-256 hashes of the run ID, target and
content digest, so another run samples different stored inputs reproducibly.
The full store still participates in coverage replay, merging and candidate
replay. This bounds mutation startup as the store grows: libFuzzer checks
`max_total_time` only after initial corpus loading. Each target's log must pass
`check_mutations.py`; a run that only loads inputs fails instead of publishing
an empty campaign. Counts appear in the Actions log. Provenance records the
stored-input limit, selection method and oracle instrumentation setting.
Nightly builds also use the oracle wrapper described below; the crate and
harness keep their fuzz instrumentation and debug checks, and ASan stays on.

`fuzz_coverage` runs `cargo fuzz coverage --sanitizer address` with
`--debug-assertions` and the careful `RUSTFLAGS` of the `fuzz` job, on prebuilt
std. A crash, ASan report or oracle mismatch fails the job. `RUSTC_WRAPPER` is
[`fuzz/oracle_rustc.py`](fuzz/oracle_rustc.py), which builds the Malachite and
bs58 oracles without coverage, sanitizer-coverage or debug checks; ASan stays.
`candidate_smoke` uses `fuzz run --sanitizer address --debug-assertions` without
`--careful`; `minimize` uses `--sanitizer none`. The replay and merge builds use
`--no-trace-compares`: compare tracing only guides mutation, and it takes most
of the replay time. `nightly` keeps it. Every `cargo fuzz` command passes
`--verbose`, so logs show each `rustc` call. The coverage step needs the nightly
`llvm-tools-preview` component. To replay the pinned corpus for one target as
an x86-64 `fuzz_coverage` row does:

```sh
python3 fuzz/seed_corpus.py
python3 fuzz/corpus_store.py validate CORPUS_ROOT
cp -a CORPUS_ROOT/. fuzz/corpus/
export RUSTFLAGS='-Zextra-const-ub-checks -Zstrict-init-checks --cfg careful -C target-cpu=generic'
RUSTC_WRAPPER="$PWD/fuzz/oracle_rustc.py" cargo +"$nightly" fuzz coverage --sanitizer address --no-trace-compares --debug-assertions --strip-dead-code=false --target x86_64-unknown-linux-gnu --target-dir target/coverage --no-default-features --features full-width encode_all fuzz/corpus/encode_all -- -timeout=120 -rss_limit_mb=4096
```

`fuzz coverage` replays batches of at least 100 inputs in one process per CPU.
The workflow gives the `llvm-cov report` and `llvm-cov export` commands and the
file filter. `CORPUS_ROOT` is the `corpora` directory of a `fuzz-corpus` checkout.

`python3 fuzz/corpus_store.py replace INCOMING... CORPUS_ROOT --exclude fuzz/corpus`
writes the union of the input roots as a fresh store under SHA-256 names, without
the seeds; `stage INCOMING CORPUS_ROOT` only adds. To add one campaign's new
coverage for one target in one build, as a `minimize` row does, with `CAMPAIGN`
a campaign directory:

```sh
python3 fuzz/seed_corpus.py
python3 fuzz/corpus_store.py replace CAMPAIGN UNION --exclude CORPUS_ROOT
mkdir -p MINIMIZED
cp -a fuzz/corpus/encode_all MINIMIZED/
cp -a CORPUS_ROOT/encode_all/. MINIMIZED/encode_all/
cargo +"$nightly" fuzz run --sanitizer none --no-trace-compares encode_all MINIMIZED/encode_all UNION/encode_all -- -merge=1 -max_len=65537 -timeout=120 -rss_limit_mb=4096
```

The output must contain `MERGE-OUTER: successful in 1 attempt(s)`; a later
attempt means that an input failed. `MINIMIZED` then holds the seeds, the stored
inputs and the selected new inputs; `replace` with `--exclude fuzz/corpus`
removes the seeds again. Use `--exclude fuzz/corpus` only while `fuzz/corpus`
holds just the seeds: after a replay copy, it would also exclude the copied
corpus inputs. To shrink the store instead, form `UNION` from `CORPUS_ROOT` and
the campaigns and leave the stored inputs out of `MINIMIZED`, in every build that
[fuzz/README.md](fuzz/README.md) lists.

## Other workflows

| Workflow | Job | Trigger | Platforms | What it does |
| --- | --- | --- | --- | --- |
| [cache-cargo-sources.yml](.github/workflows/cache-cargo-sources.yml) | `warm` | Push to `main` | x86-64 and arm64 Linux | Fetches the sources of all three lockfiles and saves them in the Actions cache |
| [miri-nextest-benchmark.yml](.github/workflows/miri-nextest-benchmark.yml) | `stock_miri` | Dispatch | x86-64 Linux | Times selected stock-Miri groups under `cargo miri test` and Nextest with two and four workers |
| [release-plz.yml](.github/workflows/release-plz.yml) | `release-plz-pr` | Push to `main`; dispatch | x86-64 Linux | Opens or updates the release pull request with `release-plz release-pr` |

Other jobs restore the source cache with the
[cache-cargo-sources](.github/actions/cache-cargo-sources/action.yml) action.
Jobs that read the mutable `fuzz-corpus` branch do not restore it. To fetch the
same sources locally:

```sh
cargo fetch --locked
cargo fetch --locked --manifest-path fuzz/Cargo.toml
cargo fetch --locked --manifest-path tools/resource-profile/Cargo.toml
```

The Nextest benchmark uses the stock-Miri groups selected in its workflow,
with `nextest run -j2` and `nextest run -j4` in place of `test`. The
[development Miri guide](ci/miri/README.md#nextest-comparison) explains how to
read the result.

`release-plz-pr` writes to GitHub with the repository token; it has no local
command. [release-plz.toml](release-plz.toml) enables semver checks. Pull requests
that this token opens do not start CI; close and reopen them to run the checks.

## Repository scripts

| Script | Purpose | Modes | CI job |
| --- | --- | --- | --- |
| [`scripts/check_ci_tests.py`](scripts/check_ci_tests.py) | Fails when a public integration test is missing from one of the four explicit test lists in `verification.yml` | `--self-test` | `lint`, `measurement_tools` |
| [`scripts/check_package.py`](scripts/check_package.py) | Checks the packaged `.crate`: allowed paths, required files, 2 MiB budget, license and feature metadata | `--self-test`; package directory (default `target/package`) | `lint` |
| [`scripts/check_public_api.py`](scripts/check_public_api.py) | Compiles downstream fixtures against seven feature profiles: normal API, `internals` namespace, removed paths, owned and checked methods | `--self-test` (needs `--output`, compiles nothing) | `tests`, `measurement_tools` (self-test) |
| [`scripts/check_simd_inlining.py`](scripts/check_simd_inlining.py) | Fails when the out-of-line crate functions of a library differ from [check_simd_inlining.txt](scripts/check_simd_inlining.txt); reads symbols only | `--build DIR`; `CONFIG=LIBRARY`; `--list LIBRARY`; `--compare OLD NEW`; `--self-test`; `--nm` | `simd_inlining`, `measurement_tools` (self-test) |
| [`scripts/code_size.py`](scripts/code_size.py) | Builds one probe per backend and direction and records linked sizes; `--count-allocations` instruments Wasm | `--self-test` | `portability`, `measurement_tools`, `simd_inlining` |
| [`scripts/code_size_wasm.mjs`](scripts/code_size_wasm.mjs) | Checks the Wasm probe ABI: allocator, pointers, UTF-8 and outputs | Wasm module paths | `portability` |
| [`scripts/wasm_allocations.mjs`](scripts/wasm_allocations.mjs) | Checks counted `empty`, `auto` and `no-alloc` Wasm modules: allocator counts and ABI | `--self-test DIR` only | `portability` |
| [`scripts/public_bench.py`](scripts/public_bench.py) | Times `code_size.py` probes and writes a public timing CSV; `--baseline` prints paired ratios | `--wasm`, `--self-test` | `portability` (refusal check), `measurement_tools` (self-test) |
| [`scripts/public_bench_wasm.mjs`](scripts/public_bench_wasm.mjs) | Wasm worker of `public_bench.py`; refuses counted modules | Started by `public_bench.py --wasm` | `measurement_tools` (syntax) |
| [`scripts/resource_report.py`](scripts/resource_report.py) | Builds and runs the internal heap and CPU workers of `tools/resource-profile` | `build`, `run`, `fingerprint`, `self-test` | `measurement_tools` |
| [`scripts/run_native_sanitizer.py`](scripts/run_native_sanitizer.py) | MemorySanitizer and ThreadSanitizer driver | `--self-test` | `native_sanitizers` |
| [`scripts/build_custom_miri.py`](scripts/build_custom_miri.py) | Builds the development Miri from the pinned source and patch | `--prepare-only` | `miri_custom` |
| [`scripts/run_custom_miri.py`](scripts/run_custom_miri.py) | Runs the exact custom-Miri selection, one process per test | `--plan-only` | `miri_custom` |
| [`scripts/miri-extended.sh`](scripts/miri-extended.sh) | Runs each unsupported SIMD probe under stock Miri | `arm`, `x86` | `miri_unsupported_simd` |
| [`scripts/kani_guard.py`](scripts/kani_guard.py) | Runs a command with a Z3 shim that stops Z3 when its verifier exits; keeps records in a new temporary directory | `-- COMMAND` | `proofs`, `proofs_open` |
| [`scripts/kani-production.sh`](scripts/kani-production.sh) | Required tiny-path and alphabet harnesses | Extra `cargo kani` arguments | `proofs` |
| [`scripts/kani-open.sh`](scripts/kani-open.sh) | Open obligations; exits nonzero | None | `proofs_open` |
| [`scripts/kani-z3-polynomial.sh`](scripts/kani-z3-polynomial.sh) | Z3 wrapper with rewriter options for the polynomial lemmas | Called by CBMC | `proofs` |
| [`scripts/process_supervisor.py`](scripts/process_supervisor.py) | Process-group ownership and signal handling for the drivers | Imported only | - |
| [`scripts/test_process_supervisor.py`](scripts/test_process_supervisor.py) | Tests `process_supervisor.py` with real child processes | - | `lint` |
| [`scripts/test_kani_guard.py`](scripts/test_kani_guard.py) | Tests `kani_guard.py` with a fake Z3 | - | `lint` |
| [`scripts/test_custom_miri.py`](scripts/test_custom_miri.py) | Tests the coverage guards of `run_custom_miri.py`; runs no Rust | - | `lint` |
| [`assets/graphs/render.py`](assets/graphs/render.py) | Checks local links in the maintained guides, the README figure references and SVG syntax; checks no numbers | `--check` (required) | `measurement_tools` |
| [`assets/graphs/render_current.py`](assets/graphs/render_current.py) | Renders public latency figures and tables from measurement CSVs into a new directory; needs Matplotlib | `--data`, `--sizes`, `--output` | - |
| [`assets/graphs/current.py`](assets/graphs/current.py) | Shared helpers of the two renderers | Imported only | - |
| [`assets/graphs/test_render.py`](assets/graphs/test_render.py) | Tests the link check and the table helpers | - | `measurement_tools` |
| [`fuzz/seed_corpus.py`](fuzz/seed_corpus.py) | Writes the deterministic seeds; fails if an existing seed differs | `--output DIR` (default `fuzz/corpus`) | Fuzz jobs |
| [`fuzz/corpus_store.py`](fuzz/corpus_store.py) | Validates, samples, adds inputs, or replaces a corpus with a union | `validate CORPUS`, `sample INCOMING CORPUS --limit N --seed SEED`, `stage INCOMING CORPUS`, `replace INCOMING... CORPUS [--exclude ROOT]` | Fuzz corpus jobs |
| [`fuzz/provenance.py`](fuzz/provenance.py) | Writes the JSON provenance of one campaign | `--output`, `--field KEY=VALUE`, `--digest KEY=PATH` | `nightly` |
| [`fuzz/check_mutations.py`](fuzz/check_mutations.py) | Requires a nonzero mutation-attempt count in a libFuzzer log | `--self-test`; `LOG --seconds N` | `fuzz`, `nightly` |
| [`fuzz/oracle_rustc.py`](fuzz/oracle_rustc.py) | `RUSTC_WRAPPER` that builds the Malachite and bs58 oracles without coverage, sanitizer-coverage or debug checks | Called by Cargo | `fuzz_coverage`, `nightly` |
| `fuzz/test_*.py` | Tests the seed generator, the corpus store, the provenance writer and the oracle wrapper | - | `fuzz`, `fuzz_coverage` |
| [`tools/code-size/`](tools/code-size/README.md) | Probe sources that `code_size.py` copies into a standalone package | - | Through `code_size.py` |
| [`tools/resource-profile/`](tools/resource-profile/Cargo.toml) | Worker of `resource_report.py`; features `count-allocations`, `cache`, `compact` | `self-check` | `measurement_tools` |
| [`ci/miri/`](ci/miri/README.md) | Pins (`pins.json`), exact selection (`coverage.json`) and patch (`shims.patch`) of the development Miri | Read by the custom-Miri scripts | `miri_custom` |

[docs/performance.md](docs/performance.md) gives the measurement commands for
`code_size.py`, `public_bench.py`, `render_current.py` and `resource_report.py`.

### Checks without a Rust build

These commands compile no Rust. They cover the script self-tests, the script unit
tests, the documentation check and the syntax checks. Run them after you change a
script or a guide. `check_public_api.py --self-test` needs a new `--output`
directory.

```sh
python3 -B scripts/check_ci_tests.py
python3 -B scripts/check_ci_tests.py --self-test
python3 -B scripts/check_package.py --self-test
python3 -B scripts/check_public_api.py --self-test --output NEW_DIR
python3 -B scripts/check_simd_inlining.py --self-test
python3 -B scripts/code_size.py --self-test
python3 -B scripts/resource_report.py self-test
python3 -B scripts/run_native_sanitizer.py --self-test
python3 -B scripts/test_process_supervisor.py
python3 -B scripts/test_kani_guard.py
python3 -B -m unittest discover -s scripts -p test_custom_miri.py -v
python3 -B -m unittest discover -s assets/graphs -p test_render.py
python3 -B assets/graphs/render.py --check
python3 -B -m unittest discover -s fuzz -p 'test_*.py'
python3 -B fuzz/check_mutations.py --self-test
python3 -B scripts/public_bench.py --self-test
node --check scripts/public_bench_wasm.mjs
node --check scripts/code_size_wasm.mjs
node --check scripts/wasm_allocations.mjs
```

# Development Miri interpreter

This development CI interpreter checks SIMD paths that the stock
`nightly-2026-10-26` Miri does not support. It does not enter the crate's build or
runtime dependencies. Keep its results separate from the stock Miri jobs.

`pins.json` binds the source revision, compiler, Cargo and reviewed patch. The
base is [elichai/miri at 52a3791566](https://github.com/elichai/miri/tree/52a3791566bfb72b99a59058527a3fcf6d38afa2).
The patch combines:

- The base commit already implements the original x86 shifts and permutations.
- Seven implementations from a maintainer patch:
  ARM rounded shift, two shift-and-insert forms, two table extensions, and two
  x86 IFMA operations. The original implementation snapshot hash is retained
  in `pins.json`.
- `llvm.aarch64.neon.shsub.v4i32`, needed by the NEON NTT. It uses
  widened signed subtraction and arithmetic right shift. Its tests cover signed
  extrema, negative odd differences and a 13 by 13 boundary grid. Native ARM
  execution agrees, and native assembly contains `shsub.4s`.

Miri and the patch are available under `MIT OR Apache-2.0`; copies of the Miri
licenses are included here. No upstream Miri submission is part of this change.

The builder accepts a clean checkout at the pinned revision, the exact compiler
sysroot, and the pinned Cargo executable. It copies only tracked source into a
fresh directory, verifies and applies the patch there, and builds the driver and
wrapper. It installs nothing and does not edit the supplied checkout or any
global toolchain. `--prepare-only` validates the source and patch without a build.
`--offline` keeps Cargo offline when its dependencies are already cached.

The matching compiler can be obtained in an isolated CI Rustup directory with
`rustup-toolchain-install-master` at the version in `pins.json`, requesting
`cargo`, `rust-src` and `rustc-dev` for the pinned compiler commit. Use Cargo from
the pinned ordinary nightly if that compiler artifact does not include Cargo.
The builder fails when either compiler or Cargo reports a different commit.

The interpreter build enables `stack-cache`, disables optional `native-lib`,
retains Miri's validation, and uses opt-level 1 with no debug info or incremental
compilation. This is sufficient for these Rust/SIMD checks; it makes no native
FFI or full-Miri-test-suite claim. `build.json` records commands, tool versions,
flags and artifact hashes. Free-space and deadline guards fail the build rather
than report a pass.

`coverage.json` names exact probes and actual kernel tests. The custom runner
uses strict provenance with an explicit alias model and seed. It rejects a
zero-test or skipped-test result. The stock diagnostic remains separate:
twelve intrinsic probes report unsupported operations, while three additional
permutation probes pass.

The buffer suite retains its complete original sequence. Fourteen additional
ignored tests expose the same twelve lengths and two patterns independently.
The custom runner checks that all fourteen names exist, then executes each with
`--ignored --exact` in its own interpreter process. This bounds interpreter
state while keeping every backend, input, buffer-capacity and canary check.
The full sequential test remains available to the stock suite and direct runs.

These checks cover concrete executions, not complete input domains or native
x86 behavior. A custom job must pass independently; it must not relabel the
stock interpreter's unsupported operations as supported.

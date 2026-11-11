#!/usr/bin/env python3
"""RUSTC_WRAPPER that compiles the reference oracles without fuzz instrumentation.

cargo-fuzz passes coverage, sanitizer-coverage and debug-check flags in RUSTFLAGS,
so they reach every crate. Cargo's profile `rustflags` cannot override them: Cargo
places profile flags before RUSTFLAGS. For the oracle crates this wrapper removes
`-Cinstrument-coverage`, `-Cpasses=sancov-module`, `-Cllvm-args=-sanitizer-coverage*`,
`-Cdebug-assertions` and `-Coverflow-checks`. Sanitizer flags stay: they must match
across crates. Coverage reports already exclude dependencies.

Usage, as Cargo calls it: oracle_rustc.py RUSTC ARGS...
"""
import os
import sys

ORACLE_CRATES = ("malachite_base", "malachite_nz", "bs58")
DROPPED = (
    "instrument-coverage",
    "passes=sancov-module",
    "llvm-args=-sanitizer-coverage",
    "debug-assertions",
    "overflow-checks",
)


def filter_args(args):
    """The rustc arguments for one crate, without fuzz instrumentation for oracles."""
    try:
        crate = args[args.index("--crate-name") + 1]
    except (ValueError, IndexError):
        return args
    if crate not in ORACLE_CRATES:
        return args
    out = []
    i = 0
    while i < len(args):
        if args[i] == "-C" and i + 1 < len(args):
            if not args[i + 1].startswith(DROPPED):
                out += args[i : i + 2]
            i += 2
            continue
        if not (args[i].startswith("-C") and args[i][2:].startswith(DROPPED)):
            out.append(args[i])
        i += 1
    return out


def main():
    rustc = sys.argv[1]
    os.execvp(rustc, [rustc, *filter_args(sys.argv[2:])])


if __name__ == "__main__":
    main()

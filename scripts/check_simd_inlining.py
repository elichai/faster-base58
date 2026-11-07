#!/usr/bin/env python3
"""Check that SIMD code stays inline, and list what a build keeps out of line.

Dispatch calls a `#[target_feature]` function, and everything it runs must be inlined
into it: a function left out of line has no target features, so each intrinsic in it
becomes a call. Pass unstripped libraries built with generic CPU flags, for example
`code_size.py --cpu generic --kinds cdylib` (`auto-both-cdylib.so`).

  check_simd_inlining.py LIBRARY...         fail on unexpected out-of-line SIMD code
  check_simd_inlining.py --list LIBRARY     every out-of-line crate function, with size
  check_simd_inlining.py --compare OLD NEW  what a change inlined or outlined

The check fails on a surviving `core::arch` intrinsic and on any out-of-line function
of a SIMD module that `EXPECTED` does not match. Symbols do not show slow code inside
an inlined body; time changes with docs/performance.md.
"""
import argparse
from collections import Counter
from fnmatch import fnmatchcase
import subprocess

CRATE = "faster_base58::"
SIMD_MODULES = tuple(CRATE + module for module in (
    "ntt::avx512::", "ntt::x86::", "ntt::neon::", "mul_ifma::",
    "encode::matrix::x86::", "encode::matrix::neon::", "matrix::x86::", "matrix::neon::"))
# CPU detection calls this once, outside any `xsave` function.
ALLOWED_INTRINSICS = {"core::core_arch::x86::xsave::_xgetbv"}
# What the SIMD modules keep out of line on purpose. Edit this list deliberately: a
# new entry is a new call on a SIMD path.
EXPECTED = (
    # NTT kernel tables: entries are called through pointers. The closures adapt the
    # `unsafe` target-feature functions to the tables' safe pointers.
    "faster_base58::ntt::avx512::kernels",
    "<faster_base58::ntt::*::kernels::{closure#*} as core::ops::function::FnOnce<*>>::call_once",
    "faster_base58::ntt::avx512::z::forward",
    "faster_base58::ntt::avx512::z::inverse",
    "faster_base58::ntt::avx512::z::pointwise",
    "faster_base58::ntt::avx512::z::square",
    "faster_base58::ntt::x86::kernels::tf_*",
    *(f"faster_base58::ntt::neon::{name}" for name in (
        "forward", "inverse", "pointwise", "square", "load", "load_add", "load_twist",
        "pair", "twist", "garner", "halves", "fold_sub")),
    # IFMA products: the entry point and the kernel widths it selects by pointer.
    "faster_base58::mul_ifma::avx512::mul_tf",
    "faster_base58::mul_ifma::avx512::kernel_tf::<*>",
    # Fixed 64-byte encoders, their column sums and the cold carry fallbacks.
    "faster_base58::encode::matrix::x86::encode_64",
    "faster_base58::encode::matrix::x86::encode_64_vbmi",
    "faster_base58::encode::matrix::x86::cols::<*>",
    "faster_base58::encode::matrix::x86::cols_512::<*>",
    "faster_base58::encode::matrix::x86::carry_cold",
    "faster_base58::encode::matrix::neon::carry_cold",
    # Matrix decoder panels and their runtime selection.
    "faster_base58::matrix::x86::panel",
    "faster_base58::matrix::x86::avx2",
    "faster_base58::matrix::x86::sse41",
    # AVX2 Matrix conversions; the panel kernel is inlined into them.
    "faster_base58::matrix::x86::convert_avx2",
    "faster_base58::encode::matrix::x86::convert_avx2",
)


def functions(nm_output):
    """(name, size) of each text symbol in `nm -n -C` output; size reaches the next one."""
    symbols = []
    for line in nm_output.splitlines():
        fields = line.split(maxsplit=2)
        if len(fields) == 3 and fields[1] in "tT":
            symbols.append((int(fields[0], 16), fields[2].removeprefix("_")))
    symbols.sort()
    ends = [address for address, _ in symbols[1:]] + [None]
    return [(name, end - address if end is not None else None)
            for (address, name), end in zip(symbols, ends)]


def expected(name):
    return any(fnmatchcase(name, pattern) for pattern in EXPECTED)


def reported(name):
    """Whether `--list` and `--compare` show a function: crate code or an intrinsic."""
    return CRATE in name or "core::core_arch::" in name


def unexpected(names):
    """Functions that show SIMD code outside its target-feature caller."""
    return sorted({name for name in names
                   if ("core::core_arch::" in name and name not in ALLOWED_INTRINSICS)
                   or (any(module in name for module in SIMD_MODULES) and not expected(name))})


def compare(old, new):
    """Reported functions only in `old`, only in `new`, and (name, old, new) size changes."""
    def table(funcs):
        sizes = Counter()
        for name, size in funcs:
            if reported(name):
                sizes[name] += size or 0
        return sizes
    a, b = table(old), table(new)
    changed = [(name, a[name], b[name]) for name in sorted(a.keys() & b.keys()) if a[name] != b[name]]
    return sorted(a.keys() - b.keys()), sorted(b.keys() - a.keys()), changed


def read(nm, library):
    funcs = functions(subprocess.run([nm, "-n", "-C", library], capture_output=True, text=True,
                                     check=True).stdout)
    if not any(name.startswith(CRATE) for name, _ in funcs):
        raise SystemExit(f"{library}: no faster_base58 symbols; pass the unstripped library")
    return funcs


def self_test():
    nm = "\n".join([
        "0000000000001000 t _core::core_arch::x86::xsave::_xgetbv",
        "0000000000001040 t core::core_arch::x86::avx512f::_mm512_mullo_epi32",
        "0000000000001080 t faster_base58::ntt::avx512::z::forward",
        "0000000000001400 t faster_base58::ntt::neon::mont",
        "0000000000001480 t <faster_base58::ntt::x86::kernels::{closure#3} as core::ops::function::FnOnce<(&mut [i32],)>>::call_once",
        "00000000000014a0 t <&mut core::array::drain::Drain<usize, faster_base58::ntt::avx512::z::Z> as core::ops::function::FnMut<(usize,)>>::call_mut",
        "0000000000001500 t faster_base58::mul_ifma::avx512::kernel_tf::<12>",
        "0000000000001600 T faster_base58_encode",
        "                 U memcpy",
        "0000000000001700 d core::core_arch::x86::avx512f::CONSTANT",
    ])
    funcs = functions(nm)
    assert funcs[0] == ("core::core_arch::x86::xsave::_xgetbv", 0x40), funcs
    assert funcs[-1] == ("faster_base58_encode", None), funcs
    assert unexpected(name for name, _ in funcs) == [
        "<&mut core::array::drain::Drain<usize, faster_base58::ntt::avx512::z::Z> as core::ops::function::FnMut<(usize,)>>::call_mut",
        "core::core_arch::x86::avx512f::_mm512_mullo_epi32",
        "faster_base58::ntt::neon::mont",
    ], unexpected(name for name, _ in funcs)
    old = [("faster_base58::a", 10), ("faster_base58::b", 20), ("core::x", 5),
           ("core::core_arch::x86::avx2::_mm256_add_epi32", 16)]
    new = [("faster_base58::b", 30), ("faster_base58::c", 40)]
    assert compare(old, new) == (["core::core_arch::x86::avx2::_mm256_add_epi32", "faster_base58::a"],
                                 ["faster_base58::c"], [("faster_base58::b", 20, 30)]), compare(old, new)
    print("check_simd_inlining self-test passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("libraries", nargs="*")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--list", action="store_true", help="list one library's out-of-line crate functions")
    mode.add_argument("--compare", action="store_true", help="compare two libraries: OLD NEW")
    mode.add_argument("--self-test", action="store_true")
    parser.add_argument("--nm", default="nm", help="an nm that demangles Rust v0 names with -C")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.list:
        if len(args.libraries) != 1:
            parser.error("--list takes one library")
        funcs = read(args.nm, args.libraries[0])
        bad = set(unexpected(name for name, _ in funcs))
        for name, size in sorted(f for f in funcs if reported(f[0])):
            tag = "UNEXPECTED" if name in bad else "simd" if expected(name) else ""
            print(f"{size if size is not None else '?':>8} {tag:10} {name}")
        for pattern in EXPECTED:
            modules = [module for module in SIMD_MODULES if module in pattern]
            present = not modules or any(modules[0] in name for name, _ in funcs)
            if present and not any(fnmatchcase(name, pattern) for name, _ in funcs):
                print(f"expected but absent: {pattern}")
        return None
    if args.compare:
        if len(args.libraries) != 2:
            parser.error("--compare takes OLD NEW")
        gone, outlined, changed = compare(*(read(args.nm, library) for library in args.libraries))
        for name in gone:
            print(f"inlined or removed: {name}")
        for name in outlined:
            print(f"newly out of line: {name}")
        for name, old_size, new_size in changed:
            print(f"size {old_size:>7} -> {new_size:<7} {name}")
        return None
    if not args.libraries:
        parser.error("pass at least one library")
    failed = False
    for library in args.libraries:
        names = [name for name, _ in read(args.nm, library)]
        bad = unexpected(names)
        for name in bad:
            print(f"{library}: unexpected out-of-line: {name}")
        failed |= bool(bad)
        print(f"{library}: {len(names)} functions, {len(bad)} unexpected")
    if failed:
        raise SystemExit("SIMD code left out of line: inline it, or add it to EXPECTED deliberately")
    return None


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Check which functions the Auto consumer library keeps out of line.

Dispatch calls a `#[target_feature]` function, and everything it runs must be inlined
into it: a function left out of line has no target features, so each intrinsic in it
becomes a call. Outside SIMD code, LLVM's inlining decisions set call overhead and code
size. `check_simd_inlining.txt` lists, for each configuration, every out-of-line
function whose name mentions the crate or a `core::arch` intrinsic. The check fails on
an unlisted function and on a listed function that is no longer out of line.

  check_simd_inlining.py --build DIR          build every configuration and check it
  check_simd_inlining.py CONFIG=LIBRARY...    check unstripped libraries
  check_simd_inlining.py --list LIBRARY       out-of-line crate functions, with size
  check_simd_inlining.py --compare OLD NEW    what a change inlined or outlined

`--build` runs on x86-64 or arm64 Linux from the repository, so the toolchain in
rust-toolchain.toml applies. It builds the other architecture with
`ARCH-linux-gnu-gcc`, `-strip` and `-nm` (Debian/Ubuntu `gcc-ARCH-linux-gnu`).
Symbols do not show slow code inside an inlined body; time changes with
docs/performance.md.
"""
import argparse
from collections import Counter
import os
from pathlib import Path
import platform
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
LIST = Path(__file__).with_suffix(".txt")
CRATE = "faster_base58::"
ARCHES = ("x86_64", "aarch64")
FEATURES = ("cache", "compact")
CONFIGS = [arch + suffix for arch in ARCHES for suffix in ("", "+cache", "+compact")]


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


def reported(name):
    """Whether a function is checked: crate code or an intrinsic."""
    return CRATE in name or "core::core_arch::" in name


def applies(terms, config):
    arch, *features = config.split("+")
    held = {"all", arch, *features}
    return all(term[1:] not in held if term.startswith("!") else term in held for term in terms)


def parse(text):
    """[(terms, names)] from the list: `[terms]` headers, one name per line, `#` comments."""
    sections = []
    for number, line in enumerate(text.splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            terms = line[1:-1].split()
            known = {"all", *ARCHES, *FEATURES}
            if not terms or any(term.removeprefix("!") not in known for term in terms):
                raise ValueError(f"line {number}: unknown section {line}")
            if not any(applies(terms, config) for config in CONFIGS):
                raise ValueError(f"line {number}: {line} applies to no configuration")
            sections.append((terms, []))
        elif not sections:
            raise ValueError(f"line {number}: name before the first section")
        else:
            sections[-1][1].append(line)
    return sections


def expected(sections, config):
    names = [name for terms, names in sections if applies(terms, config) for name in names]
    twice = sorted(name for name, count in Counter(names).items() if count > 1)
    if twice:
        raise ValueError(f"{config}: listed twice: {twice[0]}")
    return set(names)


def differences(funcs, allowed):
    """Checked functions that are out of line but not listed, and listed but absent."""
    present = {name for name, _ in funcs if reported(name)}
    return sorted(present - allowed), sorted(allowed - present)


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


def build(directory):
    """{config: (library, nm)} after building the Auto consumer for every configuration."""
    host = platform.machine()
    if sys.platform != "linux" or host not in ARCHES:
        raise SystemExit("--build runs on x86-64 or arm64 Linux")
    subprocess.run(["cargo", "fetch", "--locked"], cwd=ROOT, check=True)
    libraries = {}
    for config in CONFIGS:
        arch, *features = config.split("+")
        target = f"{arch}-unknown-linux-gnu"
        tools = "" if arch == host else f"{arch}-linux-gnu-"
        env = dict(os.environ)
        if tools:
            env.setdefault(f"CARGO_TARGET_{target.upper().replace('-', '_')}_LINKER", tools + "gcc")
        output = Path(directory).resolve() / config
        subprocess.run([sys.executable, "-B", str(ROOT / "scripts/code_size.py"), "--source", str(ROOT),
                        "--output", str(output), "--target", target, "--cpu", "generic",
                        "--backends", "auto", "--directions", "both", "--kinds", "cdylib",
                        "--strip-tool", tools + "strip", *(f"--{feature}" for feature in features)],
                       cwd=ROOT, env=env, check=True)
        libraries[config] = (output / "auto-both-cdylib.so", tools + "nm")
    return libraries


def self_test():
    nm = "\n".join([
        "0000000000001000 t _core::core_arch::x86::xsave::_xgetbv",
        "0000000000001040 t faster_base58::ntt::avx512::z::forward",
        "0000000000001400 t <&mut core::array::drain::Drain<usize, faster_base58::ntt::avx512::z::Z> as core::ops::function::FnMut<(usize,)>>::call_mut",
        "0000000000001480 t faster_base58::dc::write_bytes",
        "0000000000001500 t core::ptr::drop_glue::<alloc::string::String>",
        "0000000000001600 T faster_base58_encode",
        "                 U memcpy",
        "0000000000001700 d core::core_arch::x86::avx512f::CONSTANT",
    ])
    funcs = functions(nm)
    assert funcs[0] == ("core::core_arch::x86::xsave::_xgetbv", 0x40), funcs
    assert funcs[-1] == ("faster_base58_encode", None), funcs
    drain = funcs[2][0]
    sections = parse("\n".join([
        "# comment", "[all]", "faster_base58::dc::write_bytes",
        "[x86_64]", "  core::core_arch::x86::xsave::_xgetbv  ",
        "[x86_64 !cache]", "faster_base58::ntt::avx512::z::forward",
        "[aarch64 cache]", "faster_base58::ntt::neon::forward"]))
    assert differences(funcs, expected(sections, "x86_64")) == ([drain], [])
    assert differences(funcs, expected(sections, "x86_64+cache")) == (
        [drain, "faster_base58::ntt::avx512::z::forward"], [])
    assert differences(funcs, expected(sections, "aarch64+cache")) == (
        [drain, "core::core_arch::x86::xsave::_xgetbv", "faster_base58::ntt::avx512::z::forward"],
        ["faster_base58::ntt::neon::forward"])
    assert expected(sections, "aarch64+compact") == {"faster_base58::dc::write_bytes"}
    for bad in ["name", "[sse2]\nname", "[]\nname", "[x86_64 aarch64]\nname", "[all]\na\n[x86_64]\na"]:
        try:
            expected(parse(bad), "x86_64")
        except ValueError:
            continue
        raise AssertionError(f"accepted {bad!r}")
    old = [("faster_base58::a", 10), ("faster_base58::b", 20), ("core::x", 5),
           ("core::core_arch::x86::avx2::_mm256_add_epi32", 16)]
    new = [("faster_base58::b", 30), ("faster_base58::c", 40)]
    assert compare(old, new) == (["core::core_arch::x86::avx2::_mm256_add_epi32", "faster_base58::a"],
                                 ["faster_base58::c"], [("faster_base58::b", 20, 30)]), compare(old, new)
    assert all(applies(["all"], config) for config in CONFIGS)
    assert [config for config in CONFIGS if applies(["!cache", "!compact"], config)] == ["x86_64", "aarch64"]
    parse(LIST.read_text())
    print("check_simd_inlining self-test passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("libraries", nargs="*", help="CONFIG=LIBRARY pairs (or the --list/--compare inputs)")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--build", type=Path, metavar="DIR", help="build every configuration into DIR and check it")
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
        for name, size in sorted(f for f in read(args.nm, args.libraries[0]) if reported(f[0])):
            print(f"{size if size is not None else '?':>8} {name}")
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
    sections = parse(LIST.read_text())
    if args.build:
        if args.libraries:
            parser.error("--build takes no libraries")
        libraries = build(args.build)
    else:
        if not args.libraries:
            parser.error("pass --build DIR or CONFIG=LIBRARY pairs")
        libraries = {}
        for pair in args.libraries:
            config, _, library = pair.partition("=")
            if config not in CONFIGS or not library:
                parser.error(f"expected CONFIG=LIBRARY with CONFIG in {', '.join(CONFIGS)}: {pair}")
            libraries[config] = (library, args.nm)
    failed = False
    for config, (library, nm) in libraries.items():
        allowed = expected(sections, config)
        unlisted, absent = differences(read(nm, library), allowed)
        for name in unlisted:
            print(f"{config}: + {name}")
        for name in absent:
            print(f"{config}: - {name}")
        print(f"{config}: {len(allowed)} listed, {len(unlisted)} unlisted, {len(absent)} no longer out of line")
        failed |= bool(unlisted or absent)
    if failed:
        raise SystemExit(f"out-of-line functions changed ('+' unlisted, '-' inlined or removed): "
                         f"fix the code or update {LIST.relative_to(ROOT)}")
    return None


if __name__ == "__main__":
    main()

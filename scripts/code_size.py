#!/usr/bin/env python3
"""Build one reachable backend per artifact and record linked deployment sizes."""

import argparse
import csv
import ctypes
from datetime import datetime, timezone
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import zlib


ROOT = Path(__file__).resolve().parents[1]
BACKENDS = "empty,auto,no-alloc,batched,karatsuba,toom3,ntt,ntt-cached,ntt-portable,malachite,matrix,matrix-portable,bs58,five8,matrix-forced,ntt-same-owned,ntt-same-cached"
CONTROLS = ("matrix-forced", "ntt-same-owned", "ntt-same-cached")


def backend_semantics(backend):
    result = {"role": "control" if backend in CONTROLS else "baseline" if backend == "empty" else "api",
              "label": backend, "measurement_support": {"max_decoded_bytes": 4194304}}
    if backend == "matrix-forced":
        result.update(label="forced general matrix; no API small/fixed-size fallback",
                      measurement_support={"encode": {"min_decoded_bytes": 1, "max_decoded_bytes": 8192},
                                           "decode": {"min_encoded_chars": 1, "max_encoded_chars": 5632}})
    elif backend in ("ntt-same-owned", "ntt-same-cached"):
        result["label"] = f"same-leaf NTT, {'cached' if backend.endswith('cached') else 'owned'} twiddles; shared fallback remains"
    elif backend == "five8":
        result["measurement_support"] = {"decoded_bytes": [32, 64]}
    elif backend in ("bs58", "batched", "no-alloc"):
        result["measurement_support"] = {"max_decoded_bytes": 65536}
    return result


def uleb(data, offset):
    value = shift = 0
    while True:
        if offset >= len(data) or shift > 63:
            raise ValueError("invalid/truncated unsigned LEB128")
        byte = data[offset]
        offset += 1
        value |= (byte & 127) << shift
        if byte < 128:
            return value, offset
        shift += 7


def sections(data):
    result = dict(code_bytes=0, readonly_bytes=0, data_bytes=0, bss_bytes=0,
                  wasm_code_section_bytes="", wasm_data_section_bytes="",
                  wasm_custom_section_bytes="", wasm_initial_memory_bytes="")
    if data[:4] == b"\0asm":
        assert data[4:8] == b"\x01\0\0\0"
        result.update(bss_bytes="", readonly_bytes="", code_bytes="", data_bytes="",
                      wasm_code_section_bytes=0, wasm_data_section_bytes=0,
                      wasm_custom_section_bytes=0, wasm_initial_memory_bytes=0)
        offset = 8
        while offset < len(data):
            kind = data[offset]
            size, start = uleb(data, offset + 1)
            end = start + size
            if end > len(data):
                raise ValueError("truncated Wasm section")
            key = {0: "wasm_custom_section_bytes", 10: "wasm_code_section_bytes",
                   11: "wasm_data_section_bytes"}.get(kind)
            if key:
                result[key] += size
            if kind == 5:
                count, pos = uleb(data, start)
                assert count == 1, "probe expects one defined linear memory"
                flags, pos = uleb(data, pos)
                minimum, pos = uleb(data, pos)
                result["wasm_initial_memory_bytes"] = minimum * 65536
            offset = end
        return result
    if data[:4] == b"\x7fELF":
        assert data[4:6] == b"\x02\x01", "only little-endian ELF64 supported"
        offset = struct.unpack_from("<Q", data, 40)[0]
        stride, count = struct.unpack_from("<HH", data, 58)
        for index in range(count):
            _, kind, flags, _, _, size, _, _, _, _ = struct.unpack_from("<IIQQQQIIQQ", data, offset + index * stride)
            if flags & 2:
                key = "bss_bytes" if kind == 8 else "code_bytes" if flags & 4 else "data_bytes" if flags & 1 else "readonly_bytes"
                result[key] += size
        return result
    assert data[:4] == b"\xcf\xfa\xed\xfe", "unsupported object format"
    commands = struct.unpack_from("<I", data, 16)[0]
    offset = 32
    for _ in range(commands):
        command, length = struct.unpack_from("<II", data, offset)
        if command == 0x19:
            count = struct.unpack_from("<I", data, offset + 64)[0]
            for index in range(count):
                section = offset + 72 + 80 * index
                segment = data[section + 16:section + 32].split(b"\0")[0]
                size = struct.unpack_from("<Q", data, section + 40)[0]
                flags = struct.unpack_from("<I", data, section + 64)[0]
                if flags & 255 in (1, 12, 18):
                    key = "bss_bytes"
                elif flags & 0x80000400 or flags & 255 == 8:
                    key = "code_bytes"
                elif segment in (b"__TEXT", b"__DATA_CONST"):
                    key = "readonly_bytes"
                else:
                    key = "data_bytes"
                result[key] += size
        offset += length
    return result


def run(command, env=None, log=None):
    if log:
        log.write("$ " + " ".join(command) + "\n")
        log.flush()
    result = subprocess.run(command, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    if log:
        log.write(result.stdout)
        log.flush()
    if result.returncode:
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(command)}\n{result.stdout}")
    return result.stdout


def native_check(path, direction, backend):
    library = ctypes.CDLL(str(path))
    library.size_alloc.argtypes = (ctypes.c_size_t,)
    library.size_alloc.restype = ctypes.c_void_p
    library.size_dealloc.argtypes = (ctypes.c_void_p, ctypes.c_size_t)
    for size in (0, 1, 32, 65537):
        pointer = library.size_alloc(size)
        assert pointer and ctypes.string_at(pointer, size) == bytes(size)
        ctypes.memset(pointer, 0xA5, size)
        library.size_dealloc(pointer, size)
    vectors = [(b"\0\1", b"12")]
    if backend == "five8":
        vectors = [(bytes(size), b"1" * size) for size in (32, 64)]
    for data, text in vectors:
        for name, input_bytes, expected in (("encode", data, text), ("decode", text, data)):
            if direction not in (name, "both"):
                continue
            operation = getattr(library, "size_" + name)
            operation.argtypes = (ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t)
            operation.restype = ctypes.c_size_t
            source = ctypes.create_string_buffer(input_bytes)
            capacity = len(data) if backend == "five8" and name == "decode" else max(32, len(expected))
            if backend == "five8" and name == "encode":
                capacity = 44 if len(data) == 32 else 88
            output = ctypes.create_string_buffer(capacity)
            written = operation(source, len(input_bytes), output, capacity)
            assert written == (len(input_bytes) if backend == "empty" else len(expected))
            if backend != "empty":
                assert output.raw[:written] == expected
                assert operation(source, len(input_bytes), output, 0) == ctypes.c_size_t(-1).value
            if name == "decode":
                operation = library.size_repeat_decode_prevalidated
                operation.argtypes = (ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t, ctypes.c_size_t)
                operation.restype = ctypes.c_size_t
                assert operation(source, len(input_bytes), output, capacity, 3) == written
                assert operation(source, len(input_bytes), output, capacity, 0) == 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=ROOT)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--target", default="aarch64-apple-darwin")
    parser.add_argument("--cpu", default="apple-m4")
    parser.add_argument("--backends", default=BACKENDS)
    parser.add_argument("--directions", default="encode,decode,both")
    parser.add_argument("--kinds", default="cdylib,bin")
    parser.add_argument("--opt-level", default="3", choices=("3", "z"))
    parser.add_argument("--label", default="current")
    parser.add_argument("--strip-tool", default=shutil.which("llvm-strip") or "/opt/homebrew/opt/llvm/bin/llvm-strip")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--cache", action="store_true", help="enable optional library caches in this cohort")
    args = parser.parse_args()
    if args.self_test:
        assert uleb(bytes((0xE5, 0x8E, 0x26)), 0) == (624485, 3)
        tiny_wasm = b"\0asm\x01\0\0\0\x0a\x01\0\x0b\x01\0"
        sizes = sections(tiny_wasm)
        assert sizes["wasm_code_section_bytes"] == sizes["wasm_data_section_bytes"] == 1
        assert gzip.decompress(gzip.compress(tiny_wasm, mtime=0)) == tiny_wasm
        assert backend_semantics("matrix-forced")["measurement_support"]["decode"]["max_encoded_chars"] == 5632
        assert backend_semantics("ntt-same-owned")["role"] == "control"
        try:
            uleb(b"\x80", 0)
        except ValueError:
            pass
        else:
            raise AssertionError("truncated LEB128 accepted")
        print("code_size self-check passed")
        return
    if not args.output:
        parser.error("--output is required")
    source = args.source.resolve()
    external = (source / "tests/support/malachite.rs").exists()
    if "malachite" in args.backends.split(",") and not external:
        parser.error("the Malachite probe requires SOURCE/tests/support/malachite.rs")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    work = output / "probe"
    (work / "src").mkdir(parents=True, exist_ok=True)
    feature = ', "faster-base58/__internals"' if external else ""
    manifest = f'''[package]
name = "base58-code-size"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
faster-base58 = {{ path = {json.dumps(str(source))} }}
malachite-nz = {{ version = "=0.14.0", optional = true }}
bs58 = {{ version = "=0.5.1", optional = true }}
five8 = {{ version = "=1.0.0", optional = true }}
[features]
malachite = ["dep:malachite-nz"{feature}]
bs58 = ["dep:bs58"]
five8 = ["dep:five8"]
benchmark-controls = ["faster-base58/__internals"]
cache = ["faster-base58/cache"]
[lib]
crate-type = ["cdylib"]
[[bin]]
name = "base58-code-size"
path = "src/main.rs"
[profile.release]
opt-level = {json.dumps(args.opt_level) if args.opt_level == "z" else args.opt_level}
lto = "fat"
codegen-units = 16
debug = false
strip = false
incremental = false
'''
    (work / "Cargo.toml").write_text(manifest)
    for name in ("lib.rs", "entry.rs", "main.rs"):
        shutil.copyfile(ROOT / "tools/code-size/src" / name, work / "src" / name)
    adapter = f'#[path = {json.dumps(str(source / "tests/support/malachite.rs"))}]\nmod malachite_adapter;\n'
    (work / "src/malachite_adapter.rs").write_text(adapter)
    competitors = source / "tests/support/competitors.rs"
    if not competitors.exists():
        competitors = ROOT / "tests/support/competitors.rs"
    adapter = f'#[path = {json.dumps(str(competitors))}]\nmod competitors_adapter;\n'
    (work / "src/competitors_adapter.rs").write_text(adapter)
    env = dict(os.environ, CARGO_TARGET_DIR=str(output / "target"))
    env["RUSTFLAGS"] = "" if args.cpu == "generic" else f"-C target-cpu={args.cpu}"
    env.pop("CARGO_ENCODED_RUSTFLAGS", None)
    target_cfg = ["rustc", "--print", "cfg", "--target", args.target,
                  "-C", f"opt-level={args.opt_level}", "-C", "debug-assertions=no"]
    if args.cpu != "generic":
        target_cfg += ["-C", f"target-cpu={args.cpu}"]
    metadata = {"label": args.label, "source": str(source), "target": args.target, "cache": args.cache,
                "rustflags": env["RUSTFLAGS"], "profile": manifest,
                "rustc": run(["rustc", "-Vv"]), "commands": [],
                "rustc_target_cfg": run(target_cfg),
                "dependency_tree_default": run(["cargo", "tree", "--offline", "--manifest-path", str(work / "Cargo.toml"), "--edges", "normal"], env),
                "created_utc": datetime.now(timezone.utc).isoformat(),
                "python": sys.version, "zlib": zlib.ZLIB_VERSION,
                "strip_version": run([args.strip_tool, "--version"]),
                "source_files_sha256": {}, "harness_files_sha256": {}}
    metadata["backend_semantics"] = {backend: backend_semantics(backend) for backend in args.backends.split(",")}
    metadata["decode_timing_contract"] = "prevalidated-utf8-repeat-v1"
    metadata["repeat_timing_contract"] = "opaque-slice-lengths-v1"
    metadata["checked_decode_exports"] = ["size_decode", "size_repeat_decode"]
    metadata["probe_cargo_lock"] = (work / "Cargo.lock").read_text()
    for key, arguments in (("checkout_head", ["rev-parse", "HEAD"]), ("checkout_dirty", ["status", "--short"])):
        git = subprocess.run(["git", "-C", str(ROOT), *arguments], text=True,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        metadata[key] = git.stdout.strip() if git.returncode == 0 else None
    digest = hashlib.sha256()
    paths = list((source / "src").rglob("*.rs")) + list((source / "tests/support").rglob("*.rs"))
    paths += [source / "Cargo.toml", source / "Cargo.lock"]
    if any(backend in ("bs58", "five8") for backend in args.backends.split(",")) and competitors not in paths:
        paths.append(competitors)
    for path in sorted(paths):
        name = str(path.relative_to(source)) if path.is_relative_to(source) else str(path)
        content = path.read_bytes()
        metadata["source_files_sha256"][name] = hashlib.sha256(content).hexdigest()
        digest.update(name.encode() + b"\0" + content)
    metadata["source_sha256"] = digest.hexdigest()
    for path in sorted((ROOT / "tools/code-size").rglob("*.rs")) + [Path(__file__), ROOT / "scripts/code_size_wasm.mjs"]:
        metadata["harness_files_sha256"][str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
    rows = []
    csv_path = output / "sizes.csv"
    with (output / "build.log").open("w") as log:
        for backend in args.backends.split(","):
            assert backend in BACKENDS.split(","), backend
            for direction in args.directions.split(","):
                assert direction in ("encode", "decode", "both"), direction
                for kind in args.kinds.split(","):
                    assert kind in ("bin", "cdylib"), kind
                    if args.target.startswith("wasm") and kind == "bin":
                        continue
                    command = ["cargo", "rustc", "--manifest-path", str(work / "Cargo.toml"),
                               "--release", "--locked", "--offline", "--target", args.target,
                               "--lib" if kind == "cdylib" else "--bin"]
                    if kind == "bin":
                        command += ["base58-code-size"]
                    if backend in ("malachite", "bs58", "five8"):
                        command += ["--features", backend]
                    if backend not in ("empty", "auto", "no-alloc", "malachite", "bs58", "five8"):
                        command += ["--features", "benchmark-controls"]
                    if args.cache:
                        command += ["--features", "cache"]
                    command += ["--", "--cfg", f'size_backend="{backend}"',
                                "--cfg", f'size_direction="{direction}"']
                    if backend == "malachite" and external:
                        command += ["--cfg", "size_external_malachite"]
                    metadata["commands"].append(command)
                    run(command, env, log)
                    extension = ".wasm" if args.target.startswith("wasm") else ".dylib" if "apple" in args.target else ".so"
                    built = output / "target" / args.target / "release" / ("libbase58_code_size" + extension if kind == "cdylib" else "base58-code-size")
                    if args.target.startswith("wasm"):
                        built = built.with_name("base58_code_size.wasm")
                    name = f"{backend}-{direction}-{kind}" + (extension if kind == "cdylib" else "")
                    artifact = output / name
                    stripped = output / (name + ".stripped")
                    shutil.copy2(built, artifact)
                    shutil.copy2(built, stripped)
                    run([args.strip_tool, "--strip-all", str(stripped)], log=log)
                    data = artifact.read_bytes()
                    stripped_data = stripped.read_bytes()
                    row = dict(label=args.label, target=args.target, backend=backend,
                               direction=direction, kind=kind, opt_level=args.opt_level,
                               file_bytes=len(data), stripped_file_bytes=len(stripped_data),
                               stripped_gzip_bytes=len(gzip.compress(stripped_data, compresslevel=9, mtime=0)),
                               artifact_sha256=hashlib.sha256(data).hexdigest(),
                               stripped_sha256=hashlib.sha256(stripped_data).hexdigest(),
                               **sections(stripped_data))
                    native_host = metadata["rustc"].split("host: ")[1].splitlines()[0]
                    if kind == "cdylib" and args.target == native_host:
                        native_check(artifact, direction, backend)
                    rows.append(row)
                    with csv_path.open("w", newline="") as csv_file:
                        writer = csv.DictWriter(csv_file, fieldnames=list(rows[0]))
                        writer.writeheader()
                        writer.writerows(rows)
                    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
                    print(f"{backend:16} {direction:6} {kind:6} {row['stripped_file_bytes']:8} stripped bytes", flush=True)
    print(csv_path)


if __name__ == "__main__":
    main()

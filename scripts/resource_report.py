#!/usr/bin/env python3
"""Build and run isolated heap/CPU probes; refuse existing report contents."""

import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import platform
import shlex
import shutil
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SIZES = [0, 1, 8, 16, 21, 22, 23, 31, 32, 33, 63, 64, 65, 128, 256, 512,
                 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144,
                 524288, 1048576, 2097152, 4194304]


def command(args, **kwargs):
    return subprocess.run(args, check=True, text=True, capture_output=True, **kwargs).stdout


def optional_git(args):
    try:
        return command(["git", *args], cwd=ROOT).strip()
    except (FileNotFoundError, subprocess.CalledProcessError):
        return None


def source_identity():
    paths = {ROOT / "Cargo.toml", ROOT / "Cargo.lock", Path(__file__).resolve()}
    for directory in [ROOT / "src", ROOT / "tests/support", ROOT / "tools/resource-profile"]:
        paths.update(p for p in directory.rglob("*")
                     if p.is_file() and "target" not in p.parts and p.suffix in {".rs", ".toml", ".lock"})
    digest = hashlib.sha256()
    entries = []
    for path in sorted(paths):
        relative = str(path.relative_to(ROOT))
        content = path.read_bytes()
        # Length prefixes make path/content boundaries unambiguous.
        digest.update(len(relative.encode()).to_bytes(8, "big"))
        digest.update(relative.encode())
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
        entries.append({"path": relative, "sha256": hashlib.sha256(content).hexdigest()})
    return {"sha256": digest.hexdigest(), "files": entries}


def file_sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cargo_configs():
    import tomllib
    paths = {Path.home() / ".cargo/config", Path.home() / ".cargo/config.toml"}
    for directory in [ROOT, *ROOT.parents, ROOT / "tools/resource-profile"]:
        paths.update([directory / ".cargo/config", directory / ".cargo/config.toml"])
    result = []
    for path in sorted(paths):
        if path.is_file():
            parsed = tomllib.loads(path.read_text())
            # Keep build settings; registry credentials and environment secrets do not belong in reports.
            settings = {key: parsed[key] for key in ["build", "target", "profile", "unstable"] if key in parsed}
            result.append({"path": str(path), "sha256": file_sha(path), "settings": settings})
    return result


def environment():
    return {"platform": platform.platform(), "machine": platform.machine(),
            "hostname": platform.node(), "python": sys.version,
            "rustc": command(["rustc", "-vV"]), "cargo": command(["cargo", "-V"]),
            "flags": {key: os.environ.get(key) for key in [
                "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_RUSTFLAGS", "RUSTUP_TOOLCHAIN",
                "CARGO_PROFILE_RELEASE_LTO", "CARGO_PROFILE_RELEASE_OPT_LEVEL"]},
            "cargo_configs": cargo_configs(),
            "git_head": optional_git(["rev-parse", "HEAD"]),
            "git_status": optional_git(["status", "--short"])}


def build(args):
    identity = source_identity()
    metadata = Path(args.output).resolve()
    if metadata.exists():
        raise SystemExit(f"refusing existing build metadata: {metadata}")
    target = Path(args.target_dir).resolve()
    invocation = ["cargo", "build", "--locked", "--release", "--manifest-path",
                  str(ROOT / "tools/resource-profile/Cargo.toml"), "--target-dir", str(target)]
    features = []
    if args.count_allocations:
        features.append("count-allocations")
    if args.cache:
        features.append("cache")
    if args.compact:
        features.append("compact")
    if features:
        invocation += ["--features", ",".join(features)]
    env = dict(os.environ, RESOURCE_SOURCE_SHA256=identity["sha256"])
    print("Building:", " ".join(invocation), flush=True)
    subprocess.run(invocation, check=True, cwd=ROOT, env=env)
    if source_identity() != identity:
        raise SystemExit("source changed during build; rebuild before measuring")
    binary = target / "release/resource-profile"
    if args.binary_output:
        retained = Path(args.binary_output).resolve()
        retained.parent.mkdir(parents=True, exist_ok=True)
        with binary.open("rb") as source, retained.open("xb") as destination:
            shutil.copyfileobj(source, destination)
        shutil.copymode(binary, retained)
        binary = retained
    description = json.loads(command([str(binary), "describe"]))
    assert description["source_sha256"] == identity["sha256"]
    assert description["compiled_features"]["cache"] == args.cache
    assert description["compiled_features"]["compact"] == args.compact
    assert description["count_allocations"] == args.count_allocations
    encoded_flags = env.get("CARGO_ENCODED_RUSTFLAGS")
    if encoded_flags is not None:
        flags = encoded_flags.split("\x1f") if encoded_flags else []
    else:
        flags = shlex.split(env.get("RUSTFLAGS", env.get("CARGO_BUILD_RUSTFLAGS", "")))
    cfg_command = ["rustc", "--print", "cfg", "-C", "opt-level=3", "-C", "debug-assertions=no", *flags]
    record = {"schema": 1, "created_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
              "command": invocation, "binary": str(binary), "binary_sha256": file_sha(binary),
              "source": identity, "worker": description, "environment": environment(),
              "compiled_features": description["compiled_features"],
              "rustc_cfg": {"command": cfg_command, "stdout": command(cfg_command),
                            "scope": "release defaults plus ambient Rust flag environment; worker compiled_features records actual dispatch cfgs"}}
    metadata.parent.mkdir(parents=True, exist_ok=True)
    with metadata.open("x") as output:
        json.dump(record, output, indent=2)
    print(metadata)


def write_csv(path, rows):
    if not rows:
        return
    columns = list(dict.fromkeys(key for row in rows for key in row))
    with path.open("x", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=columns)
        writer.writeheader()
        writer.writerows(rows)


def summarize(rows):
    groups = {}
    for row in rows:
        if row["kind"] == "timing":
            metrics = {"wall_ns_per_call": row["wall_ns"] / row["iterations"],
                       "cpu_ns_per_call": row["cpu_ns"] / row["iterations"]}
        elif row["kind"] == "allocations":
            metrics = {key: value for key, value in row.items()
                       if isinstance(value, (float, int)) and key not in {"repeat", "bytes"}}
        else:
            continue
        for metric, value in metrics.items():
            key = (row["operation"], row["backend"], row["bytes"], row["phase"], metric)
            groups.setdefault(key, []).append(value)
    return [dict(zip(["operation", "backend", "bytes", "phase", "metric"], key),
                 count=len(values), median=statistics.median(values), minimum=min(values), maximum=max(values))
            for key, values in sorted(groups.items())]


def validate(events):
    for row in events:
        if row["kind"] == "allocations" and row["failed_calls"] == 0:
            delta = row["alloc_bytes"] + row["realloc_new_bytes"] - row["realloc_old_bytes"] - row["dealloc_bytes"]
            assert delta == row["retained_delta_bytes"], row
            assert row["peak_live_bytes"] >= max(row["live_before_bytes"], row["live_after_bytes"]), row


def skip_reason(backend, operation, size, encoded_chars=None):
    exact = backend.get(f"supported_{operation}_bytes")
    if exact is not None and size not in exact:
        return f"fixed-width API supports only decoded/input byte sizes {exact}"
    if size < backend.get(f"min_{operation}_bytes", 0):
        return "outside forced matrix kernel domain"
    if size > backend[f"max_{operation}_bytes"]:
        return "quadratic measurement cap" if backend["backend"] in {"batched", "no-alloc", "bs58"} else "forced matrix kernel limit"
    if operation == "decode" and "max_decode_chars" in backend:
        assert encoded_chars is not None, "forced matrix decode requires actual encoded length"
        if not backend.get("min_decode_chars", 0) <= encoded_chars <= backend["max_decode_chars"]:
            return "outside forced matrix encoded-character domain"
    return None


def shuffle_jobs(jobs, seed):
    assert 0 < seed < 2**32
    result = list(jobs)
    state = seed
    for index in range(len(result) - 1, 0, -1):
        state = (state ^ (state << 13)) & 0xFFFFFFFF
        state = (state ^ (state >> 17)) & 0xFFFFFFFF
        state = (state ^ (state << 5)) & 0xFFFFFFFF
        other = state % (index + 1)
        result[index], result[other] = result[other], result[index]
    return result


def run(args):
    output = Path(args.output).resolve()
    if output.exists() and any(output.iterdir()):
        raise SystemExit(f"refusing nonempty report directory: {output}")
    output.mkdir(parents=True, exist_ok=True)
    build_record = json.loads(Path(args.build_metadata).read_text())
    binary = Path(args.binary or build_record["binary"]).resolve()
    assert file_sha(binary) == build_record["binary_sha256"], "binary differs from build metadata"
    assert source_identity() == build_record["source"], "source differs from build metadata"
    description = json.loads(command([str(binary), "describe"]))
    assert description == build_record["worker"], "worker differs from build metadata"
    assert (args.mode == "allocations") == description["count_allocations"] or args.mode == "inventory", \
        "allocations requires the counted build; timing requires the uncounted build"
    registry = [json.loads(line) for line in command([str(binary), "list"]).splitlines()]
    selected = args.backends.split(",") if args.backends else [r["backend"] for r in registry]
    assert len(selected) == len(set(selected)), "duplicate backend"
    assert set(selected) <= {r["backend"] for r in registry}, "unknown backend"
    sizes = [int(n) for n in args.sizes.split(",")] if args.sizes else DEFAULT_SIZES
    assert len(sizes) == len(set(sizes)), "duplicate size"
    assert all(0 <= size <= description.get("max_input_bytes", 1048576) for size in sizes)
    registry_by_name = {row["backend"]: row for row in registry}
    fixture_chars = {}
    if "matrix-forced" in selected and args.mode != "inventory":
        entry = registry_by_name["matrix-forced"]
        candidates = [size for size in sizes if entry.get("min_decode_bytes", 0) <= size <= entry["max_decode_bytes"]]
        if candidates:
            events = [json.loads(line) for line in command([str(binary), "fixture-metadata", *map(str, candidates)]).splitlines()]
            fixture_chars = {row["bytes"]: row["encoded_chars"] for row in events}
            assert len(events) == len(fixture_chars) == len(candidates)
            assert set(fixture_chars) == set(candidates)
    jobs, skipped = [], []
    if args.mode == "inventory":
        jobs = [(operation, "matrix-forced", size, repeat)
                for operation, classes in [("encode", [512, 1024, 2048, 4096, 8192]),
                                           ("decode", [704, 1408, 2816, 5632])]
                for size in classes for repeat in range(args.repeats)]
    else:
        for name in selected:
            entry = registry_by_name[name]
            for size in sizes:
                for operation in ["encode", "decode"]:
                    context = {"operation": operation, "backend": name, "bytes": size}
                    reason = skip_reason(entry, operation, size, fixture_chars.get(size))
                    if reason is not None:
                        skipped.append(dict(context, reason=reason,
                                            max_bytes=entry[f"max_{operation}_bytes"],
                                            supported_bytes=entry.get(f"supported_{operation}_bytes"),
                                            encoded_chars=fixture_chars.get(size)))
                    else:
                        jobs.extend((operation, name, size, repeat) for repeat in range(args.repeats))
    jobs = shuffle_jobs(jobs, args.order_seed)
    size_key = "input_size" if args.mode == "inventory" else "decoded_bytes"
    metadata = {"schema": 1, "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "arguments": vars(args), "build": build_record, "backend_registry": registry,
                "sizes": sizes, "backends": selected,
                "timing_protocol": description.get("timing_protocol"),
                "oracle": description.get("oracle"), "fixture_encoded_chars": fixture_chars,
                "job_order_seed": args.order_seed, "job_order_algorithm": "xorshift32-fisher-yates-v1",
                "jobs": [dict(operation=operation, backend=name, **{size_key: size}, repeat=repeat)
                         for operation, name, size, repeat in jobs], "completed_jobs": 0,
                "sampling_policy": {"primary_clock": "cpu_ns", "minimum_batch_ns": description.get("minimum_batch_ns"),
                                    "max_iterations": description.get("max_iterations"),
                                    "growth_factor_min": 2, "growth_factor_max": 16},
                "run_environment": environment(), "status": "running"}
    with (output / "metadata.json").open("x") as stream:
        json.dump(metadata, stream, indent=2)
    rows = []
    with (output / "raw.jsonl").open("x") as raw:
        try:
            for index, (operation, backend, size, repeat) in enumerate(jobs):
                print(f"{index + 1}/{len(jobs)} {args.mode} {operation} {backend} {size} repeat={repeat}", flush=True)
                invocation = [str(binary), args.mode, operation]
                invocation += [str(size)] if args.mode == "inventory" else [backend, str(size), str(args.samples)]
                events = [json.loads(line) for line in command(invocation, timeout=args.timeout).splitlines()]
                validate(events)
                context = {"operation": operation, "backend": backend, "bytes": size, "repeat": repeat}
                for event in events:
                    row = dict(context, **event)
                    raw.write(json.dumps(row, sort_keys=True) + "\n")
                    rows.append(row)
                raw.flush()
                metadata["completed_jobs"] = index + 1
            assert source_identity() == build_record["source"], "source changed during measurement"
            metadata["status"] = "complete"
        finally:
            for kind, filename in [("allocations", "allocations.csv"), ("timing", "timings.csv"),
                                   ("cache", "caches.csv"), ("cache-header", "cache-headers.csv"), ("io", "io.csv")]:
                write_csv(output / filename, [row for row in rows if row["kind"] == kind])
            write_csv(output / "skipped.csv", skipped)
            write_csv(output / "summary.csv", summarize(rows))
            metadata["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            if metadata["status"] != "complete":
                metadata["status"] = "incomplete"
            (output / "metadata.json").write_text(json.dumps(metadata, indent=2))


def self_test():
    row = {"kind": "allocations", "phase": "cold", "operation": "encode", "backend": "test", "bytes": 1,
           "repeat": 0, "failed_calls": 0, "alloc_bytes": 24, "realloc_new_bytes": 32,
           "realloc_old_bytes": 16, "dealloc_bytes": 40, "retained_delta_bytes": 0,
           "live_before_bytes": 5, "peak_live_bytes": 45, "live_after_bytes": 5}
    validate([row])
    bad = dict(row, retained_delta_bytes=1)
    try:
        validate([bad])
    except AssertionError:
        pass
    else:
        raise AssertionError("flow validation accepted a wrong result")
    example = dict(row, kind="timing", iterations=10, wall_ns=200, cpu_ns=100)
    summary = summarize([example, dict(example, wall_ns=400)])
    assert next(r for r in summary if r["metric"] == "wall_ns_per_call")["median"] == 30
    five8 = {"backend": "five8", "max_encode_bytes": 64, "max_decode_bytes": 64,
             "supported_encode_bytes": [32, 64], "supported_decode_bytes": [32, 64]}
    for operation in ["encode", "decode"]:
        for size in [32, 64]:
            assert skip_reason(five8, operation, size) is None
        for size in [0, 1, 31, 33, 63, 65, 1048576]:
            assert "fixed-width" in skip_reason(five8, operation, size)
        for name in ["bs58", "batched", "no-alloc"]:
            quadratic = {"backend": name, "max_encode_bytes": 65536, "max_decode_bytes": 65536}
            assert skip_reason(quadratic, operation, 65536) is None
            assert "quadratic" in skip_reason(quadratic, operation, 1048576)
    forced = {"backend": "matrix-forced", "min_encode_bytes": 1, "max_encode_bytes": 8192,
              "min_decode_bytes": 1, "max_decode_bytes": 5632,
              "min_decode_chars": 1, "max_decode_chars": 5632}
    assert skip_reason(forced, "encode", 0) is not None
    assert skip_reason(forced, "encode", 8192) is None
    assert skip_reason(forced, "decode", 4096, 5594) is None
    assert skip_reason(forced, "decode", 4124, 5633) is not None
    assert shuffle_jobs(list(range(5)), 20261024) == [4, 2, 3, 0, 1]
    assert sorted(shuffle_jobs(list(range(10)), 20261024)) == list(range(10))
    assert source_identity() == source_identity()
    for flag in ["-O", "-OO"]:
        result = subprocess.run([sys.executable, flag, str(Path(__file__).resolve()), "--help"],
                                capture_output=True, text=True)
        assert result.returncode != 0 and "requires Python assertions" in result.stderr
    for level in ["1", "2"]:
        result = subprocess.run([sys.executable, str(Path(__file__).resolve()), "--help"],
                                env=dict(os.environ, PYTHONOPTIMIZE=level), capture_output=True, text=True)
        assert result.returncode != 0 and "requires Python assertions" in result.stderr
    print("resource_report self-test passed")


def main():
    if sys.flags.optimize:
        raise SystemExit("resource_report requires Python assertions; unset PYTHONOPTIMIZE and omit -O")
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    sub.add_parser("fingerprint")
    sub.add_parser("self-test")
    build_parser = sub.add_parser("build")
    build_parser.add_argument("--target-dir", required=True)
    build_parser.add_argument("--output", required=True, help="new build metadata JSON")
    build_parser.add_argument("--count-allocations", action="store_true")
    build_parser.add_argument("--cache", action="store_true", help="enable the library's optional process-wide caches")
    build_parser.add_argument("--compact", action="store_true", help="enable the library's smaller IFMA kernels")
    build_parser.add_argument("--binary-output", help="retain the worker at a new path before another build reuses the target directory")
    run_parser = sub.add_parser("run")
    run_parser.add_argument("--build-metadata", required=True)
    run_parser.add_argument("--binary", help="override moved binary path; its SHA256 must match")
    run_parser.add_argument("--output", required=True, help="new or empty directory")
    run_parser.add_argument("--mode", choices=["allocations", "timing", "inventory"], required=True)
    run_parser.add_argument("--sizes", help="comma-separated decoded byte counts")
    run_parser.add_argument("--backends", help="comma-separated registry names")
    run_parser.add_argument("--repeats", type=int, default=3)
    run_parser.add_argument("--samples", type=int, default=3)
    run_parser.add_argument("--order-seed", type=int, default=20261024)
    run_parser.add_argument("--timeout", type=float, default=120)
    args = parser.parse_args()
    if args.action == "build":
        build(args)
    elif args.action == "run":
        assert args.repeats > 0 and args.samples > 0 and args.timeout > 0 and 0 < args.order_seed < 2**32
        run(args)
    elif args.action == "self-test":
        self_test()
    else:
        print(source_identity()["sha256"])


if __name__ == "__main__":
    main()

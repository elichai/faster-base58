#!/usr/bin/env python3
"""Build/run internal resource workers or count public-consumer heap; refuse existing reports."""

import argparse
import csv
import ctypes
import hashlib
import json
import math
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


PUBLIC_HEAP_METRICS = ['alloc_calls', 'alloc_bytes', 'zeroed_calls', 'zeroed_bytes', 'realloc_calls',
                       'realloc_old_bytes', 'realloc_new_bytes', 'dealloc_calls', 'dealloc_bytes',
                       'failed_calls', 'live_before_bytes', 'peak_live_bytes', 'live_after_bytes']


def public_heap_fixture(size):
    """Independent integer oracle: the Base58 text consists entirely of '2'."""
    if size == 0:
        return b'', b''
    digits = math.ceil((8 * (size - 1) + math.log2(57)) / math.log2(58))
    value = (58 ** digits - 1) // 57
    assert (value.bit_length() + 7) // 8 == size
    return value.to_bytes(size, 'big'), b'2' * digits


def public_heap_stats(stats):
    assert stats['failed_calls'] == 0
    validate([dict(stats, kind='allocations',
                   retained_delta_bytes=stats['live_after_bytes'] - stats['live_before_bytes'])])
    assert stats['zeroed_calls'] <= stats['alloc_calls']
    stats['peak_extra_bytes'] = stats['peak_live_bytes'] - stats['live_before_bytes']
    stats['retained_delta_bytes'] = stats['live_after_bytes'] - stats['live_before_bytes']
    stats['requests'] = stats['alloc_calls'] + stats['realloc_calls']


def public_heap_worker(job):
    api = ctypes.CDLL(job['artifact'])
    api.size_alloc.argtypes, api.size_alloc.restype = [ctypes.c_size_t], ctypes.c_void_p
    api.size_dealloc.argtypes, api.size_dealloc.restype = [ctypes.c_void_p, ctypes.c_size_t], None
    for name in ['size_count_self_check', 'size_count_begin', 'size_count_end']:
        function = getattr(api, name)
        function.argtypes, function.restype = [], None
    api.size_count_metrics.argtypes, api.size_count_metrics.restype = [], ctypes.c_size_t
    api.size_count_get.argtypes, api.size_count_get.restype = [ctypes.c_size_t], ctypes.c_size_t
    assert api.size_count_metrics() == len(PUBLIC_HEAP_METRICS)
    api.size_count_self_check()
    function = getattr(api, 'size_' + job['operation'])
    function.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t]
    function.restype = ctypes.c_size_t
    data, text = [Path(job[key]).read_bytes() for key in ['data', 'text']]
    value, expected = (data, text) if job['operation'] == 'encode' else (text, data)
    capacity = max(len(value), len(expected)) if job['backend'] == 'empty' else len(expected)
    source_length, output_length = len(value) + 34, capacity + 38
    source, output = api.size_alloc(source_length), api.size_alloc(output_length)
    assert source and output
    assert source + source_length <= output or output + output_length <= source
    ctypes.memset(source, 0xA5, source_length)
    ctypes.memmove(source + 17, value, len(value))
    rows = []
    try:
        for phase in ['cold', 'warm']:
            ctypes.memset(output, 0xA5, output_length)
            api.size_count_begin()
            written = function(source + 17, len(value), output + 19, capacity)
            api.size_count_end()
            stats = {name: api.size_count_get(index) for index, name in enumerate(PUBLIC_HEAP_METRICS)}
            public_heap_stats(stats)
            assert ctypes.string_at(source, source_length) == b'\xA5' * 17 + value + b'\xA5' * 17
            assert written == (len(value) if job['backend'] == 'empty' else len(expected))
            if job['backend'] != 'empty':
                assert ctypes.string_at(output + 19, written) == expected
            assert ctypes.string_at(output, 19) == b'\xA5' * 19
            assert ctypes.string_at(output + 19 + written, output_length - 19 - written) == b'\xA5' * (output_length - 19 - written)
            rows.append(dict(phase=phase, statistics=stats,
                             input_allocation_bytes=source_length, output_allocation_bytes=output_length))
    finally:
        api.size_dealloc(output, output_length)
        api.size_dealloc(source, source_length)
    return rows


def public_heap_self_test():
    alphabet = b'123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
    for size in [0, 1, 2, 8, 32, 64, 512, 4096]:
        data, text = public_heap_fixture(size)
        value = 0
        for digit in text:
            value = value * 58 + alphabet.index(digit)
        assert value == int.from_bytes(data, 'big') and len(data) == size
    stats = dict(zip(PUBLIC_HEAP_METRICS, [2, 24, 1, 8, 1, 16, 32, 2, 40, 0, 5, 45, 5]))
    public_heap_stats(stats)
    assert stats['requests'] == 3 and stats['peak_extra_bytes'] == 40 and stats['retained_delta_bytes'] == 0


def public_heap(args):
    if not args.build or not args.output:
        raise SystemExit('public-heap requires --build and --output')
    assert args.repeats > 0 and len(set(args.sizes)) == len(args.sizes)
    assert all(0 <= size <= 4194304 for size in args.sizes)
    build = args.build.resolve()
    metadata = json.loads((build / 'metadata.json').read_text())
    assert metadata['count_allocations'], 'requires instrumented artifacts'
    with (build / 'sizes.csv').open() as stream:
        artifacts = list(csv.DictReader(stream))
    assert all(row['backend'] in ['empty', 'auto', 'no-alloc'] and row['kind'] == 'cdylib'
               and row['direction'] == 'both' for row in artifacts)
    assert len({row['backend'] for row in artifacts}) == len(artifacts) and artifacts
    wasm = metadata['target'] == 'wasm32-unknown-unknown'
    if not wasm:
        host = subprocess.check_output(['rustc', '-vV'], text=True).split('host: ')[1].splitlines()[0]
        assert host == metadata['target'], 'native target must match this host'
    args.output.mkdir(parents=True, exist_ok=False)
    output = args.output.resolve()
    for size in args.sizes:
        data, text = public_heap_fixture(size)
        (output / f'{size}.bin').write_bytes(data)
        (output / f'{size}.txt').write_bytes(text)
    record = dict(measured_utc=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), build=metadata,
                  host=platform.platform(), machine=platform.machine(), repeats=args.repeats,
                  oracle='integer geometric series: (58**digits - 1)//57; text is all 2',
                  clock='none; untimed allocation windows', sizes=args.sizes, status='running',
                  harness_sha256={name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
                                  for name in ['scripts/resource_report.py', 'scripts/wasm_allocations.mjs']})
    (output / 'metadata.json').write_text(json.dumps(record, indent=2))
    rows = []
    with (output / 'raw.jsonl').open('x') as raw:
        for artifact in artifacts:
            extension = '.wasm' if wasm else '.dylib' if 'apple' in metadata['target'] else '.so'
            path = build / (artifact['backend'] + '-both-cdylib' + extension + '.stripped')
            assert hashlib.sha256(path.read_bytes()).hexdigest() == artifact['stripped_sha256']
            for size in args.sizes:
                if artifact['backend'] in ['empty', 'no-alloc'] and size > 65536:
                    continue
                for operation in ['encode', 'decode']:
                    for repeat in range(args.repeats):
                        job = dict(artifact=str(path), backend=artifact['backend'], operation=operation,
                                   data=str(output / f'{size}.bin'), text=str(output / f'{size}.txt'))
                        command = ['node', str(ROOT / 'scripts/wasm_allocations.mjs'), '--worker'] if wasm else \
                            [sys.executable, '-B', str(Path(__file__).resolve()), 'public-heap', '--worker']
                        events = json.loads(subprocess.check_output(command + [json.dumps(job)], text=True, timeout=120))
                        for event in events:
                            stats = event.pop('statistics')
                            public_heap_stats(stats)
                            io_bytes = event['input_allocation_bytes'] + event['output_allocation_bytes']
                            if event['phase'] == 'cold':
                                assert stats['live_before_bytes'] == io_bytes
                            if artifact['backend'] != 'auto':
                                assert all(stats[name] == 0 for name in PUBLIC_HEAP_METRICS[:10])
                            if not metadata['cache'] or event['phase'] == 'warm':
                                assert stats['retained_delta_bytes'] == 0
                            row = dict(backend=artifact['backend'], operation=operation, decoded_bytes=size,
                                       repeat=repeat, **event, **stats,
                                       peak_internal_bytes=stats['peak_live_bytes'] - io_bytes,
                                       retained_internal_bytes=stats['live_after_bytes'] - io_bytes)
                            raw.write(json.dumps(row) + '\n')
                            raw.flush()
                            rows.append(row)
            print(f"Counted {artifact['backend']}: {metadata['target']}, cache={metadata['cache']}", flush=True)
    write_csv(output / 'heap.csv', rows)
    record['status'], record['rows'] = 'complete', len(rows)
    (output / 'metadata.json').write_text(json.dumps(record, indent=2))


def self_test():
    public_heap_self_test()
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
    public_parser = sub.add_parser("public-heap", help="count public native/Wasm consumers built by code_size.py")
    public_parser.add_argument("--build", type=Path)
    public_parser.add_argument("--output", type=Path)
    public_parser.add_argument("--sizes", type=lambda value: [int(n) for n in value.split(',')], default=DEFAULT_SIZES)
    public_parser.add_argument("--repeats", type=int, default=3)
    public_parser.add_argument("--worker", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.action == "public-heap":
        if args.worker:
            print(json.dumps(public_heap_worker(json.loads(args.worker))))
        else:
            public_heap(args)
    elif args.action == "build":
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

"""Independently check a completed Wasm campaign and optional matching host run."""

import argparse
import csv
import hashlib
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
import statistics


def sha(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def read_csv(path):
    with path.open(newline="") as source:
        return list(csv.DictReader(source))


def key(row):
    return row["backend"], row["operation"], int(row["decoded_bytes"]), int(row.get("repeat", 0))


def skip_reason(backend, size, operation=None, encoded_bytes=None):
    if backend == "five8" and size not in (32, 64):
        return "five8 supports only 32- and 64-byte decoded fixtures"
    if backend in ("batched", "no-alloc", "bs58") and size > 65536:
        return "quadratic backend capped at 65536 decoded bytes"
    if backend == "matrix-forced":
        if size == 0:
            return "forced matrix control excludes the degenerate empty input"
        if operation == "encode" and size > 8192:
            return "forced matrix encoder supports at most 8192 decoded bytes"
        if operation == "decode":
            assert encoded_bytes is not None
            if encoded_bytes > 5632:
                return "forced matrix decoder supports at most 5632 encoded characters; larger inputs use fallback"
    return None


def shuffled_jobs(jobs, seed):
    result, state = list(jobs), seed
    for index in range(len(result) - 1, 0, -1):
        state = (state ^ (state << 13)) & 0xFFFFFFFF
        state = (state ^ (state >> 17)) & 0xFFFFFFFF
        state = (state ^ (state << 5)) & 0xFFFFFFFF
        other = state % (index + 1)
        result[index], result[other] = result[other], result[index]
    return result


def backend_registry(backends):
    result = []
    for backend in backends:
        maximum = 65536 if backend in ("batched", "no-alloc", "bs58") else 64 if backend == "five8" else 4194304
        row = dict(backend=backend, max_encode_bytes=maximum, max_decode_bytes=maximum)
        if backend == "five8":
            row.update(supported_encode_bytes=[32, 64], supported_decode_bytes=[32, 64])
        if backend == "matrix-forced":
            row.update(min_encode_bytes=1, max_encode_bytes=8192, min_decode_bytes=1,
                       max_decode_bytes=5632, min_decode_chars=1, max_decode_chars=5632)
        result.append(row)
    return result


def fixture_data(size, pattern):
    result = bytearray(size)
    if pattern.startswith("xorshift64 seed decoded length + 20261023;"):
        state, mask, shifts = size + 20_261_023, (1 << 64) - 1, (13, 7, 17)
    elif pattern.startswith("xorshift32 seed 0x9e3779b9 XOR decoded length;"):
        state, mask, shifts = (0x9E3779B9 ^ size) & 0xFFFFFFFF, 0xFFFFFFFF, (13, 17, 5)
    else:
        raise AssertionError(f"unknown fixture generator: {pattern}")
    for index in range(size):
        state = (state ^ (state << shifts[0])) & mask
        state = (state ^ (state >> shifts[1])) & mask
        state = (state ^ (state << shifts[2])) & mask
        result[index] = state & 255
    if size:
        result[0] |= 1
    return result


def fnv1a64(data):
    value = 0xCBF29CE484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001B3) & ((1 << 64) - 1)
    return f"{value:016x}"


def close(actual, expected):
    assert math.isclose(float(actual), expected, rel_tol=1e-14, abs_tol=1e-10), (actual, expected)


def module_identity(meta):
    return sorted((row["backend"], row["direction"], row["bytes"], row["sha256"])
                  for row in meta["modules"])


def validate(output, modules, scripts):
    complete = json.loads((output / "complete.json").read_text())
    meta = json.loads((output / "metadata.json").read_text())
    assert set(complete["sha256"]) == {"metadata.json", "samples.csv", "summary.csv", "skipped.csv"}
    for name, want in complete["sha256"].items():
        assert sha(output / name) == want, name
    assert sha(scripts / "wasm_tradeoffs.mjs") == meta["driver_sha256"]
    assert sha(scripts / "code_size_wasm.mjs") == meta["helper_sha256"]
    assert sha(modules / "sizes.csv") == meta["sizes_sha256"]
    assert sha(modules / "metadata.json") == meta["build_metadata_sha256"]
    assert json.loads((modules / "metadata.json").read_text()) == meta["build_metadata"]
    assert len(meta["backends"]) == len(set(meta["backends"])) > 0
    assert len(meta["sizes"]) == len(set(meta["sizes"])) > 0
    repeats = meta.get("repeats", 1)
    assert isinstance(repeats, int) and repeats > 0
    assert isinstance(meta["samples"], int) and meta["samples"] > 0 and meta["samples"] % 2 == 1
    assert meta["minimum_batch_ns"] >= 3_000_000
    if meta.get("schema", 1) >= 2:
        assert meta["input_pattern"].startswith("xorshift64 seed decoded length + 20261023;")
    if meta.get("schema", 1) >= 3:
        assert meta["timing_protocol"] == "base58-timing-20261024-v1"
        assert meta["decode_timing_contract"] == "prevalidated-utf8-repeat-v1"
        assert meta["build_metadata"]["decode_timing_contract"] == meta["decode_timing_contract"]
        assert meta["repeat_timing_contract"] == "opaque-slice-lengths-v1"
        assert meta["build_metadata"]["repeat_timing_contract"] == meta["repeat_timing_contract"]
        assert meta["primary_clock"] == "elapsed_ns"
        assert meta["requires_native_fixture_crosscheck"] is True
        assert all(isinstance(size, int) and 0 <= size <= 4194304 for size in meta["sizes"])
        assert meta["backend_registry"] == backend_registry(meta["backends"])
        policy = meta["sampling_policy"]
        assert policy["kind"] == "fixed-warm-batches"
        assert policy["warm_batches_per_process"] == meta["samples"]
        assert policy["process_repeats"] == repeats
        assert policy["minimum_batch_clock"] == "elapsed_ns"
        assert policy["maximum_iterations"] == 10_000_000
        assert (policy["calibration_growth_min"], policy["calibration_growth_max"]) == (2, 16)
    directions = meta.get("module_directions", ["encode", "decode", "both"])
    assert "both" in directions and len(directions) == len(set(directions))
    assert set(directions) <= {"encode", "decode", "both"}
    expected_modules = {(backend, direction) for backend in meta["backends"] for direction in directions}
    assert len(module_identity(meta)) == len(set(module_identity(meta))) == len(expected_modules)
    assert {(row["backend"], row["direction"]) for row in meta["modules"]} == expected_modules
    sizes = read_csv(modules / "sizes.csv")
    for module in meta["modules"]:
        path = modules / Path(module["path"]).name
        assert path.stat().st_size == module["bytes"]
        assert sha(path) == module["sha256"], path
        rows = [row for row in sizes if row["backend"] == module["backend"]
                and row["direction"] == module["direction"]]
        assert len(rows) == 1
        assert rows[0]["stripped_sha256"] == module["sha256"]
        assert int(rows[0]["stripped_file_bytes"]) == module["bytes"]
        if meta.get("schema", 1) >= 3:
            for field, value in meta["build_metadata"]["backend_semantics"][module["backend"]].items():
                assert module[field] == value
    fixtures = {row["decoded_bytes"]: row for row in meta["fixtures"]}
    assert len(fixtures) == len(meta["fixtures"]) == len(meta["sizes"])
    assert set(fixtures) == set(meta["sizes"])
    for size, fixture in fixtures.items():
        data = output / f"fixtures/{size}.bin"
        text = output / f"fixtures/{size}.txt"
        assert data.stat().st_size == size
        assert text.stat().st_size == fixture["encoded_bytes"]
        assert sha(data) == fixture["data_sha256"]
        assert sha(text) == fixture["encoded_sha256"]
        pattern = meta.get("input_pattern", "xorshift32 seed 0x9e3779b9 XOR decoded length;")
        assert data.read_bytes() == fixture_data(size, pattern), f"fixture generator differs: {size}"
        if meta.get("schema", 1) >= 2:
            assert fnv1a64(data.read_bytes()) == fixture["data_fnv1a64"]
            assert fnv1a64(text.read_bytes()) == fixture["encoded_fnv1a64"]
    expected = {(backend, operation, size, repeat) for backend in meta["backends"]
                for operation in ["encode", "decode"] for size in meta["sizes"]
                for repeat in range(repeats) if not skip_reason(backend, size, operation, fixtures[size]["encoded_bytes"])}
    if meta.get("schema", 1) >= 3:
        assert meta["job_order_algorithm"] == "xorshift32-fisher-yates-v1"
        assert isinstance(meta["job_order_seed"], int) and 0 < meta["job_order_seed"] <= 0xFFFFFFFF
        canonical = [dict(backend=backend, operation=operation, decoded_bytes=size, repeat=repeat)
                     for backend in meta["backends"] for size in meta["sizes"]
                     for operation in ("encode", "decode") for repeat in range(repeats)
                     if not skip_reason(backend, size, operation, fixtures[size]["encoded_bytes"])]
        assert meta["jobs"] == shuffled_jobs(canonical, meta["job_order_seed"])
    rows = read_csv(output / "summary.csv")
    if meta.get("schema", 1) >= 2:
        assert all("repeat" in row for row in rows)
    summary = {key(row): row for row in rows}
    assert len(rows) == len(summary) == complete["completed_cases"] == meta["expected_cases"] == len(expected)
    assert set(summary) == expected
    if meta.get("schema", 1) >= 3:
        assert [key(row) for row in rows] == [key(job) for job in meta["jobs"]]
    skipped = read_csv(output / "skipped.csv")
    expected_skipped = {(backend, operation, size, 0): skip_reason(backend, size, operation, fixtures[size]["encoded_bytes"])
                        for backend in meta["backends"] for operation in ["encode", "decode"]
                        for size in meta["sizes"] if skip_reason(backend, size, operation, fixtures[size]["encoded_bytes"])}
    assert len(skipped) == len(expected_skipped)
    assert {key(row): row["reason"] for row in skipped} == expected_skipped
    samples = defaultdict(list)
    phases = Counter()
    modules_by_backend = {row["backend"]: row for row in meta["modules"] if row["direction"] == "both"}
    observed_order = []
    for row in read_csv(output / "samples.csv"):
        if meta.get("schema", 1) >= 2:
            assert "repeat" in row
        case = key(row)
        assert case in expected
        if not observed_order or observed_order[-1] != case:
            observed_order.append(case)
        samples[case].append(row)
        phases[row["phase"]] += 1
        elapsed, iterations = int(row["elapsed_ns"]), int(row["iterations"])
        assert elapsed > 0 and 0 < iterations < 0x80000000
        if meta.get("schema", 1) >= 3:
            assert iterations <= meta["sampling_policy"]["maximum_iterations"]
        close(row["ns_per_call"], elapsed / iterations)
        backend, operation, size, _ = case
        assert int(row["encoded_bytes"]) == fixtures[size]["encoded_bytes"]
        assert int(row["input_bytes"]) == (size if operation == "encode" else fixtures[size]["encoded_bytes"])
        assert row["module_sha256"] == modules_by_backend[backend]["sha256"]
        assert int(row["module_bytes"]) == modules_by_backend[backend]["bytes"]
        assert int(row["linear_memory_pages"]) > 0
        for field in ["input_bytes", "encoded_bytes", "module_bytes", "module_sha256"]:
            assert row[field] == summary[case][field]
    assert set(samples) == expected
    if meta.get("schema", 1) >= 3:
        assert observed_order == [key(job) for job in meta["jobs"]]
    for case, rows in samples.items():
        assert [row["phase"] for row in rows[:2]] == ["cold", "second"]
        assert all(int(row["iterations"]) == 1 and int(row["sample"]) == 0 for row in rows[:2])
        assert sum(row["phase"] == "cold" for row in rows) == 1
        assert sum(row["phase"] == "second" for row in rows) == 1
        sample_index = 0
        if meta.get("schema", 1) >= 3:
            second = int(rows[1]["elapsed_ns"])
            maximum = meta["sampling_policy"]["maximum_iterations"]
            next_iterations = 1 if second >= meta["minimum_batch_ns"] else min(maximum, max(1, math.ceil(meta["minimum_batch_ns"] * 1.25 / second)))
        for row in rows[2:]:
            assert int(row["sample"]) == sample_index
            assert row["phase"] in ["warm", "calibration"]
            if meta.get("schema", 1) >= 3:
                assert int(row["iterations"]) == next_iterations
            if row["phase"] == "warm":
                assert int(row["elapsed_ns"]) >= meta["minimum_batch_ns"]
                sample_index += 1
            else:
                assert int(row["elapsed_ns"]) < meta["minimum_batch_ns"]
                if meta.get("schema", 1) >= 3:
                    assert next_iterations < maximum
                    growth = min(16, max(2, math.ceil(meta["minimum_batch_ns"] * 1.25 / int(row["elapsed_ns"]))))
                    next_iterations = min(maximum, next_iterations * growth)
        assert sample_index == meta["samples"]
        pages = [int(row["linear_memory_pages"]) for row in rows]
        assert pages == sorted(pages)
        warm = [row for row in rows if row["phase"] == "warm"]
        times = [int(row["elapsed_ns"]) / int(row["iterations"]) for row in warm]
        center = statistics.median(times)
        for field, want in [("warm_median_ns", center), ("warm_min_ns", min(times)),
                            ("warm_max_ns", max(times)),
                            ("warm_mad_ns", statistics.median(abs(t - center) for t in times)),
                            ("cold_ns", int(rows[0]["elapsed_ns"])),
                            ("second_ns", int(rows[1]["elapsed_ns"])),
                            ("cold_pages", pages[0]), ("second_pages", pages[1]),
                            ("warm_max_pages", max(int(row["linear_memory_pages"]) for row in warm))]:
            close(summary[case][field], want)
    return meta, {"completed_cases": len(summary), "batches": sum(phases.values()),
                  "phases": dict(phases), "node": meta["node"], "v8": meta["versions"]["v8"],
                  "cpu_model": meta["cpu_model"], "driver_sha256": meta["driver_sha256"],
                  "helper_sha256": meta["helper_sha256"],
                  "validation": "completion/module/build metadata/fixture/driver/helper hashes; complete unique grid; every summary statistic; sample phase/index/duration/page invariants"}


def self_test():
    assert fixture_data(8, "xorshift64 seed decoded length + 20261023;").hex() == "37a0bb4ee07bcfec"
    assert fnv1a64(b"a") == "af63dc4c8601ec8c"
    assert skip_reason("ntt-same-owned", 4194304, "decode", 5720000) is None
    assert skip_reason("matrix-forced", 4097, "decode", 5596) is None
    assert skip_reason("matrix-forced", 8192, "decode", 11188)
    assert skip_reason("matrix-forced", 8193, "encode")
    assert skip_reason("matrix-forced", 0, "decode", 0)
    assert skip_reason("bs58", 65537, "decode", 90000)
    assert skip_reason("five8", 33, "encode")
    assert backend_registry(["ntt-same-cached"])[0]["max_encode_bytes"] == 4194304
    assert shuffled_jobs(list(range(10)), 20261024) == [0, 4, 8, 5, 7, 9, 1, 2, 3, 6]
    print("Wasm campaign validator self-check passed")


if __name__ == "__main__":
    if not __debug__:
        raise SystemExit("run without Python -O: validation uses assertions")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, nargs="?")
    parser.add_argument("modules", type=Path, nargs="?")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--scripts-dir", type=Path, default=Path(__file__).resolve().parent,
                        help="frozen driver/helper directory (default: repository scripts)")
    parser.add_argument("--compare", type=Path, help="previous campaign with identical frozen inputs")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        raise SystemExit(0)
    if not args.output or not args.modules:
        parser.error("output and modules are required")
    meta, result = validate(args.output, args.modules, args.scripts_dir)
    if args.compare:
        previous, _ = validate(args.compare, args.modules, args.scripts_dir)
        for field in ["node", "driver_sha256", "helper_sha256", "sizes_sha256", "build_metadata_sha256",
                      "fixtures", "sizes", "backends", "samples", "minimum_batch_ns", "worker_flags", "parent_flags"]:
            assert previous[field] == meta[field], field
        assert previous.get("repeats", 1) == meta.get("repeats", 1)
        assert previous.get("input_pattern") == meta.get("input_pattern")
        for field in ("timing_protocol", "decode_timing_contract", "repeat_timing_contract", "sampling_policy", "jobs", "job_order_seed", "job_order_algorithm", "module_directions"):
            assert previous.get(field) == meta.get(field), field
        assert previous["versions"]["v8"] == meta["versions"]["v8"]
        assert module_identity(previous) == module_identity(meta)
        result["identical_inputs_node_v8_and_flags_to"] = str(args.compare)
        result["other_runtime_version_differences"] = {
            field: {"previous": previous["versions"].get(field), "current": meta["versions"].get(field)}
            for field in sorted(set(previous["versions"]) | set(meta["versions"]))
            if previous["versions"].get(field) != meta["versions"].get(field)
        }
    print(json.dumps(result, indent=2))

#!/usr/bin/env python3
"""Validate one benchmark cohort and export entry-point tables, controls and figures."""

import argparse
from collections import defaultdict
import csv
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import statistics
import sys
import tempfile
from datetime import datetime, timezone

PROTOCOL = "base58-timing-20261024-v1"
GRID = [0, 1, 8, 16, 21, 22, 23, 31, 32, 33, 63, 64, 65, 128, 256, 512,
        1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144,
        524288, 1048576, 2097152, 4194304]
BACKENDS = ["auto", "batched", "karatsuba", "toom3", "ntt", "ntt-cached",
            "ntt-portable", "malachite", "matrix", "matrix-portable", "bs58", "five8"]
CONTROLS = ["matrix-forced", "ntt-same-owned", "ntt-same-cached"]
ALL_BACKENDS = [*BACKENDS, *CONTROLS]
LABELS = {"auto": "Auto", "batched": "Batched", "karatsuba": "Karatsuba (dc)",
          "toom3": "Toom-3", "ntt": "NTT", "ntt-cached": "NTT cached",
          "ntt-portable": "NTT portable", "malachite": "Malachite (dev adapter)",
          "matrix": "Matrix", "matrix-portable": "Matrix portable",
          "bs58": "bs58", "five8": "five8 (32 / 64 B only)",
          "matrix-forced": "Forced general matrix", "ntt-same-owned": "Same-leaf NTT, owned twiddles",
          "ntt-same-cached": "Same-leaf NTT, cached twiddles"}
COLORS = dict(zip(BACKENDS, ["#0072B2", "#E69F00", "#009E73", "#D55E00", "#7B3294",
                           "#CC79A7", "#7B3294", "#777777", "#2AA198", "#2AA198", "#111111", "#A63A50"]))
TITLES = {"gcp-default": "GCP · generic x86-64", "gcp-native": "GCP · target-cpu=native",
          "m5-native": "M5 laptop · native Rust", "m5-wasm": "M5 laptop · Wasm / V8"}
# The static route model below is valid only for these reviewed dispatch sources.
ROUTE_SOURCES = {
    "src/api.rs": "9b3cb89ebd918f65a2de08165931d15b7343d3c5b15da1589e4a82a4c399eb87",
    "src/lib.rs": "5129ca2de9f01d8c0801f4847ac5fc796ca36932d4c3fe247059694d2df074c6",
    "src/common.rs": "a79be333cc2224d1899f553f6fac5505a4773cb2132ffebf0d40a11a774c950c",
    "src/encode/mod.rs": "100193f1d7342ccb0d3aa8aeadc35f55009b7a95d0376acd5b76b85be807a7d6",
    "src/encode/matrix.rs": "f45dcab2ecf6b3a0dc75508fa156e573ea866d05d819c19e8c17e777753d291e",
    "src/encode/matrix/fixed.rs": "a08fdf6823238383e5e021ada164ad5ab8b20b40f15f82f551d3e1f85fb8b9e4",
    "src/encode/matrix/table.rs": "d5978121718be0bac3151874da7a6f3de98170f9f8592200c85d1ab60f445575",
    "src/matrix/mod.rs": "f4f0ec356ebff1389ea426ef7ae271ada79d1bb8bc88d6e17ed2320d561119ba",
    "src/matrix/x86.rs": "e5a8d7aa9308af64a40a7858f915e05061f0240841b79dcf0349aa7d4bd6626d",
    "src/matrix/table.rs": "fecf6a1032887b2dda95f5841e0432bf44601bd15ff4ad84d42c7747eb7f1b6f",
    "src/encode/dc.rs": "d348139f23c17c890077361a87f59240168faab1608608c628c981f222e14ff0",
    "src/dc.rs": "2b81e50b98be6406f9ff69ff8c9b8ec75f6e639b5c39de3934ce4c2810fb65ca",
    "src/mul.rs": "274c831b9b30a0c3241f80e90d8c4bae34db08e17a88d8c98a8f4ded697ef4bd",
    "src/toom3.rs": "9250537f493eb19db40b59b8779ed1cb2a1ac9f7fe67ee70435ea12c96f0484b",
    "src/ntt/mod.rs": "d14de8ebfd9b0b6dc646e0a8dd91a67fa455bbddd51708b529be2abd2c5a3ca1",
    "tests/support/malachite.rs": "dcda243ddcb55805ca179afe6d2cfeba8a2d7a3039654712f8c74513d761e982",
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def read_csv(path):
    with path.open(newline="") as source:
        return list(csv.DictReader(source))


def backend(name):
    return "karatsuba" if name == "dc" else name


def key(row, size_field):
    return backend(row["backend"]), row["operation"], integer(row[size_field])


def integer(value, minimum=0):
    result = int(value)
    require(str(result) == str(value) and result >= minimum, f"invalid integer: {value!r}")
    return result


def number(value, positive=True):
    result = float(value)
    require(math.isfinite(result) and (result > 0 if positive else result >= 0), f"invalid timing: {value!r}")
    return result


def close(actual, expected, context):
    require(math.isclose(number(actual, positive=expected > 0), expected, rel_tol=1e-13, abs_tol=1e-10),
            f"summary/raw mismatch: {context}: {actual} != {expected}")


def supported(name, operation, size, chars):
    if name == "five8":
        return size in (32, 64)
    if name in ("bs58", "batched"):
        return size <= 65536
    if name == "matrix-forced":
        return 1 <= size <= 8192 if operation == "encode" else 1 <= size <= 5632 and 1 <= chars <= 5632
    return True


def case_grid(sizes, names):
    return {(name, operation, size) for name in names for operation in ("encode", "decode") for size in sizes}


def expected_keys(sizes, names, fixtures):
    return {case for case in case_grid(sizes, names) if supported(*case, fixtures[case[2]][0])}


def settings(meta, wasm=False):
    args = meta if wasm else meta["arguments"]
    sizes = meta["sizes"] if wasm else [int(value) for value in args["sizes"].split(",")]
    names = [backend(value) for value in (meta["backends"] if wasm else args["backends"].split(","))]
    require(meta.get("timing_protocol") == PROTOCOL, "different benchmark cohort; sealed earlier evidence must stay separate")
    require(sizes == GRID and len(names) == len(set(names)) and set(names) == set(ALL_BACKENDS),
            "unexpected size grid or entry/control set")
    repeats, samples = integer(args["repeats"], 1), integer(args["samples"], 1)
    require(repeats % 2 == samples % 2 == 1, "sampling counts must be positive odd integers")
    return sizes, names, repeats, samples


def validate_jobs(meta, groups, cases, repeats):
    jobs = [(*key(row, "decoded_bytes"), integer(row["repeat"])) for row in meta["jobs"]]
    wanted = {(*case, repeat) for case in cases for repeat in range(repeats)}
    require(len(jobs) == len(set(jobs)) and set(jobs) == wanted, "declared job order is incomplete or duplicated")
    require(jobs == list(groups), "raw process order differs from saved job order")
    require(integer(meta["job_order_seed"]) == 20261024, "unexpected job-order seed")


def validate_registry(meta, sizes, names, fixtures):
    entries = meta["backend_registry"]
    registry = {backend(row["backend"]): row for row in entries}
    require(len(registry) == len(entries) and set(names) <= registry.keys(), "duplicate/incomplete backend registry")
    for name, operation, size in case_grid(sizes, names):
        row, chars = registry[name], fixtures[size][0]
        actual = row.get(f"min_{operation}_bytes", 0) <= size <= row[f"max_{operation}_bytes"]
        actual = actual and row.get(f"min_{operation}_chars", 0) <= chars <= row.get(f"max_{operation}_chars", 1 << 64)
        if f"supported_{operation}_bytes" in row:
            actual = actual and size in row[f"supported_{operation}_bytes"]
        require(actual == supported(name, operation, size, chars), f"unexpected support: {name}/{operation}/{size}/{chars}")


def validate_skips(directory, size_field, sizes, names, cases):
    rows = read_csv(directory / "skipped.csv")
    got = [key(row, size_field) for row in rows]
    wanted = case_grid(sizes, names) - cases
    require(len(got) == len(set(got)) and set(got) == wanted,
            f"missing or unexpected support skips: {directory}")
    require(all(row["reason"].strip() for row in rows), f"skip without reason: {directory}")
    return {key(row, size_field): row["reason"] for row in rows}


def source_files(metadata, wasm=False):
    if wasm:
        files = metadata["build_metadata"]["source_files_sha256"]
    else:
        entries = metadata["build"]["source"]["files"]
        files = {row["path"]: row["sha256"] for row in entries}
        require(len(files) == len(entries), "duplicate native source paths")
        require(metadata["build"]["worker"]["source_sha256"] == metadata["build"]["source"]["sha256"],
                "worker and native build source identities differ")
    files = {"tests/support/" + name.split("/tests/support/", 1)[1]
             if "/tests/support/" in name else name: digest for name, digest in files.items()}
    shared = {name: digest for name, digest in files.items()
              if name in ("Cargo.toml", "Cargo.lock") or name.startswith(("src/", "tests/support/"))}
    require({"Cargo.toml", "Cargo.lock", "src/lib.rs", "tests/support/malachite.rs",
             "tests/support/competitors.rs"} <= shared.keys(),
            "incomplete source identity")
    require(all(len(value) == 64 and set(value) <= set("0123456789abcdef") for value in shared.values()),
            "invalid source SHA256")
    return shared


def collect_processes(rows, size_field, timer_field, repeats, samples, cases):
    groups = defaultdict(list)
    for row in rows:
        case = key(row, size_field)
        require(case in cases, f"unexpected timed case: {case}")
        repeat = integer(row.get("repeat", 0))
        require(repeat < repeats, f"unexpected repeat: {repeat}")
        require(row["phase"] in ("cold", "second", "calibration", "warm"), "unknown phase")
        integer(row["sample"])
        integer(row["iterations"], 1)
        number(row[timer_field], positive=row["phase"] == "warm")
        groups[(*case, repeat)].append(row)
    expected = {(*case, repeat) for case in cases for repeat in range(repeats)}
    require(set(groups) == expected, "incomplete timed process grid")
    processes = []
    for (name, operation, size, repeat), group in sorted(groups.items()):
        for phase in ("cold", "second"):
            phase_rows = [row for row in group if row["phase"] == phase]
            require(len(phase_rows) == 1 and integer(phase_rows[0]["iterations"]) == 1,
                    f"missing/duplicate {phase} call: {(name, operation, size, repeat)}")
        warm = [row for row in group if row["phase"] == "warm"]
        require(sorted(integer(row["sample"]) for row in warm) == list(range(samples)),
                f"incomplete warmed sample grid: {(name, operation, size, repeat)}")
        values = [number(row[timer_field]) / integer(row["iterations"], 1) for row in warm]
        processes.append({"backend": name, "operation": operation, "decoded_bytes": size, "repeat": repeat,
                          "warm_batches": len(values), "median_ns": statistics.median(values),
                          "batch_min_ns": min(values), "batch_max_ns": max(values)})
    return processes, groups


def aggregate(processes):
    groups = defaultdict(list)
    for row in processes:
        groups[key(row, "decoded_bytes")].append(row["median_ns"])
    return [{"backend": name, "operation": operation, "decoded_bytes": size,
             "processes": len(values), "median_ns": statistics.median(values),
             "process_min_ns": min(values), "process_max_ns": max(values)}
            for (name, operation, size), values in sorted(groups.items())]


def native_campaign(directory, slug):
    files = ["metadata.json", "raw.jsonl", "summary.csv", "skipped.csv"]
    input_hashes = {str(directory / name): sha(directory / name) for name in files}
    meta = json.loads((directory / "metadata.json").read_text())
    args = meta["arguments"]
    require(meta["status"] == "complete" and args["mode"] == "timing", "incomplete/non-timing native campaign")
    require(meta["build"]["worker"]["count_allocations"] is False, "counted build used for timing")
    require(meta["oracle"] == meta["build"]["worker"]["oracle"] == "malachite-natural-radix-conversion",
            "native cohort requires an independent Natural radix-conversion fixture oracle")
    sizes, names, repeats, samples = settings(meta)
    policy = meta["sampling_policy"]
    require(policy["primary_clock"] == "cpu_ns", "native warm batches must use the process CPU clock")
    floor = integer(policy["minimum_batch_ns"], 1)
    env = meta["build"]["environment"]
    flags = " ".join(value or "" for value in env["flags"].values())
    if slug.startswith("gcp"):
        require(env["machine"] == "x86_64" and "Linux" in env["platform"], "GCP figure requires Linux x86-64")
        require(("target-cpu=native" in flags) == (slug == "gcp-native"), "GCP CPU flags do not match figure")
    else:
        require(env["machine"] in ("arm64", "aarch64") and "macOS" in env["platform"], "M5 native figure requires Apple ARM")
    raw = [json.loads(line) for line in (directory / "raw.jsonl").read_text().splitlines()]
    fixtures, io_cases = defaultdict(set), set()
    for row in raw:
        if row["kind"] == "io":
            case = (*key(row, "bytes"), integer(row["repeat"]))
            require(case not in io_cases, "duplicate native fixture case")
            io_cases.add(case)
            fixtures[integer(row["bytes"])].add((integer(row["encoded_chars"]),
                                                fingerprint(row["data_fnv1a64"]),
                                                fingerprint(row["encoded_fnv1a64"])))
    require(set(fixtures) == set(sizes) and all(len(values) == 1 for values in fixtures.values()),
            "native text fixtures disagree across cases")
    fixtures = {size: next(iter(values)) for size, values in fixtures.items()}
    for size, chars in meta["fixture_encoded_chars"].items():
        require(fixtures[integer(size)][0] == integer(chars), "native preflight and actual fixture lengths differ")
    cases = expected_keys(sizes, names, fixtures)
    validate_registry(meta, sizes, names, fixtures)
    skips = validate_skips(directory, "bytes", sizes, names, cases)
    require(io_cases == {(*case, repeat) for case in cases for repeat in range(repeats)}, "incomplete native fixture grid")
    timing = [row for row in raw if row["kind"] == "timing"]
    for row in timing:
        number(row["wall_ns"], positive=row["phase"] == "warm")
        if row["phase"] == "warm":
            require(integer(row["cpu_ns"]) >= floor, "short warmed native CPU batch")
    processes, groups = collect_processes(timing, "bytes", "cpu_ns", repeats, samples, cases)
    validate_jobs(meta, groups, cases, repeats)
    summary_groups = defaultdict(list)
    for row in timing:
        for metric, field in (("cpu_ns_per_call", "cpu_ns"), ("wall_ns_per_call", "wall_ns")):
            summary_groups[(*key(row, "bytes"), row["phase"], metric)].append(
                number(row[field], positive=row["phase"] == "warm") / integer(row["iterations"], 1))
    saved = [row for row in read_csv(directory / "summary.csv")
             if row["metric"] in ("cpu_ns_per_call", "wall_ns_per_call")]
    seen = set()
    for row in saved:
        case = (*key(row, "bytes"), row["phase"], row["metric"])
        require(case not in seen and case in summary_groups, "duplicate/unexpected native summary")
        seen.add(case)
        values = summary_groups[case]
        require(integer(row["count"]) == len(values), "native summary count mismatch")
        for field, expected in (("median", statistics.median(values)), ("minimum", min(values)), ("maximum", max(values))):
            close(row[field], expected, case)
    require(seen == set(summary_groups), "missing native saved summary")
    require(all(sha(Path(path)) == digest for path, digest in input_hashes.items()), "native inputs changed during validation")
    return {"slug": slug, "path": str(directory), "metadata": meta, "source": source_files(meta),
            "processes": processes, "observations": aggregate(processes), "timer": "process CPU",
            "input_sha256": input_hashes, "fixtures": fixtures, "skips": skips,
            "sizes": sizes, "backends": names, "repeats": repeats, "samples": samples,
            "cpu": cpu_context(meta),
            "runtime": env["rustc"].splitlines()[0], "flags": flags.strip() or "compiler defaults"}


def fixture_bytes(size):
    value = size + 20261023
    result = bytearray(size)
    mask = (1 << 64) - 1
    for index in range(size):
        value ^= (value << 13) & mask
        value ^= value >> 7
        value ^= (value << 17) & mask
        result[index] = value & 255
    if result:
        result[0] |= 1
    return result


def fingerprint(value):
    require(isinstance(value, str) and len(value) == 16 and set(value) <= set("0123456789abcdef"),
            f"invalid FNV-1a fixture fingerprint: {value!r}")
    return value


def fnv1a64(data):
    value = 0xcbf29ce484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001b3) & ((1 << 64) - 1)
    return f"{value:016x}"


def wasm_campaign(directory, modules_directory=None):
    meta = json.loads((directory / "metadata.json").read_text())
    complete = json.loads((directory / "complete.json").read_text())
    files = ["metadata.json", "samples.csv", "summary.csv", "skipped.csv"]
    input_hashes = {str(directory / name): sha(directory / name) for name in [*files, "complete.json"]}
    require(set(complete["sha256"]) == set(files), "unexpected Wasm completion file set")
    for name in files:
        require(sha(directory / name) == complete["sha256"][name], f"Wasm file changed: {name}")
    require(meta["schema"] == 3 and meta["decode_timing_contract"] == "prevalidated-utf8-repeat-v1",
            "Wasm cohort requires prevalidated UTF-8 outside each repeat timer")
    require(meta["repeat_timing_contract"] == meta["build_metadata"]["repeat_timing_contract"] == "opaque-slice-lengths-v1",
            "Wasm repeat loops must hide both slice pointers and lengths from the optimizer")
    require(meta["primary_clock"] == "elapsed_ns", "Wasm batches must identify the elapsed clock")
    require(meta["requires_native_fixture_crosscheck"] is True, "Wasm large fixtures require native oracle cross-checks")
    sizes, names, repeats, samples = settings(meta, True)
    fixtures = {integer(row["decoded_bytes"]): row for row in meta["fixtures"]}
    require(set(fixtures) == set(sizes) and len(fixtures) == len(meta["fixtures"]), "incomplete Wasm fixtures")
    fixture_fingerprints = {size: (row["encoded_bytes"], row["data_fnv1a64"], row["encoded_fnv1a64"])
                            for size, row in fixtures.items()}
    cases = expected_keys(sizes, names, fixture_fingerprints)
    validate_registry(meta, sizes, names, fixture_fingerprints)
    require(meta["platform"] == "darwin" and meta["arch"] == "arm64" and "M5" in meta["cpu_model"],
            "M5 Wasm figure requires Apple M5")
    require(meta["build_metadata"]["target"] == "wasm32-unknown-unknown" and
            meta["worker_flags"] == ["--no-liftoff", "--no-wasm-lazy-compilation", "--no-wasm-async-compilation"],
            "unexpected Wasm target or tiering flags")
    skips = validate_skips(directory, "decoded_bytes", sizes, names, cases)
    rows = read_csv(directory / "samples.csv")
    processes, groups = collect_processes(rows, "decoded_bytes", "elapsed_ns", repeats, samples, cases)
    validate_jobs(meta, groups, cases, repeats)
    saved = read_csv(directory / "summary.csv")
    require(len(saved) == len(groups) == complete["completed_cases"] == meta["expected_cases"],
            "Wasm completion count differs from process grid")
    seen = set()
    for row in saved:
        case = (*key(row, "decoded_bytes"), integer(row["repeat"]))
        require(case not in seen and case in groups, "duplicate/unexpected Wasm summary")
        seen.add(case)
        for phase in ("cold", "second"):
            single = next(item for item in groups[case] if item["phase"] == phase)
            close(row[f"{phase}_ns"], number(single["elapsed_ns"], positive=False), case)
        warm = [item for item in groups[case] if item["phase"] == "warm"]
        values = [number(item["elapsed_ns"]) / integer(item["iterations"], 1) for item in warm]
        center = statistics.median(values)
        for field, expected in (("warm_median_ns", center), ("warm_min_ns", min(values)),
                                ("warm_max_ns", max(values)),
                                ("warm_mad_ns", statistics.median(abs(value - center) for value in values))):
            if expected == 0:
                require(float(row[field]) == 0, f"nonzero Wasm MAD: {case}")
            else:
                close(row[field], expected, case)
    for size, fixture in fixtures.items():
        data, text = directory / f"fixtures/{size}.bin", directory / f"fixtures/{size}.txt"
        require(data.stat().st_size == size and sha(data) == fixture["data_sha256"], "Wasm decoded fixture changed")
        derived = fixture_bytes(size)
        require(hashlib.sha256(derived).hexdigest() == fixture["data_sha256"], "Wasm/native input pattern differs")
        require(text.stat().st_size == fixture["encoded_bytes"] and sha(text) == fixture["encoded_sha256"],
                "Wasm encoded fixture changed")
        require(fnv1a64(derived) == fingerprint(fixture["data_fnv1a64"]), "decoded fixture fingerprint mismatch")
        require(fnv1a64(text.read_bytes()) == fingerprint(fixture["encoded_fnv1a64"]), "encoded fixture fingerprint mismatch")
        input_hashes[str(data)] = fixture["data_sha256"]
        input_hashes[str(text)] = fixture["encoded_sha256"]
    modules = {row["backend"]: row for row in meta["modules"] if row["direction"] == "both"}
    require(set(modules) == set(names), "incomplete Wasm module identities")
    for module in modules.values():
        path = modules_directory / Path(module["path"]).name if modules_directory else Path(module["path"])
        require(path.stat().st_size == module["bytes"] and sha(path) == module["sha256"], "Wasm module bytes changed")
        input_hashes[str(path)] = module["sha256"]
    for row in rows:
        module = modules[row["backend"]]
        size = integer(row["decoded_bytes"])
        require(row["module_sha256"] == module["sha256"] and integer(row["module_bytes"]) == module["bytes"],
                "Wasm sample/module identity mismatch")
        require(integer(row["encoded_bytes"]) == fixtures[size]["encoded_bytes"], "Wasm sample fixture size mismatch")
        require(integer(row["input_bytes"]) == (size if row["operation"] == "encode" else fixtures[size]["encoded_bytes"]),
                "Wasm timed input size mismatch")
        close(row["ns_per_call"], number(row["elapsed_ns"], positive=row["phase"] == "warm") /
              integer(row["iterations"], 1), "Wasm sample normalization")
        if row["phase"] == "warm":
            require(integer(row["elapsed_ns"]) >= meta["minimum_batch_ns"], "short warmed Wasm batch")
    require(all(sha(Path(path)) == digest for path, digest in input_hashes.items()), "Wasm inputs changed during validation")
    return {"slug": "m5-wasm", "path": str(directory), "metadata": meta, "source": source_files(meta, True),
            "processes": processes, "observations": aggregate(processes), "timer": "Wasm elapsed",
            "input_sha256": input_hashes, "fixtures": fixture_fingerprints, "skips": skips,
            "sizes": sizes, "backends": names, "repeats": repeats, "samples": samples,
            "cpu": cpu_context(meta, True),
            "runtime": f"Node {meta['node']} · V8 {meta['versions']['v8']}",
            "flags": " ".join(meta["worker_flags"])}


def add_ratios(observations):
    baseline = {(row["operation"], row["decoded_bytes"]): row["median_ns"]
                for row in observations if row["backend"] == "bs58"}
    for row in observations:
        value = baseline.get((row["operation"], row["decoded_bytes"]))
        row["speedup_vs_bs58"] = value / row["median_ns"] if value is not None else ""


def cpu_context(meta, wasm=False):
    if wasm:
        return {"arch": "wasm32", "ifma": False, "avx2_compiled": False, "neon": False,
                "runtime_avx2": False, "runtime_sse4_1": False}
    worker = meta["build"]["worker"]
    compiled, runtime = worker["compiled_features"], worker["runtime_features"]
    require(all(type(compiled[name]) is bool for name in ("avx2", "neon", "little_endian")), "invalid compiled feature metadata")
    require(all(type(runtime[name]) is bool for name in ("avx2", "sse4_1", "avx512ifma", "avx512vbmi")),
            "invalid runtime feature metadata")
    require(compiled == meta["build"]["compiled_features"], "build/worker compiled features differ")
    cfg = set(meta["build"]["rustc_cfg"]["stdout"].splitlines())
    for name in ("avx2", "neon"):
        require(compiled[name] == (f'target_feature="{name}"' in cfg), "compiled feature/rustc cfg mismatch")
    require(compiled["little_endian"] == ('target_endian="little"' in cfg), "target endianness mismatch")
    require(worker["arch"] in ("x86_64", "aarch64"), "unsupported native route model")
    return {"arch": worker["arch"], "ifma": integer(worker["ifma_threshold"]) != 0,
            "avx2_compiled": compiled["avx2"], "neon": compiled["neon"] and compiled["little_endian"],
            "runtime_avx2": runtime["avx2"], "runtime_sse4_1": runtime["sse4_1"]}


def route(campaign, name, operation, size, chars):
    """Infer the reviewed top-level branch for these non-leading-zero fixtures."""
    cpu = campaign["cpu"]
    if name in ("bs58", "five8"):
        return {"route_inferred": f"{name} external entry point", "route_family": name,
                "leaf_policy": "external", "multiplier_policy": "external", "twiddle_policy": "not applicable"}
    if name == "auto":
        if operation == "encode":
            target = "batched" if size < 22 else "matrix"
        elif chars < (576 if cpu["neon"] else 1024):
            target = "batched"
        elif chars > 5632 or cpu["ifma"] and chars > 5120:
            target = "ntt-cached"
        elif cpu["ifma"] and cpu["avx2_compiled"]:
            target = "batched"
        else:
            target = "matrix" if cpu["neon"] or cpu["runtime_sse4_1"] else "batched"
        result = route(campaign, target, operation, size, chars)
        return {**result, "route_inferred": f"→ {target}: {result['route_inferred']}"}
    if name.startswith("matrix"):
        portable = name == "matrix-portable"
        panel = ("portable source" if portable else "NEON panel" if cpu["neon"] else
                 "AVX2 panel" if cpu["runtime_avx2"] else "SSE4.1 panel" if cpu["runtime_sse4_1"] else "portable source")
        if operation == "encode" and name != "matrix-forced" and size in (32, 64):
            policy = "portable source" if portable or size == 32 and cpu["arch"] == "x86_64" else "best kernel policy"
            return {"route_inferred": f"fixed {size}-byte matrix; {policy}", "route_family": f"fixed-matrix-{size}-{policy}",
                    "leaf_policy": "not applicable", "multiplier_policy": "matrix", "twiddle_policy": "not applicable"}
        if name != "matrix-forced":
            small = size < 22 if operation == "encode" else chars < 576
            large = size > (3584 if cpu["ifma"] else 8192) if operation == "encode" else chars > 5632
            if small:
                return route(campaign, "batched", operation, size, chars)
            if large:
                target = "ntt-cached" if operation == "encode" else "karatsuba"
                result = route(campaign, target, operation, size, chars)
                return {**result, "route_inferred": f"→ {target}: {result['route_inferred']}"}
        return {"route_inferred": f"general matrix; {panel}", "route_family": f"general-matrix-{panel}",
                "leaf_policy": "not applicable", "multiplier_policy": "matrix", "twiddle_policy": "not applicable"}
    is_ntt = name.startswith("ntt")
    cached = name in ("ntt-cached", "ntt-portable", "ntt-same-cached")
    same_leaves = name in ("ntt-same-owned", "ntt-same-cached")
    matrix_leaves = (cached or same_leaves) and (cpu["neon"] or operation == "encode" and cpu["arch"] == "x86_64")
    if operation == "encode":
        threshold = 512 if cpu["ifma"] else 4096 if is_ntt else 5120
        if name == "malachite":
            threshold = 1280 if cpu["ifma"] else 3072
        leaf = (96 if matrix_leaves and cpu["arch"] == "x86_64" else 768 if matrix_leaves else
                32 if is_ntt else 96 if name == "malachite" else 32 if cpu["ifma"] else 96)
        # encode_tuned can take one whole leaf after its initial byte threshold.
        if size:
            first = fixture_first_byte(size)
            bits = 8 * size - (8 - first.bit_length())
            minimum_digits = (((bits - 1) * 3148993499475808360) >> 64) + 1
            one_leaf = (minimum_digits + 11) // 10 <= leaf
        else:
            one_leaf = True
        small = size <= threshold
    else:
        small, one_leaf = chars <= 5120, False
    if name == "batched" or small or one_leaf and not matrix_leaves:
        return {"route_inferred": "batched" + (" (shared fallback)" if name != "batched" else ""),
                "route_family": "batched", "leaf_policy": "Horner", "multiplier_policy": "batched",
                "twiddle_policy": "not used"}
    if one_leaf:
        result = route(campaign, "matrix-forced", operation, size, chars)
        return {**result, "route_inferred": "single matrix leaf; " + result["route_inferred"]}
    multiplier = ("conditional NTT/Karatsuba" if is_ntt else "conditional Toom-3/Karatsuba" if name == "toom3"
                  else "Malachite adapter" if name == "malachite" else "Karatsuba/IFMA policy" if cpu["ifma"] else "Karatsuba policy")
    leaf = "matrix" if matrix_leaves else "Horner"
    twiddles = "cached" if cached else "owned" if is_ntt else "not applicable"
    text = f"recursive {multiplier}; {leaf} leaves"
    if is_ntt:
        text += f"; {twiddles} twiddles"
        if name == "ntt-portable":
            text += "; portable NTT source"
    kernel_policy = "portable NTT source" if name == "ntt-portable" else "best policy"
    return {"route_inferred": text, "route_family": f"recursive-{multiplier}-{leaf}-{twiddles}-{kernel_policy}",
            "leaf_policy": leaf, "multiplier_policy": multiplier, "twiddle_policy": twiddles}


def fixture_first_byte(size):
    value = size + 20261023
    value ^= (value << 13) & ((1 << 64) - 1)
    value ^= value >> 7
    value ^= (value << 17) & ((1 << 64) - 1)
    return (value & 255) | 1


def complete_observations(campaign):
    require(all(campaign["source"].get(name) == digest for name, digest in ROUTE_SOURCES.items()),
            "dispatch source differs from reviewed static route model")
    measured = {key(row, "decoded_bytes"): row for row in campaign["observations"]}
    add_ratios(campaign["observations"])
    result = []
    for name, operation, size in sorted(case_grid(campaign["sizes"], campaign["backends"])):
        case = name, operation, size
        common = {"timing_protocol": PROTOCOL, "environment": campaign["slug"], "timer": campaign["timer"],
                  "backend": name, "group": "control" if name in CONTROLS else "entry point",
                  "operation": operation, "decoded_bytes": size, "encoded_chars": campaign["fixtures"][size][0],
                  "warm_batches_per_process": campaign["samples"]}
        if case in measured:
            result.append({**common, "status": "measured", "skip_reason": "", **measured[case],
                           **route(campaign, name, operation, size, common["encoded_chars"])})
        else:
            result.append({**common, "status": "skipped", "skip_reason": campaign["skips"][case],
                           **{field: "" for field in ("processes", "median_ns", "process_min_ns", "process_max_ns",
                             "speedup_vs_bs58", "route_inferred", "route_family", "leaf_policy", "multiplier_policy", "twiddle_policy")}})
    return result


def lowest(rows):
    measured = [row for row in rows if row["status"] == "measured"]
    return min((row["median_ns"] for row in measured), default=None)


def table_time(row, best=None):
    if row["status"] != "measured":
        return "—"
    value = row["median_ns"]
    scale, unit = next(((scale, unit) for scale, unit in ((1e9, "s"), (1e6, "ms"), (1e3, "µs"), (1, "ns")) if value >= scale), (1, "ns"))
    center = f"{value / scale:.5g}"
    if value == best:
        center = f"**{center}**"
    return f"{center} [{row['process_min_ns'] / scale:.5g}, {row['process_max_ns'] / scale:.5g}] {unit}"


def markdown_cell(value):
    return str(value).replace("|", "\\|").replace("\n", " ")


def detailed_tables(campaign, observations, operation):
    lines = [f"# {TITLES[campaign['slug']]} · {operation}", "", "[Overview](README.md)", "",
             "Times are median [observed min, max] of process medians. Units apply to all three values. Bold marks the exact lowest observed median within the current entry-point or control group, including exact ties. Displayed values have five significant digits; comparisons use full precision. This is not a claim of statistical significance.", "",
             "Routes are static top-level inferences from the frozen source, fixture lengths and CPU configuration, not execution traces. Conditional NTT/Karatsuba does not prove a transform executed. Portable source can be compiler-vectorized and can call shared SIMD leaves/fallbacks. Owned twiddles do not mean cache-free: matrix-leaf tables can still be retained.", ""]
    lookup = {key(row, "decoded_bytes"): row for row in observations if row["operation"] == operation}
    for title, names in (("Entry points", BACKENDS), ("Controls", CONTROLS)):
        lines.extend([f"## {title}", ""])
        if title == "Controls":
            lines.extend(["Forced matrix bypasses wrapper dispatch only within its valid domain. The NTT controls use the same leaf and threshold policy; only twiddle ownership changes. Their minima are marked separately from entry points.", ""])
        for size in campaign["sizes"]:
            selected = [lookup[name, operation, size] for name in names]
            best = lowest(selected)
            lines.extend([f"### {size:,} decoded bytes · {campaign['fixtures'][size][0]:,} encoded characters", "",
                          "| Entry point | Inferred route or support reason | Median [process range] | bs58 / entry |",
                          "| :--- | :--- | ---: | ---: |"])
            for row in selected:
                ratio = row["speedup_vs_bs58"]
                detail = row["route_inferred"] if row["status"] == "measured" else "Skipped: " + row["skip_reason"]
                lines.append("| " + " | ".join(map(markdown_cell, [LABELS[row["backend"]], detail,
                    table_time(row, best), f"{ratio:.5g}×" if ratio != "" else "—"])) + " |")
            lines.append("")
    return "\n".join(lines)


def winner_tables(campaigns, observations):
    lines = []
    for operation in ("encode", "decode"):
        lines.extend([f"## Lowest observed entry-point medians · {operation}", "",
                      "| Decoded bytes | " + " | ".join(TITLES[campaign["slug"]] for campaign in campaigns) + " |",
                      "| ---: | " + " | ".join("---:" for _ in campaigns) + " |"])
        for size in campaigns[0]["sizes"]:
            cells = [f"{size:,}"]
            for campaign in campaigns:
                group = [row for row in observations if row["environment"] == campaign["slug"] and
                         row["operation"] == operation and row["decoded_bytes"] == size and row["group"] == "entry point"]
                best = lowest(group)
                winners = [row for row in group if row["status"] == "measured" and row["median_ns"] == best]
                cells.append(" / ".join(LABELS[row["backend"]] for row in winners) + " · **" + time_label(best) + "**")
            lines.append("| " + " | ".join(map(markdown_cell, cells)) + " |")
        lines.append("")
    return "\n".join(lines)


def series_values(observations, name, operation, field):
    values = {row["decoded_bytes"]: row[field] for row in observations
              if row["backend"] == name and row["operation"] == operation}
    return [values.get(size, math.nan) for size in GRID]


def time_label(value, _position=None):
    for scale, unit in ((1e9, "s"), (1e6, "ms"), (1e3, "µs"), (1, "ns")):
        if value >= scale:
            return f"{value / scale:g} {unit}"
    return f"{value:g} ns"


def write_csv(path, rows):
    with path.open("x", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def reserve_output(path):
    require(not path.exists(), f"refusing existing output: {path}")
    path.mkdir(parents=True)


def render(campaigns, output):
    os.environ.setdefault("MPLCONFIGDIR", str(Path(tempfile.gettempdir()) / "base58-benchmark-graphs-matplotlib"))
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from matplotlib.ticker import FuncFormatter, NullLocator

    plt.rcParams.update({"font.family": "DejaVu Sans", "font.size": 10, "axes.titlesize": 13,
                         "axes.labelsize": 11, "svg.fonttype": "none", "savefig.facecolor": "white"})
    repeats, samples = campaigns[0]["repeats"], campaigns[0]["samples"]
    require(all((campaign["repeats"], campaign["samples"]) == (repeats, samples) for campaign in campaigns),
            "sampling policy differs within the cohort")
    all_observations = [row for campaign in campaigns for row in complete_observations(campaign)]
    all_processes = [{"timing_protocol": PROTOCOL, "environment": campaign["slug"], "timer": campaign["timer"], **row}
                     for campaign in campaigns for row in campaign["processes"]]
    reports = {f"{campaign['slug']}-{operation}.md": detailed_tables(campaign,
               [row for row in all_observations if row["environment"] == campaign["slug"]], operation)
               for campaign in campaigns for operation in ("encode", "decode")}
    input_hashes = {path: digest for campaign in campaigns for path, digest in campaign["input_sha256"].items()}
    provenance = {"schema": 2, "timing_protocol": PROTOCOL, "created_utc": datetime.now(timezone.utc).isoformat(),
                  "renderer": str(Path(__file__).resolve()), "renderer_sha256": sha(Path(__file__)),
                  "python": sys.version, "python_executable": sys.executable, "platform": platform.platform(),
                  "matplotlib": matplotlib.__version__, "argv": sys.argv, "input_sha256": input_hashes,
                  "grid_decoded_bytes": GRID, "shared_source_files_sha256": campaigns[0]["source"],
                  "actual_fixture_fingerprints": campaigns[0]["fixtures"],
                  "fixture_consistency": "actual encoded lengths and data/text FNV-1a64 fingerprints match across all workers; FNV is a noncryptographic consistency check, not a cryptographic check or direct cross-worker byte comparison",
                  "oracle_evidence": "native fixtures use independent Malachite Natural radix conversion at all sizes; Wasm prepares text with Auto and has standalone JS BigInt checks only through 1024 bytes; all 29 actual fixture lengths/data/text fingerprints match across the cohort",
                  "aggregation": f"median of {repeats} per-process medians, each from {samples} warmed batch times per call",
                  "uncertainty": f"observed min/max of {repeats} process medians; not confidence intervals",
                  "bold_criterion": "exact lowest observed median within an environment/operation/size and entry-point or control group; exact ties all bold; no statistical-significance claim",
                  "route_inference": "static top-level branches for the exact source hashes and non-leading-zero fixtures, not an execution trace; inner multiplication policies remain conditional",
                  "route_sources_sha256": ROUTE_SOURCES,
                  "timers": "native process CPU nanoseconds; Wasm elapsed nanoseconds around the Rust repeat export, including call boundary",
                  "wrapper_scope": "native and Wasm decode inputs are validated before the timed loop; caller allocation is outside timing; for decoded size n, native encoder output slices have length max_encoded_len(n), Wasm encoder output slices have length 2n+32, and both decoder output slices have length n",
                  "cross_environment_comparison": "native/Wasm ratios do not isolate backend or code-generation cost because clocks, buffer lengths, link layout and harnesses differ",
                  "native_harness_scope": "native links every entry point via __internals; each Wasm module selects one entry point with both directions; application link layout and specialization may differ",
                  "limitations": "one deterministic valid input per size; warm state; host/compiler/runtime specific; no universal fastest claim",
                  "campaigns": [{key: campaign[key] for key in ("slug", "path", "timer", "runtime", "flags", "metadata")}
                                for campaign in campaigns]}
    require(all(sha(Path(path)) == digest for path, digest in input_hashes.items()), "inputs changed during validation")
    reserve_output(output)
    figure_names = []
    for campaign in campaigns:
        observations = campaign["observations"]
        for detail in (False, True):
            fig, axes = plt.subplots(1, 2, figsize=(16, 8), sharey=True)
            for axis, operation in zip(axes, ("encode", "decode")):
                for name in BACKENDS:
                    values = series_values(observations, name, operation, "median_ns")
                    lower = series_values(observations, name, operation, "process_min_ns")
                    upper = series_values(observations, name, operation, "process_max_ns")
                    # Missing support stays NaN; lines cannot cross an unsupported grid point.
                    style = "none" if name == "five8" else "--" if "portable" in name else "-"
                    marker = "*" if name == "five8" else "o" if name in ("auto", "bs58") else "."
                    axis.errorbar(GRID, values, yerr=[[v - lo for v, lo in zip(values, lower)],
                                                      [hi - v for v, hi in zip(values, upper)]],
                                  label=LABELS[name], color=COLORS[name], linestyle=style, marker=marker,
                                  markersize=11 if name == "five8" else 4 if name in ("auto", "bs58") else 3,
                                  linewidth=2.3 if name in ("auto", "bs58") else 1.2,
                                  elinewidth=0.8, capsize=2, alpha=1 if name in ("auto", "bs58", "five8") else 0.85,
                                  zorder=5 if name in ("auto", "bs58", "five8") else 2)
                axis.set_title(operation.capitalize(), loc="left", fontweight="bold")
                axis.set_xscale("symlog", base=2, linthresh=1, linscale=0.5)
                axis.set_yscale("log")
                if detail:
                    visible = [row for row in observations if row["decoded_bytes"] <= 128 and row["backend"] in BACKENDS]
                    axis.set_ylim(min(row["process_min_ns"] for row in visible) / 1.4,
                                  max(row["process_max_ns"] for row in visible) * 1.4)
                ticks = [0, 1, 8, 16, 32, 64, 128] if detail else [0, 1, 8, 32, 128, 512, 2048, 8192, 65536, 1048576, 4194304]
                tick_labels = [str(value) if value < 1024 else f"{value // 1024} KiB" if value < 1048576
                               else f"{value // 1048576} MiB" for value in ticks]
                axis.set_xticks(ticks, tick_labels)
                axis.set_xlim(-0.08, 140 if detail else max(campaign["sizes"]) * 1.15)
                axis.xaxis.set_minor_locator(NullLocator())
                axis.yaxis.set_major_formatter(FuncFormatter(time_label))
                axis.grid(which="major", color="#DADDE1", linewidth=0.6)
                axis.set_axisbelow(True)
                axis.set_xlabel("Decoded payload size (bytes; 0–1 linear, then log₂)")
                axis.spines[["top", "right"]].set_visible(False)
            timer_label = "Process CPU" if campaign["timer"] == "process CPU" else "Wasm elapsed"
            axes[0].set_ylabel(f"{timer_label} time per conversion · lower is faster")
            handles, labels = axes[0].get_legend_handles_labels()
            fig.legend(handles, labels, loc="lower center", bbox_to_anchor=(0.5, 0.025), ncol=4,
                       frameon=False, columnspacing=2.0, handlelength=2.5)
            suffix = " · small inputs" if detail else ""
            fig.suptitle(TITLES[campaign["slug"]] + suffix, x=0.07, ha="left", y=0.97, fontsize=18, fontweight="bold")
            fig.text(0.07, 0.915, campaign["runtime"], fontsize=10, color="#45505B")
            fig.text(0.07, 0.88, f"Entry points · {repeats} fresh processes × {samples} warm batches · median / observed process range (not a CI)", fontsize=10)
            fig.text(0.07, 0.225, "bs58 and batched stop at 64 KiB; five8 has separate markers only at 32 and 64 bytes. Cold calls excluded.", fontsize=9, color="#45505B")
            if campaign["slug"] == "m5-wasm":
                fig.text(0.07, 0.19, "Decode input is validated before the timed loop. Elapsed clock and Wasm harness differ from native.", fontsize=9, color="#45505B")
            fig.subplots_adjust(left=0.07, right=0.985, top=0.82, bottom=0.32, wspace=0.12)
            stem = campaign["slug"] + ("-small" if detail else "")
            for extension in ("png", "svg"):
                name = f"{stem}.{extension}"
                with (output / name).open("xb") as stream:
                    fig.savefig(stream, format=extension, dpi=200, metadata={"Creator": "scripts/benchmark_graphs.py"})
                figure_names.append(name)
            plt.close(fig)
    write_csv(output / "observations.csv", all_observations)
    write_csv(output / "processes.csv", all_processes)
    write_csv(output / "skipped.csv", [row for row in all_observations if row["status"] == "skipped"])
    for name, content in reports.items():
        with (output / name).open("x") as stream:
            stream.write(content)
    with (output / "provenance.json").open("x") as stream:
        json.dump(provenance, stream, indent=2)
        stream.write("\n")
    lines = ["# Base58 benchmark results", "", f"Cohort: `{PROTOCOL}`. " + provenance["aggregation"] + ".",
             "Bars show the observed process range, not confidence intervals. Native figures use process CPU time; Wasm uses elapsed time around the Rust repeat export, including call boundary.", "",
             "Decode input validation and caller allocation are outside timing in this cohort. For decoded size `n`, native encoder output slices have length `max_encoded_len(n)`; Wasm encoder output slices have length `2n+32`. Both decoder output slices have length `n`. Native/Wasm clocks, buffer lengths, link layout and harnesses differ, so their timing ratios do not isolate backend or code-generation cost. Winners are chosen independently in each environment.", "",
             "**Bold means the exact lowest observed median** among the entry points for the identical environment, operation and fixture size. All exact ties are bold. This is an observational comparison, not statistical significance. Displayed times are rounded; full-precision values and all process medians are preserved in CSV.", "",
             "An entry point is not necessarily a separate kernel. Auto and named wrappers can reach the same batched, fixed-size matrix or recursive path. Detailed tables label those routes using the exact frozen source, actual encoded lengths and CPU settings; these are top-level static inferences, not execution traces. Conditional NTT/Karatsuba does not assert that a transform executed.", "",
             "Native fixtures are checked with independent Malachite Natural radix conversion at every size. Wasm prepares text with Auto and checks an independent JS BigInt reference only through 1024 bytes. Acceptance requires matching actual encoded lengths and FNV-1a64 data/text fingerprints at all 29 sizes. FNV is a noncryptographic consistency check, not a cryptographic check or direct cross-worker byte comparison. The standalone large Wasm fixture generator is not described as independent.", "",
             winner_tables(campaigns, all_observations), "", "## Full results and controls", ""]
    for campaign in campaigns:
        slug = campaign["slug"]
        lines.append(f"- {TITLES[slug]}: [encode]({slug}-encode.md), [decode]({slug}-decode.md). Graphs: [PNG]({slug}.png), [SVG]({slug}.svg); small inputs: [PNG]({slug}-small.png), [SVG]({slug}-small.svg).")
    lines.extend(["", "The detailed pages separate entry points from forced general-matrix and matched-leaf NTT controls. The forced matrix is shown only in its valid domain. Owned/cached NTT controls share leaf and threshold policy; owned refers only to twiddles and can still retain matrix-leaf tables. Control minima are scoped separately.", "",
                  "Malachite is a development adapter. Portable source may be vectorized by the compiler and can reach shared SIMD leaves or nonportable wrapper fallbacks. five8 is a fixed-width comparison at exactly 32 and 64 bytes. bs58 and batched stop at 64 KiB. Skip reasons remain explicit; no timings are extrapolated.", "",
                  "Comparator versions: `bs58` 0.5.1 and `five8` 1.0.0, as recorded in the frozen source locks.", "",
                  "Native timings come from one worker that links every entry point through `__internals`; each Wasm module selects one entry point with both directions. Application link layout and specialization may differ.", "",
                  "Empty native decode uses the worker's exact-length `Vec` output policy. Observed GCP timings depend strongly on the zero-length buffer address and do not characterize every empty caller.", "",
                  "[Full observations and bs58 ratios](observations.csv), [per-process summaries](processes.csv), and [support skips](skipped.csv) preserve the full result grid and floating-point precision in nanoseconds. `bs58 / entry` is the bs58 median divided by the entry median; above one favors the entry. Ratios are blank where bs58 was not measured. Cold/calibration calls do not enter warmed results.", "",
                  "The repository's `docs/performance.md` describes measurement scope and resource accounting. Results with different source generations, grids, clocks or validation scopes must not be pooled.", "",
                  "[Provenance](provenance.json) records source identities, all consumed input hashes, build/runtime settings, Python and Matplotlib versions. Results cover one deterministic valid input per size on these hosts. They do not establish a universally fastest implementation.", ""])
    with (output / "README.md").open("x") as stream:
        stream.write("\n".join(lines))
    require(all(sha(Path(path)) == digest for path, digest in input_hashes.items()), "inputs changed during rendering")
    outputs = [*figure_names, *reports, "observations.csv", "processes.csv", "skipped.csv", "provenance.json", "README.md"]
    with (output / "complete.json").open("x") as stream:
        json.dump({"schema": 2, "timing_protocol": PROTOCOL, "sha256": {name: sha(output / name) for name in outputs}}, stream, indent=2)
        stream.write("\n")
    print(f"Saved {len(reports)} result tables, entry-point figures, full CSVs and provenance: {output}")


def self_test():
    require(backend("dc") == "karatsuba" and backend("ntt") == "ntt", "backend normalization")
    processes = [{"backend": "auto", "operation": "encode", "decoded_bytes": 32, "median_ns": value}
                 for value in (1, 3, 6)]
    row = aggregate(processes)[0]
    require((row["median_ns"], row["process_min_ns"], row["process_max_ns"]) == (3, 1, 6), "process aggregation")
    observations = [row, dict(row, backend="bs58", median_ns=6), dict(row, decoded_bytes=1048576)]
    add_ratios(observations)
    require(observations[0]["speedup_vs_bs58"] == 2 and observations[-1]["speedup_vs_bs58"] == "", "baseline ratios")
    values = series_values(observations, "bs58", "encode", "median_ns")
    require(values[GRID.index(32)] == 6 and math.isnan(values[-1]), "unsupported-size gap")
    require(time_label(1000) == "1 µs" and time_label(1000000) == "1 ms", "time units")
    close(0, 0, "zero single-call summary normalization")
    require(fixture_bytes(0) == b"" and len(fixture_bytes(32)) == 32 and fixture_bytes(32)[0] & 1, "fixture generation")
    require(fnv1a64(fixture_bytes(0)) == "cbf29ce484222325" and
            fnv1a64(fixture_bytes(32)) == "b95817c7fb38a311", "shared native/Wasm fixture anchors")
    fixtures = {size: (0 if size == 0 else ((size * (3148993499475808360 + 1)) >> 61) + 1,) for size in GRID}
    cases = expected_keys(GRID, ALL_BACKENDS, fixtures)
    rows = [{"backend": name, "operation": operation, "decoded_bytes": size, "repeat": 0,
             "phase": phase, "sample": 0, "iterations": 1, "elapsed_ns": 10 if phase == "warm" else 0}
            for name, operation, size in cases for phase in ("cold", "second", "calibration", "warm")]
    collected, groups = collect_processes(rows, "decoded_bytes", "elapsed_ns", 1, 1, cases)
    require(len(collected) == len(cases) and all(row["median_ns"] == 10 for row in collected),
            "zero singles/calibration must be accepted and excluded from warm summaries")
    bad = [dict(row, elapsed_ns=0) if row["phase"] == "warm" else row for row in rows]
    try:
        collect_processes(bad, "decoded_bytes", "elapsed_ns", 1, 1, cases)
    except ValueError:
        pass
    else:
        raise AssertionError("zero warmed batch accepted")
    missing = [row for row in rows if key(row, "decoded_bytes") != key(rows[0], "decoded_bytes")]
    for invalid_rows in (missing, [*rows, next(row for row in rows if row["phase"] == "warm")]):
        try:
            collect_processes(invalid_rows, "decoded_bytes", "elapsed_ns", 1, 1, cases)
        except ValueError:
            pass
        else:
            raise AssertionError("missing process or duplicate warmed sample accepted")
    jobs = {"job_order_seed": 20261024, "jobs": [dict(zip(("backend", "operation", "decoded_bytes", "repeat"), case))
                                                  for case in groups]}
    validate_jobs(jobs, groups, cases, 1)
    try:
        validate_jobs({**jobs, "jobs": list(reversed(jobs["jobs"]))}, groups, cases, 1)
    except ValueError:
        pass
    else:
        raise AssertionError("different saved/raw job order accepted")
    require(not supported("matrix-forced", "encode", 0, 0) and
            supported("matrix-forced", "decode", 4096, 5632) and
            not supported("matrix-forced", "decode", 4096, 5633), "forced matrix uses exact encoded-char domain")
    require(len(GRID) == 29 and GRID[-1] == 4194304 and len(cases) == 771, "full 4 MiB grid and support partition")
    for size in (1, 32, 64, 1024):
        require(fixture_first_byte(size) == fixture_bytes(size)[0], "significant-bit route input")
    arm = {"cpu": {"arch": "aarch64", "ifma": False, "avx2_compiled": False, "neon": True,
                   "runtime_avx2": False, "runtime_sse4_1": False}}
    generic = {"cpu": {"arch": "x86_64", "ifma": True, "avx2_compiled": False, "neon": False,
                       "runtime_avx2": True, "runtime_sse4_1": True}}
    native = {"cpu": {**generic["cpu"], "avx2_compiled": True}}
    require(route(generic, "auto", "decode", 1024, 1399)["multiplier_policy"] == "matrix" and
            route(native, "auto", "decode", 1024, 1399)["route_family"] == "batched", "compiled AVX2 differs from runtime AVX2")
    require(route(arm, "ntt-same-owned", "encode", 8192, 11188)["leaf_policy"] == "matrix" and
            route(arm, "ntt-same-owned", "encode", 8192, 11188)["twiddle_policy"] == "owned", "owned twiddles may retain matrix leaves")
    require(route(arm, "matrix-portable", "encode", 65536, 89499)["twiddle_policy"] == "cached" and
            route(arm, "matrix-portable", "decode", 8192, 11188)["multiplier_policy"] == "Karatsuba policy",
            "matrix wrappers have different recursive fallbacks")
    tied = [{"status": "measured", "median_ns": value, "process_min_ns": value, "process_max_ns": value}
            for value in (10.0, 10.0, 10.000001)]
    require(lowest(tied) == 10 and sum("**" in table_time(item, lowest(tied)) for item in tied) == 2,
            "bold all exact ties, not rounded near ties")
    require(all(len(digest) == 64 for digest in ROUTE_SOURCES.values()), "route source SHA256 shape")
    for invalid in ("nan", "inf", "-1", "0"):
        try:
            number(invalid)
        except ValueError:
            pass
        else:
            raise AssertionError(f"invalid timing accepted: {invalid}")
    with tempfile.TemporaryDirectory(prefix="base58-graph-self-test-") as directory:
        path = Path(directory)
        marker = path / "preserve.txt"
        marker.write_text("manual content\n")
        try:
            reserve_output(path)
        except ValueError:
            pass
        else:
            raise AssertionError("existing output accepted")
        require(marker.read_text() == "manual content\n", "existing output changed")
    print("benchmark_graphs self-test passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    for name in TITLES:
        parser.add_argument(f"--{name}", type=Path)
    parser.add_argument("--wasm-modules", type=Path, help="relocated module directory (default: recorded paths)")
    parser.add_argument("--output", type=Path, help="new output directory; existing paths are refused")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    require(all(getattr(args, slug.replace("-", "_")) is not None for slug in TITLES) and args.output,
            "provide all four campaign directories and --output")
    require(not args.output.exists(), f"refusing existing output: {args.output}")
    campaigns = [native_campaign(getattr(args, slug.replace("-", "_")).resolve(), slug)
                 for slug in ("gcp-default", "gcp-native", "m5-native")]
    campaigns.append(wasm_campaign(args.m5_wasm.resolve(), args.wasm_modules.resolve() if args.wasm_modules else None))
    require(all(campaign["source"] == campaigns[0]["source"] for campaign in campaigns[1:]),
            "campaigns do not use identical shared source files")
    require(all(campaign["fixtures"] == campaigns[0]["fixtures"] for campaign in campaigns[1:]),
            "actual campaign input/output fixture fingerprints differ")
    render(campaigns, args.output.resolve())


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"benchmark_graphs: {error}") from error

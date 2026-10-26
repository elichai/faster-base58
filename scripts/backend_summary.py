#!/usr/bin/env python3
"""Join the frozen backend measurements; never build, benchmark, or overwrite outputs."""

import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import statistics
import tarfile
import tempfile


ROOT = Path(__file__).resolve().parent.parent
SIZES = [0, 1, 32, 64, 512, 1024, 2048, 4096, 8192, 65536, 1048576]
ENTRIES = ["auto", "batched", "dc", "toom3", "ntt", "ntt-cached", "ntt-portable",
           "malachite", "matrix", "matrix-portable"]
CONTROLS = ["matrix-forced", "ntt-same-owned", "ntt-same-cached"]
NATIVE = [("m5-native", "local", "local-code-size"),
          ("zen5-generic", "gcp-default", "gcp-default-code-size"),
          ("zen5-native", "gcp-native", "gcp-native-code-size")]
WASM = [("m5-v8", "wasm-performance"), ("zen5-v8", "gcp-wasm-performance")]
SIZE_DIRS = [name for _, _, name in NATIVE] + ["wasm-code-size",
             "before-dependency-code-size-local", "before-dependency-code-size-wasm"]
KEY = ("operation", "backend", "bytes")
ALLOC_FIELDS = ["alloc_calls", "alloc_bytes", "zeroed_calls", "zeroed_bytes",
                "realloc_calls", "realloc_old_bytes", "realloc_new_bytes", "dealloc_calls",
                "dealloc_bytes", "failed_calls", "live_before_bytes", "peak_live_bytes",
                "live_after_bytes", "peak_extra_bytes", "retained_delta_bytes",
                "encode_cache_bytes", "decode_cache_bytes", "ntt_cache_bytes"]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def number(value):
    result = float(value)
    require(math.isfinite(result) and result >= 0, f"invalid measurement: {value}")
    return result


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_path(path):
    path = path.resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def rows(path):
    with path.open(newline="") as stream:
        return list(csv.DictReader(stream))


def index(values, fields):
    result = {}
    for row in values:
        key = tuple(str(row[field]) for field in fields)
        require(key not in result, f"duplicate key {key}")
        result[key] = row
    return result


def safe_destination(path):
    require(not path.exists(), f"refusing existing output: {path}")
    require(path.parent.is_dir(), f"output parent does not exist: {path.parent}")


def write_csv(path, values):
    require(values, f"no rows for {path}")
    fields = list(dict.fromkeys(field for row in values for field in row))
    with path.open("x", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        writer.writerows(values)


def role(backend):
    return ("control" if backend in CONTROLS else "empty-scaffold" if backend == "empty"
            else "development-adapter" if backend == "malachite" else "public-entry")


def core_hashes(metadata):
    if "build" in metadata:
        return {item["path"]: item["sha256"] for item in metadata["build"]["source"]["files"]
                if item["path"].startswith("src/")}
    return {path: value for path, value in metadata["source_files_sha256"].items()
            if path.startswith("src/")}


def route(config, backend, operation, size, chars, ifma):
    """Top-level static inference for the archived nonzero fixtures, not kernel tracing."""
    if backend == "empty":
        return "no conversion (ABI scaffold)"
    if size == 0:
        return "empty fast path"
    native = config.endswith("native") or config == "zen5-generic"
    arm = config == "m5-native"
    if backend == "matrix-forced":
        return "forced general matrix"
    if backend == "auto":
        if operation == "encode":
            return route(config, "batched" if size < 22 else "matrix", operation, size, chars, ifma)
        minimum = 576 if arm else 1024
        if chars < minimum:
            return "batched"
        if chars > 5632:
            return route(config, "ntt-cached", operation, size, chars, ifma)
        if ifma and (chars > 5120 or config == "zen5-native"):
            return route(config, "ntt-cached" if chars > 5120 else "batched",
                         operation, size, chars, ifma)
        return "general matrix" if native else "batched (Wasm auto dispatch)"
    if backend.startswith("matrix"):
        if operation == "encode":
            if size in (32, 64):
                return "fixed-size matrix"
            if size < 22:
                return "batched"
            if size > (3584 if ifma else 8192):
                return route(config, "ntt-cached", operation, size, chars, ifma)
            return "general matrix" + (" portable" if backend.endswith("portable") else "")
        if chars < 576:
            return "batched"
        if chars > 5632:
            # The archived matrix wrapper falls back to explicit Karatsuba.
            return route(config, "dc", operation, size, chars, ifma)
        return "general matrix" + (" portable" if backend.endswith("portable") else "")
    if backend == "batched":
        return "batched"
    if operation == "decode" and chars <= 5120:
        return "batched (recursive wrapper fallback)"
    if operation == "encode":
        threshold = 512 if ifma else (4096 if backend.startswith("ntt") else 5120)
        if backend == "malachite":
            threshold = 1280 if ifma else 3072
        if size <= threshold:
            return "batched (recursive wrapper fallback)"
    multiplier = ("NTT/short-product fallback" if backend.startswith("ntt") else
                  "Toom-3/short-product fallback" if backend == "toom3" else backend)
    return "recursive conversion; " + multiplier


def metric_rows(config, directory, kind, inputs):
    path = directory / "summary.csv"
    inputs.add(path)
    result = []
    for row in rows(path):
        require(row["phase"] in ("cold", "second", "warm", "io-data", "io-encoded", "io-output"),
                f"unknown phase in {path}")
        for field in ("median", "minimum", "maximum"):
            number(row[field])
        require(float(row["minimum"]) <= float(row["median"]) <= float(row["maximum"]),
                f"invalid summary range: {row}")
        result.append({"configuration": config, "kind": kind, **row,
                       "source_csv": source_path(path)})
    index(result, [*KEY, "phase", "metric"])
    return result


def load_native(snapshot, config, prefix, size_dir, inputs, sizes, metrics, campaigns):
    time_dir, alloc_dir = snapshot / f"{prefix}-timings", snapshot / f"{prefix}-allocations"
    tm, am = [json.loads((directory / "metadata.json").read_text()) for directory in (time_dir, alloc_dir)]
    for directory in (time_dir, alloc_dir):
        inputs.update(directory / name for name in ("metadata.json", "skipped.csv", "io.csv"))
    require(tm["status"] == am["status"] == "complete", "incomplete native campaign")
    require(tm["build"]["source"] == am["build"]["source"], "native source mismatch")
    require(not tm["build"]["worker"]["count_allocations"] and am["build"]["worker"]["count_allocations"],
            "counted timing build")
    for field in ("platform", "machine", "hostname", "rustc", "flags", "cargo_configs"):
        require(tm["build"]["environment"][field] == am["build"]["environment"][field],
                f"native environment mismatch: {field}")
    ifma = tm["build"]["worker"]["ifma_threshold"] != 0
    require(tm["build"]["worker"]["ifma_threshold"] == am["build"]["worker"]["ifma_threshold"],
            "native IFMA mismatch")
    sm = json.loads((snapshot / size_dir / "metadata.json").read_text())
    require(core_hashes(tm) == core_hashes(sm), "timing/size Rust source mismatch")
    time_metrics = metric_rows(config, time_dir, "timing", inputs)
    alloc_metrics = metric_rows(config, alloc_dir, "allocation", inputs)
    metrics.extend(time_metrics + alloc_metrics)
    ti = index(time_metrics, [*KEY, "phase", "metric"])
    ai = index(alloc_metrics, [*KEY, "phase", "metric"])
    skipped = index(rows(time_dir / "skipped.csv"), KEY)
    require(skipped == index(rows(alloc_dir / "skipped.csv"), KEY), "allocation/timing skip mismatch")
    expected = {(op, backend, str(size)) for op in ("encode", "decode")
                for backend in ENTRIES + CONTROLS for size in SIZES}
    actual = {key[:3] for key in ti}
    require(actual.isdisjoint(skipped) and actual | skipped.keys() == expected, "unexpected native case set")
    for row in time_metrics:
        require(int(row["count"]) == (21 if row["phase"] == "warm" else 3), "unexpected timing repetition count")
    require(all(int(row["count"]) == 3 for row in alloc_metrics), "unexpected allocation repetition count")
    ios = {}
    for row in rows(time_dir / "io.csv"):
        key = tuple(row[field] for field in KEY)
        values = {field: int(row[field]) for field in ("input_capacity_bytes", "output_capacity_bytes",
                  "fixture_capacity_bytes", "encoded_chars")}
        require(key not in ios or ios[key] == values, "caller capacities vary across repetitions")
        ios[key] = values
    require(rows(time_dir / "io.csv") == rows(alloc_dir / "io.csv"), "caller buffers differ between runs")
    output = []
    for operation in ("encode", "decode"):
        for size in SIZES:
            for backend in ENTRIES + CONTROLS:
                key = operation, backend, str(size)
                out = {"configuration": config, "runtime": "native", "operation": operation,
                       "backend": backend, "role": role(backend), "decoded_bytes": size,
                       "status": "skipped" if key in skipped else "measured", "reason": "",
                       "warm_time_metric": "process_cpu_ns", "cold_time_metric": "elapsed_ns",
                       "memory_metric": "requested_Rust_layout_bytes"}
                if key in skipped:
                    out["reason"] = skipped[key]["reason"]
                else:
                    out.update(ios[key])
                    out["route_inferred"] = route(config, backend, operation, size, out["encoded_chars"], ifma)
                    for phase in ("cold", "second", "warm"):
                        for metric, short in (("cpu_ns_per_call", "cpu"), ("wall_ns_per_call", "wall")):
                            value = ti[*key, phase, metric]
                            for stat, name in (("median", "median"), ("minimum", "min"), ("maximum", "max"),
                                               ("count", "samples")):
                                out[f"{phase}_{short}_{name}_ns" if name != "samples" else f"{phase}_{short}_samples"] = number(value[stat])
                    for phase in ("cold", "warm"):
                        for field in ALLOC_FIELDS:
                            out[f"{phase}_{field}"] = number(ai[*key, phase, field]["median"])
                    out["retained_cache_bytes"] = sum(out[f"warm_{field}_cache_bytes"] for field in ("encode", "decode", "ntt"))
                    out["warm_active_cache_plus_extra_bytes"] = out["retained_cache_bytes"] + out["warm_peak_extra_bytes"]
                    out["warm_ns"] = out["warm_cpu_median_ns"]
                    out["cold_ns"] = out["cold_wall_median_ns"]
                if backend in ENTRIES:
                    out.update(size_join(sizes, size_dir, backend, operation))
                output.append(out)
    campaigns.append({"configuration": config, "kind": "native timing + separate allocations",
                      "source_sha256": tm["build"]["source"]["sha256"],
                      "metadata": source_path(time_dir / "metadata.json"),
                      "allocation_metadata": source_path(alloc_dir / "metadata.json"),
                      "ifma_threshold": tm["build"]["worker"]["ifma_threshold"]})
    return output


def size_join(sizes, size_dir, backend, operation):
    artifact_backend = "karatsuba" if backend == "dc" else backend
    kind = "cdylib" if size_dir == "wasm-code-size" else "bin"
    operation_row = sizes[size_dir, artifact_backend, operation, kind]
    both = sizes[size_dir, artifact_backend, "both", kind]
    return {"linked_size_configuration": size_dir, "linked_direction_kind": f"{operation}-{kind}",
            "linked_stripped_bytes": int(operation_row["stripped_file_bytes"]),
            "linked_both_stripped_bytes": int(both["stripped_file_bytes"]),
            "linked_stripped_gzip_bytes": int(operation_row["stripped_gzip_bytes"]),
            "linked_code_bytes": operation_row["code_bytes"] or operation_row["wasm_code_section_bytes"],
            "linked_readonly_bytes": operation_row["readonly_bytes"],
            "linked_data_bytes": operation_row["data_bytes"] or operation_row["wasm_data_section_bytes"],
            "linked_bss_bytes": operation_row["bss_bytes"],
            "linked_sha256": operation_row["stripped_sha256"]}


def load_wasm(snapshot, config, directory_name, inputs, sizes, metrics, campaigns):
    directory = snapshot / directory_name
    complete = json.loads((directory / "complete.json").read_text())
    inputs.add(directory / "complete.json")
    for filename, expected in complete["sha256"].items():
        path = directory / filename
        require(sha(path) == expected, f"Wasm completion hash mismatch: {path}")
        inputs.add(path)
    metadata = json.loads((directory / "metadata.json").read_text())
    data = rows(directory / "summary.csv")
    require(len(data) == complete["completed_cases"], "Wasm case count mismatch")
    summary = index(data, ("operation", "backend", "decoded_bytes"))
    skipped = index(rows(directory / "skipped.csv"), ("operation", "backend", "decoded_bytes"))
    expected = {(op, "karatsuba" if backend == "dc" else backend, str(size))
                for op in ("encode", "decode") for backend in [*ENTRIES, "empty"] for size in SIZES if size != 2048}
    require(summary.keys().isdisjoint(skipped) and summary.keys() | skipped.keys() == expected,
            "unexpected Wasm case set")
    output = []
    for operation in ("encode", "decode"):
        for size in SIZES:
            for backend in [*ENTRIES, "empty"]:
                key = operation, "karatsuba" if backend == "dc" else backend, str(size)
                status = "measured" if key in summary else "skipped" if key in skipped else "not_measured"
                require(status != "not_measured" or size == 2048, "unexpected missing Wasm cell")
                out = {"configuration": config, "runtime": "V8 Wasm", "operation": operation,
                       "backend": backend, "role": role(backend), "decoded_bytes": size,
                       "status": status, "reason": "", "warm_time_metric": "elapsed_ns", "cold_time_metric": "elapsed_ns",
                       "memory_metric": "linear_memory_pages_65536_bytes"}
                out.update(size_join(sizes, "wasm-code-size", backend, operation))
                if status == "skipped":
                    out["reason"] = skipped[key]["reason"]
                elif status == "not_measured":
                    out["reason"] = "2048-byte size absent from the frozen Wasm campaign"
                else:
                    source = summary[key]
                    linked = sizes["wasm-code-size", key[1], "both", "cdylib"]
                    require(source["module_sha256"] == linked["stripped_sha256"], "timed module differs from size record")
                    require(int(source["module_bytes"]) == int(linked["stripped_file_bytes"]), "timed module length mismatch")
                    out.update({"input_bytes": int(source["input_bytes"]), "encoded_chars": int(source["encoded_bytes"]),
                                "timed_module_bytes": int(source["module_bytes"]),
                                "timed_module_sha256": source["module_sha256"],
                                "route_inferred": route(config, backend, operation, size, int(source["encoded_bytes"]), False)})
                    for field in ("cold_ns", "second_ns", "warm_median_ns", "warm_min_ns", "warm_max_ns",
                                  "warm_mad_ns", "cold_pages", "second_pages", "warm_max_pages"):
                        out[field] = number(source[field])
                    out["warm_ns"] = out["warm_median_ns"]
                    for field, value in out.copy().items():
                        if field in ("cold_ns", "second_ns", "warm_median_ns", "warm_min_ns", "warm_max_ns",
                                     "warm_mad_ns", "cold_pages", "second_pages", "warm_max_pages"):
                            metrics.append({"configuration": config, "kind": "wasm_summary", "operation": operation,
                                            "backend": backend, "bytes": size, "phase": field.split("_")[0],
                                            "metric": field, "count": 7 if field.startswith("warm") else 1,
                                            "median": value if field.endswith("_pages") or field in ("cold_ns", "second_ns", "warm_median_ns") else "",
                                            "minimum": "", "maximum": "", "value": value,
                                            "source_csv": source_path(directory / "summary.csv")})
                output.append(out)
    campaigns.append({"configuration": config, "kind": "V8 Wasm elapsed + page high water",
                      "metadata": source_path(directory / "metadata.json"),
                      "node": metadata["node"], "v8": metadata["versions"]["v8"],
                      "driver_sha256": metadata["driver_sha256"], "helper_sha256": metadata["helper_sha256"]})
    return output


def dominates(a, b):
    return all(x <= y for x, y in zip(a, b)) and any(x < y for x, y in zip(a, b))


def frontier(candidates, fields):
    return [row["backend"] for row in candidates if not any(
        dominates([float(other[field]) for field in fields], [float(row[field]) for field in fields])
        for other in candidates if other is not row)]


def frontiers(observations):
    result, points = [], []
    groups = {}
    for row in observations:
        if row["backend"] in ENTRIES and row["status"] == "measured":
            groups.setdefault((row["configuration"], row["operation"], row["decoded_bytes"]), []).append(row)
    for (config, operation, size), candidates in sorted(groups.items()):
        size_field = "linked_both_stripped_bytes"
        time_size = frontier(candidates, ["warm_ns", size_field])
        cold_size = frontier(candidates, ["cold_ns", size_field])
        memory_field = "warm_active_cache_plus_extra_bytes" if candidates[0]["runtime"] == "native" else "warm_max_pages"
        resources = frontier(candidates, ["warm_ns", size_field, memory_field])
        best = min(candidates, key=lambda row: row["warm_ns"])
        auto = next(row for row in candidates if row["backend"] == "auto")
        near = [row["backend"] for row in candidates if row["warm_ns"] <= best["warm_ns"] * 1.05]
        result.append({"configuration": config, "operation": operation, "decoded_bytes": size,
                       "fastest_entry": best["backend"], "fastest_ns": best["warm_ns"],
                       "auto_over_fastest": auto["warm_ns"] / best["warm_ns"],
                       "within_5_percent": ";".join(near), "warm_time_size_frontier": ";".join(time_size),
                       "cold_elapsed_size_frontier": ";".join(cold_size),
                       "warm_time_size_memory_frontier": ";".join(resources),
                       "memory_metric": memory_field, "size_metric": size_field})
        for row in candidates:
            points.append({"configuration": config, "operation": operation, "decoded_bytes": size,
                           "backend": row["backend"], "warm_over_fastest": row["warm_ns"] / best["warm_ns"],
                           "warm_time_size_pareto": row["backend"] in time_size,
                           "cold_elapsed_size_pareto": row["backend"] in cold_size,
                           "warm_time_size_memory_pareto": row["backend"] in resources})
    return result, points


def table(headers, values):
    return "\n".join(["| " + " | ".join(map(str, headers)) + " |",
                       "| " + " | ".join("---" for _ in headers) + " |"] +
                     ["| " + " | ".join(map(str, row)) + " |" for row in values])


def cell(row, field, divisor=1):
    if row["status"] == "skipped":
        return "SKIP"
    if row["status"] == "not_measured":
        return "NM"
    value = float(row[field]) / divisor
    return f"{value:.3f}" if divisor != 1 else f"{value:,.0f}"


def grids(observations, leaders):
    lookup = {(r["configuration"], r["operation"], r["decoded_bytes"], r["backend"]): r for r in observations}
    text = ["# Complete baseline grids", "",
            "Generated by `scripts/backend_summary.py`. Sizes are decoded bytes, including for decoding.",
            "`SKIP` is a recorded cap; `NM` is not measured. No interpolation. Times are descriptive medians, not confidence intervals.",
            "Native warm times are process CPU; cold times below are elapsed. V8 times are elapsed. All time tables use microseconds.",
            "Native memory is requested Rust layout bytes, excluding caller buffers/fixture/baseline/stack and allocator overhead.",
            "Warm active = retained tables + temporary extra peak; cold extra peak includes newly retained tables.",
            "Wasm memory is 64 KiB linear-memory page high water, including caller/stack/static/allocator/table space; it is not live heap.", ""]
    for config in [c[0] for c in NATIVE + WASM]:
        native = any(config == item[0] for item in NATIVE)
        for operation in ("encode", "decode"):
            text.extend([f"## {config}: {operation}", ""])
            fields = [("Warm time (us)", "warm_ns", 1000), ("First-call elapsed time (us)", "cold_ns", 1000)]
            if native:
                fields += [("Retained table heap (bytes)", "retained_cache_bytes", 1),
                           ("Warm active table plus temporary peak (bytes)", "warm_active_cache_plus_extra_bytes", 1),
                           ("Cold additional requested heap peak (bytes)", "cold_peak_extra_bytes", 1),
                           ("Warm allocation calls (excluding reallocations)", "warm_alloc_calls", 1)]
            else:
                fields += [("First-call linear-memory pages", "cold_pages", 1),
                           ("Warm maximum linear-memory pages", "warm_max_pages", 1)]
            for title, field, divisor in fields:
                text += [f"### {title}", "", table(["Bytes", *ENTRIES],
                         [[size, *(cell(lookup[config, operation, size, backend], field, divisor)
                                   for backend in ENTRIES)] for size in SIZES]), ""]
            if native:
                text += ["### Matched controls: warm CPU us / retained table bytes", "",
                         table(["Bytes", *CONTROLS], [[size, *(
                             cell(lookup[config, operation, size, backend], "warm_ns", 1000) + " / " +
                             cell(lookup[config, operation, size, backend], "retained_cache_bytes")
                             if lookup[config, operation, size, backend]["status"] == "measured" else "SKIP"
                             for backend in CONTROLS)] for size in SIZES]), ""]
    text += ["## Exact observed frontiers", "",
             "Candidates are the ten entries, including the development adapter. Controls and empty scaffolds are excluded.",
             "Every objective is minimized. Equal points survive. Exact dominance of observed medians is not a statistical claim.",
             "The size objective uses the encode+decode executable/module; the Wasm module matches the timed artifact.",
             "Native timing uses a separate all-entry worker, so linked consumer sizes are source-matched associations, not that worker's footprint.", ""]
    for config in [c[0] for c in NATIVE + WASM]:
        for operation in ("encode", "decode"):
            subset = [r for r in leaders if r["configuration"] == config and r["operation"] == operation]
            text += [f"### {config}: {operation}", "", table(
                ["Bytes", "Fastest", "Auto / fastest", "Warm time + size", "Warm time + size + memory", "Cold elapsed + size"],
                [[r["decoded_bytes"], r["fastest_entry"], f'{r["auto_over_fastest"]:.3f}',
                  r["warm_time_size_frontier"].replace(";", ", "),
                  r["warm_time_size_memory_frontier"].replace(";", ", "),
                  r["cold_elapsed_size_frontier"].replace(";", ", ")] for r in subset]), ""]
    return "\n".join(text)


def linked_grids(size_rows, snapshot):
    text = ["# Complete linked artifact grids", "", "All numbers are bytes. Raw, stripped and compressed files are distinct metrics.",
            "Native code/readonly/data/BSS follow the existing platform section parser. BSS is virtual storage.",
            "Wasm CODE/DATA are section payloads; memory is the initial declaration, including stack/static/page padding.",
            "Empty scaffolds have the same direction and artifact kind. File/section sums need not agree.", ""]
    for config in SIZE_DIRS:
        selected = [r for r in size_rows if r["size_configuration"] == config]
        text += [f"## {config}", "", f"Source: [sizes.csv](<{snapshot / config / 'sizes.csv'}>).", ""]
        for direction in ("encode", "decode", "both"):
            for kind in sorted({r["kind"] for r in selected}):
                rs = [r for r in selected if r["direction"] == direction and r["kind"] == kind]
                text += [f"### {direction}: {kind}", "", table(
                    ["Entry", "Raw file", "Stripped", "Gzip", "Code / CODE", "Readonly", "Data / DATA", "BSS", "Initial Wasm memory"],
                    [[r["backend"], r["file_bytes"], r["stripped_file_bytes"], r["stripped_gzip_bytes"],
                      r["code_bytes"] or r["wasm_code_section_bytes"], r["readonly_bytes"] or "—",
                      r["data_bytes"] or r["wasm_data_section_bytes"], r["bss_bytes"] or "—",
                      r["wasm_initial_memory_bytes"] or "—"] for r in rs]), ""]
    return "\n".join(text)


def historical(inputs, directory):
    data, inventory, text = [], [], ["# Historical campaigns (kept separate)", "",
        "These Criterion elapsed-time confidence intervals and fresh-process cold runs are older source/build campaigns.",
        "They are not pooled with the matched resource-worker or V8 results and have no joined allocation/size claims.",
        "Earlier `gcp-candidate-native` auto rows precede the final dispatch correction. Empty cells mean unmeasured.", ""]
    for path in sorted(Path(directory).resolve().glob("*.csv")):
        inputs.add(path)
        original = rows(path)
        cold = "first_ns" in original[0]
        campaign = path.stem
        for serial, row in enumerate(original):
            data.append({"campaign": campaign, "method": "fresh-process elapsed" if cold else "Criterion elapsed",
                         "record": serial, "source_csv": source_path(path), **row})
        measured_sizes = sorted({int(r["bytes"]) for r in original})
        inventory.append({"campaign": campaign, "method": "fresh-process elapsed" if cold else "Criterion elapsed",
                          "rows": len(original), "sizes": ";".join(map(str, measured_sizes)),
                          "source_csv": source_path(path),
                          "source_status": "historical; exact per-file source digest not retained in this CSV"})
        text += [f"## {campaign}", "", f"Source: [{path.name}](<{path.resolve()}>).", ""]
        if cold:
            by = {}
            for row in original:
                by.setdefault((row["operation"], int(row["bytes"])), []).append(row)
            text += [table(["Operation", "Bytes", "First elapsed median ns", "First range ns", "Second median ns", "Repeats"],
                           [[op, size, statistics.median(int(r["first_ns"]) for r in rs),
                             f'{min(int(r["first_ns"]) for r in rs)}–{max(int(r["first_ns"]) for r in rs)}',
                             statistics.median(int(r["second_ns"]) for r in rs), len(rs)]
                            for (op, size), rs in sorted(by.items())]), ""]
        else:
            lookup = index(original, KEY)
            backends = sorted({r["backend"] for r in original})
            for operation in sorted({r["operation"] for r in original}):
                text += [f"### {operation}: elapsed median ns [confidence interval]", "", table(["Bytes", *backends],
                    [[size, *(f'{float(lookup[operation, backend, str(size)]["median_ns"]):.3f} '
                              f'[{float(lookup[operation, backend, str(size)]["ci_low_ns"]):.3f}, '
                              f'{float(lookup[operation, backend, str(size)]["ci_high_ns"]):.3f}]'
                              if (operation, backend, str(size)) in lookup else "NM"
                              for backend in backends)] for size in measured_sizes]), ""]
    return data, inventory, "\n".join(text)


def build(args):
    snapshot, output = Path(args.snapshot).resolve(), Path(args.output).resolve()
    safe_destination(output)
    inputs, metrics, campaigns, observations, size_rows = set(), [], [], [], []
    baseline_hashes = None
    for name in SIZE_DIRS:
        directory = snapshot / name
        inputs.update(directory / filename for filename in ("sizes.csv", "metadata.json"))
        if not name.startswith("before"):
            hashes = core_hashes(json.loads((directory / "metadata.json").read_text()))
            require(baseline_hashes is None or hashes == baseline_hashes, "size Rust snapshots differ")
            baseline_hashes = hashes
        for row in rows(directory / "sizes.csv"):
            size_rows.append({"size_configuration": name,
                              "snapshot_role": "before-dependency" if name.startswith("before") else "baseline",
                              "source_csv": source_path(directory / "sizes.csv"), **row})
    sizes = index(size_rows, ("size_configuration", "backend", "direction", "kind"))
    for row in size_rows:
        empty = sizes.get((row["size_configuration"], "empty", row["direction"], row["kind"]))
        row["stripped_delta_empty_bytes"] = (int(row["stripped_file_bytes"]) - int(empty["stripped_file_bytes"])) if empty else ""
    for config, prefix, directory in NATIVE:
        observations += load_native(snapshot, config, prefix, directory, inputs, sizes, metrics, campaigns)
    for config, directory in WASM:
        observations += load_wasm(snapshot, config, directory, inputs, sizes, metrics, campaigns)
    require(campaigns[-1]["driver_sha256"] == campaigns[-2]["driver_sha256"], "Wasm driver mismatch")
    require(campaigns[-1]["v8"] == campaigns[-2]["v8"], "Wasm engine mismatch")
    leaders, pareto = frontiers(observations)
    history, history_inventory, history_text = historical(inputs, args.historical_benchmarks)
    # Pin every archived Rust source byte shared by the size/runtime source manifests.
    archive = snapshot / "measured-source.tgz"
    inputs.add(archive)
    size_meta = json.loads((snapshot / "local-code-size/metadata.json").read_text())
    with tarfile.open(archive, "r:gz") as stream:
        for path, expected in size_meta["source_files_sha256"].items():
            if path.startswith("src/"):
                require(hashlib.sha256(stream.extractfile(path).read()).hexdigest() == expected,
                        f"source archive mismatch: {path}")
    inputs.add(Path(__file__).resolve())
    for _, prefix, _ in NATIVE:
        inputs.update(snapshot / f"{prefix}-timings" / name for name in ("raw.jsonl", "timings.csv", "caches.csv"))
        inputs.update(snapshot / f"{prefix}-allocations" / name for name in ("raw.jsonl", "allocations.csv", "caches.csv"))
    provenance = [{"path": source_path(path), "sha256": sha(path)} for path in sorted(inputs)]
    metric_sources = {}
    for row in metrics:
        key = row["configuration"], row["kind"], row["source_csv"]
        metric_sources[key] = metric_sources.get(key, 0) + 1
    output.mkdir()
    write_csv(output / "observations.csv", observations)
    write_csv(output / "summary-sources.csv", [{"configuration": config, "kind": kind,
              "source_summary_rows": len(rows(ROOT / path)), "referenced_metric_cells": count,
              "source_csv": path, "source_sha256": sha(ROOT / path)}
              for (config, kind, path), count in metric_sources.items()])
    write_csv(output / "linked-sizes.csv", size_rows)
    write_csv(output / "frontiers.csv", leaders)
    write_csv(output / "pareto-points.csv", pareto)
    write_csv(output / "historical.csv", history)
    write_csv(output / "historical-campaigns.csv", history_inventory)
    with (output / "grids.md").open("x") as stream:
        stream.write(grids(observations, leaders))
    with (output / "historical-grids.md").open("x") as stream:
        stream.write(history_text)
    with (output / "linked-grids.md").open("x") as stream:
        stream.write(linked_grids(size_rows, snapshot))
    report = {"schema": 1, "snapshot": source_path(snapshot),
              "scope": "frozen baseline only; no optimization A/B data",
              "campaigns": campaigns, "inputs": provenance,
              "counts": {"observations": len(observations), "measured": sum(r["status"] == "measured" for r in observations),
                         "skipped": sum(r["status"] == "skipped" for r in observations),
                         "not_measured": sum(r["status"] == "not_measured" for r in observations),
                         "metrics": len(metrics), "linked_artifacts": len(size_rows),
                         "frontiers": len(leaders), "pareto_points": len(pareto),
                         "historical_records": len(history), "historical_campaigns": len(history_inventory)},
              "outputs": {path.name: sha(path) for path in sorted(output.iterdir())}}
    with (output / "metadata.json").open("x") as stream:
        json.dump(report, stream, indent=2)
        stream.write("\n")
    print(json.dumps(report["counts"], sort_keys=True))


def self_test():
    require(dominates([1, 2], [2, 2]), "strict objective")
    require(not dominates([1, 2], [1, 2]), "ties must survive")
    require(not dominates([1, 3], [2, 2]), "tradeoff must survive")
    candidates = [{"backend": "a", "time": 1, "size": 3}, {"backend": "b", "time": 2, "size": 2},
                  {"backend": "c", "time": 2, "size": 3}, {"backend": "d", "time": 1, "size": 3}]
    require(frontier(candidates, ["time", "size"]) == ["a", "b", "d"], "incorrect Pareto frontier")
    require(route("m5-native", "auto", "decode", 1024, 1399, False) == "general matrix", "ARM route")
    require(route("m5-v8", "auto", "decode", 1024, 1399, False).startswith("batched"), "Wasm route")
    require(route("zen5-native", "matrix", "decode", 8192, 11188, True).endswith("dc"), "matrix fallback")
    require(route("zen5-generic", "auto", "decode", 4096, 5594, True).startswith("recursive"), "IFMA route")
    for call in (lambda: number("nan"), lambda: index([{"k": "x"}, {"k": "x"}], ["k"])):
        try:
            call()
        except ValueError:
            pass
        else:
            raise AssertionError("bad data accepted")
    with tempfile.TemporaryDirectory(prefix="base58-summary-self-test-") as directory:
        path = Path(directory)
        require(source_path(ROOT / "scripts/../README.md") == "README.md", "relative locator")
        require(source_path(path) == str(path.resolve()), "external locator")
        write_csv(path / "cold.csv", [{"operation": "encode", "bytes": 1,
                                      "first_ns": 2, "second_ns": 1}])
        data, inventory, _ = historical(set(), path)
        require(len(data) == len(inventory) == 1, "external archive load")
        require(data[0]["source_csv"] == str((path / "cold.csv").resolve()), "archive source locator")
        require(f"[sizes.csv](<{path / SIZE_DIRS[0] / 'sizes.csv'}>)" in linked_grids([], path),
                "external archive report link")
        marker = path / "manual.txt"
        marker.write_text("preserve\n")
        try:
            safe_destination(path)
        except ValueError:
            require(marker.read_text() == "preserve\n", "manual content changed")
        else:
            raise AssertionError("existing directory accepted")
    print("backend_summary self-test passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", help="archived full raw snapshot; curated summaries are not raw inputs")
    parser.add_argument("--historical-benchmarks", help="archived Criterion CSV directory for a historical rebuild")
    parser.add_argument("--output")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
    else:
        if not args.output:
            parser.error("--output is required")
        if not args.snapshot or not args.historical_benchmarks:
            parser.error("archive rebuild requires --snapshot and --historical-benchmarks; retained summaries are in docs/benchmarks")
        try:
            build(args)
        except ValueError as error:
            parser.exit(2, f"{error}\n")


if __name__ == "__main__":
    main()

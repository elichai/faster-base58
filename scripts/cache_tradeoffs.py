#!/usr/bin/env python3
"""Derive fair same-binary cache CPU/heap tradeoffs from resource-profile CSVs."""

import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import statistics


def read_rows(directory, filename):
    with (directory / filename).open(newline="") as stream:
        return list(csv.DictReader(stream))


def groups(rows):
    result = {}
    for row in rows:
        key = row["operation"], row["backend"], int(row["bytes"]), row["phase"]
        result.setdefault(key, []).append(row)
    return result


def median(rows, field):
    return statistics.median(float(row[field]) for row in rows)


def time_values(rows, field="cpu_ns"):
    return [float(row[field]) / int(row["iterations"]) for row in rows]


def estimate(control_cold, cached_cold, control_warm, cached_warm, active):
    saving = control_warm - cached_warm
    if not active:
        return saving, None, None, "no_dynamic_table"
    if saving <= 0:
        return saving, None, None, "no_positive_warm_savings"
    setup_ratio = max(0, cached_cold - cached_warm) / saving
    first_difference = cached_cold - control_cold
    fresh_calls = 1 + math.ceil(max(0, first_difference) / saving)
    return saving, setup_ratio, fresh_calls, "estimate"


def compare(args):
    allocation_dir, timing_dir = Path(args.allocations), Path(args.timings)
    alloc_meta = json.loads((allocation_dir / "metadata.json").read_text())
    time_meta = json.loads((timing_dir / "metadata.json").read_text())
    assert alloc_meta["status"] == time_meta["status"] == "complete"
    assert alloc_meta["build"]["source"] == time_meta["build"]["source"], "source snapshots differ"
    assert alloc_meta["build"]["worker"]["count_allocations"]
    assert not time_meta["build"]["worker"]["count_allocations"]
    assert alloc_meta["build"]["worker"]["arch"] == time_meta["build"]["worker"]["arch"]
    assert alloc_meta["build"]["worker"]["ifma_threshold"] == time_meta["build"]["worker"]["ifma_threshold"]
    for field in ["platform", "machine", "hostname", "rustc", "flags", "cargo_configs"]:
        assert alloc_meta["build"]["environment"][field] == time_meta["build"]["environment"][field], field
    allocations = groups(read_rows(allocation_dir, "allocations.csv"))
    timings = groups(read_rows(timing_dir, "timings.csv"))
    output = []
    for kind, control, cached in [("ntt-twiddles", "ntt-same-owned", "ntt-same-cached"),
                                  ("general-matrix-vs-batched", "batched", "matrix-forced")]:
        cases = sorted({(operation, size) for operation, backend, size, phase in timings
                        if backend == cached and phase == "warm"})
        for operation, size in cases:
            keys = [(operation, backend, size, phase) for backend in [control, cached] for phase in ["cold", "warm"]]
            if any(key not in allocations or key not in timings for key in keys):
                continue
            control_warm_rows = timings[operation, control, size, "warm"]
            cached_warm_rows = timings[operation, cached, size, "warm"]
            control_cpu, cached_cpu = time_values(control_warm_rows), time_values(cached_warm_rows)
            control_warm, cached_warm = statistics.median(control_cpu), statistics.median(cached_cpu)
            control_cold = statistics.median(time_values(timings[operation, control, size, "cold"]))
            cached_cold = statistics.median(time_values(timings[operation, cached, size, "cold"]))
            control_memory = allocations[operation, control, size, "warm"]
            cached_memory = allocations[operation, cached, size, "warm"]
            cache_fields = ["encode_cache_bytes", "decode_cache_bytes", "ntt_cache_bytes"]
            control_retained = sum(median(control_memory, field) for field in cache_fields)
            cached_retained = sum(median(cached_memory, field) for field in cache_fields)
            table_field = "ntt_cache_bytes" if kind == "ntt-twiddles" else f"{operation}_cache_bytes"
            active = median(cached_memory, table_field) > 0
            saving, setup_ratio, fresh_calls, status = estimate(
                control_cold, cached_cold, control_warm, cached_warm, active)
            overlap = min(control_cpu) <= max(cached_cpu) and min(cached_cpu) <= max(control_cpu)
            output.append({
                "comparison": kind, "operation": operation, "bytes": size,
                "control": control, "cached": cached, "status": status,
                "warm_cpu_ranges_overlap": overlap,
                "control_warm_cpu_ns": control_warm, "cached_warm_cpu_ns": cached_warm,
                "control_warm_cpu_min_ns": min(control_cpu), "control_warm_cpu_max_ns": max(control_cpu),
                "cached_warm_cpu_min_ns": min(cached_cpu), "cached_warm_cpu_max_ns": max(cached_cpu),
                "control_warm_wall_ns": statistics.median(time_values(control_warm_rows, "wall_ns")),
                "cached_warm_wall_ns": statistics.median(time_values(cached_warm_rows, "wall_ns")),
                "control_cold_cpu_ns": control_cold, "cached_cold_cpu_ns": cached_cold,
                "warm_cpu_saved_ns": saving,
                "warm_cpu_saved_percent": 100 * saving / control_warm if control_warm else None,
                "cached_cold_minus_warm_cpu_ns": cached_cold - cached_warm,
                "first_call_cached_minus_control_cpu_ns": cached_cold - control_cold,
                "approx_cold_setup_over_warm_savings_calls": setup_ratio,
                "fresh_vs_fresh_break_even_calls": fresh_calls,
                "control_retained_cache_bytes": control_retained,
                "cached_retained_cache_bytes": cached_retained,
                "extra_retained_cache_bytes": cached_retained - control_retained,
                "control_warm_peak_extra_bytes": median(control_memory, "peak_extra_bytes"),
                "cached_warm_peak_extra_bytes": median(cached_memory, "peak_extra_bytes"),
                "control_active_peak_cache_plus_extra_bytes": control_retained + median(control_memory, "peak_extra_bytes"),
                "cached_active_peak_cache_plus_extra_bytes": cached_retained + median(cached_memory, "peak_extra_bytes"),
            })
    destination = Path(args.output)
    assert output, "no comparable cases"
    provenance = destination.with_suffix(".metadata.json")
    if destination.exists() or provenance.exists():
        raise SystemExit("refusing existing comparison output")
    with destination.open("x", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(output[0]))
        writer.writeheader()
        writer.writerows(output)
    files = [allocation_dir / "allocations.csv", allocation_dir / "metadata.json",
             timing_dir / "timings.csv", timing_dir / "metadata.json", Path(__file__)]
    with provenance.open("x") as stream:
        json.dump({"schema": 1, "source_sha256": time_meta["build"]["source"]["sha256"],
                   "inputs": [{"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                              for path in files]}, stream, indent=2)
    print(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--allocations")
    parser.add_argument("--timings")
    parser.add_argument("--output")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        assert estimate(100, 400, 50, 20, True) == (30, 380 / 30, 11, "estimate")
        assert estimate(100, 400, 20, 50, True)[1:] == (None, None, "no_positive_warm_savings")
        assert estimate(100, 400, 50, 20, False)[1:] == (None, None, "no_dynamic_table")
        assert estimate(100, 80, 50, 20, True)[2] == 1
        print("cache_tradeoffs self-test passed")
    else:
        assert args.allocations and args.timings and args.output
        compare(args)


if __name__ == "__main__":
    main()

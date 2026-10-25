#!/usr/bin/env python3
"""Export this crate's Criterion dispatch estimates: DIR BASELINE, or --self-test."""

import csv
import json
from pathlib import Path
import sys
import tempfile


def rows(root, baseline):
    result = []
    for metadata in root.rglob("benchmark.json"):
        if metadata.parent.name != baseline:
            continue
        ident = json.loads(metadata.read_text())["full_id"].split("/")
        if len(ident) != 4 or ident[0] != "dispatch":
            continue
        median = json.loads((metadata.parent / "estimates.json").read_text())["median"]
        interval = median["confidence_interval"]
        result.append((ident[1], ident[2], int(ident[3]), median["point_estimate"],
                       interval["lower_bound"], interval["upper_bound"]))
    return sorted(result, key=lambda row: (row[0], row[2], row[1]))


def self_test():
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        saved = root / "dispatch" / "encode_matrix" / "32" / "main"
        saved.mkdir(parents=True)
        (saved / "benchmark.json").write_text(json.dumps({"full_id": "dispatch/encode/matrix/32"}))
        (saved / "estimates.json").write_text(json.dumps({"median": {
            "point_estimate": 12.5,
            "confidence_interval": {"lower_bound": 12.0, "upper_bound": 13.0},
        }}))
        assert rows(root, "main") == [("encode", "matrix", 32, 12.5, 12.0, 13.0)]
        assert rows(root, "missing") == []
    print("bench_report self-test passed")


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        self_test()
    else:
        if len(sys.argv) != 3:
            raise SystemExit(__doc__)
        measurements = rows(Path(sys.argv[1]), sys.argv[2])
        if not measurements:
            raise SystemExit("No dispatch measurements found")
        writer = csv.writer(sys.stdout)
        writer.writerow(("operation", "backend", "bytes", "median_ns", "ci_low_ns", "ci_high_ns"))
        writer.writerows(measurements)

#!/usr/bin/env python3
"""Render consumer charts/tables from frozen reports; never run benchmarks."""
import argparse
import csv
import hashlib
import html
import json
import math
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
TIMINGS = ROOT / "docs/tradeoffs/2026-10-25/ac-power/timings.csv"
SIZES = ROOT / "docs/tradeoffs/2026-10-25/allocation-layout/sizes.csv"
PAYLOADS = [32, 64, 256, 1024, 4096, 65536]
PROFILES = [
    ("m5-native", "M5 Pro", "warm_cpu_ns", "m5"),
    ("gcp-default", "GCP AMD EPYC 9B45 (generic)", "warm_cpu_ns", "gcp-generic"),
    ("gcp-native", "GCP AMD EPYC 9B45 (native)", "warm_cpu_ns", "gcp-native"),
    ("m5-wasm", "Wasm / V8 on M5 Pro", "warm_wall_ns", "wasm"),
]
MODES = ["Auto", "Auto + cache", "no_alloc", "bs58"]
COLORS = ["#075c9b", "#853da2", "#007960", "#687487"]
DASHES = ["none", "7 3", "2 3", "9 3 2 3"]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def csv_rows(path):
    with path.open(newline="") as stream:
        return list(csv.DictReader(stream))


def duration(value, places=2):
    for scale, unit in [(1e9, "s"), (1e6, "ms"), (1e3, "µs"), (1, "ns")]:
        if value >= scale or scale == 1:
            return f"{value / scale:.{places}f} {unit}"


rows = csv_rows(TIMINGS)
points = []
for profile, label, metric, _ in PROFILES:
    for operation in ["encode", "decode"]:
        for mode in MODES:
            for size in PAYLOADS:
                found = [(i, row) for i, row in enumerate(rows, 2) if (row["profile"], row["operation"], row["mode"], row["metric"], int(row["bytes"])) == (profile, operation, mode, metric, size)]
                require(len(found) == 1, f"Missing/duplicate timing cell: {profile}/{operation}/{mode}/{size}")
                line, row = found[0]
                values = [float(row[k]) for k in ["minimum", "median", "maximum"]]
                require(all(math.isfinite(v) and v > 0 for v in values) and values[0] <= values[1] <= values[2], "Invalid process range")
                require(int(row["processes"]) == 3, "Expected three process medians")
                points.append(dict(profile=profile, label=label, operation=operation, mode=mode, bytes=size, metric=metric, minimum_ns=values[0], median_ns=values[1], maximum_ns=values[2], source_line=line))


def figure(operation):
    parts = ['<svg xmlns="http://www.w3.org/2000/svg" width="1120" height="800" viewBox="0 0 1120 800" role="img">',
             f'<title>{operation.capitalize()} time by input size</title>',
             '<desc>Warm per-conversion medians for Auto, optional cache, no_alloc and bs58. Native uses process CPU; Wasm uses elapsed time. Both axes are logarithmic.</desc>',
             '<rect width="1120" height="800" fill="white"/>',
             '<style>text{font-family:Arial,sans-serif;fill:#1e293b;font-size:13px}.title{font-size:23px;font-weight:700}.panel{font-size:16px;font-weight:700}.grid{stroke:#e2e8f0;stroke-width:1}</style>']
    def text(x, y, value, extra=""):
        parts.append(f'<text x="{x:.2f}" y="{y:.2f}" {extra}>{html.escape(str(value))}</text>')
    text(28, 32, operation.capitalize() + " · time per conversion", 'class="title"')
    text(28, 56, "2026-10-25 snapshot · before rename and no_std · reusable output buffers · lower is faster")
    for index, (mode, color, dash) in enumerate(zip(MODES, COLORS, DASHES)):
        x = 30 + index * 270
        parts.append(f'<path d="M{x} 82h36" stroke="{color}" stroke-width="3" stroke-dasharray="{dash}"/>')
        text(x + 44, 86, mode)
    for index, (profile, label, metric, _) in enumerate(PROFILES):
        left, top, width, height = 82 + index % 2 * 550, 144 + index // 2 * 312, 428, 218
        selected = [p for p in points if p["profile"] == profile and p["operation"] == operation]
        lo = math.floor(math.log10(min(p["minimum_ns"] for p in selected)))
        hi = math.ceil(math.log10(max(p["maximum_ns"] for p in selected)))
        x = lambda n: left + (math.log2(n) - 5) / 11 * width
        y = lambda n: top + height * (hi - math.log10(n)) / (hi - lo)
        clock = "elapsed" if profile == "m5-wasm" else "process CPU"
        text(left, top - 19, label + " · " + clock, 'class="panel"')
        for exponent in range(lo, hi + 1):
            yy = y(10 ** exponent)
            parts.append(f'<path d="M{left} {yy}h{width}" class="grid"/>')
            text(left - 8, yy + 4, duration(10 ** exponent, 0), 'text-anchor="end"')
        for size, tick in zip(PAYLOADS, ["32 B", "64 B", "256 B", "1 KiB", "4 KiB", "64 KiB"]):
            xx = x(size)
            parts.append(f'<path d="M{xx} {top}v{height}" class="grid"/>')
            text(xx, top + height + 23, tick, 'text-anchor="middle"')
        text(left + width / 2, top + height + 47, "Input bytes" if operation == "encode" else "Decoded payload bytes", 'text-anchor="middle"')
        for mode, color, dash in zip(MODES, COLORS, DASHES):
            series = sorted((p for p in selected if p["mode"] == mode), key=lambda p: p["bytes"])
            vertices = " ".join(f"{x(p['bytes'])},{y(p['median_ns'])}" for p in series)
            parts.append(f'<polyline fill="none" stroke="{color}" stroke-width="2.2" stroke-dasharray="{dash}" points="{vertices}"/>')
            for p in series:
                xx, yy = x(p["bytes"]), y(p["median_ns"])
                a, b = y(p["minimum_ns"]), y(p["maximum_ns"])
                parts.append(f'<path d="M{xx} {a}V{b} M{xx-3} {a}h6 M{xx-3} {b}h6" stroke="{color}" stroke-width="1"/>')
                parts.append(f'<circle cx="{xx}" cy="{yy}" r="3" fill="{color}"><title>{html.escape(mode)}: {duration(p["median_ns"])}; range {duration(p["minimum_ns"])}–{duration(p["maximum_ns"])}</title></circle>')
    text(28, 773, "Dots: median of 3 process medians. Bars: observed process range. Native and Wasm clocks are different.")
    parts.append("</svg>\n")
    return "\n".join(parts)


performance = ["| Host / operation (4 KiB) | Auto | Auto + `cache` | `no_alloc` | bs58 |", "| --- | ---: | ---: | ---: | ---: |"]
for profile, label, _, _ in PROFILES:
    for operation in ["encode", "decode"]:
        selected = [next(p for p in points if (p["profile"], p["operation"], p["bytes"], p["mode"]) == (profile, operation, 4096, mode)) for mode in MODES]
        lowest = min(p["median_ns"] for p in selected)
        cells = [f"**{duration(p['median_ns'])}**" if p["median_ns"] == lowest else duration(p["median_ns"]) for p in selected]
        performance.append(f"| {label} / {operation} | " + " | ".join(cells) + " |")
performance = "\n".join(performance)
size_rows = csv_rows(SIZES)
size_data, size_table = [], ["| Target | Auto | Auto + `cache` | `no_alloc` | bs58 |", "| --- | ---: | ---: | ---: | ---: |"]
for _, label, _, profile in PROFILES:
    selected = []
    for config, backend in [("default", "auto"), ("cache", "auto"), ("default", "no-alloc"), ("default", "bs58")]:
        matches = [r for r in size_rows if (r["profile"], r["config"], r["variant"], r["backend"]) == (profile, config, "v011", backend)]
        require(len(matches) == 1 and matches[0]["direction"] == "both", "Missing/mismatched linked-size cell")
        row = matches[0]
        selected.append(f"{int(row['stripped_file_bytes']) / 1024:.1f}")
        size_data.append(dict(profile=profile, config=config, backend=backend, kind=row["kind"], stripped_bytes=int(row["stripped_file_bytes"]), source_id=row["source_id"], artifact_sha256=row["stripped_sha256"]))
    size_table.append(f"| {label} | " + " | ".join(selected) + " |")
size_table = "\n".join(size_table)
data = dict(snapshot="2026-10-25, before package rename and no_std", timing_source=str(TIMINGS.relative_to(ROOT)), size_source=str(SIZES.relative_to(ROOT)), timing_points=points, linked_sizes=size_data)
outputs = {"encode.svg": figure("encode"), "decode.svg": figure("decode"), "data.json": json.dumps(data, indent=2) + "\n", "tables.md": performance + "\n\n" + size_table + "\n"}
inputs = {str(p.relative_to(ROOT)): digest(p) for p in [TIMINGS, SIZES]}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--check", action="store_true", help="check committed graphs, input hashes and README table cells")
args = parser.parse_args()
if args.check:
    receipt = json.loads((HERE / "receipt.json").read_text())
    require(receipt["inputs"] == inputs and receipt["renderer_sha256"] == digest(Path(__file__)), "Renderer/input hash mismatch")
    for name, content in outputs.items():
        require((HERE / name).read_text() == content, "Generated output differs: " + name)
        require(receipt["outputs"][name] == digest(HERE / name), "Receipt output hash mismatch")
    guide = (ROOT / "README.md").read_text()
    require(performance in guide and size_table in guide, "README table differs from frozen numeric inputs")
else:
    require(all(not (HERE / n).exists() for n in [*outputs, "receipt.json"]), "Refusing to overwrite generated files")
    for name, content in outputs.items():
        with (HERE / name).open("x") as stream:
            stream.write(content)
    receipt = dict(status="pass", snapshot=data["snapshot"], inputs=inputs, renderer_sha256=digest(Path(__file__)), python=sys.version,
                   timing_points=len(points), linked_size_cells=len(size_data), outputs={name: digest(HERE / name) for name in outputs})
    with (HERE / "receipt.json").open("x") as stream:
        json.dump(receipt, stream, indent=2)
        stream.write("\n")
for name in ["encode.svg", "decode.svg"]:
    svg = ET.fromstring((HERE / name).read_bytes())
    require(len(svg.findall("{http://www.w3.org/2000/svg}polyline")) == 16, "Wrong SVG series count")
    require(len(svg.findall("{http://www.w3.org/2000/svg}circle")) == 96, "Wrong SVG point count")
print(json.dumps(dict(status="pass", mode="check" if args.check else "render", timing_points=len(points), linked_size_cells=len(size_data))))

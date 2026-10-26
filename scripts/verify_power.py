#!/usr/bin/env python3
"""Audit the five frozen AC phases from raw inputs; never run measured commands.

verify_power(evidence, config) uses Evidence.path(value) and Evidence.read(Path).
config contains directory and guard_sha256. All reads, including this checker,
are passed through Evidence.read so a report can bind the actual inputs.
"""
import argparse
import copy
import csv
from datetime import datetime, timedelta, timezone
import hashlib
import io
import json
import math
from pathlib import Path
import re

ORIGINAL = "/private/tmp/base58-ac-cfoB8sqN"
GUARD_SHA = "cae1b520677e3b4c0bef31fe9f96856f5e2582d1f42a1529f3eb3dd7d096dc5e"
SIZES = (0, 1, 8, 16, 21, 22, 23, 31, 32, 33, 63, 64, 65, 128, 256, 512,
         1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144,
         524288, 1048576, 2097152, 4194304)
PUBLIC = ("auto", "no-alloc", "bs58")
FLAGS = ("--no-liftoff", "--no-wasm-lazy-compilation", "--no-wasm-async-compilation")
PHASES = {
    "native-public": ("native-public", PUBLIC, SIZES),
    "native-fixed": ("native-fixed", ("batched", "matrix", "matrix-portable", "matrix-forced"), (32, 64)),
    "wasm-default": ("wasm-public-default", PUBLIC, SIZES),
    "wasm-cache": ("wasm-public-cache", PUBLIC, SIZES),
    "wasm-fixed": ("wasm-fixed-timing", ("auto", "batched", "matrix", "matrix-portable", "matrix-forced"), (32, 64)),
}
LIMITS = [
    "AC is sampled, not continuously observed; changes between samples are not excluded.",
    "Low-power mode is checked before/after each phase, not continuously.",
    "pmset therm output is raw diagnostic evidence, not temperature, frequency or thermal-headroom telemetry.",
    "This checker validates power and prepared grids; the report separately validates timing samples and artifacts.",
    "Native cohort UTC metadata has second granularity; its guard-window check permits that quantization.",
]


def require(value, message):
    if not value:
        raise ValueError(message)


def utc(value):
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    require(parsed.utcoffset() == timedelta(0), f"not UTC: {value}")
    return parsed


def low_power(settings):
    raw = settings["custom"]
    require(raw["returncode"] == 0 and not raw["stderr"], "custom power query failed")
    section, values = None, []
    for line in raw["stdout"].splitlines():
        if line.strip().endswith(":"):
            section = line.strip()[:-1]
        match = re.fullmatch(r"\s*lowpowermode\s+(\d+)\s*", line)
        if match and section == "AC Power":
            values.append(int(match[1]))
    require(values == [0], "missing/ambiguous/nonzero AC lowpowermode")
    return 0


def phase_log(rows, command, guard_sha):
    require(len(rows) >= 5 and rows[0]["kind"] == "start" and rows[-1]["kind"] == "finish", "missing start/finish")
    kinds = [r["kind"] for r in rows]
    require(kinds.count("start") == kinds.count("finish") == kinds.count("child") == 1, "duplicate/missing boundary or child")
    require(set(kinds) <= {"start", "finish", "power", "child"}, "error/unknown power-log record")
    times = [utc(r["utc"]) for r in rows]
    ticks = [r["monotonic_ns"] for r in rows]
    require(all(type(n) is int and n >= 0 for n in ticks), "invalid monotonic clock")
    require(all(a <= b for a, b in zip(times, times[1:])), "UTC order reversed")
    require(all(a < b for a, b in zip(ticks, ticks[1:])), "monotonic order reversed")
    start, finish = rows[0], rows[-1]
    require(start["command"] == command, "prepared command array differs")
    require(start["script_sha256"] == guard_sha, "frozen guard identity differs")
    interval = start["interval_seconds"]
    require(type(interval) in (int, float) and math.isfinite(interval) and 0 < interval <= 60, "invalid sample interval")
    require(finish["accepted"] is True and type(finish["command_exit"]) is int and finish["command_exit"] == 0 and finish["reason"] == "", "phase rejected/command failed")
    child = rows[kinds.index("child")]
    require(type(child["pid"]) is int and child["pid"] > 0, "invalid child PID")
    powers = [r for r in rows if r["kind"] == "power"]
    require(len(powers) >= 2 and powers[0]["phase"] == "before" and powers[-1]["phase"] == "after", "missing before/after AC sample")
    require(all(r["phase"] == "sample" for r in powers[1:-1]), "duplicate/unknown AC sample phase")
    require(rows.index(powers[0]) < kinds.index("child") < rows.index(powers[-1]), "child outside AC samples")
    for row in powers:
        lines = row["stdout"].splitlines()
        require(row["ac"] is True and row["returncode"] == 0 and not row["stderr"]
                and lines and lines[0] == "Now drawing from 'AC Power'", "failed/battery AC observation")
    thermal = []
    for when, row in (("before", start), ("after", finish)):
        low_power(row["settings"])
        raw = row["settings"]["thermal"]
        require(set(raw) >= {"returncode", "stdout", "stderr"}, "missing raw thermal diagnostic")
        unsupported = raw["returncode"] != 0 or bool(re.search(r"error|failed|no CPU power status", raw["stdout"] + raw["stderr"], re.I))
        thermal.append(dict(when=when, returncode=raw["returncode"],
                            interpretation="unsupported/error diagnostic" if unsupported else "raw, uninterpreted diagnostic"))
    return dict(start_utc=start["utc"], end_utc=finish["utc"],
                start_monotonic_ns=ticks[0], end_monotonic_ns=ticks[-1], duration_ns=ticks[-1] - ticks[0],
                child_pid=child["pid"], ac_samples=len(powers), interval_seconds=interval,
                max_observed_sample_gap_ns=max(b["monotonic_ns"] - a["monotonic_ns"] for a, b in zip(powers, powers[1:])),
                lowpowermode_before=0, lowpowermode_after=0, thermal=thermal)


def csv_rows(data):
    return list(csv.DictReader(io.StringIO(data.decode())))


def keys(rows, fields):
    result = [tuple(str(row[f]) for f in fields) for row in rows]
    require(len(result) == len(set(result)), f"duplicate grid key: {fields}")
    return set(result)


def grid(evidence, directory, phase, command, window):
    output, backends, sizes = PHASES[phase]
    native = phase.startswith("native-")
    prefix = ["rtk", "proxy", "python3"] if native else ["rtk", "proxy", "node", *FLAGS]
    require(command[:len(prefix)] == prefix, f"unexpected prepared executable: {phase}")
    driver = len(prefix)
    require(Path(command[driver]).name == ("compare_threshold.py" if native else "wasm_tradeoffs.mjs"), "unexpected driver")
    tail = command[driver + (1 if native else 3):]
    require(len(tail) % 2 == 0 and len(set(tail[::2])) == len(tail[::2]), "invalid/duplicate command options")
    options = dict(zip(tail[::2], tail[1::2]))
    require(options["--backends"] == ",".join(backends) and options["--sizes"] == ",".join(map(str, sizes)), "prepared grid differs")
    require(options["--repeats"] == options["--samples"] == "3", "prepared repeat/sample count differs")
    target = evidence.path(options["--output"] if native else command[driver + 2])
    require(target == (directory / output).resolve(), "prepared output directory differs")
    data = {name: evidence.read(target / name) for name in ("metadata.json", "summary.csv", "skipped.csv")}
    meta, summary, skipped = json.loads(data["metadata.json"]), csv_rows(data["summary.csv"]), csv_rows(data["skipped.csv"])
    all_cases = {(op, backend, str(size)) for op in ("encode", "decode") for backend in backends for size in sizes}
    omitted = {(op, backend, str(size)) for op, backend, size in all_cases if backend in ("bs58", "no-alloc") and int(size) > 65_536}
    expected = all_cases - omitted
    require(meta["arguments"]["repeats"] == meta["arguments"]["samples"] == 3 if native else meta["repeats"] == meta["samples"] == 3, "metadata repeat/sample count differs")
    if native:
        require(meta["status"] == "complete" and meta["command"] == command[driver:], "incomplete/mismatched native command")
        require(meta["arguments"]["backends"] == options["--backends"] and meta["arguments"]["sizes"] == options["--sizes"], "native argument grid differs")
        require(len(meta["cases"]) == len(expected) and {tuple(map(str, r)) for r in meta["cases"]} == expected, "native cases incomplete")
        process = csv_rows(evidence.read(target / "processes.csv"))
        wanted = {(*case, str(repeat), side) for case in expected for repeat in range(3) for side in ("baseline", "candidate")}
        require(keys(process, ("operation", "backend", "bytes", "repeat", "side")) == wanted, "native process grid incomplete")
        require(meta["expected_processes"] == meta["completed_processes"] == len(wanted) and meta["complete_cases"] == len(expected), "native completion count differs")
        require(keys(summary, ("operation", "backend", "bytes", "metric")) == {(*case, metric) for case in expected for metric in ("warm_cpu_ns", "warm_wall_ns", "cold_wall_ns")}, "native summary grid incomplete")
        require(not csv_rows(evidence.read(target / "missing.csv")), "native missing cells")
        require(all(r["baseline_cap"] == r["candidate_cap"] == "65536" for r in skipped), "unexpected native cap")
        begin, end = meta["started_utc"], meta["finished_utc"]
        fresh = len(wanted)
    else:
        require(meta["backends"] == list(backends) and meta["sizes"] == list(sizes), "Wasm metadata grid differs")
        require(meta["modules_directory"] == command[driver + 1] and options["--seed"] == "20261024", "Wasm prepared modules/seed differ")
        complete = json.loads(evidence.read(target / "complete.json"))
        for name, raw in data.items():
            require(hashlib.sha256(raw).hexdigest() == complete["sha256"][name], f"Wasm completion hash differs: {name}")
        wanted = {(*case, str(repeat)) for case in expected for repeat in range(3)}
        require(keys(summary, ("operation", "backend", "decoded_bytes", "repeat")) == wanted, "Wasm process grid incomplete")
        require(complete["completed_cases"] == len(wanted), "Wasm completion count differs")
        begin, end = meta["created_utc"], complete["completed_utc"]
        fresh = len(wanted)
    require(keys(skipped, ("operation", "backend", "bytes" if native else "decoded_bytes")) == omitted, "missing/unexpected cap skips")
    slack = timedelta(seconds=1) if native else timedelta(milliseconds=1)
    require(utc(window["start_utc"]) - slack <= utc(begin) <= utc(end) <= utc(window["end_utc"]) + slack, "cohort UTC outside power window")
    return dict(cases=len(expected), cap_skips=len(omitted), fresh_processes=fresh, samples_per_process=3)


def nonoverlap(phases):
    ordered = sorted(phases, key=lambda r: r["start_monotonic_ns"])
    for left, right in zip(ordered, ordered[1:]):
        require(left["end_monotonic_ns"] <= right["start_monotonic_ns"] and utc(left["end_utc"]) <= utc(right["start_utc"]), "power phases overlap")
    return ordered


def verify_power(evidence, config):
    evidence.read(Path(__file__).resolve())
    directory = evidence.path(config["directory"])
    commands = json.loads(evidence.read(directory / "commands.json"))
    require(set(commands) == set(PHASES), "prepared phase set differs")
    guard_sha = hashlib.sha256(evidence.read(directory / "ac_power_run.py")).hexdigest()
    require(guard_sha == config["guard_sha256"], "frozen guard file changed")
    phases = []
    for name in PHASES:
        rows = [json.loads(line) for line in evidence.read(directory / f"{name}-power.jsonl").decode().splitlines()]
        result = dict(phase=name, **phase_log(rows, commands[name], guard_sha))
        result["grid"] = grid(evidence, directory, name, commands[name], result)
        phases.append(result)
    phases = nonoverlap(phases)
    return dict(status="pass", guard_sha256=guard_sha, phases=phases,
                total_fresh_processes=sum(r["grid"]["fresh_processes"] for r in phases),
                thermal_telemetry="raw-only; no temperature/frequency/headroom claim", limits=LIMITS)


def self_test():
    settings = dict(custom=dict(returncode=0, stdout="AC Power:\n lowpowermode 0\n", stderr=""),
                    thermal=dict(returncode=0, stdout="Error: No CPU power status\n", stderr=""))
    rows = [dict(kind="start", command=["prepared"], interval_seconds=30, script_sha256="guard", settings=settings),
            dict(kind="power", phase="before", ac=True, returncode=0, stdout="Now drawing from 'AC Power'\n", stderr=""),
            dict(kind="child", pid=123),
            dict(kind="power", phase="after", ac=True, returncode=0, stdout="Now drawing from 'AC Power'\n", stderr=""),
            dict(kind="finish", accepted=True, command_exit=0, reason="", settings=settings)]
    for i, row in enumerate(rows):
        row.update(utc=f"2026-10-25T00:00:0{i}+00:00", monotonic_ns=i * 1_000_000_000)
    require(phase_log(rows, ["prepared"], "guard")["thermal"][0]["interpretation"] == "unsupported/error diagnostic", "thermal false availability")
    mutations = [lambda r: r[-1].update(accepted=False), lambda r: r.pop(),
                 lambda r: r.pop(3), lambda r: r[1].update(stdout="Now drawing from 'Battery Power'\n"),
                 lambda r: r.insert(3, dict(r[2], kind="error")),
                 lambda r: r[0].update(command=["different"]), lambda r: r[0].update(script_sha256="changed"),
                 lambda r: r[-1].update(command_exit=1), lambda r: r[2].update(monotonic_ns=0),
                 lambda r: r[2].update(utc="2026-10-24T00:00:00+00:00"),
                 lambda r: r[0]["settings"]["custom"].update(stdout="AC Power:\n lowpowermode 1\n")]
    for mutate in mutations:
        bad = copy.deepcopy(rows)
        mutate(bad)
        try:
            phase_log(bad, ["prepared"], "guard")
        except ValueError:
            pass
        else:
            raise ValueError("corrupt power fixture accepted")
    try:
        nonoverlap([phase_log(rows, ["prepared"], "guard")] * 2)
    except ValueError:
        pass
    else:
        raise ValueError("overlapping phases accepted")
    return dict(status="pass", rejected_cases=len(mutations) + 1)


class LocalEvidence:
    def __init__(self, directory):
        self.directory, self.inputs = directory.resolve(strict=True), {}

    def path(self, value):
        text = str(value)
        if text == ORIGINAL or text.startswith(ORIGINAL + "/"):
            return (self.directory / text[len(ORIGINAL):].lstrip("/")).resolve(strict=True)
        return Path(value).resolve(strict=True)

    def read(self, path):
        path = Path(path)
        data = path.read_bytes()
        self.inputs[str(path)] = hashlib.sha256(data).hexdigest()
        return data


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--directory", type=Path)
    parser.add_argument("--output", type=Path, help="fresh audit receipt directory")
    args = parser.parse_args()
    check = self_test()
    if args.self_test:
        print(f"Power audit self-check passed: {check['rejected_cases']} rejected corrupt cases")
    if args.directory:
        require(args.output is not None, "--directory requires --output")
        evidence = LocalEvidence(args.directory)
        result = verify_power(evidence, dict(directory=args.directory, guard_sha256=GUARD_SHA))
        result.update(checker_self_test=check, input_sha256=evidence.inputs,
                      audited_utc=datetime.now(timezone.utc).isoformat())
        args.output.mkdir(parents=True, exist_ok=True)
        with (args.output / "audit.json").open("x") as stream:
            json.dump(result, stream, indent=2)
            stream.write("\n")
        print(f"Power audit passed: {len(result['phases'])} non-overlapping phases, {result['total_fresh_processes']} fresh processes")
    else:
        require(args.self_test, "provide --self-test or --directory and --output")

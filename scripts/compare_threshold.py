#!/usr/bin/env python3
"""Compare two frozen, uninstrumented workers using serial fresh-process AB/BA pairs."""

import argparse
import csv
import hashlib
import json
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def identity(root):
    paths = {root / "Cargo.toml", root / "Cargo.lock", root / "scripts/resource_report.py"}
    for directory in [root / "src", root / "tests/support", root / "tools/resource-profile"]:
        paths.update(p for p in directory.rglob("*")
                     if p.is_file() and "target" not in p.parts and p.suffix in {".rs", ".toml", ".lock"})
    digest, files = hashlib.sha256(), []
    for path in sorted(paths):
        name, content = str(path.relative_to(root)), path.read_bytes()
        digest.update(len(name.encode()).to_bytes(8, "big") + name.encode())
        digest.update(len(content).to_bytes(8, "big") + content)
        files.append({"path": name, "sha256": hashlib.sha256(content).hexdigest()})
    return {"sha256": digest.hexdigest(), "files": files}


def check_build(build):
    command = build["command"]
    manifest = Path(command[command.index("--manifest-path") + 1]).resolve(strict=True)
    root = manifest.parents[2]
    assert manifest == root / "tools/resource-profile/Cargo.toml", "unexpected worker manifest layout"
    assert "--release" in command and not build["worker"]["count_allocations"], "timing needs an uncounted release worker"
    assert identity(root) == build["source"], f"changed/missing sources: {root}"
    assert sha(Path(build["binary"])) == build["binary_sha256"], "changed/missing worker binary"
    assert build["worker"]["source_sha256"] == build["source"]["sha256"]
    return str(root)


def output_json(command, timeout):
    return [json.loads(line) for line in subprocess.run(
        command, check=True, capture_output=True, text=True, timeout=timeout).stdout.splitlines()]


def process_values(events, case, samples, minimum_batch_ns=0):
    operation, backend, size = case
    contexts = [e for e in events if e["kind"] == "context"]
    assert contexts == [dict(kind="context", operation=operation, backend=backend, bytes=size)]
    assert not any(e["kind"] == "allocations" for e in events), "counted timing output"
    rows = [e for e in events if e["kind"] == "timing"]
    assert [e["phase"] for e in rows[:2]] == ["cold", "second"]
    assert [e["sample"] for e in rows[:2]] == [0, 0]
    assert all(e["iterations"] > 0 and e["wall_ns"] >= 0 and e["cpu_ns"] >= 0 for e in rows)
    assert rows[0]["iterations"] == rows[1]["iterations"] == 1
    warm, previous_iterations = [], 0
    for row in rows[2:]:
        assert len(warm) < samples and row["sample"] == len(warm), "invalid batch sample index"
        assert row["iterations"] >= previous_iterations, "calibration iterations decreased"
        previous_iterations = row["iterations"]
        if row["phase"] == "calibration":
            assert minimum_batch_ns > 0 and row["cpu_ns"] < minimum_batch_ns, "invalid calibration batch"
        else:
            assert row["phase"] == "warm" and row["cpu_ns"] >= minimum_batch_ns, "short/unknown warmed batch"
            warm.append(row)
    assert len(warm) == samples, "missing warmed samples"
    assert all(e["wall_ns"] > 0 and e["cpu_ns"] > 0 for e in warm), "zero warmed batch timer"
    values = {f"warm_{clock}_ns": statistics.median(e[f"{clock}_ns"] / e["iterations"] for e in warm)
              for clock in ["cpu", "wall"]}
    assert all(v > 0 for v in values.values()), "zero process median"
    return dict(values, cold_wall_ns=rows[0]["wall_ns"], cold_cpu_ns=rows[0]["cpu_ns"],
                warm_samples=samples, calibration_batches=len(rows) - 2 - samples,
                warm_min_batch_wall_ns=min(e["wall_ns"] for e in warm),
                warm_min_batch_cpu_ns=min(e["cpu_ns"] for e in warm))


METRICS = ["warm_cpu_ns", "warm_wall_ns", "cold_wall_ns"]


def summaries(processes, cases, repeats):
    lookup = {(p["operation"], p["backend"], p["bytes"], p["repeat"], p["side"]): p for p in processes}
    assert len(lookup) == len(processes), "duplicate process result"
    pairs, rows, warnings = [], [], []
    for operation, backend, size in cases:
        prefix = dict(operation=operation, backend=backend, bytes=size)
        complete = all((operation, backend, size, repeat, side) in lookup
                       for repeat in range(repeats) for side in ["baseline", "candidate"])
        if not complete:
            continue  # Every missing process is retained separately in missing.csv.
        for metric in METRICS:
            values = {side: [lookup[operation, backend, size, repeat, side][metric]
                             for repeat in range(repeats)] for side in ["baseline", "candidate"]}
            decreases = []
            for repeat, (baseline, candidate) in enumerate(zip(values["baseline"], values["candidate"])):
                comparable = baseline > 0 and candidate > 0
                ratio = candidate / baseline if comparable else ""
                decrease = 100 * (1 - ratio) if comparable else ""
                if comparable:
                    decreases.append(decrease)
                pairs.append(dict(prefix, repeat=repeat, metric=metric, baseline=baseline,
                                  candidate=candidate, candidate_over_baseline=ratio, time_decrease_pct=decrease,
                                  comparable=comparable, comparison_reason="" if comparable else "zero single-call wall timer"))
            row = dict(prefix, metric=metric, processes_per_side=repeats,
                       comparable_pairs=len(decreases),
                       paired_comparison_reason="" if len(decreases) == repeats else "zero single-call wall timer; case percentage unavailable",
                       median_paired_time_decrease_pct=statistics.median(decreases) if len(decreases) == repeats else "")
            for side, samples in values.items():
                center = statistics.median(samples)
                spread = 100 * (max(samples) - min(samples)) / center if min(samples) > 0 else ""
                row.update({f"{side}_median": center, f"{side}_min": min(samples),
                            f"{side}_max": max(samples), f"{side}_spread_pct": spread,
                            f"{side}_spread_reason": "" if spread != "" else "zero single-call wall timer; relative spread unavailable"})
                if spread != "" and spread > 20:
                    warnings.append(dict(prefix, metric=metric, side=side, spread_pct=spread,
                                         reason="process range exceeds 20% of process median"))
            rows.append(row)
    return pairs, rows, warnings


def write_csv(path, rows, fields=None):
    if fields is None and not rows:
        return
    with path.open("x", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields or list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def run(args):
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    metadata = dict(schema=1, status="running", started_utc=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                    arguments=vars(args), command=sys.argv, script_sha256=sha(Path(__file__)),
                    host=dict(platform=platform.platform(), machine=platform.machine(), hostname=platform.node(), python=sys.version),
                    order="AB when (case index + repeat) is even, otherwise BA; one fresh sequential process per side",
                    statistics="warm process-CPU primary, warm wall and cold wall separately; paired process medians, no pooled sample independence",
                    limits="external quiet window required; no affinity/thermal control or confidence intervals; odd repeat counts balance across adjacent cases")
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    builds, records, cases, skipped, processes = {}, {}, [], [], []
    try:
        registries = {}
        for side in ["baseline", "candidate"]:
            path = Path(getattr(args, side)).resolve()
            data = path.read_bytes()
            (output / f"{side}-build.json").write_bytes(data)
            builds[side] = json.loads(data)
            records[side] = dict(path=str(path), sha256=hashlib.sha256(data).hexdigest())
        metadata["builds"] = records
        for side, build in builds.items():
            records[side]["source_root"] = check_build(build)
            assert output_json([build["binary"], "describe"], args.timeout) == [build["worker"]]
            registry = output_json([build["binary"], "list"], args.timeout)
            registries[side] = {row["backend"]: row for row in registry}
            assert len(registry) == len(registries[side]), "duplicate registry entry"
        for field in ["arch", "os", "pointer_bits", "ifma_threshold"]:
            assert builds["baseline"]["worker"][field] == builds["candidate"]["worker"][field], field
        for field in ["rustc", "cargo", "flags"]:
            assert builds["baseline"]["environment"][field] == builds["candidate"]["environment"][field], field
        configs = lambda b: [(c["sha256"], c["settings"]) for c in b["environment"]["cargo_configs"]]
        assert configs(builds["baseline"]) == configs(builds["candidate"]), "different Cargo configuration"
        for operation in ["encode", "decode"]:
            for backend in args.backends.split(","):
                assert all(backend in registry for registry in registries.values()), f"unknown backend: {backend}"
                caps = {side: registry[backend][f"max_{operation}_bytes"] for side, registry in registries.items()}
                for size in map(int, args.sizes.split(",")):
                    if size > min(caps.values()):
                        skipped.append(dict(operation=operation, backend=backend, bytes=size, baseline_cap=caps["baseline"],
                                            candidate_cap=caps["candidate"], reason="exceeds one or both worker registry caps"))
                    else:
                        cases.append((operation, backend, size))
        assert cases, "no applicable cases"
        metadata.update(cases=cases, expected_processes=len(cases) * args.repeats * 2)
        with (output / "runs.jsonl").open("x") as runs, (output / "raw.jsonl").open("x") as raw:
            for index, case in enumerate(cases):
                for repeat in range(args.repeats):
                    sides = ["baseline", "candidate"] if (index + repeat) % 2 == 0 else ["candidate", "baseline"]
                    for position, side in enumerate(sides):
                        context = dict(operation=case[0], backend=case[1], bytes=case[2], repeat=repeat,
                                       side=side, order="AB" if sides[0] == "baseline" else "BA", position=position)
                        command = [builds[side]["binary"], "timing", case[0], case[1], str(case[2]), str(args.samples)]
                        print(f"{len(processes) + 1}/{metadata['expected_processes']} {side} {case} repeat={repeat}", flush=True)
                        record = dict(context, command=command)
                        try:
                            result = subprocess.run(command, capture_output=True, text=True, timeout=args.timeout)
                            record.update(returncode=result.returncode, stdout=result.stdout, stderr=result.stderr)
                        except subprocess.TimeoutExpired as error:
                            decode = lambda value: value.decode(errors="replace") if isinstance(value, bytes) else value or ""
                            record.update(returncode=None, timeout=True, stdout=decode(error.stdout), stderr=decode(error.stderr))
                        runs.write(json.dumps(record) + "\n")
                        runs.flush()
                        assert record["returncode"] == 0, f"worker failed: {context}; see runs.jsonl"
                        events = [json.loads(line) for line in record["stdout"].splitlines()]
                        for event in events:
                            raw.write(json.dumps(dict(context, **event)) + "\n")
                        raw.flush()
                        floor = builds[side]["worker"].get("minimum_batch_ns", 0)
                        processes.append(dict(context, **process_values(events, case, args.samples, floor)))
        for side, build in builds.items():
            check_build(build)
            assert sha(Path(records[side]["path"])) == records[side]["sha256"], "build record changed during run"
        assert sha(Path(__file__)) == metadata["script_sha256"], "comparison script changed during run"
        metadata["status"] = "complete"
    except BaseException as error:
        metadata["error"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        final_status = "complete" if metadata["status"] == "complete" else "incomplete"
        metadata["status"] = "incomplete"
        try:
            pairs, summary, warnings = summaries(processes, cases, args.repeats)
            seen = {(p["operation"], p["backend"], p["bytes"], p["repeat"], p["side"]) for p in processes}
            missing = [dict(operation=op, backend=backend, bytes=size, repeat=repeat, side=side, reason="no validated process result")
                       for op, backend, size in cases for repeat in range(args.repeats) for side in ["baseline", "candidate"]
                       if (op, backend, size, repeat, side) not in seen]
            write_csv(output / "processes.csv", processes)
            write_csv(output / "pairs.csv", pairs)
            write_csv(output / "summary.csv", summary)
            write_csv(output / "skipped.csv", skipped, ["operation", "backend", "bytes", "baseline_cap", "candidate_cap", "reason"])
            write_csv(output / "missing.csv", missing, ["operation", "backend", "bytes", "repeat", "side", "reason"])
            (output / "warnings.json").write_text(json.dumps(warnings, indent=2) + "\n")
            metadata.update(status=final_status, complete_cases=len(summary) // len(METRICS), spread_warnings=len(warnings))
        except BaseException as error:
            metadata["error"] = f"{type(error).__name__}: {error}"
            raise
        finally:
            metadata.update(finished_utc=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), completed_processes=len(processes))
            (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")


def self_test():
    case = ("decode", "auto", 32)
    events = [dict(kind="context", operation=case[0], backend=case[1], bytes=case[2])]
    for phase, sample in [("cold", 0), ("second", 0), ("warm", 0), ("warm", 1), ("warm", 2)]:
        events.append(dict(kind="timing", phase=phase, sample=sample, iterations=1 if phase != "warm" else 10, wall_ns=200, cpu_ns=100))
    assert process_values(events, case, 3)["warm_cpu_ns"] == 10
    calibrated = [dict(e) for e in events]
    calibrated.insert(3, dict(kind="timing", phase="calibration", sample=0,
                              iterations=5, wall_ns=50, cpu_ns=50))
    assert process_values(calibrated, case, 3, 100)["calibration_batches"] == 1
    for invalid in [dict(calibrated[3], cpu_ns=100), dict(calibrated[3], sample=1),
                    dict(calibrated[3], iterations=11)]:
        bad = [dict(e) for e in calibrated]
        bad[3] = invalid
        try:
            process_values(bad, case, 3, 100)
        except AssertionError:
            pass
        else:
            raise AssertionError("malformed calibration accepted")
    processes = [dict(operation=case[0], backend=case[1], bytes=case[2], repeat=repeat, side=side,
                      **{metric: value for metric in METRICS})
                 for repeat, values in enumerate([(100, 80), (200, 100), (300, 300)])
                 for side, value in zip(["baseline", "candidate"], values)]
    pairs, rows, warnings = summaries(processes, [case], 3)
    assert len(pairs) == 9 and len(rows) == 3 and len(warnings) == 6
    assert abs(rows[0]["median_paired_time_decrease_pct"] - 20) < 1e-12
    assert rows[0]["baseline_median"] == 200 and rows[0]["candidate_median"] == 100
    assert summaries(processes[:-1], [case], 3) == ([], [], [])
    zero_events = [dict(e) for e in events]
    zero_events[1].update(wall_ns=0, cpu_ns=0)
    zero_events[2].update(wall_ns=0, cpu_ns=0)
    assert process_values(zero_events, case, 3)["cold_wall_ns"] == 0
    zero_processes = [dict(p, cold_wall_ns=0 if p["repeat"] < 2 else p["cold_wall_ns"]) for p in processes]
    zero_pairs, zero_rows, _ = summaries(zero_processes, [case], 3)
    cold = next(row for row in zero_rows if row["metric"] == "cold_wall_ns")
    assert cold["baseline_median"] == cold["candidate_median"] == 0
    assert cold["comparable_pairs"] == 1 and cold["median_paired_time_decrease_pct"] == ""
    assert cold["paired_comparison_reason"] and cold["baseline_spread_reason"]
    assert cold["baseline_spread_pct"] == cold["candidate_spread_pct"] == ""
    assert len(zero_pairs) == 9 and sum(not p["comparable"] for p in zero_pairs) == 2
    zero_events[3]["wall_ns"] = 0
    try:
        process_values(zero_events, case, 3)
    except AssertionError:
        pass
    else:
        raise AssertionError("zero warmed batch accepted")
    try:
        process_values(events[:-1], case, 3)
    except AssertionError:
        pass
    else:
        raise AssertionError("missing warmed sample accepted")
    print("compare_threshold pure self-test passed")


if __name__ == "__main__":
    if not __debug__:
        raise SystemExit("run without Python -O: validation uses assertions")
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["baseline", "candidate", "output"]:
        parser.add_argument(f"--{name}")
    parser.add_argument("--backends", default="auto,batched,dc,toom3,ntt,ntt-cached")
    parser.add_argument("--sizes", default="0,1,32,64,512,1024,2048,4096,8192,65536,1048576")
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--samples", type=int, default=9)
    parser.add_argument("--timeout", type=float, default=240)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
    else:
        assert args.baseline and args.candidate and args.output, "--baseline, --candidate and --output are required"
        sizes, backends = list(map(int, args.sizes.split(","))), args.backends.split(",")
        assert args.repeats > 0 and args.samples > 0 and args.timeout > 0
        assert len(sizes) == len(set(sizes)) and all(0 <= n <= 4194304 for n in sizes)
        assert len(backends) == len(set(backends)) and all(backends)
        run(args)

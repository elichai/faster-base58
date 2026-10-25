#!/usr/bin/env python3
"""Build and measure frozen public-mode workers with the existing validated probes."""

import argparse
import os
from pathlib import Path
import shlex
import subprocess
import sys

from resource_report import DEFAULT_SIZES

ROOT = Path(__file__).resolve().parents[1]


def run(arguments, environment):
    command = list(map(str, arguments))
    print(shlex.join(command), flush=True)
    subprocess.run(command, cwd=ROOT, env=environment, check=True)


def build(args, environment):
    output = args.output
    output.mkdir(parents=True, exist_ok=False)
    hardware_command = ["sysctl", "-n", "machdep.cpu.brand_string"] if sys.platform == "darwin" else ["lscpu"]
    hardware = subprocess.run(hardware_command, check=False, capture_output=True, text=True)
    (output / "hardware.txt").write_text(
        f"{shlex.join(hardware_command)}\nexit={hardware.returncode}\n{hardware.stdout}{hardware.stderr}")
    for cache in (False, True):
        for counted in (False, True):
            label = ("cache" if cache else "default") + ("-counted" if counted else "")
            command = [sys.executable, ROOT / "scripts/resource_report.py", "build",
                       "--target-dir", args.target_dir, "--output", output / f"{label}-build.json",
                       "--binary-output", output / "workers" / label]
            if cache:
                command += ["--cache"]
            if counted:
                command += ["--count-allocations"]
            run(command, environment)
            run([output / "workers" / label, "self-check"], environment)
    if args.clean_build_target:
        run(["cargo", "clean", "--manifest-path", ROOT / "tools/resource-profile/Cargo.toml",
             "--target-dir", args.target_dir], environment)
    for cache in (False, True):
        label = "cache" if cache else "default"
        command = [sys.executable, ROOT / "scripts/code_size.py", "--source", ROOT,
                   "--output", output / f"sizes-{label}", "--target", args.target,
                   "--cpu", args.cpu, "--backends", "empty,auto,no-alloc,bs58",
                   "--directions", "both", "--kinds", args.kind, "--label", args.label,
                   "--strip-tool", args.strip_tool]
        if cache:
            command += ["--cache"]
        run(command, environment)
        if args.clean_build_target:
            run(["cargo", "clean", "--manifest-path", output / f"sizes-{label}/probe/Cargo.toml",
                 "--target-dir", output / f"sizes-{label}/target"], environment)


def measure(args, environment):
    run([sys.executable, ROOT / "scripts/compare_threshold.py",
         "--baseline", args.output / "default-build.json",
         "--candidate", args.output / "cache-build.json",
         "--output", args.output / "timing", "--backends", "auto,no-alloc,bs58",
         "--sizes", args.sizes, "--repeats", args.repeats, "--samples", args.samples,
         "--timeout", args.timeout], environment)


def allocations(args, environment):
    for cache in (False, True):
        label = "cache" if cache else "default"
        run([sys.executable, ROOT / "scripts/resource_report.py", "run",
             "--build-metadata", args.output / f"{label}-counted-build.json",
             "--output", args.output / f"allocations-{label}", "--mode", "allocations",
             "--backends", "auto,no-alloc,bs58", "--sizes", args.sizes,
             "--repeats", "1", "--timeout", args.timeout], environment)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("build", "measure", "allocations"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--target-dir", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--cpu", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--strip-tool", required=True)
    parser.add_argument("--kind", choices=("bin", "cdylib"), default="bin")
    parser.add_argument("--sizes", default=",".join(map(str, DEFAULT_SIZES)))
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=300)
    parser.add_argument("--clean-build-target", action="store_true",
                        help="clear only these reproducible Cargo targets after retaining workers/artifacts")
    args = parser.parse_args()
    args.output = args.output.resolve()
    args.target_dir = args.target_dir.resolve()
    assert args.repeats > 0 and args.samples > 0 and args.timeout > 0
    environment = dict(os.environ)
    environment["PATH"] = str(Path.home() / ".cargo/bin") + os.pathsep + environment["PATH"]
    environment.pop("CARGO_ENCODED_RUSTFLAGS", None)
    environment["RUSTFLAGS"] = "" if args.cpu == "generic" else f"-C target-cpu={args.cpu}"
    environment["CARGO_BUILD_JOBS"] = "2"
    {"build": build, "measure": measure, "allocations": allocations}[args.phase](args, environment)


if __name__ == "__main__":
    main()

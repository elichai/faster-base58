#!/usr/bin/env python3
"""Run Kani commands without leaving Z3 alive after its verifier parent exits.

Usage: python3 scripts/kani_guard.py -- cargo kani ...
Records and the per-command PATH shim are retained in a new temporary directory.
"""
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time

from process_supervisor import deferred_cancellation, install_signal_handlers, owned_process


def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


def solver(arguments):
    real = os.environ["BASE58_KANI_REAL_Z3"]
    owner = int(os.environ["BASE58_KANI_GUARD_OWNER"])
    parent = os.getppid()
    if parent == 1 or not alive(owner):
        return 125
    if shutil.disk_usage(Path.cwd()).free < 2 * 1024**3:
        print("below 2 GiB free-space floor; Z3 solve not started", file=sys.stderr)
        return 125
    # Inherit the process group, so verifier group termination also reaches Z3.
    record = Path(os.environ["BASE58_KANI_GUARD_RECORDS"]) / f"solver-{os.getpid()}-{time.monotonic_ns()}.jsonl"
    with record.open("x") as stream:
        limit = int(os.environ.get("BASE58_KANI_Z3_TIMEOUT_SECONDS", "180"))
        if limit <= 0:
            raise SystemExit("BASE58_KANI_Z3_TIMEOUT_SECONDS must be positive")
        child = None
        reason = "signal"
        try:
            with deferred_cancellation():
                child = subprocess.Popen([real, *arguments, f"-T:{limit}"])
                stream.write(json.dumps({"parent": parent, "owner": owner, "solver": child.pid,
                                         "arguments": arguments, "hard_timeout_seconds": limit}) + "\n")
                stream.flush()
            while child.poll() is None:
                if os.getppid() != parent or not alive(owner):
                    reason = "parent_exited"
                    return 125
                time.sleep(0.05)
            reason = "completed"
            return child.returncode
        finally:
            if child is not None:
                with deferred_cancellation():
                    if child.poll() is None:
                        child.terminate()
                        try:
                            child.wait(timeout=2)
                        except subprocess.TimeoutExpired:
                            child.kill()
                            child.wait()
                    stream.write(json.dumps({"reason": reason, "exit_code": child.returncode}) + "\n")


def command(arguments):
    if arguments[:1] == ["--"]:
        arguments = arguments[1:]
    if not arguments:
        raise SystemExit("expected a command after --")
    real = shutil.which(os.environ.get("BASE58_KANI_REAL_Z3") or "z3")
    if real is None:
        raise SystemExit("z3 is not on PATH")
    real = str(Path(real).resolve(strict=True))
    if shutil.disk_usage(Path.cwd()).free < 2 * 1024**3:
        raise SystemExit("below 2 GiB free-space floor; proof command not started")
    records = Path(tempfile.mkdtemp(prefix="base58-kani-guard-"))
    shim = records / "z3"
    shim.write_text('#!/bin/sh\nexec "$BASE58_KANI_PYTHON" "$BASE58_KANI_GUARD_SCRIPT" --solver "$@"\n')
    shim.chmod(0o700)
    env = os.environ.copy()
    env.update(BASE58_KANI_REAL_Z3=real, BASE58_KANI_GUARD_OWNER=str(os.getpid()),
               BASE58_KANI_GUARD_RECORDS=str(records), BASE58_KANI_PYTHON=sys.executable,
               BASE58_KANI_GUARD_SCRIPT=str(Path(__file__).resolve()),
               PATH=str(records) + os.pathsep + env.get("PATH", ""))
    with owned_process(arguments, env=env, grace_seconds=0.2) as child:
        (records / "invocation.json").write_text(json.dumps({
            "arguments": arguments, "cwd": str(Path.cwd()), "owner": os.getpid(),
            "command_group": child.pid, "real_z3": real,
        }, indent=2) + "\n")
        print(f"Kani guard records: {records}", file=sys.stderr, flush=True)
        return child.wait()


def main():
    install_signal_handlers()
    if sys.argv[1:2] == ["--solver"]:
        code = solver(sys.argv[2:])
    else:
        code = command(sys.argv[1:])
    return code if code >= 0 else 128 - code


if __name__ == "__main__":
    raise SystemExit(main())

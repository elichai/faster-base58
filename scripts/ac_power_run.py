#!/usr/bin/env python3
"""Run one macOS benchmark phase with sampled AC checks and a fresh power log."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time


def query(kind):
    result = subprocess.run(["pmset", "-g", kind], capture_output=True, text=True, timeout=10)
    return dict(returncode=result.returncode, stdout=result.stdout, stderr=result.stderr)


def on_ac(result):
    lines = result["stdout"].splitlines()
    return result["returncode"] == 0 and bool(lines) and lines[0] == "Now drawing from 'AC Power'"


def stop(child):
    # start_new_session makes the child's PID our owned process-group ID.
    # The leader may already have exited while same-group descendants remain.
    for action in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(child.pid, action)
        except ProcessLookupError:
            pass
        if action == signal.SIGTERM:
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                pass
    child.wait(timeout=5)


def group_exists(child):
    try:
        os.killpg(child.pid, 0)
        return True
    except ProcessLookupError:
        return False


def run(command, output, interval=30, probe=lambda: query("batt"), settings=None):
    child = None
    with output.open("x") as stream:
        def record(kind, **fields):
            stream.write(json.dumps(dict(kind=kind, utc=datetime.now(timezone.utc).isoformat(),
                                         monotonic_ns=time.monotonic_ns(), **fields)) + "\n")
            stream.flush()

        def sample(phase):
            result = probe()
            accepted = on_ac(result)
            record("power", phase=phase, ac=accepted, **result)
            return accepted

        def cancel(signum, _frame):
            raise SystemExit(128 + signum)

        signals = (signal.SIGTERM, signal.SIGHUP, signal.SIGINT)
        previous = {sig: signal.getsignal(sig) for sig in signals}

        def cleanup():
            for sig in signals:
                signal.signal(sig, signal.SIG_IGN)
            if child is not None:
                stop(child)

        try:
            for sig in (signal.SIGTERM, signal.SIGHUP):
                signal.signal(sig, cancel)
            record("start", command=command, interval_seconds=interval,
                   script_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                   settings=settings() if settings else None)
            if not sample("before"):
                record("finish", accepted=False, command_exit=None, reason="initial AC check failed")
                return 125
            child = subprocess.Popen(command, stdin=subprocess.DEVNULL, start_new_session=True)
            record("child", pid=child.pid)
            while True:
                try:
                    code = child.wait(timeout=interval)
                except subprocess.TimeoutExpired:
                    if not sample("sample"):
                        cleanup()
                        record("finish", accepted=False, command_exit=child.returncode,
                               reason="AC power lost; owned process group stopped")
                        return 125
                    continue
                ac = sample("after")
                descendants = group_exists(child)
                accepted = ac and code == 0 and not descendants
                if not accepted:
                    cleanup()
                reason = "" if accepted else (
                    "command exited with same-group descendants" if descendants
                    else "command or final AC check failed")
                record("finish", accepted=accepted, command_exit=code, reason=reason,
                       settings=settings() if settings else None)
                return code if accepted else 125
        except BaseException as error:
            cleanup_error = ""
            try:
                cleanup()
            except BaseException as failure:
                cleanup_error = f"{type(failure).__name__}: {failure}"
            record("error", error=f"{type(error).__name__}: {error}",
                   cleanup_error=cleanup_error)
            record("finish", accepted=False,
                   command_exit=child.returncode if child else None,
                   reason="guard exception")
            raise
        finally:
            for sig, handler in previous.items():
                signal.signal(sig, handler)


def self_test():
    ac = dict(returncode=0, stdout="Now drawing from 'AC Power'\n", stderr="")
    battery = dict(ac, stdout="Now drawing from 'Battery Power'\n")
    assert on_ac(ac) and not on_ac(battery)
    assert not on_ac(dict(ac, returncode=1)) and not on_ac(dict(ac, stdout=""))
    root = Path(tempfile.mkdtemp(prefix="base58-power-self-test-"))
    print(f"AC guard self-test evidence: {root}", flush=True)
    command = [sys.executable, "-c", "pass"]
    good = root / "ac.jsonl"
    unsupported = dict(returncode=1, stdout="", stderr="thermal query unsupported")
    assert run(command, good, 0.01, probe=lambda: ac,
               settings=lambda: dict(thermal=unsupported)) == 0
    assert json.loads(good.read_text().splitlines()[-1])["accepted"]
    assert json.loads(good.read_text().splitlines()[0])["settings"]["thermal"] == unsupported
    assert run(command, root / "battery.jsonl", 0.01, probe=lambda: battery) == 125
    sleeper = root / "sleeper.py"
    sleeper.write_text("import time\ntime.sleep(60)\n")
    states = iter([ac, battery])
    lost = root / "lost.jsonl"
    assert run([sys.executable, str(sleeper)], lost, 0.01, probe=lambda: next(states)) == 125
    assert json.loads(lost.read_text().splitlines()[-1])["command_exit"] != 0
    try:
        run(command, good, probe=lambda: ac)
    except FileExistsError:
        pass
    else:
        raise AssertionError("existing power log was overwritten")

    # An unrelated session must survive every cleanup below.
    sentinel = subprocess.Popen([sys.executable, str(sleeper)], start_new_session=True)
    try:
        timeout_log = root / "probe-timeout.jsonl"
        answers = iter([ac, subprocess.TimeoutExpired(["pmset", "-g", "batt"], 10), ac, ac])

        def timeout_probe():
            answer = next(answers)
            if isinstance(answer, BaseException):
                raise answer
            return answer

        try:
            run(command, timeout_log, 1, probe=timeout_probe)
        except subprocess.TimeoutExpired:
            pass
        else:
            raise AssertionError("a failed final power query was accepted")
        assert not json.loads(timeout_log.read_text().splitlines()[-1])["accepted"]

        failed = root / "failed.jsonl"
        assert run([sys.executable, "-c", "raise SystemExit(17)"], failed,
                   0.01, probe=lambda: ac) == 125
        finish = json.loads(failed.read_text().splitlines()[-1])
        assert not finish["accepted"] and finish["command_exit"] == 17

        grandchild = root / "grandchild.py"
        grandchild.write_text(
            "import os, signal, sys, time\nfrom pathlib import Path\n"
            "signal.signal(signal.SIGTERM, signal.SIG_IGN)\n"
            "beat = Path(sys.argv[1] + '.heartbeat')\nbeat.write_text('ready')\n"
            "Path(sys.argv[1]).write_text(str(os.getpid()))\n"
            "deadline = time.monotonic() + 60\n"
            "while time.monotonic() < deadline:\n"
            "    beat.write_text(str(time.monotonic_ns()))\n    time.sleep(0.01)\n")
        leader = root / "leader.py"
        leader.write_text(
            "import subprocess, sys, time\nfrom pathlib import Path\n"
            "subprocess.Popen([sys.executable, sys.argv[1], sys.argv[2]])\n"
            "deadline = time.monotonic() + 5\n"
            "while not Path(sys.argv[2]).exists():\n"
            "    assert time.monotonic() < deadline\n    time.sleep(0.01)\n"
            "raise SystemExit(int(sys.argv[3]))\n")

        def wait_file(path):
            deadline = time.monotonic() + 5
            while not path.exists():
                assert time.monotonic() < deadline, path
                time.sleep(0.01)
            return int(path.read_text())

        def assert_stopped(pid, marker):
            # Check owned work, not PID disappearance: a dead orphan may be a zombie.
            beat = Path(str(marker) + ".heartbeat")
            time.sleep(0.02)
            value = beat.read_text()
            time.sleep(0.05)
            assert beat.read_text() == value, f"owned test process {pid} still updates its heartbeat"

        for exit_code in (0, 17):
            pid_file = root / f"grandchild-{exit_code}.pid"
            log = root / f"descendant-{exit_code}.jsonl"
            assert run([sys.executable, str(leader), str(grandchild), str(pid_file), str(exit_code)],
                       log, 0.01, probe=lambda: ac) == 125
            assert_stopped(wait_file(pid_file), pid_file)
            assert not json.loads(log.read_text().splitlines()[-1])["accepted"]

        target = root / "cancel-target.py"
        target.write_text("import os, sys, time\nfrom pathlib import Path\n"
                          "beat = Path(sys.argv[1] + '.heartbeat')\nbeat.write_text('ready')\n"
                          "Path(sys.argv[1]).write_text(str(os.getpid()))\n"
                          "deadline = time.monotonic() + 60\n"
                          "while time.monotonic() < deadline:\n"
                          "    beat.write_text(str(time.monotonic_ns()))\n    time.sleep(0.01)\n")
        wrapper = root / "cancel-guard.py"
        wrapper.write_text(
            "import importlib.util, sys\nfrom pathlib import Path\n"
            "spec = importlib.util.spec_from_file_location('guard', sys.argv[1])\n"
            "guard = importlib.util.module_from_spec(spec)\nspec.loader.exec_module(guard)\n"
            "ac = dict(returncode=0, stdout=\"Now drawing from 'AC Power'\\n\", stderr='')\n"
            "raise SystemExit(guard.run([sys.executable, sys.argv[2], sys.argv[3]],\n"
            "    Path(sys.argv[4]), 0.01, probe=lambda: ac))\n")
        for sig in (signal.SIGTERM, signal.SIGHUP):
            pid_file = root / f"cancel-{sig}.pid"
            log = root / f"cancel-{sig}.jsonl"
            guard = subprocess.Popen([sys.executable, "-B", str(wrapper), str(Path(__file__).resolve()),
                                      str(target), str(pid_file), str(log)], start_new_session=True)
            try:
                pid = wait_file(pid_file)
                guard.send_signal(sig)
                assert guard.wait(timeout=10) == 128 + sig
                assert_stopped(pid, pid_file)
                assert not json.loads(log.read_text().splitlines()[-1])["accepted"]
            finally:
                stop(guard)
        assert sentinel.poll() is None, "unrelated process was stopped"
    finally:
        stop(sentinel)
    (root / "self-test.json").write_text(json.dumps(dict(
        passed=True, script_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        unsupported_thermal_recorded=True, unrelated_sentinel_survived=True), indent=2) + "\n")
    print(f"AC power guard self-test passed; retained evidence: {root}")


if __name__ == "__main__":
    if not __debug__:
        raise SystemExit("run without Python -O: self-checks use assertions")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--interval", type=float, default=30)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.self_test:
        self_test()
    else:
        command = args.command[1:] if args.command[:1] == ["--"] else args.command
        if not args.output or not command or not math.isfinite(args.interval) or not 0 < args.interval <= 60:
            parser.error("provide a fresh --output, a command after --, and an interval in (0, 60]")
        if sys.platform != "darwin":
            parser.error("AC telemetry requires macOS pmset")
        sys.exit(run(command, args.output, args.interval,
                     settings=lambda: dict(custom=query("custom"), thermal=query("therm"))))

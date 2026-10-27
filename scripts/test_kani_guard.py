#!/usr/bin/env python3
"""Controlled ownership/forwarding checks; all temporary records are retained."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import kani_guard
import process_supervisor
from process_supervisor import owned_process


GUARD = Path(__file__).with_name("kani_guard.py").resolve()


class SolverGuardTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="base58-kani-guard-test-"))
        self.ready = self.root / "ready"
        self.stopped = self.root / "stopped"
        self.fake = self.root / "fake_z3.py"
        self.fake.write_text(
            f"#!{sys.executable}\n"
            "import json, os, signal, sys, time\nfrom pathlib import Path\n"
            "if '-in' in sys.argv:\n"
            "    print(json.dumps({'arguments':sys.argv[1:], 'input':sys.stdin.read()}))\n"
            "    raise SystemExit(7)\n"
            "def stop(*_):\n"
            "    Path(os.environ['GUARD_TEST_STOPPED']).write_text('stopped')\n"
            "    raise SystemExit(0)\n"
            "signal.signal(signal.SIGTERM, stop)\n"
            "Path(os.environ['GUARD_TEST_READY']).write_text('ready')\n"
            "time.sleep(30)\n"
        )
        self.fake.chmod(0o700)
        self.env = os.environ.copy()
        self.env.update(BASE58_KANI_REAL_Z3=str(self.fake),
                        BASE58_KANI_GUARD_OWNER=str(os.getpid()),
                        BASE58_KANI_GUARD_RECORDS=str(self.root),
                        GUARD_TEST_READY=str(self.ready), GUARD_TEST_STOPPED=str(self.stopped))

    def wait_for(self, path):
        deadline = time.monotonic() + 5
        while not path.exists():
            self.assertLess(time.monotonic(), deadline, f"missing {path}")
            time.sleep(0.01)

    def records(self):
        return [json.loads(line) for path in self.root.glob("solver-*.jsonl")
                for line in path.read_text().splitlines()]

    def test_forwarding_and_exit_status(self):
        driver = self.root / "forward.py"
        driver.write_text(
            "import subprocess\n"
            "result=subprocess.run(['z3','-in','rewriter.som=true'], input='formula', text=True)\n"
            "raise SystemExit(result.returncode)\n"
        )
        for real in [str(self.fake), self.fake.name]:
            env = dict(self.env, BASE58_KANI_REAL_Z3=real,
                       PATH=str(self.root) + os.pathsep + self.env.get("PATH", ""))
            result = subprocess.run([sys.executable, str(GUARD), "--", sys.executable, str(driver)],
                                    env=env, text=True, capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 7, result.stderr)
            self.assertEqual(json.loads(result.stdout), {
                "arguments": ["-in", "rewriter.som=true", "-T:180"], "input": "formula"})

    def test_solver_is_reaped_when_its_parent_exits(self):
        driver = self.root / "parent.py"
        driver.write_text(
            "import os, subprocess, sys, time\nfrom pathlib import Path\n"
            "subprocess.Popen([sys.executable,sys.argv[1],'--solver'])\n"
            "deadline=time.monotonic()+5\n"
            "while not Path(os.environ['GUARD_TEST_READY']).exists():\n"
            "    assert time.monotonic()<deadline\n    time.sleep(0.01)\n"
        )
        with owned_process([sys.executable, str(driver), str(GUARD)], env=self.env) as parent:
            self.assertEqual(parent.wait(timeout=5), 0)
            self.wait_for(self.stopped)
            deadline = time.monotonic() + 5
            while not any(record.get("reason") == "parent_exited" for record in self.records()):
                self.assertLess(time.monotonic(), deadline)
                time.sleep(0.01)
        self.assertTrue(any(record.get("exit_code") == 0 for record in self.records()))

    def test_wrapper_cancellation_reaps_its_solver(self):
        with owned_process([sys.executable, str(GUARD), "--solver"], env=self.env) as wrapper:
            self.wait_for(self.ready)
            wrapper.terminate()
            self.assertEqual(wrapper.wait(timeout=5), 143)
            self.assertTrue(self.stopped.exists())
        self.assertTrue(any(record.get("reason") == "signal" for record in self.records()))

    def test_low_space_does_not_start_a_solver(self):
        with patch.dict(os.environ, self.env):
            with patch("kani_guard.shutil.disk_usage", return_value=SimpleNamespace(free=0)):
                self.assertEqual(kani_guard.solver([]), 125)
        self.assertFalse(self.ready.exists())
        self.assertEqual(self.records(), [])


class CancellationTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="base58-kani-cancellation-test-"))
        self.handlers = {sig: signal.getsignal(sig) for sig in [signal.SIGTERM, signal.SIGINT]}
        process_supervisor.install_signal_handlers()
        self.events = []
        self.child = SimpleNamespace(pid=12345, returncode=None, poll=lambda: self.child.returncode,
                                     terminate=self.terminate, wait=self.wait, kill=self.kill)
        self.env = dict(BASE58_KANI_REAL_Z3="fixture-z3", BASE58_KANI_GUARD_OWNER="42",
                        BASE58_KANI_GUARD_RECORDS=str(self.root))

    def tearDown(self):
        for sig, handler in self.handlers.items():
            signal.signal(sig, handler)

    def terminate(self):
        self.events.append("terminate")

    def wait(self, timeout=None):
        self.events.append("reaped")
        self.child.returncode = 0 if self.child.returncode is None else self.child.returncode
        return self.child.returncode

    def kill(self):
        self.events.append("kill")
        self.child.returncode = -9

    def records(self):
        return [json.loads(line) for path in self.root.glob("solver-*.jsonl")
                for line in path.read_text().splitlines()]

    def test_solver_launch_cancellation_reaps_and_records(self):
        def launch(*_args, **_kwargs):
            self.events.append("spawned")
            signal.raise_signal(signal.SIGTERM)
            self.events.append("returned")
            return self.child

        with (
            patch.dict(os.environ, self.env),
            patch("kani_guard.os.getppid", return_value=101),
            patch("kani_guard.alive", return_value=True),
            patch("kani_guard.shutil.disk_usage", return_value=SimpleNamespace(free=8 * 1024**3)),
            patch("kani_guard.subprocess.Popen", side_effect=launch),
        ):
            with self.assertRaises(SystemExit) as stopped:
                kani_guard.solver([])
        self.assertEqual(stopped.exception.code, 143)
        self.assertEqual(self.events, ["spawned", "returned", "terminate", "reaped"])
        self.assertEqual(len(self.records()), 2)
        self.assertEqual(self.records()[0]["solver"], self.child.pid)
        self.assertEqual(self.records()[1], {"reason": "signal", "exit_code": 0})
        self.assertIsNone(process_supervisor._deferred)

    def test_solver_cleanup_cancellation_keeps_kill_and_record(self):
        def terminate():
            self.events.append("terminate")
            signal.raise_signal(signal.SIGINT)
            signal.raise_signal(signal.SIGTERM)

        def wait(timeout=None):
            if timeout is not None:
                self.events.append("wait-timeout")
                raise subprocess.TimeoutExpired("fixture", timeout)
            self.events.append("reaped")
            return self.child.returncode

        self.child.terminate, self.child.wait = terminate, wait
        with (
            patch.dict(os.environ, self.env),
            patch("kani_guard.os.getppid", return_value=101),
            patch("kani_guard.alive", side_effect=[True, False]),
            patch("kani_guard.shutil.disk_usage", return_value=SimpleNamespace(free=8 * 1024**3)),
            patch("kani_guard.subprocess.Popen", return_value=self.child),
        ):
            with self.assertRaises(SystemExit) as stopped:
                kani_guard.solver([])
        self.assertEqual(stopped.exception.code, 130)
        self.assertEqual(self.events, ["terminate", "wait-timeout", "kill", "reaped"])
        self.assertEqual(self.records()[-1], {"reason": "parent_exited", "exit_code": -9})
        self.assertIsNone(process_supervisor._deferred)
        self.assertIs(signal.getsignal(signal.SIGINT), process_supervisor._interrupted)


if __name__ == "__main__":
    unittest.main()

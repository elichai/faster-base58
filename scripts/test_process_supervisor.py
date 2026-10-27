#!/usr/bin/env python3
"""Behavior checks with owned children; fixtures and records are retained."""
from pathlib import Path
import select
import signal
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import process_supervisor
from process_supervisor import owned_process


class OwnershipTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="base58-supervisor-test-"))
        self.sleep = self.root / "sleep.py"
        self.sleep.write_text("import time\ntime.sleep(30)\n")

    def test_exit_status_is_preserved(self):
        program = self.root / "exit.py"
        program.write_text("raise SystemExit(7)\n")
        with owned_process([sys.executable, str(program)]) as child:
            self.assertEqual(child.wait(timeout=5), 7)
        self.assertEqual(child.returncode, 7)

    def test_timeout_reaps_leader_without_touching_other_group(self):
        control = subprocess.Popen([sys.executable, str(self.sleep)], start_new_session=True)
        try:
            with self.assertRaises(subprocess.TimeoutExpired):
                with owned_process([sys.executable, str(self.sleep)], grace_seconds=0.2) as child:
                    child.wait(timeout=0.05)
            self.assertIsNotNone(child.returncode)
            self.assertIsNone(control.poll())
        finally:
            control.terminate()
            control.wait(timeout=5)

    def test_successful_leader_does_not_leave_its_child(self):
        ready = self.root / "ready"
        stopped = self.root / "stopped"
        worker = self.root / "worker.py"
        worker.write_text(
            "from pathlib import Path\nimport os, signal, sys, time\n"
            "def stop(*_):\n    Path(sys.argv[2]).write_text('stopped')\n    raise SystemExit(0)\n"
            "signal.signal(signal.SIGTERM, stop)\nPath(sys.argv[1]).write_text(str(os.getpid()))\n"
            "time.sleep(30)\n"
        )
        leader = self.root / "leader.py"
        leader.write_text(
            "from pathlib import Path\nimport subprocess, sys, time\n"
            "subprocess.Popen([sys.executable, sys.argv[1], sys.argv[2], sys.argv[3]])\n"
            "deadline = time.monotonic() + 5\n"
            "while not Path(sys.argv[2]).exists():\n"
            "    assert time.monotonic() < deadline\n    time.sleep(0.01)\n"
        )
        with owned_process([sys.executable, str(leader), str(worker), str(ready), str(stopped)],
                           grace_seconds=0.3) as child:
            self.assertEqual(child.wait(timeout=5), 0)
        self.assertTrue(stopped.exists(), "owned child did not receive termination")

    def test_term_resistant_descendant_is_killed_after_leader_exits(self):
        worker = self.root / "ignore_term.py"
        worker.write_text(
            "import signal, time\n"
            "signal.signal(signal.SIGTERM, signal.SIG_IGN)\n"
            "print('ready', flush=True)\ntime.sleep(30)\n"
        )
        leader = self.root / "detached_leader.py"
        leader.write_text(
            "import subprocess, sys\n"
            "subprocess.Popen([sys.executable, sys.argv[1]])\n"
        )
        with owned_process([sys.executable, str(leader), str(worker)],
                           grace_seconds=0.1, stdout=subprocess.PIPE) as child:
            self.assertTrue(select.select([child.stdout], [], [], 5)[0])
            self.assertEqual(child.stdout.readline(), b"ready\n")
            self.assertEqual(child.wait(timeout=5), 0)
        self.assertTrue(select.select([child.stdout], [], [], 5)[0])
        self.assertEqual(child.stdout.read(), b"", "owned descendant still holds its pipe open")
        child.stdout.close()


class CancellationTests(unittest.TestCase):
    def setUp(self):
        self.handlers = {sig: signal.getsignal(sig) for sig in [signal.SIGTERM, signal.SIGINT]}
        process_supervisor.install_signal_handlers()
        self.events = []
        self.child = SimpleNamespace(pid=12345, returncode=7, wait=self.wait)

    def tearDown(self):
        for sig, handler in self.handlers.items():
            signal.signal(sig, handler)

    def wait(self, timeout=None):
        self.events.append("reaped")
        return self.child.returncode

    def signal_group(self, group, signum):
        self.assertEqual(group, self.child.pid)
        self.events.append(signum)
        return True

    def test_launch_cancellation_waits_for_ownership(self):
        def launch(*_args, **_kwargs):
            self.events.append("spawned")
            signal.raise_signal(signal.SIGTERM)
            self.events.append("returned")
            return self.child

        with patch("process_supervisor.subprocess.Popen", side_effect=launch):
            with patch("process_supervisor._signal_group", side_effect=self.signal_group):
                with patch("process_supervisor.time.sleep"):
                    with self.assertRaises(SystemExit) as stopped:
                        with owned_process(["fixture"], grace_seconds=0):
                            self.fail("pending launch cancellation was not delivered")
        self.assertEqual(stopped.exception.code, 143)
        self.assertEqual(self.events, ["spawned", "returned", signal.SIGTERM, "reaped",
                                      signal.SIGKILL, "reaped"])
        self.assertIsNone(process_supervisor._deferred)
        self.assertIs(signal.getsignal(signal.SIGTERM), process_supervisor._interrupted)

    def test_repeated_cleanup_cancellation_does_not_skip_kill_or_reap(self):
        def cancel(_seconds):
            signal.raise_signal(signal.SIGINT)
            signal.raise_signal(signal.SIGTERM)
            self.events.append("cleanup-continued")

        with patch("process_supervisor.subprocess.Popen", return_value=self.child):
            with patch("process_supervisor._signal_group", side_effect=self.signal_group):
                with patch("process_supervisor.time.sleep", side_effect=cancel):
                    with self.assertRaises(SystemExit) as stopped:
                        with owned_process(["fixture"], grace_seconds=0):
                            self.events.append("body")
        self.assertEqual(stopped.exception.code, 130)
        self.assertEqual(self.events, ["body", signal.SIGTERM, "reaped", "cleanup-continued",
                                      signal.SIGKILL, "reaped"])
        self.assertIsNone(process_supervisor._deferred)

    def test_nested_deferral_restores_state_on_exception(self):
        with self.assertRaisesRegex(ValueError, "body failed"):
            with process_supervisor.deferred_cancellation():
                outer = process_supervisor._deferred
                with process_supervisor.deferred_cancellation():
                    raise ValueError("body failed")
        self.assertIsNone(process_supervisor._deferred)
        self.assertEqual(outer, [])
        self.assertIs(signal.getsignal(signal.SIGINT), process_supervisor._interrupted)

    def test_nested_cancellation_is_delivered_only_after_outer_section(self):
        with self.assertRaises(SystemExit) as stopped:
            with process_supervisor.deferred_cancellation():
                outer = process_supervisor._deferred
                with process_supervisor.deferred_cancellation():
                    self.assertIs(process_supervisor._deferred, outer)
                    signal.raise_signal(signal.SIGINT)
                signal.raise_signal(signal.SIGTERM)
                self.assertIs(process_supervisor._deferred, outer)
                self.assertEqual(outer, [signal.SIGINT])
                self.events.append("outer-continued")
        self.assertEqual(stopped.exception.code, 130)
        self.assertEqual(self.events, ["outer-continued"])
        self.assertIsNone(process_supervisor._deferred)

    def test_other_thread_rejection_preserves_outer_state(self):
        with process_supervisor.deferred_cancellation():
            outer = process_supervisor._deferred
            with patch("process_supervisor.threading.current_thread", return_value=object()):
                with self.assertRaisesRegex(RuntimeError, "requires the main thread"):
                    with process_supervisor.deferred_cancellation():
                        self.fail("non-main thread entered a critical section")
            self.assertIs(process_supervisor._deferred, outer)
        self.assertIsNone(process_supervisor._deferred)

    def test_launch_failure_preserves_exception_without_signalling_a_group(self):
        with patch("process_supervisor.subprocess.Popen", side_effect=OSError("exec failed")):
            with patch("process_supervisor._signal_group") as groups:
                with self.assertRaisesRegex(OSError, "exec failed"):
                    with owned_process(["fixture"]):
                        self.fail("launch failure entered the body")
        groups.assert_not_called()
        self.assertIsNone(process_supervisor._deferred)


if __name__ == "__main__":
    unittest.main()

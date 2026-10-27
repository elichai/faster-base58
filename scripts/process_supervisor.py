"""POSIX subprocess ownership for local verification drivers."""
from contextlib import contextmanager
import os
import signal
import subprocess
import threading
import time


_deferred = None


def _interrupted(signum, _frame):
    if _deferred is not None:
        if not _deferred:
            _deferred.append(signum)
        return
    raise SystemExit(128 + signum)


def install_signal_handlers():
    """Call from a CLI main so cancellation unwinds owned_process contexts."""
    for signum in [signal.SIGTERM, signal.SIGINT]:
        signal.signal(signum, _interrupted)


@contextmanager
def deferred_cancellation():
    """Defer our installed handler in a main-thread critical section.

    No signal masks or handlers are changed. Nested sections share the outer
    pending list. The first cancellation across the outer section wins, and
    only that outer section delivers it on exit.
    """
    if threading.current_thread() is not threading.main_thread():
        raise RuntimeError("cancellation deferral requires the main thread")
    global _deferred
    if _deferred is not None:
        yield
        return
    pending = []
    _deferred = pending
    try:
        yield
    finally:
        _deferred = None
        if pending:
            _interrupted(pending[0], None)


def _signal_group(group, signum):
    try:
        os.killpg(group, signum)
        return True
    except ProcessLookupError:
        return False


@contextmanager
def owned_process(arguments, *, grace_seconds=2, **kwargs):
    """Yield ordinary Popen; always terminate/reap only its private process group.

    Callers retain their usual wait(timeout=...), logs, and TimeoutExpired handling.
    The group is cleaned after success too: its leader may have left child solvers.
    CLI callers install_signal_handlers() on the main thread to defer cancellation
    while launching and cleaning up; other signal handlers are left untouched.
    """
    child = None
    try:
        with deferred_cancellation():
            child = subprocess.Popen(arguments, start_new_session=True, **kwargs)
        yield child
    finally:
        if child is not None:
            with deferred_cancellation():
                try:
                    if _signal_group(child.pid, signal.SIGTERM):
                        deadline = time.monotonic() + grace_seconds
                        try:
                            child.wait(timeout=grace_seconds)
                        except subprocess.TimeoutExpired:
                            pass
                        time.sleep(max(0, deadline - time.monotonic()))
                        _signal_group(child.pid, signal.SIGKILL)
                finally:
                    child.wait(timeout=grace_seconds)

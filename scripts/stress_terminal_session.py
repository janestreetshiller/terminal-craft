#!/usr/bin/env python3
"""Run real PTY checks while repeatedly descheduling only their game child.

Unix-only regression for hosted-runner timing races. Build the release binary first.
No user game process or normal save is touched.
"""
import importlib.util
from pathlib import Path
import signal
import subprocess
import threading
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]


def main():
    spec = importlib.util.spec_from_file_location(
        'terminal_session', ROOT / 'tests/test_terminal_session.py'
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    original = subprocess.Popen

    class SlowGame(original):
        def __init__(self, args, *a, **kw):
            super().__init__(args, *a, **kw)
            if args == [str(ROOT / 'target/release/terminal-craft')]:
                def throttle():
                    while self.poll() is None:
                        try:
                            self.send_signal(signal.SIGSTOP)
                            time.sleep(.09)
                            self.send_signal(signal.SIGCONT)
                            time.sleep(.035)
                        except ProcessLookupError:
                            break
                threading.Thread(target=throttle, daemon=True).start()

    subprocess.Popen = SlowGame
    try:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(module.TerminalSessionTests)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        return 0 if result.wasSuccessful() else 1
    finally:
        subprocess.Popen = original


if __name__ == '__main__':
    raise SystemExit(main())

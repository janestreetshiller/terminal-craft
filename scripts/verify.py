#!/usr/bin/env python3
"""Verify an isolated source snapshot offline. GUI checks require exclusive approval."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from contextlib import contextmanager

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(root):
    files = subprocess.check_output(
        ['git', 'ls-files', '-co', '--exclude-standard', '-z'], cwd=root
    ).decode().split('\0')
    return {name: digest(root / name) for name in sorted(set(filter(None, files)))}


@contextmanager
def gui_lock(enabled):
    # Shared by verifier sessions for this user, not tied to the checkout path.
    path = Path.home() / '.local/state/terminal-craft/native-qa.lock'
    if not enabled:
        yield
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        handle = path.open('x')
    except FileExistsError:
        raise SystemExit(f'Native QA already owned: {path}. Contact that owner; do not remove an active lock.')
    try:
        with handle:
            handle.write(json.dumps({'pid': os.getpid(), 'started_utc': datetime.now(timezone.utc).isoformat()}))
        yield
    finally:
        path.unlink()


def verify(args):
    output = Path(args.output).expanduser().absolute() if args.output else Path(tempfile.mkdtemp(prefix='terminal-craft-verify-'))
    if output.resolve().is_relative_to(ROOT):
        raise SystemExit('Evidence output must be outside the source checkout.')
    if args.output:
        output.mkdir(parents=True, exist_ok=False)
    print(f'Isolated evidence: {output}', flush=True)
    before = inventory(ROOT)
    snapshot = output / 'source'
    for name in before:
        destination = snapshot / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / name, destination)
    if any(digest(snapshot / name) != sha for name, sha in before.items()):
        raise SystemExit('Source changed while copying; retry with the source owner.')
    cache = output / 'cargo-home'
    cache.mkdir()
    existing = Path(os.environ.get('CARGO_HOME', Path.home() / '.cargo'))
    # Copy dependency caches only, never credentials/configuration. No installs/downloads.
    for name in ('registry', 'git'):
        if (existing / name).is_dir():
            shutil.copytree(existing / name, cache / name)
    env = os.environ.copy()
    for name in list(env):
        if name.startswith(('TERMINAL_CRAFT_', 'TUICRAFT_')):
            env.pop(name)
    for name in ('tmp', 'cache', 'data', 'test-output'):
        (output / name).mkdir()
    env.update(CARGO_HOME=str(cache), CARGO_TARGET_DIR=str(snapshot / 'target'),
               TMPDIR=str(output / 'tmp'), XDG_CACHE_HOME=str(output / 'cache'),
               XDG_DATA_HOME=str(output / 'data'), PYTHONDONTWRITEBYTECODE='1',
               TERMINAL_CRAFT_SAVE=str(output / 'data/world.tcrf'),
               TERMINAL_CRAFT_TEST_OUTPUT=str(output / 'test-output'))
    if args.gui:
        env.update(TERMINAL_CRAFT_TEST_GUI='1', TERMINAL_CRAFT_TEST_INSTALLED='1')
    cargo = shutil.which('cargo')
    python = shutil.which('python3')
    if not cargo or not python:
        raise SystemExit('Existing Cargo and Python 3 are required.')
    commands = [
        [cargo, 'fmt', '--check'],
        [cargo, 'clippy', '--offline', '--all-targets', '--locked', '--', '-D', 'warnings'],
        [cargo, 'test', '--offline', '--locked'],
        [cargo, 'build', '--release', '--offline', '--locked'],
        [python, '-m', 'unittest', 'discover', '-s', 'tests', '-v'],
        [python, 'scripts/stress_terminal_session.py'],
    ]
    results = []
    for i, command in enumerate(commands, 1):
        process = subprocess.run(command, cwd=snapshot, env=env, capture_output=True, text=True)
        text = process.stdout + process.stderr
        (output / f'{i}.log').write_text(text)
        result = {'command': [Path(command[0]).name, *command[1:]], 'exit_code': process.returncode, 'log': f'{i}.log'}
        rust = re.search(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', text)
        if rust:
            result.update(rust_passed=int(rust[1]), rust_failed=int(rust[2]), rust_ignored=int(rust[3]))
        count = re.search(r'Ran (\d+) tests? in', text)
        skipped = re.search(r'skipped=(\d+)', text)
        if count:
            result.update(python_run=int(count[1]), python_skipped=int(skipped[1]) if skipped else 0)
        results.append(result)
        print(f'{"PASS" if process.returncode == 0 else "FAIL"}: {" ".join(result["command"])}', flush=True)
        if process.returncode:
            print(text)
            break
    unchanged = inventory(ROOT) == before
    snapshot_unchanged = all(digest(snapshot / name) == sha for name, sha in before.items())
    report = {
        'schema': 1, 'timestamp_utc': datetime.now(timezone.utc).isoformat(),
        'scope': 'source-and-approved-gui' if args.gui else 'source-only; GUI and installed checks skipped',
        'source_sha256': before, 'source_unchanged': unchanged,
        'snapshot_inputs_unchanged': snapshot_unchanged, 'results': results,
        'passed': unchanged and snapshot_unchanged and len(results) == len(commands) and all(r['exit_code'] == 0 for r in results),
        'release_ready': False,  # Requires visual/rights/hosted CI and explicit release approval.
    }
    binary = snapshot / 'target/release/terminal-craft'
    if binary.exists():
        report['binary_sha256'] = digest(binary)
    if args.gui:
        installed = Path('/Applications/Terminal Craft.app/Contents/MacOS/terminal-craft-bin')
        if installed.exists():
            report['installed_sha256'] = digest(installed)
            report['installed_matches_build'] = report['installed_sha256'] == report.get('binary_sha256')
        # Passing GUI tests on a different installed binary do not bind it to these sources.
        report['installed_source_binding'] = 'not established by behavior alone; inspect bundle build-manifest.json'
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(f'Report: {output / "report.json"}')
    return 0 if report['passed'] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', help='new, nonexistent evidence directory (default: unique temporary directory)')
    parser.add_argument('--gui', action='store_true', help='owner-approved exclusive native/installed checks; captures pointer')
    args = parser.parse_args()
    with gui_lock(args.gui):
        return verify(args)


if __name__ == '__main__':
    raise SystemExit(main())

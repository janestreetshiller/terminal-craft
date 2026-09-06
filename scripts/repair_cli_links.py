#!/usr/bin/env python3
"""Repair only Terminal Craft CLI aliases; no build, install, app or save access."""
from datetime import datetime, timezone
import os
from pathlib import Path
import tempfile

ALIASES = ('terminal-craft', 'terminalcraft', 'termcraft')


def repair_links(launcher: Path, directory: Path) -> list[Path]:
    launcher = launcher.resolve(strict=True)
    if not launcher.is_file() or not os.access(launcher, os.X_OK):
        raise ValueError('Launcher must be an existing executable file')
    directory.mkdir(parents=True, exist_ok=True)
    links = [directory / name for name in ALIASES]
    # Preflight all replacements, including dangling links, before changing any.
    for link in links:
        if os.path.lexists(link) and not link.is_symlink():
            raise ValueError(f'Refusing to replace non-symlink: {link}')
    backups = []
    stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
    for link in links:
        if link.is_symlink() and link.resolve() == launcher:
            continue
        if link.is_symlink():
            backup = link.with_name(f'{link.name}.backup-{stamp}')
            backup.symlink_to(os.readlink(link))
            backups.append(backup)
        fd, name = tempfile.mkstemp(prefix=f'.{link.name}-', dir=directory)
        os.close(fd)
        temporary = Path(name)
        try:
            temporary.unlink()
            temporary.symlink_to(launcher)
            os.replace(temporary, link)
        finally:
            temporary.unlink(missing_ok=True)
    return backups


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[1]
    for backup in repair_links(root / 'bin/terminal-craft', Path.home() / '.local/bin'):
        print(f'Backup: {backup}')
    print('Terminal Craft aliases now resolve to this checkout; no app was modified.')

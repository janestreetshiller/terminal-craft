"""Package updates never truncate an executable another process has mapped."""
import hashlib
import os
from pathlib import Path
import shutil
import tempfile

def source_manifest(root: Path) -> dict[str, str]:
    """Hash build/packaging inputs using portable relative paths, never user data."""
    files = [root / name for name in ('Cargo.toml', 'Cargo.lock', 'kitty-minigame.conf')]
    for directory in ('src', 'assets', '.cargo', 'bin', 'scripts'):
        files.extend(p for p in (root / directory).rglob('*')
                     if p.is_file() and '__pycache__' not in p.parts and p.suffix != '.pyc')
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(files)}


def atomic_copy(source: Path, destination: Path) -> None:
    fd, name = tempfile.mkstemp(prefix='.terminal-craft-package-', dir=destination.parent)
    os.close(fd)
    temporary = Path(name)
    try:
        shutil.copy2(source, temporary)
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)

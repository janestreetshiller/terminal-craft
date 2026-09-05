"""Package updates never truncate an executable another process has mapped."""
import os
from pathlib import Path
import shutil
import tempfile

def atomic_copy(source: Path, destination: Path) -> None:
    fd, name = tempfile.mkstemp(prefix='.terminal-craft-package-', dir=destination.parent)
    os.close(fd)
    temporary = Path(name)
    try:
        shutil.copy2(source, temporary)
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)

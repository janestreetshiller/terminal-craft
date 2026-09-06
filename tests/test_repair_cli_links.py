"""CLI relocation repair must preserve existing files and be repeatable."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('repair', ROOT / 'scripts/repair_cli_links.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class RepairLinksTests(unittest.TestCase):
    def test_dangling_aliases_are_backed_up_and_repair_is_idempotent(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            launcher = root / 'launcher'
            launcher.write_text('#!/bin/sh\nexit 0\n')
            launcher.chmod(0o755)
            directory = root / 'bin'
            directory.mkdir()
            for name in module.ALIASES:
                (directory / name).symlink_to(root / 'old-missing-checkout')
            backups = module.repair_links(launcher, directory)
            self.assertEqual(len(backups), 3)
            for backup in backups:
                self.assertEqual(os.readlink(backup), str(root / 'old-missing-checkout'))
            for name in module.ALIASES:
                self.assertEqual((directory / name).resolve(strict=True), launcher.resolve(strict=True))
            self.assertEqual(module.repair_links(launcher, directory), [])

    def test_preflight_preserves_all_aliases_when_one_is_not_a_symlink(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            launcher = root / 'launcher'
            launcher.write_text('#!/bin/sh\nexit 0\n')
            launcher.chmod(0o755)
            (root / 'terminal-craft').symlink_to('missing')
            (root / 'terminalcraft').write_text('user command')
            with self.assertRaises(ValueError):
                module.repair_links(launcher, root)
            self.assertEqual(os.readlink(root / 'terminal-craft'), 'missing')
            self.assertEqual((root / 'terminalcraft').read_text(), 'user command')
            self.assertEqual(list(root.glob('*.backup-*')), [])


if __name__ == '__main__':
    unittest.main(verbosity=2)

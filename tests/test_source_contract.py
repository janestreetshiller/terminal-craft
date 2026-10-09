"""Portable documentation, provenance and verification-safety regressions."""
import hashlib
import importlib.util
from pathlib import Path
import re
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class SourceContractTests(unittest.TestCase):
    def test_documentation_local_links_exist(self):
        files = [ROOT / 'README.md', ROOT / 'assets/maps/README.md', *ROOT.glob('docs/*.md')]
        for path in files:
            for link in re.findall(r'\]\(([^)]+)\)', path.read_text()):
                if '://' not in link and not link.startswith('#'):
                    with self.subTest(document=path.name, link=link):
                        self.assertTrue((path.parent / link.split('#')[0]).exists())

    def test_public_documents_do_not_embed_absolute_user_paths(self):
        for path in (ROOT / 'docs').rglob('*'):
            if path.is_file() and path.suffix in ('.md', '.json', '.log'):
                with self.subTest(document=str(path.relative_to(ROOT))):
                    self.assertIsNone(re.search(r'/(?:Users|home)/[^/\s]+/', path.read_text()))
        self.assertIn('All rights reserved', (ROOT / 'LICENSE').read_text())

    def test_build_manifest_is_relative_and_tracks_changed_inputs(self):
        package = load('package_files')
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name in ('Cargo.toml', 'Cargo.lock', 'kitty-minigame.conf'):
                (root / name).write_text(name)
            (root / 'src').mkdir()
            (root / 'src/main.rs').write_text('first')
            (root / 'target').mkdir()
            (root / 'target/private-save.tcrf').write_text('not a build input')
            before = package.source_manifest(root)
            self.assertNotIn('target/private-save.tcrf', before)
            self.assertTrue(all(not Path(name).is_absolute() for name in before))
            (root / 'src/main.rs').write_text('second')
            after = package.source_manifest(root)
            self.assertNotEqual(before['src/main.rs'], after['src/main.rs'])
            self.assertEqual(after['src/main.rs'], hashlib.sha256(b'second').hexdigest())

    def test_gui_lock_refuses_a_second_owner_and_preserves_their_lock(self):
        verifier = load('verify')
        with tempfile.TemporaryDirectory() as tmp, patch.object(Path, 'home', return_value=Path(tmp)):
            lock = Path(tmp) / '.local/state/terminal-craft/native-qa.lock'
            with verifier.gui_lock(True):
                before = lock.read_bytes()
                with self.assertRaises(SystemExit):
                    with verifier.gui_lock(True):
                        self.fail('second owner acquired lock')
                self.assertEqual(lock.read_bytes(), before)
            self.assertFalse(lock.exists())

    def test_launcher_exports_cmake_policy_before_manifest_path_build(self):
        launcher = (ROOT / 'bin/terminal-craft').read_text()
        policy = 'export CMAKE_POLICY_VERSION_MINIMUM="${CMAKE_POLICY_VERSION_MINIMUM:-3.5}"'
        build = 'cargo build --release --locked --manifest-path "$ROOT/Cargo.toml"'
        self.assertIn(policy, launcher)
        self.assertLess(launcher.index(policy), launcher.index(build))


if __name__ == '__main__':
    unittest.main(verbosity=2)

"""Safe replacement must leave an already-open executable unchanged."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[1]
class PackageFilesTests(unittest.TestCase):
    def test_atomic_copy_preserves_old_open_inode(self):
        script=ROOT/'scripts/package_files.py'
        self.assertTrue(script.is_file(), 'Missing safe package-copy helper')
        spec=importlib.util.spec_from_file_location('package_files',script)
        assert spec is not None and spec.loader is not None
        module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as td:
            src=Path(td)/'source'; dst=Path(td)/'running'
            src.write_bytes(b'new binary'); dst.write_bytes(b'old running binary')
            with dst.open('rb') as old:
                module.atomic_copy(src,dst)
                self.assertEqual(old.read(),b'old running binary')
            self.assertEqual(dst.read_bytes(),b'new binary')

if __name__=='__main__': unittest.main(verbosity=2)

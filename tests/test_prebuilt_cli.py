"""Bundled maps must export intact and never overwrite an existing world."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[1]
BIN=ROOT/'target/release/terminal-craft'

class PrebuiltCliTests(unittest.TestCase):
    def test_catalog_and_all_exports(self):
        catalog=json.loads((ROOT/'assets/maps/catalog.json').read_text())
        run=subprocess.run([str(BIN),'--maps'],capture_output=True,text=True)
        self.assertEqual(run.returncode,0,run.stderr)
        self.assertEqual([line.split()[0] for line in run.stdout.splitlines()],[m['id'] for m in catalog])
        with tempfile.TemporaryDirectory() as temp:
            for m in catalog:
                with self.subTest(map=m['id']):
                    path=Path(temp)/f"{m['id']}.tcrf"
                    export=subprocess.run([str(BIN),'--export-map',m['id'],str(path)],capture_output=True,text=True)
                    self.assertEqual(export.returncode,0,export.stderr)
                    self.assertEqual(path.read_bytes(),(ROOT/'assets/maps'/path.name).read_bytes())
                    subprocess.run([str(BIN),'--check-save',str(path)],check=True,capture_output=True)
                    path.write_bytes(b'existing edits')
                    again=subprocess.run([str(BIN),'--export-map',m['id'],str(path)],capture_output=True)
                    self.assertNotEqual(again.returncode,0)
                    self.assertEqual(path.read_bytes(),b'existing edits')
    def test_export_refuses_symlinks_and_cleans_temporary_files(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            victim=root/'existing.tcrf'
            victim.write_bytes(b'user edits')
            for destination in [root/'link.tcrf',root/'dangling.tcrf']:
                destination.symlink_to(victim if destination.name=='link.tcrf' else root/'absent')
                run=subprocess.run([str(BIN),'--export-map','foundry',str(destination)],capture_output=True)
                self.assertNotEqual(run.returncode,0)
                self.assertTrue(destination.is_symlink())
                self.assertEqual(victim.read_bytes(),b'user edits')
                self.assertFalse((root/'absent').exists())
                self.assertEqual(list(root.glob('.prebuilt-*.tmp')),[])
            missing=root/'missing'/'map.tcrf'
            run=subprocess.run([str(BIN),'--export-map','foundry',str(missing)],capture_output=True)
            self.assertNotEqual(run.returncode,0)
            self.assertFalse(missing.parent.exists())

    def test_concurrent_exports_publish_one_complete_file(self):
        with tempfile.TemporaryDirectory() as temp:
            destination=Path(temp)/'map.tcrf'
            processes=[subprocess.Popen([str(BIN),'--export-map','foundry',str(destination)],
                                        stdout=subprocess.PIPE,stderr=subprocess.PIPE) for _ in range(4)]
            for process in processes:
                process.communicate(timeout=10)
            self.assertEqual(sum(p.returncode==0 for p in processes),1)
            self.assertEqual(destination.read_bytes(),(ROOT/'assets/maps/foundry.tcrf').read_bytes())
            self.assertEqual(list(Path(temp).glob('.prebuilt-*.tmp')),[])

    def test_unknown_map_is_rejected(self):
        result=subprocess.run([str(BIN),'--map','../outside'],capture_output=True,text=True,timeout=5)
        self.assertEqual(result.returncode,2)
        self.assertIn('unknown prebuilt map',result.stderr)

if __name__=='__main__':unittest.main(verbosity=2)

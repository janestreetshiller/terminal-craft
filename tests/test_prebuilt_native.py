"""Exercise all five pristine arenas through the actual SDL map picker."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[1]

@unittest.skipUnless(os.environ.get('TERMINAL_CRAFT_TEST_GUI') == '1',
                     'native GUI tests require exclusive owner approval (--gui verifier)')
class PrebuiltNativeTests(unittest.TestCase):
    def test_all_maps_in_release_and_installed_app(self):
        expected=[m['id'] for m in json.loads((ROOT/'assets/maps/catalog.json').read_text())]
        binaries=[ROOT/'target/release/terminal-craft',Path(os.environ.get('TERMINAL_CRAFT_TEST_INSTALLED_BINARY', '/Applications/Terminal Craft.app/Contents/MacOS/terminal-craft-bin'))]
        with tempfile.TemporaryDirectory(prefix='terminal-craft-maps-') as temp:
            for i,binary in enumerate(binaries):
                with self.subTest(binary=str(binary)):
                    directory=Path(temp)/f'run-{i}'
                    env=dict(os.environ,TERMINAL_CRAFT_SENS='0.0024')
                    env.pop('TERMINAL_CRAFT_UI',None)
                    run=subprocess.run([str(binary),'--native-map-test',str(directory)],env=env,capture_output=True,text=True,timeout=180)
                    if directory.exists() and os.environ.get('TERMINAL_CRAFT_TEST_OUTPUT'):
                        shutil.copytree(directory,Path(os.environ['TERMINAL_CRAFT_TEST_OUTPUT'])/f'prebuilt-native-{i}')
                    self.assertEqual(run.returncode,0,run.stdout+run.stderr)
                    report=json.loads((directory/'report.json').read_text())
                    self.assertTrue(report['passed'],report)
                    self.assertTrue(report['default_world_unchanged'])
                    self.assertTrue(report['pointer_released_on_exit'])
                    self.assertEqual(report['map_count'],5)
                    self.assertEqual([m['id'] for m in report['maps']],expected)
                    for m in report['maps']:
                        for key,value in m.items():
                            if isinstance(value,bool):self.assertTrue(value,(m['id'],key))
                    self.assertTrue((directory/'picker-compact.ppm').read_bytes().startswith(b'P6\n640 420\n'))

if __name__=='__main__':unittest.main(verbosity=2)

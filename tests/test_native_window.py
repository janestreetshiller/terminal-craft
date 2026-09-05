"""Real SDL-window QA of release/packaged executables and both UI skins."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import shutil
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[1]

class NativeWindowTests(unittest.TestCase):
    def test_native_motion_and_packaged_runtime(self):
        saves=[Path.home()/'.local/share'/name/'world.tcrf' for name in ['terminal-craft','tuicraft']]
        before={p:hashlib.sha256(p.read_bytes()).hexdigest() for p in saves if p.exists()}
        binaries=[ROOT/'target/release/terminal-craft',Path('/Applications/Terminal Craft.app/Contents/MacOS/terminal-craft-bin')]
        with tempfile.TemporaryDirectory(prefix='terminal-craft-native-') as temp:
            for theme in ['classic','gilded']:
                for i,binary in enumerate(binaries):
                    with self.subTest(binary=str(binary),theme=theme):
                        directory=Path(temp)/f'{theme}-{i}'
                        env=dict(os.environ,TERMINAL_CRAFT_UI=theme)
                        run=subprocess.run([str(binary),'--native-smoke-test',str(directory)],env=env,capture_output=True,text=True,timeout=60)
                        if directory.exists():
                            shutil.copytree(directory,ROOT/'target/verification'/f'native-{theme}-{i}',dirs_exist_ok=True)
                        self.assertEqual(run.returncode,0,run.stdout+run.stderr)
                        report=json.loads((directory/'report.json').read_text())
                        self.assertTrue(report['passed'],report)
                        self.assertEqual(report['frames'],142)
                        self.assertEqual(report['ui_theme'],theme.upper())
                        self.assertTrue(report['ui_theme_round_trip'])
                        for name,value in report.items():
                            if isinstance(value,bool): self.assertTrue(value,name)
        for p,digest in before.items(): self.assertEqual(hashlib.sha256(p.read_bytes()).hexdigest(),digest,f'Save changed: {p}')

if __name__=='__main__': unittest.main(verbosity=2)

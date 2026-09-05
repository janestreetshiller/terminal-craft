import os
from pathlib import Path
import plistlib
import subprocess
import unittest

ROOT=Path(__file__).resolve().parents[1]
class InstallTests(unittest.TestCase):
    def test_release_identifies_as_terminal_craft(self):
        binary=ROOT/'target/release/terminal-craft'
        self.assertTrue(binary.is_file())
        p=subprocess.run([str(binary),'--version'],capture_output=True,text=True)
        self.assertEqual(p.returncode,0,p.stderr)
        self.assertIn('Terminal Craft',p.stdout)

    def test_installed_app_and_cli_work_without_shell_configuration(self):
        app=Path('/Applications/Terminal Craft.app')
        self.assertTrue(app.is_dir())
        with (app/'Contents/Info.plist').open('rb') as f: info=plistlib.load(f)
        self.assertEqual(info['CFBundleDisplayName'],'Terminal Craft')
        env={'HOME':str(Path.home()),'PATH':'/usr/bin:/bin:/usr/sbin:/sbin'}
        for executable in [app/'Contents/MacOS'/info['CFBundleExecutable'],Path.home()/'.local/bin/terminal-craft',Path.home()/'.local/bin/termcraft']:
            p=subprocess.run([str(executable),'--help'],env=env,capture_output=True,text=True)
            self.assertEqual(p.returncode,0,p.stderr)
            self.assertIn('Terminal Craft',p.stdout)
            self.assertIn('pickaxe',p.stdout.lower())

if __name__=='__main__': unittest.main(verbosity=2)

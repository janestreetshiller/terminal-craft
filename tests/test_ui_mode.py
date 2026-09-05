"""UI selection is process-local and does not silently accept an unknown skin."""
import os
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[1]

class UiModeTests(unittest.TestCase):
    def test_help_advertises_gilded_mode_and_live_switch(self):
        run = subprocess.run([str(ROOT/'target/release/terminal-craft'),'--help'],
                             capture_output=True,text=True,check=True)
        self.assertIn('--gilded',run.stdout)
        self.assertIn('F6',run.stdout)
        self.assertIn('TERMINAL_CRAFT_UI=classic|gilded',run.stdout)

    def test_invalid_ui_name_is_rejected_before_startup(self):
        env=dict(os.environ,TERMINAL_CRAFT_UI='not-a-theme')
        run = subprocess.run([str(ROOT/'target/release/terminal-craft'),'--version'],
                             env=env,capture_output=True,text=True)
        self.assertEqual(run.returncode,2)
        self.assertIn('classic or gilded',run.stderr)

if __name__ == '__main__':
    unittest.main(verbosity=2)

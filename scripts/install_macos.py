#!/usr/bin/env python3
"""Build/package Terminal Craft locally. No uploads or save resets; initial Cargo dependencies may download."""
from pathlib import Path
from datetime import datetime
import hashlib
import os
import plistlib
import shutil
import subprocess
import sys
from package_files import atomic_copy

ROOT=Path(__file__).resolve().parents[1]
if sys.platform!='darwin': raise SystemExit('macOS installer; use cargo build --release on other Unix systems.')
subprocess.run(['cargo','build','--release','--locked'],cwd=ROOT,check=True)
source=ROOT/'target/release/terminal-craft'
app=Path('/Applications/Terminal Craft.app')
backup=Path.home()/'.local/state/terminal-craft/install-backups'/datetime.now().strftime('%Y%m%d-%H%M%S-%f')
backup.mkdir(parents=True)
# Preflight all CLI replacements before changing any of them.
commands=['terminal-craft','terminalcraft','termcraft']
local_bin=Path.home()/'.local/bin'
for name in commands:
    p=local_bin/name
    if p.exists() and not p.is_symlink(): raise SystemExit(f'Refusing to replace non-symlink {p}')
if app.exists(): shutil.copytree(app,backup/app.name,symlinks=True)
contents=app/'Contents'; macos=contents/'MacOS'; resources=contents/'Resources'
macos.mkdir(parents=True,exist_ok=True); resources.mkdir(parents=True,exist_ok=True)
atomic_copy(source,macos/'terminal-craft-bin')
atomic_copy(ROOT/'bin/terminal-craft',macos/'launch')
(macos/'launch').chmod(0o755)
shutil.copy2(ROOT/'kitty-minigame.conf',resources/'kitty-minigame.conf')
with (contents/'Info.plist').open('wb') as f:
    plistlib.dump({'CFBundleName':'Terminal Craft','CFBundleDisplayName':'Terminal Craft',
      'CFBundleIdentifier':'local.terminal-craft','CFBundleExecutable':'launch',
      'CFBundlePackageType':'APPL','CFBundleShortVersionString':'0.9.0','CFBundleVersion':'10',
      'LSUIElement':False,'NSHighResolutionCapable':True},f)
subprocess.run(['codesign','--force','--sign','-',str(app)],check=True)
subprocess.run(['codesign','--verify','--strict',str(app)],check=True)
local_bin.mkdir(parents=True,exist_ok=True)
(ROOT/'bin/terminal-craft').chmod(0o755)
for name in commands:
    p=local_bin/name
    if p.is_symlink():
        (backup/name).symlink_to(os.readlink(p)); p.unlink()
    p.symlink_to(ROOT/'bin/terminal-craft')
old=Path('/Applications/TermCraft.app')
if old.exists(): shutil.move(str(old),backup/old.name)
subprocess.run(['/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister','-f',str(app)],check=True)
assert hashlib.sha256(source.read_bytes()).digest()==hashlib.sha256((macos/'terminal-craft-bin').read_bytes()).digest()
print(f'Installed {app}\nSource {ROOT}\nBackups {backup}\nSaves are preserved; legacy native save is copied on first game launch.')

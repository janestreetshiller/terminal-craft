#!/usr/bin/env python3
"""Run native release checks and preserve actual output under target/verification."""
from datetime import datetime, timezone
from pathlib import Path
import hashlib
import json
import re
import subprocess
import sys

root=Path(__file__).resolve().parents[1]
output=root/'target/verification'; output.mkdir(parents=True,exist_ok=True)
commands=[
    ['cargo','fmt','--check'],
    ['cargo','clippy','--all-targets','--locked','--','-D','warnings'],
    ['cargo','test','--locked'],
    ['cargo','build','--release','--locked'],
    [sys.executable,'-m','unittest','discover','-s','tests','-v'],
    ['git','diff','--check'],
]
results=[]
for i,command in enumerate(commands):
    p=subprocess.run(command,cwd=root,capture_output=True,text=True)
    log=output/f'{i+1}.log'; log.write_text(p.stdout+p.stderr)
    result={'command':command,'exit_code':p.returncode,'log':str(log)}
    text=p.stdout+p.stderr
    count=re.search(r'test result: ok\. (\d+) passed',text)
    if count: result['rust_tests_passed']=int(count.group(1))
    count=re.search(r'Ran (\d+) tests? in',text)
    if count and p.returncode==0: result['python_tests_passed']=int(count.group(1))
    results.append(result)
    print(f'{"PASS" if p.returncode==0 else "FAIL"}: {" ".join(command)}')
    if p.returncode:
        print(text); break
report={'timestamp':datetime.now(timezone.utc).isoformat(),'results':results,'passed':all(r['exit_code']==0 for r in results) and len(results)==len(commands)}
binary=root/'target/release/terminal-craft'
if binary.exists(): report['release_sha256']=hashlib.sha256(binary.read_bytes()).hexdigest()
installed=Path('/Applications/Terminal Craft.app/Contents/MacOS/terminal-craft-bin')
if installed.exists():
    report['installed_sha256']=hashlib.sha256(installed.read_bytes()).hexdigest()
    report['installed_matches_release']=report['installed_sha256']==report.get('release_sha256')
    report['passed']=report['passed'] and report['installed_matches_release']
(output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
raise SystemExit(0 if report['passed'] else 1)

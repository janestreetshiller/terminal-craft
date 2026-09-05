"""Exercise the actual release executable through a PTY and a disposable world."""
import fcntl
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import tempfile
import termios
import time
import unittest

ROOT=Path(__file__).resolve().parents[1]
SX,SY,SZ=96,40,96
SIZE=SX*SY*SZ

def cell(x,y,z): return y*SZ*SX+z*SX+x

def fixture(path):
    blocks=bytearray(SIZE)
    blocks[19*SZ*SX:20*SZ*SX]=bytes([2])*(SZ*SX)
    blocks[cell(23,21,20)]=2
    blocks[cell(24,21,20)]=2
    header=struct.pack('<4sHI3H2B5H',b'TCRF',2,42,SX,SY,SZ,0,0,5,0,0,0,0)
    state=struct.pack('<B5f2B',1,20.5,20.001,20.5,0.,0.,0,0)
    path.write_bytes(header+blocks+state)

def snapshot(path):
    data=path.read_bytes()
    return {'blocks':data[28:28+SIZE], 'inventory':struct.unpack_from('<5H',data,18),
            'pose':struct.unpack_from('<5f',data,29+SIZE), 'tool':data[49+SIZE]}

class TerminalSessionTests(unittest.TestCase):
    def test_legacy_terminal_session(self):
        self.session(False)

    def test_enhanced_press_release_terminal_session(self):
        self.session(True)

    def session(self, enhanced):
        with tempfile.TemporaryDirectory(prefix='terminal-craft-pty-') as td:
            save=Path(td)/'world.tcrf'; fixture(save)
            master,slave=pty.openpty()
            fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',30,100,800,480))
            env=dict(os.environ,TERM='xterm-256color',TERMINAL_CRAFT_ASCII='1',TERMINAL_CRAFT_SAVE=str(save))
            env.pop('KITTY_WINDOW_ID',None)
            if enhanced: env.update(TERM='xterm-kitty',KITTY_WINDOW_ID='1')
            proc=subprocess.Popen([str(ROOT/'target/release/terminal-craft')],stdin=slave,stdout=slave,stderr=slave,env=env,close_fds=True)
            os.close(slave)
            output=bytearray()
            def pump(seconds):
                end=time.monotonic()+seconds
                while time.monotonic()<end:
                    if select.select([master],[],[],0.02)[0]:
                        try: output.extend(os.read(master,65536))
                        except OSError: break
                        if len(output)>2_000_000: del output[:-1_000_000]
            def send(text,wait=.2):
                if not enhanced:
                    os.write(master,text); pump(wait); return
                codes=[(113,5) if b==17 else (b,1) for b in text]
                os.write(master,b''.join(f'\x1b[{code};{mod}:1u'.encode() for code,mod in codes))
                pump(wait)
                if proc.poll() is None:
                    os.write(master,b''.join(f'\x1b[{code};{mod}:3u'.encode() for code,mod in codes))
                    pump(.03)
            def wait_until(predicate, description, tick=None):
                deadline=time.monotonic()+8
                while time.monotonic()<deadline:
                    self.assertIsNone(proc.poll(), f'Game exited while waiting for {description}')
                    if tick: tick()
                    pump(.03)
                    if predicate(): return
                self.fail(f'Timed out waiting for {description}')
            def saved_until(predicate, description, repeat=None):
                state=None
                def read_state():
                    nonlocal state
                    state=snapshot(save)
                    return predicate(state)
                def request_save():
                    if repeat: os.write(master,repeat)
                    send(b'r',.03)
                wait_until(read_state, description, request_save)
                assert state is not None
                return state
            def hold_until_saved(code, predicate, description):
                # Enhanced mode holds ONE press until observed progress, then releases.
                # Legacy terminals need repeat presses to refresh their hold timeout.
                if enhanced: os.write(master,f'\x1b[{code};1:1u'.encode())
                try:
                    return saved_until(predicate,description,None if enhanced else bytes([code]))
                finally:
                    if enhanced and proc.poll() is None:
                        os.write(master,f'\x1b[{code};1:3u'.encode())
                        pump(.03)
            try:
                wait_until(lambda: b'TERMINAL CRAFT' in output,'main menu')
                send(b'\r',.3) # Continue fixture world
                send(b'7')
                saved_until(lambda s: s['tool']==2,'equipped pickaxe')
                # Wall time is not simulation time on a loaded runner (dt is capped).
                # Wait for the real block mutation, not a fixed 420 ms sleep.
                hold_until_saved(ord('e'),lambda s: s['blocks'][cell(23,21,20)]==0,'first block mined')
                output.clear()
                send(b'h',.03) # pause mining before the next target
                wait_until(lambda: b'CONTROLS AND RECIPES' in output,'controls overlay')
                # Tool/block changes prove this is the updated save, not the fixture.
                after=saved_until(lambda s: s['blocks'][cell(23,21,20)]==0,'mined save')
                self.assertEqual(after['tool'],2)
                self.assertEqual(after['blocks'][cell(23,21,20)],0)
                self.assertEqual(after['blocks'][cell(24,21,20)],2)
                self.assertEqual(after['inventory'][1],1)
                send(b'h')
                send(b'1q')
                placed=saved_until(lambda s: s['blocks'][cell(23,21,20)]==1,'placed block')
                self.assertEqual(placed['blocks'][cell(23,21,20)],1)
                self.assertEqual(placed['inventory'][0],4)
                moved=hold_until_saved(ord('a'),lambda s: s['pose'][:3]!=placed['pose'][:3],'player movement')
                self.assertNotEqual(moved['pose'][:3],placed['pose'][:3])
                output.clear()
                send(b'\x1b',.03) # Escape pauses rather than quitting.
                wait_until(lambda: b'CONTROLS AND RECIPES' in output,'pause overlay')
                self.assertIsNone(proc.poll())
                send(b'\x11',.3) # Ctrl-Q saves and quits.
                proc.wait(timeout=8)
                self.assertEqual(proc.returncode,0)
                check=subprocess.run([str(ROOT/'target/release/terminal-craft'),'--check-save',str(save)],capture_output=True,text=True)
                self.assertEqual(check.returncode,0,check.stderr)
            finally:
                if proc.poll() is None: proc.terminate(); proc.wait(timeout=5)
                os.close(master)

if __name__=='__main__': unittest.main(verbosity=2)

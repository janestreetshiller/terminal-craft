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
    def test_real_keyboard_mining_placement_help_movement_and_save(self):
        with tempfile.TemporaryDirectory(prefix='terminal-craft-pty-') as td:
            save=Path(td)/'world.tcrf'; fixture(save)
            master,slave=pty.openpty()
            fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',30,100,800,480))
            env=dict(os.environ,TERM='xterm-256color',TERMINAL_CRAFT_ASCII='1',TERMINAL_CRAFT_SAVE=str(save))
            env.pop('KITTY_WINDOW_ID',None)
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
            def send(text,wait=.2): os.write(master,text); pump(wait)
            try:
                pump(.4)
                self.assertIn(b'TERMINAL CRAFT',output)
                send(b'\r',.3) # Continue fixture world
                send(b'7')
                send(b'e',.42)
                send(b'h',.1) # pause mining before the next target
                self.assertIn(b'CONTROLS AND RECIPES',output)
                send(b'r')
                after=snapshot(save)
                self.assertEqual(after['tool'],2)
                self.assertEqual(after['blocks'][cell(23,21,20)],0)
                self.assertEqual(after['blocks'][cell(24,21,20)],2)
                self.assertEqual(after['inventory'][1],1)
                send(b'h')
                send(b'1q')
                send(b'r')
                placed=snapshot(save)
                self.assertEqual(placed['blocks'][cell(23,21,20)],1)
                self.assertEqual(placed['inventory'][0],4)
                send(b'a',.45)
                send(b'r')
                moved=snapshot(save)
                self.assertNotEqual(moved['pose'][:3],placed['pose'][:3])
                send(b'\x1b',.3)
                proc.wait(timeout=5)
                self.assertEqual(proc.returncode,0)
                check=subprocess.run([str(ROOT/'target/release/terminal-craft'),'--check-save',str(save)],capture_output=True,text=True)
                self.assertEqual(check.returncode,0,check.stderr)
            finally:
                if proc.poll() is None: proc.terminate(); proc.wait(timeout=5)
                os.close(master)

if __name__=='__main__': unittest.main(verbosity=2)

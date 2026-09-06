"""Select every map through the real PTY menu, move, save and resume it."""
import fcntl
import json
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
from test_terminal_session import fixture,snapshot

ROOT=Path(__file__).resolve().parents[1]
BIN=ROOT/'target/release/terminal-craft'

class PrebuiltTerminalTests(unittest.TestCase):
    def test_every_map_and_direct_resume_preserve_default_world(self):
        catalog=json.loads((ROOT/'assets/maps/catalog.json').read_text())
        with tempfile.TemporaryDirectory(prefix='terminal-craft-prebuilt-pty-') as temp:
            base=Path(temp)/'world.tcrf';fixture(base);original=base.read_bytes()
            for index,m in enumerate(catalog):
                with self.subTest(map=m['id']):
                    path=Path(str(base)+'.prebuilt')/f"{m['id']}.tcrf"
                    self.session(base,path,m,index,False)
                    saved=snapshot(path)
                    self.assertGreater(saved['pose'][0],m['spawn'][0]+0.1)
                    self.session(base,path,m,index,True)
                    resumed=snapshot(path)
                    self.assertEqual(resumed['blocks'],saved['blocks'])
                    self.assertEqual(resumed['pose'],saved['pose'])
                    self.assertEqual(base.read_bytes(),original)
            self.assertEqual(len(list(Path(str(base)+'.prebuilt').glob('*.tcrf'))),5)

    def session(self,base,path,m,index,resume):
        master,slave=pty.openpty()
        fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',30,100,800,480))
        env=dict(os.environ,TERM='xterm-256color',TERMINAL_CRAFT_ASCII='1',TERMINAL_CRAFT_SAVE=str(base))
        env.pop('KITTY_WINDOW_ID',None)
        args=[str(BIN),'--here']+(['--map',m['id']] if resume else [])
        proc=subprocess.Popen(args,stdin=slave,stdout=slave,stderr=slave,env=env,close_fds=True)
        os.close(slave);output=bytearray()
        def pump():
            if select.select([master],[],[],0.03)[0]:
                try:output.extend(os.read(master,65536))
                except OSError:return
                if len(output)>500000:del output[:-250000]
        def wait(predicate,description,tick=None):
            until=time.monotonic()+8
            while time.monotonic()<until:
                self.assertIsNone(proc.poll(),f'Exited waiting for {description}: {output[-1500:]}')
                if tick:tick()
                pump()
                if predicate():return
            self.fail(f'Timed out waiting for {description}')
        try:
            if not resume:
                wait(lambda:b'PREBUILT WORLDS' in output,'main menu')
                output.clear();os.write(master,b'4')
                wait(lambda:b'FOUNDRY' in output and b'DUNE OUTPOST' in output,'five-map picker')
                os.write(master,str(index+1).encode())
                wait(path.exists,'map creation')
                wait(lambda:snapshot(path)['pose'][0]>m['spawn'][0]+0.1,'walking and saving',lambda:os.write(master,b'wr'))
            else:
                output.clear()
                wait(lambda:m['title'].upper().encode() in output,'resumed game')
            os.write(master,b'\x11')
            until=time.monotonic()+8
            while proc.poll() is None and time.monotonic()<until:pump()
            self.assertIsNotNone(proc.poll(),'save-and-quit did not finish')
            self.assertEqual(proc.returncode,0,output[-1500:])
        finally:
            if proc.poll() is None:proc.kill();proc.wait()
            os.close(master)

if __name__=='__main__':unittest.main(verbosity=2)

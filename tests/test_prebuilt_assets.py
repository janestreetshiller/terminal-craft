"""Validate original authored map assets, not random seed variants."""
from pathlib import Path
import hashlib
import json
import struct
import unittest

ROOT=Path(__file__).resolve().parents[1]

class PrebuiltAssetsTests(unittest.TestCase):
    def test_templates_are_reproducible(self):
        import subprocess,sys
        run=subprocess.run([sys.executable,str(ROOT/'scripts/build_maps.py'),'--check'],capture_output=True,text=True)
        self.assertEqual(run.returncode,0,run.stdout+run.stderr)

    def test_five_distinct_walkable_maps_with_clear_spawns(self):
        self.assertTrue((ROOT/'assets/maps/catalog.json').is_file(), 'prebuilt catalog is missing')
        catalog=json.loads((ROOT/'assets/maps/catalog.json').read_text())
        self.assertEqual(len(catalog),5)
        self.assertEqual(len({m['id'] for m in catalog}),5)
        hashes=set()
        for m in catalog:
            with self.subTest(map=m['id']):
                blob=(ROOT/'assets/maps'/f"{m['id']}.tcrf").read_bytes()
                self.assertEqual(blob[:4],b'TCRF')
                self.assertEqual(struct.unpack_from('<3H',blob,10),(96,40,96))
                hashes.add(hashlib.sha256(blob).hexdigest())
                blocks=blob[28:28+96*40*96]
                x,y,z,yaw,pitch=struct.unpack_from('<5f',blob,28+len(blocks)+1)
                at=lambda xx,yy,zz:blocks[yy*96*96+zz*96+xx]
                self.assertNotEqual(at(int(x),int(y)-1,int(z)),0)
                for yy in [int(y),int(y)+1,int(y)+2]:
                    self.assertEqual(at(int(x),yy,int(z)),0)
                self.assertGreater(m['walkable_reachable_cells'],2500)
                self.assertGreater(m['structure_blocks'],1500)
                self.assertEqual(m['origin'],'original')
                self.assertEqual(hashlib.sha256(blob).hexdigest(),m['sha256'])
        self.assertEqual(len(hashes),5)

if __name__=='__main__': unittest.main(verbosity=2)

#!/usr/bin/env python3
"""Build five original, fixed arena layouts. No downloaded geometry or textures.

Outputs are ordinary v2 TCRF saves embedded in the app. Reproducible with Python
stdlib only; this script never reads or modifies player save directories.
"""
from collections import deque
from pathlib import Path
import hashlib
import json
import math
import struct

ROOT=Path(__file__).resolve().parents[1]
AIR,TERRA,SLATE,SAND,WOOD,LEAF,RUST,JUNGLE,BLUE,RUBY,CORE,PIPE,BEACON=range(13)
W,H,D=96,40,96

class Build:
    def __init__(self,floor):
        self.data=bytearray(W*H*D)
        self.box(0,0,0,95,2,95,SLATE)
        self.box(0,3,0,95,3,95,floor)
    def get(self,x,y,z):
        if not(0<=x<W and 0<=y<H and 0<=z<D): return AIR
        return self.data[y*W*D+z*W+x]
    def box(self,x,y,z,xx,yy,zz,block):
        assert 0<=x<=xx<W and 0<=y<=yy<H and 0<=z<=zz<D
        row=bytes([block])*(xx-x+1)
        for j in range(y,yy+1):
            for k in range(z,zz+1):
                at=j*W*D+k*W+x
                self.data[at:at+len(row)]=row
    def shell(self,x,z,w,d,height,wall,roof,y=4):
        self.box(x,y,z,x+w-1,y+height-1,z+d-1,wall)
        self.box(x+1,y,z+1,x+w-2,y+height-2,z+d-2,AIR)
        self.box(x,y+height-1,z,x+w-1,y+height-1,z+d-1,roof)
        # Open west and east doors. No non-functional solid door blocks.
        mid=z+d//2
        for xx in [x,x+w-1]: self.box(xx,y,mid-1,xx,y+3,mid+1,AIR)
        # Open windows rather than pretending opaque blocks are glass.
        for xx in range(x+3,x+w-2,5):
            for zz in [z,z+d-1]:self.box(xx,y+3,zz,xx+1,y+4,zz,AIR)
    def cylinder(self,cx,cz,r,y,top,block,hollow=False):
        for z in range(cz-r,cz+r+1):
            for x in range(cx-r,cx+r+1):
                dist=(x-cx)**2+(z-cz)**2
                if dist<=r*r and (not hollow or dist>=(r-1)**2):
                    self.box(x,y,z,x,top,z,block)
    def stair(self,x,z,count,y=4,width=3,axis='z',block=SLATE):
        for i in range(count):
            xx=x+(i if axis=='x' else 0)
            zz=z+(i if axis=='z' else 0)
            endx=xx+(width-1 if axis=='z' else 0)
            endz=zz+(width-1 if axis=='x' else 0)
            self.box(xx,4,zz,endx,y+i,endz,block)
            self.box(xx,y+i+1,zz,endx,y+i+3,endz,AIR)
    def crate(self,x,z,size=4,block=WOOD,y=4):
        self.box(x,y,z,x+size-1,y+size-1,z+size-1,block)
        for xx in [x,x+size-1]:
            for zz in [z,z+size-1]:self.box(xx,y,zz,xx,y+size-1,zz,SLATE)
    def tree(self,x,z):
        self.box(x,4,z,x,10,z,WOOD)
        self.box(x-2,9,z-2,x+2,12,z+2,LEAF)
    def walkable(self,x,y,z):
        return (1<=x<95 and 1<=z<95 and 1<=y<38 and
                self.get(x,y-1,z)!=AIR and self.get(x,y,z)==AIR and self.get(x,y+1,z)==AIR)
    def reachable(self,spawn):
        start=tuple(int(v) for v in spawn[:3]); seen={start}; q=deque([start])
        assert self.walkable(*start)
        while q:
            x,y,z=q.popleft()
            for dx,dz in [(1,0),(-1,0),(0,1),(0,-1)]:
                for dy in [0,1,-1]:
                    p=(x+dx,y+dy,z+dz)
                    if p not in seen and self.walkable(*p):
                        seen.add(p); q.append(p)
        return seen

def foundry():
    b=Build(SAND)
    # An offset refinery tower and long approach ramp, not Rust's layout.
    b.box(34,3,33,58,3,58,SLATE)
    for x in [36,55]:
        for z in [36,55]: b.box(x,4,z,x+1,26,z+1,RUST)
    for y in [13,24]:
        b.box(36,y,36,56,y,56,SLATE)
        for z in [36,56]:b.box(36,y+1,z,56,y+1,z,RUST)
        b.box(42,y+1,36,45,y+2,36,AIR)
    for y in range(5,23,3):
        b.box(36,y,36,37,y,56,RUST)
        b.box(55,y,36,56,y,56,RUST)
    b.stair(42,16,21,block=RUST,width=4)
    for x,z,r in [(73,23,8),(75,47,6)]:
        b.cylinder(x,z,r,4,15,RUST,True)
        b.cylinder(x,z,r,15,15,SLATE)
        b.box(x-r,4,z-1,x-r+1,7,z+1,AIR)
        b.box(x,16,z,x,18,z,PIPE)
    b.box(57,10,47,75,11,48,PIPE)
    b.box(72,10,24,73,11,47,PIPE)
    b.shell(16,65,29,15,10,RUST,SLATE)
    b.box(28,14,68,29,29,69,SLATE)
    b.box(27,29,67,30,30,70,RUST)
    for x,z in [(20,25),(24,35),(61,68),(69,74),(48,77)]:b.crate(x,z)
    for z in range(12,88,12):b.box(8,4,z,8,6,z,PIPE)
    return b,(12.5,4.001,48.5,0.0,0.12),[(44,25,42),(24,4,71)]

def switchyard():
    b=Build(SLATE)
    for x in [18,38,58,78]:
        for z in range(8,89,3):b.box(x-2,3,z,x+2,3,z,WOOD)
        for xx in [x-1,x+1]:b.box(xx,3,6,xx,3,90,PIPE)
    for x,z,color in [(14,12,RUST),(34,60,WOOD),(54,14,BLUE),(74,61,RUST)]:
        b.shell(x,z,9,21,7,color,SLATE)
        for zz in range(z+2,z+20,4):
            for xx in [x,x+8]:b.box(xx,5,zz,xx,10,zz,SLATE)
        for xx in [x+1,x+7]:
            for zz in [z+3,z+17]:b.box(xx,4,zz,xx,5,zz,CORE)
    # Overhead signal gantry, reachable from the east stair.
    for x in [9,86]:
        for z in [45,51]:b.box(x,4,z,x+1,18,z+1,RUST)
    b.box(9,18,45,87,18,52,SLATE)
    for z in [45,52]:b.box(9,19,z,87,19,z,RUST)
    b.stair(83,31,15,width=3,block=SLATE)
    b.box(83,19,45,85,21,46,AIR)
    for x in [20,40,60,80]:b.box(x,16,45,x+1,17,45,BEACON)
    b.shell(30,78,37,13,10,WOOD,BLUE)
    for x,z in [(22,59),(46,25),(64,62),(27,31)]:b.crate(x,z,3)
    return b,(10.5,4.001,48.5,0.0,0.08),[(50,19,48),(84,4,23)]

def citadel():
    b=Build(TERRA)
    for z in [22,73]:b.box(22,4,z,73,11,z+2,SLATE)
    for x in [22,73]:b.box(x,4,22,x+2,11,75,SLATE)
    for x in [22,73]:b.box(x,4,45,x+2,8,51,AIR)
    for x in [23,73]:
        for z in [23,73]:
            b.cylinder(x,z,6,4,18,SLATE,True)
            b.cylinder(x,z,6,18,18,SLATE)
            for dx,dz in [(-5,0),(5,0),(0,-5),(0,5)]:b.box(x+dx,19,z+dz,x+dx,20,z+dz,RUST)
    b.box(7,3,46,88,3,50,SAND)
    b.box(46,3,8,50,3,88,SAND)
    b.shell(39,35,19,26,13,SLATE,SLATE)
    for x in range(39,58,3):
        for z in [35,60]:b.box(x,17,z,x+1,18,z,SLATE)
    b.stair(43,37,16,width=3)
    b.box(50,4,41,53,4,44,RUST)
    b.box(51,5,42,52,6,43,BEACON)
    # Outer ramp gives access to the south rampart.
    b.stair(61,65,8,width=4)
    for x,z in [(10,17),(12,78),(83,16),(83,80),(31,30),(65,64)]:b.tree(x,z)
    return b,(12.5,4.001,48.5,0.0,0.09),[(30,4,48),(63,12,73)]

def skybridge():
    b=Build(BLUE)
    for x,z in [(10,10),(68,10),(10,68),(68,68)]:
        for xx in [x+2,x+15]:
            for zz in [z+2,z+15]:b.box(xx,4,zz,xx+1,14,zz+1,SLATE)
        b.box(x,14,z,x+18,14,z+18,SLATE)
        b.shell(x+3,z+3,13,13,8,BLUE,SLATE,y=15)
        b.box(x+7,23,z+7,x+11,24,z+11,CORE)
    b.box(8,14,46,88,14,50,SLATE)
    b.box(46,14,8,50,14,88,SLATE)
    for z in [18,76]:b.box(19,14,z,77,14,z+3,SLATE)
    for x in [18,76]:b.box(x,14,19,x+3,14,77,SLATE)
    b.box(36,14,36,60,14,60,SLATE)
    for x in [37,58]:
        for z in [37,58]:b.box(x,15,z,x+1,28,z+1,BLUE)
    b.box(36,28,36,60,28,60,SLATE)
    b.box(42,29,42,54,29,54,CORE)
    b.box(46,30,46,50,32,50,BEACON)
    for x in [9,29,66,87]:
        for z in [46,50]:b.box(x,15,z,x,16,z,BEACON)
    b.stair(12,36,11,width=3)
    return b,(12.5,15.001,48.5,0.0,0.08),[(48,15,48),(48,15,20)]

def dune_outpost():
    b=Build(SAND)
    # Low wind-shaped perimeter dunes, leaving a broad connected inner yard.
    for cx,cz in [(3,6),(90,9),(7,88),(88,87)]:
        for z in range(max(0,cz-12),min(96,cz+13)):
            for x in range(max(0,cx-12),min(96,cx+13)):
                y=int(8-math.sqrt((x-cx)**2+(z-cz)**2)*0.55)
                if y>=4:b.box(x,4,z,x,y,z,SAND)
    for x,z,w,d in [(23,18,19,19),(61,62,18,18),(19,64,21,18),(64,18,18,21)]:
        b.shell(x,z,w,d,9,SAND,WOOD)
        for xx in [x,x+w-1]:b.box(xx,12,z,xx,13,z+d-1,RUST)
        b.stair(x+2,z-8,9,width=3,block=SAND)
    # Courtyard marker and asymmetric watchtower.
    b.box(44,3,44,53,3,53,BLUE)
    b.box(47,4,47,50,4,50,SAND)
    b.box(48,5,48,49,7,49,BEACON)
    b.shell(49,24,10,12,22,SAND,RUST)
    b.box(48,26,23,59,26,36,WOOD)
    for x in [48,59]:
        for z in [23,36]:b.box(x,27,z,x,29,z,RUST)
    b.stair(54,4,22,width=3,block=SAND)
    b.box(54,26,24,56,29,25,AIR)
    for x,z in [(30,44),(65,45)]:
        for xx in [x,x+10]:
            for zz in [z,z+10]:b.box(xx,4,zz,xx,10,zz,WOOD)
        b.box(x,11,z,x+10,11,z+10,RUST)
        b.crate(x+2,z+3,3)
    return b,(12.5,4.001,48.5,0.0,0.1),[(20,4,48),(45,4,70)]

BUILDERS=[('foundry','Foundry','Refinery tower, tanks and service yard',foundry),
          ('switchyard','Switchyard','Freight cars, rail lanes and signal gantry',switchyard),
          ('citadel','Citadel','Stone keep, gate passages and ramparts',citadel),
          ('skybridge','Skybridge','Elevated walkways and four satellite pavilions',skybridge),
          ('dune-outpost','Dune Outpost','Desert courtyards, rooftops and watchtower',dune_outpost)]

def main():
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true',help='verify reproducibility without writing assets')
    args=parser.parse_args()
    out=ROOT/'assets/maps'
    if not args.check:out.mkdir(parents=True,exist_ok=True)
    catalog=[]
    for i,(key,title,description,fn) in enumerate(BUILDERS):
        b,pose,checkpoints=fn()
        reachable=b.reachable(pose)
        for p in checkpoints:assert p in reachable,(key,'unreachable checkpoint',p)
        header=b'TCRF'+struct.pack('<HI3HBB5H',2,0x54430000+i,W,H,D,1,0,*[100]*5)
        data=header+b.data+b'\x01'+struct.pack('<5fBB',*pose,1,0)
        if args.check:assert (out/f'{key}.tcrf').read_bytes()==data, f'{key}: stale template'
        else:(out/f'{key}.tcrf').write_bytes(data)
        m={'id':key,'title':title,'description':description,'origin':'original','author':'janestreetshiller',
           'sha256':hashlib.sha256(data).hexdigest(),'spawn':pose,
           'structure_blocks':sum(c!=0 for c in b.data[4*W*D:]),
           'walkable_reachable_cells':len(reachable),'checked_landmarks':checkpoints}
        catalog.append(m)
    text=json.dumps(catalog,indent=2)+'\n'
    if args.check:assert (out/'catalog.json').read_text()==text, 'stale map catalog'
    else:(out/'catalog.json').write_text(text)
    print(json.dumps(catalog,indent=2))

if __name__=='__main__':main()

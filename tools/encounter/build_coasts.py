#!/usr/bin/env python3
"""Regenerate OceanCoasts.swift from the repository's public-domain Natural Earth land.

Run from any directory: python3 tools/encounter/build_coasts.py.
The clipping creates fill edges; a separately clipped shore retains only real coasts.
"""
import json, math
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
j=json.loads((ROOT / 'tools/plates/data/land-50m.json').read_text())
sx,sy=j['transform']['scale']; tx,ty=j['transform']['translate']
arcs=[]
for a in j['arcs']:
 x=y=0; pts=[]
 for dx,dy in a:
  x+=dx;y+=dy;pts.append((x*sx+tx,y*sy+ty))
 arcs.append(pts)
def arc(k): return arcs[k] if k>=0 else arcs[~k][::-1]
def stitch(aa):
 o=[]
 for k in aa:o.extend(arc(k) if not o else arc(k)[1:])
 return o
def simp(p,eps=.085):
 if len(p)<3:return p
 a,b=p[0],p[-1]; dx=b[0]-a[0];dy=b[1]-a[1]; n=dx*dx+dy*dy
 vals=[]
 for x,y in p[1:-1]:
  t=max(0,min(1,((x-a[0])*dx+(y-a[1])*dy)/n)) if n else 0
  vals.append(math.hypot(x-a[0]-t*dx,y-a[1]-t*dy))
 m=max(vals,default=0)
 if m<=eps:return [a,b]
 k=vals.index(m)+1
 return simp(p[:k+1],eps)[:-1]+simp(p[k:],eps)
bounds=[(0,27,True),(0,123,False),(1,-31,True),(1,27,False)]
def clip(p):
 for axis,v,greater in bounds:
  out=[]
  for a,b in zip(p[-1:]+p[:-1],p):
   ia=a[axis]>=v if greater else a[axis]<=v
   ib=b[axis]>=v if greater else b[axis]<=v
   if ia!=ib:
    t=(v-a[axis])/(b[axis]-a[axis]);out.append((a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1])))
   if ib:out.append(b)
  p=out
  if not p:break
 return p
def segment(a,b):
 t0,t1=0.,1.;dx=b[0]-a[0];dy=b[1]-a[1]
 for p,q in [(-dx,a[0]-27),(dx,123-a[0]),(-dy,a[1]+31),(dy,27-a[1])]:
  if p==0:
   if q<0:return None
  else:
   r=q/p
   if p<0:t0=max(t0,r)
   else:t1=min(t1,r)
   if t0>t1:return None
 return ((a[0]+dx*t0,a[1]+dy*t0),(a[0]+dx*t1,a[1]+dy*t1))
polys=[];shores=[]
for g in j['objects']['land']['geometries']:
 pp=g['arcs'] if g['type']=='MultiPolygon' else [g['arcs']]
 for polygon in pp:
  ring=stitch(polygon[0])
  if max(x for x,y in ring)<27 or min(x for x,y in ring)>123 or max(y for x,y in ring)<-31 or min(y for x,y in ring)>27:continue
  ring=simp(ring)
  cp=clip(ring)
  if len(cp)<3:continue
  area=abs(sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(cp,cp[1:]+cp[:1])))/2
  if area<.015:continue
  polys.append(cp)
  current=[]
  for a,b in zip(ring,ring[1:]):
   seg=segment(a,b)
   if not seg:
    if current:shores.append(current);current=[]
    continue
   if current and math.dist(current[-1],seg[0])>.001:shores.append(current);current=[]
   if not current:current=[seg[0]]
   current.append(seg[1])
  if current:shores.append(current)
def enc(rings):return '|'.join(';'.join(f'{x:.3f},{y:.3f}' for x,y in r) for r in rings)
p=ROOT / 'App/TerraIncognita/Experience/OceanCoasts.swift'
p.write_text('''import CoreGraphics

/// Natural Earth 1:50m land, public domain; decoded from tools/plates/data/land-50m.json.
/// Clipped to 27–123° E, 31° S–27° N and simplified by 0.085°. Coast strokes exclude
/// artificial clipping edges. Coordinates are decoded once, never on a display-link frame.
enum OceanCoasts {
    static let land = decode("""
'''+enc(polys)+'''
""")
    static let shores = decode("""
'''+enc(shores)+'''
""")
    private static func decode(_ value: String) -> [[CGPoint]] {
        value.split(separator: "|").map { ring in
            ring.split(separator: ";").compactMap { pair in
                let xy = pair.split(separator: ",").compactMap { Double($0) }
                guard xy.count == 2 else { return nil }
                return CGPoint(x: xy[0], y: xy[1])
            }
        }
    }
}
''')
print(len(polys),'polygons',sum(map(len,polys)),'points',p.stat().st_size,'bytes')

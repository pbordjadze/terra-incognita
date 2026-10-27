#!/usr/bin/env python3
"""Build the bounded river instrument from RiverATLAS v1.0 (CC BY 4.0).

Usage: PYTHONPATH=<pyshp,shapely> python3 tools/water/build.py <RiverATLAS-directory>
Fetch South American components from https://doi.org/10.6084/m9.figshare.9890531 .
No HydroRIVERS-only download is redistributed. RiverATLAS supplies geometry and
NEXT_DOWN topology; its derived, simplified network is not a water simulation.
The script keeps reaches with >=500 km² upstream area plus downstream closure,
merges chains without kept tributaries, simplifies to 0.006°, and rounds to 0.001°.
Never invents a downstream connection. Source checksums/settings go in the manifest;
constituent IDs go in reach-audit.json beside the external source files.
"""
import sys,json,hashlib,math
from pathlib import Path
from collections import defaultdict
import shapefile
from shapely.geometry import LineString
import numpy as np
from PIL import Image, ImageDraw, ImageFilter
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'Water'
FIELDS=['HYRIV_ID','NEXT_DOWN','MAIN_RIV','LENGTH_KM','DIST_DN_KM','UPLAND_SKM','ENDORHEIC','ORD_STRA']

def build(folder):
    rows={}; readers=[]; total=0
    for path in sorted(folder.glob('RiverATLAS_v10_sa*.shp')):
        reader=shapefile.Reader(str(path)); readers.append(reader)
        print('Reading',path.name,flush=True)
        for rec in reader.iterRecords(fields=FIELDS):
            p=rec.as_dict()
            total+=1
            if p['UPLAND_SKM']>=500: rows[p['HYRIV_ID']]=(p,reader,rec.oid)
    assert rows, 'Expected RiverATLAS South America shapefiles'
    keep=set(rows)
    missing={p['NEXT_DOWN'] for p,_,_ in rows.values()}-keep-{0}
    while missing:
        for reader in readers:
            for rec in reader.iterRecords(fields=FIELDS):
                p=rec.as_dict()
                if p['HYRIV_ID'] in missing: rows[p['HYRIV_ID']]=(p,reader,rec.oid)
        assert missing.issubset(rows), 'Missing downstream source reach'
        keep=set(rows); missing={p['NEXT_DOWN'] for p,_,_ in rows.values()}-keep-{0}
    # Preserve exact downstream links even if a local area attribute dips.
    for k in list(keep):
        while k and k in rows:
            keep.add(k); k=rows[k][0]['NEXT_DOWN']
            if k in keep: break
    parents=defaultdict(list)
    for k in keep:
        d=rows[k][0]['NEXT_DOWN']
        if d in keep: parents[d].append(k)
    starts=sorted(k for k in keep if len(parents[k])!=1)
    merged=[]; owner={}; allids=[]
    for k in starts:
        chain=[]; cur=k
        while cur in keep:
            chain.append(cur)
            d=rows[cur][0]['NEXT_DOWN']
            if d not in keep or len(parents[d])!=1: break
            cur=d
        coords=[]
        for q in chain:
            p,r,oid=rows[q]
            pts=list(r.shape(oid).points)
            if coords and math.dist(coords[-1],pts[0])>0.06:
                raise ValueError(f'Discontinuous downstream geometry {q}')
            coords.extend(pts if not coords else pts[1:])
        simple=list(LineString(coords).simplify(0.006,preserve_topology=False).coords)
        p=rows[chain[-1]][0]; length=sum(rows[q][0]['LENGTH_KM'] for q in chain)
        idx=len(merged)
        for q in chain: owner[q]=idx
        merged.append([p['NEXT_DOWN'],int(p['ORD_STRA']),round(length,1),int(p['MAIN_RIV']),int(p['ENDORHEIC']),[v for pt in simple for v in [round(pt[0],3),round(pt[1],3)]],chain[0],round(p['UPLAND_SKM'])])
        allids.append(chain)
    for m in merged: m[0]=owner.get(m[0],-1)
    assert len(owner)==len(keep),(len(owner),len(keep))
    # Chain-end distances are original data-derived lengths, not screen lengths.
    for i,m in enumerate(merged):
        seen=set(); j=i
        while j>=0:
            assert j not in seen, 'Cycle in derived network'; seen.add(j); j=merged[j][0]
    print('Reaches',total,'kept',len(keep),'merged',len(merged),flush=True)
    write('network.js','window.WATER_DATA=',{'version':1,'bounds':[-83,-57,-33,14],'rivers':merged})
    manifest={'source':'RiverATLAS v1.0','doi':'10.6084/m9.figshare.9890531','license':'CC BY 4.0','upstreamThresholdKm2':500,'simplificationDegrees':0.006,'roundingDegrees':0.001,'inputReaches':total,'keptReaches':len(keep),'mergedReaches':len(merged),'inputs':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(folder.glob('RiverATLAS_v10_sa*')) if p.suffix in ['.shp','.dbf','.shx']}}
    (OUT/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    # Audit mapping useful to validate aggregation against the source (not in app).
    (folder/'reach-audit.json').write_text(json.dumps(allids,separators=(',',':')))
    terrain()
    write('places.js','window.WATER_PLACES=',json.loads((ROOT/'tools/water/labels.json').read_text()))

def write(name,prefix,obj):
    (OUT/name).write_text(prefix+json.dumps(obj,separators=(',',':'))+';\n')

def terrain():
    # A quiet, original rendering from the existing GEBCO-derived globe textures.
    # Pixel centers remain lon/lat, equirectangular. No hand-painted mountains.
    b=(-84,-58,-32,15); size=(1560,2190)
    src=Image.open(ROOT/'Globe/terrain.png').convert('RGB')
    geo=Image.open(ROOT/'Globe/geo.png').convert('RGB')
    box=((b[0]+180)/360*src.width,(90-b[3])/180*src.height,(b[2]+180)/360*src.width,(90-b[1])/180*src.height)
    a=np.array(src.crop(box).resize(size,Image.Resampling.BILINEAR)).astype(float)
    g=np.array(geo.crop(box).resize(size,Image.Resampling.BILINEAR)).astype(float)
    land=np.clip((g[:,:,0]-126)/4,0,1)[:,:,None]
    elev=np.maximum(a[:,:,1]-128,0)*50
    shade=(a[:,:,0]-128)/128
    low=np.array([239,236,224]); high=np.array([190,183,194]); sea=np.array([225,234,235])
    z=np.clip(elev/4500,0,1)[:,:,None]
    color=low*(1-z)+high*z+shade[:,:,None]*18
    # Contour-like shading follows height, and is only aesthetic: not survey contours.
    contour=(np.abs((elev+100)%700-350)<35)&(elev>600)
    color[contour]-=6
    color=color*land+sea*(1-land)
    Image.fromarray(np.uint8(np.clip(color,0,255))).save(OUT/'terrain.jpg',quality=90,optimize=True)
    # Natural Earth land polygons, already in repo, for crisp coastlines.
    topo=json.loads((ROOT/'tools/plates/data/land-50m.json').read_text())
    polys=[]
    def arc(i):
        a=topo['arcs'][i if i>=0 else ~i]; x=y=0; out=[]
        for dx,dy in a:
            x+=dx;y+=dy
            out.append((x*topo['transform']['scale'][0]+topo['transform']['translate'][0],y*topo['transform']['scale'][1]+topo['transform']['translate'][1]))
        return out if i>=0 else out[::-1]
    from shapely.geometry import Polygon,box as rect
    def decode(rings):
        result=[]
        for aa in rings:
            pts=[]
            for i in aa: pts.extend(arc(i) if not pts else arc(i)[1:])
            result.append(pts)
        return result
    def each(g):
        if g['type']=='GeometryCollection':
            for child in g['geometries']: each(child)
        else:
            for rings in ([g['arcs']] if g['type']=='Polygon' else g['arcs']):
                rs=decode(rings); p=Polygon(rs[0],rs[1:])
                if not p.intersects(rect(-84,-58,-32,15)): continue
                p=p.simplify(.008)
                polys.append([[[round(x,3),round(y,3)] for x,y in p.exterior.coords]]+[[[round(x,3),round(y,3)] for x,y in ring.coords] for ring in p.interiors])
    for g in topo['objects'].values(): each(g)
    write('land.js','window.WATER_LAND=',polys)
    # Atlas city names from existing Natural Earth-derived country/capital data.
    data=json.loads((ROOT/'Globe/globe.json').read_text())
    print('globe keys',list(data))

if __name__=='__main__':
    OUT.mkdir(exist_ok=True); build(Path(sys.argv[1]))

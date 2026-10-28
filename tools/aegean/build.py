"""Prepare the offline leaf geometry. Inputs are cached upstream public-domain files.

python3 tools/aegean/build.py /path/to/cache
land.geojson: Natural Earth ne_10m_land.geojson
elevation.png, bathymetry.png: NASA Visible Earth's grayscale GEBCO 2008 imagery (see Leaf/SOURCES.md).
No network at build time. Geometry is a cylindrical local chart, not a navigation chart.
"""
import base64, json, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage
import contourpy

Image.MAX_IMAGE_PIXELS = None
ROOT = Path(__file__).resolve().parents[2]
CACHE = Path(sys.argv[1])
OUT = ROOT / 'Leaf' / 'data'
OUT.mkdir(parents=True, exist_ok=True)
BOX = [15, 35, 31, 45]
COS = float(np.cos(np.radians(37.5)))
S = 120

def xy(p):
    return [round((p[0]-24.6)*COS*S, 2), round((38-p[1])*S, 2)]

def clipped(ring):
    p = ring
    for axis, threshold, sign in [(0,BOX[0],1),(0,BOX[1],-1),(1,BOX[2],1),(1,BOX[3],-1)]:
        q = []
        for a,b in zip(p,p[1:]+p[:1]):
            ain = (a[axis]-threshold)*sign >= 0
            bin = (b[axis]-threshold)*sign >= 0
            if ain: q.append(a)
            if ain != bin:
                t = (threshold-a[axis])/(b[axis]-a[axis])
                q.append([a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1])])
        p = q
        if not p: break
    return [xy(pt) for pt in p]

polys = []
for f in json.loads((CACHE/'land.geojson').read_text())['features']:
    g=f['geometry']; ps = g['coordinates'] if g['type']=='MultiPolygon' else [g['coordinates']]
    for poly in ps:
        ring=poly[0]
        if max(p[0] for p in ring)<BOX[0] or min(p[0] for p in ring)>BOX[1] or max(p[1] for p in ring)<BOX[2] or min(p[1] for p in ring)>BOX[3]: continue
        rings=[p for p in (clipped(r) for r in poly) if len(p)>2]
        if rings: polys.append(rings)

# The sampled rasters are kept coarse and explicitly generalised. The coast is the vector coast.
W,H=700,630
lons=np.linspace(BOX[0],BOX[1],W); lats=np.linspace(BOX[3],BOX[2],H)
bounds=(BOX[0]*60+10800,5400-BOX[3]*60,BOX[1]*60+10800,5400-BOX[2]*60)
def crop(name):
    im=Image.open(CACHE/name)
    return np.asarray(im.crop(tuple(map(int,bounds))).convert('L').resize((W,H),Image.Resampling.BILINEAR)).astype(float)
z=ndimage.gaussian_filter(crop('elevation.png')*25, .8)
depth=ndimage.gaussian_filter((255-crop('bathymetry.png'))*43,1.2)
X=(lons-24.6)*COS*S; Y=(38-lats)*S

def contours(arr,levels):
    cg=contourpy.contour_generator(x=X,y=Y,z=arr,name='serial')
    result=[]
    for level in levels:
        for seg in cg.lines(level):
            seg=seg[::2] if len(seg)>12 else seg
            if len(seg)<5: continue
            result.append([level, np.around(seg,2).tolist()])
    return result

# Bounded hachures oriented down the actual gradient, with density from slope.
gy,gx=np.gradient(z); rng=np.random.default_rng(41)
hachures=[]
for j in range(3,H-3,3):
    for i in range(3,W-3,3):
        height=z[j,i]; dx=gx[j,i]; dy=gy[j,i]; slope=float(np.hypot(dx,dy))
        if height<170 or slope<22 or rng.random()>.55: continue
        x=X[i]+rng.uniform(-1,1); y=Y[j]+rng.uniform(-1,1)
        length=min(5.0,1.5+slope/65)
        hachures.append([round(x,2),round(y,2),round(dx/slope*length,2),round(dy/slope*length,2)])

# Occupancy and bathymetry for geographic hit testing and the sounding lens.
mask=Image.new('L',(W,H),0); d=ImageDraw.Draw(mask)
def ij(p): return ((p[0]-X[0])/(X[-1]-X[0])*(W-1),(p[1]-Y[0])/(Y[-1]-Y[0])*(H-1))
for poly in polys:
    d.polygon([ij(p) for p in poly[0]],fill=255)
    for hole in poly[1:]: d.polygon([ij(p) for p in hole],fill=0)
mw,mh=350,315
mask_bytes=mask.resize((mw,mh),Image.Resampling.NEAREST).tobytes()
depth_bytes=Image.fromarray(np.clip(depth/25,0,255).astype('uint8')).resize((mw,mh),Image.Resampling.BILINEAR).tobytes()
data={'projection':{'lon0':24.6,'lat0':38,'cos':COS,'scale':S},'bounds':[X[0],Y[0],X[-1]-X[0],Y[-1]-Y[0]],'polygons':polys,'relief':contours(z,[250,650,1100,1600,2200]),'soundings':contours(depth,[250,500,1000,1500,2000,3000,4000]),'hachures':hachures,'raster':{'width':mw,'height':mh,'land':base64.b64encode(mask_bytes).decode(),'depth':base64.b64encode(depth_bytes).decode()}}
(OUT/'chart.js').write_text('window.LeafChart='+json.dumps(data,separators=(',',':'))+';\n')
print(len(polys),'land polygons,',len(hachures),'hachures;',len((OUT/'chart.js').read_bytes()),'bytes')

"""Cannae's relief for the sand table.

Run: python3 prep.py <tiles dir> [preview.png]
<tiles dir> holds Terrarium z13 tiles x 4461..4465, y 3060..3064 named <x>_<y>.png, fetched with
  curl -o <x>_<y>.png https://s3.amazonaws.com/elevation-tiles-prod/terrarium/13/<x>/<y>.png
Writes relief.js beside this file: a 4:3 crop around Cannae (41.30 N 16.13 E) as metres, 192x144.
"""
import sys, os, json
import numpy as np
from PIL import Image

T = sys.argv[1]
M = np.zeros((1280, 1280))
for i, x in enumerate(range(4461, 4466)):
    for j, y in enumerate(range(3060, 3065)):
        a = np.asarray(Image.open(f'{T}/{x}_{y}.png').convert('RGB')).astype(float)
        M[j*256:(j+1)*256, i*256:(i+1)*256] = a[..., 0]*256 + a[..., 1] + a[..., 2]/256 - 32768

# Cannae at pixel (4463.047-4461)*256, (3062.176-3060)*256 = (524, 557); pixel ~14.4 m.
cx, cy = 524, 557
W, H = 640, 480                    # ~9.2 km x 6.9 km
x0, y0 = int(os.environ.get('CX', cx)) - W//2 + int(os.environ.get('DX', 0)), cy - H//2 + int(os.environ.get('DY', 0))
C = M[y0:y0+H, x0:x0+W]
img = Image.fromarray(C.astype(np.float32), mode='F').resize((192, 144), Image.BILINEAR)
R = np.asarray(img)
print('min', R.min(), 'max', R.max())
out = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'relief.js')
with open(out, 'w') as f:
    f.write('// Cannae relief, metres, 192x144, north up; built by prep.py from Terrarium (Mapzen/AWS) z13\n')
    f.write('window.RELIEF={w:192,h:144,m:' + json.dumps([round(float(v), 1) for v in R.ravel()]) + '};\n')
if len(sys.argv) > 2:
    v = (R - R.min()) / (R.max() - R.min())
    Image.fromarray((v**0.7*255).astype(np.uint8)).resize((768, 576)).save(sys.argv[2])

"""Cannae's real ground for the ink-vellum spike: Terrarium tiles -> ground.png.

    python3 build_ground.py <tiles dir>      # tiles 13/4461..4465/3060..3064, fetched with:
    curl -sS -o <dir>/13_X_Y.png https://s3.amazonaws.com/elevation-tiles-prod/terrarium/13/X/Y.png

ground.png (512x384, RGB): R = elevation (normalised over the crop), G = log flow accumulation
(D8 on a priority-flooded DEM: the Aufidus/Ofanto and its gullies), B = the Ofanto alone, softened. The crop is rotated
55 degrees so the Ofanto runs up the page's left third, the Roman line facing down with the river on
its right, as at Cannae; it spans about 11 km.
"""
import sys, heapq
import numpy as np
from PIL import Image

T = sys.argv[1].rstrip('/') + '/'
M = np.zeros((1280, 1280))
for i, x in enumerate(range(4461, 4466)):
    for j, y in enumerate(range(3060, 3065)):
        a = np.asarray(Image.open(T + f'13_{x}_{y}.png').convert('RGB')).astype(float)
        M[j*256:(j+1)*256, i*256:(i+1)*256] = a[..., 0]*256 + a[..., 1] + a[..., 2]/256 - 32768
H = M.reshape(640, 2, 640, 2).mean((1, 3))
n = 640
filled = H.copy(); done = np.zeros_like(H, bool); pq = []
for i in range(n):
    for (a, b) in ((i, 0), (i, n-1), (0, i), (n-1, i)):
        if not done[a, b] and H[a, b] < 2.0:   # drain only to the sea (the coast, top right)
            done[a, b] = True; heapq.heappush(pq, (H[a, b], a, b))
nb = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)]
order = []
while pq:
    h, a, b = heapq.heappop(pq); order.append((a, b))
    for da, db in nb:
        x, y = a+da, b+db
        if 0 <= x < n and 0 <= y < n and not done[x, y]:
            done[x, y] = True; filled[x, y] = max(H[x, y], h+1e-3); heapq.heappush(pq, (filled[x, y], x, y))
acc = np.ones_like(H)
inlet = 460 + int(H[460:560, 1].argmin())     # the Ofanto enters on the west edge with all its catchment
acc[inlet, 1] += 4e5
for a, b in reversed(order):
    best = None; bd = 0
    for da, db in nb:
        x, y = a+da, b+db
        if 0 <= x < n and 0 <= y < n:
            d = (filled[a, b]-filled[x, y]) / (1.414 if da and db else 1)
            if d > bd: bd = d; best = (x, y)
    if best: acc[best] += acc[a, b]
la = np.log(acc) / np.log(acc.max())
if len(sys.argv) > 2:
    Image.fromarray((np.clip(la,0,1)*255).astype(np.uint8)).save(sys.argv[2])

def rot(arr, cx, cy):
    im = Image.fromarray(arr.astype(np.float32), mode='F')
    im = im.rotate(55, resample=Image.BILINEAR, center=(cx, cy))
    w, h = 400, 300
    return np.asarray(im.crop((int(cx-w/2), int(cy-h/2), int(cx+w/2), int(cy+h/2))).resize((512, 384), Image.BILINEAR))

cx, cy = float(sys.argv[3]) if len(sys.argv) > 3 else 283, float(sys.argv[4]) if len(sys.argv) > 4 else 326
# the Ofanto alone (all the inflow), widened and softened so it survives the turn and reads as a wet line
rv = (acc > 1e5).astype(float)
for _ in range(2):
    rv = np.maximum.reduce([rv, np.roll(rv, 1, 0), np.roll(rv, -1, 0), np.roll(rv, 1, 1), np.roll(rv, -1, 1)])
for _ in range(10):
    rv = (rv*4 + np.roll(rv, 1, 0) + np.roll(rv, -1, 0) + np.roll(rv, 1, 1) + np.roll(rv, -1, 1)) / 8
e = rot(H, cx, cy); f = rot(la, cx, cy); r = rot(rv, cx, cy)
e = (e - e.min()) / (e.max() - e.min())
out = np.dstack([e*255, f.clip(0, 1)*255, r.clip(0, 1)*255]).astype(np.uint8)
Image.fromarray(out).save('ground.png')
print('ground.png', e.shape)
import base64
open('ground.js', 'w').write("// built by build_ground.py from ground.png (see its header); never hand-edit\nwindow.GROUND = 'data:image/png;base64,"
                             + base64.b64encode(open('ground.png', 'rb').read()).decode() + "';\n")
print('ground.js')

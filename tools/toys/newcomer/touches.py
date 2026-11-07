#!/usr/bin/env python3
"""Fingertips on a newcomer's shots (painting-company row 246): every touch that began in the 1.5 s before
a shot is drawn on it as a 44-point ring (white, dark-edged; a drag's path, a pinch's two fingers), so a
tester sees where its finger really landed and a miss is told from a fault. try.sh runs it.

    python3 tools/toys/newcomer/touches.py <plan.json> <out-dir> <ppp>
"""
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

plan = json.loads(Path(sys.argv[1]).read_text()); out = Path(sys.argv[2]); k = float(sys.argv[3])
shots = [(s, f'{s:g}') if not isinstance(s, dict) else (s['t'], s.get('name', f"{s['t']:g}")) for s in plan.get('shots', [])]

def pts(e):
    for key in ('tap', 'hold', 'down', 'move', 'up'):
        if key in e and isinstance(e[key], list) and len(e[key]) == 2 and all(isinstance(v, (int, float)) for v in e[key]):
            return [tuple(e[key])], None
    if 'drag' in e:
        a, b = e['drag']
        if all(isinstance(p, list) for p in (a, b)):
            return [tuple(a), tuple(b)], 'path'
    if 'pinch' in e:
        a, b = e['pinch']; s = e.get('to', 1.0)
        c = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        far = lambda p: (c[0] + (p[0] - c[0]) * s, c[1] + (p[1] - c[1]) * s)
        return [tuple(a), tuple(b), far(a), far(b)], 'pinch'
    return [], None

for t, name in shots:
    for f in (out / f'{name}.png', out / f'{name}.shell.png'):
        if not f.exists():
            continue
        im = Image.open(f).convert('RGB'); d = ImageDraw.Draw(im)
        for e in plan.get('events', []):
            if not (t - 1.5 <= e.get('t', -9) <= t):
                continue
            p, kind = pts(e)
            P = [(x * k, y * k) for x, y in p]
            if kind == 'path':
                d.line(P, fill=(30, 30, 30), width=int(5 * k)); d.line(P, fill=(255, 255, 255), width=int(2 * k))
            if kind == 'pinch':
                d.line([P[0], P[2]], fill=(255, 255, 255), width=int(2 * k)); d.line([P[1], P[3]], fill=(255, 255, 255), width=int(2 * k))
                P = P[:2]
            for x, y in P[:1] if kind == 'path' else P:
                r = 22 * k
                d.ellipse([x - r, y - r, x + r, y + r], outline=(20, 20, 20), width=int(3 * k))
                d.ellipse([x - r + 2 * k, y - r + 2 * k, x + r - 2 * k, y + r - 2 * k], outline=(255, 255, 255), width=int(1.5 * k))
        im.save(f)

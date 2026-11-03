#!/usr/bin/env python3
"""Shrink out/ for the commit (run after `node render.cjs all`): python3 slim.py

Keeps out/film.mp4 (the good plan, then the bad one, 10 s each), takes the moments and the two
contact sheets back out of it at 1x, and drops the 2x PNGs and the per-plan films.
"""
import os, subprocess
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'out')
film = os.path.join(OUT, 'film.mp4')
for f in os.listdir(OUT):
    if f != 'film.mp4' and not f.startswith('.'): os.remove(os.path.join(OUT, f))
ff = lambda *a: subprocess.run(['ffmpeg', '-y', '-loglevel', 'error', '-i', film, *a], check=True)
for name, n in [('good-070', 70), ('good-130', 130), ('good-190', 190), ('bad-190', 490)]:
    ff('-vf', f'select=eq(n\\,{n})', '-frames:v', '1', os.path.join(OUT, name + '.png'))
ff('-vf', "select='lt(n\\,300)*not(mod(n-8\\,36))',scale=458:344,tile=3x3", '-frames:v', '1', os.path.join(OUT, 'contact.png'))
ff('-vf', "select='gte(n\\,308)*not(mod(n-308\\,36))',scale=458:344,tile=3x3", '-frames:v', '1', os.path.join(OUT, 'bad-contact.png'))
for f in sorted(os.listdir(OUT)): print(f, os.path.getsize(os.path.join(OUT, f)))

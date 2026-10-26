#!/usr/bin/env python3
"""Checks the bundled looks before a Mac builds them: what the player would trap on or show wrong.

    python3 ci/check_atlas.py [Atlas]

For every look in looks.json: its plates.json has every field the app's decoder requires
(App/TerraIncognita/Atlas/Plates.swift: a missing one drops the look from the app without a word),
starts and ends on a stop (not a pass plate), every plate has a positive size and layers, every
plate but the last a child rectangle inside it, every stop a caption title, every layer file and
caption font exists, and no file in the look's folder goes unused (the IPA carries every byte of
Atlas/). Prints each look's size.
"""
import json, os, sys

root = sys.argv[1] if len(sys.argv) > 1 else 'Atlas'
errors = []


def err(msg):
    errors.append(msg)
    print(f'error: {msg}')


def need(where, obj, fields):
    """Reports the fields of obj (name: type) that the decoder requires and finds missing or wrong."""
    if not isinstance(obj, dict):
        err(f'{where}: not an object')
        return False
    ok = True
    for name, kind in fields.items():
        if not isinstance(obj.get(name), kind) or isinstance(obj.get(name), bool) and kind is not bool:
            err(f'{where}: {name} missing or not {kind}')
            ok = False
    return ok


NUM = (int, float)


looks = json.load(open(os.path.join(root, 'looks.json')))
if not looks:
    err('looks.json lists no looks')
total = 0
for look in looks:
    if not need('looks.json entry', look, {'id': str, 'name': str, 'plates': str}):
        continue
    lid = look['id']
    path = os.path.join(root, look.get('plates', ''))
    try:
        data = json.load(open(path))
    except (OSError, ValueError) as e:
        err(f'{lid}: {path}: {e}')
        continue
    folder = os.path.dirname(path)
    used = {'plates.json'}
    need(lid, data, {'name': str, 'plates': list})
    style = data.get('style')
    if style is not None and need(f'{lid} style', style, {}):
        for part in ('eyebrow', 'title', 'text'):
            if style.get(part) is not None:
                need(f'{lid} style.{part}', style[part], {'font': str, 'size': NUM, 'color': str})
    plates = data.get('plates') or []
    if not plates:
        err(f'{lid}: no plates')
        continue
    if plates[0].get('pass') or plates[-1].get('pass'):
        err(f'{lid}: the first and last plates must be stops')
    for i, p in enumerate(plates):
        pid = f"{lid}/{p.get('id', i)}"
        need(pid, p, {'id': str, 'size': list, 'layers': list})
        need(f'{pid} caption', p.get('caption'), {'eyebrow': str, 'title': str, 'text': str})
        size = p.get('size') or []
        if len(size) != 2 or min(size) <= 0:
            err(f'{pid}: size {size}')
            continue
        if not p.get('layers'):
            err(f'{pid}: no layers')
        for layer in p.get('layers') or []:
            if not need(f'{pid} layer', layer, {'file': str, 'depth': NUM}):
                continue
            if layer.get('motion') is not None:
                need(f'{pid} {layer["file"]} motion', layer['motion'], {'kind': str})
            f = layer['file']
            used.add(f)
            if not os.path.isfile(os.path.join(folder, f)):
                err(f'{pid}: missing {f}')
        if not p.get('pass') and not (p.get('caption') or {}).get('title'):
            err(f'{pid}: a stop without a caption')
        if i < len(plates) - 1:
            c = p.get('child') or []
            if len(c) != 4 or c[2] <= 0 or c[3] <= 0:
                err(f'{pid}: child {c}')
            elif c[0] < -1 or c[1] < -1 or c[0] + c[2] > size[0] + 1 or c[1] + c[3] > size[1] + 1:
                err(f'{pid}: child {c} reaches outside the plate')
    for f in (data.get('style') or {}).get('fonts') or []:
        used.add(f)
        if not os.path.isfile(os.path.join(folder, f)):
            err(f'{lid}: missing font {f}')
    size = 0
    for dirpath, _, files in os.walk(folder):
        for name in files:
            rel = os.path.relpath(os.path.join(dirpath, name), folder)
            size += os.path.getsize(os.path.join(dirpath, name))
            # the fonts' licences travel with them
            if rel not in used and not (rel.startswith('fonts/') and name.upper().startswith(('OFL', 'LICEN'))):
                err(f'{lid}: {rel} is in the bundle but no plate uses it')
    total += size
    print(f'{lid}: {len(plates)} plates, {size / 1e6:.1f} MB')
print(f'Atlas: {len(looks)} looks, {total / 1e6:.1f} MB')
sys.exit(1 if errors else 0)

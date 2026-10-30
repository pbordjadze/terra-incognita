#!/usr/bin/env python3
"""What's new, off the Mac (painting-company row 187; docs/whatsnew.md): the queue a build would play,
checked and seen before the IPA exists.

    python3 tools/whatsnew/whatsnew.py list                 the pieces as the app orders them (newest first), with their start
    python3 tools/whatsnew/whatsnew.py validate             every piece sound (the app's WhatsNew.json and each pack's whatsNew);
                                                            ids unique, `start.run` names prototypes that exist; exit 1 on an error
    python3 tools/whatsnew/whatsnew.py preview <out dir> [--only id,id] [--device ipad|iphone]
                                                            each pack piece's start photographed in the desktop preview
                                                            (tools/packart/moments.cjs): <out dir>/<id>.png; native pieces are listed, not drawn

The pieces come from App/TerraIncognita/WhatsNew/WhatsNew.json (native work) and every pack's
prototype.json `whatsNew` (a pack's entry wins over a central one of the same id; the app does the same,
WhatsNewQueue.merged). Native prototype ids are read from Catalog.swift and QuietGlobe.swift.
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools' / 'playground'))
import playground as pg  # noqa: E402

CENTRAL = ROOT / 'App' / 'TerraIncognita' / 'WhatsNew' / 'WhatsNew.json'
NATIVE_FILES = [ROOT / 'App' / 'TerraIncognita' / 'Playground' / 'Catalog.swift',
                ROOT / 'App' / 'TerraIncognita' / 'Quiet' / 'QuietGlobe.swift']


def native_ids():
    ids = set()
    for f in NATIVE_FILES:
        if f.exists():
            ids.update(re.findall(r'\bid:\s*"([a-z0-9-]+)"', f.read_text()))
    return ids


def pack_pieces(rep):
    """(pack id, piece) for every pack with a whatsNew entry, its id and title filled in as the app does."""
    out = []
    for folder in sorted(pg.prototype_dirs()):
        try:
            m = pg.load_json(folder / 'prototype.json')
        except Exception as e:  # noqa: BLE001
            rep.error(pg.display(folder / 'prototype.json'), f'unreadable: {e}')
            continue
        if not isinstance(m, dict) or 'whatsNew' not in m:
            continue
        piece = m['whatsNew']
        if isinstance(piece, dict):
            piece = dict(piece)
            piece['id'] = folder.name
            piece.setdefault('title', m.get('title', folder.name))
            piece.setdefault('start', {'run': [folder.name]})
            piece['_from'] = 'pack'
        out.append((folder.name, piece, pg.display(folder / 'prototype.json')))
    return out


def central_pieces(rep):
    if not CENTRAL.exists():
        rep.error(pg.display(CENTRAL), 'missing')
        return []
    try:
        data = pg.load_json(CENTRAL)
    except Exception as e:  # noqa: BLE001
        rep.error(pg.display(CENTRAL), f'unreadable: {e}')
        return []
    pieces = data.get('pieces') if isinstance(data, dict) else None
    if not isinstance(pieces, list):
        rep.error(pg.display(CENTRAL), 'must be { "pieces": [ ... ] }')
        return []
    out = []
    for p in pieces:
        if isinstance(p, dict):
            p = dict(p)
            p['_from'] = 'central'
        out.append(p)
    return out


def merged(central, packs):
    """As the app orders a build's pieces: the central list first, a pack's entry replacing a central one
    of the same id, then the packs' own; then the newest first."""
    own = {pid: piece for pid, piece, _ in packs if isinstance(piece, dict)}
    all_pieces = [own.get(p.get('id'), p) if isinstance(p, dict) else p for p in central]
    seen = {p.get('id') for p in all_pieces if isinstance(p, dict)}
    for pid, piece, _ in packs:
        if pid not in seen:
            all_pieces.append(piece)
            seen.add(pid)
    indexed = list(enumerate(all_pieces))
    indexed.sort(key=lambda ip: (-(int(ip[1].get('since', '0').replace('-', '') or 0) if isinstance(ip[1], dict) and isinstance(ip[1].get('since'), str) and ip[1]['since'].replace('-', '').isdigit() else 0), ip[0]))
    return [p for _, p in indexed]


def gather(rep):
    packs = pack_pieces(rep)
    central = central_pieces(rep)
    prototypes = native_ids() | {f.name for f in pg.prototype_dirs()}
    where = pg.display(CENTRAL)
    seen = {}
    def plain(p):
        return {k: v for k, v in p.items() if k != '_from'} if isinstance(p, dict) else p
    for p in central:
        pg.check_whats_new(rep, where, plain(p), prototype_ids=prototypes)
        if isinstance(p, dict) and isinstance(p.get('id'), str):
            if p['id'] in seen:
                rep.error(where, f'pieces[{p["id"]}] is listed twice')
            seen[p['id']] = 'central'
    for pid, piece, pwhere in packs:
        pg.check_whats_new(rep, pwhere, plain(piece), pack_id=pid, prototype_ids=prototypes)
        if seen.get(pid) == 'central':
            rep.warn(where, f'pieces[{pid}] is also the pack\'s own whatsNew, which wins; drop the central one')
    return merged(central, packs)


def describe_start(start):
    if not isinstance(start, dict):
        return '(the pack itself)'
    parts = []
    if start.get('run'):
        parts.append('run ' + ', '.join(start['run']))
    if 'camera' in start and isinstance(start['camera'], dict):
        c = start['camera']
        parts.append(f'camera {c.get("lon")},{c.get("lat")},{c.get("zoom")}')
    for k in ('date', 'playing', 'wikipedia', 'clouds', 'select', 'open'):
        if k in start:
            parts.append(f'{k} {start[k]}')
    return '; '.join(parts) or '(the globe as it is)'


def cmd_list(args):
    rep = pg.Report()
    pieces = gather(rep)
    for i, p in enumerate(pieces, 1):
        if not isinstance(p, dict):
            continue
        print(f'{i:2}. {p.get("id")}  ·  {p.get("title")}  ·  since {p.get("since")} v{p.get("version", "1")}  ·  {p.get("_from")}')
        if p.get('what'):
            print(f'      {p["what"]}')
        if p.get('ask'):
            print(f'      ask: {p["ask"]}')
        print(f'      start: {describe_start(p.get("start"))}')
    print(f'{len(pieces)} pieces; {rep.errors} errors, {rep.warnings} warnings')
    rep.print(sys.stderr)
    return 1 if rep.errors else 0


def cmd_validate(args):
    rep = pg.Report()
    pieces = gather(rep)
    rep.print()
    print(f'Checked {len(pieces)} What\'s new pieces: {rep.errors} errors, {rep.warnings} warnings.')
    return 1 if rep.errors else 0


def cmd_preview(args):
    rep = pg.Report()
    pieces = gather(rep)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    only = set(args.only.split(',')) if args.only else None
    packs = {f.name for f in pg.prototype_dirs()}
    rc = 0
    for p in pieces:
        if not isinstance(p, dict) or (only and p.get('id') not in only):
            continue
        start = p.get('start') if isinstance(p.get('start'), dict) else {}
        run = start.get('run') or ([p['id']] if p.get('_from') == 'pack' else [])
        pack_run = [r for r in run if r in packs]
        if not pack_run:
            print(f'{p.get("id")}: native ({describe_start(start)}); the simulator shows it (ci/sim_shots.sh, WhatsNewTests)')
            continue
        plan = []
        if 'camera' in start and isinstance(start['camera'], dict):
            c = start['camera']
            plan.append({'at': [c.get('lon'), c.get('lat'), c.get('zoom', 1)]})
        plan += [{'wait': args.wait}, {'shot': p['id']}]
        cmd = ['node', str(ROOT / 'tools' / 'packart' / 'moments.cjs'), pack_run[-1], str(out), json.dumps(plan), '--device', args.device]
        print(f'{p.get("id")}: {" ".join(cmd[1:4])} …')
        r = subprocess.run(cmd, cwd=ROOT)
        if r.returncode:
            rc = 1
    return rc


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    sub = ap.add_subparsers(dest='cmd', required=True)
    sub.add_parser('list', help='the pieces as the app orders them')
    sub.add_parser('validate', help='every piece sound; exit 1 on an error')
    pv = sub.add_parser('preview', help='each pack piece\'s start photographed in the desktop preview')
    pv.add_argument('out')
    pv.add_argument('--only', default='')
    pv.add_argument('--device', default='ipad', choices=['ipad', 'iphone'])
    pv.add_argument('--wait', type=float, default=4)
    args = ap.parse_args(argv)
    return {'list': cmd_list, 'validate': cmd_validate, 'preview': cmd_preview}[args.cmd](args)


if __name__ == '__main__':
    sys.exit(main())

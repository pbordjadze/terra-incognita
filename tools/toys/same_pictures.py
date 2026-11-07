#!/usr/bin/env python3
"""Plays every toy's plans (and Adrift's scenes) in two checkouts and says whether their pictures are the
same, pixel for pixel, so a dependency bump (wgpu, Rust) can be shown to change nothing a person sees.

    python3 tools/toys/same_pictures.py play <checkout> <out> [toy ...]   # every plan of each toy (all by default)
    python3 tools/toys/same_pictures.py adrift <checkout> <out>            # Adrift's scenes, iPhone-sized
    python3 tools/toys/same_pictures.py compare <out-a> <out-b>            # every PNG in both, by name

e.g. the bump's base in a worktree (git worktree add ../base origin/main), both played, then compared:
    python3 tools/toys/same_pictures.py play ../base /tmp/a & python3 tools/toys/same_pictures.py play . /tmp/b
    python3 tools/toys/same_pictures.py compare /tmp/a /tmp/b

Needs what the toys' CI check needs (Mesa's software Vulkan: mesa-vulkan-drivers; the toolchain the
checkout names). A frame that differs is listed with how many pixels differ and by how much at most; a
GPU's rounding can move a channel by one, which is reported but called the same (`--exact` refuses it).
Mold's slime is not deterministic (two runs of one build differ: its agents' deposits race on the GPU), so
its frames, and Terra's of it, are judged by eye. Played single-threaded per toy; the two checkouts can run at once (separate target directories).
"""
import json
import os
import subprocess
import sys
from pathlib import Path


def toys(checkout):
    members = json.loads(subprocess.run(['cargo', 'metadata', '--no-deps', '--format-version', '1'], cwd=checkout / 'toys',
                                        capture_output=True, text=True, check=True).stdout)['packages']
    return sorted(p['name'] for p in members if (checkout / 'toys' / p['name'] / 'plans').is_dir())


def play(checkout, out, only):
    checkout, out = Path(checkout).resolve(), Path(out).resolve()
    env = dict(os.environ, KIT_TAP_CHECK='1')
    for toy in only or toys(checkout):
        subprocess.run([sys.executable, 'tools/toys/assets.py', toy, '--link'], cwd=checkout, check=True, stdout=subprocess.DEVNULL)
        subprocess.run(['cargo', 'build', '-q', '--release', '-p', toy, '--bin', f'{toy}-check'], cwd=checkout / 'toys', check=True)
        for plan in sorted((checkout / 'toys' / toy / 'plans').glob('*.json')):
            dest = out / toy / plan.stem
            dest.mkdir(parents=True, exist_ok=True)
            r = subprocess.run(['cargo', 'run', '-q', '--release', '-p', toy, '--bin', f'{toy}-check', '--', 'play', str(plan), str(dest)],
                               cwd=checkout / 'toys', env=env, capture_output=True, text=True, check=False)
            (dest / 'play.log').write_text(r.stdout + r.stderr)
            print(f'{toy}/{plan.stem}: {"ok" if r.returncode == 0 else f"exit {r.returncode}"}', flush=True)


def adrift(checkout, out):
    checkout, out = Path(checkout).resolve(), Path(out).resolve()
    subprocess.run([sys.executable, 'tools/adrift/assets.py', 'adrift/build/assets', '--link'], cwd=checkout, check=True, stdout=subprocess.DEVNULL)
    subprocess.run(['cargo', 'build', '-q', '--release', '-p', 'adrift-check'], cwd=checkout / 'adrift', check=True)
    out.mkdir(parents=True, exist_ok=True)
    for scene in sorted((checkout / 'adrift/crates/adrift-check/scenes').glob('*.json')):
        r = subprocess.run(['cargo', 'run', '-q', '--release', '-p', 'adrift-check', '--', 'render', str(scene), str(out / f'{scene.stem}.png'),
                            '--size', '1179x2556', '--ppp', '3'], cwd=checkout / 'adrift', capture_output=True, text=True, check=False)
        print(f'adrift/{scene.stem}: {"ok" if r.returncode == 0 else f"exit {r.returncode}: {r.stderr[-300:]}"}', flush=True)


def compare(a, b, exact):
    import numpy as np
    from PIL import Image
    a, b = Path(a), Path(b)
    names = sorted({p.relative_to(a) for p in a.rglob('*.png')} | {p.relative_to(b) for p in b.rglob('*.png')})
    same = off_by_one = 0
    bad = []
    unreadable = []  # (a frame cut short, say, by a full disk: play that plan again)
    for n in names:
        if not (a / n).exists() or not (b / n).exists():
            bad.append(f'{n}: only in {"b" if not (a / n).exists() else "a"}')
            continue
        try:
            x, y = np.asarray(Image.open(a / n)).astype(int), np.asarray(Image.open(b / n)).astype(int)
        except OSError as e:
            unreadable.append(f'{n}: {e}')
            continue
        if x.shape != y.shape:
            bad.append(f'{n}: {x.shape} vs {y.shape}')
            continue
        d = np.abs(x - y)
        if d.max() == 0:
            same += 1
        elif d.max() <= 1 and not exact:
            off_by_one += 1
        else:
            px = int((d.max(axis=-1) > (0 if exact else 1)).sum()) if d.ndim == 3 else int((d > 0).sum())
            bad.append(f'{n}: {px} pixels differ, by up to {d.max()}')
    print(f'{len(names)} pictures: {same} identical, {off_by_one} within one step of a channel, {len(bad)} different')
    for line in bad:
        print('  ' + line)
    if unreadable:
        print(f'{len(unreadable)} unreadable:')
        for line in unreadable:
            print('  ' + line)
    return 1 if bad else 0


if __name__ == '__main__':
    args = sys.argv[1:]
    exact = '--exact' in args
    args = [x for x in args if x != '--exact']
    if len(args) >= 3 and args[0] == 'play':
        play(args[1], args[2], args[3:])
    elif len(args) == 3 and args[0] == 'adrift':
        adrift(args[1], args[2])
    elif len(args) == 3 and args[0] == 'compare':
        sys.exit(compare(args[1], args[2], exact))
    else:
        sys.exit(__doc__)

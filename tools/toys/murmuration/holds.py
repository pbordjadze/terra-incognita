#!/usr/bin/env python3
"""How much flock a hand sees while it flies: plays a Murmuration plan headless and counts, in every
drawn frame (its shots and film) while the eye is down among the birds, the birds on the glass and
how many of them stand against the sky rather than the land.

    python3 tools/toys/murmuration/holds.py <plan.json> [<out-dir>] [--min 150] [--near 40] [--size WxH --ppp n]

Prints the share of flying frames with fewer than --min birds on screen (the critic's check:
holding a touch must never lose the murmuration), the median count, and, since a count of specks
at the horizon flatters an empty sky, the birds near you: those whose wingspan is 12 points or more
on the glass (fewer than --near is a frame with little flock round you); the worst moments by that. Needs
the toy's check built (cargo, from toys/); MURMURATION_COUNT makes the toy print its counts.
"""
import os, subprocess, sys, statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def main():
    args = sys.argv[1:]
    def flag(k, d=None):
        if k in args:
            i = args.index(k); v = args[i + 1]; del args[i:i + 2]; return v
        return d
    least = int(flag('--min', '150'))
    near_least = int(flag('--near', '40'))
    size, ppp = flag('--size'), flag('--ppp')
    if not args:
        sys.exit(__doc__)
    plan = Path(args[0]).resolve()
    out = Path(args[1]).resolve() if len(args) > 1 else ROOT / 'toys/build/murmuration/out' / (plan.stem + '-holds')
    cmd = ['cargo', 'run', '-q', '--release', '-p', 'murmuration', '--bin', 'murmuration-check', '--', 'play', str(plan), str(out)]
    if size: cmd += ['--size', size]
    if ppp: cmd += ['--ppp', ppp]
    env = dict(os.environ, MURMURATION_COUNT='1')
    run = subprocess.run(cmd, cwd=ROOT / 'toys', env=env, capture_output=True, text=True)
    if run.returncode:
        sys.exit(run.stderr)
    frames = {}
    for line in run.stdout.splitlines():
        if line.startswith('visible '):
            f = line.split()
            t, n, sky, near = float(f[1]), int(f[2]), int(f[3]), float(f[5])
            big = int(f[7]) if len(f) > 7 else n
            frames[t] = (n, sky, near, big)
    flying = [(t, n, s, b) for t, (n, s, near, b) in sorted(frames.items()) if near > 0.5]
    if not flying:
        print('no flying frames'); return
    few = [f for f in flying if f[1] < least]
    near_few = [f for f in flying if f[3] < near_least]
    print(f'{plan.name}: {len(flying)} flying frames; {100 * len(few) / len(flying):.0f}% with fewer than {least} birds; '
          f'median {statistics.median(f[1] for f in flying):.0f} on screen, {statistics.median(f[2] for f in flying):.0f} against the sky')
    print(f'  near birds (a wingspan of 12 pt or more on the glass): median {statistics.median(f[3] for f in flying):.0f}; '
          f'{100 * len(near_few) / len(flying):.0f}% of frames with fewer than {near_least}')
    for t, n, s, b in sorted(flying, key=lambda f: f[3])[:5]:
        print(f'  t={t:6.2f}  {n} birds, {s} against the sky, {b} near')


if __name__ == '__main__':
    main()

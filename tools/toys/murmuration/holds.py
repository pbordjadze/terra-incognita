#!/usr/bin/env python3
"""How much flock a hand sees while it flies: plays a Murmuration plan headless and counts, in every
drawn frame (its shots and film) while the eye is down among the birds, the birds on the glass and
how many of them stand against the sky rather than the land.

    python3 tools/toys/murmuration/holds.py <plan.json> [<out-dir>] [--min 150] [--size WxH --ppp n]

Prints the share of flying frames with fewer than --min birds on screen (the critic's check:
holding a touch must never lose the murmuration), the median count, and the worst moments. Needs
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
    size, ppp = flag('--size'), flag('--ppp')
    if not args:
        sys.exit(__doc__)
    plan = Path(args[0]).resolve()
    out = Path(args[1]) if len(args) > 1 else ROOT / 'toys/build/murmuration/out' / (plan.stem + '-holds')
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
            _, t, n, sky, _, near = line.split()
            frames[float(t)] = (int(n), int(sky), float(near))
    flying = [(t, n, s) for t, (n, s, near) in sorted(frames.items()) if near > 0.5]
    if not flying:
        print('no flying frames'); return
    few = [f for f in flying if f[1] < least]
    print(f'{plan.name}: {len(flying)} flying frames; {100 * len(few) / len(flying):.0f}% with fewer than {least} birds; '
          f'median {statistics.median(f[1] for f in flying):.0f} on screen, {statistics.median(f[2] for f in flying):.0f} against the sky')
    for t, n, s in sorted(flying, key=lambda f: f[1])[:5]:
        print(f'  t={t:6.2f}  {n} birds, {s} against the sky')


if __name__ == '__main__':
    main()

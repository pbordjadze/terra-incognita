#!/usr/bin/env python3
"""Newcomer plans for Battle Ink: what a first-time player on a phone does, in screen points (what they
see), not world units. Written into a directory as <name>.json, played by battle-check like any plan.

    python3 tools/toys/battle/newcomer.py <out dir> [seconds]
    cd toys && ./target/release/battle-check play <out dir>/<name>.json <shots dir>

idle: touches nothing. tapper: taps the armies now and then. scribbler: draws a line from Carthage's
ranks toward Rome's, then another every 12 s for a minute. holder: puts a finger down to think, holds
it, then draws. wrong: draws from Rome toward Carthage. Positions are the opening framing on the phone.
"""
import json, math, sys
from pathlib import Path

SIZE, PPP = (1179, 2556), 3


def stroke(ev, pts, t0, dur, fid, hold=0.0):
    seg = [math.dist(a, b) for a, b in zip(pts, pts[1:])]
    total = sum(seg)
    ev.append({'t': round(t0, 3), 'down': pts[0], 'id': fid})
    t0 += hold
    n = max(2, int(dur * 30))
    for k in range(1, n + 1):
        d, i = total * k / n, 0
        while i < len(seg) - 1 and d > seg[i]:
            d -= seg[i]
            i += 1
        f = min(1.0, d / seg[i]) if seg[i] else 0
        ev.append({'t': round(t0 + dur * k / n, 3), 'move': [round(pts[i][0] + (pts[i + 1][0] - pts[i][0]) * f, 1), round(pts[i][1] + (pts[i + 1][1] - pts[i][1]) * f, 1)], 'id': fid})
    ev.append({'t': round(t0 + dur + 0.01, 3), 'up': pts[-1], 'id': fid})


def main():
    out = Path(sys.argv[1])
    secs = float(sys.argv[2]) if len(sys.argv) > 2 else 150
    out.mkdir(parents=True, exist_ok=True)
    shots = [{'t': t, 'name': f'{t:05.1f}'} for t in range(0, int(secs) + 1, 10)]
    plans = {}
    plans['idle'] = []
    ev = []
    for k, t in enumerate(range(3, 60, 4)):
        ev.append({'t': t, 'tap': [120 + (k * 53) % 160, 300 + (k * 37) % 180]})
    plans['tapper'] = ev
    ev = []
    lines = [
        [[196, 460], [196, 330]],                     # the crescent straight at the Roman centre
        [[60, 470], [60, 330], [120, 250]],          # the left horse up and round
        [[340, 470], [330, 330]],                     # the right horse up
        [[110, 470], [170, 340]],
        [[290, 470], [230, 340]],
        [[196, 470], [196, 300]],
    ]
    for k, l in enumerate(lines):
        stroke(ev, l, 4 + k * 12, 1.0, 100 + k)
    plans['scribbler'] = ev
    ev = []
    stroke(ev, [[196, 460], [196, 330]], 4, 1.0, 100, hold=2.0)
    stroke(ev, [[60, 470], [60, 300]], 14, 1.0, 101, hold=2.0)
    plans['holder'] = ev
    ev = []
    stroke(ev, [[196, 320], [196, 460]], 4, 1.0, 100)
    plans['wrong'] = ev
    for name, ev in plans.items():
        ev.sort(key=lambda e: e['t'])
        p = {'_': f'newcomer: {name}', 'size': list(SIZE), 'ppp': PPP, 'seconds': secs, 'fps': 30, 'events': ev, 'shots': shots}
        (out / f'{name}.json').write_text(json.dumps(p, indent=1) + '\n')


if __name__ == '__main__':
    main()

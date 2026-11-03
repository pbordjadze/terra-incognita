#!/usr/bin/env python3
"""Writes Battle Ink's plans (toys/battle/plans/*.json) from strokes drawn in the toy's world units, placed
on each screen by the same view the toy uses (lib.rs: the battle's box BOX_C/BOX kept whole).

    python3 tools/toys/battle/plans.py

The strokes are the ink-orders spike's plans (painting-company row 206): the good one leaves the crescent
to give ground, hooks the Africans in and sends Hasdrubal up the river and round the Roman rear; the bad
one throws everything at the Roman centre.
"""
import json, math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'toys' / 'battle' / 'plans'
BOX_C, BOX = (530.0, 455.0), (600.0, 560.0)

GOOD = [
    [[262, 616], [258, 540], [252, 460], [258, 400], [276, 320], [312, 252], [390, 206], [520, 190], [640, 200], [706, 250], [724, 322]],
    [[334, 664], [332, 600], [332, 540], [344, 500], [374, 476], [410, 466]],
    [[706, 664], [708, 600], [708, 540], [696, 500], [666, 476], [630, 466]],
]
BAD = [
    [[520, 548], [520, 470], [520, 380]],
    [[334, 664], [380, 640], [440, 610], [480, 590]],
    [[706, 664], [660, 640], [600, 610], [560, 590]],
]


def view(size, ppp):
    wp, hp = size[0] / ppp, size[1] / ppp
    s = min(wp / BOX[0], hp / BOX[1])
    return lambda p: [round((p[0] - BOX_C[0]) * s + wp / 2, 1), round((p[1] - BOX_C[1]) * s + hp / 2, 1)]


def stroke(events, to, pts, t0, dur, fid):
    """A finger drawn along a polyline at an even pace, a move every 1/30 s."""
    sp = [to(p) for p in pts]
    seg = [math.dist(a, b) for a, b in zip(sp, sp[1:])]
    total = sum(seg)
    n = max(2, int(dur * 30))
    events.append({'t': round(t0, 3), 'down': sp[0], 'id': fid})
    for k in range(1, n + 1):
        d, i = total * k / n, 0
        while i < len(seg) - 1 and d > seg[i]:
            d -= seg[i]
            i += 1
        f = min(1.0, d / seg[i]) if seg[i] else 0
        p = [round(sp[i][0] + (sp[i + 1][0] - sp[i][0]) * f, 1), round(sp[i][1] + (sp[i + 1][1] - sp[i][1]) * f, 1)]
        events.append({'t': round(t0 + dur * k / n, 3), 'move': p, 'id': fid})
    events.append({'t': round(t0 + dur + 0.01, 3), 'up': sp[-1], 'id': fid})


def plan(name, note, size, ppp, strokes, seconds, shots, film=None, taps=(), extra=(), night=False, reduce_motion=False):
    to = view(size, ppp)
    ev = []
    for k, (pts, t0, dur) in enumerate(strokes):
        stroke(ev, to, pts, t0, dur, 100 + k)
    for t, p in taps:
        ev.append({'t': t, 'tap': to(p)})
    ev += list(extra)
    ev.sort(key=lambda e: e['t'])
    p = {'_': note, 'size': list(size), 'ppp': ppp, 'seconds': seconds, 'fps': 30}
    if night:
        p['night'] = True
    if reduce_motion:
        p['reduce_motion'] = True
    p['events'] = ev
    p['shots'] = [{'t': t, 'name': n} for t, n in shots]
    if film:
        p['film'] = {'from': film[0], 'to': film[1], 'every': film[2]}
    (OUT / f'{name}.json').write_text(json.dumps(p, indent=1) + '\n')


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    phone, ipad, ipad_l = (1179, 2556), (2064, 2752), (2752, 2064)
    good = [(GOOD[0], 2.0, 1.6), (GOOD[1], 4.6, 0.8), (GOOD[2], 6.2, 0.8)]
    bad = [(BAD[0], 2.0, 0.6), (BAD[1], 3.6, 0.7), (BAD[2], 5.0, 0.7)]
    plan('start', 'The first touch on the phone: the engraving at rest, then one stroke that sends Hasdrubal\'s horse up the river and round the Roman rear (watch it comb the purple), a drop of ink on the open flank, and the day running on.',
         phone, 3, [good[0]], 40, [(0.1, 'first'), (2.9, 'drawing'), (5.0, 'combed'), (14.0, 'riding'), (40.0, 'later')],
         film=(0, 40, 0.25), taps=[(9.0, [800, 520])])
    plan('good', 'Three strokes on the phone: Hasdrubal round the rear, both African wings turned in; the double envelopment closes and the Roman mass ends a crisp pocket inside the purple ring.',
         phone, 3, good, 75, [(3.0, 'drawing'), (8.0, 'orders'), (22.0, 'clash'), (38.0, 'closing'), (55.0, 'pocket'), (75.0, 'print')],
         film=(0, 75, 0.5))
    plan('bad', 'A bad plan on the phone: everything thrown at the Roman centre; the Roman depth breaks the crescent.',
         phone, 3, bad, 75, [(8.0, 'orders'), (25.0, 'clash'), (45.0, 'broken'), (75.0, 'print')], film=(0, 75, 0.5))
    plan('touch', 'The hand on the phone at night: taps (drops of ink pushing rings through both armies), a scribble, a long hold that starts a fresh page, then a shake that starts another.',
         phone, 3, [([[420, 620], [470, 560], [430, 520], [500, 470]], 6.0, 0.7)], 24,
         [(3.5, 'drops'), (7.5, 'scribble'), (12.0, 'held'), (13.5, 'fresh'), (20.5, 'shaken')], film=(0, 24, 0.25),
         taps=[(2.0, [600, 560]), (2.6, [450, 300])],
         extra=[{'t': 10.0, 'hold': view(phone, 3)([520, 760]), 'for': 2.2}, {'t': 19.0, 'shake': True}], night=True)
    plan('ipad', 'The good plan on the 13-inch iPad, portrait.',
         ipad, 2, good, 70, [(0.1, 'first'), (5.0, 'drawing'), (20.0, 'clash'), (40.0, 'closing'), (70.0, 'print')])
    plan('ipad-landscape', 'The good plan on the 13-inch iPad, landscape, at night.',
         ipad_l, 2, good, 70, [(0.1, 'first'), (5.0, 'drawing'), (20.0, 'clash'), (40.0, 'closing'), (70.0, 'print')], night=True)


if __name__ == '__main__':
    main()

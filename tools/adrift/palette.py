#!/usr/bin/env python3
"""Adrift's palette held to the contrast rule (CLAUDE.md > Design rules).

    python3 tools/adrift/palette.py            # prints every ink's contrast against its grounds; exits 1 below the rule
    python3 tools/adrift/palette.py --md       # the same as a Markdown table (pasted into look/PALETTE.md)

Reads adrift/crates/adrift-core/src/look/palette.json. An entry with "rule": "ink" owes 4.5:1 (WCAG AA
text) against each named ground; "mark" (a non-text mark that must be read at a glance: a rim, an edge,
a lamp) owes 3:1 (WCAG's large text and graphical objects, 1.4.11). A colour with no rule is a wash or a
glow, laid over something else, and owes nothing on its own. A ground may be a colour of the same style
(the lamp against the rim) or one of the sea's tones. Alpha is ignored for the rule: an ink is read at
full strength or not shown.
"""
import json, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = ROOT / 'adrift' / 'crates' / 'adrift-core' / 'src' / 'look' / 'palette.json'
RULE = {'ink': 4.5, 'mark': 3.0}


def srgb(h):
    return [int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)]


def lum(h):
    r, g, b = [c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4 for c in srgb(h)]
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(a, b):
    la, lb = lum(a), lum(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)


def check(spec):
    rows, bad = [], 0
    for style in ('day', 'night'):
        grounds = dict(spec['grounds'][style])
        grounds.update({k: v['hex'] for k, v in spec[style].items()})
        for name, c in spec[style].items():
            rule = c.get('rule')
            if not rule:
                continue
            for g in c['against']:
                k = contrast(c['hex'], grounds[g])
                ok = k >= RULE[rule]
                bad += not ok
                rows.append((style, name, c['hex'], g, grounds[g], rule, k, ok))
    return rows, bad


def main():
    spec = json.loads(SPEC.read_text())
    rows, bad = check(spec)
    md = '--md' in sys.argv
    if md:
        print('| style | colour | hex | against | ground | owes | has | |\n| --- | --- | --- | --- | --- | --- | --- | --- |')
    for style, name, hx, g, gh, rule, k, ok in rows:
        if md:
            print(f'| {style} | {name} | `{hx}` | {g} | `{gh}` | {RULE[rule]}:1 | {k:.1f}:1 | {"ok" if ok else "**low**"} |')
        else:
            print(f'{style:5} {name:12} {hx} on {g:10} {gh}  owes {RULE[rule]}  has {k:5.2f}  {"ok" if ok else "LOW"}')
    if bad:
        print(f'{bad} below the rule', file=sys.stderr)
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())

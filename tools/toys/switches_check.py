#!/usr/bin/env python3
"""Every toy on Terra's globe has its rules as switches (painting-company row 358): fails, naming the toy,
when one of the toys Terra's doors open (toys/terra/src/doors/places.rs) has no `fn switches` in its crate
or no `plans/switches.json` that turns a switch. That plan is the proof the switches work: CI plays every
plan, and a plan's `"switch"` fails when the toy has no rules (toy-kit check.rs). Run by ci/lint.sh.

    python3 tools/toys/switches_check.py
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOYS = ROOT / 'toys'


def held_crates() -> dict[str, Path]:
    """Each crate Terra links, by name, and its directory."""
    cargo = (TOYS / 'terra' / 'Cargo.toml').read_text()
    return {name: (TOYS / 'terra' / path).resolve() for name, path in re.findall(r'^(\w+) = \{ path = "([^"]+)" \}', cargo, re.MULTILINE)}


def problems() -> list[str]:
    places = (TOYS / 'terra' / 'src' / 'doors' / 'places.rs').read_text()
    opened = re.findall(r'rooms::open::<(\w+)::\w+>', places)
    crates = held_crates()
    out = []
    if not opened:
        out.append('no doors found in toys/terra/src/doors/places.rs (has its shape changed?)')
    for name in opened:
        crate = crates.get(name)
        if crate is None:
            out.append(f'{name}: a door opens it, but toys/terra/Cargo.toml does not link it by path')
            continue
        source = ''.join(p.read_text() for p in (crate / 'src').rglob('*.rs'))
        if not re.search(r'\bfn switches\s*\(', source):
            out.append(f'{name}: no `fn switches` (Toy::switches) in {crate.relative_to(ROOT)}/src')
        plan = crate / 'plans' / 'switches.json'
        if not plan.exists():
            out.append(f'{name}: no {plan.relative_to(ROOT)}')
        elif not any('switch' in e for e in json.loads(plan.read_text()).get('events', [])):
            out.append(f'{name}: {plan.relative_to(ROOT)} never turns a switch')
    return out


if __name__ == '__main__':
    found = problems()
    for p in found:
        print(f'switches: {p}')
    if found:
        print('Every toy on Terra\'s globe has its rules as switches, each shown by plans/switches.json (toys/README.md).')
        sys.exit(1)
    print(f'switches: every toy on the globe has them ({len(re.findall("rooms::open::", (TOYS / "terra/src/doors/places.rs").read_text()))})')

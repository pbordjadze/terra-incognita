#!/usr/bin/env python3
"""docs/tools.md kept true to the code (painting-company row 183).

    python3 ci/check_tools.py           check: exit 1 if the catalog is out of date or names what isn't there
    python3 ci/check_tools.py --write   write the generated columns into docs/tools.md

1. The *Used by* columns of the tables whose callers are code are generated from the code: what a pack
   calls (read from each pack's scripts and prototype.json, `ROWS` below: one detector a row), the packs'
   shared script, and what a native prototype calls (read from each `Prototype(id: ...)` entry in the app's
   Swift). A row the detectors don't know is left as written. The check fails when the file differs from
   what --write would make, so a pack that starts or stops using a tool shows up in the same commit.
2. Every path the catalog names under tools/, ci/, Prototypes/, App/, docs/ or .github/workflows/ exists
   (a placeholder such as `<id>` is left alone; a retired tool is named by its file name alone).
3. Every tool is in it: each script under tools/ and ci/ (.py, .cjs, .js, .mjs, .sh) is named by its path,
   by a folder it lies in (`tools/plates/<look>/` stands for every look's folder), or by its file name on a
   line that names its folder.

Standard library only; packs.yml and CI `plates` run it.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOC = ROOT / 'docs' / 'tools.md'
PROTOTYPES = ROOT / 'Prototypes'
SWIFT = ROOT / 'App' / 'TerraIncognita'

# how the catalog calls each pack (short names, as a reader says them)
NAMES = {'fires-across-the-sea': 'fires', 'gathering-of-water': 'gathering of water',
         'gathering-of-water-alive': 'gathering alive', 'hello-pack': 'hello', 'herds': 'herds',
         'lead-line': 'lead line', 'low-sea': 'low sea', 'monsoon-sails': 'monsoon',
         'sea-of-whales': 'whales', 'vine': 'vine'}


class Pack:
    def __init__(self, folder):
        self.id = folder.name
        self.name = NAMES.get(self.id, self.id)
        self.manifest_text = (folder / 'prototype.json').read_text(encoding='utf-8')
        self.manifest = json.loads(self.manifest_text)
        self.current = self.manifest.get('shelf', 'current') == 'current'
        self.files = {str(p.relative_to(folder)) for p in folder.rglob('*') if p.is_file()}
        # its own scripts (not the shared script's copy: what that offers is its own table)
        self.code = '\n'.join((folder / s).read_text(encoding='utf-8', errors='replace')
                              for s in self.manifest.get('scripts') or [] if s != 'shared.js' and (folder / s).is_file())
        door = self.manifest.get('doorway') or {}
        self.doorway = door

    def has(self, pattern):
        return re.search(pattern, self.code) is not None


def packs():
    return [Pack(p) for p in sorted(PROTOTYPES.iterdir())
            if p.is_dir() and not p.name.startswith(('_', '.')) and (p / 'prototype.json').is_file()]


def near(pack, needle, anchor, reach=400):
    """`needle` within `reach` characters after (or before) an `anchor` match: an option of that call."""
    for m in re.finditer(anchor, pack.code):
        window = pack.code[max(0, m.start() - reach):m.end() + reach]
        if re.search(needle, window):
            return True
    return False


def style(name):
    return lambda p: p.has(r"style:\s*'" + name + "'")


def code(pattern):
    return lambda p: p.has(pattern)


GEO = r'terra\.geo\.set'
# first cell of a row → (detector or None, [(label, detector)], text before the generated list)
ROWS = {
    'Geography': (code(GEO), [], ''),
    '↳ `border`': (style('border'), [], ''),
    '↳ `area`': (style('area'), [], ''),
    '↳ `dot`, `ring`': (None, [('', style('dot')), ('', style('ring'))], ''),
    '↳ `label`': (style('label'), [], ''),
    '↳ `symbol`': (style('symbol'), [], ''),
    '↳ `sounding`': (style('sounding'), [], ''),
    '↳ `ghost`, `track`': (lambda p: style('ghost')(p) or style('track')(p), [], ''),
    '↳ touchable': (lambda p: near(p, r'touchable:\s*true', r"style:\s*'(border|area|ghost|track|dot|ring|label|symbol|sounding)'", 300), [], ''),
    '↳ dated': (lambda p: p.has(r'dated:\s*true'), [], ''),
    '↳ `flow`': (lambda p: p.has(r'\bflow:\s*(true|\{|[A-Za-z_])'), [], ''),
    'Pictures on the globe': (code(r'terra\.images\.set'), [], ''),
    'Sprites': (code(r'terra\.sprites\.set'), [], ''),
    '↳ sprite sheets': (lambda p: re.search(r'\.png#\d+,\d+,\d+,\d+', p.code + p.manifest_text) is not None, [], ''),
    '↳ looks by zoom': (code(r'\blooks\s*[:=]'), [('`every`', code(r'\bevery:\s*[\w(]')), ('`face`', code(r'face\s*[:=]\s*true'))], ''),
    '↳ dated way': (lambda p: p.has(r'\bdates\s*:') and p.has(r'terra\.sprites\.set'), [], ''),
    'Doorway': (lambda p: bool(p.doorway), [('`path`', lambda p: 'path' in p.doorway), ('`breathes`', lambda p: bool(p.doorway.get('breathes')))], ''),
    'Glint': (lambda p: 'doorway-glint.png' in p.files, [], ''),
    'Touch': (lambda p: p.has(r"terra\.on\('(touch|tap)'"), [('`tap`', code(r"terra\.on\('tap'"))], ''),
    '↳ schemes': (None, [('`hold`', code(r"scheme\('hold'\)")), ('`one-finger`', code(r"scheme\('one-finger'\)"))],
                  '`touchables` (default): all; '),
    'The date': (code(r'terra\.time\.show'), [('`day`', code(r"unit:\s*'day'"))], ''),
    '↳ pace, speed': (None, [('pace', code(r'\bpace:\s*[A-Za-z_\[]')), ('speed', code(r'terra\.time\.speed\('))], ''),
    'Caption': (code(r'terra\.caption\.set'), [], ''),
    'Panel': (code(r'terra\.panel\.show'), [], ''),
    "Wikipedia's words": (code(r"wikipedia:\s*'[^']+"), [], ''),
    'The globe': (code(r'terra\.globe\.set'), [('`historical`', code(r'historical:\s*true'))], ''),
    'Camera': (code(r'terra\.camera|terra\.on\(\'camera\''), [('`project`', code(r'terra\.project\(')), ('`unproject`', code(r'terra\.unproject\(')),
                                                            ('`flyTo`', code(r'terra\.flyTo\('))], ''),
    'Many at once': (None, [('focus', code(r"terra\.focus|terra\.on\('focus'")), ('entry', code(r'terra\.entry'))], ''),
    'Settings': (code(r"terra\.settings|terra\.on\('settings'"), [], ''),
    'Files': (code(r'terra\.read(JSON)?\('), [], ''),
    'Timers, frames': (code(r'\bset(Timeout|Interval)\('), [('rAF', code(r'requestAnimationFrame\('))], ''),
    'Open, close': (None, [('open', code(r"terra\.on\('open'")), ('close', code(r"terra\.on\('close'|terra\.close\("))], ''),
    # the packs' shared script
    'Great circles': (code(r'Shared\.(along|arc|unit)\b'), [('along', code(r'Shared\.along\b')), ('arc', code(r'Shared\.arc\b')),
                                                          ('unit', code(r'Shared\.unit\b'))], ''),
    'Frame names': (code(r'Shared\.frames\b'), [], ''),
}


def listing(names, everyone, current):
    """A list of packs as the catalog says it: every pack, every current pack (but …), or the names; a
    parked pack (hello) named after."""
    names = set(names)
    if not names:
        return 'none yet'
    if names >= everyone:
        return 'every pack'
    ours, parked, missing = names & current, sorted(names - current), current - names
    if not missing:
        said = 'every current pack'
    elif len(current) >= 6 and len(missing) <= 2:
        said = 'every current pack but ' + ' and '.join(sorted(missing))
    else:
        return ', '.join(sorted(names))
    return said + (', and ' + ', '.join(parked) if parked else '')


def pack_cell(spec, all_packs):
    main, subs, prefix = spec
    everyone = {p.name for p in all_packs}
    current = {p.name for p in all_packs if p.current}
    parts = []
    head = listing([p.name for p in all_packs if main(p)], everyone, current) if main else None
    tails = []
    for label, detect in subs:
        found = listing([p.name for p in all_packs if detect(p)], everyone, current)
        tails.append(f'{label}: {found}' if label else found)
    if main is None and subs and not any(label for label, _ in subs):
        # (two kinds side by side, as `dot`, `ring`)
        return prefix + '; '.join(f'{t} ({k})' for t, k in zip(tails, ['dot', 'ring']))
    if head is not None:
        parts.append(head + (f' ({"; ".join(tails)})' if tails else ''))
    else:
        parts.append('; '.join(tails))
    return prefix + ''.join(parts)


# ---- natives: each `Prototype(id: "...")` entry and what its block asks of the stage
NATIVE_ROWS = {
    'Style': r'stage\??\.style\s*=',
    'Marker': r'stage\??\.place\(',
    'Lens': r'stage\??\.openLens',
    'Screen': r'stage\??\.present\(',
    'Quiet rules': r'stage\??\.quiet(Names|Clouds|Frame)\(',
    'Still sea': r'stage\??\.stillSea\(',
    'Veils': r'veils:\s*false',
}


def natives():
    out = {}
    for f in sorted(SWIFT.rglob('*.swift')):
        text = f.read_text(encoding='utf-8')
        starts = [m for m in re.finditer(r'Prototype\(\s*id:\s*"([\w-]+)"', text)]
        for i, m in enumerate(starts):
            end = starts[i + 1].start() if i + 1 < len(starts) else len(text)
            out[m.group(1)] = text[m.start():end]
    return out


def native_cell(row, blocks):
    found = [n for n, block in blocks.items() if re.search(NATIVE_ROWS[row], block)]
    if row == 'Veils' and found:
        return 'the ' + {4: 'four'}.get(len(found), str(len(found))) + ' quiet rules' if all(n.startswith('quiet') or n == 'night' for n in found) else ', '.join(found)
    return ', '.join(found) if found else 'none yet'


# ---- the tables
def cells(line):
    return [c.strip() for c in re.split(r'(?<!\\)\|', line.strip())[1:-1]]


def regenerate(text):
    all_packs = packs()
    blocks = natives()
    out, header = [], None
    for line in text.split('\n'):
        if line.startswith('| ') and line.rstrip().endswith('|'):
            c = cells(line)
            if 'Used by' in c:
                header = c.index('Used by')
            elif header is not None and not set(c[0]) <= set('- '):
                row = c[0]
                new = None
                section = current_section(out)
                if row in ROWS and section == 'pack':
                    new = pack_cell(ROWS[row], all_packs)
                elif row in NATIVE_ROWS and section == 'native':
                    new = native_cell(row, blocks)
                if new is not None and header < len(c) and c[header] != new:
                    parts = re.split(r'(?<!\\)\|', line)
                    parts[header + 1] = ' ' + new + ' '
                    line = '|'.join(parts)
        else:
            header = None
        out.append(line)
    return '\n'.join(out)


def current_section(lines):
    for line in reversed(lines):
        if line.startswith('## '):
            return 'native' if 'native prototype' in line else 'pack' if 'What a pack calls' in line else 'other'
    return 'other'


# ---- the paths it names, and the tools it must name
PATH = re.compile(r'(?<![\w/.<-])((?:tools|ci|Prototypes|App|docs|\.github/workflows)/[\w.<>-][\w./<>-]*)')
SCRIPTS = ('.py', '.cjs', '.js', '.mjs', '.sh')


def named_paths(text):
    for m in PATH.finditer(text):
        p = m.group(1).rstrip('.')
        if '<' in p:
            continue
        yield p


def missing_paths(text):
    return sorted({p for p in named_paths(text) if not (ROOT / p).exists()})


def unnamed_tools(text):
    lines = text.split('\n')
    folders = []
    for m in PATH.finditer(text):
        p = m.group(1)
        if p.endswith('/'):
            folders.append(re.compile('^' + re.sub(r'<[^>]*>', '[^/]+', re.escape(p).replace(r'\<', '<').replace(r'\>', '>'))))
    out = []
    for base in ('tools', 'ci'):
        for f in sorted((ROOT / base).rglob('*')):
            if not f.is_file() or f.suffix not in SCRIPTS or {'__pycache__', 'node_modules'} & set(f.parts):
                continue
            rel = str(f.relative_to(ROOT))
            if rel in text or any(r.match(rel) for r in folders):
                continue
            parent = str(f.parent.relative_to(ROOT))
            word = re.compile(r'(?<![\w/])' + re.escape(f.name) + r'(?!\w)')
            if any(parent + '/' in l and word.search(l) for l in lines):
                continue
            out.append(rel)
    return out


def main(argv):
    text = DOC.read_text(encoding='utf-8')
    made = regenerate(text)
    if '--write' in argv:
        if made != text:
            DOC.write_text(made, encoding='utf-8')
            print(f'{DOC.relative_to(ROOT)}: the used-by columns written from the code')
        text = made
    problems = []
    if made != text:
        changed = [b for a, b in zip(text.split('\n'), made.split('\n')) if a != b]
        problems.append('the used-by columns are out of date with the code (python3 ci/check_tools.py --write):\n    '
                        + '\n    '.join(changed))
    gone = missing_paths(text)
    if gone:
        problems.append('it names what is not there: ' + ', '.join(gone))
    unnamed = unnamed_tools(text)
    if unnamed:
        problems.append('tools it does not name (add each to its table): ' + ', '.join(unnamed))
    for p in problems:
        print('FAIL ' + p)
    if not problems:
        print(f'{DOC.relative_to(ROOT)}: used by as the code has it; every path it names exists; every tool is in it')
    return 1 if problems else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))

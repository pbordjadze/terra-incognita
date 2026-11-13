#!/usr/bin/env python3
"""Three limits on the code, each a ratchet: new code keeps under it; what was already past it may only get better.

    python3 ci/ratchet.py                       # all three, over the repository; exits 1 if anything is past
    python3 ci/ratchet.py sizes                 # one of: sizes, complexity, types
    python3 ci/ratchet.py complexity a.rs b.py  # only these files (the pre-commit hook passes what is staged)
    python3 ci/ratchet.py --tighten             # lowers the ceilings to what is there now; drops what is fixed

- sizes: no source file past 800 lines. A long file is where an agent loses its way: it reads a part, misses
  the rest and edits blind. Split it by what it does (a module, a type, its tests).
- complexity: no function past 100 lines of code or a cyclomatic complexity of 20 (lizard, the same measure in
  Rust, Swift, Python and JavaScript), so a short file can't hide a 300-line function. Split it into steps.
- types: no type error in the Python (pyright, standard mode, pyrightconfig.json), counted per file.

What was already past a limit when it came in is listed in ci/ceilings/<limit>.txt, one line each:
`<key> <value>...  # reason`. The key is a path, or `path::function` for complexity (the worst of the
functions of that name in the file). A listed thing may not pass its ceiling; when it gets better, `--tighten`
lowers the ceiling in the same commit (CI says when one can come down). Raising a ceiling or listing something
new takes a reason after a `#` on its line, which review reads: the ratchet is meant to turn one way.
"""
import csv
import io
import json
import pathlib
import re
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CEILINGS = ROOT / 'ci' / 'ceilings'
LINE = re.compile(r'^(\S+)((?:[ \t]+\d+)+)[ \t]*(#.*)?$')


def tracked(patterns, only):
    out = subprocess.run(['git', 'ls-files', '-z', *patterns], cwd=ROOT, capture_output=True, text=True, check=True).stdout
    files = [f for f in out.split('\0') if f]
    if only is not None:
        files = [f for f in files if f in only]
    return [f for f in files if (ROOT / f).is_file()]


def tool(name):
    venv = ROOT / '.venv' / 'bin' / name
    return str(venv) if venv.exists() else shutil.which(name)


def sizes(only):
    out = {}
    for f in tracked(['*.rs', '*.swift', '*.py', '*.js', '*.cjs', '*.mjs', '*.sh', '*.metal', '*.wgsl'], only):
        try:
            with open(ROOT / f, encoding='utf-8') as fh:
                out[f] = (sum(1 for _ in fh),)
        except UnicodeDecodeError:
            continue
    return out


def complexity(only):
    files = tracked(['*.rs', '*.swift', '*.py', '*.js', '*.cjs', '*.mjs'], only)
    if not files:
        return {}
    lizard = tool('lizard')
    if not lizard:
        raise SystemExit('ratchet: lizard is not installed (pip install -r tools/requirements.txt)')
    run = subprocess.run([lizard, '--csv', *files], cwd=ROOT, capture_output=True, text=True, check=False)
    out = {}
    # lizard's columns: nloc, ccn, tokens, parameters, length, location, file, name, long name, start, end
    for row in csv.reader(io.StringIO(run.stdout)):
        if len(row) < 8 or not row[0].isdigit():
            continue
        key = row[6] + '::' + re.sub(r'[ \t#]', '_', row[7])
        nloc, ccn = int(row[0]), int(row[1])
        old = out.get(key, (0, 0))
        out[key] = (max(old[0], nloc), max(old[1], ccn))
    return out


def types(only):
    files = tracked(['*.py'], only)
    if not files:
        return {}
    pyright = tool('pyright')
    if not pyright:
        raise SystemExit('ratchet: pyright is not installed (pip install -r tools/requirements.txt)')
    # the packages pyright resolves imports against decide what it can check: tools/requirements.txt's, in .venv
    python = ROOT / '.venv' / 'bin' / 'python'
    args = [pyright, '--outputjson', '--pythonpath', str(python if python.exists() else sys.executable)]
    run = subprocess.run(args + (files if only is not None else []), cwd=ROOT, capture_output=True, text=True, check=False)
    try:
        report = json.loads(run.stdout)
    except json.JSONDecodeError:
        raise SystemExit(f'ratchet: pyright failed:\n{run.stdout}{run.stderr}') from None
    out = {f: (0,) for f in files}
    for d in report.get('generalDiagnostics', []):
        if d.get('severity') == 'error':
            f = str(pathlib.Path(d['file']).resolve().relative_to(ROOT))
            if f in out:  # tracked files only, as CI sees them: an untracked scratch script doesn't count
                out[f] = (out[f][0] + 1,)
    return out


LIMITS = {
    'sizes': (sizes, (800,), ('lines',), 'split it by what it does'),
    'complexity': (complexity, (100, 20), ('lines of code', 'complexity'), 'split it into steps'),
    'types': (types, (0,), ('type errors',), 'fix them: pyright --outputjson <file> lists them'),
}


def read_ceilings(name, width):
    path = CEILINGS / f'{name}.txt'
    rows, entries = [], {}
    for i, text in enumerate(path.read_text(encoding='utf-8').splitlines(), 1):
        if not text.strip() or text.lstrip().startswith('#'):
            rows.append(text)
            continue
        m = LINE.match(text)
        values = tuple(int(v) for v in m.group(2).split()) if m else ()
        if not m or len(values) != width:
            raise SystemExit(f'ci/ceilings/{name}.txt:{i}: not `<key> {" ".join(["<n>"] * width)}  # reason`: {text}')
        entries[m.group(1)] = (values, m.group(3) or '')
        rows.append(m.group(1))
    return path, rows, entries


def past(name, now, entries, over):
    """What is past its ceiling (if listed) or the limit (if not), as messages."""
    _, limits, units, advice = LIMITS[name]
    bad = []
    for key, values in sorted(now.items()):
        if key in entries:
            ceiling = [max(c, lim) for c, lim in zip(entries[key][0], limits)]
            where, rest = 'ceiling', f'ci/ceilings/{name}.txt; {advice}, don\'t raise it'
        elif key in over:
            ceiling, where, rest = limits, 'limit', advice
        else:
            continue
        parts = [f'{v} {u} ({where} {c})' for v, c, u in zip(values, ceiling, units) if v > c]
        if parts:
            bad.append(f'{key}: {", ".join(parts)}; {rest}')
    return bad


def lowered(name, entries, over, in_scope):
    """Each listed key's ceiling as it can be now: lower where it improved, None where it is fixed or gone."""
    limits = LIMITS[name][1]
    new = {}
    for key, (ceiling, _) in entries.items():
        if not in_scope(key):
            new[key] = ceiling
        elif key in over:
            new[key] = tuple(min(c, max(v, lim)) for c, v, lim in zip(ceiling, over[key], limits))
        else:
            new[key] = None
    return new


def rewrite(path, rows, entries, new):
    """Writes the ceilings back with the new values, each line keeping its place and its reason."""
    lines = []
    for row in rows:
        if row not in entries:
            lines.append(row)
        elif new[row] is not None:
            reason = entries[row][1]
            lines.append(f'{row} {" ".join(map(str, new[row]))}' + (f'  {reason}' if reason else ''))
    path.write_text('\n'.join(lines) + '\n', encoding='utf-8')


def check(name, only, tighten):
    measure, limits, _, _ = LIMITS[name]
    path, rows, entries = read_ceilings(name, len(limits))
    now = measure(only)
    over = {k: v for k, v in now.items() if any(a > b for a, b in zip(v, limits))}
    bad = past(name, now, entries, over)
    in_scope = (lambda key: key.split('::')[0] in only) if only is not None else (lambda key: True)
    new = lowered(name, entries, over, in_scope)
    better = [k for k in entries if new[k] != entries[k][0]]
    if tighten and better:
        rewrite(path, rows, entries, new)
        print(f'{name}: lowered or dropped {len(better)} ceiling(s) in ci/ceilings/{name}.txt')
    elif better:
        print(f'{name}: {len(better)} ceiling(s) can come down: python3 ci/ratchet.py {name} --tighten')
    for b in bad:
        print(b)
    print(f'{name}: {len(bad)} past')
    return not bad


def main(argv):
    tighten = '--tighten' in argv
    args = [a for a in argv if a != '--tighten']
    names = [a for a in args if a in LIMITS] or list(LIMITS)
    files = [a for a in args if a not in LIMITS]
    only = set(files) if files else None
    ok = [check(n, only, tighten) for n in names]
    return 0 if all(ok) else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))

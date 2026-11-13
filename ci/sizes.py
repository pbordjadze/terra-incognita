#!/usr/bin/env python3
"""No source file grows past LIMIT lines; the files already past it may only shrink (a ratchet).

    python3 ci/sizes.py            # lists each file too long; exits 1 if there is any
    python3 ci/sizes.py --tighten  # lowers ci/sizes.txt's ceilings to the files' lengths now, drops gone files

A long file is where an agent loses its way: it reads a part, misses the rest, and edits blind. A file
past the limit is split by what it does (a module, a type, its tests) rather than allowed to grow.
ci/sizes.txt lists the files that were already longer when the limit came in, each with its ceiling: the
length it may not pass. When one shrinks, `--tighten` lowers its ceiling in the same commit. Raising a
ceiling, or listing a new file, takes a reason on its line (`path ceiling  # why`), which review reads.
ci/qa/ is generated from painting-company's shared copy and checked there; data and generated sources
(levels, harness bundles) are listed like any other file rather than excused.
"""
import pathlib
import subprocess
import sys

LIMIT = 800
SOURCE = ('*.rs', '*.swift', '*.py', '*.js', '*.cjs', '*.mjs', '*.sh', '*.metal', '*.wgsl')
CEILINGS = pathlib.Path(__file__).with_name('sizes.txt')


def lengths():
    out = subprocess.run(['git', 'ls-files', *SOURCE], capture_output=True, text=True, check=True).stdout
    sizes = {}
    for f in out.split('\n'):
        if not f or f.startswith('ci/qa/'):
            continue
        try:
            with open(f, encoding='utf-8') as fh:
                sizes[f] = sum(1 for _ in fh)
        except (OSError, UnicodeDecodeError):
            continue
    return sizes


def ceilings():
    listed = {}
    for line in CEILINGS.read_text(encoding='utf-8').splitlines():
        body = line.split('#', 1)[0].split()
        if len(body) == 2:
            listed[body[0]] = (int(body[1]), line)
    return listed


def tighten(sizes, listed):
    keep = [line for line in CEILINGS.read_text(encoding='utf-8').splitlines() if not line.split('#', 1)[0].split()]
    for path, (ceiling, line) in sorted(listed.items()):
        n = sizes.get(path)
        if n is None or n <= LIMIT:
            continue  # gone, or back under the limit: no longer listed
        if n < ceiling:
            line = line.replace(f'{path} {ceiling}', f'{path} {n}', 1) if f'{path} {ceiling}' in line else f'{path} {n}'
        keep.append(line)
    CEILINGS.write_text('\n'.join(keep) + '\n', encoding='utf-8')


def main():
    sizes, listed = lengths(), ceilings()
    if '--tighten' in sys.argv:
        tighten(sizes, listed)
        sizes, listed = lengths(), ceilings()
    bad, slack = [], []
    for path, n in sorted(sizes.items()):
        if path in listed:
            ceiling = listed[path][0]
            if n > ceiling:
                bad.append(f'{path}: {n} lines, past its ceiling of {ceiling} in ci/sizes.txt (split it, don\'t raise it)')
            elif n < ceiling:
                slack.append(path)
        elif n > LIMIT:
            bad.append(f'{path}: {n} lines, past the limit of {LIMIT} (split it by what it does)')
    gone = [p for p in listed if p not in sizes or sizes[p] <= LIMIT]
    for b in bad:
        print(b)
    if slack or gone:
        print(f'ci/sizes.txt: {len(slack) + len(gone)} ceiling(s) can come down: python3 ci/sizes.py --tighten')
    print(f'files too long: {len(bad)}')
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())

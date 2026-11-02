#!/usr/bin/env python3
"""Assembles the data directory a toy loads, from toys/<toy>/manifest.json.

    python3 tools/toys/assets.py <toy> [<out-dir>] [--link]

The default out-dir is toys/build/<toy>/assets, where the headless check looks (TOY_ASSETS overrides).
Each file is copied (or hard-linked with --link) under the name the code uses; the IPA bundles the same
directory as data/.
"""
import json, os, shutil, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    if not 1 <= len(args) <= 2:
        sys.exit(__doc__)
    toy = args[0]
    out = Path(args[1]) if len(args) == 2 else ROOT / 'toys' / 'build' / toy / 'assets'
    out.mkdir(parents=True, exist_ok=True)
    manifest = json.load(open(ROOT / 'toys' / toy / 'manifest.json'))
    total = 0
    for name, src in manifest['files'].items():
        s, d = ROOT / src, out / name
        if d.exists() or d.is_symlink():
            d.unlink()
        if '--link' in sys.argv:
            os.link(s, d)
        else:
            shutil.copyfile(s, d)
        total += s.stat().st_size
    print(f'{toy}: {len(manifest["files"])} files, {total / 1e6:.1f} MB, in {out}')


if __name__ == '__main__':
    main()

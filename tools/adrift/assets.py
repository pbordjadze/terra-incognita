#!/usr/bin/env python3
"""Assembles the directory of data files a shell of Adrift loads, from adrift/data/manifest.json.

    python3 tools/adrift/assets.py <out-dir> [--link]

Each file in the manifest is copied (or, --link, hard-linked) under the name the code uses. The
checks read it with ADRIFT_ASSETS=<out-dir>; the web build serves it; the device build bundles it.
"""
import json, os, shutil, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    if len(args) != 1:
        sys.exit(__doc__)
    out = Path(args[0])
    out.mkdir(parents=True, exist_ok=True)
    manifest = json.load(open(ROOT / 'adrift' / 'data' / 'manifest.json'))
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
    print(f'{len(manifest["files"])} files, {total / 1e6:.1f} MB, in {out}')


if __name__ == '__main__':
    main()

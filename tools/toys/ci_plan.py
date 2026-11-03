#!/usr/bin/env python3
"""Decides what toys.yml does for a push (or a run started by hand): which toys are checked on Linux
and which are built as IPAs on the shared macOS runners. Prints `check=<json list>` and
`ipa=<json list>` for $GITHUB_OUTPUT.

    python3 tools/toys/ci_plan.py <event> <ref-name> <before-sha> <sha> "<head commit message>" [<toy> <ipa>]

- checked: every toy whose own directory (toys/<toy>/) changed; every toy when the shared parts did
  (toys/crates, toys/ios, toys/Cargo.*, tools/toys, the workflow).
- built: on main, every toy whose own directory changed (one IPA a toy a merge: a round's merge is one
  build), plus any named; on a branch only the toys named by `[toys-ipa <toy> ...]` (or `all`) in the
  commit's first line. A run started by hand checks its `toy` (or all) and builds it when `ipa`.
"""
import json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOYS = sorted(p.parent.name for p in (ROOT / 'toys').glob('*/toy.json'))
SHARED = ('toys/crates/', 'toys/ios/', 'toys/Cargo.', 'tools/toys/', '.github/workflows/toys.yml')

event, ref, before, sha, message = sys.argv[1:6]
named = []
m = re.search(r'\[toys-ipa ([^\]]*)\]', message.splitlines()[0] if message else '')
if m:
    named = TOYS if m.group(1).strip() == 'all' else [t for t in m.group(1).split() if t in TOYS]

if event == 'workflow_dispatch':
    toy, ipa = (sys.argv[6:8] + ['', 'false'])[:2]
    check = [toy] if toy in TOYS else TOYS
    build = check if ipa == 'true' else []
else:
    try:
        if set(before) == {'0'}:
            # a new branch: everything it changed since it left main (its head commit alone may be a
            # docs line, and a kit change earlier on the branch would go unchecked)
            before = subprocess.run(['git', 'merge-base', 'origin/main', sha], check=True, capture_output=True, text=True, cwd=ROOT).stdout.strip()
        files = subprocess.run(['git', 'diff', '--name-only', before, sha], check=True, capture_output=True, text=True, cwd=ROOT).stdout.split()
    except Exception:
        files = subprocess.run(['git', 'show', '--name-only', '--format=', sha], check=True, capture_output=True, text=True, cwd=ROOT).stdout.split()
    own = [t for t in TOYS if any(f.startswith(f'toys/{t}/') for f in files)]
    shared = any(f.startswith(SHARED) for f in files)
    check = TOYS if shared else own
    build = sorted(set((own if ref == 'main' else []) + named))
    check = sorted(set(check + build))
print(f'check={json.dumps(check)}')
print(f'ipa={json.dumps(build)}')

#!/usr/bin/env python3
"""Every silenced lint says why, beside it (CLAUDE.md › Staying current), so a lint is never muted quietly.

    python3 ci/silenced.py        # lists each silence without a reason; exits 1 if there is any

- Rust: `#[allow(...)]`, `#![allow(...)]`, `#[expect(...)]` carry a `// reason` on the same line (or on the
  line directly above, where rustfmt moves it).
- Python: `# noqa: CODE` is followed by its reason on the same line.
- Shell and Swift: a `# shellcheck disable=` or `// swiftlint:disable` directive has a comment line with
  the reason directly above it (neither tool reads text after its directive).
- The rules loosened for a whole tool are in its config file, each with its reason: Cargo.toml's
  [workspace.lints], ruff.toml, .swiftlint.yml, .github/actionlint.yaml.
"""
import re
import subprocess
import sys

RUST = re.compile(r'#!?\[(allow|expect)\(')
NOQA = re.compile(r'#\s*noqa(?::\s*[A-Z]+[0-9]+(?:\s*,\s*[A-Z]+[0-9]+)*)?(.*)$')
DIRECTIVE = {'.sh': re.compile(r'^\s*#\s*shellcheck\s+disable='), '.swift': re.compile(r'^\s*//\s*swiftlint:disable')}
COMMENT = {'.sh': re.compile(r'^\s*#(?!\s*shellcheck)\s*\S'), '.swift': re.compile(r'^\s*//(?!\s*swiftlint)\s*\S')}


def files():
    out = subprocess.run(['git', 'ls-files', '*.rs', '*.py', '*.sh', '*.swift'], capture_output=True, text=True, check=True).stdout
    return out.split()


def main():
    bad = []
    for f in files():
        try:
            with open(f, encoding='utf-8') as fh:
                lines = fh.read().split('\n')
        except (OSError, UnicodeDecodeError):
            continue
        ext = f[f.rfind('.'):]
        for i, line in enumerate(lines):
            if ext == '.rs' and RUST.search(line):
                tail = line[line.index(')]') + 2:] if ')]' in line else ''
                above = lines[i - 1] if i else ''
                if not re.search(r'//\s*\S', tail) and not re.match(r'^\s*//[^/!]\s*\S', above):
                    bad.append(f'{f}:{i + 1}: {line.strip()}  (a `// reason` on the same line or the line above)')
            elif ext == '.py' and (m := NOQA.search(line)) and not m.group(1).strip():
                bad.append(f'{f}:{i + 1}: {line.strip()}  (its reason after the codes)')
            elif ext in DIRECTIVE and DIRECTIVE[ext].search(line) and (i == 0 or not COMMENT[ext].search(lines[i - 1])):
                bad.append(f'{f}:{i + 1}: {line.strip()}  (a comment with the reason on the line above)')
    for b in bad:
        print(b)
    print(f'silenced lints without a reason: {len(bad)}')
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())

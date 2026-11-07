#!/usr/bin/env python3
"""A web build directory made ready to publish as a claude.ai artifact (docs/kit.md).

The recipe three sessions each worked out again: the Artifact tool publishes an `index.html` as the page
and every other file through its `files` map ({published path: source path}); it serves only standard web
media types (a `.bin` sheet is refused), at most 15 MB a binary and 16 MB a text file, 255 files and 64 MB
a publish. So a binary of another type is wrapped as `<name>.json` holding {"base64": "..."} and listed in
`assets/index.json` ({name: path}), which the web shell reads first and unwraps (adrift-web's lib.rs does;
a new web shell must do the same, see `WEB_SHELL` below).

    python3 tools/kit/artifact.py <web-dir> [--assets assets]

Writes, in <web-dir>: the wrapped files, `<assets>/index.json`, and `artifact.json`:
    {"file_path": "<web-dir>/index.html", "batches": [{published path: source path or {from, contentType}}, ...]}
Each batch fits one publish; publish the first with `file_path` and `files`, the rest to the same url.
Fails (exit 1) on a file too large for the host even wrapped, or without index.html.
"""
import base64
import json
import os
import sys
from pathlib import Path

# What the host serves as itself (by extension). Anything else is wrapped.
SERVED = {'.html', '.js', '.mjs', '.css', '.json', '.wasm', '.png', '.jpg', '.jpeg', '.gif', '.webp', '.svg',
          '.ico', '.txt', '.md', '.csv', '.wav', '.mp3', '.ogg', '.mp4', '.webm', '.woff', '.woff2', '.ttf', '.otf',
          '.pdf'}
TEXT = {'.html', '.js', '.mjs', '.css', '.json', '.svg', '.txt', '.md', '.csv'}
EXPLICIT = {'.wasm': 'application/wasm', '.mjs': 'text/javascript', '.wav': 'audio/wav', '.woff2': 'font/woff2'}
MB = 1 << 20
LIMIT_TEXT, LIMIT_BINARY, BATCH_FILES, BATCH_BYTES = 16 * MB, 15 * MB, 255, 64 * MB

WEB_SHELL = '''what a web shell does to read the data (Rust, as adrift-web):
    index = fetch("assets/index.json") as {name: path}, or {} when absent
    bytes = fetch(index.get(name) or "assets/" + name)
    if path ends in ".json" and name does not: bytes = base64-decode(json["base64"])'''


def main():
    argv = sys.argv[1:]
    if not argv or argv[0] in ('-h', '--help'):
        print(__doc__)
        print(WEB_SHELL)
        return 0
    web = os.path.abspath(argv[0])
    assets = argv[argv.index('--assets') + 1] if '--assets' in argv else 'assets'
    if not os.path.isfile(os.path.join(web, 'index.html')):
        sys.exit(f'{web} has no index.html')
    adir = os.path.join(web, assets)
    index_path = os.path.join(adir, 'index.json')
    index = json.loads(Path(index_path).read_text()) if os.path.isfile(index_path) else {}

    # wrap what the host won't serve
    wrapped = 0
    if os.path.isdir(adir):
        for name in sorted(os.listdir(adir)):
            src = os.path.join(adir, name)
            ext = os.path.splitext(name)[1].lower()
            if not os.path.isfile(src) or ext in SERVED:
                continue
            with open(src, 'rb') as f:
                data = f.read()
            out = src + '.json'
            with open(out, 'w') as f:
                json.dump({'base64': base64.b64encode(data).decode()}, f)
            index[name] = f'{assets}/{name}.json'
            wrapped += 1
    if index:
        os.makedirs(adir, exist_ok=True)
        with open(index_path, 'w') as f:
            json.dump(index, f, indent=1, sort_keys=True)

    # every file the page needs, minus the page itself, the originals now wrapped and this tool's output
    skip = {os.path.join(adir, n) for n in index if not n.endswith('.json')} | {os.path.join(web, 'index.html'), os.path.join(web, 'artifact.json')}
    files, problems = [], []
    for root, _, names in os.walk(web):
        for n in sorted(names):
            src = os.path.join(root, n)
            if src in skip or n.startswith('.'):
                continue
            ext = os.path.splitext(n)[1].lower()
            size = os.path.getsize(src)
            limit = LIMIT_TEXT if ext in TEXT else LIMIT_BINARY
            if ext not in SERVED:
                problems.append(f'{os.path.relpath(src, web)}: a {ext or "typeless"} file outside {assets}/ is not served; move it under {assets}/ to have it wrapped')
            elif size > limit:
                problems.append(f'{os.path.relpath(src, web)}: {size / MB:.1f} MB, over the host\'s {limit // MB} MB a file')
            files.append((os.path.relpath(src, web), src, size))
    if problems:
        sys.exit('cannot publish:\n  ' + '\n  '.join(problems))

    batches, cur, cur_bytes = [], {}, 0
    for rel, src, size in sorted(files):
        if cur and (len(cur) >= BATCH_FILES or cur_bytes + size > BATCH_BYTES):
            batches.append(cur)
            cur, cur_bytes = {}, 0
        # a type the host may not infer from the extension is stated: {from, contentType}
        cur[rel] = {'from': src, 'contentType': EXPLICIT[os.path.splitext(rel)[1].lower()]} if os.path.splitext(rel)[1].lower() in EXPLICIT else src
        cur_bytes += size
    if cur:
        batches.append(cur)
    with open(os.path.join(web, 'artifact.json'), 'w') as f:
        json.dump({'file_path': os.path.join(web, 'index.html'), 'batches': batches}, f, indent=1)
    total = sum(s for _, _, s in files)
    print(f'artifact: {len(files)} files, {total / MB:.1f} MB, {wrapped} wrapped as base64, '
          f'{len(batches)} publish(es); the map is {os.path.join(web, "artifact.json")}')
    return 0


if __name__ == '__main__':
    sys.exit(main())

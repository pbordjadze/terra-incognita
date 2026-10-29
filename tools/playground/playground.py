#!/usr/bin/env python3
"""The prototype playground's developer tool (docs/runtime3.md, docs/playground.md): scaffold, check,
pack and publish the packs in Prototypes/. A pack is runtime 3: scripts run in JavaScriptCore, whose
geography, pictures and sprites the globe draws; no pack has a web page. Standard library only.

    playground.py new <id> [--title T]                         scaffold Prototypes/<id>/ from template/
    playground.py validate [ids...]                            the contract, sizes and the offline boundary
    playground.py pack [ids...] [--from DIR ...] [--out DIR] [--base FILE_OR_URL]
                                                               catalog.json, packs/<id>/r<rev>/, open/<id>.html
    playground.py publish --into DIR [ids...] [--public-url URL] [--commit-message FILE] [--dry-run] [--summary FILE]
                                                               revise the published tree in DIR (a checkout of
                                                               the `packs` branch); committing is the caller's
    playground.py publish [ids...] [--dry-run] [--summary FILE]
                                                               pack against the live R2 bucket, upload what changed
    playground.py retire <id> [--out DIR] [--undo]             mark a pack withdrawn (catalog only)
    playground.py share [ids...]                               copy Prototypes/_shared/shared.js into each pack
                                                               that lists shared.js in its scripts
    playground.py list [--remote URL]                          the bundled prototypes, and a remote catalog's

publish --into needs no credentials (packs.yml publishes the `packs` branch with it). publish and
retire without a folder use R2 and read R2_ACCOUNT_ID, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_BUCKET
and PACKS_PUBLIC_URL from the environment (R2_ENDPOINT overrides the S3 endpoint, for a local
S3-compatible server). Exit status: 0 done, 1 findings or a failed check, 2 usage or environment.
"""
import argparse
import datetime
import hashlib
import hmac
import html
import json
import math
import os
import re
import shutil
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from zoneinfo import ZoneInfo

ROOT = Path(__file__).resolve().parents[2]
PROTOTYPES = ROOT / 'Prototypes'
TEMPLATES = Path(__file__).resolve().parent / 'template'
DEFAULT_OUT = ROOT / 'build' / 'packs'
RUNTIME = 6
# a pack declares the runtime it needs: 3, 4 for what docs/runtime3.md marks new in 4, 5 for what it
# marks new in 5 (a sprite on a dated way, the date's pace), 6 for what it marks new in 6 (sprite sheets)
RUNTIMES = (3, 4, 5, 6)
# a frame of a sprite sheet, 'art/early.png#x,y,w,h' (pixels), came with runtime 6
SHEET_RUNTIME = 6
SHEET_FRAME = re.compile(r'(.+)#(\d+),(\d+),(\d+),(\d+)')
# a doorway's life (looks by zoom, a path, a breath) came with runtime 4
DOORWAY_LIFE_RUNTIME = 4
OPEN_SCHEME = 'terraincognita'
R3_ONLY = 'runtime 3 only: a pack is scripts drawn by the globe'

ID_RE = re.compile(r'[a-z0-9](?:[a-z0-9-]{0,38}[a-z0-9])')
PATH_RE = re.compile(r'[A-Za-z0-9._-]+(?:/[A-Za-z0-9._-]+)*')
REVISION_DIR = re.compile(r'r([1-9][0-9]*)')
# the packs' shared script (docs/runtime3.md › Shared script): a pack carries a copy, which must be this file
SHARED = PROTOTYPES / '_shared' / 'shared.js'
SHARED_NAME = 'shared.js'

# decimal megabytes: if the app counts in binary ones, a pack that passes here passes there too
MAX_PACK_BYTES = 50_000_000
MAX_FILE_BYTES = 25_000_000
MAX_FILES = 500
MAX_FRAMES = 24   # a sprite's (or doorway's) look, docs/runtime3.md

TOP_KEYS = {'id', 'title', 'summary', 'runtime', 'presentation', 'scripts', 'anchor', 'doorway', 'taps', 'shelf'}
# what runtimes 1 and 2 declared and runtime 3 has no place for
WEB_KEYS = {'entry': 'a web page', 'overlay': 'a web page over the globe',
            'markers': 'pictures placed by the host; a "doorway" or terra.sprites does that now'}
ANCHOR_KEYS = {'lon', 'lat', 'zoom'}
DOORWAY_KEYS = {'lon', 'lat', 'label', 'look', 'looks', 'path', 'breathes'}
# a doorway's life (runtime 4): its looks by zoom, the way it stirs along, a slow breath
DOORWAY_LIFE_KEYS = ('looks', 'path', 'breathes')
PATH_KEYS = {'coordinates', 'seconds', 'loop'}
LOOK_KEYS = {'image', 'frames', 'fps', 'size', 'minZoom', 'maxZoom', 'every', 'for', 'face'}
IMAGE_EXTS = ('.png', '.jpg', '.jpeg')
PAGE_EXTS = ('.html', '.htm')
SCRIPT_EXTS = ('.js', '.mjs')

CONTENT_TYPES = {
    '.html': 'text/html; charset=utf-8', '.htm': 'text/html; charset=utf-8',
    '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8',
    '.css': 'text/css; charset=utf-8', '.json': 'application/json', '.txt': 'text/plain; charset=utf-8',
    '.md': 'text/markdown; charset=utf-8', '.svg': 'image/svg+xml', '.png': 'image/png',
    '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.gif': 'image/gif', '.webp': 'image/webp',
    '.avif': 'image/avif', '.woff': 'font/woff', '.woff2': 'font/woff2', '.otf': 'font/otf',
    '.ttf': 'font/ttf', '.mp3': 'audio/mpeg', '.m4a': 'audio/mp4', '.wav': 'audio/wav',
    '.ogg': 'audio/ogg', '.mp4': 'video/mp4', '.webm': 'video/webm', '.wasm': 'application/wasm',
}

R2_VARS = ('R2_ACCOUNT_ID', 'R2_ACCESS_KEY_ID', 'R2_SECRET_ACCESS_KEY', 'R2_BUCKET', 'PACKS_PUBLIC_URL')
USER_AGENT = 'terra-incognita-playground/1'
EASTERN = 'America/New_York'


class ToolError(Exception):
    """A failure to report in one line (bad usage, missing environment, unreachable bucket)."""


def content_type(name):
    return CONTENT_TYPES.get(Path(name).suffix.lower(), 'application/octet-stream')


def now_eastern():
    # Paul's rule: every time a session gives him is Eastern, with its offset
    return datetime.datetime.now(ZoneInfo(EASTERN)).isoformat(timespec='seconds')


def fmt_bytes(n):
    return f'{n} B' if n < 1024 else f'{n / 1024:.1f} KB' if n < 1024 * 1024 else f'{n / 1024 / 1024:.1f} MB'


def display(path):
    p = Path(path)
    try:
        return p.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


# ---------------------------------------------------------------- findings

class Report:
    def __init__(self):
        self.items = []  # (level, where, message)

    def error(self, where, message):
        self.items.append(('ERROR', str(where), message))

    def warn(self, where, message):
        self.items.append(('WARN', str(where), message))

    @property
    def errors(self):
        return sum(1 for i in self.items if i[0] == 'ERROR')

    @property
    def warnings(self):
        return sum(1 for i in self.items if i[0] == 'WARN')

    def lines(self):
        return [f'{level:<5} {where}: {message}' for level, where, message in self.items]

    def print(self, file=None):
        for line in self.lines():
            print(line, file=file or sys.stdout)


# ---------------------------------------------------------------- validation

def is_int(v):
    return isinstance(v, int) and not isinstance(v, bool)


def is_num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v)


def safe_path(p):
    """A path a pack may contain or refer to: portable characters, relative, nothing hidden or `..`."""
    return (isinstance(p, str) and PATH_RE.fullmatch(p) is not None
            and not any(seg.startswith('.') for seg in p.split('/')))


def _no_dupes(pairs):
    out = {}
    for k, v in pairs:
        if k in out:
            raise ValueError(f'duplicate key "{k}"')
        out[k] = v
    return out


def _no_const(name):
    raise ValueError(f'{name} is not valid JSON')


def load_json(path):
    return json.loads(Path(path).read_text(encoding='utf-8'), object_pairs_hook=_no_dupes,
                      parse_constant=_no_const)


def prototype_dirs(base=None):
    base = Path(base or PROTOTYPES)
    if not base.is_dir():
        return []
    return sorted(p for p in base.iterdir()
                  if p.is_dir() and not p.name.startswith(('_', '.')) and not p.is_symlink())


def scan_files(folder, rep):
    """{posix path: Path} of the regular, non-hidden files, reporting what a pack must not contain.
    Hidden files (.DS_Store) are ignored, as the packer ignores them."""
    folder = Path(folder)
    where = display(folder)
    files, total, folded = {}, 0, {}
    for dirpath, dirnames, filenames in os.walk(folder):
        keep = []
        for d in sorted(dirnames):
            if d.startswith('.'):
                continue
            if os.path.islink(os.path.join(dirpath, d)):
                rep.error(f'{where}/{Path(dirpath, d).relative_to(folder).as_posix()}', 'symlinks are not allowed')
                continue
            keep.append(d)
        dirnames[:] = keep
        for name in sorted(filenames):
            if name.startswith('.'):
                continue
            full = Path(dirpath, name)
            rel = full.relative_to(folder).as_posix()
            if full.is_symlink():
                rep.error(f'{where}/{rel}', 'symlinks are not allowed')
                continue
            if not full.is_file():
                rep.error(f'{where}/{rel}', 'not a regular file')
                continue
            if not safe_path(rel):
                rep.error(f'{where}/{rel}', 'path may use only letters, digits, . _ - and /, with no segment starting with "."')
                continue
            size = full.stat().st_size
            if size > MAX_FILE_BYTES:
                rep.error(f'{where}/{rel}', f'{fmt_bytes(size)} is over the {fmt_bytes(MAX_FILE_BYTES)} file limit')
            if rel.lower() in folded:
                rep.error(f'{where}/{rel}', f'differs from {folded[rel.lower()]} only in case (collides on a case-insensitive volume)')
            folded.setdefault(rel.lower(), rel)
            files[rel] = full
            total += size
    if len(files) > MAX_FILES:
        rep.error(where, f'{len(files)} files is over the {MAX_FILES} file limit')
    if total > MAX_PACK_BYTES:
        rep.error(where, f'{fmt_bytes(total)} is over the {fmt_bytes(MAX_PACK_BYTES)} pack limit')
    return files


def _line_of(text, pos):
    return text.count('\n', 0, pos) + 1


_WORD = re.compile(r'[A-Za-z0-9_$]+')
# after these a "/" starts a regular expression; after a value (a name, a number, ")" or "]") it divides
_REGEX_AFTER = set('(,=:[!&|?{};+-*%<>~^')
_REGEX_WORDS = {'return', 'typeof', 'instanceof', 'in', 'of', 'new', 'delete', 'void', 'throw', 'case', 'do',
                'else', 'yield', 'await'}


def js_scan(text):
    """A script split, approximately, into code, string literals and comments: enough to tell a call
    from the same word in a string or a comment. Returns (code, strings, comments): `code` is the text
    with every literal's contents (strings, template literals, regular expressions) and every comment
    blanked, newlines kept; `strings` is [(start, end, raw text between the quotes)] and `comments`
    [(start, end)]."""
    n = len(text)
    out = list(text)
    strings, comments = [], []
    braces, templates = 0, []   # brace depth, and the depth at each open ${ of a template literal
    last = ''                   # the last token: a punctuator, a word, or 'value' after a literal
    i = 0

    def blank(a, b):
        for k in range(a, b):
            if out[k] != '\n':
                out[k] = ' '

    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ''
        if c.isspace() or c == '\ufeff':
            i += 1
        elif c == '/' and nxt == '/':
            j = text.find('\n', i)
            j = n if j < 0 else j
            comments.append((i, j))
            blank(i, j)
            i = j
        elif c == '/' and nxt == '*':
            j = text.find('*/', i + 2)
            j = n if j < 0 else j + 2
            comments.append((i, j))
            blank(i, j)
            i = j
        elif c in '"\'':
            j = i + 1
            while j < n and text[j] != c and text[j] != '\n':
                j += 2 if text[j] == '\\' else 1
            j = min(j, n)
            strings.append((i, j + 1, text[i + 1:j]))
            blank(i + 1, j)
            i, last = j + 1, 'value'
        elif c == '`' or (c == '}' and templates and braces == templates[-1]):
            # a template literal, or the rest of one after a ${ … }
            if c == '}':
                templates.pop()
            j = i + 1
            while j < n and text[j] != '`' and not (text[j] == '$' and j + 1 < n and text[j + 1] == '{'):
                j += 2 if text[j] == '\\' else 1
            j = min(j, n)
            strings.append((i, j + 1, text[i + 1:j]))
            blank(i + 1, j)
            if j < n and text[j] == '$':
                templates.append(braces)
                i, last = j + 2, '{'
            else:
                i, last = j + 1, 'value'
        elif c == '/' and (last == '' or last in _REGEX_AFTER or last in _REGEX_WORDS):
            j, in_class = i + 1, False
            while j < n and text[j] != '\n':
                ch = text[j]
                if ch == '\\':
                    j += 2
                    continue
                if in_class:
                    in_class = ch != ']'
                elif ch == '[':
                    in_class = True
                elif ch == '/':
                    break
                j += 1
            if j < n and text[j] == '/':
                blank(i + 1, j)
                m = _WORD.match(text, j + 1)
                i, last = (m.end() if m else j + 1), 'value'
            else:
                i, last = i + 1, '/'
        else:
            m = _WORD.match(text, i)
            if m:
                i, last = m.end(), m.group(0)
            else:
                if c == '{':
                    braces += 1
                elif c == '}':
                    braces -= 1
                i, last = i + 1, c
    return ''.join(out), strings, comments


_GLOBAL = r'(?<![\w$.])(?:(?:window|globalThis|self)\.)?'
_NO_NET = 'a pack has no network and loads nothing beyond its "scripts" (docs/runtime3.md): bundle the data and read it with terra.read'
# what a script must not use: there is no network in JavaScriptCore, and scripts are plain scripts run in order
NO_NETWORK = [
    (re.compile(_GLOBAL + r'fetch\s*\('), 'fetch()', _NO_NET),
    (re.compile(_GLOBAL + r'XMLHttpRequest\b'), 'XMLHttpRequest', _NO_NET),
    (re.compile(_GLOBAL + r'WebSocket\b'), 'WebSocket', _NO_NET),
    (re.compile(_GLOBAL + r'EventSource\b'), 'EventSource', _NO_NET),
    (re.compile(r'\bsendBeacon\s*\('), 'sendBeacon()', _NO_NET),
    (re.compile(r'(?<![\w$.])importScripts\s*\('), 'importScripts()', _NO_NET),
    (re.compile(r'(?<![\w$.])new\s+(?:Shared)?Worker\s*\('), 'a Worker', 'a pack runs in one context, with no workers'),
    (re.compile(r'(?<![\w$.])import\s*\('), 'import()', _NO_NET),
    (re.compile(r'''(?<![\w$.])import\b(?=\s*[\w$*{"'`.])'''), 'an import statement',
     'scripts are plain scripts, run in the order "scripts" lists them (no modules)'),
]
ANY_URL = re.compile(r'''(?:https?|wss?)://[^\s"'`<>)\]]+''')
# a string that is the value of a `url` key: a source link, as terra.panel's sources give them
SOURCE_KEY = re.compile(r'''(?:(?<![\w$])url|["']url["'])\s*:\s*$''')


def scan_script(rel, text, files, rep, where):
    """The offline boundary in one script: no network and no code from outside the pack (errors);
    links outside a source's `url` (warnings: a link is fine to cite, and never loaded); pictures it
    names that are not in the pack (warnings)."""
    code, strings, comments = js_scan(text)
    for pattern, what, why in NO_NETWORK:
        for m in pattern.finditer(code):
            rep.error(f'{where}/{rel}:{_line_of(text, m.start())}', f'{what}: {why}')
    said = set()   # each missing picture and each link once a file: data files repeat them

    def mention(pos, url):
        if url not in said:
            said.add(url)
            rep.warn(f'{where}/{rel}:{_line_of(text, pos)}',
                     f'mentions {url[:60]}: cite a link in SOURCES.md or as a terra.panel source\'s url (it is never loaded)')

    for start, end, value in strings:
        if value.lower().endswith(IMAGE_EXTS) and safe_path(value) and value not in files and value not in said:
            said.add(value)
            rep.warn(f'{where}/{rel}:{_line_of(text, start)}', f'refers to "{value}", which is not in the pack')
        if not SOURCE_KEY.search(text[max(0, start - 40):start]):
            for m in ANY_URL.finditer(value):
                mention(start, m.group(0))
    for start, end in comments:
        for m in ANY_URL.finditer(text, start, end):
            mention(m.start(), m.group(0))


def _check_str(rep, where, m, key, lo, hi):
    v = m.get(key)
    if not isinstance(v, str):
        rep.error(where, f'"{key}" is required and must be a string')
    elif not lo <= len(v) <= hi or not v.strip():
        rep.error(where, f'"{key}" must be {lo}-{hi} characters (it has {len(v.strip())})')


def _check_num(rep, where, name, v, lo, hi):
    if not is_num(v):
        rep.error(where, f'{name} must be a number')
        return False
    if not lo <= v <= hi:
        rep.error(where, f'{name} {v} is outside {lo} to {hi}')
        return False
    return True


def _check_file(rep, where, key, ref, files, exts=None, kind='file'):
    if not isinstance(ref, str) or not ref:
        rep.error(where, f'{key} must be a path inside the pack')
        return False
    if not safe_path(ref):
        rep.error(where, f'{key} "{ref}" must be a relative path of letters, digits, . _ - and / with no hidden segment')
        return False
    if ref not in files:
        rep.error(where, f'{key} "{ref}" does not exist in the pack')
        return False
    if exts and Path(ref).suffix.lower() not in exts:
        rep.error(where, f'{key} "{ref}" must be {" or ".join(exts)}')
        return False
    return True


def _check_image_bytes(rep, where, key, path):
    head = Path(path).read_bytes()[:8]
    ext = Path(path).suffix.lower()
    ok = head.startswith(b'\x89PNG\r\n\x1a\n') if ext == '.png' else head.startswith(b'\xff\xd8\xff')
    if not ok:
        rep.error(where, f'{key} is not really a {ext[1:].upper()} file')


def _check_zooms(rep, where, tag, obj):
    lo_z = hi_z = None
    for k in ('minZoom', 'maxZoom'):
        if k in obj and _check_num(rep, where, f'{tag}.{k}', obj[k], 0.8, 8):
            if k == 'minZoom':
                lo_z = obj[k]
            else:
                hi_z = obj[k]
    if lo_z is not None and hi_z is not None and lo_z > hi_z:
        rep.error(where, f'{tag}.minZoom {lo_z} is above maxZoom {hi_z}')


def png_size(path):
    """A PNG's width and height from its header, or None."""
    head = Path(path).read_bytes()[:24]
    if not head.startswith(b'\x89PNG\r\n\x1a\n'):
        return None
    return int.from_bytes(head[16:20], 'big'), int.from_bytes(head[20:24], 'big')


def _check_picture(rep, where, key, ref, files, runtime=None):
    """A picture of the pack, or (runtime 6) a frame of a sprite sheet: 'sheet.png#x,y,w,h', a
    rectangle of whole pixels inside the sheet."""
    if isinstance(ref, str) and '#' in ref:
        m = SHEET_FRAME.fullmatch(ref)
        if not m or int(m[4]) < 1 or int(m[5]) < 1:
            rep.error(where, f'{key} "{ref}" names a frame of a sprite sheet as sheet.png#x,y,width,height (whole pixels)')
            return
        if is_int(runtime) and runtime < SHEET_RUNTIME:
            rep.error(where, f'{key} is a frame of a sprite sheet, new in runtime {SHEET_RUNTIME} (docs/runtime3.md › Sprites): '
                             f'declare "runtime": {SHEET_RUNTIME} or later')
        if _check_file(rep, where, key, m[1], files, ('.png',)):
            size = png_size(files[m[1]])
            x, y, w, h = (int(m[i]) for i in range(2, 6))
            if size is None:
                rep.error(where, f'{key}: its sheet is not really a PNG file')
            elif x + w > size[0] or y + h > size[1]:
                rep.error(where, f'{key} "{ref}" lies outside its sheet ({size[0]} × {size[1]})')
        return
    if _check_file(rep, where, key, ref, files, IMAGE_EXTS):
        _check_image_bytes(rep, where, key, files[ref])


def check_look(rep, where, tag, look, files, runtime=None):
    """A look, as a sprite's (docs/runtime3.md › Sprites): one picture or frames, a size in points, a
    zoom range, an occasional showing (`every`/`for`), `face`."""
    if not isinstance(look, dict):
        rep.error(where, f'{tag} must be an object {{ image or frames, size, minZoom, maxZoom, every, for }}')
        return
    for k in look:
        if k not in LOOK_KEYS:
            rep.warn(where, f'unknown key "{tag}.{k}"')
    if ('image' in look) == ('frames' in look):
        rep.error(where, f'{tag} needs one picture ("image") or a list of them shown in turn ("frames"), not both')
    if 'image' in look:
        _check_picture(rep, where, f'{tag}.image', look['image'], files, runtime)
    if 'frames' in look:
        frames = look['frames']
        if not isinstance(frames, list) or not 1 <= len(frames) <= MAX_FRAMES:
            rep.error(where, f'{tag}.frames must be a list of 1 to {MAX_FRAMES} pictures')
        else:
            for j, f in enumerate(frames):
                _check_picture(rep, where, f'{tag}.frames[{j}]', f, files, runtime)
        if 'fps' not in look:
            rep.error(where, f'{tag}.fps is required with "frames": how many are shown a second')
    if 'fps' in look:
        if 'frames' not in look:
            rep.warn(where, f'{tag}.fps only times "frames"')
        _check_num(rep, where, f'{tag}.fps', look['fps'], 0.1, 60)
    size = look.get('size')
    if 'size' not in look:
        rep.error(where, f'{tag}.size is required: [width, height] in points')
    elif not (isinstance(size, list) and len(size) == 2):
        rep.error(where, f'{tag}.size must be [width, height] in points')
    else:
        for name, v in zip(('width', 'height'), size):
            _check_num(rep, where, f'{tag}.size {name}', v, 1, 512)
    _check_zooms(rep, where, tag, look)
    timed = [k for k in ('every', 'for') if k in look and _check_num(rep, where, f'{tag}.{k}', look[k], 0.05, 3600)]
    if ('every' in look) != ('for' in look):
        rep.warn(where, f'{tag}: "every" and "for" go together (it shows for "for" seconds every "every")')
    elif len(timed) == 2 and look['for'] > look['every']:
        rep.error(where, f'{tag}.for {look["for"]} is longer than every {look["every"]}')
    if 'face' in look and not isinstance(look['face'], bool):
        rep.error(where, f'{tag}.face must be true or false')


def validate_manifest(m, name, files, rep, where):
    """Checks one prototype.json against docs/runtime3.md. Returns True when it is a runtime 1 or 2
    manifest (one error says so; nothing more is checked)."""
    if not isinstance(m, dict):
        rep.error(where, 'prototype.json must be a JSON object')
        return False
    runtime = m.get('runtime')
    if is_int(runtime) and 1 <= runtime < RUNTIMES[0]:
        rep.error(where, f'"runtime" is {runtime}: {R3_ONLY} (docs/runtime3.md). Its pages become scripts; '
                         'its markers, a doorway or sprites')
        return True
    for k in m:
        if k not in TOP_KEYS and k not in WEB_KEYS:
            rep.warn(where, f'unknown key "{k}"')

    if not ID_RE.fullmatch(name):
        rep.error(where, f'the folder name "{name}" must match {ID_RE.pattern} (lower-case letters, digits, hyphens; 2-40)')
    pid = m.get('id')
    if not isinstance(pid, str):
        rep.error(where, '"id" is required and must be a string')
    elif pid != name:
        rep.error(where, f'"id" ("{pid}") must equal the folder name ("{name}")')
    _check_str(rep, where, m, 'title', 1, 60)
    _check_str(rep, where, m, 'summary', 1, 200)
    if not is_int(runtime) or runtime not in RUNTIMES:
        rep.error(where, f'"runtime" must be {RUNTIMES[0]} to {RUNTIME} (the newest this repository\'s app hosts; '
                         f'4 or {RUNTIME} for what docs/runtime3.md marks new in them)')
    pres = m.get('presentation')
    if pres == 'screen':
        rep.error(where, f'"presentation" is "screen": {R3_ONLY}, and there are no pack screens ("globe")')
    elif pres != 'globe':
        rep.error(where, '"presentation" must be "globe"')
    for k, what in WEB_KEYS.items():
        if k in m:
            rep.error(where, f'"{k}" ({what}) is runtime 1 and 2: {R3_ONLY}')
    if 'shelf' in m and m['shelf'] not in ('current', 'parked'):
        rep.error(where, '"shelf" must be "current" or "parked"')
    if 'taps' in m and not isinstance(m['taps'], bool):
        rep.error(where, '"taps" must be true or false')

    scripts = m.get('scripts')
    if 'scripts' not in m:
        rep.error(where, '"scripts" is required: the pack\'s .js files, run in order (e.g. ["main.js"])')
    elif not isinstance(scripts, list) or not scripts:
        rep.error(where, '"scripts" must be a non-empty list of the pack\'s .js files, run in order')
    else:
        seen = set()
        for i, s in enumerate(scripts):
            if _check_file(rep, where, f'scripts[{i}]', s, files, ('.js',)):
                if s in seen:
                    rep.error(where, f'scripts[{i}] "{s}" is listed twice')
                seen.add(s)

    anchor = m.get('anchor')
    if 'anchor' in m:
        if not isinstance(anchor, dict):
            rep.error(where, '"anchor" must be an object { lon, lat, zoom }')
        else:
            for k in anchor:
                if k not in ANCHOR_KEYS:
                    rep.warn(where, f'unknown key "anchor.{k}"')
            for k, lo, hi in (('lon', -180, 180), ('lat', -90, 90), ('zoom', 0.8, 8)):
                if k not in anchor:
                    rep.error(where, f'anchor.{k} is required')
                else:
                    _check_num(rep, where, f'anchor.{k}', anchor[k], lo, hi)

    door = m.get('doorway')
    if 'doorway' in m:
        if not isinstance(door, dict):
            rep.error(where, '"doorway" must be an object { lon, lat, label, look }')
        else:
            for k in door:
                if k not in DOORWAY_KEYS:
                    rep.warn(where, f'unknown key "doorway.{k}"')
            for k, lo, hi in (('lon', -180, 180), ('lat', -90, 90)):
                if k not in door:
                    rep.error(where, f'doorway.{k} is required')
                else:
                    _check_num(rep, where, f'doorway.{k}', door[k], lo, hi)
            label = door.get('label')
            if not isinstance(label, str) or not label.strip() or len(label) > 200:
                rep.error(where, 'doorway.label is required: the words VoiceOver speaks (1-200 characters)')
            life = [k for k in DOORWAY_LIFE_KEYS if k in door]
            if life and is_int(runtime) and runtime < DOORWAY_LIFE_RUNTIME:
                rep.error(where, f'doorway.{life[0]} is new in runtime {DOORWAY_LIFE_RUNTIME} (docs/runtime3.md › Many at once): '
                                 f'declare "runtime": {DOORWAY_LIFE_RUNTIME} or later')
            if 'look' not in door and 'looks' not in door:
                rep.error(where, 'doorway.look is required: how it shows on the globe (as a sprite\'s look), '
                                 'or doorway.looks, its looks by zoom')
            elif 'look' in door and 'looks' in door:
                rep.error(where, 'doorway: one "look", or "looks" by zoom, not both')
            else:
                looks = door['looks'] if 'looks' in door else [door['look']]
                if 'looks' in door and not (isinstance(looks, list) and 1 <= len(looks) <= 8):
                    rep.error(where, 'doorway.looks must be a list of 1 to 8 looks (as a sprite\'s: the first whose zoom range holds is drawn)')
                    looks = []
                for i, look in enumerate(looks):
                    check_look(rep, where, f'doorway.looks[{i}]' if 'looks' in door else 'doorway.look', look, files, runtime)
                for i, look in enumerate(looks):
                    if isinstance(look, dict) and 'minZoom' not in look and 'every' not in look:
                        tag = f'doorway.looks[{i}]' if 'looks' in door else 'doorway.look'
                        rep.warn(where, f'{tag} shows on the whole Earth: keep that view sparse with a minZoom, '
                                        'or make it a small trace (every/for)')
                        break
            if 'path' in door:
                check_path(rep, where, 'doorway.path', door['path'])
            if 'breathes' in door and not isinstance(door['breathes'], bool):
                rep.error(where, 'doorway.breathes must be true or false')
        if m.get('shelf') == 'parked':
            rep.warn(where, 'a parked pack shows no doorway')
    return False


def check_path(rep, where, tag, path):
    """A way along great circles, as a sprite's (docs/runtime3.md › Sprites): coordinates [[lon, lat], …]
    (2 to 200 of them), the seconds the whole of it takes, and loop true, false or "back"."""
    if not isinstance(path, dict):
        rep.error(where, f'{tag} must be an object {{ coordinates: [[lon, lat], …], seconds, loop }}')
        return
    for k in path:
        if k not in PATH_KEYS:
            rep.warn(where, f'unknown key "{tag}.{k}"')
    coords = path.get('coordinates')
    if not isinstance(coords, list) or not 2 <= len(coords) <= 200:
        rep.error(where, f'{tag}.coordinates must be a list of 2 to 200 places [lon, lat]')
    else:
        for i, c in enumerate(coords):
            if not (isinstance(c, list) and len(c) == 2 and all(is_num(v) for v in c)):
                rep.error(where, f'{tag}.coordinates[{i}] must be [lon, lat]')
                break
            _check_num(rep, where, f'{tag}.coordinates[{i}] lon', c[0], -540, 540)
            _check_num(rep, where, f'{tag}.coordinates[{i}] lat', c[1], -90, 90)
    if 'seconds' in path:
        _check_num(rep, where, f'{tag}.seconds', path['seconds'], 1, 86400)
    if 'loop' in path and path['loop'] not in (True, False, 'back'):
        rep.error(where, f'{tag}.loop must be true, false or "back"')


def validate_pack(folder, rep):
    """Everything docs/runtime3.md asks of one Prototypes/<id>/ folder. Returns (manifest, files),
    or (None, files) when prototype.json could not be read."""
    folder = Path(folder)
    where = display(folder)
    if folder.is_symlink() or not folder.is_dir():
        rep.error(where, 'not a folder')
        return None, {}
    files = scan_files(folder, rep)
    manifest_file = folder / 'prototype.json'
    if 'prototype.json' not in files:
        rep.error(where, 'prototype.json is missing')
        return None, files
    try:
        manifest = load_json(manifest_file)
    except (ValueError, UnicodeDecodeError) as e:
        rep.error(f'{where}/prototype.json', f'cannot be read: {e}')
        return None, files
    if 'terra.js' in {Path(f).name for f in files}:
        rep.error(where, 'a pack must not contain its own terra.js: the host provides the runtime')
    if SHARED_NAME in files and SHARED.is_file() and files[SHARED_NAME].read_bytes() != SHARED.read_bytes():
        rep.error(f'{where}/{SHARED_NAME}', f'differs from {display(SHARED)}, the packs\' shared script: '
                                            f'run `playground.py share` (change the shared script there, never a copy)')
    if validate_manifest(manifest, folder.name, files, rep, f'{where}/prototype.json'):
        return manifest, files
    listed = manifest.get('scripts') if isinstance(manifest, dict) else None
    listed = set(listed) if isinstance(listed, list) and all(isinstance(s, str) for s in listed) else None
    if 'SOURCES.md' not in files:
        rep.warn(where, 'SOURCES.md is missing: where its facts and pictures come from, and what it does not claim')
    for rel, full in files.items():
        low = rel.lower()
        if low.endswith(PAGE_EXTS):
            rep.error(f'{where}/{rel}', f'a web page: {R3_ONLY}, and nothing opens a page (move what it does into the scripts)')
        elif low.endswith(SCRIPT_EXTS):
            if listed is not None and low.endswith('.js') and rel not in listed and Path(rel).name != 'terra.js':
                rep.warn(f'{where}/{rel}', 'is not in "scripts", so it never runs (terra.read can still read it)')
            scan_script(rel, full.read_text(encoding='utf-8', errors='replace'), files, rep, where)
    return manifest, files


def url_ok(url):
    """HTTPS, or plain http only to this machine (the simulator's local server)."""
    u = urllib.parse.urlparse(url)
    if not u.hostname or u.username or u.password:
        return False
    return u.scheme == 'https' or (u.scheme == 'http' and u.hostname in ('127.0.0.1', 'localhost'))


def validate_sources(path, rep):
    where = display(path)
    try:
        data = load_json(path)
    except FileNotFoundError:
        rep.error(where, 'is missing')
        return
    except (ValueError, UnicodeDecodeError) as e:
        rep.error(where, f'cannot be read: {e}')
        return
    if not isinstance(data, dict):
        rep.error(where, 'must be a JSON object')
        return
    for k in data:
        if k not in ('format', 'sources'):
            rep.warn(where, f'unknown key "{k}"')
    if not is_int(data.get('format')) or data.get('format') != 1:
        rep.error(where, '"format" must be the integer 1')
    sources = data.get('sources')
    if not isinstance(sources, list):
        rep.error(where, '"sources" must be a list')
        return
    names = set()
    for i, s in enumerate(sources):
        tag = f'sources[{i}]'
        if not isinstance(s, dict):
            rep.error(where, f'{tag} must be an object')
            continue
        for k in s:
            if k not in ('name', 'url'):
                rep.warn(where, f'unknown key "{tag}.{k}"')
        name = s.get('name')
        if not isinstance(name, str) or not name.strip():
            rep.error(where, f'{tag}.name is required')
        elif name in names:
            rep.error(where, f'{tag}.name "{name}" is used twice')
        else:
            names.add(name)
        url = s.get('url')
        if not isinstance(url, str) or not url_ok(url):
            rep.error(where, f'{tag}.url must be an https:// URL (http only for 127.0.0.1 or localhost), without credentials')
        elif not urllib.parse.urlparse(url).path.endswith('/catalog.json'):
            rep.error(where, f'{tag}.url must end in /catalog.json')


def validate_all(ids, rep, protos=None):
    """Validate the named prototypes, or every one (and sources.json). Returns how many were checked."""
    protos = Path(protos or PROTOTYPES)
    folders = [protos / i for i in ids] if ids else prototype_dirs(protos)
    for f in folders:
        validate_pack(f, rep)
    if not ids and (protos / '_runtime').is_dir():
        validate_sources(protos / '_runtime' / 'sources.json', rep)
    return len(folders)


# ---------------------------------------------------------------- packs and the catalog

def content_hash(pairs):
    """sha256 of "<path> <sha256>\\n" for each file, in byte order of the POSIX path."""
    h = hashlib.sha256()
    for path, digest in sorted(pairs, key=lambda p: p[0].encode('utf-8')):
        h.update(f'{path} {digest}\n'.encode('utf-8'))
    return h.hexdigest()


class Pack:
    def __init__(self, folder, manifest, files):
        self.folder = Path(folder)
        self.manifest = manifest
        self.id = manifest['id']
        self.files = []  # dicts: path, bytes, sha256, data (sorted by path)
        for rel in sorted(files, key=lambda p: p.encode('utf-8')):
            data = Path(files[rel]).read_bytes()
            self.files.append({'path': rel, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(), 'data': data})
        self.hash = content_hash((f['path'], f['sha256']) for f in self.files)
        self.bytes = sum(f['bytes'] for f in self.files)


def load_packs(folders, rep):
    """Validate each folder; the Pack of every valid one. (A folder with errors is reported, not packed.)"""
    packs = []
    for f in folders:
        before = rep.errors
        manifest, files = validate_pack(f, rep)
        if manifest is not None and rep.errors == before:
            packs.append(Pack(f, manifest, files))
    ids = [p.id for p in packs]
    for i in sorted({i for i in ids if ids.count(i) > 1}):
        rep.error(i, 'is given twice')
    return packs


def empty_catalog():
    return {'format': 1, 'generated': None, 'packs': []}


def parse_catalog(text, origin):
    try:
        data = json.loads(text)
    except ValueError as e:
        raise ToolError(f'{origin}: not valid JSON ({e})')
    if not isinstance(data, dict) or data.get('format') != 1 or not isinstance(data.get('packs'), list):
        raise ToolError(f'{origin}: not a catalog this tool understands (format 1 with a "packs" list)')
    for e in data['packs']:
        if not (isinstance(e, dict) and isinstance(e.get('id'), str) and is_int(e.get('revision'))):
            raise ToolError(f'{origin}: a pack entry lacks its id or revision')
    return data


def make_entry(pack, revision, published, retired=False):
    m = pack.manifest
    return {
        'id': pack.id, 'revision': revision, 'runtime': m['runtime'], 'title': m['title'], 'summary': m['summary'],
        'presentation': m['presentation'], 'shelf': m.get('shelf', 'current'), 'hash': pack.hash,
        'path': f'packs/{pack.id}/r{revision}/', 'bytes': pack.bytes,
        'files': [{'path': f['path'], 'bytes': f['bytes'], 'sha256': f['sha256']} for f in pack.files],
        'published': published, 'retired': retired,
    }


class Result:
    def __init__(self, pack, status, revision, before):
        self.pack, self.status, self.revision, self.before = pack, status, revision, before

    @property
    def changed(self):
        return self.status != 'same'


def plan(packs, base, now, floors=None):
    """Decide each pack's revision against the base catalog. Returns (catalog, results).
    New id: revision 1. Same content hash: the base entry, untouched (so `published` stays).
    Different: the base revision + 1. `floors` ({id: the highest revision already stored}) lifts a
    new revision above any stored one, so no revision's files are ever replaced. Entries for packs
    not packed are kept as they are. A retired pack stays retired until `retire --undo`."""
    entries = {e['id']: e for e in base['packs']}
    floors = floors or {}
    results = []
    for p in packs:
        old = entries.get(p.id)
        floor = floors.get(p.id, 0)
        if old is None:
            results.append(Result(p, 'new', floor + 1, None))
            entries[p.id] = make_entry(p, floor + 1, now)
        elif old.get('hash') == p.hash:
            results.append(Result(p, 'same', old['revision'], old['revision']))
        else:
            revision = max(old['revision'], floor) + 1
            results.append(Result(p, 'updated', revision, old['revision']))
            entries[p.id] = make_entry(p, revision, now, bool(old.get('retired', False)))
    changed = any(r.changed for r in results)
    catalog = {
        'format': 1,
        'generated': now if changed or not base.get('generated') else base['generated'],
        'packs': [entries[k] for k in sorted(entries)],
    }
    return catalog, results


def open_page(pack_id, title):
    t, link = html.escape(title), f'{OPEN_SCHEME}://open/{pack_id}'
    return f'''<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{t} - Terra Incognita</title>
<style>
  :root {{ --paper: #F6F2E7; --ink: #3B2C16; --quiet: #5A4A33; --link: #0A5A80; }}
  html {{ background: var(--paper); color: var(--ink); color-scheme: light; }}
  body {{ margin: 0; font: 18px/1.5 ui-serif, "New York", Georgia, serif; }}
  main {{ max-width: 34rem; margin: 0 auto; padding: 18vh 20px 32px; }}
  h1 {{ font-size: 28px; line-height: 1.2; margin: 0 0 .5em; }}
  a {{ color: var(--link); }}
  .quiet {{ color: var(--quiet); font-size: 16px; }}
</style>
</head>
<body>
<main>
<h1>{t}</h1>
<p>Opening it in Terra Incognita.</p>
<p><a href="{link}">Open {t} in Terra Incognita</a></p>
<p class="quiet">Nothing happened? This link needs the Terra Incognita development build installed on this device. Install the app first, then come back to this page.</p>
</main>
<script>location.href = "{link}";</script>
</body>
</html>
'''


def atomic_write(path, data):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name + '.')
    try:
        with os.fdopen(fd, 'wb') as f:
            f.write(data)
        os.replace(tmp, path)
    except BaseException:
        if os.path.exists(tmp):
            os.unlink(tmp)
        raise


def catalog_bytes(catalog):
    return (json.dumps(catalog, indent=2, ensure_ascii=False) + '\n').encode('utf-8')


def _dir_matches(dest, pack):
    if not dest.is_dir():
        return False
    have = {p.relative_to(dest).as_posix() for p in dest.rglob('*') if p.is_file()}
    if have != {f['path'] for f in pack.files}:
        return False
    return all((dest / f['path']).read_bytes() == f['data'] for f in pack.files)


def _write_revision(dest, pack):
    shutil.rmtree(dest, ignore_errors=True)
    for f in pack.files:
        (dest / f['path']).parent.mkdir(parents=True, exist_ok=True)
        (dest / f['path']).write_bytes(f['data'])


def write_tree(out, results, catalog):
    """The packed output: packs/<id>/r<rev>/, open/<id>.html, and catalog.json last (atomically), so
    a catalog never names files that are not there yet."""
    out = Path(out)
    for r in results:
        dest = out / 'packs' / r.pack.id / f'r{r.revision}'
        if not _dir_matches(dest, r.pack):
            _write_revision(dest, r.pack)
        atomic_write(out / 'open' / f'{r.pack.id}.html', open_page(r.pack.id, r.pack.manifest['title']).encode('utf-8'))
    atomic_write(out / 'catalog.json', catalog_bytes(catalog))


IMMUTABLE = 'public, max-age=31536000, immutable'
NO_CACHE = 'no-cache'
CATALOG_CACHE = 'no-cache, max-age=0'


def uploads(results, catalog, open_pages=True):
    """What a publish writes, in order, whatever the store (a folder, a git branch, a bucket):
    [(key, bytes, content type, cache control)]: each changed pack's files under packs/<id>/r<rev>/
    (a new revision's: a published revision's files never change), its open page when the store
    serves web pages, and catalog.json last, so a reader never finds a catalog naming files that are
    not there yet. Empty when nothing changed."""
    changed = [r for r in results if r.changed]
    out = []
    for r in changed:
        for f in r.pack.files:
            out.append((f'packs/{r.pack.id}/r{r.revision}/{f["path"]}', f['data'], content_type(f['path']), IMMUTABLE))
        if open_pages:
            page = open_page(r.pack.id, r.pack.manifest['title']).encode('utf-8')
            out.append((f'open/{r.pack.id}.html', page, content_type('x.html'), NO_CACHE))
    if changed:
        out.append(('catalog.json', catalog_bytes(catalog), 'application/json', CATALOG_CACHE))
    return out


def stored_revisions(folder):
    """{id: the highest r<n> folder} under <folder>/packs/: revisions a new one must stay above."""
    out = {}
    base = Path(folder) / 'packs'
    if not base.is_dir():
        return out
    for d in base.iterdir():
        if d.is_dir():
            revs = [int(m.group(1)) for r in d.iterdir() if r.is_dir() and (m := REVISION_DIR.fullmatch(r.name))]
            if revs:
                out[d.name] = max(revs)
    return out


# the published tree is bytes the catalog has hashed: git must never convert a line ending in it
GITATTRIBUTES = b'# Written by tools/playground/playground.py: every file is kept byte for byte (the catalog has its SHA-256).\n* -text\n'


def publish_dir(packs, into, now, dry_run=False):
    """Publishes into a folder that holds a published tree (a checkout of the `packs` branch, or an
    empty folder for the first time): reads its catalog.json, gives each pack whose contents changed
    the next revision (never one already stored there), writes packs/<id>/r<rev>/ and then
    catalog.json, with paths relative to it. Unchanged packs keep their revision; if one's stored
    files are missing or altered they are restored from the pack, which the catalog's hash vouches
    for. Retired entries stay retired. Writes nothing on a dry run. Returns (catalog, results,
    restored), restored being the unchanged results whose files were (or would be) rewritten."""
    into = Path(into)
    existing = into / 'catalog.json'
    base = parse_catalog(existing.read_text(encoding='utf-8'), display(existing)) if existing.is_file() else empty_catalog()
    catalog, results = plan(packs, base, now, stored_revisions(into))
    restored = [r for r in results if not r.changed
                and not _dir_matches(into / 'packs' / r.pack.id / f'r{r.revision}', r.pack)]
    if dry_run:
        return catalog, results, restored
    for r in results:
        if r.changed and (into / 'packs' / r.pack.id / f'r{r.revision}').exists():
            raise ToolError(f'packs/{r.pack.id}/r{r.revision} already exists: a revision\'s files never change')
    for r in restored:
        _write_revision(into / 'packs' / r.pack.id / f'r{r.revision}', r.pack)
    for key, data, _, _ in uploads(results, catalog, open_pages=False):
        if key == 'catalog.json':
            atomic_write(into / key, data)
        else:
            (into / key).parent.mkdir(parents=True, exist_ok=True)
            (into / key).write_bytes(data)
    if not existing.is_file():
        atomic_write(existing, catalog_bytes(catalog))
    attributes = into / '.gitattributes'
    if not attributes.is_file() or attributes.read_bytes() != GITATTRIBUTES:
        atomic_write(attributes, GITATTRIBUTES)
    return catalog, results, restored


# ---------------------------------------------------------------- network

def fetch_url(url, timeout=30):
    """GET a public URL: bytes, or None for a 404."""
    if not url_ok(url):
        raise ToolError(f'{url}: only https:// URLs (or http://127.0.0.1) are fetched')
    req = urllib.request.Request(url, headers={'User-Agent': USER_AGENT, 'Cache-Control': 'no-cache'})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.read()
    except urllib.error.HTTPError as e:
        if e.code == 404:
            return None
        raise ToolError(f'{url}: the server answered {e.code} {e.reason}')
    except (urllib.error.URLError, OSError) as e:
        raise ToolError(f'{url}: could not be reached ({getattr(e, "reason", e)})')


def _hmac(key, msg):
    return hmac.new(key, msg.encode('utf-8'), hashlib.sha256).digest()


def _enc(s):
    return urllib.parse.quote(s, safe='-_.~')


def canonical_path(path):
    """Each segment URI-encoded once (S3 does not encode twice), slashes kept."""
    return '/'.join(_enc(seg) for seg in path.split('/'))


def sigv4(method, path, query, headers, payload_hash, access_key, secret_key, region, service, amz_date):
    """AWS Signature Version 4, header form. `headers` are exactly the ones to sign (host,
    x-amz-content-sha256, x-amz-date, ...); `path` is the raw path. Returns (Authorization, canonical
    request, string to sign)."""
    names = sorted(h.lower() for h in headers)
    lowered = {h.lower(): ' '.join(str(v).split()) for h, v in headers.items()}
    signed = ';'.join(names)
    canon_query = '&'.join(f'{_enc(k)}={_enc(v)}' for k, v in sorted((query or {}).items()))
    canonical = '\n'.join([
        method, canonical_path(path), canon_query,
        ''.join(f'{n}:{lowered[n]}\n' for n in names), signed, payload_hash,
    ])
    day = amz_date[:8]
    scope = f'{day}/{region}/{service}/aws4_request'
    to_sign = '\n'.join(['AWS4-HMAC-SHA256', amz_date, scope, hashlib.sha256(canonical.encode('utf-8')).hexdigest()])
    key = _hmac(_hmac(_hmac(_hmac(('AWS4' + secret_key).encode('utf-8'), day), region), service), 'aws4_request')
    signature = hmac.new(key, to_sign.encode('utf-8'), hashlib.sha256).hexdigest()
    auth = f'AWS4-HMAC-SHA256 Credential={access_key}/{scope},SignedHeaders={signed},Signature={signature}'
    return auth, canonical, to_sign


class R2:
    """Just enough S3 (path-style, SigV4) for Cloudflare R2: GET and PUT one object."""

    def __init__(self, account, key_id, secret, bucket, endpoint=None, region='auto'):
        self.key_id, self.secret, self.bucket, self.region = key_id, secret, bucket, region
        self.endpoint = (endpoint or f'https://{account}.r2.cloudflarestorage.com').rstrip('/')
        self.host = urllib.parse.urlparse(self.endpoint).netloc

    @classmethod
    def from_env(cls, env):
        missing = [v for v in R2_VARS[:4] if not env.get(v)]
        if missing:
            raise ToolError('missing environment: ' + ', '.join(missing))
        return cls(env['R2_ACCOUNT_ID'], env['R2_ACCESS_KEY_ID'], env['R2_SECRET_ACCESS_KEY'], env['R2_BUCKET'],
                   env.get('R2_ENDPOINT'))

    def request(self, method, key, body=b'', extra=None, attempts=3):
        path = f'/{self.bucket}/{key}'
        for attempt in range(1, attempts + 1):
            amz_date = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')
            payload = hashlib.sha256(body).hexdigest()
            signed = {'host': self.host, 'x-amz-content-sha256': payload, 'x-amz-date': amz_date}
            auth, _, _ = sigv4(method, path, {}, signed, payload, self.key_id, self.secret, self.region, 's3', amz_date)
            req = urllib.request.Request(self.endpoint + canonical_path(path), method=method,
                                         data=body if method == 'PUT' else None)
            req.add_header('x-amz-content-sha256', payload)
            req.add_header('x-amz-date', amz_date)
            req.add_header('Authorization', auth)
            req.add_header('User-Agent', USER_AGENT)
            for k, v in (extra or {}).items():
                req.add_header(k, v)
            try:
                with urllib.request.urlopen(req, timeout=60) as r:
                    return r.status, r.read()
            except urllib.error.HTTPError as e:
                detail = e.read()[:400].decode('utf-8', 'replace')
                code = (re.search(r'<Code>([^<]+)</Code>', detail) or [None, e.reason])[1]
                if e.code == 404 and method == 'GET' and code != 'NoSuchBucket':
                    return 404, b''
                if e.code >= 500 and attempt < attempts:
                    time.sleep(attempt)
                    continue
                hint = ' (check the token: it needs Object Read & Write on this bucket)' if e.code in (401, 403) else ''
                raise ToolError(f'{method} {key}: {self.host} answered {e.code} {code}{hint}')
            except (urllib.error.URLError, OSError) as e:
                if attempt < attempts:
                    time.sleep(attempt)
                    continue
                raise ToolError(f'{method} {key}: cannot reach {self.host} ({getattr(e, "reason", e)})')

    def get(self, key):
        status, body = self.request('GET', key)
        return None if status == 404 else body

    def put(self, key, body, ctype, cache):
        self.request('PUT', key, body, {'Content-Type': ctype, 'Cache-Control': cache})


def public_url(env, path):
    base = (env.get('PACKS_PUBLIC_URL') or '').rstrip('/')
    return f'{base or "<PACKS_PUBLIC_URL>"}/{path}'


# ---------------------------------------------------------------- commands

def default_title(pid):
    t = pid.replace('-', ' ')
    return t[:1].upper() + t[1:]


def _substitute(name, data, pid, title):
    """{{id}} and {{title}} in a template file. In JSON and scripts the title lands inside a
    double-quoted string, escaped as one; Markdown takes it as it is."""
    ext = Path(name).suffix.lower()
    if ext not in ('.json', '.js', '.md'):
        return data
    text = data.decode('utf-8')
    t = json.dumps(title)[1:-1] if ext in ('.json', '.js') else title
    return text.replace('{{id}}', pid).replace('{{title}}', t).encode('utf-8')


def cmd_new(args):
    pid = args.id
    if not ID_RE.fullmatch(pid):
        raise ToolError(f'"{pid}" is not a valid id: lower-case letters, digits and hyphens, 2-40 characters, '
                        'starting and ending with a letter or digit')
    title = args.title if args.title is not None else default_title(pid)
    if not 1 <= len(title.strip()) <= 60:
        raise ToolError('--title must be 1-60 characters')
    dest = PROTOTYPES / pid
    if dest.exists():
        raise ToolError(f'{display(dest)} already exists (ids are never reused; pick another)')
    if args.screen:
        print('note: --screen makes a globe pack too: in runtime 3 every pack lives on the globe (there are no pack screens).')
    kind = 'globe'
    src = TEMPLATES / kind
    if not src.is_dir():
        raise ToolError(f'template missing: {display(src)}')
    try:
        for f in sorted(src.rglob('*')):
            if f.is_file():
                target = dest / f.relative_to(src)
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(_substitute(f.name, f.read_bytes(), pid, title.strip()))
    except BaseException:
        shutil.rmtree(dest, ignore_errors=True)
        raise
    rep = Report()
    validate_pack(dest, rep)
    rep.print()
    if rep.errors:
        raise ToolError('the scaffold does not validate; this is a bug in the template')
    print(f'Created {display(dest)}/ (runtime {RUNTIMES[0]}: prototype.json, main.js, SOURCES.md, marker.png).')
    print(f'  preview   node tools/playground/serve.cjs      then  http://127.0.0.1:8790/preview/{pid}')
    print(f'  check     node tools/playground/check.cjs {pid}')
    print('  validate  python3 tools/playground/playground.py validate')
    return 0


def cmd_validate(args):
    rep = Report()
    count = validate_all(args.ids, rep)
    rep.print()
    what = f'{count} prototype{"" if count == 1 else "s"}' + ('' if args.ids else ' and sources.json')
    print(f'Checked {what}: {rep.errors} error{"" if rep.errors == 1 else "s"}, '
          f'{rep.warnings} warning{"" if rep.warnings == 1 else "s"}.')
    return 1 if rep.errors else 0


def select_folders(ids, extra):
    folders = [PROTOTYPES / i for i in ids] + [Path(f) for f in extra or []]
    if not folders:
        folders = prototype_dirs()
    return folders


def load_base(base, out):
    """The catalog to revise: --base (file or https URL), else <out>/catalog.json, else empty."""
    if base:
        if re.match(r'https?://', base):
            data = fetch_url(base)
            return empty_catalog() if data is None else parse_catalog(data.decode('utf-8', 'replace'), base)
        p = Path(base)
        if p.is_dir():
            p = p / 'catalog.json'
        if not p.is_file():
            raise ToolError(f'--base {base}: no such file')
        return parse_catalog(p.read_text(encoding='utf-8'), str(p))
    existing = Path(out) / 'catalog.json'
    if existing.is_file():
        return parse_catalog(existing.read_text(encoding='utf-8'), str(existing))
    return empty_catalog()


def describe(results):
    for r in results:
        rev = f'r{r.before} -> r{r.revision}' if r.status == 'updated' else f'r{r.revision}'
        print(f'  {r.status:<8} {r.pack.id}  {rev}  ({len(r.pack.files)} files, {fmt_bytes(r.pack.bytes)})')


def prepare(ids, extra, rep):
    packs = load_packs(select_folders(ids, extra), rep)
    rep.print()
    if rep.errors:
        n = rep.errors
        raise ToolError(f'{n} error{"" if n == 1 else "s"}: nothing was packed. Fix them (validate shows the same list).')
    if not packs:
        raise ToolError('no prototypes to pack')
    return packs


def cmd_pack(args):
    out = Path(args.out) if args.out else DEFAULT_OUT
    packs = prepare(args.ids, args.from_, Report())
    base = load_base(args.base, out)
    catalog, results = plan(packs, base, now_eastern())
    write_tree(out, results, catalog)
    describe(results)
    print(f'Packed {len(results)} prototype{"" if len(results) == 1 else "s"} into {display(out)} '
          f'({sum(1 for r in results if r.changed)} changed). Catalog: {display(out / "catalog.json")}')
    return 0


def publish_into(args):
    """publish --into DIR: revise the published tree in a folder (a checkout of the `packs` branch);
    committing and pushing are the caller's (packs.yml)."""
    into = Path(args.into)
    if not into.is_dir():
        raise ToolError(f'--into {args.into}: no such folder (check out the packs branch there, or make an empty folder)')
    if args.public_url and not url_ok(args.public_url):
        raise ToolError('--public-url must be an https:// URL')
    packs = prepare(args.ids, [], Report())
    catalog, results, restored = publish_dir(packs, into, now_eastern(), args.dry_run)
    describe(results)
    changed = [r for r in results if r.changed]
    base = (args.public_url or '').rstrip('/')
    where = f'{base}/catalog.json' if base else display(into / 'catalog.json')
    for r in restored:
        print(f'  {"would restore" if args.dry_run else "restored"} packs/{r.pack.id}/r{r.revision}/ (its files were missing or altered)')
    if not changed and not restored:
        print('Nothing to publish: every pack is already at its current revision.')
    elif args.dry_run:
        print(f'Dry run: nothing written. {len(changed)} pack{"" if len(changed) == 1 else "s"} would get a new revision.')
    else:
        print(f'Published into {display(into)}: {len(changed)} new revision{"" if len(changed) == 1 else "s"}. Catalog: {where}')
    parts = [f'{r.pack.id} r{r.revision}' for r in changed] + [f'{r.pack.id} r{r.revision} restored' for r in restored]
    if args.commit_message and parts and not args.dry_run:
        Path(args.commit_message).write_text('Publish packs: ' + ', '.join(parts) + '\n', encoding='utf-8')
    if args.summary:
        with open(args.summary, 'a', encoding='utf-8') as f:
            if not parts:
                f.write('### Packs\nNothing to publish: every pack is already at its current revision.\n')
            else:
                f.write(f'### Packs {"that would be published (dry run)" if args.dry_run else "published"}\n')
                for r in changed:
                    f.write(f'- `{r.pack.id}` revision {r.revision}: opens with `{OPEN_SCHEME}://open/{r.pack.id}`\n')
                for r in restored:
                    f.write(f'- `{r.pack.id}` revision {r.revision}: its files restored\n')
                f.write(f'\nCatalog: {where}\n')
    return 0


def cmd_publish(args, env=None):
    if args.into:
        return publish_into(args)
    if args.public_url or args.commit_message:
        raise ToolError('--public-url and --commit-message go with --into DIR')
    env = os.environ if env is None else env
    missing = [v for v in R2_VARS if not env.get(v)]
    have_keys = not [v for v in R2_VARS[:4] if not env.get(v)]
    if missing and not (args.dry_run and (have_keys or env.get('PACKS_PUBLIC_URL'))):
        raise ToolError('missing environment: ' + ', '.join(missing) + '. Publishing needs all five (docs/playground.md, '
                        '"Setting up R2")' + ('; a dry run needs the four R2_ variables, or PACKS_PUBLIC_URL to read the public catalog.'
                                              if args.dry_run else '.'))
    if env.get('PACKS_PUBLIC_URL') and not url_ok(env['PACKS_PUBLIC_URL']):
        raise ToolError('PACKS_PUBLIC_URL must be an https:// URL')
    packs = prepare(args.ids, [], Report())

    client = R2.from_env(env) if have_keys else None
    if client:
        raw = client.get('catalog.json')
        source = f'bucket {env["R2_BUCKET"]}'
    else:
        print(f'note: no R2 credentials in the environment ({", ".join(v for v in R2_VARS[:4] if not env.get(v))} missing); '
              'reading the public catalog instead.')
        raw = fetch_url(env['PACKS_PUBLIC_URL'].rstrip('/') + '/catalog.json')
        source = env['PACKS_PUBLIC_URL']
    base = empty_catalog() if raw is None else parse_catalog(raw.decode('utf-8', 'replace'), f'{source} catalog.json')
    print(f'Live catalog: {len(base["packs"])} pack{"" if len(base["packs"]) == 1 else "s"} ({source}'
          f'{"" if raw is not None else ", none yet"}).')

    catalog, results = plan(packs, base, now_eastern())
    describe(results)
    changed = [r for r in results if r.changed]
    objects = uploads(results, catalog)
    if not changed:
        print('Nothing to publish: every pack is already at its current revision.')
    elif args.dry_run:
        print(f'Dry run: would upload {len(objects)} objects ({fmt_bytes(sum(len(u[1]) for u in objects))}), catalog last:')
        for key, body, _, _ in objects:
            print(f'  {key}  ({fmt_bytes(len(body))})')
    else:
        for i, (key, body, ctype, cache) in enumerate(objects, 1):
            client.put(key, body, ctype, cache)
            print(f'  uploaded {i}/{len(objects)}  {key}')
    if changed:
        print('Open pages:' if not args.dry_run else 'Open pages (after a real publish):')
        for r in changed:
            print(f'  {r.pack.id} r{r.revision}: {public_url(env, f"open/{r.pack.id}.html")}')
        print(f'Catalog: {public_url(env, "catalog.json")}')
    if args.summary:
        with open(args.summary, 'a', encoding='utf-8') as f:
            if not changed:
                f.write('### Packs\nNothing to publish: every pack is already at its current revision.\n')
            else:
                f.write(f'### Packs {"that would be published (dry run)" if args.dry_run else "published"}\n')
                for r in changed:
                    f.write(f'- `{r.pack.id}` revision {r.revision}: {public_url(env, f"open/{r.pack.id}.html")}\n')
                f.write(f'\nCatalog: {public_url(env, "catalog.json")}\n')
    return 0


def cmd_retire(args, env=None):
    env = os.environ if env is None else env
    flag = not args.undo
    if args.out:
        path = Path(args.out) / 'catalog.json'
        if not path.is_file():
            raise ToolError(f'{display(path)}: no catalog there (pack first)')
        catalog, client = parse_catalog(path.read_text(encoding='utf-8'), str(path)), None
    else:
        client = R2.from_env(env)
        raw = client.get('catalog.json')
        if raw is None:
            raise ToolError(f'bucket {env["R2_BUCKET"]} has no catalog.json yet')
        catalog = parse_catalog(raw.decode('utf-8', 'replace'), 'the bucket\'s catalog.json')
    entry = next((e for e in catalog['packs'] if e['id'] == args.id), None)
    if entry is None:
        raise ToolError(f'"{args.id}" is not in the catalog (it has: {", ".join(e["id"] for e in catalog["packs"]) or "nothing"})')
    if bool(entry.get('retired', False)) == flag:
        print(f'{args.id} is already {"retired" if flag else "not retired"}; nothing changed.')
        return 0
    entry['retired'] = flag
    catalog['generated'] = now_eastern()
    body = catalog_bytes(catalog)
    if client:
        client.put('catalog.json', body, 'application/json', CATALOG_CACHE)
    else:
        atomic_write(path, body)
    print(f'{args.id} is now {"retired" if flag else "back in the catalog"} (its files are untouched).')
    return 0


def cmd_share(args):
    """Writes the shared script into every pack that lists it in its scripts (or the packs named)."""
    source = SHARED.read_bytes()
    folders = [PROTOTYPES / i for i in args.ids] if args.ids else prototype_dirs()
    for folder in folders:
        try:
            scripts = load_json(folder / 'prototype.json').get('scripts') or []
        except (OSError, ValueError):
            continue
        if SHARED_NAME not in scripts:
            continue
        target = folder / SHARED_NAME
        if not target.is_file() or target.read_bytes() != source:
            target.write_bytes(source)
            print(f'{display(target)}: written')
    return 0


def cmd_list(args):
    rows = []
    for f in prototype_dirs():
        try:
            m = load_json(f / 'prototype.json')
            rows.append((f.name, m.get('presentation', '?'), m.get('shelf', 'current'), m.get('title', '')))
        except (OSError, ValueError):
            rows.append((f.name, '?', '?', '(prototype.json cannot be read)'))
    print('Bundled (Prototypes/):')
    if not rows:
        print('  none')
    for r in rows:
        print(f'  {r[0]:<24} {r[1]:<7} {r[2]:<8} {r[3]}')
    if args.remote:
        url = args.remote
        if not re.match(r'https?://', url):
            raise ToolError('--remote takes a catalog URL (https://…/catalog.json)')
        if not urllib.parse.urlparse(url).path.endswith('.json'):
            url = url.rstrip('/') + '/catalog.json'
        raw = fetch_url(url)
        if raw is None:
            raise ToolError(f'{url}: no catalog there (404)')
        catalog = parse_catalog(raw.decode('utf-8', 'replace'), url)
        print(f'\nRemote ({url}, generated {catalog.get("generated") or "?"}):')
        if not catalog['packs']:
            print('  none')
        for e in catalog['packs']:
            print(f'  {e["id"]:<24} r{e["revision"]:<4} {"retired" if e.get("retired") else "":<8} {e.get("title", "")}')
    return 0


def build_parser():
    p = argparse.ArgumentParser(prog='playground.py', description=__doc__.split('\n\n')[0])
    sub = p.add_subparsers(dest='command', required=True)

    s = sub.add_parser('new', help='scaffold a runtime-3 pack in Prototypes/<id>/')
    s.add_argument('id')
    kind = s.add_mutually_exclusive_group()
    kind.add_argument('--globe', action='store_true', help='accepted, and the default: every pack lives on the globe')
    kind.add_argument('--screen', action='store_true', help='accepted for older instructions: makes a globe pack too')
    s.add_argument('--title')
    s.set_defaults(run=cmd_new)

    s = sub.add_parser('validate', help='check prototypes against the contract')
    s.add_argument('ids', nargs='*')
    s.set_defaults(run=cmd_validate)

    s = sub.add_parser('pack', help='write catalog.json, packs/ and open/ pages')
    s.add_argument('ids', nargs='*')
    s.add_argument('--from', dest='from_', nargs='+', metavar='DIR', help='pack folders from anywhere')
    s.add_argument('--out', help=f'default {display(DEFAULT_OUT)}')
    s.add_argument('--base', help='catalog to revise: a file or https URL (default <out>/catalog.json)')
    s.set_defaults(run=cmd_pack)

    s = sub.add_parser('publish', help='give changed packs their next revision: into a folder (--into, the '
                                       'packs branch) or the R2 bucket')
    s.add_argument('ids', nargs='*')
    s.add_argument('--into', metavar='DIR', help='a checkout of the packs branch (or an empty folder): '
                                                 'write the new revisions and catalog.json there; commit them yourself')
    s.add_argument('--public-url', metavar='URL', help='with --into: where DIR is served, for the summary '
                                                       '(e.g. https://raw.githubusercontent.com/<owner>/<repo>/packs)')
    s.add_argument('--commit-message', metavar='FILE', help='with --into: write "Publish packs: <id> r<n>, ..." '
                                                            'there when something changed')
    s.add_argument('--dry-run', action='store_true', help='print the plan; write or upload nothing')
    s.add_argument('--summary', metavar='FILE', help='append a markdown summary (e.g. $GITHUB_STEP_SUMMARY)')
    s.set_defaults(run=cmd_publish)

    s = sub.add_parser('retire', help='withdraw a published pack (catalog only)')
    s.add_argument('id')
    s.add_argument('--out', '--into', dest='out', metavar='DIR',
                   help='a published folder (a checkout of the packs branch, or pack\'s output) instead of the bucket')
    s.add_argument('--undo', action='store_true')
    s.set_defaults(run=cmd_retire)

    s = sub.add_parser('share', help=f'copy the packs\' shared script ({display(SHARED)}) into each pack that lists '
                                     f'"{SHARED_NAME}" in its scripts')
    s.add_argument('ids', nargs='*')
    s.set_defaults(run=cmd_share)

    s = sub.add_parser('list', help='bundled prototypes, and a remote catalog with --remote')
    s.add_argument('--remote', metavar='URL')
    s.set_defaults(run=cmd_list)
    return p


def main(argv=None):
    args = build_parser().parse_args(argv)
    try:
        return args.run(args)
    except ToolError as e:
        print(f'error: {e}', file=sys.stderr)
        return 2
    except BrokenPipeError:
        return 0


if __name__ == '__main__':
    sys.exit(main())

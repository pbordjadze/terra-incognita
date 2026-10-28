#!/usr/bin/env python3
"""The prototype playground's developer tool (docs/playground.md): scaffold, check, pack and publish
the web prototypes in Prototypes/. Standard library only.

    playground.py new <id> (--globe | --screen) [--title T]    scaffold Prototypes/<id>/ from template/
    playground.py validate [ids...]                            the contract, sizes and the offline boundary
    playground.py pack [ids...] [--from DIR ...] [--out DIR] [--base FILE_OR_URL]
                                                               catalog.json, packs/<id>/r<rev>/, open/<id>.html
    playground.py publish [ids...] [--dry-run] [--summary FILE]
                                                               pack against the live bucket, upload what changed
    playground.py retire <id> [--out DIR] [--undo]             mark a pack withdrawn (catalog only)
    playground.py list [--remote URL]                          the bundled prototypes, and a remote catalog's

publish and retire (without --out) read R2_ACCOUNT_ID, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY,
R2_BUCKET and PACKS_PUBLIC_URL from the environment (R2_ENDPOINT overrides the S3 endpoint, for a
local S3-compatible server). Exit status: 0 done, 1 findings or a failed check, 2 usage or environment.
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
RUNTIME = 2
OPEN_SCHEME = 'terraincognita'

ID_RE = re.compile(r'[a-z0-9](?:[a-z0-9-]{0,38}[a-z0-9])')
PATH_RE = re.compile(r'[A-Za-z0-9._-]+(?:/[A-Za-z0-9._-]+)*')
COLOUR_RE = re.compile(r'#[0-9A-Fa-f]{6}')

# decimal megabytes: if the app counts in binary ones, a pack that passes here passes there too
MAX_PACK_BYTES = 50_000_000
MAX_FILE_BYTES = 25_000_000
MAX_FILES = 500

TOP_KEYS = {'id', 'title', 'summary', 'runtime', 'presentation', 'entry', 'anchor', 'markers', 'overlay',
            'taps', 'shelf', 'background'}
MARKER_KEYS = {'id', 'lon', 'lat', 'image', 'width', 'height', 'label', 'minZoom', 'maxZoom', 'opens'}
ANCHOR_KEYS = {'lon', 'lat', 'zoom'}
GLOBE_ONLY = ('anchor', 'markers', 'overlay', 'taps')
IMAGE_EXTS = ('.png', '.jpg', '.jpeg')
TEXT_EXTS = ('.html', '.htm', '.js', '.mjs', '.css')

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


_REMOTE = r'(?:(?:https?|wss?):)?//'
_QUOTE = r'''["'`]'''
# a load of something remote, by the pack's own page: the host would block it, so say so before a phone does
REMOTE_LOADS = [
    (re.compile(r'''\b(?:src|href|poster|srcset|action)\s*=\s*["']?\s*''' + _REMOTE, re.I), 'remote src/href'),
    (re.compile(r'''url\(\s*["']?\s*''' + _REMOTE, re.I), 'remote url()'),
    (re.compile(r'''@import\s+["']\s*''' + _REMOTE, re.I), 'remote @import'),
    (re.compile(r'\b(?:fetch|importScripts)\s*\(\s*' + _QUOTE + r'\s*' + _REMOTE), 'remote fetch/importScripts'),
    (re.compile(r'\bimport\s*\(?\s*' + _QUOTE + r'\s*' + _REMOTE), 'remote import'),
    (re.compile(r'\bfrom\s*' + _QUOTE + r'\s*' + _REMOTE), 'remote import'),
    (re.compile(r'\bnew\s+(?:WebSocket|EventSource|Worker|SharedWorker)\s*\(\s*' + _QUOTE + r'\s*' + _REMOTE), 'remote socket/worker'),
    (re.compile(r'\.open\s*\(\s*' + _QUOTE + r'[A-Za-z]+' + _QUOTE + r'\s*,\s*' + _QUOTE + r'\s*' + _REMOTE), 'remote XMLHttpRequest'),
    (re.compile(r'\bsendBeacon\s*\(\s*' + _QUOTE + r'\s*' + _REMOTE), 'remote sendBeacon'),
]
ANY_URL = re.compile(r'''(?:https?|wss?)://[^\s"'`<>)\]]+''')
NAMESPACE = re.compile(r'https?://www\.w3\.org/(?:2000/svg|1999/xhtml|1999/xlink|1998/Math/MathML)')
TERRA_REF = [
    re.compile(r'''\b(?:src|href)\s*=\s*["']?[^"'\s>]*\bterra\.js\b''', re.I),
    re.compile(r'\b(?:import|importScripts|fetch)\b[^;\n]*\bterra\.js\b'),
]
ATTR_PATH = re.compile(r'''\b(?:src|href)\s*=\s*["']([^"']+)["']''', re.I)


def scan_text(rel, text, files, rep, where):
    """The offline boundary in one .html/.js/.css/.mjs file, and references to files that are not there."""
    spans = []
    for pattern, kind in REMOTE_LOADS:
        for m in pattern.finditer(text):
            spans.append((m.start(), m.end()))
            rep.error(f'{where}/{rel}:{_line_of(text, m.start())}', f'{kind}: a pack cannot load anything outside itself')
    for pattern in TERRA_REF:
        for m in pattern.finditer(text):
            rep.error(f'{where}/{rel}:{_line_of(text, m.start())}', 'references terra.js, which the host injects itself')
    for m in ANY_URL.finditer(text):
        if any(a <= m.start() < b for a, b in spans) or NAMESPACE.match(m.group(0)):
            continue
        rep.warn(f'{where}/{rel}:{_line_of(text, m.start())}', f'mentions {m.group(0)[:60]} (a link is fine to cite; it is never loaded)')
    if rel.lower().endswith(('.html', '.htm')):
        here = Path(rel).parent
        for m in ATTR_PATH.finditer(text):
            ref = m.group(1).split('#')[0].split('?')[0]
            if not ref or ref.startswith('//') or re.match(r'[A-Za-z][A-Za-z0-9+.-]*:', ref) or '{' in ref:
                continue
            if ref.startswith('/'):
                # the app serves a pack from terra-pack://<id>/, the preview from /pack/<id>/: only a relative path means the same in both
                rep.warn(f'{where}/{rel}:{_line_of(text, m.start())}', f'"{ref}" starts with /: write it relative so it resolves the same in the app and the preview')
                continue
            target = os.path.normpath((here / ref).as_posix()).replace('\\', '/')
            if target not in files:
                rep.warn(f'{where}/{rel}:{_line_of(text, m.start())}', f'refers to "{ref}", which is not in the pack')


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


def validate_manifest(m, name, files, rep, where):
    if not isinstance(m, dict):
        rep.error(where, 'prototype.json must be a JSON object')
        return
    for k in m:
        if k not in TOP_KEYS:
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
    if not is_int(m.get('runtime')) or not 1 <= m.get('runtime') <= RUNTIME:
        rep.error(where, f'"runtime" must be an integer from 1 to {RUNTIME} (the newest this repository\'s app hosts)')
    pres = m.get('presentation')
    if pres not in ('screen', 'globe'):
        rep.error(where, '"presentation" must be "screen" or "globe"')
    if 'shelf' in m and m['shelf'] not in ('current', 'parked'):
        rep.error(where, '"shelf" must be "current" or "parked"')
    if 'background' in m and not (isinstance(m['background'], str) and COLOUR_RE.fullmatch(m['background'])):
        rep.error(where, '"background" must be a colour like #RRGGBB')
    if 'taps' in m and not isinstance(m['taps'], bool):
        rep.error(where, '"taps" must be true or false')

    entry_ok = False
    if 'entry' in m:
        entry_ok = _check_file(rep, where, 'entry', m['entry'], files)
        if entry_ok and not m['entry'].lower().endswith(('.html', '.htm')):
            rep.warn(where, f'entry "{m["entry"]}" is not an .html page')
    else:
        entry_ok = 'index.html' in files
        if pres == 'screen' and not entry_ok:
            rep.error(where, 'a screen prototype needs its entry page: index.html (or "entry")')

    if pres == 'screen':
        for k in GLOBE_ONLY:
            if k in m:
                rep.warn(where, f'"{k}" is only used by globe prototypes')

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

    opens = False
    markers = m.get('markers')
    if 'markers' in m:
        if not isinstance(markers, list):
            rep.error(where, '"markers" must be a list')
        else:
            seen = set()
            for i, mk in enumerate(markers):
                tag = f'markers[{i}]'
                if not isinstance(mk, dict):
                    rep.error(where, f'{tag} must be an object')
                    continue
                for k in mk:
                    if k not in MARKER_KEYS:
                        rep.warn(where, f'unknown key "{tag}.{k}"')
                mid = mk.get('id')
                if not isinstance(mid, str) or not 1 <= len(mid) <= 40:
                    rep.error(where, f'{tag}.id is required: a string of 1-40 characters')
                elif mid in seen:
                    rep.error(where, f'{tag}.id "{mid}" is used twice')
                else:
                    seen.add(mid)
                    tag = f'markers[{mid}]'
                for k, lo, hi in (('lon', -180, 180), ('lat', -90, 90), ('width', 8, 200), ('height', 8, 200)):
                    if k not in mk:
                        rep.error(where, f'{tag}.{k} is required')
                    else:
                        _check_num(rep, where, f'{tag}.{k}', mk[k], lo, hi)
                if 'image' not in mk:
                    rep.error(where, f'{tag}.image is required')
                elif _check_file(rep, where, f'{tag}.image', mk['image'], files, IMAGE_EXTS):
                    _check_image_bytes(rep, where, f'{tag}.image', files[mk['image']])
                if not isinstance(mk.get('label'), str) or not mk['label'].strip():
                    rep.error(where, f'{tag}.label is required: the words VoiceOver speaks')
                lo_z = hi_z = None
                for k in ('minZoom', 'maxZoom'):
                    if k in mk and _check_num(rep, where, f'{tag}.{k}', mk[k], 0.8, 8):
                        if k == 'minZoom':
                            lo_z = mk[k]
                        else:
                            hi_z = mk[k]
                if lo_z is not None and hi_z is not None and lo_z > hi_z:
                    rep.error(where, f'{tag}.minZoom {lo_z} is above maxZoom {hi_z}')
                if 'opens' in mk and not isinstance(mk['opens'], bool):
                    rep.error(where, f'{tag}.opens must be true or false')
                opens = opens or mk.get('opens') is True
    if opens and not entry_ok:
        rep.error(where, 'a marker has "opens": true but the pack has no entry page to open')

    overlay = m.get('overlay')
    if 'overlay' in m:
        _check_file(rep, where, 'overlay', overlay, files)
    if m.get('taps') is True and 'overlay' not in m:
        rep.warn(where, '"taps": true does nothing without an overlay')
    if pres == 'globe' and not m.get('markers') and 'overlay' not in m:
        rep.warn(where, 'a globe prototype with no markers and no overlay shows nothing')


def validate_pack(folder, rep):
    """Everything docs/playground.md asks of one Prototypes/<id>/ folder. Returns (manifest, files),
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
    validate_manifest(manifest, folder.name, files, rep, f'{where}/prototype.json')
    if 'terra.js' in {Path(f).name for f in files}:
        rep.error(where, 'a pack must not contain its own terra.js: the host injects the runtime')
    for rel, full in files.items():
        if rel.lower().endswith(TEXT_EXTS):
            scan_text(rel, full.read_text(encoding='utf-8', errors='replace'), files, rep, where)
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


def plan(packs, base, now):
    """Decide each pack's revision against the base catalog. Returns (catalog, results).
    New id: revision 1. Same content hash: the base entry, untouched (so `published` stays).
    Different: the base revision + 1. Entries for packs not packed are kept as they are. A retired
    pack stays retired until `retire --undo`."""
    entries = {e['id']: e for e in base['packs']}
    results = []
    for p in packs:
        old = entries.get(p.id)
        if old is None:
            results.append(Result(p, 'new', 1, None))
            entries[p.id] = make_entry(p, 1, now)
        elif old.get('hash') == p.hash:
            results.append(Result(p, 'same', old['revision'], old['revision']))
        else:
            results.append(Result(p, 'updated', old['revision'] + 1, old['revision']))
            entries[p.id] = make_entry(p, old['revision'] + 1, now, bool(old.get('retired', False)))
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


def write_tree(out, results, catalog):
    """The packed output: packs/<id>/r<rev>/, open/<id>.html, and catalog.json last (atomically), so
    a catalog never names files that are not there yet."""
    out = Path(out)
    for r in results:
        dest = out / 'packs' / r.pack.id / f'r{r.revision}'
        if not _dir_matches(dest, r.pack):
            shutil.rmtree(dest, ignore_errors=True)
            for f in r.pack.files:
                (dest / f['path']).parent.mkdir(parents=True, exist_ok=True)
                (dest / f['path']).write_bytes(f['data'])
        atomic_write(out / 'open' / f'{r.pack.id}.html', open_page(r.pack.id, r.pack.manifest['title']).encode('utf-8'))
    atomic_write(out / 'catalog.json', catalog_bytes(catalog))


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


IMMUTABLE = 'public, max-age=31536000, immutable'
NO_CACHE = 'no-cache'
CATALOG_CACHE = 'no-cache, max-age=0'


def public_url(env, path):
    base = (env.get('PACKS_PUBLIC_URL') or '').rstrip('/')
    return f'{base or "<PACKS_PUBLIC_URL>"}/{path}'


# ---------------------------------------------------------------- commands

def default_title(pid):
    t = pid.replace('-', ' ')
    return t[:1].upper() + t[1:]


def _substitute(name, data, pid, title):
    ext = Path(name).suffix.lower()
    if ext not in ('.json', '.html', '.js', '.css', '.md'):
        return data
    text = data.decode('utf-8')
    if ext == '.json':
        t = json.dumps(title)[1:-1]
    elif ext == '.html':
        t = html.escape(title)
    else:
        t = title
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
    kind = 'globe' if args.globe else 'screen'
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
    print(f'Created {display(dest)}/ ({kind}).')
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


def cmd_publish(args, env=None):
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

    with tempfile.TemporaryDirectory(prefix='playground-pack-') as tmp:
        catalog, results = plan(packs, base, now_eastern())
        write_tree(tmp, results, catalog)
        describe(results)
        changed = [r for r in results if r.changed]
        uploads = []  # (key, body, content type, cache control); the catalog goes last
        for r in changed:
            for f in r.pack.files:
                uploads.append((f'packs/{r.pack.id}/r{r.revision}/{f["path"]}', f['data'], content_type(f['path']), IMMUTABLE))
            page = (Path(tmp) / 'open' / f'{r.pack.id}.html').read_bytes()
            uploads.append((f'open/{r.pack.id}.html', page, content_type('x.html'), NO_CACHE))
        if changed:
            uploads.append(('catalog.json', (Path(tmp) / 'catalog.json').read_bytes(), 'application/json', CATALOG_CACHE))

        if not changed:
            print('Nothing to publish: every pack is already at its current revision.')
        elif args.dry_run:
            print(f'Dry run: would upload {len(uploads)} objects ({fmt_bytes(sum(len(u[1]) for u in uploads))}), catalog last:')
            for key, body, _, _ in uploads:
                print(f'  {key}  ({fmt_bytes(len(body))})')
        else:
            for i, (key, body, ctype, cache) in enumerate(uploads, 1):
                client.put(key, body, ctype, cache)
                print(f'  uploaded {i}/{len(uploads)}  {key}')
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

    s = sub.add_parser('new', help='scaffold Prototypes/<id>/')
    s.add_argument('id')
    kind = s.add_mutually_exclusive_group(required=True)
    kind.add_argument('--globe', action='store_true', help='markers and an overlay on the globe')
    kind.add_argument('--screen', action='store_true', help='its own full-screen page')
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

    s = sub.add_parser('publish', help='pack against the live bucket and upload what changed')
    s.add_argument('ids', nargs='*')
    s.add_argument('--dry-run', action='store_true', help='print the plan; upload nothing')
    s.add_argument('--summary', metavar='FILE', help='append a markdown summary (e.g. $GITHUB_STEP_SUMMARY)')
    s.set_defaults(run=cmd_publish)

    s = sub.add_parser('retire', help='withdraw a published pack (catalog only)')
    s.add_argument('id')
    s.add_argument('--out', help='a packed folder instead of the bucket')
    s.add_argument('--undo', action='store_true')
    s.set_defaults(run=cmd_retire)

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

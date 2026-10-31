#!/usr/bin/env python3
"""Wikipedia's words: the articles behind the atlas's paragraphs, fetched once and bundled.

No current users: the switch and the experiences it served were retired on October 31, 2026
(painting-company row 195; the app's side is at before-retirement-2026-10-31). `--app` would write a
Wikipedia.json the app no longer reads, into a folder the app bundles: don't run it until the switch
comes back.

The app has a switch, Wikipedia's words (App/TerraIncognita/Wikipedia/): on, every paragraph the atlas
wrote itself gives way to Wikipedia's own first sentences on the same subject and a link to the
article. The app fetches nothing (its rule: no network), so the articles are bundled, and this tool
is what fetches them, from the English Wikipedia's Action API, in batches, with the attribution
Wikipedia's licence (CC BY-SA 4.0) asks for kept beside every extract.

    python3 tools/wikipedia/build.py              # the app's index and every pack's, from their keys
    python3 tools/wikipedia/build.py --app        # App/TerraIncognita/Wikipedia/Wikipedia.json only
    python3 tools/wikipedia/build.py --pack sea-between [--pack ...]
    python3 tools/wikipedia/build.py --keys       # list the keys each index would hold, and fetch nothing

A run keeps what it fetched in build/wikipedia-cache.json for the day, so one cut short by the API's
rate limit (429) picks up where it left when run again; `--refresh` fetches everything anew.

What is fetched, and for whom:

- The app's index, Wikipedia.json: every key the app's Swift names (`wikipedia: "Title"` arguments
  and `WikipediaIndex.shared["Title"]`), every country of Globe/globe.json (its long name, for the
  caption of a chosen country), and the keys listed under [app] in articles.txt.
- A pack's wikipedia.json: every key its scripts and data name (`wikipedia: 'Title'` in a script,
  `"wikipedia": "Title"` in JSON, every value of an object named `WIKIPEDIA`) and the keys under
  [pack:<id>] in articles.txt. A pack carries its
  own words, as it carries everything else, so a downloaded revision can name articles the app never
  heard of.

articles.txt lists keys by section; a line `Key => Title` fetches the article `Title` for the key
`Key` (an old map's "Byelarus" is Wikipedia's "Belarus"). Keys are what the code says; titles are
Wikipedia's, and redirects are followed, so "Czechia" lands on the article wherever Wikipedia keeps it.

Each article is stored as `{ title, description, extract, url, revision, timestamp }`: the title as
Wikipedia shows it, the short description (Wikidata's, as the app shows under a title), the lead
section as plain text (pronunciation guides in parentheses removed, the rest as written, cut to at
most 900 characters at a sentence's end), the article's URL, and the revision the words come from.
The app chooses how much of the extract to show (a sentence at the foot, two on a card).

A key with no article is reported and left out: the app then drops that paragraph when the switch is
on, and ci/check_wikipedia.py fails the build until it is given an article or spelled right. Run this
tool again whenever a key is added (the check says which are missing), or to refresh the words.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
APP = ROOT / 'App' / 'TerraIncognita'
APP_INDEX = APP / 'Wikipedia' / 'Wikipedia.json'
PACKS = ROOT / 'Prototypes'
ARTICLES = Path(__file__).resolve().parent / 'articles.txt'
GLOBE = ROOT / 'Globe' / 'globe.json'

API = 'https://en.wikipedia.org/w/api.php'
USER_AGENT = 'TerraIncognita-build/0.1 (https://github.com/pbordjadze/terra-incognita; builds a bundled index, no live use)'
BATCH = 20            # the Action API's limit for extracts with exintro
PAUSE = 3.0           # seconds between batches: a courteous pace (the API answers 429 to a hurried one)
EXTRACT_LIMIT = 900   # characters, cut at a sentence's end
# what was fetched today, so a run cut short by a 429 picks up where it left (build/ is not committed)
CACHE = ROOT / 'build' / 'wikipedia-cache.json'

# the keys the code names (Swift: a `wikipedia:` argument or the index's subscript; packs: a `wikipedia:`
# field in a script or JSON)
SWIFT_KEY = re.compile(r'wikipedia:\s*"((?:[^"\\]|\\.)+)"|WikipediaIndex\.shared\["((?:[^"\\]|\\.)+)"\]')
PACK_KEY = re.compile(r'''\bwikipedia\s*:\s*(?:'((?:[^'\\]|\\.)+)'|"((?:[^"\\]|\\.)+)")|"wikipedia"\s*:\s*"((?:[^"\\]|\\.)+)"''')
# a pack that keeps its subjects in one object (`var WIKIPEDIA = { leaf: 'Aegean Sea', ... }`): every string value in it
PACK_OBJECT = re.compile(r'\bWIKIPEDIA\s*=\s*\{([^}]*)\}')
PACK_OBJECT_VALUE = re.compile(r''':\s*(?:'((?:[^'\\]|\\.)+)'|"((?:[^"\\]|\\.)+)")''')


def unescape(s: str) -> str:
    return s.replace("\\'", "'").replace('\\"', '"').replace('\\\\', '\\')


def read_articles_txt() -> dict[str, dict[str, str]]:
    """{ 'app' | 'pack:<id>': { key: title } } from articles.txt."""
    sections: dict[str, dict[str, str]] = {}
    section = 'app'
    if not ARTICLES.exists():
        return sections
    for raw in ARTICLES.read_text(encoding='utf-8').splitlines():
        line = raw.split('#', 1)[0].strip()
        if not line:
            continue
        if line.startswith('[') and line.endswith(']'):
            section = line[1:-1].strip()
            sections.setdefault(section, {})
            continue
        key, _, title = (p.strip() for p in line.partition('=>'))
        sections.setdefault(section, {})[key] = title or key
    return sections


def scan(paths, pattern: re.Pattern) -> set[str]:
    keys: set[str] = set()
    for p in paths:
        try:
            text = p.read_text(encoding='utf-8')
        except (OSError, UnicodeDecodeError):
            continue
        for m in pattern.finditer(text):
            key = next(g for g in m.groups() if g is not None)
            keys.add(unescape(key))
    return keys


def app_keys() -> dict[str, str]:
    """The app's keys → the article title to fetch for each."""
    listed = read_articles_txt().get('app', {})
    keys = {k: listed.get(k, k) for k in scan(APP.rglob('*.swift'), SWIFT_KEY)}
    keys.update(listed)
    globe = json.loads(GLOBE.read_text(encoding='utf-8'))
    for c in globe.get('countries', []):
        name = c.get('long') or c.get('n')
        if name and name not in keys:
            keys[name] = listed.get(name, name)
    return keys


def pack_dirs() -> list[Path]:
    return sorted(p for p in PACKS.iterdir() if p.is_dir() and not p.name.startswith('_') and (p / 'prototype.json').exists())


def pack_keys(pack: Path) -> dict[str, str]:
    """A pack's keys → titles: what its scripts and data name, and articles.txt's [pack:<id>]."""
    listed = read_articles_txt().get('pack:' + pack.name, {})
    files = [p for p in pack.iterdir() if p.suffix in ('.js', '.mjs', '.json') and p.name != 'wikipedia.json']
    found = scan(files, PACK_KEY)
    for f in files:
        try:
            for m in PACK_OBJECT.finditer(f.read_text(encoding='utf-8')):
                found |= {unescape(next(g for g in v.groups() if g is not None)) for v in PACK_OBJECT_VALUE.finditer(m.group(1))}
        except (OSError, UnicodeDecodeError):
            continue
    keys = {k: listed.get(k, k) for k in found}
    keys.update(listed)
    return keys


# ---- fetching -----------------------------------------------------------------------------------

def get(params: dict, tries: int = 8) -> dict:
    query = urllib.parse.urlencode({**params, 'format': 'json', 'formatversion': '2', 'maxlag': '5'})
    req = urllib.request.Request(API + '?' + query, headers={'User-Agent': USER_AGENT, 'Accept': 'application/json'})
    wait = 15.0
    for attempt in range(tries):
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                data = json.loads(r.read().decode('utf-8'))
            if 'error' in data and data['error'].get('code') == 'maxlag':
                raise urllib.error.HTTPError(API, 503, 'maxlag', {}, None)
            return data
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 502, 503, 504) and attempt + 1 < tries:
                asked = e.headers.get('Retry-After') if e.headers else None
                pause = max(wait, float(asked)) if asked and asked.isdigit() else wait
                print(f'  {e.code} from Wikipedia; waiting {pause:.0f} s', file=sys.stderr)
                time.sleep(pause)
                wait = min(wait * 2, 300)
                continue
            raise
        except urllib.error.URLError:
            if attempt + 1 < tries:
                time.sleep(wait)
                wait *= 2
                continue
            raise
    raise RuntimeError('unreachable')


# A parenthesis in an article's first words that is a pronunciation, a transliteration or an etymology
# (Wikipedia's plain text drops the IPA itself and leaves "( or ; Ancient Greek: …)"): not words to read
# aloud. One that says something ("(DPRK)", "(crown land)") stays.
ASIDE = re.compile(r"""^\s*(?:or\b|;|,|/|:|$)|\bpronounced\b|\bpronunciation\b|\blisten\b|\bromani[sz]ed\b|\blit\.|\btransl(?:it)?\.|
    /[^/\s][^/]*/|\[[^\]]*[ˈˌː][^\]]*\]|\b[A-Z]{2,}-[a-zA-Z]|\b(?:[A-Z][a-z]+\s){0,3}[A-Z][a-z]+:\s""", re.X)
PAREN = re.compile(r'\s*\(([^()]*)\)')
SENTENCE_END = re.compile(r'(?<=[.!?…])["”’)]?\s')


def strip_asides(text: str, within: int = 320) -> str:
    """Removes the asides (above) among the parentheses in the first `within` characters, innermost first;
    a parenthesis that says something is kept (its brackets hidden from the next pass, then put back)."""
    t = text
    while True:
        m = next((m for m in PAREN.finditer(t) if m.start() < within), None)
        if not m:
            break
        inner = m.group(1)
        if ASIDE.search(inner) or not inner.strip():
            t = t[:m.start()] + t[m.end():]
        else:
            t = t[:m.start()] + ' \x00' + inner + '\x01' + t[m.end():]
    t = t.replace('\x00', '(').replace('\x01', ')')
    return re.sub(r'\s+([,;.])', r'\1', t)


def clean_extract(text: str) -> str:
    t = strip_asides(text or '')
    t = re.sub(r'\s+', ' ', t).strip()
    if len(t) <= EXTRACT_LIMIT:
        return t
    cut = t[:EXTRACT_LIMIT + 1]
    ends = [m.end() for m in SENTENCE_END.finditer(cut)]
    return cut[:ends[-1]].strip() if ends else cut[:EXTRACT_LIMIT].rstrip() + '…'


def read_cache() -> dict:
    try:
        cache = json.loads(CACHE.read_text(encoding='utf-8'))
        return cache if cache.get('day') == dt.date.today().isoformat() else {}
    except (OSError, ValueError):
        return {}


def write_cache(cache: dict) -> None:
    CACHE.parent.mkdir(parents=True, exist_ok=True)
    cache['day'] = dt.date.today().isoformat()
    CACHE.write_text(json.dumps(cache, ensure_ascii=False), encoding='utf-8')


def fetch(titles: list[str], refresh: bool = False) -> tuple[dict[str, dict], list[str]]:
    """{ requested title: article } and the titles Wikipedia has no page for."""
    cache = {} if refresh else read_cache()
    # (an older cache of the day held cleaned extracts under "found"; they clean again well enough)
    pages = {**{k: {**v, 'raw': v.get('raw', v.get('extract', ''))} for k, v in cache.get('found', {}).items()}, **cache.get('pages', {})}
    found: dict[str, dict] = {t: pages[t] for t in titles if t in pages}
    missing: list[str] = [t for t in titles if t in cache.get('missing', [])]
    titles = [t for t in titles if t not in found and t not in missing]
    if found or missing:
        print(f'  {len(found) + len(missing)} from today\'s cache ({CACHE.relative_to(ROOT)})', file=sys.stderr)
    for i in range(0, len(titles), BATCH):
        batch = titles[i:i + BATCH]
        data = get({'action': 'query', 'redirects': '1', 'titles': '|'.join(batch),
                    'prop': 'extracts|description|info|revisions', 'exintro': '1', 'explaintext': '1', 'exlimit': 'max',
                    'inprop': 'url', 'rvprop': 'ids|timestamp'})
        q = data.get('query', {})
        # what each requested title became: normalized (capitalisation, underscores), then redirected
        to = {}
        for n in q.get('normalized', []):
            to[n['from']] = n['to']
        redirected = {r['from']: r['to'] for r in q.get('redirects', [])}
        pages = {p['title']: p for p in q.get('pages', [])}
        for requested in batch:
            title = to.get(requested, requested)
            seen = 0
            while title in redirected and seen < 5:
                title = redirected[title]
                seen += 1
            page = pages.get(title)
            if not page or page.get('missing') or page.get('invalid'):
                missing.append(requested)
                continue
            raw = re.sub(r'\s+', ' ', page.get('extract', '') or '').strip()
            if not raw:
                missing.append(requested)
                continue
            rev = (page.get('revisions') or [{}])[0]
            found[requested] = {
                'title': page['title'],
                'description': page.get('description') or '',
                'raw': raw,
                'url': page.get('fullurl') or 'https://en.wikipedia.org/wiki/' + urllib.parse.quote(page['title'].replace(' ', '_')),
                'revision': rev.get('revid'),
                'timestamp': rev.get('timestamp'),
            }
        print(f'  {min(i + BATCH, len(titles))}/{len(titles)} fetched', file=sys.stderr)
        write_cache({'pages': {**pages, **found}, 'missing': sorted(set(cache.get('missing', [])) | set(missing))})
        if i + BATCH < len(titles):
            time.sleep(PAUSE)
    return found, missing


def build(keys: dict[str, str], out: Path, what: str, refresh: bool = False) -> list[str]:
    titles = sorted(set(keys.values()))
    print(f'{what}: {len(keys)} keys, {len(titles)} articles', file=sys.stderr)
    found, missing = fetch(titles, refresh)
    articles = {}
    for key, title in sorted(keys.items()):
        if title in found:
            page = found[title]
            articles[key] = {k: page[k] for k in ('title', 'description')} | {'extract': clean_extract(page['raw'])} | {k: page.get(k) for k in ('url', 'revision', 'timestamp')}
    lost = sorted(key for key, title in keys.items() if title not in found)
    index = {
        'source': 'English Wikipedia (en.wikipedia.org), the lead section of each article as plain text',
        'license': 'CC BY-SA 4.0 (https://creativecommons.org/licenses/by-sa/4.0/); each article links to its page and history',
        'fetched': dt.date.today().isoformat(),
        'tool': 'tools/wikipedia/build.py',
        'articles': articles,
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(index, ensure_ascii=False, indent=1) + '\n', encoding='utf-8')
    print(f'  wrote {out.relative_to(ROOT)} ({len(articles)} articles, {out.stat().st_size // 1024} KB)', file=sys.stderr)
    for key in lost:
        print(f'  no article: {key!r} (asked for {keys[key]!r})', file=sys.stderr)
    return lost


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--app', action='store_true', help="the app's index only")
    ap.add_argument('--pack', action='append', default=[], help="this pack's wikipedia.json (repeatable)")
    ap.add_argument('--keys', action='store_true', help='list the keys and fetch nothing')
    ap.add_argument('--refresh', action='store_true', help="fetch everything again, ignoring today's cache")
    args = ap.parse_args(argv)

    jobs: list[tuple[str, dict[str, str], Path]] = []
    if args.app or not args.pack:
        jobs.append(('the app', app_keys(), APP_INDEX))
    packs = [PACKS / p for p in args.pack] if args.pack else ([] if args.app else pack_dirs())
    for pack in packs:
        if not (pack / 'prototype.json').exists():
            print(f'no pack at {pack}', file=sys.stderr)
            return 2
        keys = pack_keys(pack)
        if keys or (pack / 'wikipedia.json').exists():
            jobs.append((pack.name, keys, pack / 'wikipedia.json'))

    if args.keys:
        for what, keys, out in jobs:
            print(f'{what} → {out.relative_to(ROOT)}')
            for k, t in sorted(keys.items()):
                print(f'  {k}' + (f' => {t}' if t != k else ''))
        return 0

    lost_any = False
    for what, keys, out in jobs:
        if not keys:
            if out.exists():
                out.unlink()
                print(f'{what}: no keys; removed {out.relative_to(ROOT)}', file=sys.stderr)
            continue
        lost_any |= bool(build(keys, out, what, args.refresh))
    return 1 if lost_any else 0


if __name__ == '__main__':
    sys.exit(main())

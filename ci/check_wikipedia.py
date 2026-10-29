#!/usr/bin/env python3
"""Wikipedia's words are all there: every article the app and the packs name is bundled, and sound.

The switch (App/TerraIncognita/Wikipedia/) puts Wikipedia's words where the atlas's paragraphs are, from
bundled indexes built by tools/wikipedia/build.py. This check reads the same keys the tool does (the
Swift's `wikipedia: "Title"` arguments and `WikipediaIndex.shared["Title"]`, every country of
Globe/globe.json, a pack's `wikipedia:` fields, articles.txt) and fails when one has no article in the
index it would be looked up in: a paragraph would then vanish under the switch with nothing in its
place. It also fails an article the app couldn't show (no title, no extract, a URL off Wikipedia, HTML
in the text). A stale index (fetched more than half a year ago) is a warning, not a failure.

    python3 ci/check_wikipedia.py            # the app's index and every pack's
    python3 ci/check_wikipedia.py --packs    # the packs' only (packs.yml, where the app isn't built)
"""
from __future__ import annotations

import datetime as dt
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools' / 'wikipedia'))
import build  # noqa: E402  (tools/wikipedia/build.py: the keys and the files)

STALE_DAYS = 183


def load(path: Path, rep: list[str]) -> dict | None:
    try:
        index = json.loads(path.read_text(encoding='utf-8'))
    except OSError:
        rep.append(f'{path.relative_to(ROOT)}: missing (python3 tools/wikipedia/build.py)')
        return None
    except ValueError as e:
        rep.append(f'{path.relative_to(ROOT)}: not JSON ({e})')
        return None
    if not isinstance(index.get('articles'), dict):
        rep.append(f'{path.relative_to(ROOT)}: no "articles" object')
        return None
    return index


def check_articles(path: Path, index: dict, rep: list[str]) -> None:
    where = path.relative_to(ROOT)
    for key, a in index['articles'].items():
        if not isinstance(a, dict):
            rep.append(f'{where}: "{key}" is not an article object')
            continue
        if not str(a.get('title', '')).strip():
            rep.append(f'{where}: "{key}" has no title')
        extract = str(a.get('extract', ''))
        if not extract.strip():
            rep.append(f'{where}: "{key}" has no extract')
        if len(extract) > 1200:
            rep.append(f'{where}: "{key}" extract is {len(extract)} characters (the tool cuts at 900)')
        if re.search(r'<[a-zA-Z/][^>]*>', extract):
            rep.append(f'{where}: "{key}" extract carries HTML')
        url = str(a.get('url', ''))
        if not re.match(r'^https://en\.wikipedia\.org/wiki/.+', url):
            rep.append(f'{where}: "{key}" url is not an en.wikipedia.org article ({url!r})')
    fetched = index.get('fetched')
    try:
        age = (dt.date.today() - dt.date.fromisoformat(fetched)).days
        if age > STALE_DAYS:
            print(f'warning: {where} was fetched {age} days ago (python3 tools/wikipedia/build.py refreshes it)')
    except (TypeError, ValueError):
        rep.append(f'{where}: no "fetched" date')


def check_keys(path: Path, index: dict, keys: dict[str, str], rep: list[str], fallback: dict | None = None) -> None:
    have = set(index['articles'])
    if fallback:
        have |= set(fallback['articles'])
    for key in sorted(keys):
        if key not in have:
            rep.append(f'{path.relative_to(ROOT)}: no article for "{key}" (asked for "{keys[key]}"; python3 tools/wikipedia/build.py)')


def main(argv=None) -> int:
    packs_only = '--packs' in (argv or sys.argv[1:])
    rep: list[str] = []
    app_index = None
    if not packs_only:
        app_index = load(build.APP_INDEX, rep)
        if app_index:
            check_articles(build.APP_INDEX, app_index, rep)
            check_keys(build.APP_INDEX, app_index, build.app_keys(), rep)
            print(f'{build.APP_INDEX.relative_to(ROOT)}: {len(app_index["articles"])} articles, fetched {app_index.get("fetched")}')
    for pack in build.pack_dirs():
        keys = build.pack_keys(pack)
        path = pack / 'wikipedia.json'
        if not keys and not path.exists():
            continue
        index = load(path, rep)
        if not index:
            continue
        check_articles(path, index, rep)
        check_keys(path, index, keys, rep, fallback=None if packs_only else app_index)
        print(f'{path.relative_to(ROOT)}: {len(index["articles"])} articles, fetched {index.get("fetched")}')
    for line in rep:
        print('error:', line)
    print('wikipedia: ok' if not rep else f'wikipedia: {len(rep)} problem(s)')
    return 1 if rep else 0


if __name__ == '__main__':
    sys.exit(main())

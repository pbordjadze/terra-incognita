#!/usr/bin/env python3
"""Tests for tools/playground/playground.py (standard library only):

    python3 -m unittest tools/playground/test_playground.py      (or run this file)
"""
import contextlib
import hashlib
import http.server
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
import urllib.parse
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import playground as pg  # noqa: E402

ROOT = pg.ROOT
PNG = b'\x89PNG\r\n\x1a\n' + b'\x00' * 24


def run(*argv, env=None):
    """pg.main(argv): (exit status, stdout, stderr)."""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        if env is None:
            code = pg.main(list(argv))
        else:
            with mock.patch.dict(os.environ, env, clear=True):
                code = pg.main(list(argv))
    return code, out.getvalue(), err.getvalue()


def good_manifest(pid='demo-pack'):
    return {
        'id': pid, 'title': 'Demo', 'summary': 'A demonstration.', 'runtime': 1, 'presentation': 'globe',
        'entry': 'index.html', 'anchor': {'lon': -30, 'lat': 30, 'zoom': 1.6},
        'markers': [{'id': 'm', 'lon': -30, 'lat': 30, 'image': 'm.png', 'width': 44, 'height': 44,
                     'label': 'Demo marker', 'minZoom': 1, 'maxZoom': 6, 'opens': True}],
        'overlay': 'overlay.html', 'taps': True, 'shelf': 'current', 'background': '#F6F2E7',
    }


def write_pack(root, pid='demo-pack', manifest=None, extra=None):
    folder = Path(root) / pid
    folder.mkdir(parents=True, exist_ok=True)
    (folder / 'prototype.json').write_text(json.dumps(manifest or good_manifest(pid)), encoding='utf-8')
    (folder / 'index.html').write_text('<!doctype html><html lang="en"><body><h1>Demo</h1></body></html>', encoding='utf-8')
    (folder / 'overlay.html').write_text('<!doctype html><html lang="en"><body></body></html>', encoding='utf-8')
    (folder / 'm.png').write_bytes(PNG)
    for name, data in (extra or {}).items():
        (folder / name).parent.mkdir(parents=True, exist_ok=True)
        (folder / name).write_bytes(data if isinstance(data, bytes) else data.encode('utf-8'))
    return folder


def findings(folder):
    rep = pg.Report()
    pg.validate_pack(folder, rep)
    return rep


class TempCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix='playground-test-'))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)


class SigV4(unittest.TestCase):
    """AWS's published S3 examples (docs: "Signature Calculations for the Authorization Header:
    Transferring Payload in a Single Chunk"). Region us-east-1, 24 May 2013."""
    ACCESS = 'AKIAIOSFODNN7EXAMPLE'
    SECRET = 'wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY'
    EMPTY = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'

    def test_get_object_with_a_range(self):
        headers = {'Host': 'examplebucket.s3.amazonaws.com', 'Range': 'bytes=0-9',
                   'x-amz-content-sha256': self.EMPTY, 'x-amz-date': '20130524T000000Z'}
        auth, canonical, to_sign = pg.sigv4('GET', '/test.txt', {}, headers, self.EMPTY, self.ACCESS, self.SECRET,
                                            'us-east-1', 's3', '20130524T000000Z')
        self.assertEqual(canonical, '\n'.join([
            'GET', '/test.txt', '', 'host:examplebucket.s3.amazonaws.com', 'range:bytes=0-9',
            'x-amz-content-sha256:' + self.EMPTY, 'x-amz-date:20130524T000000Z', '',
            'host;range;x-amz-content-sha256;x-amz-date', self.EMPTY]))
        self.assertEqual(to_sign.split('\n')[3], '7344ae5b7ee6c3e7e6b0fe0640412a37625d1fbfff95c48bbb2dc43964946972')
        self.assertEqual(auth, 'AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request,'
                               'SignedHeaders=host;range;x-amz-content-sha256;x-amz-date,'
                               'Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41')

    def test_put_object_encodes_the_key_once(self):
        body_hash = '44ce7dd67c959e0d3524ffac1771dfbba87d2b6b4b4e99e42034a8b803f8b072'
        headers = {'Date': 'Fri, 24 May 2013 00:00:00 GMT', 'Host': 'examplebucket.s3.amazonaws.com',
                   'x-amz-date': '20130524T000000Z', 'x-amz-storage-class': 'REDUCED_REDUNDANCY',
                   'x-amz-content-sha256': body_hash}
        auth, canonical, to_sign = pg.sigv4('PUT', '/test$file.text', {}, headers, body_hash, self.ACCESS, self.SECRET,
                                            'us-east-1', 's3', '20130524T000000Z')
        self.assertIn('/test%24file.text', canonical)
        self.assertEqual(to_sign.split('\n')[3], '9e0e90d9c76de8fa5b200d8c849cd5b8dc7a3be3951ddb7f6a76b4158342019d')
        self.assertTrue(auth.endswith('Signature=98ad721746da40c64f1a55b78f14c238d841ea1380cd77a1b5971af0ece108bd'))

    def test_key_segments_are_encoded_but_slashes_kept(self):
        self.assertEqual(pg.canonical_path('/b/packs/a b/r1/ü.png'), '/b/packs/a%20b/r1/%C3%BC.png')


class Hashing(TempCase):
    def test_hash_is_over_sorted_path_and_digest_lines(self):
        a, b, c = (hashlib.sha256(x).hexdigest() for x in (b'1', b'2', b'3'))
        # byte order: "B.txt" < "a.txt" < "b/c.txt" (not a locale's order)
        expected = hashlib.sha256(f'B.txt {b}\na.txt {a}\nb/c.txt {c}\n'.encode()).hexdigest()
        self.assertEqual(pg.content_hash([('b/c.txt', c), ('a.txt', a), ('B.txt', b)]), expected)

    def test_pack_hash_matches_the_definition(self):
        folder = write_pack(self.tmp, extra={'sub/data.json': '{"a":1}', '.DS_Store': 'junk'})
        rep = pg.Report()
        manifest, files = pg.validate_pack(folder, rep)
        pack = pg.Pack(folder, manifest, files)
        lines = ''.join(f'{p.relative_to(folder).as_posix()} {hashlib.sha256(p.read_bytes()).hexdigest()}\n'
                        for p in sorted(folder.rglob('*'), key=lambda p: p.relative_to(folder).as_posix().encode())
                        if p.is_file() and p.name != '.DS_Store')
        self.assertEqual(pack.hash, hashlib.sha256(lines.encode()).hexdigest())
        self.assertNotIn('.DS_Store', [f['path'] for f in pack.files])


class PathSafety(unittest.TestCase):
    def test_ids(self):
        for ok in ('ab', 'tide-pool', 'a1', 'x' * 40, '0a'):
            self.assertTrue(pg.ID_RE.fullmatch(ok), ok)
        for bad in ('a', 'x' * 41, '-ab', 'ab-', 'Ab', 'a_b', 'a b', 'ab\n', '', 'é1'):
            self.assertIsNone(pg.ID_RE.fullmatch(bad), repr(bad))

    def test_paths(self):
        for ok in ('index.html', 'img/a-b_c.png', 'a/b/c.json', 'v1.2/x.js'):
            self.assertTrue(pg.safe_path(ok), ok)
        for bad in ('', '/abs.png', '../up.png', 'a/../b.png', './a.png', '.hidden', 'a/.hidden/b.png', 'a//b.png',
                    'a b.png', 'a\\b.png', 'a/', 'é.png', 'a\nb', '..', 'a/..'):
            self.assertFalse(pg.safe_path(bad), repr(bad))


class Validate(TempCase):
    def errors(self, rep):
        return [m for level, _, m in rep.items if level == 'ERROR']

    def warnings(self, rep):
        return [m for level, _, m in rep.items if level == 'WARN']

    def mutate(self, change, pid='demo-pack', extra=None):
        manifest = good_manifest(pid)
        change(manifest)
        return findings(write_pack(self.tmp, pid, manifest, extra))

    def assertError(self, rep, needle):
        self.assertTrue(any(needle in e for e in self.errors(rep)), f'{needle!r} not in {self.errors(rep)}')

    def test_a_good_pack_is_clean(self):
        rep = findings(write_pack(self.tmp))
        self.assertEqual(rep.items, [])

    def test_every_class_of_manifest_error(self):
        def drop(key):
            return lambda m: m.pop(key)

        def put(key, value):
            return lambda m: m.__setitem__(key, value)

        def marker(key, value):
            return lambda m: m['markers'][0].__setitem__(key, value)

        def anchor(key, value):
            return lambda m: m['anchor'].__setitem__(key, value)

        cases = [
            (drop('id'), '"id" is required'), (put('id', 'other-id'), 'must equal the folder name'),
            (put('id', 7), '"id" is required'),
            (drop('title'), '"title"'), (put('title', ''), '"title"'), (put('title', 'x' * 61), '"title"'),
            (put('title', '   '), '"title"'), (put('title', 3), '"title"'),
            (drop('summary'), '"summary"'), (put('summary', 'x' * 201), '"summary"'), (put('summary', ''), '"summary"'),
            (drop('runtime'), '"runtime"'), (put('runtime', 2), '"runtime"'), (put('runtime', '1'), '"runtime"'),
            (put('runtime', True), '"runtime"'), (put('runtime', 1.5), '"runtime"'),
            (drop('presentation'), '"presentation"'), (put('presentation', 'both'), '"presentation"'),
            (put('shelf', 'archive'), '"shelf"'),
            (put('background', 'red'), '"background"'), (put('background', '#FFF'), '"background"'),
            (put('background', '#GGGGGG'), '"background"'),
            (put('taps', 'yes'), '"taps"'),
            (put('entry', 'missing.html'), 'does not exist'), (put('entry', '../x.html'), 'relative path'),
            (put('overlay', 'missing.html'), 'does not exist'), (put('overlay', '/abs.html'), 'relative path'),
            (put('anchor', [1, 2]), '"anchor" must be an object'),
            (anchor('lon', 181), 'anchor.lon'), (anchor('lon', -181), 'anchor.lon'), (anchor('lat', 91), 'anchor.lat'),
            (anchor('lat', -91), 'anchor.lat'), (anchor('zoom', 0.79), 'anchor.zoom'), (anchor('zoom', 8.1), 'anchor.zoom'),
            (anchor('lon', '1'), 'anchor.lon'), (lambda m: m['anchor'].pop('zoom'), 'anchor.zoom is required'),
            (put('markers', {}), '"markers" must be a list'), (put('markers', [3]), 'must be an object'),
            (marker('lon', 180.5), 'lon'), (marker('lat', -90.5), 'lat'), (marker('lon', None), 'lon'),
            (marker('image', 'nope.png'), 'does not exist'), (marker('image', 'index.html'), 'must be .png'),
            (marker('width', 7), 'width'), (marker('width', 201), 'width'), (marker('height', 7), 'height'),
            (marker('height', 201), 'height'), (marker('width', 'wide'), 'width'),
            (marker('label', ''), 'label'), (marker('label', '  '), 'label'), (lambda m: m['markers'][0].pop('label'), 'label'),
            (marker('minZoom', 0.7), 'minZoom'), (marker('maxZoom', 8.5), 'maxZoom'),
            (lambda m: m['markers'][0].update(minZoom=5, maxZoom=2), 'above maxZoom'),
            (marker('opens', 'yes'), 'opens'), (lambda m: m['markers'][0].pop('id'), 'id is required'),
            (lambda m: m['markers'].append(dict(m['markers'][0])), 'used twice'),
        ]
        for change, needle in cases:
            with self.subTest(needle=needle):
                shutil.rmtree(self.tmp / 'demo-pack', ignore_errors=True)
                self.assertError(self.mutate(change), needle)

    def test_screen_needs_its_entry_page(self):
        def screen(m):
            for k in ('anchor', 'markers', 'overlay', 'taps'):
                m.pop(k)
            m['presentation'] = 'screen'
            m.pop('entry')
        folder = write_pack(self.tmp, manifest=(lambda m: (screen(m), m)[1])(good_manifest()))
        self.assertEqual(findings(folder).items, [])
        (folder / 'index.html').unlink()
        self.assertError(findings(folder), 'needs its entry page')

    def test_globe_entry_is_optional_unless_a_marker_opens_it(self):
        def quiet(m):
            m.pop('entry')
            m['markers'][0]['opens'] = False
        folder = write_pack(self.tmp, manifest=(lambda m: (quiet(m), m)[1])(good_manifest()))
        (folder / 'index.html').unlink()
        self.assertEqual(self.errors(findings(folder)), [])
        manifest = good_manifest()
        manifest.pop('entry')
        (folder / 'prototype.json').write_text(json.dumps(manifest))
        self.assertError(findings(folder), 'no entry page to open')

    def test_unreadable_manifests(self):
        folder = write_pack(self.tmp)
        (folder / 'prototype.json').write_text('{nope')
        self.assertError(findings(folder), 'cannot be read')
        (folder / 'prototype.json').write_text('{"id": "a", "id": "b"}')
        self.assertError(findings(folder), 'duplicate key')
        (folder / 'prototype.json').write_text('{"runtime": NaN}')
        self.assertError(findings(folder), 'not valid JSON')
        (folder / 'prototype.json').write_text('[]')
        self.assertError(findings(folder), 'must be a JSON object')
        (folder / 'prototype.json').unlink()
        self.assertError(findings(folder), 'prototype.json is missing')

    def test_folder_name_must_be_an_id(self):
        folder = write_pack(self.tmp, 'Bad_Name', good_manifest('Bad_Name'))
        self.assertError(findings(folder), 'folder name')

    def test_unknown_keys_only_warn(self):
        def extra(m):
            m['colour'] = 'blue'
            m['anchor']['tilt'] = 3
            m['markers'][0]['glow'] = True
        rep = self.mutate(extra)
        self.assertEqual(self.errors(rep), [])
        self.assertEqual(sorted(re.search(r'"([^"]+)"', w).group(1) for w in self.warnings(rep)),
                         ['anchor.tilt', 'colour', 'markers[0].glow'])

    def test_screen_prototypes_warn_about_globe_keys(self):
        manifest = good_manifest()
        manifest['presentation'] = 'screen'
        rep = findings(write_pack(self.tmp, manifest=manifest))
        self.assertEqual(self.errors(rep), [])
        self.assertEqual(len(self.warnings(rep)), 4)

    def test_images_must_be_what_they_say(self):
        folder = write_pack(self.tmp)
        (folder / 'm.png').write_bytes(b'GIF89a....')
        self.assertError(findings(folder), 'not really a PNG')

    def test_limits(self):
        folder = write_pack(self.tmp, extra={'big.bin': b'x' * 100})
        with mock.patch.object(pg, 'MAX_FILE_BYTES', 50):
            self.assertError(findings(folder), 'file limit')
        with mock.patch.object(pg, 'MAX_PACK_BYTES', 200):
            self.assertError(findings(folder), 'pack limit')
        with mock.patch.object(pg, 'MAX_FILES', 3):
            self.assertError(findings(folder), 'files is over the 3 file limit')
        self.assertEqual(findings(folder).items, [])

    def test_the_real_limits_are_the_documented_ones(self):
        self.assertEqual((pg.MAX_PACK_BYTES, pg.MAX_FILE_BYTES, pg.MAX_FILES), (50_000_000, 25_000_000, 500))

    def test_paths_hidden_files_and_symlinks(self):
        folder = write_pack(self.tmp, extra={'.DS_Store': 'x', '.hidden/secret.txt': 'x', 'ok/data.json': '{}'})
        self.assertEqual(findings(folder).items, [])
        (folder / 'bad name.txt').write_text('x')
        self.assertError(findings(folder), 'path may use only')
        (folder / 'bad name.txt').unlink()
        (folder / 'ü.txt').write_text('x')
        self.assertError(findings(folder), 'path may use only')
        (folder / 'ü.txt').unlink()
        (folder / 'Data.json').write_text('{}')
        (folder / 'data.json').write_text('{}')
        self.assertError(findings(folder), 'only in case')
        (folder / 'Data.json').unlink()
        (folder / 'data.json').unlink()
        try:
            (folder / 'link.html').symlink_to(folder / 'index.html')
        except OSError:
            self.skipTest('symlinks unavailable')
        self.assertError(findings(folder), 'symlinks are not allowed')
        (folder / 'link.html').unlink()
        (folder / 'linkdir').symlink_to(folder / 'ok')
        self.assertError(findings(folder), 'symlinks are not allowed')

    def scan(self, name, text):
        shutil.rmtree(self.tmp / 'demo-pack', ignore_errors=True)
        rep = findings(write_pack(self.tmp, extra={name: text}))
        return self.errors(rep), self.warnings(rep)

    def test_the_offline_boundary_errors(self):
        remote = [
            ('p.html', '<script src="https://cdn.example.com/a.js"></script>'),
            ('p.html', "<img src='http://example.com/a.png'>"),
            ('p.html', '<link rel="stylesheet" href="//cdn.example.com/a.css">'),
            ('p.html', '<a href=https://example.com/>x</a>'),
            ('p.html', '<img src = "https://example.com/a.png">'),
            ('p.css', 'body { background: url(https://example.com/a.png) }'),
            ('p.css', 'body { background: url( "//example.com/a.png" ) }'),
            ('p.css', '@import "https://example.com/a.css";'),
            ('p.css', '@import url(//example.com/a.css);'),
            ('p.js', 'fetch("https://api.example.com/x")'),
            ('p.js', "fetch('//api.example.com/x')"),
            ('p.js', 'import("https://example.com/m.js")'),
            ('p.js', 'import "https://example.com/m.js";'),
            ('p.js', 'import x from "https://example.com/m.js";'),
            ('p.mjs', "import { a } from 'https://example.com/m.mjs';"),
            ('p.js', 'new WebSocket("wss://example.com/s")'),
            ('p.js', "new EventSource('https://example.com/e')"),
            ('p.js', 'x.open("GET", "https://example.com/d")'),
            ('p.js', 'navigator.sendBeacon("https://example.com/b", "x")'),
            ('p.js', 'importScripts("https://example.com/w.js")'),
            ('p.js', 'img.src = "https://example.com/a.png"'),
        ]
        for name, text in remote:
            with self.subTest(text=text):
                errors, _ = self.scan(name, text)
                self.assertTrue(any('a pack cannot load anything outside itself' in e for e in errors), errors)
                self.assertTrue(errors and not [e for e in errors if 'outside' not in e], errors)

    def test_a_mentioned_link_is_only_a_warning(self):
        for name, text in (('p.html', '<p>See https://en.wikipedia.org/wiki/Atlas for more.</p>'),
                           ('p.js', '// source: https://example.org/data.csv\nvar a = 1;'),
                           ('p.css', '/* from https://example.org/style */ body{}')):
            with self.subTest(text=text):
                errors, warnings = self.scan(name, text)
                self.assertEqual(errors, [])
                self.assertEqual(len(warnings), 1)
                self.assertIn('https://', warnings[0])

    def test_local_loads_and_namespaces_are_fine(self):
        errors, warnings = self.scan('p.html', '<svg xmlns="http://www.w3.org/2000/svg"></svg>'
                                              '<img src="m.png"><script>fetch("data.json"); import("./x.js")</script>')
        self.assertEqual(errors, [])
        self.assertEqual([w for w in warnings if 'mentions' in w], [])

    def test_missing_local_references_warn(self):
        errors, warnings = self.scan('p.html', '<img src="img/none.png"><a href="#top">x</a><a href="mailto:a@b.c">m</a>')
        self.assertEqual(errors, [])
        self.assertEqual(len(warnings), 1)
        self.assertIn('img/none.png', warnings[0])

    def test_root_absolute_references_warn(self):
        errors, warnings = self.scan('p.html', '<img src="/m.png"><script src="/x.js"></script>')
        self.assertEqual(errors, [])
        self.assertEqual(len(warnings), 2)
        self.assertTrue(all('starts with /' in w for w in warnings))

    def test_a_pack_must_not_bring_terra_js(self):
        for text in ('<script src="terra.js"></script>', '<script src="../_runtime/terra.js"></script>',
                     '<script src=/_runtime/terra.js></script>'):
            with self.subTest(text=text):
                errors, _ = self.scan('p.html', text)
                self.assertTrue(any('terra.js' in e for e in errors), errors)
        folder = write_pack(self.tmp, 'other-pack', extra={'lib/terra.js': '// a copy'})
        self.assertError(findings(folder), 'must not contain its own terra.js')

    def test_sources_json(self):
        def check(data, raw=None):
            path = self.tmp / 'sources.json'
            path.write_text(raw if raw is not None else json.dumps(data))
            rep = pg.Report()
            pg.validate_sources(path, rep)
            return rep
        good = {'format': 1, 'sources': [{'name': 'Packs', 'url': 'https://packs.example.com/catalog.json'},
                                         {'name': 'Local', 'url': 'http://127.0.0.1:8790/packs/catalog.json'},
                                         {'name': 'Local 2', 'url': 'http://localhost:8790/catalog.json'}]}
        self.assertEqual(check(good).items, [])
        self.assertEqual(check({'format': 1, 'sources': []}).items, [])
        bad = [
            ({'format': 2, 'sources': []}, '"format"'), ({'sources': []}, '"format"'), ({'format': 1}, '"sources"'),
            ({'format': 1, 'sources': [{'name': 'a', 'url': 'http://example.com/catalog.json'}]}, 'https://'),
            ({'format': 1, 'sources': [{'name': 'a', 'url': 'ftp://example.com/catalog.json'}]}, 'https://'),
            ({'format': 1, 'sources': [{'name': 'a', 'url': 'https://example.com/index.json'}]}, '/catalog.json'),
            ({'format': 1, 'sources': [{'name': 'a', 'url': 'https://u:p@example.com/catalog.json'}]}, 'credentials'),
            ({'format': 1, 'sources': [{'name': '', 'url': 'https://example.com/catalog.json'}]}, 'name'),
            ({'format': 1, 'sources': [{'url': 'https://example.com/catalog.json'}]}, 'name'),
            ({'format': 1, 'sources': [{'name': 'a'}]}, 'url'),
            ({'format': 1, 'sources': ['x']}, 'object'),
            ({'format': 1, 'sources': [{'name': 'a', 'url': 'https://e.com/catalog.json'}] * 2}, 'twice'),
        ]
        for data, needle in bad:
            with self.subTest(needle=needle):
                rep = check(data)
                self.assertTrue(any(needle in e for e in self.errors(rep)), (data, rep.items))
        self.assertTrue(any('cannot be read' in e for e in self.errors(check(None, raw='{'))))
        self.assertEqual(self.warnings(check({'format': 1, 'sources': [], 'x': 1})), ['unknown key "x"'])

    def test_validate_exit_status_and_summary(self):
        write_pack(self.tmp)
        with mock.patch.object(pg, 'PROTOTYPES', self.tmp):
            code, out, _ = run('validate')
            self.assertEqual(code, 0)
            self.assertIn('Checked 1 prototype and sources.json: 0 errors, 0 warnings.', out)
            (self.tmp / 'demo-pack' / 'prototype.json').write_text('{}')
            code, out, _ = run('validate', 'demo-pack')
            self.assertEqual(code, 1)
            self.assertRegex(out, r'ERROR \S+demo-pack/prototype.json: ')
            self.assertIn('Checked 1 prototype: ', out)


class Packing(TempCase):
    def setUp(self):
        super().setUp()
        self.src = self.tmp / 'src'
        self.out = self.tmp / 'out'
        self.clock = iter(f'2026-10-27T09:00:{n:02d}-04:00' for n in range(60))
        patcher = mock.patch.object(pg, 'now_eastern', lambda: next(self.clock))
        patcher.start()
        self.addCleanup(patcher.stop)

    def pack(self, *extra, ids=('demo-pack',)):
        code, out, err = run('pack', '--from', *[str(self.src / i) for i in ids], '--out', str(self.out), *extra)
        self.assertEqual(code, 0, out + err)
        return json.loads((self.out / 'catalog.json').read_text())

    def entry(self, catalog, pid='demo-pack'):
        return next(e for e in catalog['packs'] if e['id'] == pid)

    def test_catalog_format(self):
        write_pack(self.src)
        catalog = self.pack()
        self.assertEqual(list(catalog), ['format', 'generated', 'packs'])
        self.assertEqual(catalog['format'], 1)
        self.assertRegex(catalog['generated'], r'^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d-0[45]:00$')
        e = self.entry(catalog)
        self.assertEqual(list(e), ['id', 'revision', 'runtime', 'title', 'summary', 'presentation', 'shelf', 'hash', 'path',
                                   'bytes', 'files', 'published', 'retired'])
        self.assertEqual((e['id'], e['revision'], e['runtime'], e['path'], e['retired']), ('demo-pack', 1, 1, 'packs/demo-pack/r1/', False))
        self.assertEqual(e['bytes'], sum(f['bytes'] for f in e['files']))
        self.assertEqual([f['path'] for f in e['files']], sorted(f['path'] for f in e['files']))
        self.assertEqual(sorted(f['path'] for f in e['files']), ['index.html', 'm.png', 'overlay.html', 'prototype.json'])

    def test_files_are_byte_identical_copies(self):
        write_pack(self.src, extra={'sub/data.bin': bytes(range(256))})
        self.pack()
        for p in (self.src / 'demo-pack').rglob('*'):
            if p.is_file():
                self.assertEqual((self.out / 'packs/demo-pack/r1' / p.relative_to(self.src / 'demo-pack')).read_bytes(), p.read_bytes())

    def test_unchanged_pack_keeps_revision_and_published(self):
        write_pack(self.src)
        first = self.pack()
        raw = (self.out / 'catalog.json').read_bytes()
        again = self.pack()
        self.assertEqual(self.entry(again), self.entry(first))
        self.assertEqual(self.entry(again)['revision'], 1)
        self.assertEqual(self.entry(again)['published'], self.entry(first)['published'])
        self.assertEqual((self.out / 'catalog.json').read_bytes(), raw)

    def test_a_changed_file_is_the_next_revision(self):
        write_pack(self.src)
        first = self.entry(self.pack())
        (self.src / 'demo-pack' / 'index.html').write_text('<!doctype html><html lang="en"><body><h1>Changed</h1></body></html>')
        second = self.entry(self.pack())
        self.assertEqual(second['revision'], 2)
        self.assertEqual(second['path'], 'packs/demo-pack/r2/')
        self.assertNotEqual(second['hash'], first['hash'])
        self.assertNotEqual(second['published'], first['published'])
        self.assertIn('Changed', (self.out / 'packs/demo-pack/r2/index.html').read_text())
        self.assertIn('h1>Demo<', (self.out / 'packs/demo-pack/r1/index.html').read_text())   # the old revision stays
        (self.src / 'demo-pack' / 'extra.txt').write_text('new file')
        self.assertEqual(self.entry(self.pack())['revision'], 3)
        (self.src / 'demo-pack' / 'extra.txt').unlink()
        self.assertEqual(self.entry(self.pack())['revision'], 4)   # a removed file is a change too

    def test_a_renamed_file_changes_the_hash_even_with_the_same_content(self):
        write_pack(self.src, extra={'a.txt': 'same'})
        first = self.entry(self.pack())
        (self.src / 'demo-pack' / 'a.txt').rename(self.src / 'demo-pack' / 'b.txt')
        self.assertNotEqual(self.entry(self.pack())['hash'], first['hash'])

    def test_other_entries_are_kept_and_new_ids_start_at_one(self):
        write_pack(self.src)
        write_pack(self.src, 'second-pack')
        self.pack(ids=('demo-pack',))
        catalog = self.pack(ids=('second-pack',))
        self.assertEqual([e['id'] for e in catalog['packs']], ['demo-pack', 'second-pack'])
        self.assertEqual(self.entry(catalog, 'second-pack')['revision'], 1)
        # an entry for a pack that is not being packed (and not on disk at all) is kept as it is
        foreign = {'id': 'elsewhere', 'revision': 7, 'hash': 'abc', 'retired': True, 'mystery': [1, 2]}
        catalog['packs'].append(foreign)
        (self.out / 'catalog.json').write_text(json.dumps(catalog))
        (self.src / 'demo-pack' / 'x.txt').write_text('x')
        after = self.pack(ids=('demo-pack',))
        self.assertEqual(self.entry(after, 'elsewhere'), foreign)
        self.assertEqual(self.entry(after, 'second-pack'), self.entry(catalog, 'second-pack'))
        self.assertEqual(self.entry(after)['revision'], 2)

    def test_base_option_overrides_the_output_catalog(self):
        write_pack(self.src)
        self.pack()
        base = self.tmp / 'live.json'
        shutil.copy(self.out / 'catalog.json', base)
        live = json.loads(base.read_text())
        live['packs'][0]['revision'] = 9
        live['packs'][0]['hash'] = 'not-the-same'
        base.write_text(json.dumps(live))
        other = self.tmp / 'other-out'
        code, _, err = run('pack', '--from', str(self.src / 'demo-pack'), '--out', str(other), '--base', str(base))
        self.assertEqual(code, 0, err)
        self.assertEqual(self.entry(json.loads((other / 'catalog.json').read_text()))['revision'], 10)
        self.assertTrue((other / 'packs/demo-pack/r10/prototype.json').is_file())

    def test_an_unchanged_pack_still_gets_its_files_in_a_fresh_output(self):
        write_pack(self.src)
        self.pack()
        fresh = self.tmp / 'fresh'
        code, _, _ = run('pack', '--from', str(self.src / 'demo-pack'), '--out', str(fresh), '--base', str(self.out / 'catalog.json'))
        self.assertEqual(code, 0)
        self.assertTrue((fresh / 'packs/demo-pack/r1/prototype.json').is_file())

    def test_retired_stays_retired_until_undone(self):
        write_pack(self.src)
        self.pack()
        code, out, err = run('retire', 'demo-pack', '--out', str(self.out))
        self.assertEqual(code, 0, err)
        self.assertTrue(self.entry(json.loads((self.out / 'catalog.json').read_text()))['retired'])
        self.assertTrue((self.out / 'packs/demo-pack/r1/prototype.json').is_file())   # files untouched
        self.assertTrue(self.entry(self.pack())['retired'])
        (self.src / 'demo-pack' / 'x.txt').write_text('x')
        self.assertTrue(self.entry(self.pack())['retired'])
        code, out, _ = run('retire', 'demo-pack', '--out', str(self.out), '--undo')
        self.assertEqual(code, 0)
        self.assertFalse(self.entry(json.loads((self.out / 'catalog.json').read_text()))['retired'])
        code, out, _ = run('retire', 'demo-pack', '--out', str(self.out), '--undo')
        self.assertIn('already', out)
        code, _, err = run('retire', 'nobody', '--out', str(self.out))
        self.assertEqual(code, 2)
        self.assertIn('"nobody" is not in the catalog', err)

    def test_invalid_packs_are_refused_and_nothing_is_written(self):
        write_pack(self.src)
        bad = write_pack(self.src, 'bad-pack')
        (bad / 'prototype.json').write_text('{}')
        code, out, err = run('pack', '--from', str(self.src / 'demo-pack'), str(bad), '--out', str(self.out))
        self.assertEqual(code, 2)
        self.assertIn('nothing was packed', err)
        self.assertIn('ERROR', out)
        self.assertFalse(self.out.exists())

    def test_default_packs_everything_in_prototypes(self):
        write_pack(self.src)
        write_pack(self.src, 'second-pack')
        (self.src / '_runtime').mkdir()
        (self.src / '_runtime' / 'terra.js').write_text('//')
        with mock.patch.object(pg, 'PROTOTYPES', self.src):
            code, _, err = run('pack', '--out', str(self.out))
        self.assertEqual(code, 0, err)
        self.assertEqual([e['id'] for e in json.loads((self.out / 'catalog.json').read_text())['packs']], ['demo-pack', 'second-pack'])

    def test_open_page(self):
        write_pack(self.src, manifest=dict(good_manifest(), title='A <tricky> "title"'))
        self.pack()
        page = (self.out / 'open/demo-pack.html').read_text()
        self.assertIn('terraincognita://open/demo-pack', page)
        self.assertIn('href="terraincognita://open/demo-pack"', page)
        self.assertIn('install', page)
        self.assertIn('A &lt;tricky&gt; &quot;title&quot;', page)
        self.assertNotRegex(page, r'(?i)(src|href)\s*=\s*["\']?(https?:)?//')
        self.assertNotIn('http', page)
        self.assertNotIn('<link', page)
        self.assertEqual(pg.open_page('demo-pack', 'x'), pg.open_page('demo-pack', 'x'))   # no timestamps: repacks are identical


class New(TempCase):
    def setUp(self):
        super().setUp()
        patcher = mock.patch.object(pg, 'PROTOTYPES', self.tmp)
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_both_kinds_validate_cleanly(self):
        for pid, flag in (('tide-pool', '--globe'), ('quiet-room', '--screen')):
            with self.subTest(kind=flag):
                code, out, err = run('new', pid, flag)
                self.assertEqual(code, 0, err + out)
                rep = findings(self.tmp / pid)
                self.assertEqual(rep.items, [], rep.lines())
        manifest = json.loads((self.tmp / 'tide-pool' / 'prototype.json').read_text())
        self.assertEqual((manifest['id'], manifest['title'], manifest['presentation']), ('tide-pool', 'Tide pool', 'globe'))
        self.assertTrue((self.tmp / 'tide-pool' / 'marker.png').read_bytes().startswith(b'\x89PNG'))
        self.assertTrue((self.tmp / 'tide-pool' / 'overlay.html').is_file())
        self.assertFalse((self.tmp / 'quiet-room' / 'overlay.html').exists())
        self.assertEqual(json.loads((self.tmp / 'quiet-room' / 'prototype.json').read_text())['presentation'], 'screen')

    def test_title_is_escaped_for_json_and_html(self):
        code, _, err = run('new', 'odd-title', '--screen', '--title', 'The "odd" <one> & co')
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads((self.tmp / 'odd-title' / 'prototype.json').read_text())['title'], 'The "odd" <one> & co')
        self.assertIn('The &quot;odd&quot; &lt;one&gt; &amp; co', (self.tmp / 'odd-title' / 'index.html').read_text())
        self.assertEqual(findings(self.tmp / 'odd-title').items, [])
        self.assertNotIn('{{', (self.tmp / 'odd-title' / 'index.html').read_text())

    def test_refusals(self):
        self.assertEqual(run('new', 'one-pack', '--globe')[0], 0)
        code, _, err = run('new', 'one-pack', '--screen')
        self.assertEqual(code, 2)
        self.assertIn('already exists', err)
        for bad in ('Bad', 'x', 'no-', 'a_b', 'x' * 41, '../up'):
            with self.subTest(id=bad):
                code, _, err = run('new', bad, '--screen')
                self.assertEqual(code, 2)
                self.assertIn('not a valid id', err)
        self.assertEqual(run('new', 'long-title', '--screen', '--title', 'x' * 61)[0], 2)
        self.assertEqual(sorted(p.name for p in self.tmp.iterdir()), ['one-pack'])   # nothing half-made


class Templates(unittest.TestCase):
    """The accessibility rules the scaffolds and the delivery-check pack promise."""
    PAGES = [HERE / 'template/screen/index.html', HERE / 'template/globe/index.html', ROOT / 'Prototypes/hello-pack/index.html']

    @staticmethod
    def luminance(hex_colour):
        r, g, b = (int(hex_colour[i:i + 2], 16) / 255 for i in (1, 3, 5))
        lin = lambda c: c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4  # noqa: E731
        return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)

    def contrast(self, a, b):
        hi, lo = sorted((self.luminance(a), self.luminance(b)), reverse=True)
        return (hi + 0.05) / (lo + 0.05)

    def test_pages(self):
        for page in self.PAGES:
            text = page.read_text()
            with self.subTest(page=page.relative_to(ROOT).as_posix()):
                self.assertIn('<html lang="en">', text)
                self.assertIn('<meta name="viewport"', text)
                self.assertIn('viewport-fit=cover', text)
                for needle in ('<main', '<h1', '<button type="button"', '--terra-text-scale', 'safe-area-inset-top',
                               'ui-serif, "New York", Georgia, serif', 'terra.close()'):
                    self.assertIn(needle, text)
                self.assertIn('56px', text)                                   # the Back button's zone, below the safe area
                for size in re.findall(r'font-size:\s*(?:calc\()?(\d+(?:\.\d+)?)px', text):
                    self.assertGreaterEqual(float(size), 15, 'text below 15px at scale 1')
                colours = dict(re.findall(r'--(paper|ink|quiet|focus):\s*(#[0-9A-Fa-f]{6})', text))
                self.assertEqual(set(colours), {'paper', 'ink', 'quiet', 'focus'})
                self.assertGreaterEqual(self.contrast(colours['ink'], colours['paper']), 4.5)
                self.assertGreaterEqual(self.contrast(colours['quiet'], colours['paper']), 4.5)
                self.assertGreaterEqual(self.contrast(colours['focus'], colours['paper']), 3)   # a focus ring, not text

    def test_scaffolds_honour_reduced_motion(self):
        for page in self.PAGES[:2]:
            self.assertIn('[data-reduce-motion]', page.read_text())
        self.assertIn('terra.settings.reduceMotion', (HERE / 'template/globe/overlay.html').read_text())
        self.assertIn('terra.settings.reduceMotion', (ROOT / 'Prototypes/hello-pack/overlay.js').read_text())

    def test_hello_pack(self):
        pack = ROOT / 'Prototypes/hello-pack'
        manifest = json.loads((pack / 'prototype.json').read_text())
        self.assertEqual((manifest['title'], manifest['summary'], manifest['presentation'], manifest['taps']),
                         ('Delivery check', 'A test pack: which revision arrived, and from where.', 'globe', True))
        self.assertTrue(manifest['markers'][0]['opens'])
        index = (pack / 'index.html').read_text()
        self.assertEqual(index.count('First words.'), 1)
        self.assertIn('<p id="note">First words.</p>', index)
        self.assertIn('Back to the globe', index)
        self.assertEqual(findings(pack).items, [])


class FakeBucket(http.server.ThreadingHTTPServer):
    """An S3-shaped server that checks every signature the way S3 would."""
    daemon_threads = True

    def __init__(self, access, secret, bucket):
        super().__init__(('127.0.0.1', 0), FakeBucketHandler)
        self.access, self.secret, self.bucket = access, secret, bucket
        self.objects, self.puts, self.gets, self.bad = {}, [], [], []

    @property
    def url(self):
        return f'http://127.0.0.1:{self.server_address[1]}'


class FakeBucketHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def authorised(self, body):
        s = self.server
        m = re.match(r'AWS4-HMAC-SHA256 Credential=([^/]+)/(\d{8})/auto/s3/aws4_request,SignedHeaders=([^,]+),Signature=([0-9a-f]{64})$',
                     self.headers.get('Authorization', ''))
        if not m or m.group(1) != s.access:
            return False
        names = m.group(3).split(';')
        if not {'host', 'x-amz-content-sha256', 'x-amz-date'} <= set(names):
            return False
        if self.headers['x-amz-content-sha256'] != hashlib.sha256(body).hexdigest():
            return False
        headers = {n: self.headers[n] for n in names}
        auth, _, _ = pg.sigv4(self.command, urllib.parse.unquote(self.path), {}, headers, self.headers['x-amz-content-sha256'],
                              s.access, s.secret, 'auto', 's3', self.headers['x-amz-date'])
        return auth.endswith(m.group(4))

    def reply(self, status, body=b'', ctype='application/xml'):
        self.send_response(status)
        self.send_header('Content-Type', ctype)
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        s = self.server
        path = urllib.parse.unquote(self.path)
        if path == '/public/catalog.json':                       # the public URL, unsigned
            obj = s.objects.get('catalog.json')
            return self.reply(200, obj[0], 'application/json') if obj else self.reply(404)
        if not self.authorised(b''):
            s.bad.append(path)
            return self.reply(403, b'<Error><Code>SignatureDoesNotMatch</Code></Error>')
        key = path[len(f'/{s.bucket}/'):]
        s.gets.append(key)
        if key in s.objects:
            return self.reply(200, s.objects[key][0], 'application/json')
        self.reply(404, b'<Error><Code>NoSuchKey</Code></Error>')

    def do_PUT(self):
        s = self.server
        body = self.rfile.read(int(self.headers.get('Content-Length', 0)))
        path = urllib.parse.unquote(self.path)
        if not self.authorised(body):
            s.bad.append(path)
            return self.reply(403, b'<Error><Code>SignatureDoesNotMatch</Code></Error>')
        key = path[len(f'/{s.bucket}/'):]
        s.objects[key] = (body, self.headers.get('Content-Type'), self.headers.get('Cache-Control'))
        s.puts.append(key)
        self.reply(200)


class Publishing(TempCase):
    def setUp(self):
        super().setUp()
        self.src = self.tmp / 'protos'
        write_pack(self.src)
        write_pack(self.src, 'second-pack')
        patcher = mock.patch.object(pg, 'PROTOTYPES', self.src)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.bucket = FakeBucket('AKIATESTKEY', 'secret/key+with=odd', 'terra-packs')
        threading.Thread(target=self.bucket.serve_forever, kwargs={"poll_interval": 0.01}, daemon=True).start()
        self.addCleanup(self.bucket.server_close)
        self.addCleanup(self.bucket.shutdown)
        self.env = {'R2_ACCOUNT_ID': 'acct', 'R2_ACCESS_KEY_ID': 'AKIATESTKEY', 'R2_SECRET_ACCESS_KEY': 'secret/key+with=odd',
                    'R2_BUCKET': 'terra-packs', 'PACKS_PUBLIC_URL': 'https://packs.example.test/', 'R2_ENDPOINT': self.bucket.url}

    def publish(self, *argv, env=None):
        return run('publish', *argv, env=self.env if env is None else env)

    def test_first_publish_uploads_everything_with_the_catalog_last(self):
        code, out, err = self.publish()
        self.assertEqual(code, 0, out + err)
        self.assertEqual(self.bucket.bad, [])
        puts = self.bucket.puts
        self.assertEqual(puts[-1], 'catalog.json')
        self.assertEqual(puts.count('catalog.json'), 1)
        self.assertIn('packs/demo-pack/r1/prototype.json', puts)
        self.assertIn('open/second-pack.html', puts)
        body, ctype, cache = self.bucket.objects['packs/demo-pack/r1/m.png']
        self.assertEqual((ctype, cache), ('image/png', 'public, max-age=31536000, immutable'))
        self.assertEqual(body, PNG)
        self.assertEqual(self.bucket.objects['packs/demo-pack/r1/index.html'][1], 'text/html; charset=utf-8')
        self.assertEqual(self.bucket.objects['open/demo-pack.html'][2], 'no-cache')
        self.assertEqual(self.bucket.objects['catalog.json'][1:], ('application/json', 'no-cache, max-age=0'))
        catalog = json.loads(self.bucket.objects['catalog.json'][0])
        self.assertEqual([(e['id'], e['revision']) for e in catalog['packs']], [('demo-pack', 1), ('second-pack', 1)])
        self.assertIn('https://packs.example.test/open/demo-pack.html', out)       # no double slash
        self.assertIn('https://packs.example.test/catalog.json', out)

    def test_a_second_publish_with_nothing_changed_uploads_nothing(self):
        self.publish()
        before = len(self.bucket.puts)
        code, out, _ = self.publish()
        self.assertEqual(code, 0)
        self.assertEqual(len(self.bucket.puts), before)
        self.assertIn('Nothing to publish', out)

    def test_only_a_changed_pack_goes_up_and_the_catalog_last(self):
        self.publish()
        n = len(self.bucket.puts)
        (self.src / 'demo-pack' / 'index.html').write_text('<!doctype html><html lang="en"><body>new</body></html>')
        summary = self.tmp / 'summary.md'
        code, out, err = self.publish('--summary', str(summary))
        self.assertEqual(code, 0, err)
        new = self.bucket.puts[n:]
        self.assertEqual(new[-1], 'catalog.json')
        self.assertTrue(all(k.startswith(('packs/demo-pack/r2/', 'open/demo-pack.html')) or k == 'catalog.json' for k in new), new)
        self.assertEqual(len(new), 4 + 1 + 1)   # four files, its open page, the catalog
        self.assertEqual(json.loads(self.bucket.objects['catalog.json'][0])['packs'][0]['revision'], 2)
        self.assertIn('packs/demo-pack/r1/index.html', self.bucket.objects)          # the old revision stays
        text = summary.read_text()
        self.assertIn('https://packs.example.test/open/demo-pack.html', text)
        self.assertNotIn('second-pack', text)

    def test_dry_run_uploads_nothing(self):
        code, out, _ = self.publish('--dry-run')
        self.assertEqual(code, 0)
        self.assertEqual(self.bucket.puts, [])
        self.assertIn('Dry run: would upload', out)
        self.assertIn('packs/demo-pack/r1/prototype.json', out)
        self.assertLess(out.index('packs/demo-pack/r1/prototype.json'), out.rindex('catalog.json'))
        self.assertIn('catalog.json', self.bucket.gets)

    def test_dry_run_falls_back_to_the_public_catalog_without_credentials(self):
        self.publish()
        (self.src / 'demo-pack' / 'x.txt').write_text('x')
        env = {'PACKS_PUBLIC_URL': self.bucket.url + '/public'}
        n = len(self.bucket.puts)
        code, out, err = self.publish('--dry-run', env=env)
        self.assertEqual(code, 0, err)
        self.assertIn('reading the public catalog', out)
        self.assertIn('r1 -> r2', out)
        self.assertEqual(len(self.bucket.puts), n)

    def test_missing_environment_is_one_clear_line(self):
        for argv in ((), ('--dry-run',)):
            code, out, err = self.publish(*argv, env={})
            self.assertEqual(code, 2)
            for name in pg.R2_VARS:
                self.assertIn(name, err)
            self.assertNotIn('Traceback', err)
        code, _, err = self.publish(env={k: v for k, v in self.env.items() if k != 'R2_BUCKET'})
        self.assertEqual(code, 2)
        self.assertIn('R2_BUCKET', err)
        self.assertEqual(self.bucket.puts, [])

    def test_a_wrong_secret_is_reported_not_raised(self):
        code, _, err = self.publish(env=dict(self.env, R2_SECRET_ACCESS_KEY='wrong'))
        self.assertEqual(code, 2)
        self.assertIn('403', err)
        self.assertIn('SignatureDoesNotMatch', err)
        self.assertNotIn('Traceback', err)

    def test_an_unreachable_bucket_is_a_clear_message(self):
        with mock.patch.object(pg.time, 'sleep'):
            code, _, err = self.publish(env=dict(self.env, R2_ENDPOINT='http://127.0.0.1:9'))
        self.assertEqual(code, 2)
        self.assertIn('cannot reach', err)
        self.assertNotIn('Traceback', err)

    def test_retire_in_the_bucket(self):
        self.publish()
        puts = len(self.bucket.puts)
        code, out, err = run('retire', 'second-pack', env=self.env)
        self.assertEqual(code, 0, err)
        self.assertEqual(self.bucket.puts[puts:], ['catalog.json'])                 # the catalog only
        by_id = {e['id']: e for e in json.loads(self.bucket.objects['catalog.json'][0])['packs']}
        self.assertTrue(by_id['second-pack']['retired'])
        self.assertFalse(by_id['demo-pack']['retired'])
        code, _, _ = run('retire', 'second-pack', '--undo', env=self.env)
        self.assertEqual(code, 0)
        self.assertFalse(json.loads(self.bucket.objects['catalog.json'][0])['packs'][1]['retired'])

    def test_list(self):
        self.publish()
        code, out, err = run('list', '--remote', self.bucket.url + '/public/catalog.json')
        self.assertEqual(code, 0, err)
        self.assertRegex(out, r'demo-pack\s+globe\s+current\s+Demo')
        self.assertRegex(out, r'demo-pack\s+r1\s')
        self.assertIn('Remote (', out)
        self.assertEqual(run('list')[0], 0)


class RemotePacksScript(TempCase):
    def test_the_remote_copy_differs_from_the_bundled_one(self):
        out = self.tmp / 'remote'
        done = subprocess.run(['bash', str(ROOT / 'ci/remote_packs.sh'), str(out)], capture_output=True, text=True, cwd=self.tmp)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        catalog = json.loads((out / 'catalog.json').read_text())
        entry = next(e for e in catalog['packs'] if e['id'] == 'hello-pack')
        self.assertEqual(entry['revision'], 1)
        self.assertIn('Second words.', (out / 'packs/hello-pack/r1/index.html').read_text())
        self.assertNotIn('First words.', (out / 'packs/hello-pack/r1/index.html').read_text())
        self.assertIn('First words.', (ROOT / 'Prototypes/hello-pack/index.html').read_text())   # the bundled copy is untouched
        bundled = pg.Pack(ROOT / 'Prototypes/hello-pack', *pg.validate_pack(ROOT / 'Prototypes/hello-pack', pg.Report()))
        self.assertNotEqual(entry['hash'], bundled.hash)
        self.assertIn('hello-pack', done.stdout)                                      # the catalog is printed


if __name__ == '__main__':
    unittest.main()

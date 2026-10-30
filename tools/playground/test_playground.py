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


MAIN_JS = '''(function () {
  'use strict';
  terra.caption.set({ title: 'Demo', text: 'A demonstration.' });
  terra.sprites.set('m', { at: [-30, 30], looks: [{ image: 'm.png', size: [44, 44] }],
                           label: 'Demo marker', touchable: true });
  terra.on('touch', function (t) {
    if (!t.target || t.target.sprite !== 'm') return;
    terra.panel.show({ title: 'Demo', text: 'More words.',
                       sources: [{ title: 'A source', url: 'https://example.org/source', note: 'the facts' }] });
  });
})();
'''


def good_manifest(pid='demo-pack'):
    return {
        'id': pid, 'title': 'Demo', 'summary': 'A demonstration.', 'runtime': 3, 'presentation': 'globe',
        'scripts': ['main.js'], 'anchor': {'lon': -30, 'lat': 30, 'zoom': 1.6},
        'doorway': {'lon': -30, 'lat': 30, 'label': 'Demo doorway',
                    'look': {'image': 'm.png', 'size': [28, 28], 'minZoom': 1.5, 'maxZoom': 8}},
        'taps': True, 'shelf': 'current',
    }


def write_pack(root, pid='demo-pack', manifest=None, extra=None):
    folder = Path(root) / pid
    folder.mkdir(parents=True, exist_ok=True)
    (folder / 'prototype.json').write_text(json.dumps(manifest or good_manifest(pid)), encoding='utf-8')
    (folder / 'main.js').write_text(MAIN_JS, encoding='utf-8')
    (folder / 'SOURCES.md').write_text('# Demo\n\nWhere its facts come from: https://example.org/source\n', encoding='utf-8')
    (folder / 'm.png').write_bytes(PNG)
    for name, data in (extra or {}).items():
        (folder / name).parent.mkdir(parents=True, exist_ok=True)
        (folder / name).write_bytes(data if isinstance(data, bytes) else data.encode('utf-8'))
    return folder


def findings(folder):
    rep = pg.Report()
    pg.validate_pack(folder, rep)
    return rep


def tree(folder):
    """{posix path: bytes} of every file under folder."""
    folder = Path(folder)
    return {p.relative_to(folder).as_posix(): p.read_bytes() for p in sorted(folder.rglob('*')) if p.is_file()}


class TempCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix='playground-test-'))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)


class Shared(TempCase):
    """The packs' shared script (docs/runtime3.md › Shared script): a pack's copy must be the source, and
    `share` writes the copies into the packs that list it."""
    def setUp(self):
        super().setUp()
        self.protos = self.tmp / 'Prototypes'
        (self.protos / '_shared').mkdir(parents=True)
        (self.protos / '_shared' / 'shared.js').write_text('window.Shared = {};\n', encoding='utf-8')
        for name, value in (('PROTOTYPES', self.protos), ('SHARED', self.protos / '_shared' / 'shared.js')):
            self.addCleanup(setattr, pg, name, getattr(pg, name))
            setattr(pg, name, value)

    def test_share_writes_the_copies_and_validate_holds_them_to_the_source(self):
        manifest = good_manifest()
        manifest['scripts'] = ['shared.js', 'main.js']
        folder = write_pack(self.protos, manifest=manifest, extra={'shared.js': 'window.Shared = { old: 1 };\n'})
        write_pack(self.protos, 'other-pack')
        rep = findings(folder)
        self.assertTrue(any('differs from' in m and 'playground.py share' in m for level, _, m in rep.items if level == 'ERROR'), rep.lines())
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(pg.main(['share']), 0)
        self.assertEqual((folder / 'shared.js').read_text(), 'window.Shared = {};\n')
        self.assertFalse((self.protos / 'other-pack' / 'shared.js').exists())
        self.assertEqual(findings(folder).items, [])


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

    def test_the_smallest_pack_is_clean(self):
        manifest = {k: v for k, v in good_manifest().items() if k in ('id', 'title', 'summary', 'runtime', 'presentation', 'scripts')}
        self.assertEqual(findings(write_pack(self.tmp, manifest=manifest)).items, [])

    def test_every_class_of_manifest_error(self):
        def drop(key):
            return lambda m: m.pop(key)

        def put(key, value):
            return lambda m: m.__setitem__(key, value)

        def anchor(key, value):
            return lambda m: m['anchor'].__setitem__(key, value)

        def door(key, value):
            return lambda m: m['doorway'].__setitem__(key, value)

        def look(key, value):
            return lambda m: m['doorway']['look'].__setitem__(key, value)

        def frames(fs, **more):
            def change(m):
                m['doorway']['look'].pop('image')
                m['doorway']['look'].update(frames=fs, **more)
            return change

        cases = [
            (drop('id'), '"id" is required'), (put('id', 'other-id'), 'must equal the folder name'),
            (put('id', 7), '"id" is required'),
            (drop('title'), '"title"'), (put('title', ''), '"title"'), (put('title', 'x' * 61), '"title"'),
            (put('title', '   '), '"title"'), (put('title', 3), '"title"'),
            (drop('summary'), '"summary"'), (put('summary', 'x' * 201), '"summary"'), (put('summary', ''), '"summary"'),
            (drop('runtime'), '"runtime" must be 3'), (put('runtime', 7), '"runtime" must be 3 to 6'),
            (put('runtime', 0), '"runtime" must be 3'), (put('runtime', '3'), '"runtime" must be 3'),
            (put('runtime', True), '"runtime" must be 3'), (put('runtime', 3.0), '"runtime" must be 3'),
            (drop('presentation'), '"presentation" must be "globe"'), (put('presentation', 'both'), '"presentation" must be "globe"'),
            (put('presentation', 'screen'), 'no pack screens'),
            (put('shelf', 'archive'), '"shelf"'), (put('taps', 'yes'), '"taps"'),
            (drop('scripts'), '"scripts" is required'), (put('scripts', []), 'non-empty list'),
            (put('scripts', 'main.js'), 'non-empty list'), (put('scripts', ['missing.js']), 'does not exist'),
            (put('scripts', ['../main.js']), 'relative path'), (put('scripts', ['SOURCES.md']), 'must be .js'),
            (put('scripts', ['main.js', 'main.js']), 'listed twice'), (put('scripts', [3]), 'must be a path'),
            (put('entry', 'main.js'), 'runtime 3 only'), (put('overlay', 'main.js'), 'runtime 3 only'),
            (put('markers', []), 'runtime 3 only'),
            (put('anchor', [1, 2]), '"anchor" must be an object'),
            (anchor('lon', 181), 'anchor.lon'), (anchor('lon', -181), 'anchor.lon'), (anchor('lat', 91), 'anchor.lat'),
            (anchor('lat', -91), 'anchor.lat'), (anchor('zoom', 0.79), 'anchor.zoom'), (anchor('zoom', 8.1), 'anchor.zoom'),
            (anchor('lon', '1'), 'anchor.lon'), (lambda m: m['anchor'].pop('zoom'), 'anchor.zoom is required'),
            (put('doorway', [1]), '"doorway" must be an object'),
            (door('lon', 180.5), 'doorway.lon'), (door('lat', -90.5), 'doorway.lat'), (door('lon', None), 'doorway.lon'),
            (lambda m: m['doorway'].pop('lat'), 'doorway.lat is required'),
            (door('label', ''), 'doorway.label'), (door('label', '  '), 'doorway.label'), (door('label', 'x' * 201), 'doorway.label'),
            (lambda m: m['doorway'].pop('label'), 'doorway.label'),
            (lambda m: m['doorway'].pop('look'), 'doorway.look is required'), (door('look', 3), 'doorway.look must be an object'),
            (look('image', 'nope.png'), 'does not exist'), (look('image', 'main.js'), 'must be .png'),
            (look('frames', ['m.png']), 'not both'), (lambda m: m['doorway']['look'].pop('image'), 'not both'),
            (frames([], fps=4), 'frames must be a list'), (frames(['m.png'] * 25, fps=4), 'frames must be a list'),
            (frames('m.png', fps=4), 'frames must be a list'), (frames(['m.png', 'nope.png'], fps=4), 'frames[1]'),
            (frames(['m.png']), 'fps is required'), (frames(['m.png'], fps=0), 'fps'), (frames(['m.png'], fps='fast'), 'fps'),
            (lambda m: m['doorway']['look'].pop('size'), 'size is required'), (look('size', [10]), 'size must be'),
            (look('size', 10), 'size must be'), (look('size', ['a', 2]), 'size width'), (look('size', [0, 10]), 'size width'),
            (look('size', [10, 600]), 'size height'),
            (look('minZoom', 0.7), 'minZoom'), (look('maxZoom', 8.5), 'maxZoom'),
            (lambda m: m['doorway']['look'].update(minZoom=5, maxZoom=2), 'above maxZoom'),
            (lambda m: m['doorway']['look'].update(every=2, **{'for': 3}), 'longer than every'),
            (lambda m: m['doorway']['look'].update(every=-1, **{'for': 1}), 'every'),
            (look('face', 'yes'), 'face'),
        ]
        for change, needle in cases:
            with self.subTest(needle=needle):
                shutil.rmtree(self.tmp / 'demo-pack', ignore_errors=True)
                self.assertError(self.mutate(change), needle)

    def test_runtime_1_and_2_manifests_fail_with_one_clear_message(self):
        old = {'id': 'demo-pack', 'title': 'Demo', 'summary': 'A demonstration.', 'presentation': 'screen',
               'entry': 'index.html', 'overlay': 'overlay.html', 'background': '#F6F2E7'}
        for runtime in (1, 2):
            with self.subTest(runtime=runtime):
                shutil.rmtree(self.tmp / 'demo-pack', ignore_errors=True)
                folder = write_pack(self.tmp, manifest=dict(old, runtime=runtime),
                                    extra={'index.html': '<!doctype html><title>x</title>', 'overlay.html': '<p>x</p>'})
                rep = findings(folder)
                self.assertEqual(len(rep.items), 1, rep.lines())
                self.assertIn(f'"runtime" is {runtime}: runtime 3 only: a pack is scripts drawn by the globe', self.errors(rep)[0])

    def test_web_pages_are_refused(self):
        rep = findings(write_pack(self.tmp, extra={'index.html': '<!doctype html><title>x</title>', 'about/more.htm': 'x'}))
        self.assertEqual(len(self.errors(rep)), 2)
        self.assertTrue(all('a web page: runtime 3 only: a pack is scripts drawn by the globe' in e for e in self.errors(rep)))

    def test_unknown_keys_only_warn(self):
        def extra(m):
            m['colour'] = 'blue'
            m['background'] = '#F6F2E7'
            m['anchor']['tilt'] = 3
            m['doorway']['glow'] = True
            m['doorway']['look']['sparkle'] = 1
        rep = self.mutate(extra)
        self.assertEqual(self.errors(rep), [])
        self.assertEqual(sorted(re.search(r'"([^"]+)"', w).group(1) for w in self.warnings(rep)),
                         ['anchor.tilt', 'background', 'colour', 'doorway.glow', 'doorway.look.sparkle'])

    def test_doorway_advice(self):
        rep = self.mutate(lambda m: m['doorway']['look'].pop('minZoom'))
        self.assertEqual(self.errors(rep), [])
        self.assertEqual(len(self.warnings(rep)), 1)
        self.assertIn('keep that view sparse', self.warnings(rep)[0])
        trace = lambda m: (m['doorway']['look'].pop('minZoom'), m['doorway']['look'].update(every=11, **{'for': 1.4}))  # noqa: E731
        shutil.rmtree(self.tmp / 'demo-pack')
        self.assertEqual(self.mutate(trace).items, [])
        shutil.rmtree(self.tmp / 'demo-pack')
        rep = self.mutate(lambda m: m['doorway']['look'].update(every=11))
        self.assertEqual(self.errors(rep), [])
        self.assertIn('"every" and "for" go together', self.warnings(rep)[0])
        shutil.rmtree(self.tmp / 'demo-pack')
        rep = self.mutate(lambda m: m['doorway']['look'].update(fps=4))
        self.assertEqual(self.errors(rep), [])
        self.assertIn('fps only times "frames"', self.warnings(rep)[0])
        shutil.rmtree(self.tmp / 'demo-pack')
        rep = self.mutate(lambda m: m.update(shelf='parked'))
        self.assertEqual(self.errors(rep), [])
        self.assertIn('a parked pack shows no doorway', self.warnings(rep)[0])

    def test_a_doorway_may_be_frames(self):
        def frames(m):
            m['doorway']['look'] = {'frames': ['m.png', 'f/2.png', 'm.png'], 'fps': 5, 'size': [46, 20], 'minZoom': 3, 'face': True}
        self.assertEqual(self.mutate(frames, extra={'f/2.png': PNG}).items, [])

    def test_a_doorway_may_be_frames_of_a_sprite_sheet(self):
        """Frames of a sprite sheet (docs/runtime3.md › Sprites, runtime 6): 'sheet.png#x,y,w,h', a rectangle
        of whole pixels inside the sheet."""
        # (a PNG's header, 8 × 4 pixels: what the check reads)
        sheet_png = b'\x89PNG\r\n\x1a\n' + b'\x00\x00\x00\x0dIHDR' + (8).to_bytes(4, 'big') + (4).to_bytes(4, 'big') + b'\x08\x06\x00\x00\x00' + b'\x00' * 4
        side = (8, 4)
        def sheet(frames, runtime=6):
            def change(m):
                m['runtime'] = runtime
                m['doorway']['look'] = {'frames': frames, 'fps': 5, 'size': [46, 20], 'minZoom': 3}
            return change
        whole = f's.png#0,0,{side[0]},{side[1]}'
        self.assertEqual(self.mutate(sheet([whole, 's.png#0,0,4,4', 'm.png']), extra={'s.png': sheet_png}).items, [])
        for frames, runtime, needle in (([whole], 5, 'new in runtime 6'),
                                        ([f's.png#1,0,{side[0]},{side[1]}'], 6, 'lies outside its sheet'),
                                        (['s.png#0,0,2'], 6, 'sheet.png#x,y,width,height'),
                                        (['s.png#0,0,0,2'], 6, 'sheet.png#x,y,width,height'),
                                        (['gone.png#0,0,1,1'], 6, 'does not exist')):
            shutil.rmtree(self.tmp / 'demo-pack')
            self.assertError(self.mutate(sheet(frames, runtime), extra={'s.png': sheet_png}), needle)

    def test_a_doorway_with_a_life_is_runtime_4(self):
        """Looks by zoom, a path and a breath (docs/runtime3.md › Many at once) need "runtime": 4; a
        runtime-3 pack with them is told so in one error."""
        def life(m):
            m['runtime'] = 4
            m['doorway'].pop('look')
            m['doorway']['looks'] = [{'image': 'm.png', 'size': [11, 11], 'maxZoom': 1.5, 'every': 9, 'for': 1.2},
                                     {'frames': ['m.png', 'f/2.png'], 'fps': 2, 'size': [18, 21], 'minZoom': 1.5, 'face': True}]
            m['doorway']['path'] = {'coordinates': [[-30.2, 29.9], [-29.8, 30.1]], 'seconds': 150, 'loop': 'back'}
            m['doorway']['breathes'] = True
        self.assertEqual(self.mutate(life, extra={'f/2.png': PNG}).items, [])
        shutil.rmtree(self.tmp / 'demo-pack')
        rep = self.mutate(lambda m: (life(m), m.update(runtime=3)), extra={'f/2.png': PNG})
        self.assertEqual(len(self.errors(rep)), 1, rep.lines())
        self.assertIn('doorway.looks is new in runtime 4', self.errors(rep)[0])
        cases = [
            (lambda m: (life(m), m['doorway'].__setitem__('look', m['doorway']['looks'][1])), 'not both'),
            (lambda m: (life(m), m['doorway'].__setitem__('looks', [])), 'list of 1 to 8 looks'),
            (lambda m: (life(m), m['doorway']['looks'].__setitem__(1, 3)), 'doorway.looks[1] must be an object'),
            (lambda m: (life(m), m['doorway'].__setitem__('path', [1, 2])), 'doorway.path must be an object'),
            (lambda m: (life(m), m['doorway']['path'].__setitem__('coordinates', [[0, 0]])), '2 to 200 places'),
            (lambda m: (life(m), m['doorway']['path'].__setitem__('coordinates', [[0, 0], [0, 91]])), 'lat'),
            (lambda m: (life(m), m['doorway']['path'].__setitem__('coordinates', [[0, 0], 'x'])), 'must be [lon, lat]'),
            (lambda m: (life(m), m['doorway']['path'].__setitem__('seconds', 0)), 'doorway.path.seconds'),
            (lambda m: (life(m), m['doorway']['path'].__setitem__('loop', 'forth')), 'true, false or "back"'),
            (lambda m: (life(m), m['doorway'].__setitem__('breathes', 'yes')), 'doorway.breathes must be true or false'),
        ]
        for change, needle in cases:
            with self.subTest(needle=needle):
                shutil.rmtree(self.tmp / 'demo-pack', ignore_errors=True)
                self.assertError(self.mutate(change, extra={'f/2.png': PNG}), needle)
        shutil.rmtree(self.tmp / 'demo-pack')
        rep = self.mutate(lambda m: (life(m), m['doorway']['path'].__setitem__('speed', 3)), extra={'f/2.png': PNG})
        self.assertEqual(self.errors(rep), [])
        self.assertIn('doorway.path.speed', self.warnings(rep)[0])
        shutil.rmtree(self.tmp / 'demo-pack')
        rep = self.mutate(lambda m: (life(m), m['doorway']['looks'][0].pop('every'), m['doorway']['looks'][0].pop('for')), extra={'f/2.png': PNG})
        self.assertEqual(self.errors(rep), [])
        self.assertIn('doorway.looks[0] shows on the whole Earth', self.warnings(rep)[0])

    def test_unlisted_scripts_and_missing_sources_warn(self):
        folder = write_pack(self.tmp, extra={'data.js': 'window.DATA = { a: 1 };', 'tool.cjs': 'module.exports = 1;'})
        rep = findings(folder)
        self.assertEqual(self.errors(rep), [])
        self.assertEqual(len(self.warnings(rep)), 1)
        self.assertIn('is not in "scripts", so it never runs', self.warnings(rep)[0])
        manifest = good_manifest()
        manifest['scripts'] = ['data.js', 'main.js']
        (folder / 'prototype.json').write_text(json.dumps(manifest))
        self.assertEqual(findings(folder).items, [])
        (folder / 'SOURCES.md').unlink()
        rep = findings(folder)
        self.assertEqual(self.errors(rep), [])
        self.assertIn('SOURCES.md is missing', self.warnings(rep)[0])

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
        self.assertEqual((pg.MAX_PACK_BYTES, pg.MAX_FILE_BYTES, pg.MAX_FILES, pg.MAX_FRAMES), (50_000_000, 25_000_000, 500, 24))

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
            (folder / 'link.txt').symlink_to(folder / 'SOURCES.md')
        except OSError:
            self.skipTest('symlinks unavailable')
        self.assertError(findings(folder), 'symlinks are not allowed')
        (folder / 'link.txt').unlink()
        (folder / 'linkdir').symlink_to(folder / 'ok')
        self.assertError(findings(folder), 'symlinks are not allowed')

    def scan(self, text, name='main.js'):
        shutil.rmtree(self.tmp / 'demo-pack', ignore_errors=True)
        folder = write_pack(self.tmp, extra={name: text})
        rep = findings(folder)
        return self.errors(rep), self.warnings(rep)

    def test_no_network_and_no_code_from_outside(self):
        cases = [
            ('fetch("data.json")', 'fetch()'), ('window.fetch(url)', 'fetch()'), ('globalThis.fetch (u)', 'fetch()'),
            ('var x = new XMLHttpRequest();', 'XMLHttpRequest'), ('var s = new WebSocket("wss://example.com/s");', 'WebSocket'),
            ('new EventSource(u)', 'EventSource'), ('navigator.sendBeacon(u, d)', 'sendBeacon()'),
            ('importScripts("x.js")', 'importScripts()'), ('new Worker("w.js")', 'a Worker'),
            ('import("./x.js").then(go)', 'import()'), ('import x from "./x.js";', 'an import statement'),
            ("import './x.js';", 'an import statement'), ('import { a } from "./a.js";', 'an import statement'),
            ('import * as a from "./a.js";', 'an import statement'), ('var u = import.meta.url;', 'an import statement'),
            ('if (ok) { var t = `${fetch(u)}`; }', 'fetch()'),
        ]
        for text, what in cases:
            with self.subTest(text=text):
                errors, _ = self.scan(text)
                self.assertTrue(any(e.startswith(what + ':') for e in errors), errors)
        errors, _ = self.scan('fetch(u)', name='lib/mod.mjs')
        self.assertTrue(any(e.startswith('fetch()') for e in errors), errors)

    def test_the_same_words_in_strings_comments_and_names_are_fine(self):
        text = '\n'.join([
            "// we never fetch() anything: no XMLHttpRequest, no import()",
            '/* new WebSocket("x"); import x from "y"; */',
            "var s = \"fetch(this) and import('x')\";",
            "var t = `no fetch() ${ ({ a: 1 }).a } here, nor XMLHttpRequest ${ `nested fetch()` }`;",
            "var cache = { fetch: function () {}, import: 1 };",
            "cache.fetch('k'); prefetch(1); fetchData(2); var important = 1; cache.import(3);",
            "var re = /fetch\\(/g, re2 = /[\"'/]/, half = 4 / 2 / 1;",
            "var q = 'it\\'s fine: fetch(x)';",
            "function f() { return /import\\(/.test(s); }",
        ])
        errors, warnings = self.scan(text)
        self.assertEqual(errors, [])
        self.assertEqual(warnings, [])

    def test_links_in_scripts_only_warn_unless_they_are_sources(self):
        quiet = [
            "terra.panel.show({ sources: [{ title: 'S', url: 'https://example.org/a', note: 'n' }] });",
            'var d = { "url": "https://example.org/b" };',
        ]
        for text in quiet:
            with self.subTest(text=text):
                self.assertEqual(self.scan(text), ([], []))
        for text in ('// from https://example.org/data.csv\nvar a = 1;',
                     "var credit = 'https://example.org/c';",
                     "var a = 'https://x.org/1', b = 'see https://x.org/1';",
                     "var ws = 'wss://x.org/socket';"):
            with self.subTest(text=text):
                errors, warnings = self.scan(text)
                self.assertEqual(errors, [])
                self.assertEqual(len(warnings), 1, warnings)
                self.assertIn('://', warnings[0])
                self.assertIn('never loaded', warnings[0])

    def test_pictures_a_script_names_must_be_in_the_pack(self):
        errors, warnings = self.scan("terra.sprites.set('a', { at: [0, 0], looks: [{ image: 'nope.png', size: [4, 4] },"
                                     " { frames: ['m.png', 'gone/2.jpg', 'nope.png'], fps: 2, size: [4, 4] }] });")
        self.assertEqual(errors, [])
        self.assertEqual(sorted(re.search(r'"([^"]+)"', w).group(1) for w in warnings), ['gone/2.jpg', 'nope.png'])

    def test_line_numbers_point_at_the_call(self):
        rep = findings(write_pack(self.tmp, extra={'main.js': '// one\nvar a = 1;\n\nfetch("x");\n'}))
        self.assertEqual([w for level, w, _ in rep.items if level == 'ERROR'], [f'{pg.display(self.tmp / "demo-pack")}/main.js:4'])

    def test_js_scan_keeps_the_shape_of_the_text(self):
        text = "var a = 'x\\'y'; // c\nvar b = `t ${ {k: 'v'}.k } u`; /* d\ne */ var r = /[/]/g;\n"
        code, strings, comments = pg.js_scan(text)
        self.assertEqual(len(code), len(text))
        self.assertEqual(code.count('\n'), text.count('\n'))
        self.assertEqual([s[2] for s in strings], ["x\\'y", 't ', 'v', ' u'])
        self.assertEqual([text[a:b] for a, b in comments], ['// c', '/* d\ne */'])
        self.assertNotIn('[/]', code)
        self.assertIn('var r =', code)

    def test_a_pack_must_not_bring_terra_js(self):
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

    def test_the_bundled_sources_name_the_packs_branch(self):
        path = ROOT / 'Prototypes/_runtime/sources.json'
        rep = pg.Report()
        pg.validate_sources(path, rep)
        self.assertEqual(rep.items, [])
        urls = [s['url'] for s in json.loads(path.read_text())['sources']]
        self.assertIn('https://raw.githubusercontent.com/pbordjadze/terra-incognita/packs/catalog.json', urls)

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
        self.assertEqual((e['id'], e['revision'], e['runtime'], e['path'], e['retired']), ('demo-pack', 1, 3, 'packs/demo-pack/r1/', False))
        self.assertEqual(e['bytes'], sum(f['bytes'] for f in e['files']))
        self.assertEqual([f['path'] for f in e['files']], sorted(f['path'] for f in e['files']))
        self.assertEqual(sorted(f['path'] for f in e['files']), ['SOURCES.md', 'm.png', 'main.js', 'prototype.json'])

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
        (self.src / 'demo-pack' / 'main.js').write_text(MAIN_JS.replace("title: 'Demo'", "title: 'Changed'"))
        second = self.entry(self.pack())
        self.assertEqual(second['revision'], 2)
        self.assertEqual(second['path'], 'packs/demo-pack/r2/')
        self.assertNotEqual(second['hash'], first['hash'])
        self.assertNotEqual(second['published'], first['published'])
        self.assertIn("title: 'Changed'", (self.out / 'packs/demo-pack/r2/main.js').read_text())
        self.assertIn("title: 'Demo'", (self.out / 'packs/demo-pack/r1/main.js').read_text())   # the old revision stays
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

    def test_the_scaffold_is_a_clean_runtime_3_pack(self):
        code, out, err = run('new', 'tide-pool')
        self.assertEqual(code, 0, err + out)
        folder = self.tmp / 'tide-pool'
        self.assertEqual(findings(folder).items, [], findings(folder).lines())
        self.assertEqual(sorted(tree(folder)), ['SOURCES.md', 'doorway-glint.png', 'main.js', 'marker.png', 'prototype.json'])
        manifest = json.loads((folder / 'prototype.json').read_text())
        self.assertEqual((manifest['id'], manifest['title'], manifest['runtime'], manifest['presentation'], manifest['scripts']),
                         ('tide-pool', 'Tide pool', 4, 'globe', ['main.js']))
        self.assertEqual(manifest['doorway']['label'], 'Tide pool')
        self.assertTrue((folder / 'marker.png').read_bytes().startswith(b'\x89PNG'))
        main = (folder / 'main.js').read_text()
        for needle in ('terra.caption.set', 'terra.sprites.set', "terra.on('touch'", 'terra.panel.show', 'label:'):
            self.assertIn(needle, main)
        self.assertIn('# Tide pool', (folder / 'SOURCES.md').read_text())
        for text in (main, (folder / 'prototype.json').read_text(), (folder / 'SOURCES.md').read_text()):
            self.assertNotIn('{{', text)

    def test_globe_and_screen_are_accepted_and_both_make_a_globe_pack(self):
        code, out, err = run('new', 'on-globe', '--globe')
        self.assertEqual(code, 0, err)
        self.assertNotIn('note:', out)
        code, out, err = run('new', 'on-screen', '--screen')
        self.assertEqual(code, 0, err)
        self.assertIn('--screen makes a globe pack too', out)
        for pid in ('on-globe', 'on-screen'):
            self.assertEqual(json.loads((self.tmp / pid / 'prototype.json').read_text())['presentation'], 'globe')
            self.assertEqual(findings(self.tmp / pid).items, [])

    def test_title_is_escaped_for_json_and_scripts(self):
        title = 'The "odd" <one> & co\\'
        code, _, err = run('new', 'odd-title', '--title', title)
        self.assertEqual(code, 0, err)
        folder = self.tmp / 'odd-title'
        self.assertEqual(json.loads((folder / 'prototype.json').read_text())['title'], title)
        main = (folder / 'main.js').read_text()
        self.assertIn(json.dumps(title), main)
        self.assertEqual(findings(folder).items, [])
        if shutil.which('node'):
            done = subprocess.run(['node', '--check', str(folder / 'main.js')], capture_output=True, text=True)
            self.assertEqual(done.returncode, 0, done.stderr)

    def test_refusals(self):
        self.assertEqual(run('new', 'one-pack')[0], 0)
        code, _, err = run('new', 'one-pack')
        self.assertEqual(code, 2)
        self.assertIn('already exists', err)
        for bad in ('Bad', 'x', 'no-', 'a_b', 'x' * 41, '../up'):
            with self.subTest(id=bad):
                code, _, err = run('new', bad)
                self.assertEqual(code, 2)
                self.assertIn('not a valid id', err)
        self.assertEqual(run('new', 'long-title', '--title', 'x' * 61)[0], 2)
        self.assertEqual(sorted(p.name for p in self.tmp.iterdir()), ['one-pack'])   # nothing half-made

    def test_the_template_has_no_pages(self):
        files = [p.relative_to(HERE / 'template').as_posix() for p in (HERE / 'template').rglob('*') if p.is_file()]
        self.assertEqual(sorted(files), ['globe/SOURCES.md', 'globe/doorway-glint.png', 'globe/main.js', 'globe/marker.png', 'globe/prototype.json'])
        self.assertFalse([f for f in files if f.endswith(pg.PAGE_EXTS)])


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
        self.assertEqual(self.bucket.objects['packs/demo-pack/r1/main.js'][1], 'text/javascript; charset=utf-8')
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
        (self.src / 'demo-pack' / 'main.js').write_text(MAIN_JS + '// new\n')
        summary = self.tmp / 'summary.md'
        code, out, err = self.publish('--summary', str(summary))
        self.assertEqual(code, 0, err)
        new = self.bucket.puts[n:]
        self.assertEqual(new[-1], 'catalog.json')
        self.assertTrue(all(k.startswith(('packs/demo-pack/r2/', 'open/demo-pack.html')) or k == 'catalog.json' for k in new), new)
        self.assertEqual(len(new), 4 + 1 + 1)   # four files, its open page, the catalog
        self.assertEqual(json.loads(self.bucket.objects['catalog.json'][0])['packs'][0]['revision'], 2)
        self.assertIn('packs/demo-pack/r1/main.js', self.bucket.objects)             # the old revision stays
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


class BranchPublishing(TempCase):
    """publish --into: the packs branch's checkout, revised in place; the caller commits."""

    def setUp(self):
        super().setUp()
        self.src = self.tmp / 'protos'
        write_pack(self.src)
        write_pack(self.src, 'second-pack')
        self.into = self.tmp / 'packs-branch'
        self.into.mkdir()
        self.message = self.tmp / 'message.txt'
        self.clock = iter(f'2026-10-28T09:00:{n:02d}-04:00' for n in range(60))
        for patcher in (mock.patch.object(pg, 'PROTOTYPES', self.src), mock.patch.object(pg, 'now_eastern', lambda: next(self.clock))):
            patcher.start()
            self.addCleanup(patcher.stop)

    def publish(self, *argv, code=0):
        if self.message.exists():
            self.message.unlink()
        # no environment at all: the branch publish needs no credentials
        result, out, err = run('publish', '--into', str(self.into), '--commit-message', str(self.message), *argv, env={})
        self.assertEqual(result, code, out + err)
        return out + err

    def catalog(self):
        return json.loads((self.into / 'catalog.json').read_text())

    def entry(self, pid='demo-pack'):
        return next(e for e in self.catalog()['packs'] if e['id'] == pid)

    def test_a_fresh_branch(self):
        out = self.publish()
        catalog = self.catalog()
        self.assertEqual(list(catalog), ['format', 'generated', 'packs'])
        self.assertEqual(catalog['format'], 1)
        self.assertEqual([(e['id'], e['revision'], e['path'], e['runtime'], e['presentation'], e['retired']) for e in catalog['packs']],
                         [('demo-pack', 1, 'packs/demo-pack/r1/', 3, 'globe', False),
                          ('second-pack', 1, 'packs/second-pack/r1/', 3, 'globe', False)])
        for e in catalog['packs']:
            self.assertEqual(list(e), ['id', 'revision', 'runtime', 'title', 'summary', 'presentation', 'shelf', 'hash', 'path',
                                       'bytes', 'files', 'published', 'retired'])
            self.assertEqual(sorted(f['path'] for f in e['files']), ['SOURCES.md', 'm.png', 'main.js', 'prototype.json'])
            for f in e['files']:   # paths relative to the catalog
                data = (self.into / e['path'] / f['path']).read_bytes()
                self.assertEqual((len(data), hashlib.sha256(data).hexdigest()), (f['bytes'], f['sha256']))
        self.assertEqual(sorted(p.name for p in self.into.iterdir()), ['.gitattributes', 'catalog.json', 'packs'])
        self.assertIn('* -text', (self.into / '.gitattributes').read_text())   # no line ending is ever converted
        self.assertEqual(self.message.read_text(), 'Publish packs: demo-pack r1, second-pack r1\n')
        self.assertIn('new      demo-pack  r1', out)

    def test_an_unchanged_pack_keeps_its_revision_and_nothing_is_written(self):
        self.publish()
        before = tree(self.into)
        out = self.publish()
        self.assertIn('Nothing to publish', out)
        self.assertFalse(self.message.exists())
        self.assertEqual(tree(self.into), before)

    def test_a_changed_pack_gets_the_next_revision_and_the_old_one_stays(self):
        self.publish()
        first = self.catalog()
        old_files = {k: v for k, v in tree(self.into).items() if k.startswith('packs/demo-pack/r1/')}
        (self.src / 'demo-pack' / 'main.js').write_text(MAIN_JS.replace('A demonstration.', 'Changed.'))
        self.publish()
        e = self.entry()
        self.assertEqual((e['revision'], e['path']), (2, 'packs/demo-pack/r2/'))
        self.assertIn('Changed.', (self.into / 'packs/demo-pack/r2/main.js').read_text())
        self.assertEqual({k: v for k, v in tree(self.into).items() if k.startswith('packs/demo-pack/r1/')}, old_files)
        self.assertEqual(self.entry('second-pack'), next(x for x in first['packs'] if x['id'] == 'second-pack'))
        self.assertNotEqual(self.catalog()['generated'], first['generated'])
        self.assertEqual(self.message.read_text(), 'Publish packs: demo-pack r2\n')
        (self.src / 'demo-pack' / 'extra.json').write_text('{}')
        self.publish()
        self.assertEqual(self.entry()['revision'], 3)

    def test_only_the_named_packs_and_other_entries_kept(self):
        self.publish('second-pack')
        self.assertEqual([e['id'] for e in self.catalog()['packs']], ['second-pack'])
        self.publish('demo-pack')
        self.assertEqual([(e['id'], e['revision']) for e in self.catalog()['packs']], [('demo-pack', 1), ('second-pack', 1)])

    def test_retired_stays_retired(self):
        self.publish()
        code, _, err = run('retire', 'demo-pack', '--into', str(self.into))
        self.assertEqual(code, 0, err)
        self.assertTrue(self.entry()['retired'])
        self.publish()
        self.assertEqual((self.entry()['revision'], self.entry()['retired']), (1, True))
        (self.src / 'demo-pack' / 'main.js').write_text(MAIN_JS + '\n')
        self.publish()
        self.assertEqual((self.entry()['revision'], self.entry()['retired']), (2, True))
        self.assertFalse(self.entry('second-pack')['retired'])
        self.assertEqual(run('retire', 'demo-pack', '--out', str(self.into), '--undo')[0], 0)
        self.assertFalse(self.entry()['retired'])

    def test_revisions_only_go_up_even_when_the_catalog_forgets(self):
        self.publish()
        r1 = tree(self.into / 'packs/demo-pack/r1')
        (self.into / 'catalog.json').unlink()
        self.publish()
        self.assertEqual(self.entry()['revision'], 2)          # above the r1 already stored, never over it
        self.assertEqual(tree(self.into / 'packs/demo-pack/r1'), r1)
        shutil.copytree(self.into / 'packs/demo-pack/r2', self.into / 'packs/demo-pack/r7')   # a stray revision
        (self.src / 'demo-pack' / 'x.json').write_text('{}')
        self.publish()
        self.assertEqual(self.entry()['revision'], 8)

    def test_missing_files_are_restored_under_the_same_revision(self):
        self.publish()
        catalog = (self.into / 'catalog.json').read_bytes()
        shutil.rmtree(self.into / 'packs/second-pack/r1')
        out = self.publish()
        self.assertIn('restored packs/second-pack/r1/', out)
        self.assertTrue((self.into / 'packs/second-pack/r1/main.js').is_file())
        self.assertEqual((self.into / 'catalog.json').read_bytes(), catalog)
        self.assertEqual(self.message.read_text(), 'Publish packs: second-pack r1 restored\n')

    def test_dry_run_writes_nothing(self):
        out = self.publish('--dry-run')
        self.assertIn('Dry run', out)
        self.assertEqual(list(self.into.iterdir()), [])
        self.assertFalse(self.message.exists())

    def test_summary(self):
        summary = self.tmp / 'summary.md'
        self.publish('--public-url', 'https://raw.githubusercontent.com/owner/repo/packs/', '--summary', str(summary))
        text = summary.read_text()
        self.assertIn('### Packs published', text)
        self.assertIn('`demo-pack` revision 1', text)
        self.assertIn('terraincognita://open/demo-pack', text)
        self.assertIn('Catalog: https://raw.githubusercontent.com/owner/repo/packs/catalog.json', text)
        self.publish('--summary', str(summary))
        self.assertIn('Nothing to publish', summary.read_text())

    def test_refusals(self):
        code, _, err = run('publish', '--into', str(self.tmp / 'nowhere'), env={})
        self.assertEqual(code, 2)
        self.assertIn('no such folder', err)
        self.assertEqual(run('publish', '--into', str(self.into), '--public-url', 'http://example.com/', env={})[0], 2)
        code, _, err = run('publish', '--commit-message', str(self.message), env={})
        self.assertEqual(code, 2)
        self.assertIn('go with --into', err)
        (self.src / 'second-pack' / 'prototype.json').write_text('{}')
        code, out, err = run('publish', '--into', str(self.into), env={})
        self.assertEqual(code, 2)
        self.assertIn('nothing was packed', err)
        self.assertEqual(list(self.into.iterdir()), [])

    def test_a_git_checkout_keeps_every_byte(self):
        if not shutil.which('git'):
            self.skipTest('git unavailable')
        crlf = b'line one\r\nline two\r\n'
        (self.src / 'demo-pack' / 'notes.txt').write_bytes(crlf)

        def git(*args):
            done = subprocess.run(['git', '-C', str(self.into), *args], capture_output=True, text=True,
                                  env=dict(os.environ, GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM='1'))
            self.assertEqual(done.returncode, 0, done.stderr)
            return done.stdout
        git('init', '-q', '-b', 'packs')
        git('config', 'core.autocrlf', 'true')                   # the worst case: a checkout that converts
        self.publish()
        git('add', '-A')
        git('-c', 'user.name=t', '-c', 'user.email=t@example.com', 'commit', '-q', '-m', self.message.read_text())
        blob = subprocess.run(['git', '-C', str(self.into), 'show', 'HEAD:packs/demo-pack/r1/notes.txt'], capture_output=True).stdout
        self.assertEqual(blob, crlf)
        self.assertEqual(git('log', '--format=%s'), 'Publish packs: demo-pack r1, second-pack r1\n')


class RemotePacksScript(TempCase):
    def test_the_remote_copy_differs_from_the_bundled_one(self):
        pack = ROOT / 'Prototypes/hello-pack'
        if json.loads((pack / 'prototype.json').read_text()).get('runtime') != pg.RUNTIME:
            self.skipTest('hello-pack is not a runtime-3 pack yet')
        out = self.tmp / 'remote'
        done = subprocess.run(['bash', str(ROOT / 'ci/remote_packs.sh'), str(out)], capture_output=True, text=True, cwd=self.tmp)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        catalog = json.loads((out / 'catalog.json').read_text())
        entry = next(e for e in catalog['packs'] if e['id'] == 'hello-pack')
        self.assertEqual(entry['revision'], 1)
        bundled = pg.Pack(pack, *pg.validate_pack(pack, pg.Report()))
        self.assertNotEqual(entry['hash'], bundled.hash)
        self.assertIn('hello-pack', done.stdout)                                      # the catalog is printed


if __name__ == '__main__':
    unittest.main()

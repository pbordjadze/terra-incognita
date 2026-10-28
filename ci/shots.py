"""A CI report's screenshots by name (ci/fetch.sh unpacks the report).

    python3 ci/shots.py <report> [name ...]

With no names, lists every shot: device, where it came from (a UI test, or the simulator's own
shots: the stage's places, the globe's styles, the film), its name and its file. With names, prints
the file of each shot whose name holds one of them (any case), and fails if a name matches none.
The UI tests' attachments are named by their manifest (`56-old-maps-early_0_<uuid>.png` is
`56-old-maps-early`); the simulator's shots by their file (`sim-stage-descent.png` is `stage-descent`).
"""
import json, re, signal, sys
from pathlib import Path


def shots(report):
    report = Path(report)
    for manifest in sorted(report.rglob('manifest.json')):
        device = manifest.parent.name
        for test in json.loads(manifest.read_text()):
            for a in test.get('attachments', []):
                name = re.sub(r'_\d+_[0-9A-Fa-f-]{36}\.\w+$', '', a.get('suggestedHumanReadableName', ''))
                yield device, test.get('testIdentifier', '?'), name, manifest.parent / a['exportedFileName']
    for png in sorted(report.rglob('sim-*.png')):
        yield png.parent.name, 'simulator', png.stem[len('sim-'):], png


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    found = list(shots(argv[0]))
    names = argv[1:]
    if not names:
        for device, source, name, path in found:
            print(f'{device:7} {source:60} {name:32} {path}')
        return 0
    missing = 0
    for want in names:
        hits = [path for _, _, name, path in found if want.lower() in name.lower()]
        for path in hits:
            print(path)
        if not hits:
            print(f'no shot named like {want!r} in {argv[0]}', file=sys.stderr)
            missing += 1
    return 1 if missing else 0


if __name__ == '__main__':
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)          # (into head, say)
    sys.exit(main(sys.argv[1:]))

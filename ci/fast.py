"""What a branch's run checks, when it is asked for less than everything: from the pushed commit's
message, or the inputs of a run started by hand. Prints GitHub step outputs (fast, tests, places).

    [shots]                          the stage at the default places, on the iPad: no Release build,
                                     no UI tests (about 6 minutes on a free runner)
    [shots earth:15,8,1 12,-20,3.4]  those places (a name is optional)
    [test OceanEncounterTests]       only those UI test classes (or Class/testMethod), on the iPad

Main always runs everything, whatever its head commit's message says: a branch's tag that comes to
main with a fast-forward merge must not narrow the release run. `[build-only]` and
`[playground-check]` are read by ci.yml itself.
"""
import os, re, sys

# The stage at rest at each (the ground's month, the clouds' drift and the sea's motion held): what
# tools/metal/metal.py's `shot` draws at the same names, so the two can be set side by side.
PLACES = 'earth:15,8,1 atlantic:-35,35,1 descent:12,-20,3.4 ground:14,-22,5.5 aegean:25,38,4 close:15,-23,8'
PLACE = re.compile(r'^(?:([\w-]+):)?(-?[\d.]+),(-?[\d.]+),([\d.]+)$')
TEST = re.compile(r'^[A-Za-z_]\w*(/test\w*)?$')


def parse(message, branch, tests='', places=''):
    """The outputs, or an error naming what couldn't be read."""
    out = {'fast': '', 'tests': '', 'places': ''}
    if branch == 'main':
        return out
    asked_tests, asked_places, shots = tests.split(), places.split(), bool(places.strip())
    for m in re.finditer(r'\[(shots|test)(?:\s+([^\]]*))?\]', message or ''):
        words = (m.group(2) or '').split()
        if m.group(1) == 'shots':
            shots = True
            asked_places += words
        else:
            if not words:
                raise ValueError('[test] names no test class: [test OceanEncounterTests]')
            asked_tests += words
    for t in asked_tests:
        if not TEST.match(t):
            raise ValueError(f'not a test class or Class/testMethod: {t}')
    named = []
    for i, p in enumerate(asked_places):
        m = PLACE.match(p)
        if not m:
            raise ValueError(f'not a place (name:lon,lat,zoom or lon,lat,zoom): {p}')
        named.append(f'{m.group(1) or f"place{i + 1}"}:{m.group(2)},{m.group(3)},{m.group(4)}')
    if asked_tests:
        out['fast'] = 'tests'
        out['tests'] = ' '.join(f'TerraIncognitaUITests/{t}' for t in asked_tests)
    if shots:
        out['fast'] = 'shots' if not asked_tests else 'tests'
        out['places'] = ' '.join(named) or PLACES
    return out


def main():
    try:
        out = parse(os.environ.get('MESSAGE', ''), os.environ.get('BRANCH', ''),
                    os.environ.get('TESTS', ''), os.environ.get('PLACES', ''))
    except ValueError as e:
        print(f'::error::{e}', file=sys.stderr)
        return 1
    for k, v in out.items():
        print(f'{k}={v}')
    return 0


if __name__ == '__main__':
    sys.exit(main())

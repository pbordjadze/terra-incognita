"""The hand-drawn creatures, as ASCII art, written to Web/sprites.json for cell.glsl to stamp.

    python3 tools/sprites.py [--show]

Each creature is one or more frames of text (animated in turn) and colouring rules: a base
colour, colours by character and by row, and boxes (x0, y0, x1, y1, colour) for special parts.
Spaces enclosed by the drawing (found by flooding from the edges) become an opaque blank, so the
sea doesn't show through a whale's body; other spaces are see-through. `anchor` is the cell that
sits on the creature's place on the map. Mirror images are made here: draw each facing one way.
"""
import json, sys
from pathlib import Path

# colour letter -> palette entry (render.js PAL_KEYS)
COLOURS = {'g': 'VERDHI', 'G': 'VERD', 'r': 'VERMHI', 'R': 'VERM', 'y': 'GOLDHI', 'Y': 'GOLD', 'w': 'SPRAY', 'b': 'SEA',
           'B': 'SEAHI', 'c': 'WHALEHI', 'C': 'WHALE', 'h': 'HULL', 's': 'SAIL', 'f': 'FIRE', 'v': 'VELLUMHI', 'k': 'CLOUD'}
WAVES = {'~': 'b'}

ART = {
    'serpent': dict(anchor=(14, 6), base='g', chars={**WAVES, 'o': 'y', 'v': 'r', 'V': 'r'}, rows={0: 'r'},
        frames=[r'''
      /\/\/\        /\/\/\/\           /\/\/\/\
   _,-'  o  `-.   ,-""""""-.         ,-""""""-.
  <__,-.____   \ /  ,----.  `.      /  ,----.  `.      _
   `vVvVv'  `.  /  /      \   \    /  /      \   \   ,' )
             \    /        \   \  /  /        \   \_/ ,'
~~~~~~~~~~~~~~\~~/~~~~~~~~~~\~~~\/~~/~~~~~~~~~~\~~~~~/~~~~
    ~~   ~~~      ~~~   ~~         ~~~    ~~~      ~~
''', r'''
      /\/\/\        /\/\/\/\           /\/\/\/\
   _,-'  o  `-.   ,-""""""-.         ,-""""""-.
  <__,-.____   \ /  ,----.  `.      /  ,----.  `.      _
   `vVvVv'  `.  /  /      \   \    /  /      \   \   ,' )
             \    /        \   \  /  /        \   \_/ ,'
~~~~~~~~~~~~~~\~~/~~~~~~~~~~\~~~\/~~/~~~~~~~~~~\~~~~~/~~~~
  ~~~   ~~       ~~   ~~~        ~~~   ~~~       ~~~
''']),
    'dragon': dict(anchor=(16, 9), base='r', chars={**WAVES, 'o': 'y', 'v': 'y', 'V': 'y'},
        boxes=[(7, 0, 40, 4, 'Y'), (17, 4, 40, 6, 'Y')],
        frames=[r'''
                    __
            _____  /  \______
          ,'  ,-.`/  /       `-._
         /  ,'  /  ,'      __    `.
   /\/\  | /   /  /     ,-'  `-.   \
 _/ o  `.|/   /  /    ,'        \   |
<_,--.  \/   /  /   ,'           |  |   __
 `vVv'\  \  (  (   /    ,-.      |  |  /  )
       \  `--`--`-'    /   \    /  /  /  /
~~~~~~~~\~~~~~~~~~~~~~/~~~~~\~~/~~/~~/~~/~~~~
   ~~~    ~~~   ~~       ~~~       ~~~
''', r'''
                    __
            _____  /  \______
          ,'  ,-.`/  /       `-._
         /  ,'  /  ,'      __    `.
   /\/\  | /   /  /     ,-'  `-.   \
 _/ o  `.|/   /  /    ,'        \   |
<_,--.  \/   /  /   ,'           |  |   __
 `vVv'\  \  (  (   /    ,-.      |  |  /  )
       \  `--`--`-'    /   \    /  /  /  /
~~~~~~~~\~~~~~~~~~~~~~/~~~~~\~~/~~/~~/~~/~~~~
 ~~~    ~~~   ~~~       ~~       ~~~   ~~
''']),
    'whale': dict(anchor=(14, 6), base='c', chars={**WAVES, 'o': 'y'},
        boxes=[(19, 0, 29, 3, 'w'), (21, 3, 23, 4, 'w')],
        frames=[r'''
                      .  .
                    .  ::  .
           _______    :
      _.-''       ''-.:_
    ,'  o              `-.     _
   (                      `-._/ )
~~~~`--.____        ____.---~~~\~~~
  ~~~       `""""""'     ~~~    ~~
''', r'''
                     .    .
                       ::
           _______    :
      _.-''       ''-.:_
    ,'  o              `-.     _
   (                      `-._/ )
~~~~`--.____        ____.---~~~\~~~
 ~~~        `""""""'    ~~~     ~~
''', r'''


           _______
      _.-''       ''-._
    ,'  o              `-.     _
   (                      `-._/ )
~~~~`--.____        ____.---~~~\~~~
  ~~~       `""""""'     ~~~    ~~
''']),
    'brendan': dict(anchor=(15, 7), base='c', chars={**WAVES, 'o': 'y'},
        boxes=[(14, 0, 26, 3, 'k'), (15, 3, 23, 4, 'f'), (14, 4, 24, 5, 'f'), (11, 4, 14, 5, 'h'), (24, 4, 27, 5, 'h'), (14, 5, 24, 6, 'h')],
        frames=[r'''
                 (  )
                  )(   (
                 (  )   )
               __ )( __
           ___/ +^^^^+ \___
      _.-''    `------'    ''-._
    ,'  o                       `-.    _
   (                               `-./ )
~~~~`--.____             ____.---~~~~\~~~
  ~~~       `"""""""""""'      ~~~     ~~
''', r'''
                  ( )
                 (  )  (
                  )(    )
               __(  )__
           ___/ +^^^^+ \___
      _.-''    `------'    ''-._
    ,'  o                       `-.    _
   (                               `-./ )
~~~~`--.____             ____.---~~~~\~~~
 ~~~        `"""""""""""'     ~~~      ~~
''']),
    'ship': dict(anchor=(11, 10), base='h', chars={**WAVES, 'o': 'y', '+': 'r', '>': 'r', '~': 'b'},
        boxes=[(4, 2, 18, 7, 's'), (20, 2, 25, 7, 's'), (11, 0, 15, 1, 'r'), (21, 2, 23, 3, 'r')],
        frames=[r'''
          |>~~
          |
     .----+----.     |>
    /   \ | /   \    |
   |  -- +|+ --  |   |\
   |    / | \    |   | \
    \.-'  |  `-./    |__\
       ___|_________/__
  \``-'  o   o   o   o  /
   \___________________/
~~~~~~\______________/~~~~~
  ~~~    ~~~     ~~~    ~~
''', r'''
          |~~>
          |
     .----+----.     |~>
    /   \ | /   \    |
   |  -- +|+ --  |   |\
   |    / | \    |   | \
    \.-'  |  `-./    |__\
       ___|_________/__
  \``-'  o   o   o   o  /
   \___________________/
~~~~~~\______________/~~~~~
 ~~~     ~~~    ~~~     ~~
''']),
}

MIRROR = str.maketrans("/\\()<>{}[]`'", "\\/)(><}{][\'`")


def lines(text, w=None):
    ls = text.strip('\n').split('\n')
    w = w or max(len(l) for l in ls)
    return [l.ljust(w) for l in ls]


def interior(g):
    """Spaces not reachable from the edge (4-connected, through spaces) are inside the drawing."""
    h, w = len(g), len(g[0])
    seen = [[False] * w for _ in range(h)]
    stack = [(x, y) for y in range(h) for x in (0, w - 1)] + [(x, y) for x in range(w) for y in (0, h - 1)]
    while stack:
        x, y = stack.pop()
        if not (0 <= x < w and 0 <= y < h) or seen[y][x] or g[y][x] != ' ':
            continue
        seen[y][x] = True
        stack += [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
    return [[g[y][x] == ' ' and not seen[y][x] for x in range(w)] for y in range(h)]


def colour(spec, g):
    inside = interior(g)
    out = []
    for y, row in enumerate(g):
        m = []
        for x, ch in enumerate(row):
            if ch == ' ':
                m.append(':' if inside[y][x] else ' ')
                continue
            c = spec['chars'].get(ch) if ch in spec['chars'] and ch in '~' else None
            for (x0, y0, x1, y1, cc) in spec.get('boxes', []):
                if x0 <= x < x1 and y0 <= y < y1:
                    c = cc
            if c is None:
                c = spec['chars'].get(ch) or spec.get('rows', {}).get(y) or spec['base']
            m.append(c)
        out.append(''.join(m))
    return out


def main():
    data = {'colours': COLOURS, 'art': {}}
    for name, spec in ART.items():
        w = max(len(l) for f in spec['frames'] for l in f.strip('\n').split('\n'))
        frs = [lines(f, w) for f in spec['frames']]
        h = max(len(f) for f in frs)
        frs = [f + [' ' * w] * (h - len(f)) for f in frs]
        ax, ay = spec['anchor']
        fr = [{'g': g, 'm': colour(spec, g)} for g in frs]
        data['art'][name] = {'w': w, 'h': h, 'anchor': [ax, ay], 'frames': fr}
        data['art'][name + '_flip'] = {'w': w, 'h': h, 'anchor': [w - 1 - ax, ay],
            'frames': [{'g': [l[::-1].translate(MIRROR) for l in f['g']], 'm': [l[::-1] for l in f['m']]} for f in fr]}
        if '--show' in sys.argv:
            print(f'--- {name} ({w}x{h}, {len(fr)} frames)')
            for a, b in zip(fr[0]['g'], fr[0]['m']): print(a, '|', b)
    out = Path(__file__).resolve().parent.parent / 'Web' / 'sprites.json'
    out.write_text(json.dumps(data, separators=(',', ':')))
    print('wrote', out, out.stat().st_size)


if __name__ == '__main__':
    main()

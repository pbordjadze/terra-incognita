// The sampler's charts: two marking alphabets (the map's names, cross-stitched over one thread) and the
// motifs of the sampler tradition, counted on the plate's grid. In a motif each letter is a thread
// (the legend in levels.js); '.' is bare linen.
'use strict';
const Charts = (() => {
  // A seven-row marking alphabet, as girls learnt it on their first sampler.
  const A7 = {
    A: '..#..|.#.#.|#...#|#...#|#####|#...#|#...#', B: '####.|#...#|#...#|####.|#...#|#...#|####.',
    C: '.###.|#...#|#....|#....|#....|#...#|.###.', D: '###..|#..#.|#...#|#...#|#...#|#..#.|###..',
    E: '#####|#....|#....|####.|#....|#....|#####', F: '#####|#....|#....|####.|#....|#....|#....',
    G: '.###.|#...#|#....|#.###|#...#|#...#|.####', H: '#...#|#...#|#...#|#####|#...#|#...#|#...#',
    I: '###|.#.|.#.|.#.|.#.|.#.|###', J: '..###|...#.|...#.|...#.|...#.|#..#.|.##..',
    K: '#...#|#..#.|#.#..|##...|#.#..|#..#.|#...#', L: '#....|#....|#....|#....|#....|#....|#####',
    M: '#...#|##.##|#.#.#|#.#.#|#...#|#...#|#...#', N: '#...#|##..#|#.#.#|#.#.#|#..##|#...#|#...#',
    O: '.###.|#...#|#...#|#...#|#...#|#...#|.###.', P: '####.|#...#|#...#|####.|#....|#....|#....',
    Q: '.###.|#...#|#...#|#...#|#.#.#|#..#.|.##.#', R: '####.|#...#|#...#|####.|#.#..|#..#.|#...#',
    S: '.###.|#...#|#....|.###.|....#|#...#|.###.', T: '#####|..#..|..#..|..#..|..#..|..#..|..#..',
    U: '#...#|#...#|#...#|#...#|#...#|#...#|.###.', V: '#...#|#...#|#...#|#...#|#...#|.#.#.|..#..',
    W: '#...#|#...#|#...#|#.#.#|#.#.#|##.##|#...#', X: '#...#|#...#|.#.#.|..#..|.#.#.|#...#|#...#',
    Y: '#...#|#...#|.#.#.|..#..|..#..|..#..|..#..', Z: '#####|....#|...#.|..#..|.#...|#....|#####',
    ' ': '..|..|..|..|..|..|..', '.': '.|.|.|.|.|.|#', "'": '#|#|.|.|.|.|.', '-': '...|...|...|###|...|...|...',
  };
  // A five-row alphabet for the smaller names.
  const A5 = {
    A: '.#.|#.#|###|#.#|#.#', B: '##.|#.#|##.|#.#|##.', C: '.##|#..|#..|#..|.##', D: '##.|#.#|#.#|#.#|##.',
    E: '###|#..|##.|#..|###', F: '###|#..|##.|#..|#..', G: '.##|#..|#.#|#.#|.##', H: '#.#|#.#|###|#.#|#.#',
    I: '###|.#.|.#.|.#.|###', J: '..#|..#|..#|#.#|.#.', K: '#.#|#.#|##.|#.#|#.#', L: '#..|#..|#..|#..|###',
    M: '#...#|##.##|#.#.#|#...#|#...#', N: '#..#|##.#|#.##|#..#|#..#', O: '.#.|#.#|#.#|#.#|.#.',
    P: '##.|#.#|##.|#..|#..', Q: '.#.|#.#|#.#|#.#|.##', R: '##.|#.#|##.|#.#|#.#', S: '.##|#..|.#.|..#|##.',
    T: '###|.#.|.#.|.#.|.#.', U: '#.#|#.#|#.#|#.#|.#.', V: '#.#|#.#|#.#|#.#|.#.', W: '#...#|#...#|#.#.#|##.##|#...#',
    X: '#.#|#.#|.#.|#.#|#.#', Y: '#.#|#.#|.#.|.#.|.#.', Z: '###|..#|.#.|#..|###',
    ' ': '.|.|.|.|.', '.': '.|.|.|.|#', "'": '#|#|.|.|.', '-': '..|..|##|..|..',
  };
  const FONTS = { 7: A7, 5: A5 };

  // the cells of a line of text: [[i, j], ...] from its top left, and its width in cells
  function letters(text, size = 7, gap = 1) {
    const font = FONTS[size], cells = [];
    let x = 0;
    for (const ch of text.toUpperCase()) {
      const g = font[ch] || font[' '];
      const rows = g.split('|');
      rows.forEach((row, j) => { for (let i = 0; i < row.length; i++) if (row[i] === '#') cells.push([x + i, j]); });
      x += rows[0].length + gap;
    }
    return { cells, w: x - gap, h: size };
  }

  const M = {};
  // a strawberry, its calyx up
  M.berry = `
.g...g.
..ggg..
.grgrg.
.rrrrr.
rrprrrr
rrrrrpr
.prrrr.
..rrpr.
...rr..`;
  M.leaf = `
...gg
..ggg
.gGgg
gGgg.
Gg...`;
  M.tulip = `
r.r.r
rrrrr
rprrr
.rrr.
g.G.g
.gGg.
..G..`;
  // a bird on the sampler's bough, facing right
  M.bird = `
.........bb...
........bbbbo.
........bkbb..
.BB....bbbbb..
BBBB.bbbbbbb..
.BBBbbbbbbbb..
..BBbbbBBbbb..
...bbbBBBbb...
....bbbbbb....
......o..o....`;
  // the tree between them
  M.tree = `
....g....
...ggg...
..gglgg..
.ggrgrgg.
..ggggg..
.gglgggg.
ggrggrggg
.ggggggg.
...gng...
....n....
....n....
..nnnnn..`;
  // a stitched ship, sails set
  M.ship = `
..........r..........
..........rr.........
....r.....k.....r....
....rr...ymy....rr...
....k...yyyyy...k....
...ymy.yyyyyyy.ymy...
..yyyy.mmmmmmm.yyyy..
..mmmm.yyyyyyy.mmmm..
..yyyy.yyyyyyy.yyyy..
...yy..mmmmmmm..yy...
....k...yyyyy...k....
....k.....k.....k....
nnnnnnnnnnnnnnnnnnnnn
.nwnnwnnwnnwnnwnnwnn.
..rrrrrrrrrrrrrrrrr..
...nnnnnnnnnnnnnnn...`;
  // the sea serpent, three coils and a curled tail
  M.serpent = `
......r.r..........................
....gggggg.........................
...ggkgggggo.......................
..gggggggggoo......................
..ww......ggg......r.r.......r.r...
.r.wgggg..gggg....gggggg....gggggg.
r...ggg...gGGgg..ggGGGGgg..ggGGGGgg
..........gGyGg.ggGGyyGGgg.gGGyyGGg
..........ggyGg.gGGy..yGGg.gGy..yGg
bb.bb.bb.bggyGgbgGyb.bbyGgbgGybb.yG
.bb.bb.bb.bggGbbggb.bb.bggbggb.bb.b`;
  return { A7, A5, letters, M };
})();
window.Charts = Charts;

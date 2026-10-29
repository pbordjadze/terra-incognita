/* When the sea was low: the shelves (pictures built by tools/lowsea/build_shelves.py, their bounds
   west, south, east, north), the rivers and the lake that ran across them when they were dry (drawn
   in outline after the reconstructions in SOURCES.md), and the ways the mammoths walk. */
window.LOWSEA = {
  shelves: [
    { id: 'doggerland', bounds: [-12, 47, 12, 62] },
    { id: 'sunda', bounds: [95, -12, 122, 14] },
    { id: 'sahul', bounds: [112, -24, 152, 0] },
    { id: 'beringia-west', bounds: [155, 50, 180, 72] },
    { id: 'beringia-east', bounds: [-180, 50, -150, 72] },
    { id: 'gulf', bounds: [44, 21, 60, 31] },
    { id: 'adriatic-black', bounds: [11, 36, 42, 48] },
    { id: 'patagonia', bounds: [-72, -56, -52, -36] }
  ],
  // the rivers of the dry shelves, in outline: the Channel River gathering the Thames, the Rhine and
  // the Seine and running west to the Atlantic; the Elbe on north across the dry North Sea; the great
  // river of the Sunda shelf (the North Sunda or Molengraaff river) running north-east to the South
  // China Sea; the Gulf's river, the Tigris and Euphrates running on down the dry Gulf
  rivers: [
    { name: 'The Channel River', line: [[1.0, 51.4], [0.2, 50.9], [-1.2, 50.3], [-2.8, 49.9], [-4.4, 49.4], [-6.0, 48.9], [-7.4, 48.5], [-8.6, 48.0]] },
    { name: 'The Rhine, on to the Channel', line: [[4.1, 51.9], [3.0, 51.8], [1.9, 51.6], [1.0, 51.4]] },
    { name: 'The Seine, on to the Channel', line: [[0.1, 49.5], [-1.0, 49.9], [-2.0, 50.1], [-2.8, 49.9]] },
    { name: 'The Elbe, on across the dry North Sea', line: [[8.6, 53.9], [7.4, 54.6], [6.0, 55.6], [4.5, 56.6], [3.4, 57.6]] },
    { name: 'The great river of the Sunda shelf', line: [[106.0, -1.5], [107.4, 0.0], [108.4, 1.8], [109.0, 3.8], [109.6, 5.8], [110.4, 7.4]] },
    { name: 'The Tigris and Euphrates, down the dry Gulf', line: [[48.6, 29.9], [49.6, 28.8], [50.8, 27.6], [52.0, 26.6], [53.6, 25.9], [55.2, 26.0]] }
  ],
  // Lake Carpentaria, the lake in the Gulf of Carpentaria when the gulf was dry land
  lakes: [
    { name: 'Lake Carpentaria', ring: [[139.0, -14.5], [139.8, -14.2], [140.6, -13.6], [141.0, -12.8], [140.6, -12.0], [139.8, -11.8], [139.0, -12.2], [138.6, -13.0], [138.6, -13.9], [139.0, -14.5]] }
  ],
  // the mammoths' ways over the dry land, each a few of them in file
  herds: [
    { on: 'doggerland', path: [[1.0, 54.0], [2.6, 54.6], [4.4, 55.2], [5.8, 55.0]], seconds: 420, count: 3 },
    { on: 'doggerland', path: [[-0.2, 52.4], [1.4, 52.9], [2.8, 53.4]], seconds: 300, count: 2 },
    { on: 'beringia', path: [[172.0, 64.6], [176.0, 65.2], [-178.0, 65.4], [-172.0, 65.0], [-166.0, 64.4]], seconds: 600, count: 4 },
    { on: 'beringia', path: [[-170.0, 62.0], [-174.0, 61.4], [-178.0, 61.6]], seconds: 400, count: 2 }
  ]
};

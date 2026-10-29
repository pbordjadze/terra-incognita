'use strict';
// Draws the boto of A gathering of water, alive (Prototypes/gathering-of-water-alive): a pink river dolphin
// surfacing, as 24 frames the sprite runs through at 4 a second (six seconds: a breath, its back arching over,
// the ripples, then quiet water until the next). Drawn here from code as SVG and rendered by Chromium; no
// picture is traced from a photograph. Facing east, as a sprite's `face` expects. Frame 1 is its whole head up
// (what Reduce Motion holds). An SVG sheet for the one art drawer:
//   node tools/packart/render.cjs tools/water/boto.cjs Prototypes/gathering-of-water-alive
// Writes boto-01.png ... (the frames with something in them), boto-quiet.png (the quiet water) and boto.json
// (the frames in order, fps, size in points) into the pack. Never hand-edit those; change this and run it.
const fs = require('node:fs'), path = require('node:path');

const OUT = path.resolve(__dirname, '../../Prototypes/gathering-of-water-alive');
const SIZE = [30, 16];                 // points
const SCALE = 4;                       // pixels a point (3 on a phone, 1.2 reading scale on the large iPad)
const W = 60, H = 32, WATER = 20;      // the drawing's units: two a point; the water's surface

const INK = '#7E4553', PINK_TOP = '#C47D8E', PINK = '#E6A0AE', BELLY = '#F6CBD0';

// the boto's whole head up: back, the round melon, the long beak (it breathes through the top of its head)
const HEAD = 'M 21 20 C 22.5 15.4 28.5 12.6 34.6 12.6 C 38.8 12.6 41.8 13.7 43 15.6 C 45.2 16.4 50.2 17.5 55.2 18.5'
  + ' C 56 18.7 55.9 19.5 55 19.5 C 50.4 19.5 46.2 19.7 42 20 Z';
// its back arching over, with the low ridge it has instead of a fin
const BACK = 'M 11 20 C 15 15.6 23 13.1 30.5 13.2 C 33 13.25 34.6 12.6 37 13.3 C 41.5 14.5 45.5 17.2 48 20 Z';
const SINK = 'M 17 20 C 21 17.2 28.5 16.2 34.5 17.2 C 37.6 17.8 40.2 18.9 41.5 20 Z';
const STOCK = 'M 24 20 C 26 18.6 29.6 18.2 32.4 18.9 L 34.6 17.6 C 35 18.6 34.6 19.4 34 20 Z';

function body(d, dy, opacity) {
  return `<g transform="translate(0 ${dy})" opacity="${opacity}" clip-path="url(#above)">
    <path d="${d}" fill="url(#pink)" stroke="${INK}" stroke-width="0.7" stroke-linejoin="round"/>
  </g>`;
}
function eye(dy) { return `<circle cx="40.4" cy="${16.7 + dy}" r="0.62" fill="#3B2128"/>`; }
function patch(opacity) {
  return `<ellipse cx="33" cy="${WATER}" rx="25" ry="5" fill="url(#water)" opacity="${opacity}"/>`;
}
function ripples(list) {
  return list.map(([r, a]) => `<ellipse cx="32" cy="${WATER + 0.3}" rx="${r}" ry="${(r * 0.22).toFixed(2)}" fill="none"
    stroke="#FFFFFF" stroke-opacity="${a}" stroke-width="0.8"/>`).join('');
}
function line(x0, x1, a) {
  return `<path d="M ${x0} ${WATER} L ${x1} ${WATER}" stroke="#FFFFFF" stroke-opacity="${a}" stroke-width="0.9" stroke-linecap="round"/>`;
}
function blow(r, a) {
  // a breath of mist over the blowhole, rising and spreading
  return `<ellipse cx="33.4" cy="${(9.4 - (r - 1) * 2).toFixed(2)}" rx="${(2.2 * r).toFixed(2)}" ry="${(1.6 * r).toFixed(2)}"
    fill="#FFFFFF" fill-opacity="${a}" filter="url(#mist)"/>`;
}
function drops(a) {
  return [[44, 16.4, 0.5], [47, 15.6, 0.4], [40.5, 15.2, 0.45], [49.5, 17.4, 0.35]]
    .map(([x, y, r]) => `<circle cx="${x}" cy="${y}" r="${r}" fill="#FFFFFF" fill-opacity="${a}"/>`).join('');
}

// [what is drawn] per frame, in order; null is the quiet water
const FRAMES = [
  patch(1) + body(HEAD, 0, 1) + eye(0) + blow(1, 0.75) + line(19, 44, 0.9),
  patch(1) + body(HEAD, 0, 1) + eye(0) + blow(1.6, 0.5) + line(19, 44, 0.9),
  patch(1) + body(HEAD, 2.2, 1) + eye(2.2) + blow(2.1, 0.25) + line(20, 44, 0.85) + ripples([[22, 0.35]]),
  patch(1) + body(BACK, 1.6, 1) + line(12, 47, 0.85),
  patch(1) + body(BACK, 0, 1) + line(10, 49, 0.9),
  patch(0.9) + body(SINK, 0, 1) + line(16, 42, 0.85) + ripples([[24, 0.3]]),
  patch(0.8) + body(STOCK, 0, 1) + line(23, 35, 0.8) + ripples([[10, 0.7], [20, 0.3]]),
  patch(0.6) + ripples([[8, 0.85], [15, 0.55], [23, 0.25]]),
  patch(0.4) + ripples([[12, 0.6], [19, 0.35], [26, 0.15]]),
  patch(0.2) + ripples([[16, 0.35], [24, 0.15]]),
  null, null, null, null, null, null, null, null, null, null,
  patch(0.2) + ripples([[5, 0.4]]),
  patch(0.5) + ripples([[4, 0.7], [9, 0.35]]) + drops(0.5),
  patch(0.8) + body(HEAD, 4.6, 1) + eye(4.6) + line(30, 44, 0.9) + drops(0.85),
  patch(1) + body(HEAD, 1.8, 1) + eye(1.8) + line(21, 44, 0.9) + drops(0.4),
];

function svg(content) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${SIZE[0] * SCALE}" height="${SIZE[1] * SCALE}" viewBox="0 0 ${W} ${H}">
  <defs>
    <linearGradient id="pink" x1="0" y1="0.25" x2="0" y2="1">
      <stop offset="0" stop-color="${PINK_TOP}"/><stop offset="0.45" stop-color="${PINK}"/><stop offset="1" stop-color="${BELLY}"/>
    </linearGradient>
    <radialGradient id="water" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#3C7FAE" stop-opacity="0.55"/><stop offset="0.7" stop-color="#3C7FAE" stop-opacity="0.3"/>
      <stop offset="1" stop-color="#3C7FAE" stop-opacity="0"/>
    </radialGradient>
    <filter id="mist" x="-1" y="-1" width="3" height="3"><feGaussianBlur stdDeviation="0.9"/></filter>
    <clipPath id="above"><rect x="0" y="0" width="${W}" height="${WATER + 0.2}"/></clipPath>
  </defs>
  ${content || ''}
</svg>`;
}

/** The sheet for tools/packart/render.cjs: the frames in order (the quiet water is one file, shown
    wherever the boto is under), and `done` writes boto.json. */
function sheet({ out = OUT } = {}) {
  for (const f of fs.readdirSync(out)) if (/^boto-.*\.png$/.test(f)) fs.rmSync(path.join(out, f));
  const names = [], pictures = [];
  let n = 0;
  for (const content of FRAMES) {
    const name = content ? `boto-${String(++n).padStart(2, '0')}.png` : 'boto-quiet.png';
    names.push(name);
    pictures.push({ file: name, width: SIZE[0], height: SIZE[1], scale: SCALE, svg: svg(content) });
  }
  return {
    pictures,
    done() {
      const spec = { about: 'A boto, drawn by tools/water/boto.cjs: the frames in order (six seconds at 4 a second), and the still one',
                     fps: 4, size: SIZE, frames: names, still: names[0] };
      fs.writeFileSync(path.join(out, 'boto.json'), JSON.stringify(spec, null, 1) + '\n');
    }
  };
}

module.exports = { sheet };

// run alone, it is the one art drawer's sheet: `node tools/water/boto.cjs`
if (require.main === module) {
  require('../packart/render.cjs').main([__filename, OUT, ...process.argv.slice(2)])
    .catch((e) => { console.error(e); process.exit(1); });
}

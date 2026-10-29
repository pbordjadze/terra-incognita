// The vine's art (Prototypes/vine/), drawn from code for the one art drawer: the grapevine's leaf, five-lobed
// and toothed, in the greens of summer, the golds and reds of the vintage and the brown of a leaf that
// hangs on through winter; the tendril that writhes at a growing tip (eight frames); a cluster of grapes,
// green and ripe; the qvevri, the clay vessel buried to its mouth in which the first wine was made and
// still is; and the doorway. Rebuild with
//   node tools/packart/render.cjs tools/vine/art.js Prototypes/vine
// Every picture is drawn at three pixels a point; the script shows one picture at several sizes.
(function () {
  'use strict';
  const B = window.Brush;
  const INK = '#2B2616', STEM = '#5A3A1C';
  // the map's discipline: nothing shouts. Greens toward the ground's own, autumn ochre and rust rather
  // than orange, grapes a dusty vine-black with one light
  const GREENS = [['#3F6F33', '#7AA35A'], ['#4A7C3A', '#8DB06A'], ['#36652E', '#6C9A4E']];
  const AUTUMNS = [['#A8772A', '#D6AD58'], ['#94482E', '#C47A4A'], ['#B08630', '#DDB968']];
  const WINTER = ['#6E5230', '#9C7A4C'];
  const GRAPE_RIPE = ['#3C2040', '#4B2A52', '#573262', '#2E1733'], GRAPE_GREEN = ['#93AE5C', '#A6BE72', '#88A352', '#B3C884'];
  const CLAY = '#B97A49', CLAY_DARK = '#8A5631', EARTH = '#6B4E33', WINE = '#3A1230';

  /** The outline of a vine leaf, in a unit box round (0, 0): five lobes (the middle longest), two deep
      sinuses either side of it, two shallower ones to the base, the petiolar sinus where the stalk comes
      in, and teeth along every edge. Returns points for a smooth closed curve. */
  function leafOutline(r, teeth) {
    // the lobe tips and the sinuses between, as (angle from straight up, radius), clockwise from the base
    const spine = [
      [-168, 0.30], [-150, 0.62], [-132, 0.42], [-110, 0.86], [-85, 0.48], [-60, 0.94], [-32, 0.56],
      [0, 1.0],
      [32, 0.56], [60, 0.94], [85, 0.48], [110, 0.86], [132, 0.42], [150, 0.62], [168, 0.30],
    ];
    const pts = [];
    for (let i = 0; i < spine.length; i++) {
      const [a0, r0] = spine[i];
      const jitter = 1 + (r() - 0.5) * 0.06;
      const push = (a, rr) => { const t = (a - 90) * Math.PI / 180; pts.push([Math.cos(t) * rr, Math.sin(t) * rr]); };
      push(a0, r0 * jitter);
      if (i < spine.length - 1 && teeth) {
        // teeth between a tip and a sinus: small notches in and out along the edge
        const [a1, r1] = spine[i + 1], n = 2;
        for (let k = 1; k <= n; k++) {
          const f = k / (n + 1), a = a0 + (a1 - a0) * f, base = r0 + (r1 - r0) * f;
          push(a, base * (k % 2 ? 1.04 : 0.97));
        }
      }
    }
    pts.push([0, 0.18]);   // the petiolar sinus, where the stalk comes in
    return pts;
  }

  /** A vine leaf: its outline inked and filled with light at the centre, veins to every lobe, a stalk. */
  function leaf(ctx, w, h, palette, angle, seed, options) {
    options = options || {};
    const r = B.rand(seed || 1);
    ctx.save(); ctx.translate(w / 2, h * 0.52); ctx.rotate(angle || 0);
    const R = Math.min(w, h) * (options.scale || 0.46);
    const pts = leafOutline(r, true).map(([x, y]) => [x * R * (options.narrow || 1), y * R]);
    const g = ctx.createRadialGradient(0, -R * 0.15, R * 0.05, 0, -R * 0.1, R * 1.05);
    g.addColorStop(0, palette[1]); g.addColorStop(1, palette[0]);
    B.inked(ctx, (c) => { B.curve(c, pts, true); }, g, INK, Math.max(0.5, R * 0.075));
    // the blotches of a leaf turning (autumn): a few soft spots of the other tone
    if (options.blotch) {
      ctx.save(); ctx.beginPath(); B.curve(ctx, pts, true); ctx.clip();
      for (let i = 0; i < 5; i++) B.glow(ctx, (r() - 0.5) * R * 1.2, (r() - 0.7) * R * 1.2, R * (0.25 + r() * 0.2), options.blotch, 0.55, 0);
      ctx.restore();
    }
    // the veins from the base to each lobe, with side veins
    ctx.strokeStyle = B.rgba(options.vein || '#1E3A14', 0.42); ctx.lineWidth = Math.max(0.3, R * 0.05); ctx.lineCap = 'round';
    const base = [0, R * 0.14];
    [-110, -60, 0, 60, 110].forEach((a, i) => {
      const t = (a - 90) * Math.PI / 180, L = [0.86, 0.94, 1.0, 0.94, 0.86][i] * 0.78;
      const tip = [Math.cos(t) * R * L, Math.sin(t) * R * L];
      ctx.beginPath(); ctx.moveTo(base[0], base[1]); ctx.quadraticCurveTo(tip[0] * 0.45, base[1] + (tip[1] - base[1]) * 0.55, tip[0], tip[1]); ctx.stroke();
      ctx.lineWidth = Math.max(0.25, R * 0.03);
      for (const f of [0.45, 0.7]) {
        const px = base[0] + (tip[0] - base[0]) * f, py = base[1] + (tip[1] - base[1]) * f;
        ctx.beginPath(); ctx.moveTo(px, py); ctx.lineTo(px + Math.cos(t + 0.9) * R * 0.2, py + Math.sin(t + 0.9) * R * 0.2); ctx.stroke();
        ctx.beginPath(); ctx.moveTo(px, py); ctx.lineTo(px + Math.cos(t - 0.9) * R * 0.2, py + Math.sin(t - 0.9) * R * 0.2); ctx.stroke();
      }
      ctx.lineWidth = Math.max(0.3, R * 0.05);
    });
    // the stalk
    ctx.strokeStyle = STEM; ctx.lineWidth = Math.max(0.5, R * 0.07);
    ctx.beginPath(); ctx.moveTo(0, R * 0.18); ctx.quadraticCurveTo(-R * 0.1, R * 0.55, -R * 0.3, R * 0.9); ctx.stroke();
    ctx.restore();
  }

  /** A bunch of grapes: berries packed in a tapering cluster, each lit from the upper left, the stalk above. */
  function grapes(ctx, w, h, colours, seed) {
    const r = B.rand(seed || 2);
    ctx.strokeStyle = STEM; ctx.lineWidth = h * 0.045; ctx.lineCap = 'round';
    ctx.beginPath(); ctx.moveTo(w * 0.5, h * 0.02); ctx.quadraticCurveTo(w * 0.52, h * 0.12, w * 0.5, h * 0.22); ctx.stroke();
    const rows = [[0.3, 3], [0.42, 4], [0.54, 3], [0.66, 3], [0.78, 2], [0.9, 1]];
    const rad = w * 0.125;
    rows.forEach(([y, n], row) => {
      for (let i = 0; i < n; i++) {
        const x = w * 0.5 + (i - (n - 1) / 2) * rad * 1.75 + (r() - 0.5) * rad * 0.25, yy = h * y + (r() - 0.5) * rad * 0.25;
        const colour = colours[(row * 3 + i) % colours.length];
        const g = ctx.createRadialGradient(x - rad * 0.35, yy - rad * 0.35, rad * 0.1, x, yy, rad * 1.1);
        g.addColorStop(0, B.mix(colour, '#FFFFFF', 0.45)); g.addColorStop(0.5, colour); g.addColorStop(1, B.mix(colour, INK, 0.35));
        B.inked(ctx, (c) => { c.arc(x, yy, rad, 0, 6.2832); }, g, INK, rad * 0.16);
      }
    });
  }

  /** The tendril at a growing tip, facing east (the sprite is mirrored when the tip moves west): a shoot
      coming in from the west that thins and curls into a spiral, the spiral writhing with the frame. A pale
      young leaf unfurls where the shoot begins. */
  function tendril(ctx, w, h, t) {
    const sway = Math.sin(t * 6.2832), turns = 1.9 + 0.45 * sway;
    const cx = w * 0.62, cy = h * 0.48 + sway * h * 0.04;
    const pts = [[w * 0.04, h * 0.72], [w * 0.2, h * 0.66 + sway * h * 0.03], [w * 0.38, h * 0.5]];
    // the spiral, from the shoot in to its centre
    const n = 48;
    for (let i = 0; i <= n; i++) {
      const a = i / n * turns * 6.2832 + Math.PI, rr = h * 0.26 * (1 - i / n) + h * 0.015;
      pts.push([cx + Math.cos(a + t * 6.2832 * 0.5) * rr, cy + Math.sin(a + t * 6.2832 * 0.5) * rr]);
    }
    // drawn in pieces that thin toward the tip, ink under green
    for (const pass of [[INK, 1.0, 0.55], ['#6E9E3A', 0.62, 1]]) {
      ctx.strokeStyle = pass[0]; ctx.globalAlpha = pass[2]; ctx.lineCap = 'round'; ctx.lineJoin = 'round';
      for (let i = 1; i < pts.length; i++) {
        const f = i / (pts.length - 1);
        ctx.lineWidth = Math.max(0.5, h * 0.075 * (1 - f * 0.75)) * pass[1] + (pass[0] === INK ? h * 0.012 : 0);
        ctx.beginPath(); ctx.moveTo(pts[i - 1][0], pts[i - 1][1]); ctx.lineTo(pts[i][0], pts[i][1]); ctx.stroke();
      }
    }
    ctx.globalAlpha = 1;
    // the young leaf, bronze-green, small, at the shoot's second joint
    ctx.save(); ctx.translate(w * 0.17, h * 0.44); leaf(ctx, w * 0.42, h * 0.42, ['#7E9A3A', '#B7CF6E'], -0.9 + sway * 0.1, 11, { vein: '#4A5A18' }); ctx.restore();
  }

  /** The qvevri as the globe sees it, from above: a buried vessel is a dark round mouth in the earth, its
      clay rim level with the ground, the wine deep inside catching one light, a stone lid drawn half off it,
      and the turned earth round it. (The vine's first stem grows out of this mouth.) */
  function qvevri(ctx, w, h, options) {
    options = options || {};
    const cx = w / 2, cy = h / 2, R = Math.min(w, h) * 0.5;
    // the turned earth round the mouth, soft at its edge
    const eg = ctx.createRadialGradient(cx, cy, R * 0.45, cx, cy, R);
    eg.addColorStop(0, B.rgba(EARTH, 0.55)); eg.addColorStop(0.75, B.rgba(EARTH, 0.22)); eg.addColorStop(1, B.rgba(EARTH, 0));
    ctx.fillStyle = eg; ctx.beginPath(); ctx.arc(cx, cy, R, 0, 6.2832); ctx.fill();
    // the rim: a ring of clay, lit from the upper left
    const rim = R * 0.56, mouth = R * 0.4;
    const cg = ctx.createRadialGradient(cx - rim * 0.3, cy - rim * 0.3, mouth * 0.5, cx, cy, rim);
    cg.addColorStop(0, B.mix(CLAY, '#FFFFFF', 0.25)); cg.addColorStop(0.7, CLAY); cg.addColorStop(1, CLAY_DARK);
    B.inked(ctx, (c) => { c.arc(cx, cy, rim, 0, 6.2832); }, cg, INK, R * 0.05);
    // the mouth: the wine deep within, darkest at the far side, one light near the rim
    const wg = ctx.createRadialGradient(cx + mouth * 0.25, cy + mouth * 0.25, 0, cx, cy, mouth);
    wg.addColorStop(0, '#120409'); wg.addColorStop(0.7, WINE); wg.addColorStop(1, B.mix(WINE, '#000000', 0.5));
    B.inked(ctx, (c) => { c.arc(cx, cy, mouth, 0, 6.2832); }, wg, INK, R * 0.04);
    ctx.fillStyle = 'rgba(255, 230, 220, 0.35)'; ctx.beginPath(); ctx.ellipse(cx - mouth * 0.45, cy - mouth * 0.45, mouth * 0.22, mouth * 0.1, -0.7, 0, 6.2832); ctx.fill();
    // the stone lid, drawn half off the mouth
    const lg = ctx.createRadialGradient(cx + rim * 0.5, cy + rim * 0.55, 0, cx + rim * 0.62, cy + rim * 0.66, mouth * 1.05);
    lg.addColorStop(0, '#C4BDAE'); lg.addColorStop(1, '#8E887C');
    B.inked(ctx, (c) => { c.arc(cx + rim * 0.62, cy + rim * 0.66, mouth * 0.95, 0, 6.2832); }, lg, INK, R * 0.04);
    if (options.leaf) {
      ctx.save(); ctx.translate(cx - R * 0.95, cy - R * 0.98); leaf(ctx, R * 0.95, R * 0.95, GREENS[0], -0.5, 4); ctx.restore();
    }
  }

  /** The doorway: the qvevri's mouth in the ground with a first leaf at its rim, the whole idea in a thumb's width. */
  function doorway(ctx, w, h) { qvevri(ctx, w, h, { leaf: true }); }

  /** The spark that runs along the vine to the place touched: a white core in a warm glow. */
  function spark(ctx, w, h) {
    B.glow(ctx, w / 2, h / 2, w * 0.5, '#FFB83A', 0.75, 0);
    B.glow(ctx, w / 2, h / 2, w * 0.22, '#FFFFFF', 1, 0);
  }

  const P = [];
  for (let k = 0; k < 3; k++) {
    P.push({ name: 'leaf-green-' + (k + 1), width: 30, height: 30, draw(ctx, w, h) { leaf(ctx, w, h, GREENS[k], (k - 1) * 0.55, 10 + k); } });
    P.push({ name: 'leaf-autumn-' + (k + 1), width: 30, height: 30, draw(ctx, w, h) { leaf(ctx, w, h, AUTUMNS[k], (k - 1) * 0.55, 20 + k, { blotch: k === 1 ? '#E9B23A' : '#B9402A', vein: '#5A2A10' }); } });
    P.push({ name: 'leaf-winter-' + (k + 1), width: 30, height: 30, draw(ctx, w, h) { leaf(ctx, w, h, WINTER, (k - 1) * 0.55 + 0.3, 30 + k, { narrow: 0.78, vein: '#3A2A14' }); } });
  }
  P.push({ name: 'grapes-ripe', width: 20, height: 30, draw(ctx, w, h) { grapes(ctx, w, h, GRAPE_RIPE, 31); } });
  P.push({ name: 'grapes-green', width: 20, height: 30, draw(ctx, w, h) { grapes(ctx, w, h, GRAPE_GREEN, 32); } });
  P.push({ name: 'tendril', width: 34, height: 30, frames: 8, draw(ctx, w, h, i, n) { tendril(ctx, w, h, i / n); } });
  P.push({ name: 'qvevri', width: 64, height: 64, draw(ctx, w, h) { qvevri(ctx, w, h); } });
  P.push({ name: 'doorway', width: 32, height: 32, draw: doorway });
  P.push({ name: 'spark', width: 24, height: 24, draw: spark });
  window.SHEET = { pictures: P };
})();

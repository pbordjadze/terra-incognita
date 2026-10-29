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
    // (a vine leaf is rounder than a maple's: shallow sinuses, broad lobes)
    const spine = [
      [-168, 0.34], [-148, 0.66], [-128, 0.56], [-108, 0.86], [-82, 0.66], [-58, 0.95], [-30, 0.74],
      [0, 1.0],
      [30, 0.74], [58, 0.95], [82, 0.66], [108, 0.86], [128, 0.56], [148, 0.66], [168, 0.34],
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
          push(a, base * (k % 2 ? 1.03 : 0.975));
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
    const sway = Math.sin(t * 6.2832), turns = 1.35 + 0.3 * sway;
    const cx = w * 0.7, cy = h * 0.5 + sway * h * 0.04;
    const pts = [[w * 0.02, h * 0.74], [w * 0.22, h * 0.7 + sway * h * 0.03], [w * 0.46, h * 0.56]];
    // the spiral, from the shoot in to its centre: a thin curl, the stem's own brown-green, not a shell
    const n = 40;
    for (let i = 0; i <= n; i++) {
      const a = i / n * turns * 6.2832 + Math.PI, rr = h * 0.2 * (1 - i / n) + h * 0.012;
      pts.push([cx + Math.cos(a + t * 6.2832 * 0.5) * rr, cy + Math.sin(a + t * 6.2832 * 0.5) * rr]);
    }
    for (const pass of [[INK, 1.0, 0.45], ['#7B7A3A', 0.6, 1]]) {
      ctx.strokeStyle = pass[0]; ctx.globalAlpha = pass[2]; ctx.lineCap = 'round'; ctx.lineJoin = 'round';
      for (let i = 1; i < pts.length; i++) {
        const f = i / (pts.length - 1);
        ctx.lineWidth = Math.max(0.4, h * 0.06 * (1 - f * 0.8)) * pass[1] + (pass[0] === INK ? h * 0.01 : 0);
        ctx.beginPath(); ctx.moveTo(pts[i - 1][0], pts[i - 1][1]); ctx.lineTo(pts[i][0], pts[i][1]); ctx.stroke();
      }
    }
    ctx.globalAlpha = 1;
    // the young leaf, bronze-green, at the shoot's second joint
    ctx.save(); ctx.translate(w * 0.2, h * 0.42); leaf(ctx, w * 0.4, h * 0.4, ['#7E8A3A', '#B3C26A'], -0.9 + sway * 0.1, 11, { vein: '#4A5A18' }); ctx.restore();
  }

  /** The qvevri as the globe sees it, from above: a buried vessel is a dark round mouth in the earth, its
      clay rim level with the ground, the wine deep inside catching one light, a stone lid drawn half off it,
      and the turned earth round it. (The vine's first stem grows out of this mouth.) */
  function qvevri(ctx, w, h, options) {
    options = options || {};
    const cx = w / 2, cy = h / 2, R = Math.min(w, h) * 0.5;
    const r = B.rand(21);
    // the turned earth round the mouth, soft at its edge, and the cracks of dried clay
    const eg = ctx.createRadialGradient(cx, cy, R * 0.5, cx, cy, R * 0.92);
    eg.addColorStop(0, B.rgba(EARTH, 0.42)); eg.addColorStop(0.6, B.rgba(EARTH, 0.14)); eg.addColorStop(1, B.rgba(EARTH, 0));
    ctx.fillStyle = eg; ctx.beginPath(); ctx.arc(cx, cy, R, 0, 6.2832); ctx.fill();
    ctx.strokeStyle = B.rgba(INK, 0.16); ctx.lineWidth = Math.max(0.4, R * 0.01); ctx.lineCap = 'round';
    for (let i = 0; i < 5; i++) {
      const a = i / 9 * 6.2832 + r() * 0.5, r0 = R * (0.6 + r() * 0.08), r1 = R * (0.8 + r() * 0.15);
      ctx.beginPath(); ctx.moveTo(cx + Math.cos(a) * r0, cy + Math.sin(a) * r0);
      const am = a + (r() - 0.5) * 0.25, rm = (r0 + r1) / 2;
      ctx.lineTo(cx + Math.cos(am) * rm, cy + Math.sin(am) * rm);
      ctx.lineTo(cx + Math.cos(a + (r() - 0.5) * 0.3) * r1, cy + Math.sin(a + (r() - 0.5) * 0.3) * r1); ctx.stroke();
    }
    // the rim: a ring of clay, lit from the upper left
    const rim = R * 0.6, mouth = R * 0.45;
    // the earth itself: mottled, a few pebbles
    for (let i = 0; i < 70; i++) {
      const a = r() * 6.2832, d = R * (0.62 + r() * 0.3), pr = R * (0.006 + r() * 0.014);
      ctx.fillStyle = B.rgba(r() < 0.5 ? CLAY_DARK : '#4A3620', 0.18 + r() * 0.2);
      ctx.beginPath(); ctx.ellipse(cx + Math.cos(a) * d, cy + Math.sin(a) * d, pr * (1 + r()), pr, r() * 3, 0, 6.2832); ctx.fill();
    }
    const cg = ctx.createRadialGradient(cx - rim * 0.3, cy - rim * 0.3, mouth * 0.5, cx, cy, rim);
    cg.addColorStop(0, B.mix(CLAY, '#FFFFFF', 0.18)); cg.addColorStop(0.7, CLAY); cg.addColorStop(1, CLAY_DARK);
    B.inked(ctx, (c) => { c.arc(cx, cy, rim, 0, 6.2832); }, cg, INK, R * 0.05);
    // the rim worn: chips and wine stains along it, and the coil lines of the potter's clay
    ctx.save(); ctx.beginPath(); ctx.arc(cx, cy, rim, 0, 6.2832); ctx.arc(cx, cy, mouth, 0, 6.2832, true); ctx.clip();
    for (let i = 0; i < 40; i++) {
      const a = r() * 6.2832, d = mouth + (rim - mouth) * r(), pr = R * (0.008 + r() * 0.02);
      ctx.fillStyle = B.rgba(r() < 0.6 ? CLAY_DARK : WINE, 0.15 + r() * 0.25);
      ctx.beginPath(); ctx.ellipse(cx + Math.cos(a) * d, cy + Math.sin(a) * d, pr * 1.6, pr, a, 0, 6.2832); ctx.fill();
    }
    ctx.strokeStyle = B.rgba(CLAY_DARK, 0.3); ctx.lineWidth = Math.max(0.3, R * 0.008);
    for (const f of [0.3, 0.55, 0.8]) { ctx.beginPath(); ctx.arc(cx, cy, mouth + (rim - mouth) * f, 0, 6.2832); ctx.stroke(); }
    ctx.restore();
    // the mouth: the wine deep within, darkest at the far side, one light near the rim
    const wg = ctx.createRadialGradient(cx + mouth * 0.25, cy + mouth * 0.25, 0, cx, cy, mouth);
    wg.addColorStop(0, '#0C0306'); wg.addColorStop(0.75, B.mix(WINE, '#000000', 0.3)); wg.addColorStop(1, B.mix(WINE, '#000000', 0.6));
    B.inked(ctx, (c) => { c.arc(cx, cy, mouth, 0, 6.2832); }, wg, INK, R * 0.04);
    // the inside of the rim, in shadow on the near side: the wall of the vessel seen going down
    ctx.save(); ctx.beginPath(); ctx.arc(cx, cy, mouth, 0, 6.2832); ctx.clip();
    const sh = ctx.createRadialGradient(cx - mouth * 0.25, cy - mouth * 0.3, mouth * 0.55, cx, cy, mouth);
    sh.addColorStop(0, 'rgba(0,0,0,0)'); sh.addColorStop(1, 'rgba(0,0,0,0.5)');
    ctx.fillStyle = sh; ctx.fillRect(cx - mouth, cy - mouth, 2 * mouth, 2 * mouth); ctx.restore();
    // the lid: a flat flagstone, its edges straight and uneven as a split stone's, drawn a third off the
    // mouth, its sliver of shadow on the rim
    const lx = cx + rim * 0.72, ly = cy + rim * 0.74, ls = mouth * 0.82;
    const stone = [[-0.95, -0.42], [-0.55, -0.9], [0.1, -1.0], [0.62, -0.72], [1.0, -0.05], [0.78, 0.55], [0.2, 0.95], [-0.45, 0.82], [-0.88, 0.3]]
      .map(([x, y]) => [lx + x * ls * (0.94 + r() * 0.12), ly + y * ls * (0.94 + r() * 0.12)]);
    const poly = (c, pts) => { c.moveTo(pts[0][0], pts[0][1]); for (let i = 1; i < pts.length; i++) c.lineTo(pts[i][0], pts[i][1]); c.closePath(); };
    ctx.fillStyle = 'rgba(20, 10, 5, 0.3)'; ctx.beginPath(); poly(ctx, stone.map(([x, y]) => [x + ls * 0.09, y + ls * 0.11])); ctx.fill();
    const lg = ctx.createLinearGradient(lx - ls, ly - ls, lx + ls, ly + ls);
    lg.addColorStop(0, '#CFC6B2'); lg.addColorStop(0.5, '#B3AA97'); lg.addColorStop(1, '#8A8275');
    B.inked(ctx, (c) => { poly(c, stone); }, lg, INK, R * 0.03);
    // the stone's grain: fine lines across it, and a few flecks
    ctx.save(); ctx.beginPath(); poly(ctx, stone); ctx.clip();
    // the stone's surface: mottled, a few flecks and one hairline crack; no ruled lines
    for (let i = 0; i < 90; i++) { ctx.fillStyle = B.rgba(i % 3 ? '#FFFFFF' : INK, 0.05 + r() * 0.08); ctx.beginPath(); ctx.ellipse(lx + (r() - 0.5) * 2 * ls, ly + (r() - 0.5) * 2 * ls, R * (0.006 + r() * 0.02), R * (0.004 + r() * 0.012), r() * 3, 0, 6.2832); ctx.fill(); }
    ctx.strokeStyle = B.rgba(INK, 0.22); ctx.lineWidth = Math.max(0.3, R * 0.006);
    ctx.beginPath(); ctx.moveTo(lx - ls * 0.6, ly + ls * 0.5); ctx.lineTo(lx - ls * 0.2, ly + ls * 0.1); ctx.lineTo(lx + ls * 0.1, ly - ls * 0.05); ctx.stroke();
    ctx.restore();
    if (options.leaf) {
      ctx.save(); ctx.translate(cx - R * 0.95, cy - R * 0.98); leaf(ctx, R * 0.95, R * 0.95, GREENS[0], -0.5, 4); ctx.restore();
    }
  }

  /** The doorway: ordinary vine life, slightly off. A vine leaf with a cluster under it lying on a Georgian
      valley, the size of a village: nothing says "touch here"; whoever is paying attention sees a leaf
      where no leaf should be seen from so high, and touches it. */
  function doorway(ctx, w, h) {
    ctx.save(); ctx.translate(-w * 0.04, -h * 0.06); leaf(ctx, w * 0.8, h * 0.8, GREENS[1], -0.35, 7); ctx.restore();
    ctx.save(); ctx.translate(w * 0.5, h * 0.4); grapes(ctx, w * 0.42, h * 0.6, GRAPE_RIPE, 8); ctx.restore();
  }

  /** The spark that runs along the vine to the place touched: a white core in a warm glow. */
  function spark(ctx, w, h) {
    B.glow(ctx, w / 2, h / 2, w * 0.5, '#FFB83A', 0.75, 0);
    B.glow(ctx, w / 2, h / 2, w * 0.22, '#FFFFFF', 1, 0);
  }
  /** The light breathing where the finger was: six frames of a glow swelling and settling. */
  function pulse(ctx, w, h, t) {
    const k = 0.72 + 0.28 * Math.sin(t * 6.2832);
    B.glow(ctx, w / 2, h / 2, w * 0.5 * k, '#F2B23A', 0.5, 0);
    B.glow(ctx, w / 2, h / 2, w * 0.16 * k, '#FFF6DC', 0.95, 0);
  }
  /** The ship that carried the vine over the sea: a small lateen-rigged hull seen from the side, facing east,
      rocking a little between two frames; drawn small and plain, a mark on the sea, not a portrait. */
  function ship(ctx, w, h, t) {
    const rock = (t ? 1 : -1) * 0.05;
    ctx.save(); ctx.translate(w / 2, h * 0.62); ctx.rotate(rock);
    // the hull
    B.inked(ctx, (c) => { c.moveTo(-w * 0.42, -h * 0.06); c.quadraticCurveTo(-w * 0.3, h * 0.26, 0, h * 0.28); c.quadraticCurveTo(w * 0.3, h * 0.26, w * 0.46, -h * 0.1);
      c.lineTo(w * 0.36, -h * 0.06); c.closePath(); }, '#6B4A2E', INK, h * 0.05);
    // the mast and the sail, bellied
    ctx.strokeStyle = INK; ctx.lineWidth = h * 0.04; ctx.beginPath(); ctx.moveTo(-w * 0.02, -h * 0.06); ctx.lineTo(w * 0.02, -h * 0.6); ctx.stroke();
    const sg = ctx.createLinearGradient(-w * 0.3, 0, w * 0.3, 0);
    sg.addColorStop(0, '#F3E9D2'); sg.addColorStop(1, '#D9C9A6');
    B.inked(ctx, (c) => { c.moveTo(w * 0.02, -h * 0.58); c.quadraticCurveTo(w * 0.34, -h * 0.36, w * 0.3, -h * 0.08); c.lineTo(-w * 0.26, -h * 0.08); c.quadraticCurveTo(-w * 0.1, -h * 0.34, w * 0.02, -h * 0.58); c.closePath(); }, sg, INK, h * 0.035);
    ctx.restore();
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
  // (a picture lying on the globe, so it grows as the reader comes close: drawn large)
  P.push({ name: 'qvevri', width: 256, height: 256, draw(ctx, w, h) { qvevri(ctx, w, h); } });
  P.push({ name: 'doorway', width: 30, height: 32, draw: doorway });
  P.push({ name: 'spark', width: 24, height: 24, draw: spark });
  P.push({ name: 'pulse', width: 32, height: 32, frames: 6, draw(ctx, w, h, i, n) { pulse(ctx, w, h, i / n); } });
  P.push({ name: 'ship', width: 30, height: 24, frames: 2, draw(ctx, w, h, i) { ship(ctx, w, h, i); } });
  window.SHEET = { pictures: P };
})();

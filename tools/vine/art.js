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
  const GREENS = [['#3E7A2E', '#6FAF4A'], ['#4A8A36', '#86BE5A'], ['#35702A', '#5E9E42']];
  const AUTUMNS = [['#B9741F', '#E7B64A'], ['#A8402A', '#DE7A3C'], ['#C58A22', '#F0CB5E']];
  const WINTER = ['#6E5230', '#9C7A4C'];
  const GRAPE_RIPE = ['#4A2458', '#5F2F72', '#6E3A82', '#3C1B49'], GRAPE_GREEN = ['#9DBE5E', '#B4D078', '#8FB252', '#C3DB8C'];
  const CLAY = '#B97A49', CLAY_DARK = '#8A5631', EARTH = '#6B4E33', WINE = '#4A1A36';

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

  /** The qvevri, seen with the ground cut away: the earth, the egg-shaped vessel buried to its mouth, the
      dark wine within, the stone lid beside the mouth, and the first shoot rising from the ground. */
  function qvevri(ctx, w, h) {
    const ground = h * 0.36;
    // the earth below the surface, only round the vessel, fading
    const g = ctx.createRadialGradient(w * 0.5, ground + h * 0.22, h * 0.02, w * 0.5, ground + h * 0.22, h * 0.5);
    g.addColorStop(0, B.rgba(EARTH, 0.5)); g.addColorStop(0.7, B.rgba(EARTH, 0.18)); g.addColorStop(1, B.rgba(EARTH, 0));
    ctx.save(); ctx.beginPath(); ctx.rect(0, ground, w, h - ground); ctx.clip();
    ctx.fillStyle = g; ctx.beginPath(); ctx.ellipse(w * 0.5, ground + h * 0.1, w * 0.5, h * 0.5, 0, 0, 6.2832); ctx.fill(); ctx.restore();
    // the surface: a thin band of turned earth, soft at its ends
    const sg = ctx.createLinearGradient(0, 0, w, 0);
    sg.addColorStop(0, B.rgba('#8A6A44', 0)); sg.addColorStop(0.18, '#8A6A44'); sg.addColorStop(0.82, '#8A6A44'); sg.addColorStop(1, B.rgba('#8A6A44', 0));
    ctx.fillStyle = sg; ctx.fillRect(0, ground - h * 0.02, w, h * 0.04);
    // the vessel, egg-shaped, widest a third down, tapering to a point
    const vessel = (c) => {
      c.moveTo(w * 0.36, ground); c.bezierCurveTo(w * 0.1, ground + h * 0.12, w * 0.12, ground + h * 0.4, w * 0.4, ground + h * 0.58);
      c.quadraticCurveTo(w * 0.5, ground + h * 0.64, w * 0.6, ground + h * 0.58);
      c.bezierCurveTo(w * 0.88, ground + h * 0.4, w * 0.9, ground + h * 0.12, w * 0.64, ground); c.closePath();
    };
    const cg = ctx.createLinearGradient(w * 0.15, 0, w * 0.85, 0);
    cg.addColorStop(0, CLAY); cg.addColorStop(0.45, B.mix(CLAY, '#FFFFFF', 0.18)); cg.addColorStop(1, CLAY_DARK);
    B.inked(ctx, vessel, cg, INK, h * 0.03);
    // the bands a potter's coils leave
    ctx.save(); ctx.beginPath(); vessel(ctx); ctx.clip();
    ctx.strokeStyle = B.rgba(CLAY_DARK, 0.35); ctx.lineWidth = h * 0.012;
    for (let i = 1; i <= 6; i++) { const y = ground + h * 0.08 * i; ctx.beginPath(); ctx.moveTo(0, y); ctx.quadraticCurveTo(w * 0.5, y + h * 0.03, w, y); ctx.stroke(); }
    ctx.restore();
    // the mouth at the surface, the wine dark within, the lid set beside it
    B.inked(ctx, (c) => { c.ellipse(w * 0.5, ground, w * 0.14, h * 0.045, 0, 0, 6.2832); }, WINE, INK, h * 0.025);
    ctx.fillStyle = 'rgba(255,255,255,0.18)'; ctx.beginPath(); ctx.ellipse(w * 0.46, ground - h * 0.01, w * 0.05, h * 0.012, 0, 0, 6.2832); ctx.fill();
    B.inked(ctx, (c) => { c.ellipse(w * 0.8, ground - h * 0.035, w * 0.11, h * 0.03, 0, 0, 6.2832); }, '#A9A295', INK, h * 0.022);
    // the first shoot, rising from the ground beside the mouth, with a leaf and a cluster
    ctx.strokeStyle = STEM; ctx.lineWidth = h * 0.04; ctx.lineCap = 'round';
    ctx.beginPath(); ctx.moveTo(w * 0.24, ground - h * 0.01); ctx.bezierCurveTo(w * 0.16, ground - h * 0.16, w * 0.3, ground - h * 0.22, w * 0.3, ground - h * 0.3); ctx.stroke();
    ctx.save(); ctx.translate(w * 0.17, h * 0.0); leaf(ctx, w * 0.34, h * 0.34, GREENS[0], -0.45, 4); ctx.restore();
    ctx.save(); ctx.translate(w * 0.3, ground - h * 0.3); grapes(ctx, w * 0.17, h * 0.26, GRAPE_RIPE, 5); ctx.restore();
  }

  /** The doorway: the qvevri's mouth in the ground with its first leaf, the pack's whole idea in a thumb's width. */
  function doorway(ctx, w, h) { qvevri(ctx, w, h); }

  const P = [];
  for (let k = 0; k < 3; k++) {
    P.push({ name: 'leaf-green-' + (k + 1), width: 30, height: 30, draw(ctx, w, h) { leaf(ctx, w, h, GREENS[k], (k - 1) * 0.55, 10 + k); } });
    P.push({ name: 'leaf-autumn-' + (k + 1), width: 30, height: 30, draw(ctx, w, h) { leaf(ctx, w, h, AUTUMNS[k], (k - 1) * 0.55, 20 + k, { blotch: k === 1 ? '#E9B23A' : '#B9402A', vein: '#5A2A10' }); } });
    P.push({ name: 'leaf-winter-' + (k + 1), width: 30, height: 30, draw(ctx, w, h) { leaf(ctx, w, h, WINTER, (k - 1) * 0.55 + 0.3, 30 + k, { narrow: 0.78, vein: '#3A2A14' }); } });
  }
  P.push({ name: 'grapes-ripe', width: 20, height: 30, draw(ctx, w, h) { grapes(ctx, w, h, GRAPE_RIPE, 31); } });
  P.push({ name: 'grapes-green', width: 20, height: 30, draw(ctx, w, h) { grapes(ctx, w, h, GRAPE_GREEN, 32); } });
  P.push({ name: 'tendril', width: 34, height: 30, frames: 8, draw(ctx, w, h, i, n) { tendril(ctx, w, h, i / n); } });
  P.push({ name: 'qvevri', width: 64, height: 72, draw: qvevri });
  P.push({ name: 'doorway', width: 30, height: 34, draw: doorway });
  window.SHEET = { pictures: P };
})();

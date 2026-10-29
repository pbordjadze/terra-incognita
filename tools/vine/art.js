// The vine's art (Prototypes/vine/), drawn from code: the grapevine's five-lobed leaf in its summer
// green and its autumn golds and reds; a cluster of grapes, green and ripe; the tendril that curls at
// a growing tip; the qvevri, the clay vessel buried to its mouth in which the first wine was made and
// still is; and the doorway, a leaf with a cluster. Rebuild with
//   node tools/packart/render.cjs tools/vine/art.js Prototypes/vine
(function () {
  'use strict';
  const B = window.Brush;
  const INK = '#2E2A1C', STEM = '#5E3B1E';
  const GREENS = ['#4F8A3C', '#5E9A47', '#3F7A32'], AUTUMNS = ['#D8A531', '#C96A2B', '#A9402E', '#E0B94A'];
  const GRAPE_RIPE = ['#5A2D6B', '#6E3A82', '#4A2458'], GRAPE_GREEN = ['#A9C96A', '#B9D47E', '#98BA5C'];
  const CLAY = '#B5784A', CLAY_DARK = '#8A5631', EARTH = '#6B4E33';

  /** A vine leaf: five lobes with sinuses between, the petiolar sinus at the base, and the veins. */
  function leaf(ctx, w, h, fill, angle, seed) {
    const r = B.rand(seed || 1);
    ctx.save(); ctx.translate(w / 2, h * 0.56); ctx.rotate(angle || 0);
    const R = Math.min(w, h) * 0.46;
    const pts = [];
    // the outline: lobes at five angles, sinuses between, as a radial polygon smoothed
    const lobes = [-150, -105, -50, 0, 50, 105, 150].map((a) => a * Math.PI / 180);
    for (let i = 0; i < lobes.length; i++) {
      const a = lobes[i] - Math.PI / 2, deep = (i === 0 || i === 6) ? 0.55 : (i === 1 || i === 5) ? 0.86 : i === 3 ? 1 : 0.95;
      pts.push([Math.cos(a) * R * deep * (0.95 + r() * 0.1), Math.sin(a) * R * deep * (0.95 + r() * 0.1)]);
      if (i < lobes.length - 1) {
        const m = (lobes[i] + lobes[i + 1]) / 2 - Math.PI / 2, sd = (i === 2 || i === 3) ? 0.55 : 0.5;
        pts.push([Math.cos(m) * R * sd, Math.sin(m) * R * sd]);
      }
    }
    pts.push([0, R * 0.2]);   // the base's sinus, where the stalk comes in
    B.inked(ctx, (c) => { B.curve(c, pts, true); }, fill, INK, Math.max(0.5, R * 0.08));
    // the veins from the base to each lobe
    ctx.strokeStyle = B.rgba(INK, 0.35); ctx.lineWidth = Math.max(0.3, R * 0.045); ctx.beginPath();
    for (let i = 0; i < 5; i++) { const a = [-105, -50, 0, 50, 105][i] * Math.PI / 180 - Math.PI / 2; ctx.moveTo(0, R * 0.15); ctx.lineTo(Math.cos(a) * R * 0.75, Math.sin(a) * R * 0.75); }
    ctx.stroke();
    ctx.restore();
  }

  /** A bunch of grapes: berries packed in a tapering cluster, lit from the upper left, with the stalk. */
  function grapes(ctx, w, h, colours, seed) {
    const r = B.rand(seed || 2);
    ctx.strokeStyle = STEM; ctx.lineWidth = h * 0.05; ctx.beginPath(); ctx.moveTo(w * 0.5, h * 0.02); ctx.lineTo(w * 0.5, h * 0.22); ctx.stroke();
    const rows = [[0.3, 3], [0.42, 4], [0.55, 3], [0.68, 2], [0.8, 2], [0.9, 1]];
    const rad = w * 0.11;
    rows.forEach(([y, n]) => {
      for (let i = 0; i < n; i++) {
        const x = w * 0.5 + (i - (n - 1) / 2) * rad * 1.7 + (r() - 0.5) * rad * 0.3, yy = h * y + (r() - 0.5) * rad * 0.3;
        const colour = colours[Math.floor(r() * colours.length)];
        B.inked(ctx, (c) => { c.arc(x, yy, rad, 0, 6.28); }, colour, INK, rad * 0.18);
        ctx.fillStyle = 'rgba(255,255,255,0.35)'; ctx.beginPath(); ctx.arc(x - rad * 0.3, yy - rad * 0.3, rad * 0.3, 0, 6.28); ctx.fill();
      }
    });
  }

  /** The tendril at a growing tip: a stem that curls, the curl turning with the frame. */
  function tendril(ctx, w, h, t) {
    ctx.strokeStyle = '#6F9B3C'; ctx.lineWidth = Math.max(0.8, h * 0.07); ctx.lineCap = 'round';
    ctx.beginPath();
    const cx = w * 0.5, cy = h * 0.55, turns = 1.6 + 0.5 * Math.sin(t * 6.28);
    ctx.moveTo(w * 0.1, h * 0.85);
    ctx.quadraticCurveTo(w * 0.3, h * 0.4, cx, cy);
    for (let i = 0; i <= 40; i++) {
      const a = i / 40 * turns * 6.28, rr = h * 0.3 * (1 - i / 40);
      ctx.lineTo(cx + Math.cos(a + t * 6.28) * rr, cy + Math.sin(a + t * 6.28) * rr);
    }
    ctx.stroke();
    ctx.strokeStyle = INK; ctx.lineWidth = Math.max(0.3, h * 0.025); ctx.globalAlpha = 0.5; ctx.stroke(); ctx.globalAlpha = 1;
    // a glint at the tip from afar is the same picture small: a touch of light here
    B.glow(ctx, w * 0.1, h * 0.85, w * 0.12, '#B9E08A', 0.5, 0);
  }

  /** The qvevri, seen from above the ground it is buried in: the earth cut away to show its egg shape
      below, its mouth at the surface with the stone lid beside it, and a vine rising from the ground. */
  function qvevri(ctx, w, h) {
    // the ground, a band at the surface and the earth below shown only round the vessel, fading
    const g = ctx.createLinearGradient(0, h * 0.4, 0, h);
    g.addColorStop(0, B.rgba(EARTH, 0.55)); g.addColorStop(1, B.rgba(EARTH, 0.0));
    ctx.fillStyle = g; ctx.beginPath(); ctx.ellipse(w * 0.5, h * 0.4, w * 0.48, h * 0.58, 0, 0, Math.PI); ctx.fill();
    ctx.fillStyle = '#85643F'; ctx.fillRect(w * 0.05, h * 0.37, w * 0.9, h * 0.05);
    ctx.strokeStyle = INK; ctx.lineWidth = h * 0.02; ctx.beginPath(); ctx.moveTo(w * 0.05, h * 0.37); ctx.lineTo(w * 0.95, h * 0.37); ctx.stroke();
    // the vessel below the ground, egg-shaped, tapering to a point
    B.inked(ctx, (c) => { c.moveTo(w * 0.36, h * 0.4); c.quadraticCurveTo(w * 0.18, h * 0.55, w * 0.3, h * 0.78);
      c.quadraticCurveTo(w * 0.42, h * 0.98, w * 0.5, h * 0.98); c.quadraticCurveTo(w * 0.58, h * 0.98, w * 0.7, h * 0.78);
      c.quadraticCurveTo(w * 0.82, h * 0.55, w * 0.64, h * 0.4); c.closePath(); }, CLAY, INK, h * 0.03);
    ctx.fillStyle = B.rgba(CLAY_DARK, 0.5); ctx.beginPath(); ctx.moveTo(w * 0.5, h * 0.4); ctx.quadraticCurveTo(w * 0.78, h * 0.55, w * 0.68, h * 0.8); ctx.quadraticCurveTo(w * 0.6, h * 0.95, w * 0.5, h * 0.97); ctx.closePath(); ctx.fill();
    // the mouth at the surface, and the dark wine within
    B.inked(ctx, (c) => { c.ellipse(w * 0.5, h * 0.4, w * 0.15, h * 0.05, 0, 0, 6.28); }, '#4A1E3E', INK, h * 0.025);
    // the lid, a stone, set beside it
    B.inked(ctx, (c) => { c.ellipse(w * 0.78, h * 0.36, w * 0.1, h * 0.035, 0, 0, 6.28); }, '#A9A295', INK, h * 0.025);
    // the vine rising at the left, with a leaf and a cluster
    ctx.strokeStyle = STEM; ctx.lineWidth = h * 0.04; ctx.beginPath(); ctx.moveTo(w * 0.2, h * 0.4); ctx.quadraticCurveTo(w * 0.15, h * 0.2, w * 0.3, h * 0.08); ctx.stroke();
    ctx.save(); ctx.translate(w * 0.22, h * 0.05); leaf(ctx, w * 0.3, h * 0.3, GREENS[0], -0.4, 4); ctx.restore();
    ctx.save(); ctx.translate(w * 0.28, h * 0.17); grapes(ctx, w * 0.16, h * 0.24, GRAPE_RIPE, 5); ctx.restore();
  }

  function doorway(ctx, w, h) {
    ctx.save(); ctx.translate(w * 0.0, h * 0.0); leaf(ctx, w * 0.75, h * 0.75, AUTUMNS[0], -0.5, 7); ctx.restore();
    ctx.save(); ctx.translate(w * 0.5, h * 0.36); grapes(ctx, w * 0.42, h * 0.62, GRAPE_RIPE, 8); ctx.restore();
  }

  const P = [];
  for (let k = 0; k < 3; k++) {
    P.push({ name: 'leaf-green-' + (k + 1), width: 16, height: 16, draw(ctx, w, h) { leaf(ctx, w, h, GREENS[k], (k - 1) * 0.5, 10 + k); } });
    P.push({ name: 'leaf-autumn-' + (k + 1), width: 16, height: 16, draw(ctx, w, h) { leaf(ctx, w, h, AUTUMNS[k], (k - 1) * 0.5, 20 + k); } });
    P.push({ name: 'leaf-green-big-' + (k + 1), width: 34, height: 34, draw(ctx, w, h) { leaf(ctx, w, h, GREENS[k], (k - 1) * 0.5, 10 + k); } });
    P.push({ name: 'leaf-autumn-big-' + (k + 1), width: 34, height: 34, draw(ctx, w, h) { leaf(ctx, w, h, AUTUMNS[k], (k - 1) * 0.5, 20 + k); } });
  }
  P.push({ name: 'grapes-ripe', width: 12, height: 18, draw(ctx, w, h) { grapes(ctx, w, h, GRAPE_RIPE, 31); } });
  P.push({ name: 'grapes-green', width: 12, height: 18, draw(ctx, w, h) { grapes(ctx, w, h, GRAPE_GREEN, 32); } });
  P.push({ name: 'grapes-ripe-big', width: 26, height: 40, draw(ctx, w, h) { grapes(ctx, w, h, GRAPE_RIPE, 31); } });
  P.push({ name: 'grapes-green-big', width: 26, height: 40, draw(ctx, w, h) { grapes(ctx, w, h, GRAPE_GREEN, 32); } });
  P.push({ name: 'tendril', width: 20, height: 20, frames: 8, draw(ctx, w, h, i, n) { tendril(ctx, w, h, i / n); } });
  P.push({ name: 'tendril-big', width: 44, height: 44, frames: 8, draw(ctx, w, h, i, n) { tendril(ctx, w, h, i / n); } });
  P.push({ name: 'splash', width: 24, height: 24, frames: 4, draw(ctx, w, h, i, n) {
    const t = (i + 1) / n, r = B.rand(50 + i);
    for (let k = 0; k < 8; k++) { const a = k / 8 * 6.28 + r() * 0.4, d = w * 0.18 + t * w * 0.3; B.glow(ctx, w / 2 + Math.cos(a) * d, h / 2 + Math.sin(a) * d, w * 0.09 * (1 - t * 0.5), '#6E3A82', 0.9 * (1 - t * 0.6), 0); }
  } });
  P.push({ name: 'qvevri', width: 64, height: 72, draw: qvevri });
  P.push({ name: 'qvevri-small', width: 28, height: 32, draw: qvevri });
  P.push({ name: 'doorway', width: 26, height: 28, draw: doorway });
  window.SHEET = { pictures: P };
})();

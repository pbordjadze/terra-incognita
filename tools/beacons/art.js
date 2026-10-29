// The beacons' art (Prototypes/fires-across-the-sea/), drawn from code: a fire on a peak, its flames
// licking and its smoke rising; the ember it is from afar; the pyre unlit, a stack of wood on a cairn;
// and the doorway, a fire already burning. Rebuild with
//   node tools/packart/render.cjs tools/beacons/art.js Prototypes/fires-across-the-sea
(function () {
  'use strict';
  const B = window.Brush;
  const INK = '#2C2418', WOOD = '#5A4330', STONE = '#8C8578', STONE_INK = '#4F4A42';
  const EMBER = '#F2A33A', FLAME = '#FFD166', FLAME_CORE = '#FFF4C2', FLAME_EDGE = '#E2552B', SMOKE = '#8E8A86';

  /** A tongue of flame from (x, base) up to a tip swaying by `sway`, `width` at its base. */
  function tongue(ctx, x, base, height, width, sway, fill, edge) {
    B.inked(ctx, (c) => {
      c.moveTo(x - width / 2, base);
      c.quadraticCurveTo(x - width * 0.55, base - height * 0.45, x + sway * 0.4 - width * 0.1, base - height * 0.7);
      c.quadraticCurveTo(x + sway * 0.9, base - height * 0.9, x + sway, base - height);
      c.quadraticCurveTo(x + sway * 0.7 + width * 0.25, base - height * 0.75, x + width * 0.5, base - height * 0.3);
      c.quadraticCurveTo(x + width * 0.55, base - height * 0.1, x + width / 2, base);
      c.closePath();
    }, fill, edge, edge ? 0.9 : 0);
  }

  /** The fire at frame t (0–1 of its flicker), on a pile of wood; `glow` adds the light round it. */
  function fire(ctx, w, h, t, glow) {
    const base = h * 0.84, cx = w / 2;
    if (glow) B.glow(ctx, cx, base - h * 0.25, w * 0.62, EMBER, 0.55, 0);
    // the wood: three logs crossed
    ctx.strokeStyle = WOOD; ctx.lineWidth = h * 0.07; ctx.lineCap = 'round';
    for (const [a, b] of [[-0.3, 0.26], [-0.26, -0.3], [0.1, 0.34]]) {
      ctx.beginPath(); ctx.moveTo(cx + w * a, base); ctx.lineTo(cx + w * b, base - h * 0.1); ctx.stroke();
    }
    ctx.strokeStyle = INK; ctx.lineWidth = h * 0.02;
    for (const [a, b] of [[-0.3, 0.26], [-0.26, -0.3], [0.1, 0.34]]) {
      ctx.beginPath(); ctx.moveTo(cx + w * a, base); ctx.lineTo(cx + w * b, base - h * 0.1); ctx.stroke();
    }
    // the flames: an outer, a middle and a core, each swaying on its own
    const s1 = Math.sin(t * 6.28) * w * 0.12, s2 = Math.sin(t * 6.28 * 2 + 1) * w * 0.08, s3 = Math.sin(t * 6.28 * 3 + 2) * w * 0.05;
    const hh = h * (0.68 + 0.08 * Math.sin(t * 6.28 * 2));
    tongue(ctx, cx, base - h * 0.06, hh, w * 0.56, s1, FLAME_EDGE, INK);
    tongue(ctx, cx + w * 0.02, base - h * 0.08, hh * 0.72, w * 0.38, s2, FLAME, null);
    tongue(ctx, cx - w * 0.01, base - h * 0.1, hh * 0.4, w * 0.2, s3, FLAME_CORE, null);
    // sparks
    const r = B.rand(Math.round(t * 97) + 5);
    ctx.fillStyle = FLAME;
    for (let i = 0; i < 4; i++) {
      const sx = cx + (r() - 0.5) * w * 0.6, sy = base - hh - r() * h * 0.2;
      ctx.beginPath(); ctx.arc(sx, sy, w * 0.025, 0, 6.28); ctx.fill();
    }
  }

  /** The smoke rising from a fire: wisps drifting up and a little to the right, fading. */
  function smoke(ctx, w, h, t) {
    const r = B.rand(13);
    for (let i = 0; i < 5; i++) {
      const u = (t + i / 5) % 1, x = w * 0.5 + Math.sin(u * 6 + i) * w * 0.12 + u * w * 0.25, y = h * 0.95 - u * h * 0.9;
      B.glow(ctx, x, y, w * (0.12 + u * 0.22), SMOKE, 0.45 * (1 - u) * (0.6 + 0.4 * r()), 0);
    }
  }

  /** The pyre waiting: the logs on a cairn of stones. */
  function pyre(ctx, w, h) {
    const base = h * 0.78, cx = w / 2;
    B.haloed(ctx, w, h, (c) => {
      c.lineCap = 'round'; c.lineJoin = 'round';
      for (const [dx, dy, rr] of [[-0.28, 0, 0.16], [0, 0.02, 0.18], [0.28, 0, 0.15], [-0.14, -0.16, 0.14], [0.14, -0.16, 0.14]]) {
        B.inked(c, (p) => { p.ellipse(cx + w * dx, base + h * dy, w * rr, h * 0.11, 0, 0, 6.28); }, STONE, STONE_INK, h * 0.05);
      }
      c.strokeStyle = WOOD; c.lineWidth = h * 0.09;
      for (const [a, b] of [[-0.3, 0.26], [-0.26, -0.3], [0.1, 0.34]]) {
        c.beginPath(); c.moveTo(cx + w * a, base - h * 0.22); c.lineTo(cx + w * b, base - h * 0.34); c.stroke();
      }
      c.strokeStyle = INK; c.lineWidth = h * 0.025;
      for (const [a, b] of [[-0.3, 0.26], [-0.26, -0.3], [0.1, 0.34]]) {
        c.beginPath(); c.moveTo(cx + w * a, base - h * 0.22); c.lineTo(cx + w * b, base - h * 0.34); c.stroke();
      }
    }, '#F4ECD8', 0.9);
  }

  /** The fire from afar: a point of light. */
  function ember(ctx, w, h, t) {
    const a = 0.75 + 0.25 * Math.sin(t * 6.28);
    B.glow(ctx, w / 2, h / 2, w * 0.5, EMBER, 0.9 * a, 0);
    ctx.fillStyle = FLAME_CORE; ctx.beginPath(); ctx.arc(w / 2, h / 2, w * 0.12, 0, 6.28); ctx.fill();
  }

  window.SHEET = { pictures: [
    // the fire with its smoke above it, one picture (the runtime holds 64 sprites: a fire is one)
    { name: 'fire', width: 30, height: 66, frames: 12, draw(ctx, w, h, i, n) {
        ctx.save(); ctx.translate(0, 0); smoke(ctx, w, h * 0.62, i / n); ctx.restore();
        ctx.save(); ctx.translate(0, h - 38); fire(ctx, w, 38, (i % 10) / 10, true); ctx.restore(); } },
    { name: 'fire-mid', width: 16, height: 35, frames: 12, draw(ctx, w, h, i, n) {
        ctx.save(); smoke(ctx, w, h * 0.62, i / n); ctx.restore();
        ctx.save(); ctx.translate(0, h - 20); fire(ctx, w, 20, (i % 10) / 10, true); ctx.restore(); } },
    { name: 'fire-still', width: 30, height: 38, draw(ctx, w, h) { fire(ctx, w, h, 0.3, true); } },
    { name: 'fire-still-mid', width: 16, height: 20, draw(ctx, w, h) { fire(ctx, w, h, 0.3, true); } },
    { name: 'smoke', width: 30, height: 44, frames: 12, draw(ctx, w, h, i, n) { smoke(ctx, w, h, i / n); } },
    { name: 'ember', width: 12, height: 12, frames: 6, draw(ctx, w, h, i, n) { ember(ctx, w, h, i / n); } },
    { name: 'pyre', width: 14, height: 11, draw: pyre },
    { name: 'pyre-mid', width: 8, height: 6, draw: pyre },
    { name: 'doorway', width: 22, height: 28, draw(ctx, w, h) { fire(ctx, w, h, 0.3, true); } },
  ] };
})();

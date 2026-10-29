// The monsoon's art (Prototypes/monsoon-sails/), drawn from code: a dhow as the old charts drew ships,
// from the side, its lateen sail filling and the hull riding the swell; the same with the sail furled
// in port; a white speck of sail from afar; and the wind, a streak of air passing. Rebuild with
//   node tools/packart/render.cjs tools/monsoon/art.js Prototypes/monsoon-sails
// The dhow faces east (the runtime mirrors a sprite heading west). The wind streaks come in the two
// directions the monsoon blows: toward the north-east (the summer wind) and toward the south-west.
(function () {
  'use strict';
  const B = window.Brush;
  const INK = '#2C2418', HULL = '#6B4A2B', HULL_LIGHT = '#9A6F45', SAIL = '#F6EFDF', SAIL_SHADE = '#D9CBB0', FOAM = '#FFFFFF';

  /** The dhow from the side, heading east: a curved hull low in the water, a raked mast, a lateen sail
      on its long yard; `fill` 0–1 how much the sail bellies; `roll` the hull's tilt in radians. */
  function dhow(ctx, w, h, fill, roll, furled) {
    ctx.save();
    ctx.translate(w * 0.5, h * 0.78); ctx.rotate(roll); ctx.translate(-w * 0.5, -h * 0.78);
    const wl = h * 0.78;                          // the waterline
    // the hull: sheer rising to a high stern (left) and a long bow (right)
    B.inked(ctx, (c) => {
      c.moveTo(w * 0.14, wl - h * 0.16);
      c.quadraticCurveTo(w * 0.3, wl - h * 0.03, w * 0.5, wl - h * 0.04);
      c.quadraticCurveTo(w * 0.75, wl - h * 0.05, w * 0.93, wl - h * 0.14);
      c.lineTo(w * 0.86, wl + h * 0.05);
      c.quadraticCurveTo(w * 0.5, wl + h * 0.12, w * 0.2, wl + h * 0.05);
      c.closePath();
    }, HULL, INK, h * 0.05);
    // a lighter strake along the side
    ctx.strokeStyle = HULL_LIGHT; ctx.lineWidth = h * 0.035; ctx.beginPath();
    ctx.moveTo(w * 0.2, wl - h * 0.02); ctx.quadraticCurveTo(w * 0.5, wl + h * 0.045, w * 0.85, wl - h * 0.03); ctx.stroke();
    // the mast, raked forward, and the yard
    const mx0 = w * 0.44, my0 = wl - h * 0.03, mx1 = w * 0.52, my1 = h * 0.12;
    ctx.strokeStyle = INK; ctx.lineWidth = h * 0.04; ctx.beginPath(); ctx.moveTo(mx0, my0); ctx.lineTo(mx1, my1); ctx.stroke();
    if (furled) {
      // the yard lowered along the mast, the sail gathered to it
      ctx.lineWidth = h * 0.035; ctx.beginPath(); ctx.moveTo(w * 0.3, wl - h * 0.14); ctx.lineTo(w * 0.72, h * 0.3); ctx.stroke();
      B.inked(ctx, (c) => { c.moveTo(w * 0.31, wl - h * 0.13); c.quadraticCurveTo(w * 0.5, wl - h * 0.3, w * 0.71, h * 0.31);
        c.quadraticCurveTo(w * 0.5, wl - h * 0.24, w * 0.31, wl - h * 0.1); c.closePath(); }, SAIL_SHADE, INK, h * 0.03);
    } else {
      // the yard: a long spar from low at the bow to high astern... on a dhow heading east it rises
      // toward the stern (left) — the peak aft and the tack forward and low
      const yx0 = w * 0.84, yy0 = wl - h * 0.2, yx1 = w * 0.22, yy1 = h * 0.08;
      ctx.lineWidth = h * 0.035; ctx.beginPath(); ctx.moveTo(yx0, yy0); ctx.lineTo(yx1, yy1); ctx.stroke();
      // the sail: the triangle under the yard, its foot bellying out to leeward (toward the viewer)
      const belly = h * (0.08 + 0.22 * fill);
      B.inked(ctx, (c) => {
        c.moveTo(yx0, yy0); c.lineTo(yx1, yy1);
        c.quadraticCurveTo(w * 0.42 + belly * 0.6, wl - h * 0.1 + belly * 0.3, w * 0.52, wl - h * 0.1);
        c.quadraticCurveTo(w * 0.7 + belly * 0.4, wl - h * 0.2, yx0, yy0);
        c.closePath();
      }, SAIL, INK, h * 0.035);
      // the sail's shade where it curves away
      ctx.save(); ctx.beginPath();
      ctx.moveTo(yx0, yy0); ctx.lineTo(yx1, yy1);
      ctx.quadraticCurveTo(w * 0.42 + belly * 0.6, wl - h * 0.1 + belly * 0.3, w * 0.52, wl - h * 0.1);
      ctx.quadraticCurveTo(w * 0.7 + belly * 0.4, wl - h * 0.2, yx0, yy0); c_clip(ctx);
      ctx.fillStyle = B.rgba(SAIL_SHADE, 0.35 + 0.35 * fill); ctx.beginPath();
      ctx.moveTo(yx1, yy1); ctx.quadraticCurveTo(w * 0.45, h * 0.5, w * 0.52, wl - h * 0.1); ctx.lineTo(w * 0.3, wl); ctx.lineTo(w * 0.1, h * 0.1); ctx.closePath(); ctx.fill();
      ctx.restore();
    }
    ctx.restore();
    // the water: a line of foam at the bow and the hull's shadow
    ctx.strokeStyle = B.rgba(FOAM, 0.8); ctx.lineWidth = h * 0.04; ctx.beginPath();
    ctx.moveTo(w * 0.84, wl + h * 0.06); ctx.quadraticCurveTo(w * 0.95, wl + h * 0.06, w * 0.98, wl + h * 0.01); ctx.stroke();
    ctx.strokeStyle = B.rgba('#1B3A4E', 0.35); ctx.lineWidth = h * 0.05; ctx.beginPath();
    ctx.moveTo(w * 0.22, wl + h * 0.1); ctx.quadraticCurveTo(w * 0.5, wl + h * 0.16, w * 0.84, wl + h * 0.1); ctx.stroke();
  }
  function c_clip(ctx) { ctx.clip(); }

  /** A sail from afar: a small white triangle on a bead of foam. */
  function speck(ctx, w, h) {
    B.glow(ctx, w / 2, h * 0.6, w * 0.45, '#FFFFFF', 0.5, 0);
    B.inked(ctx, (c) => { c.moveTo(w * 0.3, h * 0.72); c.lineTo(w * 0.52, h * 0.18); c.lineTo(w * 0.72, h * 0.72); c.closePath(); }, '#FBF7EE', INK, h * 0.06);
  }

  /** The wind: pale streaks passing across the picture toward (dx, dy), several at different stages so
      any moment of the cycle reads as wind. `t` the cycle 0–1. */
  function wind(ctx, w, h, t, dx, dy) {
    const r = B.rand(41);
    const L = Math.hypot(w, h);
    for (let k = 0; k < 4; k++) {
      const phase = (t + k * 0.25) % 1, off = (r() - 0.5) * 0.6 * L;
      // a streak runs from behind the picture to beyond it; its middle is at `phase` along the way
      const cx = w / 2 - dx * L * 0.7 + dx * L * 1.4 * phase - dy * off, cy = h / 2 - dy * L * 0.7 + dy * L * 1.4 * phase + dx * off;
      const len = L * (0.22 + 0.14 * r()), a = Math.sin(phase * Math.PI);
      const g = ctx.createLinearGradient(cx - dx * len, cy - dy * len, cx + dx * len, cy + dy * len);
      g.addColorStop(0, 'rgba(255,255,255,0)'); g.addColorStop(0.5, `rgba(255,255,255,${0.75 * a})`); g.addColorStop(1, 'rgba(255,255,255,0)');
      ctx.strokeStyle = g; ctx.lineWidth = h * 0.028; ctx.lineCap = 'round'; ctx.beginPath();
      // a slight wave along the streak
      for (let i = 0; i <= 12; i++) {
        const u = i / 12 - 0.5, s = Math.sin(u * 6 + phase * 6) * h * 0.012;
        const x = cx + dx * len * u * 2 - dy * s, y = cy + dy * len * u * 2 + dx * s;
        i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
      }
      ctx.stroke();
    }
  }
  const NE = [Math.SQRT1_2, -Math.SQRT1_2], SW = [-Math.SQRT1_2, Math.SQRT1_2];

  window.SHEET = { pictures: [
    { name: 'dhow', width: 44, height: 36, frames: 6, draw(ctx, w, h, i, n) { const t = i / n; dhow(ctx, w, h, 0.65 + 0.35 * Math.sin(t * 6.28), Math.sin(t * 6.28) * 0.035, false); } },
    { name: 'dhow-mid', width: 22, height: 18, frames: 6, draw(ctx, w, h, i, n) { const t = i / n; dhow(ctx, w, h, 0.65 + 0.35 * Math.sin(t * 6.28), Math.sin(t * 6.28) * 0.035, false); } },
    { name: 'dhow-furled', width: 44, height: 36, frames: 4, draw(ctx, w, h, i, n) { dhow(ctx, w, h, 0, Math.sin(i / n * 6.28) * 0.03, true); } },
    { name: 'dhow-furled-mid', width: 22, height: 18, frames: 4, draw(ctx, w, h, i, n) { dhow(ctx, w, h, 0, Math.sin(i / n * 6.28) * 0.03, true); } },
    { name: 'speck', width: 9, height: 9, draw: speck },
    { name: 'wind-ne', width: 80, height: 80, frames: 24, draw(ctx, w, h, i, n) { wind(ctx, w, h, i / n, NE[0], NE[1]); } },
    { name: 'wind-sw', width: 80, height: 80, frames: 24, draw(ctx, w, h, i, n) { wind(ctx, w, h, i / n, SW[0], SW[1]); } },
    { name: 'doorway', width: 30, height: 25, draw(ctx, w, h) { dhow(ctx, w, h, 0.8, 0.02, false); } },
  ] };
})();

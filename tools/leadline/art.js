// The lead line's art (Prototypes/lead-line/), drawn from code: HMS Challenger as the old engravings
// show her, a three-masted corvette under sail with her funnel smoking, riding the swell; her small self
// from afar; the sounding glass's weight; and the doorway. Rebuild with
//   node tools/packart/render.cjs tools/leadline/art.js Prototypes/lead-line
// She faces east (the runtime mirrors a sprite heading west). The drawing follows her rig in outline
// (three masts, square sails, one funnel between the fore and main), not any particular plate.
(function () {
  'use strict';
  const B = window.Brush;
  const INK = '#22201C', HULL = '#2F2A26', HULL_BAND = '#E9E2CF', SAIL = '#F4EDDC', SAIL_SHADE = '#D5CBB3', SMOKE = '#6E6A66', FOAM = '#FFFFFF';

  function ship(ctx, w, h, t, far) {
    const pitch = Math.sin(t * 6.28) * 0.04, heave = Math.sin(t * 6.28 + 1) * h * 0.02;
    ctx.save();
    ctx.translate(w / 2, h * 0.74 + heave); ctx.rotate(pitch); ctx.translate(-w / 2, -h * 0.74);
    const wl = h * 0.74;
    // the hull: a low black hull with a white band of gunports, a clipper bow to the right
    B.inked(ctx, (c) => {
      c.moveTo(w * 0.08, wl - h * 0.1); c.quadraticCurveTo(w * 0.5, wl - h * 0.04, w * 0.9, wl - h * 0.1);
      c.lineTo(w * 0.96, wl - h * 0.16); c.lineTo(w * 0.88, wl + h * 0.06);
      c.quadraticCurveTo(w * 0.5, wl + h * 0.1, w * 0.12, wl + h * 0.06); c.closePath();
    }, HULL, INK, h * 0.04);
    if (!far) {
      ctx.strokeStyle = HULL_BAND; ctx.lineWidth = h * 0.028; ctx.beginPath();
      ctx.moveTo(w * 0.14, wl - h * 0.03); ctx.quadraticCurveTo(w * 0.5, wl + h * 0.02, w * 0.86, wl - h * 0.04); ctx.stroke();
      ctx.fillStyle = INK;
      for (let i = 0; i < 7; i++) { ctx.fillRect(w * (0.2 + i * 0.09), wl - h * 0.045, w * 0.025, h * 0.03); }
    }
    // the bowsprit
    ctx.strokeStyle = INK; ctx.lineWidth = h * 0.03; ctx.beginPath(); ctx.moveTo(w * 0.9, wl - h * 0.14); ctx.lineTo(w * 1.0, wl - h * 0.26); ctx.stroke();
    // three masts with square sails, the mainmast tallest; the funnel between the first two
    const masts = [[0.3, 0.86], [0.52, 1.0], [0.72, 0.8]];
    ctx.lineWidth = h * 0.028;
    for (const [mx, mh] of masts) {
      const x = w * mx, top = wl - h * 0.08 - h * 0.58 * mh;
      ctx.strokeStyle = INK; ctx.beginPath(); ctx.moveTo(x, wl - h * 0.08); ctx.lineTo(x, top); ctx.stroke();
      // yards and sails, three courses, each a little narrower up
      for (let k = 0; k < 3; k++) {
        const y = top + h * 0.58 * mh * (0.12 + k * 0.28), yw = w * (0.17 - k * 0.035) * (0.8 + 0.2 * mh);
        const belly = h * 0.03 * (1 + 0.3 * Math.sin(t * 6.28 + k));
        B.inked(ctx, (c) => {
          c.moveTo(x - yw, y); c.lineTo(x + yw, y);
          c.quadraticCurveTo(x + yw * 0.9 + belly, y + h * 0.1, x + yw * 0.85, y + h * 0.17);
          c.lineTo(x - yw * 0.85, y + h * 0.17); c.quadraticCurveTo(x - yw * 0.9 + belly, y + h * 0.1, x - yw, y); c.closePath();
        }, k % 2 ? SAIL_SHADE : SAIL, INK, h * 0.022);
      }
    }
    // the funnel and its smoke, blown aft (to the left)
    const fx = w * 0.41, fy = wl - h * 0.1;
    B.inked(ctx, (c) => { c.rect(fx - w * 0.02, fy - h * 0.16, w * 0.04, h * 0.16); }, '#8A5A2E', INK, h * 0.022);
    ctx.restore();
    if (!far) {
      const r = B.rand(Math.round(t * 50) + 3);
      for (let i = 0; i < 5; i++) {
        const u = (t + i / 5) % 1;
        B.glow(ctx, fx - u * w * 0.3 + (r() - 0.5) * w * 0.04, fy - h * 0.2 - u * h * 0.22, w * (0.03 + u * 0.05), SMOKE, 0.5 * (1 - u), 0);
      }
    }
    // a bow wave and the wake
    ctx.strokeStyle = B.rgba(FOAM, 0.85); ctx.lineWidth = h * 0.03; ctx.beginPath();
    ctx.moveTo(w * 0.86, wl + h * 0.08); ctx.quadraticCurveTo(w * 0.95, wl + h * 0.09, w * 1.0, wl + h * 0.03); ctx.stroke();
    ctx.strokeStyle = B.rgba(FOAM, 0.5); ctx.beginPath();
    ctx.moveTo(w * 0.12, wl + h * 0.08); ctx.quadraticCurveTo(w * 0.04, wl + h * 0.1, w * 0.0, wl + h * 0.06); ctx.stroke();
  }

  /** The sounding weight: an iron sinker, a cylinder with its ring. */
  function weight(ctx, w, h) {
    B.inked(ctx, (c) => { c.rect(w * 0.32, h * 0.3, w * 0.36, h * 0.6); }, '#4A4A4C', INK, h * 0.06);
    ctx.strokeStyle = INK; ctx.lineWidth = h * 0.06; ctx.beginPath(); ctx.arc(w * 0.5, h * 0.22, w * 0.12, 0, 6.28); ctx.stroke();
  }

  window.SHEET = { pictures: [
    { name: 'ship', width: 64, height: 50, frames: 8, draw(ctx, w, h, i, n) { ship(ctx, w, h, i / n, false); } },
    { name: 'ship-mid', width: 32, height: 25, frames: 8, draw(ctx, w, h, i, n) { ship(ctx, w, h, i / n, false); } },
    { name: 'ship-far', width: 14, height: 11, draw(ctx, w, h) { ship(ctx, w, h, 0.2, true); } },
    { name: 'weight', width: 10, height: 14, draw: weight },
    { name: 'doorway', width: 30, height: 23, draw(ctx, w, h) { ship(ctx, w, h, 0.2, false); } },
  ] };
})();

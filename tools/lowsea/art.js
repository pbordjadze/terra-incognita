// The low sea's art (Prototypes/low-sea/): a woolly mammoth walking, tiny as the globe sees it and
// larger close in, drawn from code: a high domed head, the hump at the shoulders, the long shaggy coat
// hanging below the belly, the curved tusks and the trunk swinging; and the doorway. Rebuild with
//   node tools/packart/render.cjs tools/lowsea/art.js Prototypes/low-sea
// It faces east (the runtime mirrors a sprite heading west). An illustration after the animal's
// known shape, not any one find.
(function () {
  'use strict';
  const B = window.Brush;
  const INK = '#2A1F14', COAT = '#6B4A2A', COAT_LIGHT = '#8E6A42', TUSK = '#F1E8D2', HALO = '#F4ECD8';

  function mammoth(ctx, w, h, t, halo) {
    const draw = (c) => {
      c.lineCap = 'round'; c.lineJoin = 'round';
      const ground = h * 0.96, sway = Math.sin(t * 6.28);
      // legs: the two of one side swing together, as an elephant walks (a loose amble)
      const legs = [[0.3, 0], [0.42, 0.5], [0.66, 0.5], [0.76, 0]];
      legs.forEach(([lx, ph]) => {
        const s = Math.sin((t + ph) * 6.28) * w * 0.05;
        B.inked(c, (p) => { p.moveTo(w * lx - w * 0.04 + s * 0.3, h * 0.6); p.lineTo(w * lx + w * 0.04 + s * 0.3, h * 0.6);
          p.lineTo(w * lx + w * 0.045 + s, ground); p.lineTo(w * lx - w * 0.045 + s, ground); p.closePath(); }, COAT, INK, h * 0.04);
      });
      // the body: high at the shoulders, sloping to the rump, the coat hanging in strands below
      B.inked(c, (p) => {
        p.moveTo(w * 0.2, h * 0.72);
        p.quadraticCurveTo(w * 0.16, h * 0.4, w * 0.3, h * 0.3);
        p.quadraticCurveTo(w * 0.48, h * 0.12, w * 0.62, h * 0.2);
        p.quadraticCurveTo(w * 0.76, h * 0.26, w * 0.8, h * 0.5);
        p.quadraticCurveTo(w * 0.8, h * 0.72, w * 0.72, h * 0.74);
        for (let i = 0; i < 9; i++) { const x = w * (0.72 - i * 0.058); p.lineTo(x - w * 0.02, h * (0.78 + (i % 2) * 0.05)); p.lineTo(x - w * 0.04, h * 0.74); }
        p.closePath();
      }, COAT, INK, h * 0.045);
      // the lighter top of the coat
      c.save(); c.globalAlpha = 0.55; c.fillStyle = COAT_LIGHT; c.beginPath();
      c.moveTo(w * 0.3, h * 0.32); c.quadraticCurveTo(w * 0.48, h * 0.16, w * 0.62, h * 0.23); c.quadraticCurveTo(w * 0.5, h * 0.3, w * 0.3, h * 0.42); c.closePath(); c.fill(); c.restore();
      // the head: domed, with the ear, and the trunk hanging and swinging
      B.inked(c, (p) => { p.moveTo(w * 0.74, h * 0.34); p.quadraticCurveTo(w * 0.8, h * 0.12, w * 0.9, h * 0.26);
        p.quadraticCurveTo(w * 0.96, h * 0.4, w * 0.9, h * 0.5); p.lineTo(w * 0.78, h * 0.52); p.closePath(); }, COAT, INK, h * 0.045);
      B.inked(c, (p) => { p.moveTo(w * 0.86, h * 0.44); p.quadraticCurveTo(w * 0.98 + sway * w * 0.02, h * 0.7, w * 0.9 + sway * w * 0.05, h * 0.92);
        p.lineTo(w * 0.86 + sway * w * 0.05, h * 0.92); p.quadraticCurveTo(w * 0.9 + sway * w * 0.02, h * 0.7, w * 0.8, h * 0.5); p.closePath(); }, COAT, INK, h * 0.04);
      // the tusks: long, curving out and up
      c.strokeStyle = INK; c.lineWidth = h * 0.07; c.beginPath(); c.moveTo(w * 0.84, h * 0.5); c.quadraticCurveTo(w * 1.0, h * 0.62, w * 0.9, h * 0.38); c.stroke();
      c.strokeStyle = TUSK; c.lineWidth = h * 0.04; c.stroke();
      // the eye
      c.fillStyle = INK; c.beginPath(); c.arc(w * 0.85, h * 0.3, h * 0.025, 0, 6.28); c.fill();
    };
    if (halo) B.haloed(ctx, w, h, draw, HALO, 0.9); else draw(ctx);
  }

  window.SHEET = { pictures: [
    { name: 'mammoth', width: 48, height: 40, frames: 6, draw(ctx, w, h, i, n) { mammoth(ctx, w, h, i / n, false); } },
    { name: 'mammoth-small', width: 20, height: 17, frames: 6, draw(ctx, w, h, i, n) { mammoth(ctx, w, h, i / n, true); } },
    { name: 'mammoth-tiny', width: 10, height: 8, frames: 4, draw(ctx, w, h, i, n) { mammoth(ctx, w, h, i / n, true); } },
    { name: 'doorway', width: 24, height: 20, draw(ctx, w, h) { mammoth(ctx, w, h, 0.2, true); } },
  ] };
})();

// Small helpers for a pack's art sheet (tools/packart/render.cjs loads this before the sheet).
// Everything here draws with the atlas's own habits: the ink laid down first and wider, the fills
// over it, so only the silhouette carries a line, as an engraver's would; nothing is a photograph.
(function (global) {
  'use strict';
  /** A deterministic random: the same picture every build. */
  function rand(seed) {
    let s = (seed * 9301 + 49297) % 233280;
    return function () { s = (s * 9301 + 49297) % 233280; return s / 233280; };
  }
  /** A path from points, optionally closed, as a smooth curve through them (Catmull-Rom). */
  function curve(ctx, pts, closed) {
    const n = pts.length;
    if (n < 2) return;
    const P = (i) => pts[closed ? ((i % n) + n) % n : Math.max(0, Math.min(n - 1, i))];
    ctx.moveTo(pts[0][0], pts[0][1]);
    const last = closed ? n : n - 1;
    for (let i = 0; i < last; i++) {
      const p0 = P(i - 1), p1 = P(i), p2 = P(i + 1), p3 = P(i + 2);
      ctx.bezierCurveTo(p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6,
                        p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6, p2[0], p2[1]);
    }
    if (closed) ctx.closePath();
  }
  /** A filled shape with an outline laid under it: the ink first, wider, then the fill. */
  function inked(ctx, build, fill, ink, width) {
    ctx.save();
    ctx.beginPath(); build(ctx);
    if (ink && width) { ctx.strokeStyle = ink; ctx.lineWidth = width; ctx.stroke(); }
    if (fill) { ctx.fillStyle = fill; ctx.fill(); }
    ctx.restore();
  }
  /** A soft radial glow. */
  function glow(ctx, x, y, r, color, alphaInner, alphaOuter) {
    const g = ctx.createRadialGradient(x, y, 0, x, y, r);
    g.addColorStop(0, rgba(color, alphaInner == null ? 0.9 : alphaInner));
    g.addColorStop(1, rgba(color, alphaOuter == null ? 0 : alphaOuter));
    ctx.fillStyle = g;
    ctx.beginPath(); ctx.arc(x, y, r, 0, Math.PI * 2); ctx.fill();
  }
  /** '#RRGGBB' (or '#RRGGBBAA') with an alpha. */
  function rgba(hex, a) {
    const h = hex.replace('#', '');
    const r = parseInt(h.slice(0, 2), 16), g = parseInt(h.slice(2, 4), 16), b = parseInt(h.slice(4, 6), 16);
    return `rgba(${r}, ${g}, ${b}, ${a})`;
  }
  /** Mix two hex colours. */
  function mix(a, b, t) {
    const A = a.replace('#', ''), B = b.replace('#', '');
    const c = (i) => Math.round(parseInt(A.slice(i, i + 2), 16) * (1 - t) + parseInt(B.slice(i, i + 2), 16) * t);
    return '#' + [0, 2, 4].map((i) => c(i).toString(16).padStart(2, '0')).join('');
  }
  /** The halo every tiny figure gets, so it reads on any ground: the picture drawn once more under
      itself in the paper's tone, a little wider. `draw` must draw the figure from a clean state. */
  function haloed(ctx, w, h, draw, tone, spread) {
    tone = tone || '#F4ECD8'; spread = spread == null ? 1.1 : spread;
    const c = document.createElement('canvas'); const s = 3;
    c.width = Math.ceil(w * s); c.height = Math.ceil(h * s);
    const g = c.getContext('2d'); g.scale(s, s); g.lineCap = 'round'; g.lineJoin = 'round';
    draw(g);
    // the silhouette, toned and dilated
    const m = document.createElement('canvas'); m.width = c.width; m.height = c.height;
    const mg = m.getContext('2d');
    for (let a = 0; a < Math.PI * 2; a += Math.PI / 6) mg.drawImage(c, Math.cos(a) * spread * s, Math.sin(a) * spread * s);
    mg.globalCompositeOperation = 'source-in'; mg.fillStyle = tone; mg.fillRect(0, 0, m.width, m.height);
    ctx.save(); ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.drawImage(m, 0, 0, ctx.canvas.width, ctx.canvas.height);
    ctx.drawImage(c, 0, 0, ctx.canvas.width, ctx.canvas.height);
    ctx.restore();
  }
  global.Brush = { rand, curve, inked, glow, rgba, mix, haloed };
})(window);

// The whales' art (Prototypes/sea-of-whales/), drawn from code: an archaeocete seen from above as the
// globe sees everything, swimming with a sideways sweep of the tail; its blow from afar; its dive;
// the bones it left in the sand. Rebuild with
//   node tools/packart/render.cjs tools/whales/art.js Prototypes/sea-of-whales
// The pictures face east (the runtime mirrors a sprite heading west). They are illustrations after the
// skeletons' proportions (SOURCES.md), not reconstructions of colour or skin.
(function () {
  'use strict';
  const B = window.Brush;
  const INK = '#1C2730', BACK = '#3D5061', BELLY = '#6F8798', LIGHT = '#9FB4C1', FOAM = '#F2FAFF';
  const BONE = '#EFE7D3', BONE_INK = '#5E5240', SAND_HALO = '#F6ECD2';

  /** Half-width of the body at fraction u (0 head, 1 tail tip) for a body of `slender` (0.1 narrow, 0.3 stout). */
  function profile(u, slender) {
    const head = Math.min(1, u / 0.12) ** 0.7;             // the snout fills out quickly
    const taper = Math.pow(Math.max(0, 1 - (u - 0.18) / 0.78), 0.9); // and thins toward the tail stock
    return slender * (u < 0.18 ? head : Math.max(0.12, taper));
  }

  /** The whale from above: head at the right, tail sweeping; `phase` the swim cycle 0–1. */
  function whale(ctx, w, h, phase, slender, alpha) {
    const L = w * 0.92, x0 = w * 0.04, cy = h / 2, amp = h * 0.11;
    const spine = [];
    for (let i = 0; i <= 40; i++) {
      const u = i / 40, x = x0 + L * (1 - u);
      const sway = Math.sin(phase * Math.PI * 2 - u * 2.4) * amp * Math.pow(u, 1.6);
      spine.push([x, cy + sway, profile(u, slender) * h]);
    }
    const top = spine.map(([x, y, r]) => [x, y - r]), bottom = spine.map(([x, y, r]) => [x, y + r]).reverse();
    ctx.globalAlpha = alpha == null ? 1 : alpha;
    // the flukes, a crescent across the tail stock, seen from above
    const [tx, ty] = spine[40], span = h * 0.46;
    B.inked(ctx, (c) => {
      // two swept lobes with a notch between them, as flukes read from above
      c.moveTo(tx + w * 0.05, ty - h * 0.03);
      c.quadraticCurveTo(tx + w * 0.02, ty - span * 0.5, tx - w * 0.075, ty - span);
      c.quadraticCurveTo(tx - w * 0.055, ty - span * 0.45, tx - w * 0.02, ty - h * 0.04);
      c.lineTo(tx - w * 0.005, ty);
      c.lineTo(tx - w * 0.02, ty + h * 0.04);
      c.quadraticCurveTo(tx - w * 0.055, ty + span * 0.45, tx - w * 0.075, ty + span);
      c.quadraticCurveTo(tx + w * 0.02, ty + span * 0.5, tx + w * 0.05, ty + h * 0.03);
      c.closePath();
    }, BACK, INK, h * 0.07);
    // the flippers, a little behind the head
    const [fx, fy, fr] = spine[8];
    for (const s of [-1, 1]) {
      B.inked(ctx, (c) => { c.ellipse(fx - w * 0.01, fy + s * (fr + h * 0.06), w * 0.055, h * 0.06, s * 0.5, 0, Math.PI * 2); }, BELLY, INK, h * 0.06);
    }
    // the body
    B.inked(ctx, (c) => { B.curve(c, top.concat(bottom), true); }, BACK, INK, h * 0.08);
    // a lighter back along the spine, and the eye
    ctx.save(); ctx.beginPath(); B.curve(ctx, top.concat(bottom), true); ctx.clip();
    ctx.strokeStyle = B.rgba(LIGHT, 0.55); ctx.lineWidth = h * 0.09;
    ctx.beginPath(); B.curve(ctx, spine.slice(2, 36).map(([x, y]) => [x, y - h * 0.02])); ctx.stroke();
    ctx.restore();
    const [ex, ey] = spine[3];
    ctx.fillStyle = INK; ctx.beginPath(); ctx.arc(ex, ey - h * 0.05 * (slender / 0.17), h * 0.035, 0, Math.PI * 2); ctx.fill();
    ctx.globalAlpha = 1;
  }

  /** The surface broken round a swimming whale: a faint foam line along its back. */
  function wake(ctx, w, h, phase) {
    ctx.strokeStyle = B.rgba(FOAM, 0.35); ctx.lineWidth = h * 0.05;
    for (let k = 0; k < 2; k++) {
      ctx.beginPath();
      for (let i = 0; i <= 10; i++) {
        const u = i / 10, x = w * 0.2 + u * w * 0.7;
        const y = h / 2 + (k ? 1 : -1) * (h * 0.22 + Math.sin(phase * 6.28 + u * 9) * h * 0.03);
        i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
      }
      ctx.stroke();
    }
  }

  function blow(ctx, w, h, i, n) {
    const t = (i + 0.5) / n, cx = w / 2, cy = h * 0.6;
    B.glow(ctx, cx, cy - t * h * 0.25, w * (0.2 + 0.3 * t), FOAM, 0.9 * (1 - t * 0.45), 0);
    const r = B.rand(7 + i);
    for (let k = 0; k < 7; k++) {
      const a = -Math.PI / 2 + (r() - 0.5) * 1.6, d = h * (0.1 + 0.35 * t) * (0.6 + r() * 0.6);
      B.glow(ctx, cx + Math.cos(a) * d, cy + Math.sin(a) * d, w * (0.08 + 0.1 * t), FOAM, 0.8 * (1 - t * 0.5), 0);
    }
  }

  /** The dive: the head goes first; what is left is the tail and the rings on the water. */
  function dive(ctx, w, h, i, n, slender) {
    const t = (i + 1) / n;
    ctx.save();
    ctx.beginPath(); ctx.rect(0, 0, w * (1 - t * 0.85), h); ctx.clip();
    whale(ctx, w, h, 0.25 + t * 0.4, slender, 1 - t * 0.6);
    ctx.restore();
    ctx.strokeStyle = B.rgba(FOAM, 0.55 * (1 - t * 0.5)); ctx.lineWidth = h * 0.04;
    for (let k = 0; k < 3; k++) {
      const rr = h * (0.15 + t * 0.35) * (1 + k * 0.35);
      ctx.beginPath(); ctx.ellipse(w * 0.4, h / 2, rr * 1.6, rr * 0.55, 0, 0, Math.PI * 2); ctx.stroke();
    }
  }

  /** Bones in the sand, seen from above: skull, the spine's vertebrae, ribs, the tail's last bones. */
  function bones(ctx, w, h, seed) {
    const r = B.rand(seed || 3);
    const cy = h / 2;
    B.haloed(ctx, w, h, (c) => {
      c.lineCap = 'round'; c.lineJoin = 'round';
      // the skull: long and narrow, a triangle softened
      B.inked(c, (p) => { p.moveTo(w * 0.98, cy); p.quadraticCurveTo(w * 0.86, cy - h * 0.2, w * 0.72, cy - h * 0.14);
        p.lineTo(w * 0.72, cy + h * 0.14); p.quadraticCurveTo(w * 0.86, cy + h * 0.2, w * 0.98, cy); p.closePath(); }, BONE, BONE_INK, h * 0.05);
      // the vertebrae, a chain thinning to the tail, with a slight bend as they lie
      let x = w * 0.71;
      for (let k = 0; k < 24 && x > w * 0.03; k++) {
        const u = k / 24, size = h * (0.11 - u * 0.07), bend = Math.sin(u * 2.6) * h * 0.12;
        B.inked(c, (p) => { p.rect(x - size, cy + bend - size * 0.9, size * 1.6, size * 1.8); }, BONE, BONE_INK, h * 0.04);
        // ribs in the body's middle
        if (k > 1 && k < 10) {
          for (const s of [-1, 1]) {
            c.strokeStyle = BONE_INK; c.lineWidth = h * 0.045; c.beginPath();
            c.moveTo(x, cy + bend + s * size); c.quadraticCurveTo(x - size * 0.9, cy + bend + s * h * 0.26, x - size * 0.6 + (r() - 0.5) * size, cy + bend + s * h * (0.36 - u * 0.1));
            c.stroke();
            c.strokeStyle = BONE; c.lineWidth = h * 0.022; c.stroke();
          }
        }
        x -= size * 2.1;
      }
    }, SAND_HALO, 0.9);
  }

  /** Stranded: the whale lying on the sand, the sea gone, drying paler with a shadow under it. */
  function stranded(ctx, w, h, slender, t) {
    ctx.save(); ctx.globalAlpha = 0.35; ctx.fillStyle = '#5A4A2E';
    ctx.beginPath(); ctx.ellipse(w * 0.5, h * 0.62, w * 0.44, h * 0.3, 0, 0, Math.PI * 2); ctx.fill(); ctx.restore();
    whale(ctx, w, h, 0.1, slender, 1);
    // the sun on it: paler as it dries
    ctx.save(); ctx.globalCompositeOperation = 'source-atop'; ctx.fillStyle = B.rgba('#E8D9B8', 0.25 + 0.45 * t);
    ctx.fillRect(0, 0, w, h); ctx.restore();
  }

  window.SHEET = { pictures: [
    { name: 'basilosaurus-stranded', width: 120, height: 36, frames: 3, draw(ctx, w, h, i, n) { stranded(ctx, w, h, 0.16, i / (n - 1)); } },
    { name: 'dorudon-stranded', width: 56, height: 20, frames: 3, draw(ctx, w, h, i, n) { stranded(ctx, w, h, 0.24, i / (n - 1)); } },
    { name: 'basilosaurus', width: 120, height: 36, frames: 8, draw(ctx, w, h, i, n) { wake(ctx, w, h, i / n); whale(ctx, w, h, i / n, 0.16, 1); } },
    { name: 'basilosaurus-mid', width: 48, height: 15, frames: 8, draw(ctx, w, h, i, n) { whale(ctx, w, h, i / n, 0.17, 1); } },
    { name: 'dorudon', width: 56, height: 20, frames: 8, draw(ctx, w, h, i, n) { wake(ctx, w, h, i / n); whale(ctx, w, h, i / n, 0.24, 1); } },
    { name: 'dorudon-mid', width: 24, height: 9, frames: 8, draw(ctx, w, h, i, n) { whale(ctx, w, h, i / n, 0.25, 1); } },
    { name: 'blow', width: 16, height: 16, frames: 4, draw: blow },
    { name: 'dive', width: 120, height: 36, frames: 6, draw(ctx, w, h, i, n) { dive(ctx, w, h, i, n, 0.16); } },
    { name: 'dive-small', width: 56, height: 20, frames: 6, draw(ctx, w, h, i, n) { dive(ctx, w, h, i, n, 0.24); } },
    { name: 'bones', width: 64, height: 22, draw(ctx, w, h) { bones(ctx, w, h, 3); } },
    { name: 'bones-2', width: 44, height: 16, draw(ctx, w, h) { bones(ctx, w, h, 11); } },
    { name: 'bones-3', width: 30, height: 12, draw(ctx, w, h) { bones(ctx, w, h, 19); } },
    { name: 'doorway', width: 34, height: 13, draw(ctx, w, h) { bones(ctx, w, h, 5); } },
  ] };
})();

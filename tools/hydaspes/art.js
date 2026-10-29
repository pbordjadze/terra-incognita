// The night crossing's art (Prototypes/hydaspes/), drawn from code: the valley of the Jhelum by night
// and at dawn as pictures lying on the globe (from tools/hydaspes/terrain.js, built from the river's
// course today), and the sprites that live on them: the camps as a strategy game draws them, the
// column, the boats and hides, the soldiers wading the second channel, the horse, the phalanx, the
// chariots, Porus' line of elephants, the rain and the lightning, and the doorway. Rebuild with
//   node tools/packart/render.cjs tools/hydaspes/art.js Prototypes/hydaspes --with tools/hydaspes/terrain.js
(function () {
  'use strict';
  const B = window.Brush, T = window.TERRAIN;
  const TAU = Math.PI * 2;

  // ---- palettes: the night of the storm, and the morning after ---------------------------------------
  const NIGHT = { plain: '#141827', plain2: '#1A1F31', plain3: '#101422', hill: '#0C0F1B', hillRim: '#1E2237',
                  water: '#26242A', water2: '#2E2A2D', sheen: '#6E7382', bank: '#302E38', foam: '#8A8EA0', wood: '#0A0E16', wood2: '#121A1E',
                  kas: '#1F2436', sand: '#262636', edge: '#141827', moon: '#8E99B8' };
  const DAY = { plain: '#99917A', plain2: '#A89F86', plain3: '#89816B', hill: '#766E57', hillRim: '#B3A684',
                water: '#6B5137', water2: '#7A5E42', sheen: '#C9A070', bank: '#7E6A4E', foam: '#D9C9A8', wood: '#3C4F30', wood2: '#485E3A',
                kas: '#7E7760', sand: '#A8997A', edge: '#8F8A72', moon: '#F0C890' };
  const INK = '#15110C', FIRE = '#FFB347', FLAME = '#FFE08A', EMBER = '#E2552B', TENT = '#E8DCC2', TENT_SHADE = '#B9AC90';
  const SKIN = '#D9B48C', BRONZE = '#C9A24B', IRON = '#8A8D92', RED = '#9E2B25', HORSE = '#5A3D28', HORSE2 = '#8C6A48';
  const GREY = '#7E7F84', GREY2 = '#5E5F64', WATER_NIGHT = '#2E2C35', FOAM = '#8A8793';

  // ---- the ground: a box of the valley into a picture's pixels ------------------------------------------
  function projector(box, w, h) {
    const [W, S, E, N] = box;
    return (c) => [(c[0] - W) / (E - W) * w, (N - c[1]) / (N - S) * h];
  }
  function kmPerPx(box, w) { return (box[2] - box[0]) * 111.32 * Math.cos(32.75 * Math.PI / 180) / w; }
  function poly(ctx, pts, P) {
    ctx.beginPath();
    pts.forEach((c, i) => { const [x, y] = P(c); i ? ctx.lineTo(x, y) : ctx.moveTo(x, y); });
    ctx.closePath();
  }
  function line(ctx, pts, P) {
    ctx.beginPath();
    pts.forEach((c, i) => { const [x, y] = P(c); i ? ctx.lineTo(x, y) : ctx.moveTo(x, y); });
  }
  /** The headland's grove and the island before it, laid across the river by the builder (lon/lat). */
  function landforms() {
    const A = T.authored;
    return { head: wavy(A.headPoly, 3), isl: wavy(A.island, 5), A };
  }
  /** A polygon's edges broken into a wavering shore. */
  function wavy(pts, seed) {
    const r = B.rand(seed), out = [];
    for (let i = 0; i < pts.length; i++) {
      const a = pts[i], b = pts[(i + 1) % pts.length], n = 7;
      for (let k = 0; k < n; k++) {
        const f = k / n, j = k === 0 ? 0 : (r() - 0.5) * 0.0005;
        out.push([a[0] + (b[0] - a[0]) * f + j, a[1] + (b[1] - a[1]) * f + j * 0.8]);
      }
    }
    return out;
  }
  /** The hills' skirt: the Salt Range's foot north-west of the river, with its spur to the headland. */
  function hills() {
    const r = B.rand(77), out = [], pts = hillsPts();
    for (let i = 0; i < pts.length - 1; i++) {
      const a = pts[i], b = pts[i + 1], n = 10;
      for (let k = 0; k < n; k++) {
        const f = k / n, jitter = i < 10 ? (r() - 0.5) * 0.012 : 0;
        out.push([a[0] + (b[0] - a[0]) * f + jitter, a[1] + (b[1] - a[1]) * f + jitter * 0.7]);
      }
    }
    // (smoothed twice, so the waver is of hills, not of a saw)
    for (let pass = 0; pass < 2; pass++) for (let i = 1; i < out.length - 1; i++) out[i] = [(out[i - 1][0] + 2 * out[i][0] + out[i + 1][0]) / 4, (out[i - 1][1] + 2 * out[i][1] + out[i + 1][1]) / 4];
    return out;
  }
  function hillsPts() {
    return [[73.25, 33.02], [73.33, 32.93], [73.42, 32.86], [73.50, 32.80], [73.56, 32.78], [73.60, 32.76], [73.60, 32.74], [73.52, 32.70],
            [73.455, 32.668], [73.44, 32.6565], [73.432, 32.6585], [73.40, 32.67], [73.34, 32.70], [73.24, 32.76], [73.20, 32.84], [73.20, 33.05]];
  }
  function terrain(ctx, w, h, box, pal, night) {
    const P = projector(box, w, h), r = B.rand(box[0] * 1000 | 0), kpp = kmPerPx(box, w);
    const { head, isl } = landforms();
    const px = (km) => km / kpp;                       // kilometres into pixels
    ctx.fillStyle = pal.plain; ctx.fillRect(0, 0, w, h);
    // the plain: broad soft variation first (the land's lie), then the mottle of fields and scrub
    for (let i = 0; i < 14; i++) {
      const x = r() * w, y = r() * h, rad = (0.25 + r() * 0.35) * Math.max(w, h);
      B.glow(ctx, x, y, rad, r() < 0.5 ? pal.plain2 : pal.plain3, 0.35, 0);
    }
    const n = Math.round(w * h / (kpp > 0.006 ? 12000 : 16000));
    for (let i = 0; i < n; i++) {
      const x = r() * w, y = r() * h, rx = px(0.12 + r() * 0.5) * (kpp > 0.006 ? 1 : 0.7), ry = rx * (0.35 + r() * 0.5);
      ctx.fillStyle = B.rgba(r() < 0.5 ? pal.plain2 : pal.plain3, (kpp > 0.006 ? 0.25 : 0.1) + r() * 0.2);
      ctx.beginPath(); ctx.ellipse(x, y, rx, ry, r() * TAU, 0, TAU); ctx.fill();
    }
    // the hills' skirt: a mass with a soft foot, and the ridges as short strokes, denser upslope
    const H = hills();
    for (const [alpha, width] of [[0.12, px(2.2)], [0.18, px(1.4)], [0.3, px(0.7)]]) {
      poly(ctx, H, P); ctx.strokeStyle = B.rgba(pal.hill, alpha); ctx.lineWidth = width; ctx.lineJoin = 'round'; ctx.stroke();
    }
    poly(ctx, H, P); ctx.fillStyle = pal.hill; ctx.fill();
    ctx.save(); poly(ctx, H, P); ctx.clip();
    ctx.lineCap = 'round';
    for (let i = 0; i < (kpp > 0.006 ? 1400 : 700); i++) {
      const x = r() * w, y = r() * h, len = px(0.3 + r() * 1.2), a = -0.55 + (r() - 0.5) * 0.5;
      ctx.strokeStyle = B.rgba(pal.hillRim, night ? 0.18 + r() * 0.28 : 0.07 + r() * 0.12); ctx.lineWidth = Math.max(1, px(0.025));
      ctx.beginPath(); ctx.moveTo(x, y); ctx.quadraticCurveTo(x + Math.cos(a) * len * 0.5, y + Math.sin(a) * len * 0.5 - len * 0.12, x + Math.cos(a) * len, y + Math.sin(a) * len); ctx.stroke();
    }
    ctx.restore();
    // the torrents off the hills, running tonight
    ctx.strokeStyle = B.rgba(pal.kas, 0.7); ctx.lineWidth = Math.max(1, px(0.03)); ctx.lineCap = 'round';
    for (const s of T.streams) { line(ctx, s, P); ctx.stroke(); }
    ctx.fillStyle = B.rgba(pal.kas, 0.9); ctx.lineWidth = Math.max(1, px(0.06));
    for (const tr of (T.torrents || [])) { poly(ctx, tr, P); ctx.fill(); ctx.stroke(); }
    // the river in flood: today's bed widened by the monsoon's rise, its margin wet mud, the sandbars
    // under it; then what light there is, lying along it
    const flood = px(0.3);
    ctx.lineJoin = 'round'; ctx.lineCap = 'round';
    const bed = (width, style) => {
      ctx.strokeStyle = style; ctx.fillStyle = style; ctx.lineWidth = width;
      for (const wtr of T.water) for (const o of wtr.outer) { poly(ctx, o, P); ctx.fill(); ctx.stroke(); }
      line(ctx, T.centre, P); ctx.lineWidth = width * 1.5; ctx.stroke();
    };
    bed(flood * 2.2, B.rgba(pal.water, 0.25));
    bed(flood * 1.78, B.rgba(pal.bank, 0.85));        // the wet margin: mud the flood has just left
    bed(flood * 1.6, pal.water);
    bed(flood * 0.7, pal.water2);
    // at the bend the flood spreads wide, round the island: a wider stretch of the middle
    // (a long stretch, the widening tapered, so it reads as the river spreading, not a pool)
    const bendPx = P(T.authored.bend), near = T.centre.map((c, i) => [i, P(c)]).filter(([, q]) => Math.hypot(q[0] - bendPx[0], q[1] - bendPx[1]) < px(5)).map(([i]) => i);
    if (near.length > 3) {
      const i0 = near[0], i1 = near[near.length - 1];
      ctx.lineCap = 'round';
      for (const [width, style] of [[2.0, B.rgba(pal.water, 0.3)], [1.75, B.rgba(pal.bank, 0.85)], [1.6, pal.water], [0.9, pal.water2]]) {
        for (let i = i0; i < i1; i++) {
          const t = (i - i0) / (i1 - i0), taper = Math.pow(Math.sin(t * Math.PI), 0.6);
          const [x, y] = P(T.centre[i]), [x2, y2] = P(T.centre[i + 1]);
          ctx.strokeStyle = style; ctx.lineWidth = Math.max(flood * 0.7, px(width) * taper);
          ctx.beginPath(); ctx.moveTo(x, y); ctx.lineTo(x2, y2); ctx.stroke();
        }
      }
    }
    // the water's surface: fine streaks running with the current, the moon's light on them, and foam
    // where the flood frets at its banks
    ctx.save();
    const C = T.centre;
    const streak = (i0, len, off, width, alpha, style) => {
      ctx.strokeStyle = style; ctx.lineWidth = width; ctx.globalAlpha = alpha; ctx.lineCap = 'round';
      ctx.beginPath();
      for (let i = i0; i < Math.min(C.length - 1, i0 + len); i++) {
        const [x, y] = P(C[i]); const [x2, y2] = P(C[i + 1]);
        const dx = x2 - x, dy = y2 - y, L = Math.hypot(dx, dy) || 1;
        const t = (i - i0) / len, taper = Math.sin(t * Math.PI), o = off * (0.85 + 0.3 * taper) + Math.sin(i * 0.9 + off) * flood * 0.06;
        i === i0 ? ctx.moveTo(x - dy / L * o, y + dx / L * o) : ctx.lineTo(x - dy / L * o, y + dx / L * o);
      }
      ctx.stroke();
    };
    const many = kpp > 0.006 ? 90 : 160;
    for (let k = 0; k < many; k++) streak(Math.floor(r() * (C.length - 30)), 6 + Math.floor(r() * 24), (r() - 0.5) * flood * 1.3, Math.max(0.8, px(0.012 + r() * 0.02)), (night ? 0.16 : 0.22) + r() * 0.14, pal.sheen);
    for (let k = 0; k < many / 3; k++) streak(Math.floor(r() * (C.length - 30)), 4 + Math.floor(r() * 10), (r() < 0.5 ? -1 : 1) * flood * (0.72 + r() * 0.12), Math.max(0.8, px(0.01)), 0.35 + r() * 0.3, pal.foam);
    ctx.restore();
    // the riverine woods as mapped; the headland, its ground above the flood with the grove on it;
    // the island full of trees, a strand round it
    const trees = (pts, dense) => {
      ctx.save(); poly(ctx, pts, P); ctx.clip();
      poly(ctx, pts, P); ctx.fillStyle = B.rgba(pal.wood, dense ? 1 : 0.7); ctx.fill();
      const bb = pts.map(P), xs = bb.map((p) => p[0]), ys = bb.map((p) => p[1]);
      const x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys);
      const m = Math.min(6000, Math.round((x1 - x0) * (y1 - y0) / (px(0.05) * px(0.05) * (dense ? 2 : 5))));
      for (let i = 0; i < m; i++) {
        const x = x0 + r() * (x1 - x0), y = y0 + r() * (y1 - y0), rr = Math.max(1.2, px(0.012 + r() * 0.025));
        ctx.fillStyle = B.rgba(r() < 0.6 ? pal.wood2 : pal.wood, 0.75);
        ctx.beginPath(); ctx.arc(x, y, rr, 0, TAU); ctx.fill();
        ctx.fillStyle = B.rgba(INK, 0.5); ctx.beginPath(); ctx.arc(x + rr * 0.3, y + rr * 0.35, rr * 0.5, 0, TAU); ctx.fill();
      }
      ctx.restore();
    };
    for (const wd of T.woods) trees(wd.c, false);
    poly(ctx, head, P); ctx.fillStyle = pal.plain2; ctx.fill();
    ctx.strokeStyle = B.rgba(pal.sand, 0.8); ctx.lineWidth = Math.max(1, px(0.04)); ctx.stroke();
    trees(head, true);
    poly(ctx, isl, P); ctx.fillStyle = pal.sand; ctx.fill();
    ctx.strokeStyle = B.rgba(pal.sand, 0.9); ctx.lineWidth = Math.max(1, px(0.05)); ctx.stroke();
    trees(isl.map((c) => [c[0] + 0.0002, c[1] - 0.00025]), true);
    // the picture ends softly: a pool of night (or of morning), round, not a square laid on the globe
    const mask = document.createElement('canvas'); mask.width = w; mask.height = h;
    const mg = mask.getContext('2d');
    const g = mg.createRadialGradient(w / 2, h / 2, 0, w / 2, h / 2, 1);
    g.addColorStop(0, 'rgba(0,0,0,1)'); g.addColorStop(kpp > 0.006 ? 0.68 : 0.8, 'rgba(0,0,0,1)'); g.addColorStop(1, 'rgba(0,0,0,0)');
    mg.save(); mg.translate(w / 2, h / 2); mg.scale(w / 2, h / 2); mg.translate(-w / 2, -h / 2);
    mg.fillStyle = g; mg.fillRect(0, 0, w, h); mg.restore();
    ctx.save();
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.globalCompositeOperation = 'destination-in';
    ctx.drawImage(mask, 0, 0);
    ctx.restore();
  }
  // (the pictures are equirectangular: rows a little stretched so the ground's pixels are near square)
  const pictureHeight = (box, w) => Math.round(w * (box[3] - box[1]) / (box[2] - box[0]) / Math.cos(32.75 * Math.PI / 180));

  // ---- the small figures (a high three-quarter view, as a strategy game draws them) ------------------
  /** A foot soldier `s` tall at (x, y) his feet; `step` 0–1 swings the legs; `shield` up (over the head) or at the side. */
  function soldier(ctx, x, y, s, step, opts) {
    opts = opts || {};
    const sw = Math.sin(step * TAU) * s * 0.18;
    ctx.lineCap = 'round';
    // legs
    ctx.strokeStyle = INK; ctx.lineWidth = s * 0.12;
    ctx.beginPath(); ctx.moveTo(x, y - s * 0.4); ctx.lineTo(x - s * 0.1 + sw, y); ctx.moveTo(x, y - s * 0.4); ctx.lineTo(x + s * 0.1 - sw, y); ctx.stroke();
    // body (a tunic), the head
    B.inked(ctx, (c) => { c.moveTo(x - s * 0.18, y - s * 0.4); c.lineTo(x + s * 0.18, y - s * 0.4); c.lineTo(x + s * 0.14, y - s * 0.78); c.lineTo(x - s * 0.14, y - s * 0.78); c.closePath(); },
            opts.tunic || RED, INK, s * 0.08);
    ctx.fillStyle = BRONZE; ctx.beginPath(); ctx.arc(x, y - s * 0.86, s * 0.11, 0, TAU); ctx.fill();
    // the spear, and the shield
    ctx.strokeStyle = INK; ctx.lineWidth = s * 0.05;
    ctx.beginPath(); ctx.moveTo(x + s * 0.2, y - s * 0.1); ctx.lineTo(x + s * 0.28, y - s * 1.25); ctx.stroke();
    if (opts.shield === 'up') {
      B.inked(ctx, (c) => c.ellipse(x, y - s * 1.02, s * 0.3, s * 0.1, 0, 0, TAU), BRONZE, INK, s * 0.06);
    } else if (opts.shield !== 'none') {
      B.inked(ctx, (c) => c.ellipse(x - s * 0.2, y - s * 0.58, s * 0.14, s * 0.2, 0, 0, TAU), BRONZE, INK, s * 0.06);
    }
  }
  /** A horse `s` tall at (x, y) its hooves, with or without a rider; `gait` 0–1. Facing east (right). */
  function horse(ctx, x, y, s, gait, rider, color) {
    const g = Math.sin(gait * TAU);
    ctx.lineCap = 'round';
    ctx.strokeStyle = INK; ctx.lineWidth = s * 0.1;
    // legs
    for (const [dx, ph] of [[-0.32, 0], [-0.22, 0.5], [0.2, 0.5], [0.3, 0]]) {
      const k = Math.sin(gait * TAU + ph * TAU) * s * 0.16;
      ctx.beginPath(); ctx.moveTo(x + dx * s, y - s * 0.45); ctx.lineTo(x + dx * s + k, y); ctx.stroke();
    }
    // body, neck and head
    B.inked(ctx, (c) => { c.ellipse(x, y - s * 0.55, s * 0.42, s * 0.2, 0, 0, TAU); }, color || HORSE, INK, s * 0.08);
    B.inked(ctx, (c) => { c.moveTo(x + s * 0.3, y - s * 0.62); c.lineTo(x + s * 0.52, y - s * 0.98 + g * s * 0.04); c.lineTo(x + s * 0.62, y - s * 0.9); c.lineTo(x + s * 0.42, y - s * 0.5); c.closePath(); },
            color || HORSE, INK, s * 0.07);
    ctx.strokeStyle = INK; ctx.lineWidth = s * 0.08;
    ctx.beginPath(); ctx.moveTo(x - s * 0.42, y - s * 0.6); ctx.lineTo(x - s * 0.55, y - s * 0.25); ctx.stroke();   // the tail
    if (rider) {
      B.inked(ctx, (c) => { c.moveTo(x - s * 0.12, y - s * 0.7); c.lineTo(x + s * 0.12, y - s * 0.7); c.lineTo(x + s * 0.08, y - s * 1.05); c.lineTo(x - s * 0.08, y - s * 1.05); c.closePath(); }, RED, INK, s * 0.07);
      ctx.fillStyle = BRONZE; ctx.beginPath(); ctx.arc(x, y - s * 1.12, s * 0.1, 0, TAU); ctx.fill();
      ctx.strokeStyle = INK; ctx.lineWidth = s * 0.05;
      ctx.beginPath(); ctx.moveTo(x + s * 0.1, y - s * 0.85); ctx.lineTo(x + s * 0.45, y - s * 1.3); ctx.stroke();
    }
  }
  /** A tent `s` wide at (x, y) its foot: a wedge of cloth in the lamplight. */
  function tent(ctx, x, y, s, lit) {
    B.inked(ctx, (c) => { c.moveTo(x - s * 0.5, y); c.lineTo(x + s * 0.5, y); c.lineTo(x + s * 0.1, y - s * 0.62); c.lineTo(x - s * 0.25, y - s * 0.62); c.closePath(); },
            lit ? TENT : TENT_SHADE, INK, s * 0.07);
    ctx.fillStyle = B.rgba(INK, 0.35);
    ctx.beginPath(); ctx.moveTo(x + s * 0.5, y); ctx.lineTo(x + s * 0.1, y - s * 0.62); ctx.lineTo(x + s * 0.25, y - s * 0.3); ctx.closePath(); ctx.fill();
  }
  /** A camp fire `s` across at (x, y), flickering by `t` (0–1), with its glow on the ground. */
  function fire(ctx, x, y, s, t, glow) {
    const f = 0.85 + 0.15 * Math.sin(t * TAU * 2 + x);
    if (glow) B.glow(ctx, x, y - s * 0.1, s * 2.6 * f, FIRE, 0.42, 0);
    ctx.fillStyle = EMBER; ctx.beginPath(); ctx.ellipse(x, y, s * 0.5, s * 0.22, 0, 0, TAU); ctx.fill();
    const h = s * (0.8 + 0.3 * Math.sin(t * TAU * 3 + x * 2)), sway = Math.sin(t * TAU + x) * s * 0.15;
    B.inked(ctx, (c) => { c.moveTo(x - s * 0.3, y); c.quadraticCurveTo(x - s * 0.2 + sway, y - h * 0.6, x + sway, y - h); c.quadraticCurveTo(x + s * 0.25 + sway, y - h * 0.5, x + s * 0.3, y); c.closePath(); }, FIRE, null, 0);
    B.inked(ctx, (c) => { c.moveTo(x - s * 0.15, y); c.quadraticCurveTo(x + sway * 0.5, y - h * 0.45, x + sway * 0.6, y - h * 0.6); c.quadraticCurveTo(x + s * 0.12, y - h * 0.3, x + s * 0.15, y); c.closePath(); }, FLAME, null, 0);
  }
  /** An elephant `s` tall at (x, y) its feet, facing `dir` (1 east, -1 west, 0 toward the viewer), swaying by `t`. */
  function elephant(ctx, x, y, s, t, dir, mahout) {
    const sway = Math.sin(t * TAU + x * 0.1) * s * 0.03;
    ctx.save(); ctx.translate(x, y); if (dir < 0) ctx.scale(-1, 1);
    ctx.lineCap = 'round';
    ctx.strokeStyle = INK; ctx.lineWidth = s * 0.08;
    for (const dx of [-0.3, -0.12, 0.14, 0.3]) { ctx.beginPath(); ctx.moveTo(dx * s, -s * 0.35); ctx.lineTo(dx * s + (dx < 0 ? -sway : sway) * 0.5, 0); ctx.stroke(); }
    ctx.strokeStyle = GREY2; ctx.lineWidth = s * 0.065;
    for (const dx of [-0.3, -0.12, 0.14, 0.3]) { ctx.beginPath(); ctx.moveTo(dx * s, -s * 0.35); ctx.lineTo(dx * s + (dx < 0 ? -sway : sway) * 0.5, 0); ctx.stroke(); }
    if (dir === 0) {
      B.inked(ctx, (c) => c.ellipse(0, -s * 0.55, s * 0.3, s * 0.36, 0, 0, TAU), GREY, INK, s * 0.07);
      B.inked(ctx, (c) => c.ellipse(0, -s * 0.72, s * 0.24, s * 0.2, 0, 0, TAU), GREY, INK, s * 0.06);
      B.inked(ctx, (c) => c.ellipse(-s * 0.26, -s * 0.74, s * 0.12, s * 0.16, 0, 0, TAU), GREY2, INK, s * 0.05);
      B.inked(ctx, (c) => c.ellipse(s * 0.26, -s * 0.74, s * 0.12, s * 0.16, 0, 0, TAU), GREY2, INK, s * 0.05);
      ctx.strokeStyle = GREY; ctx.lineWidth = s * 0.1; ctx.beginPath(); ctx.moveTo(0, -s * 0.6); ctx.quadraticCurveTo(sway * 3, -s * 0.3, sway * 4, -s * 0.05); ctx.stroke();
      ctx.strokeStyle = INK; ctx.lineWidth = s * 0.02; ctx.beginPath(); ctx.moveTo(0, -s * 0.6); ctx.quadraticCurveTo(sway * 3, -s * 0.3, sway * 4, -s * 0.05); ctx.stroke();
    } else {
      B.inked(ctx, (c) => c.ellipse(-s * 0.05, -s * 0.58, s * 0.46, s * 0.3, 0, 0, TAU), GREY, INK, s * 0.07);
      B.inked(ctx, (c) => c.ellipse(s * 0.38, -s * 0.66, s * 0.2, s * 0.2, 0, 0, TAU), GREY, INK, s * 0.06);
      B.inked(ctx, (c) => c.ellipse(s * 0.3, -s * 0.72, s * 0.11, s * 0.15, 0.3, 0, TAU), GREY2, INK, s * 0.05);
      ctx.strokeStyle = GREY; ctx.lineWidth = s * 0.09; ctx.beginPath(); ctx.moveTo(s * 0.52, -s * 0.6); ctx.quadraticCurveTo(s * 0.7 + sway * 2, -s * 0.35, s * 0.66 + sway * 3, -s * 0.05); ctx.stroke();
      ctx.strokeStyle = B.rgba('#F4EDE0', 0.9); ctx.lineWidth = s * 0.04; ctx.beginPath(); ctx.moveTo(s * 0.5, -s * 0.52); ctx.lineTo(s * 0.68, -s * 0.44); ctx.stroke();   // a tusk
    }
    if (mahout) {
      ctx.fillStyle = INK; ctx.beginPath(); ctx.arc(dir === 0 ? 0 : s * 0.25, -s * 0.98, s * 0.07, 0, TAU); ctx.fill();
      B.inked(ctx, (c) => { c.rect(-s * 0.16, -s * 1.02, s * 0.32, s * 0.16); }, RED, INK, s * 0.04);
    }
    ctx.restore();
  }
  /** A chariot `s` tall with two horses, facing east. */
  function chariot(ctx, x, y, s, gait) {
    horse(ctx, x + s * 0.55, y, s * 0.8, gait, false, HORSE2);
    horse(ctx, x + s * 0.45, y + s * 0.08, s * 0.8, gait + 0.3, false, HORSE);
    B.inked(ctx, (c) => c.arc(x - s * 0.25, y - s * 0.22, s * 0.22, 0, TAU), IRON, INK, s * 0.07);
    B.inked(ctx, (c) => { c.rect(x - s * 0.5, y - s * 0.75, s * 0.42, s * 0.4); }, BRONZE, INK, s * 0.07);
    ctx.fillStyle = INK; ctx.beginPath(); ctx.arc(x - s * 0.3, y - s * 0.86, s * 0.09, 0, TAU); ctx.fill();
  }
  /** A thirty-oared galley `s` long, oars at `t`; and a raft of stuffed hides with horses standing on it. */
  function galley(ctx, x, y, s, t) {
    B.inked(ctx, (c) => { c.moveTo(x - s * 0.5, y); c.quadraticCurveTo(x, y + s * 0.12, x + s * 0.5, y - s * 0.02); c.lineTo(x + s * 0.42, y - s * 0.1); c.lineTo(x - s * 0.44, y - s * 0.1); c.closePath(); }, '#6B4A2E', INK, s * 0.03);
    ctx.strokeStyle = INK; ctx.lineWidth = s * 0.015;
    for (let i = 0; i < 8; i++) {
      const ox = x - s * 0.36 + i * s * 0.1, dip = Math.sin(t * TAU + i * 0.4) * s * 0.06;
      ctx.beginPath(); ctx.moveTo(ox, y - s * 0.08); ctx.lineTo(ox - s * 0.04, y + s * 0.1 + dip); ctx.stroke();
    }
    for (let i = 0; i < 9; i++) { ctx.fillStyle = i % 3 ? BRONZE : RED; ctx.beginPath(); ctx.arc(x - s * 0.38 + i * s * 0.095, y - s * 0.13, s * 0.025, 0, TAU); ctx.fill(); }
  }
  function raft(ctx, x, y, s, t) {
    const bob = Math.sin(t * TAU + x) * s * 0.03;
    for (let i = 0; i < 4; i++) B.inked(ctx, (c) => c.ellipse(x - s * 0.3 + i * s * 0.2, y + bob, s * 0.12, s * 0.07, 0, 0, TAU), '#A88C5E', INK, s * 0.03);
    horse(ctx, x - s * 0.1, y - s * 0.03 + bob, s * 0.32, 0, false, HORSE);
    horse(ctx, x + s * 0.18, y - s * 0.01 + bob, s * 0.3, 0.2, false, HORSE2);
  }
  /** Water with ripples, `w` by `h` at (x, y) its top left, by `t`. */
  function water(ctx, x, y, w, h, t, color) {
    ctx.fillStyle = color || WATER_NIGHT; ctx.fillRect(x, y, w, h);
    ctx.strokeStyle = B.rgba(FOAM, 0.35); ctx.lineWidth = Math.max(0.6, h * 0.02);
    for (let i = 0; i < 6; i++) {
      const yy = y + h * (0.1 + i * 0.15) + Math.sin(t * TAU + i) * h * 0.02;
      ctx.beginPath(); ctx.moveTo(x, yy);
      for (let xx = x; xx <= x + w; xx += w / 12) ctx.lineTo(xx, yy + Math.sin(xx / w * 9 + t * TAU * 2 + i) * h * 0.03);
      ctx.stroke();
    }
  }

  // ---- the camps ---------------------------------------------------------------------------------------
  /** A camp: rows of tents, horse lines, fires, pickets, a few figures walking (`walkers`), by `t`. */
  function camp(ctx, w, h, t, seed, opts) {
    const r = B.rand(seed), s = h * 0.075;
    opts = opts || {};
    // the ground it stands on, trodden
    ctx.fillStyle = B.rgba('#2A2420', 0.55); ctx.beginPath(); ctx.ellipse(w / 2, h * 0.55, w * 0.47, h * 0.4, 0, 0, TAU); ctx.fill();
    // fires first (their glow lies under everything)
    const fires = [];
    for (let i = 0; i < (opts.fires || 14); i++) fires.push([w * (0.12 + r() * 0.76), h * (0.22 + r() * 0.68), 0.6 + r() * 0.6]);
    for (const f of fires) B.glow(ctx, f[0], f[1], s * 2.2 * f[2], FIRE, 0.38, 0);
    // tents in rows, back to front
    const rows = opts.rows || 4, cols = opts.cols || 7;
    for (let j = 0; j < rows; j++) for (let i = 0; i < cols; i++) {
      if (r() < 0.12) continue;
      const x = w * (0.12 + (i + (j % 2) * 0.5) * 0.76 / cols) + (r() - 0.5) * s * 0.3, y = h * (0.3 + j * 0.62 / rows) + (r() - 0.5) * s * 0.2;
      tent(ctx, x, y, s * 1.3, opts.night !== false);
    }
    // horse lines along one side
    for (let i = 0; i < (opts.horses || 6); i++) horse(ctx, w * 0.1 + i * s * 0.9, h * 0.9, s * 0.8, (i * 0.17 + t) % 1, false, i % 2 ? HORSE : HORSE2);
    // pickets: stakes round the edge
    ctx.strokeStyle = INK; ctx.lineWidth = s * 0.08; ctx.lineCap = 'round';
    for (let i = 0; i < 26; i++) {
      const a = i / 26 * TAU, x = w / 2 + Math.cos(a) * w * 0.47, y = h * 0.56 + Math.sin(a) * h * 0.41;
      ctx.beginPath(); ctx.moveTo(x, y); ctx.lineTo(x + s * 0.1, y - s * 0.5); ctx.stroke();
    }
    for (const f of fires) fire(ctx, f[0], f[1], s * 0.5 * f[2], t + f[2], false);
    // figures walking between the fires
    const n = opts.walkers == null ? 8 : opts.walkers;
    for (let i = 0; i < n; i++) {
      const a = fires[i % fires.length], b = fires[(i * 3 + 1) % fires.length], u = (t * (i % 2 ? 1 : -1) + i / n + 1) % 1;
      soldier(ctx, a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u + s * 0.3, s * 0.9, (t * 4 + i * 0.3) % 1, { shield: 'none', tunic: i % 3 ? RED : TENT_SHADE });
    }
    if (opts.elephants) for (let i = 0; i < opts.elephants; i++) elephant(ctx, w * (0.12 + i * 0.76 / (opts.elephants - 1)), h * 0.97, s * 1.5, t + i * 0.2, 0, false);
  }

  const pictures = [];
  // ---- the ground pictures ----------------------------------------------------------------------------
  for (const [name, box] of [['ground-wide', T.wide], ['ground-close', T.close]]) {
    // (3072 pixels across: a kilometre is about 60 pixels in the wide picture and 230 in the close one,
    // more than the deepest zoom shows; 4096 cost twice the bytes for nothing seen)
    const w = 3072, h = pictureHeight(box, 3072);
    pictures.push({ name: name + '-night', width: w, height: h, scale: 1, shrink: 'posterize', draw(ctx) { terrain(ctx, w, h, box, NIGHT, true); } });
    pictures.push({ name: name + '-day', width: w, height: h, scale: 1, shrink: 'posterize', draw(ctx) { terrain(ctx, w, h, box, DAY, false); } });
  }

  // ---- the sprites ------------------------------------------------------------------------------------
  const frames = (n) => n;
  pictures.push({ name: 'camp-far', width: 12, height: 12, frames: 4, draw(ctx, w, h, i, n) {
    const t = i / n, r = B.rand(3);
    for (let k = 0; k < 5; k++) { const x = w * (0.2 + r() * 0.6), y = h * (0.2 + r() * 0.6); B.glow(ctx, x, y, 3 + Math.sin(t * TAU + k) * 0.6, FIRE, 0.9, 0); }
  } });
  pictures.push({ name: 'camp-mid', width: 40, height: 30, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n, r = B.rand(5);
    ctx.fillStyle = B.rgba('#2A2420', 0.5); ctx.beginPath(); ctx.ellipse(w / 2, h / 2, w * 0.45, h * 0.42, 0, 0, TAU); ctx.fill();
    for (let k = 0; k < 16; k++) { const x = w * (0.12 + r() * 0.76), y = h * (0.15 + r() * 0.7); B.glow(ctx, x, y, 3.2 + Math.sin(t * TAU * 2 + k) * 0.7, FIRE, 0.95, 0); }
    for (let k = 0; k < 10; k++) tent(ctx, w * (0.15 + r() * 0.7), h * (0.2 + r() * 0.65), 4, true);
  } });
  pictures.push({ name: 'camp-a', scale: 2, width: 230, height: 160, frames: 8, draw(ctx, w, h, i, n) { camp(ctx, w, h, i / n, 11, { fires: 16, rows: 4, cols: 8, horses: 7, walkers: 10 }); } });
  // the camp left behind: its fires kept burning, a few figures still walking among them
  pictures.push({ name: 'camp-decoy', scale: 2, width: 230, height: 160, frames: 8, draw(ctx, w, h, i, n) { camp(ctx, w, h, i / n, 11, { fires: 16, rows: 4, cols: 8, horses: 2, walkers: 4 }); } });
  pictures.push({ name: 'camp-p', scale: 2, width: 250, height: 170, frames: 8, draw(ctx, w, h, i, n) { camp(ctx, w, h, i / n, 23, { fires: 18, rows: 4, cols: 9, horses: 5, walkers: 8 }); } });
  // the elephants at the water, watching the far bank (toward the viewer's left, the river); by day they turn
  pictures.push({ name: 'elephants-watch', width: 180, height: 46, frames: 6, draw(ctx, w, h, i, n) {
    for (let k = 0; k < 7; k++) elephant(ctx, w * (0.08 + k * 0.14), h * 0.95, h * 0.62, i / n + k * 0.17, 0, false);
  } });
  pictures.push({ name: 'elephants-mid', width: 40, height: 10, frames: 4, draw(ctx, w, h, i, n) {
    for (let k = 0; k < 7; k++) { ctx.fillStyle = GREY; ctx.beginPath(); ctx.ellipse(w * (0.08 + k * 0.14), h * 0.5 + Math.sin(i / n * TAU + k) * 0.4, 2.4, 2, 0, 0, TAU); ctx.fill(); }
  } });
  // the column on the march: ranks of foot with their spears, horse at its head, no fires
  pictures.push({ name: 'column-far', width: 24, height: 6, frames: 4, draw(ctx, w, h, i, n) {
    // what you would see of a column in the dark from across the river: a thin line of pale points, creeping
    for (let k = 0; k < 11; k++) { const x = 1.5 + k * 2.1 + ((i + k) % 4) * 0.12; ctx.fillStyle = B.rgba('#D8CFB8', 0.55 + ((k + i) % 3) * 0.15); ctx.beginPath(); ctx.arc(x, h / 2 + Math.sin(k * 1.3) * 0.7, 0.75, 0, TAU); ctx.fill(); }
  } });
  pictures.push({ name: 'column', width: 120, height: 40, frames: 8, draw(ctx, w, h, i, n) {
    const t = i / n, s = 9;
    for (let k = 0; k < 3; k++) horse(ctx, w * 0.86 + k * 9 - k * 2, h * 0.5 + k * 7, s * 0.9, (t + k * 0.3) % 1, true);
    for (let row = 0; row < 3; row++) for (let k = 0; k < 11; k++) {
      soldier(ctx, w * 0.06 + k * 7.6 + row * 2.2, h * 0.42 + row * 8.5, s, (t + k * 0.09 + row * 0.2) % 1, { tunic: (k + row) % 2 ? RED : TENT_SHADE });
    }
  } });
  pictures.push({ name: 'column-mid', width: 60, height: 16, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    for (let row = 0; row < 2; row++) for (let k = 0; k < 12; k++) soldier(ctx, 4 + k * 4.4 + row * 1.5, h * 0.62 + row * 5, 5.2, (t + k * 0.1 + row * 0.3) % 1, { shield: 'none', tunic: (k + row) % 2 ? RED : '#E6DCC4' });
    horse(ctx, w * 0.92, h * 0.7, 5, t, true);
  } });
  // the boats and the hides pushed across
  pictures.push({ name: 'boats', width: 90, height: 40, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    galley(ctx, w * 0.35, h * 0.45, 52, t);
    raft(ctx, w * 0.75, h * 0.62, 26, t);
    raft(ctx, w * 0.18, h * 0.8, 22, t + 0.4);
    galley(ctx, w * 0.62, h * 0.88, 40, t + 0.5);
  } });
  pictures.push({ name: 'boats-mid', width: 24, height: 10, frames: 4, draw(ctx, w, h, i, n) {
    const t = i / n;
    for (let k = 0; k < 3; k++) B.inked(ctx, (c) => { c.ellipse(4 + k * 8, h * 0.5 + Math.sin(t * TAU + k) * 0.5, 3.4, 1.2, 0, 0, TAU); }, '#6B4A2E', INK, 0.5);
  } });
  // formed up on what they took for the far bank: the horse in front, the foot behind
  pictures.push({ name: 'formed', width: 110, height: 44, frames: 4, draw(ctx, w, h, i, n) {
    const t = i / n, s = 9;
    for (let k = 0; k < 6; k++) horse(ctx, w * 0.14 + k * 14, h * 0.5, s * 0.9, (t * 0.3 + k * 0.1) % 1, true);
    for (let row = 0; row < 2; row++) for (let k = 0; k < 12; k++) soldier(ctx, w * 0.06 + k * 7.6 + row * 3, h * 0.72 + row * 9, s, 0, { tunic: (k + row) % 2 ? RED : TENT_SHADE });
  } });
  // THE MOMENT: the second channel, swollen by the storm. Hand-placed, no two alike: the near rank to
  // the chest, some to the shoulder, one or two with only the shield and the crown of the head above
  // the water, a horse with just its head up, a shield gone floating; the current drags pale streaks
  // downstream from every body. The river itself is the ground picture's: only what rises out of it is here.
  function wader(ctx, x, y, s, depth, tilt, shieldTilt, shield, tunic) {
    // depth: how much of the man the water covers (0.55 chest, 0.75 shoulder, 0.92 crown); y his feet
    const top = y - s * 1.35, water = y - s * depth;
    // the current round him: streaks dragging away downstream (to the right)
    ctx.strokeStyle = B.rgba(FOAM, 0.55); ctx.lineWidth = 0.9; ctx.lineCap = 'round';
    for (let k = 0; k < 3; k++) {
      const yy = water + 1 + k * 2.2, len = s * (0.5 + k * 0.25);
      ctx.beginPath(); ctx.moveTo(x - s * 0.1 + k * 2, yy); ctx.quadraticCurveTo(x + len * 0.5, yy + 0.8, x + len, yy + (k - 1) * 0.6); ctx.stroke();
    }
    ctx.beginPath(); ctx.ellipse(x, water + 0.5, s * 0.32, s * 0.07, 0, Math.PI, TAU); ctx.strokeStyle = B.rgba(FOAM, 0.7); ctx.lineWidth = 1.1; ctx.stroke();
    ctx.save();
    ctx.beginPath(); ctx.rect(x - s, top, s * 2, water - top); ctx.clip();
    ctx.translate(x, y); ctx.rotate(tilt); ctx.translate(-x, -y);
    soldier(ctx, x, y, s, 0, { shield: 'none', tunic });
    ctx.restore();
    if (shield) {
      ctx.save(); ctx.translate(x, y - s * 1.03); ctx.rotate(shieldTilt);
      B.inked(ctx, (c) => c.ellipse(0, 0, s * 0.3, s * 0.1 + Math.abs(shieldTilt) * s * 0.18, 0, 0, TAU), BRONZE, INK, s * 0.06);
      ctx.restore();
    }
  }
  function horseHead(ctx, x, y, s, rider, color) {
    ctx.strokeStyle = B.rgba(FOAM, 0.55); ctx.lineWidth = 0.9;
    for (let k = 0; k < 3; k++) { ctx.beginPath(); ctx.moveTo(x + s * 0.2 + k * 3, y + 1 + k * 2.4); ctx.lineTo(x + s * 0.9 + k * 4, y + 2 + k * 2.4); ctx.stroke(); }
    ctx.beginPath(); ctx.ellipse(x + s * 0.45, y + 0.5, s * 0.5, s * 0.08, 0, Math.PI, TAU); ctx.strokeStyle = B.rgba(FOAM, 0.7); ctx.lineWidth = 1.1; ctx.stroke();
    ctx.save(); ctx.beginPath(); ctx.rect(x - s, y - s * 1.6, s * 2.4, s * 1.6); ctx.clip();
    // the neck rising from the water, the head thrown up
    B.inked(ctx, (c) => { c.moveTo(x + s * 0.2, y + s * 0.1); c.lineTo(x + s * 0.5, y - s * 0.6); c.lineTo(x + s * 0.66, y - s * 0.55); c.lineTo(x + s * 0.55, y + s * 0.1); c.closePath(); }, color, INK, s * 0.07);
    B.inked(ctx, (c) => { c.moveTo(x + s * 0.45, y - s * 0.62); c.lineTo(x + s * 0.9, y - s * 0.5); c.lineTo(x + s * 0.92, y - s * 0.38); c.lineTo(x + s * 0.62, y - s * 0.36); c.closePath(); }, color, INK, s * 0.06);
    ctx.fillStyle = INK; ctx.beginPath(); ctx.arc(x + s * 0.58, y - s * 0.64, s * 0.05, 0, TAU); ctx.fill();   // an eye
    if (rider) {
      B.inked(ctx, (c) => { c.moveTo(x + s * 0.15, y - s * 0.2); c.lineTo(x + s * 0.4, y - s * 0.2); c.lineTo(x + s * 0.38, y - s * 0.62); c.lineTo(x + s * 0.17, y - s * 0.62); c.closePath(); }, RED, INK, s * 0.06);
      ctx.fillStyle = BRONZE; ctx.beginPath(); ctx.arc(x + s * 0.27, y - s * 0.7, s * 0.1, 0, TAU); ctx.fill();
    }
    ctx.restore();
  }
  pictures.push({ name: 'waders', width: 250, height: 120, frames: 8, draw(ctx, w, h, i, n) {
    const t = i / n, s = 30, bob = (k) => Math.sin(t * TAU + k * 1.7) * 1.3;
    // the far ones first, deeper: a crown and a shield, a shoulder
    wader(ctx, 36, 52 + bob(1), s * 0.85, 0.92, 0.0, 0.3, true, RED);
    wader(ctx, 92, 46 + bob(2), s * 0.85, 0.8, -0.1, -0.2, true, TENT_SHADE);
    horseHead(ctx, 128, 50 + bob(3), s * 0.95, true, HORSE);
    wader(ctx, 176, 48 + bob(4), s * 0.85, 0.75, 0.12, 0.05, true, RED);
    wader(ctx, 214, 56 + bob(5), s * 0.85, 0.88, 0.0, 0.5, false, TENT_SHADE);
    // a shield gone, turning in the current
    ctx.save(); ctx.translate(232, 70 + bob(6)); ctx.rotate(0.9 + t * 0.4); B.inked(ctx, (c) => c.ellipse(0, 0, s * 0.28, s * 0.09, 0, 0, TAU), BRONZE, INK, 1.5); ctx.restore();
    // the near rank, the water to the chest, the shields over their heads
    wader(ctx, 20, 100 + bob(7), s, 0.55, -0.08, 0.15, true, RED);
    wader(ctx, 58, 104 + bob(8), s, 0.6, 0.05, -0.3, true, TENT_SHADE);
    wader(ctx, 98, 98 + bob(9), s, 0.55, 0.0, 0.0, true, RED);
    horseHead(ctx, 118, 96 + bob(10), s * 1.05, false, HORSE2);
    wader(ctx, 168, 102 + bob(11), s, 0.68, 0.22, 0.6, true, RED);      // stumbling, the shield going over
    wader(ctx, 206, 100 + bob(12), s, 0.56, -0.05, 0.1, true, TENT_SHADE);
    wader(ctx, 240, 106 + bob(13), s, 0.62, 0.0, -0.15, true, RED);
  } });
  pictures.push({ name: 'waders-mid', width: 40, height: 14, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    ctx.fillStyle = WATER_NIGHT; ctx.fillRect(0, 0, w, h);
    for (let k = 0; k < 9; k++) { const x = 3 + k * 4.2, y = 4 + Math.sin(t * TAU + k) * 0.6; ctx.fillStyle = BRONZE; ctx.beginPath(); ctx.ellipse(x, y, 1.8, 0.7, 0, 0, TAU); ctx.fill(); ctx.fillStyle = INK; ctx.beginPath(); ctx.arc(x, y + 1.6, 0.9, 0, TAU); ctx.fill(); }
  } });
  // the horse at the gallop, the Companions (Alexander's) and Coenus' regiments
  pictures.push({ name: 'cavalry', width: 110, height: 44, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    for (let row = 0; row < 3; row++) for (let k = 0; k < 6; k++) horse(ctx, w * 0.1 + k * 16 + row * 5, h * 0.5 + row * 9, 9, (t + k * 0.15 + row * 0.3) % 1, true, (k + row) % 2 ? HORSE : HORSE2);
  } });
  pictures.push({ name: 'cavalry-mid', width: 30, height: 10, frames: 4, draw(ctx, w, h, i, n) {
    for (let k = 0; k < 7; k++) { ctx.fillStyle = (k + i) % 2 ? HORSE : RED; ctx.beginPath(); ctx.ellipse(3 + k * 4, 5 + Math.sin(i / n * TAU + k) * 1.2, 2, 1.3, 0, 0, TAU); ctx.fill(); }
  } });
  pictures.push({ name: 'archers', width: 70, height: 36, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    for (let row = 0; row < 2; row++) for (let k = 0; k < 5; k++) horse(ctx, w * 0.1 + k * 13 + row * 6, h * 0.55 + row * 11, 8, (t + k * 0.2) % 1, true, HORSE2);
  } });
  // the phalanx: a block of pikes, advancing
  pictures.push({ name: 'phalanx', width: 130, height: 50, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n, s = 9;
    for (let row = 0; row < 4; row++) for (let k = 0; k < 14; k++) {
      const x = w * 0.05 + k * 8 + row * 2.5, y = h * 0.4 + row * 9;
      soldier(ctx, x, y, s, (t + k * 0.05) % 1, { tunic: TENT_SHADE });
      ctx.strokeStyle = INK; ctx.lineWidth = 0.6; ctx.beginPath(); ctx.moveTo(x + 2, y - 3); ctx.lineTo(x + 20, y - 14); ctx.stroke();   // the sarissa
    }
  } });
  pictures.push({ name: 'phalanx-mid', width: 36, height: 12, frames: 4, draw(ctx, w, h, i, n) {
    for (let row = 0; row < 2; row++) for (let k = 0; k < 10; k++) { ctx.fillStyle = TENT_SHADE; ctx.fillRect(2 + k * 3.2 + row, 3 + row * 4 + ((i + k) % 2) * 0.4, 1.8, 2.4); }
  } });
  // Porus' son's chariots, and the same bogged in the clay
  pictures.push({ name: 'chariots', width: 110, height: 40, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    for (let row = 0; row < 2; row++) for (let k = 0; k < 4; k++) chariot(ctx, w * 0.12 + k * 26 + row * 8, h * 0.55 + row * 12, 11, (t + k * 0.25) % 1);
  } });
  pictures.push({ name: 'chariots-bogged', width: 110, height: 40, frames: 4, draw(ctx, w, h, i, n) {
    const t = i / n;
    for (let row = 0; row < 2; row++) for (let k = 0; k < 4; k++) {
      ctx.save(); ctx.translate(w * 0.12 + k * 26 + row * 8, h * 0.6 + row * 12); ctx.rotate((k % 2 ? 1 : -1) * 0.18);
      chariot(ctx, 0, 0, 11, 0.5 + Math.sin(t * TAU + k) * 0.05); ctx.restore();
    }
    ctx.fillStyle = B.rgba('#3A2E22', 0.55); for (let k = 0; k < 8; k++) { ctx.beginPath(); ctx.ellipse(10 + k * 13, h * 0.9, 9, 2.5, 0, 0, TAU); ctx.fill(); }
  } });
  pictures.push({ name: 'chariots-mid', width: 30, height: 10, frames: 4, draw(ctx, w, h, i, n) {
    for (let k = 0; k < 5; k++) { ctx.fillStyle = BRONZE; ctx.fillRect(2 + k * 5.5, 3 + ((i + k) % 2) * 0.6, 3, 3); }
  } });
  // Porus' line: the elephants a plethron apart in front, the foot behind, the chariots on the wings;
  // facing the enemy (west, the viewer's left: drawn facing east and mirrored by the script's `face`),
  // then turning on the horse that came round
  // Porus' line lies across the line of advance (the builder's axis), every figure upright and facing the
  // landing: the elephants a plethron apart along the perpendicular, the foot a little behind them
  const AX = T.authored.axis;
  const A = [AX[0], -AX[1]];                       // the axis on the screen (y down)
  const PP = [-A[1], A[0]];                        // across it
  const FACE = A[0] > 0 ? -1 : 1;                  // the enemy is back along the axis
  function lineOf(ctx, w, h, t, turning) {
    const cx = w / 2, cy = h / 2;
    const at = (along, across) => [cx + A[0] * along + PP[0] * across, cy + A[1] * along + PP[1] * across];
    const order = [];
    for (let k = 0; k < 9; k++) order.push({ y: at(0, (k - 4) * 30)[1], draw: () => {
      const p = at(turning ? Math.sin(k + t * TAU) * 4 : 0, (k - 4) * 30);
      elephant(ctx, p[0], p[1] + 12, 24, t + k * 0.13, turning ? (k % 3 === 0 ? 0 : (k % 3 === 1 ? -FACE : FACE)) : FACE, true);
    } });
    for (let row = 0; row < 2; row++) for (let k = 0; k < 22; k++) {
      const p = at(22 + row * 9 + (turning ? Math.cos(k + t * TAU) * 3 : 0), (k - 10.5) * 12 + row * 4);
      order.push({ y: p[1], draw: () => soldier(ctx, p[0], p[1], 8, turning ? (t + k * 0.1) % 1 : 0, { shield: 'none', tunic: (k + row) % 2 ? '#C77B3E' : TENT_SHADE }) });
    }
    for (const sgn of [-1, 1]) { const p = at(6, sgn * 150); order.push({ y: p[1], draw: () => { ctx.save(); if (FACE < 0) { ctx.translate(p[0], 0); ctx.scale(-1, 1); ctx.translate(-p[0], 0); } chariot(ctx, p[0], p[1], 11, t); ctx.restore(); } }); }
    order.sort((a, b) => a.y - b.y).forEach((o) => o.draw());   // the nearer drawn last, as in a high view
  }
  pictures.push({ name: 'line', width: 320, height: 320, frames: 6, draw(ctx, w, h, i, n) { lineOf(ctx, w, h, i / n, false); } });
  pictures.push({ name: 'line-turning', width: 320, height: 320, frames: 6, draw(ctx, w, h, i, n) { lineOf(ctx, w, h, i / n, true); } });
  pictures.push({ name: 'line-mid', width: 60, height: 16, frames: 4, draw(ctx, w, h, i, n) {
    for (let k = 0; k < 9; k++) { ctx.fillStyle = GREY; ctx.beginPath(); ctx.ellipse(4 + k * 6.4, 10 + Math.sin(i / n * TAU + k) * 0.5, 2.6, 2.2, 0, 0, TAU); ctx.fill(); }
    for (let k = 0; k < 20; k++) { ctx.fillStyle = '#C77B3E'; ctx.fillRect(2 + k * 2.9, 2 + (k % 2), 1.4, 2); }
  } });
  // the rain, drifting over the plain's edges; the lightning over the hills
  pictures.push({ name: 'rain', width: 200, height: 240, frames: 8, scale: 2, draw(ctx, w, h, i, n) {
    const t = i / n, r = B.rand(41);
    // a curtain of rain: strokes that thin to nothing towards the field's round edge, so it drifts
    // over the plain with no edge of its own
    ctx.save(); ctx.translate(w / 2, h / 2); ctx.scale(w / 2, h / 2);
    const g = ctx.createRadialGradient(0, 0, 0, 0, 0, 1);
    g.addColorStop(0, 'rgba(120,130,160,0.16)'); g.addColorStop(0.6, 'rgba(120,130,160,0.08)'); g.addColorStop(1, 'rgba(120,130,160,0)');
    ctx.fillStyle = g; ctx.beginPath(); ctx.arc(0, 0, 1, 0, TAU); ctx.fill(); ctx.restore();
    ctx.strokeStyle = 'rgba(200,210,235,0.6)'; ctx.lineWidth = 0.8; ctx.lineCap = 'round';
    for (let k = 0; k < 220; k++) {
      const x = r() * w, y0 = (r() * h + t * h * 1.3) % h, len = 6 + r() * 10, a = 0.3;
      const d = Math.hypot((x - w / 2) / (w / 2), (y0 - h / 2) / (h / 2));
      ctx.globalAlpha = Math.pow(Math.max(0, 1 - d), 2) * (0.5 + r() * 0.5);
      ctx.beginPath(); ctx.moveTo(x, y0); ctx.lineTo(x - Math.sin(a) * len, y0 + Math.cos(a) * len); ctx.stroke();
    }
    ctx.globalAlpha = 1;
  } });
  pictures.push({ name: 'lightning', width: 400, height: 400, frames: 3, scale: 1.5, draw(ctx, w, h, i, n) {
    // a wash of light over the whole field, then the bolt's after-image
    B.glow(ctx, w / 2, h / 2, w * 0.5, '#DCE6FF', 0.5 - i * 0.14, 0);
    if (i < 2) {
      ctx.strokeStyle = 'rgba(245,248,255,0.95)'; ctx.lineWidth = 2.4 - i; ctx.lineCap = 'round';
      const r = B.rand(7 + i); let x = w * 0.5, y = h * 0.15;
      ctx.beginPath(); ctx.moveTo(x, y);
      for (let k = 0; k < 10; k++) { x += (r() - 0.5) * 34; y += 20 + r() * 16; ctx.lineTo(x, y); }
      ctx.stroke();
      B.glow(ctx, x, y, 40, '#DCE6FF', 0.5, 0);
    }
  } });
  // the camps' fires on the water: a warm light laid at the bank, flickering
  pictures.push({ name: 'fireglow', width: 160, height: 70, frames: 4, scale: 1, draw(ctx, w, h, i, n) {
    const f = 0.9 + 0.1 * Math.sin(i / n * TAU);
    ctx.save(); ctx.translate(w / 2, h / 2); ctx.scale(1, h / w);
    B.glow(ctx, 0, 0, w / 2 * f, FIRE, 0.5, 0); ctx.restore();
    ctx.strokeStyle = B.rgba(FIRE, 0.35); ctx.lineWidth = 1; ctx.lineCap = 'round';
    const r = B.rand(3 + i);
    for (let k = 0; k < 14; k++) { const x = w * (0.2 + r() * 0.6), y = h * (0.3 + r() * 0.4), len = 6 + r() * 14; ctx.beginPath(); ctx.moveTo(x - len / 2, y); ctx.lineTo(x + len / 2, y); ctx.stroke(); }
  } });
  // dawn's light: a warm wash from the east, laid over the ground as the night thins
  pictures.push({ name: 'dawn', width: 400, height: 400, scale: 1, draw(ctx, w, h) {
    const g = ctx.createRadialGradient(w * 0.9, h * 0.5, 0, w * 0.9, h * 0.5, w * 0.9);
    g.addColorStop(0, 'rgba(255,190,120,0.45)'); g.addColorStop(0.5, 'rgba(255,170,110,0.18)'); g.addColorStop(1, 'rgba(255,160,110,0)');
    ctx.fillStyle = g; ctx.beginPath(); ctx.ellipse(w / 2, h / 2, w / 2, h / 2, 0, 0, TAU); ctx.fill();
  } });
  // the doorway: a river bend at night with a camp's fires on each bank
  pictures.push({ name: 'doorway', width: 34, height: 30, draw(ctx, w, h) {
    B.haloed(ctx, w, h, (c) => {
      c.fillStyle = NIGHT.plain; c.beginPath(); c.ellipse(w / 2, h / 2, w * 0.46, h * 0.44, 0, 0, TAU); c.fill();
      c.strokeStyle = NIGHT.sheen; c.lineWidth = 3.2; c.lineCap = 'round';
      c.beginPath(); c.moveTo(w * 0.1, h * 0.28); c.quadraticCurveTo(w * 0.5, h * 0.3, w * 0.62, h * 0.55); c.quadraticCurveTo(w * 0.75, h * 0.8, w * 0.95, h * 0.78); c.stroke();
      const r = B.rand(9);
      for (let k = 0; k < 5; k++) B.glow(c, w * (0.15 + r() * 0.35), h * (0.45 + r() * 0.4), 2.6, FIRE, 1, 0);
      for (let k = 0; k < 4; k++) B.glow(c, w * (0.6 + r() * 0.3), h * (0.15 + r() * 0.3), 2.4, FIRE, 1, 0);
    }, '#F4ECD8', 1.1);
  } });

  window.SHEET = { pictures };
})();

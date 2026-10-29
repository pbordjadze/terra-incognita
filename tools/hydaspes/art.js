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
  const NIGHT = { plain: '#141827', plain2: '#181D2E', plain3: '#111526', hill: '#0C0F1B', hillRim: '#1E2237',
                  water: '#1C1B22', water2: '#232127', sheen: '#8A93AE', bank: '#0E0E16', foam: '#9AA2BC', wood: '#0A0E16', wood2: '#121A1E',
                  kas: '#1F2436', sand: '#52526A', sand2: '#3E3E52', edge: '#141827', moon: '#8E99B8' };
  const DAY = { plain: '#8C8E86', plain2: '#9A9B90', plain3: '#7C7E78', hill: '#6E7069', hillRim: '#B0AB98',
                water: '#5E4A3A', water2: '#6E5844', sheen: '#E6B98A', bank: '#5C5248', foam: '#F0DCC0', wood: '#44523C', wood2: '#505F46',
                kas: '#7E8078', sand: '#BBAF95', sand2: '#A89C84', edge: '#8C8E86', moon: '#F0C890' };
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
        const f = k / n, jitter = (r() - 0.5) * (i < 10 ? 0.012 : 0.006);
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
  // Every mark on the ground is placed in longitude and latitude from one seeded scatter over the wide
  // box, so the wide picture and the close one are the same scene at two resolutions and can meet at a
  // near-hard edge: the close picture ends in a short rim and the wide one has a matching hole, so the
  // night is never laid twice over one place (two soft rims stacked showed as a ring while the night thinned).
  const KM_LON = 111.32 * Math.cos(32.75 * Math.PI / 180), KM_LAT = 110.57;
  function scatter(seed, perKm2, box) {
    const r = B.rand(seed), area = (box[2] - box[0]) * KM_LON * (box[3] - box[1]) * KM_LAT, out = [];
    for (let i = 0, n = Math.round(area * perKm2); i < n; i++) out.push([box[0] + r() * (box[2] - box[0]), box[1] + r() * (box[3] - box[1]), r(), r(), r(), r()]);
    return out;
  }
  const SCATTER = {
    lie: scatter(101, 0.004, T.wide),        // the land's broad lie: a few soft lights and darks
    mottle: scatter(102, 0.9, T.wide),       // fields and scrub
    fine: scatter(103, 6, T.wide),           // the finer mottle, seen close
    ridges: scatter(104, 1.6, T.wide),       // the hills' ridge strokes (kept where the hills are)
    trees: scatter(105, 80, T.wide),         // trees (kept where the woods are)
    water: scatter(106, 0.5, T.wide),        // streaks of light on the water: which stretch, which offset
  };
  function insidePoly(pt, poly) {
    let inside = false;
    for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
      const xi = poly[i][0], yi = poly[i][1], xj = poly[j][0], yj = poly[j][1];
      if (((yi > pt[1]) !== (yj > pt[1])) && (pt[0] < (xj - xi) * (pt[1] - yi) / (yj - yi) + xi)) inside = !inside;
    }
    return inside;
  }
  const HILLS = hills();
  function terrain(ctx, w, h, box, pal, night) {
    const P = projector(box, w, h), kpp = kmPerPx(box, w), close = box === T.close;
    const { head, isl } = landforms();
    const px = (km) => km / kpp;                       // kilometres into pixels
    const within = (c, margin) => c[0] >= box[0] - margin && c[0] <= box[2] + margin && c[1] >= box[1] - margin && c[1] <= box[3] + margin;
    ctx.fillStyle = pal.plain; ctx.fillRect(0, 0, w, h);
    // the plain: broad soft variation first (the land's lie), then the mottle of fields and scrub
    for (const m of SCATTER.lie) if (within(m, 0.3)) { const [x, y] = P(m); B.glow(ctx, x, y, px(14 + m[2] * 18), m[3] < 0.5 ? pal.plain2 : pal.plain3, 0.35, 0); }
    for (const m of SCATTER.mottle) if (within(m, 0.02)) {
      const [x, y] = P(m), rx = px(0.12 + m[2] * 0.5), ry = rx * (0.35 + m[3] * 0.5);
      ctx.fillStyle = B.rgba(m[4] < 0.5 ? pal.plain2 : pal.plain3, 0.1 + m[5] * 0.12);
      ctx.beginPath(); ctx.ellipse(x, y, rx, ry, m[4] * TAU, 0, TAU); ctx.fill();
    }
    for (const m of SCATTER.fine) if (within(m, 0.01)) {
      const [x, y] = P(m), rx = Math.max(1, px(0.03 + m[2] * 0.12)), ry = rx * (0.4 + m[3] * 0.5);
      ctx.fillStyle = B.rgba(m[4] < 0.5 ? pal.plain2 : pal.plain3, 0.04 + m[5] * 0.08);
      ctx.beginPath(); ctx.ellipse(x, y, rx, ry, m[5] * TAU, 0, TAU); ctx.fill();
    }
    // the hills' skirt: a mass with a soft foot, and the ridges as short strokes
    for (const [alpha, width] of [[0.12, px(2.2)], [0.18, px(1.4)], [0.3, px(0.7)]]) {
      poly(ctx, HILLS, P); ctx.strokeStyle = B.rgba(pal.hill, alpha); ctx.lineWidth = width; ctx.lineJoin = 'round'; ctx.stroke();
    }
    poly(ctx, HILLS, P); ctx.fillStyle = pal.hill; ctx.fill();
    ctx.save(); poly(ctx, HILLS, P); ctx.clip();
    ctx.lineCap = 'round';
    for (const m of SCATTER.ridges) {
      if (!within(m, 0.02) || !insidePoly(m, HILLS)) continue;
      const [x, y] = P(m), len = px(0.3 + m[2] * 1.2), a = -0.55 + (m[3] - 0.5) * 0.5;
      ctx.strokeStyle = B.rgba(pal.hillRim, night ? 0.18 + m[4] * 0.28 : 0.07 + m[4] * 0.12); ctx.lineWidth = Math.max(1, px(0.025));
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
    // at the bend the flood spreads wide, round the island: a long stretch of the middle, tapered
    const bendPx = P(T.authored.bend), near = T.centre.map((c, i) => [i, P(c)]).filter(([, q]) => Math.hypot(q[0] - bendPx[0], q[1] - bendPx[1]) < px(5)).map(([i]) => i);
    if (near.length > 3) {
      const i0 = near[0], i1 = near[near.length - 1];
      for (const [width, style] of [[2.0, B.rgba(pal.water, 0.3)], [1.75, B.rgba(pal.bank, 0.85)], [1.6, pal.water], [0.9, pal.water2]]) {
        for (let i = i0; i < i1; i++) {
          const t = (i - i0) / (i1 - i0), taper = Math.pow(Math.sin(t * Math.PI), 0.6);
          const [x, y] = P(T.centre[i]), [x2, y2] = P(T.centre[i + 1]);
          ctx.strokeStyle = style; ctx.lineWidth = Math.max(flood * 0.7, px(width) * taper);
          ctx.beginPath(); ctx.moveTo(x, y); ctx.lineTo(x2, y2); ctx.stroke();
        }
      }
    }
    // the water's surface: fine streaks running with the current in the moon's light, and foam where the
    // flood frets at its banks: each from the scatter, so both pictures carry the same streaks
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
    for (const m of SCATTER.water) {
      // the nearest point of the middle to this mark decides which stretch; the mark's own numbers the rest
      let best = 0, bd = Infinity;
      for (let i = 0; i < C.length; i += 3) { const d = Math.hypot((C[i][0] - m[0]) * KM_LON, (C[i][1] - m[1]) * KM_LAT); if (d < bd) { bd = d; best = i; } }
      if (bd > 2.5 || !within(C[best], 0.15)) continue;
      streak(best, 6 + Math.floor(m[2] * 24), (m[3] - 0.5) * flood * 1.3, Math.max(0.8, px(0.01 + m[4] * 0.016)), (night ? 0.15 : 0.12) + m[5] * 0.14, pal.sheen);
      if (m[5] < 0.35) streak(best + 2, 4 + Math.floor(m[2] * 10), (m[4] < 0.5 ? -1 : 1) * flood * (0.72 + m[3] * 0.12), Math.max(0.8, px(0.01)), 0.35 + m[2] * 0.3, pal.foam);
    }
    ctx.restore();
    // the riverine woods as mapped; the headland, its ground above the flood with the grove on it;
    // the island, sand above the flood, pale against the water, a strand of foam round it, the grove in
    // its middle (so both channels read as water and the island as ground to be stranded on)
    const trees = (pts, dense) => {
      ctx.save(); poly(ctx, pts, P); ctx.clip();
      poly(ctx, pts, P); ctx.fillStyle = B.rgba(pal.wood, dense ? 0.75 : 0.4); ctx.fill();
      for (const m of SCATTER.trees) {
        if (!within(m, 0.01) || !insidePoly(m, pts) || (!dense && m[5] > 0.45)) continue;
        const [x, y] = P(m), rr = Math.max(1.2, px(0.012 + m[2] * 0.025));
        ctx.fillStyle = B.rgba(m[3] < 0.6 ? pal.wood2 : pal.wood, 0.75);
        ctx.beginPath(); ctx.arc(x, y, rr, 0, TAU); ctx.fill();
        ctx.fillStyle = B.rgba(INK, 0.5); ctx.beginPath(); ctx.arc(x + rr * 0.3, y + rr * 0.35, rr * 0.5, 0, TAU); ctx.fill();
        if (dense) { ctx.fillStyle = B.rgba(pal.hillRim, night ? 0.35 : 0.5); ctx.beginPath(); ctx.arc(x - rr * 0.3, y - rr * 0.35, rr * 0.35, 0, TAU); ctx.fill(); }
      }
      ctx.restore();
    };
    for (const wd of T.woods) trees(wavy(wd.c, 11), false);
    poly(ctx, head, P); ctx.fillStyle = pal.plain2; ctx.fill();
    ctx.strokeStyle = B.rgba(pal.sand, 0.8); ctx.lineWidth = Math.max(1, px(0.04)); ctx.stroke();
    trees(head, true);
    poly(ctx, isl, P); ctx.strokeStyle = B.rgba(pal.bank, 0.9); ctx.lineWidth = Math.max(1, px(0.09)); ctx.stroke();   // the water's edge, dark
    poly(ctx, isl, P); ctx.fillStyle = pal.sand; ctx.fill();
    ctx.strokeStyle = B.rgba(pal.sand2, 0.9); ctx.lineWidth = Math.max(1, px(0.04)); ctx.stroke();
    const mid = isl.reduce((acc, c) => [acc[0] + c[0] / isl.length, acc[1] + c[1] / isl.length], [0, 0]);
    trees(isl.map((c) => [mid[0] + (c[0] - mid[0]) * 0.62, mid[1] + (c[1] - mid[1]) * 0.62]), true);
    // the picture's edge: the wide one a pool of night (or of morning) that ends softly on the globe, with
    // a hole where the close one lies; the close one a disc with a short rim that just fills that hole
    const mask = document.createElement('canvas'); mask.width = w; mask.height = h;
    const mg = mask.getContext('2d');
    const ellipse = (bx, g) => {
      const cx = (P([bx[0], 0])[0] + P([bx[2], 0])[0]) / 2, cy = (P([0, bx[1]])[1] + P([0, bx[3]])[1]) / 2;
      const rx = (P([bx[2], 0])[0] - P([bx[0], 0])[0]) / 2, ry = (P([0, bx[1]])[1] - P([0, bx[3]])[1]) / 2;
      mg.save(); mg.translate(cx, cy); mg.scale(rx, ry); mg.fillStyle = g(mg); mg.beginPath(); mg.arc(0, 0, 1, 0, TAU); mg.fill(); mg.restore();
    };
    if (close) {
      // (opaque almost to its edge: the wide picture's hole closes under its last few per cent, so nothing shows through between them)
      ellipse(box, (c) => { const g = c.createRadialGradient(0, 0, 0, 0, 0, 1); g.addColorStop(0.97, 'rgba(0,0,0,1)'); g.addColorStop(1, 'rgba(0,0,0,0)'); return g; });
    } else {
      ellipse(box, (c) => { const g = c.createRadialGradient(0, 0, 0, 0, 0, 1); g.addColorStop(0, 'rgba(0,0,0,1)'); g.addColorStop(0.72, 'rgba(0,0,0,1)'); g.addColorStop(1, 'rgba(0,0,0,0)'); return g; });
      mg.globalCompositeOperation = 'destination-out';
      ellipse(T.close, (c) => { const g = c.createRadialGradient(0, 0, 0, 0, 0, 1); g.addColorStop(0.93, 'rgba(0,0,0,1)'); g.addColorStop(0.97, 'rgba(0,0,0,0)'); return g; });
    }
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
      ctx.strokeStyle = B.rgba('#FFF1C8', 0.8); ctx.lineWidth = s * 0.04; ctx.beginPath(); ctx.ellipse(x, y - s * 1.02, s * 0.2, s * 0.05, 0, Math.PI * 1.1, Math.PI * 1.9); ctx.stroke();
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
    const leg = (dx, k) => { ctx.beginPath(); ctx.moveTo(dx * s, -s * 0.38); ctx.lineTo(dx * s + k, 0); ctx.stroke(); };
    ctx.strokeStyle = INK; ctx.lineWidth = s * 0.17;
    for (const [dx, ph] of [[-0.3, 0], [-0.13, 0.5], [0.12, 0.5], [0.28, 0]]) leg(dx, Math.sin(t * TAU + ph * TAU) * sway * 6);
    ctx.strokeStyle = GREY2; ctx.lineWidth = s * 0.12;
    for (const [dx, ph] of [[-0.3, 0], [-0.13, 0.5], [0.12, 0.5], [0.28, 0]]) leg(dx, Math.sin(t * TAU + ph * TAU) * sway * 6);
    if (dir === 0) {
      B.inked(ctx, (c) => c.ellipse(0, -s * 0.55, s * 0.3, s * 0.36, 0, 0, TAU), GREY, INK, s * 0.07);
      B.inked(ctx, (c) => c.ellipse(0, -s * 0.72, s * 0.24, s * 0.2, 0, 0, TAU), GREY, INK, s * 0.06);
      B.inked(ctx, (c) => c.ellipse(-s * 0.26, -s * 0.74, s * 0.12, s * 0.16, 0, 0, TAU), GREY2, INK, s * 0.05);
      B.inked(ctx, (c) => c.ellipse(s * 0.26, -s * 0.74, s * 0.12, s * 0.16, 0, 0, TAU), GREY2, INK, s * 0.05);
      ctx.strokeStyle = GREY; ctx.lineWidth = s * 0.1; ctx.beginPath(); ctx.moveTo(0, -s * 0.6); ctx.quadraticCurveTo(sway * 3, -s * 0.3, sway * 4, -s * 0.05); ctx.stroke();
    } else {
      // the body: a high domed back sloping to the rump, the head lower, the ear a flap, the trunk down
      B.inked(ctx, (c) => {
        c.moveTo(-s * 0.48, -s * 0.5); c.quadraticCurveTo(-s * 0.5, -s * 0.95, -s * 0.05, -s * 0.98); c.quadraticCurveTo(s * 0.3, -s * 0.98, s * 0.42, -s * 0.75);
        c.lineTo(s * 0.42, -s * 0.4); c.quadraticCurveTo(0, -s * 0.3, -s * 0.46, -s * 0.36); c.closePath();
      }, GREY, INK, s * 0.07);
      B.inked(ctx, (c) => { c.moveTo(s * 0.3, -s * 0.9); c.quadraticCurveTo(s * 0.62, -s * 0.98, s * 0.66, -s * 0.7); c.quadraticCurveTo(s * 0.66, -s * 0.5, s * 0.5, -s * 0.44); c.quadraticCurveTo(s * 0.34, -s * 0.5, s * 0.3, -s * 0.7); c.closePath(); }, GREY, INK, s * 0.06);
      B.inked(ctx, (c) => c.ellipse(s * 0.3, -s * 0.72, s * 0.12, s * 0.17, 0.2, 0, TAU), GREY2, INK, s * 0.05);   // the ear
      ctx.strokeStyle = INK; ctx.lineWidth = s * 0.12; ctx.beginPath(); ctx.moveTo(s * 0.6, -s * 0.55); ctx.quadraticCurveTo(s * 0.74 + sway * 2, -s * 0.3, s * 0.68 + sway * 3, -s * 0.04); ctx.stroke();
      ctx.strokeStyle = GREY; ctx.lineWidth = s * 0.08; ctx.beginPath(); ctx.moveTo(s * 0.6, -s * 0.55); ctx.quadraticCurveTo(s * 0.74 + sway * 2, -s * 0.3, s * 0.68 + sway * 3, -s * 0.04); ctx.stroke();
      ctx.strokeStyle = '#F4EDE0'; ctx.lineWidth = s * 0.045; ctx.beginPath(); ctx.moveTo(s * 0.56, -s * 0.5); ctx.quadraticCurveTo(s * 0.7, -s * 0.46, s * 0.76, -s * 0.36); ctx.stroke();   // the tusk
      ctx.fillStyle = INK; ctx.beginPath(); ctx.arc(s * 0.52, -s * 0.76, s * 0.035, 0, TAU); ctx.fill();   // the eye
      ctx.strokeStyle = GREY2; ctx.lineWidth = s * 0.05; ctx.beginPath(); ctx.moveTo(-s * 0.48, -s * 0.6); ctx.lineTo(-s * 0.56, -s * 0.3); ctx.stroke();   // the tail
    }
    if (mahout) {
      ctx.fillStyle = INK; ctx.beginPath(); ctx.arc(dir === 0 ? 0 : s * 0.1, -s * 1.08, s * 0.07, 0, TAU); ctx.fill();
      B.inked(ctx, (c) => { c.rect(-s * 0.2, -s * 1.08, s * 0.4, s * 0.14); }, RED, INK, s * 0.04);
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
    // the fires: hot points, each with a small halo on the ground (not a blur over the camp)
    const fires = [];
    for (let i = 0; i < (opts.fires || 14); i++) fires.push([w * (0.12 + r() * 0.76), h * (0.22 + r() * 0.68), 0.6 + r() * 0.6]);
    for (const f of fires) B.glow(ctx, f[0], f[1], s * 1.3 * f[2], FIRE, 0.5, 0);
    // tents in staggered rows, dimmed to the night, their fire-side faces lit
    const rows = opts.rows || 4, cols = opts.cols || 7, tents = [];
    for (let j = 0; j < rows; j++) for (let i = 0; i < cols; i++) {
      if (r() < 0.15) continue;
      const x = w * (0.12 + (i + (j % 2) * 0.5) * 0.76 / cols) + (r() - 0.5) * s * 0.8, y = h * (0.3 + j * 0.62 / rows) + (r() - 0.5) * s * 0.5;
      tents.push([x, y]);
    }
    tents.sort((a, b) => a[1] - b[1]);
    for (const [x, y] of tents) {
      let near = Infinity, side = 1;
      for (const f of fires) { const d = Math.hypot(f[0] - x, f[1] - y); if (d < near) { near = d; side = f[0] < x ? -1 : 1; } }
      const lit = Math.max(0, 1 - near / (s * 3.2));
      B.inked(ctx, (c) => { c.moveTo(x - s * 0.65, y); c.lineTo(x + s * 0.65, y); c.lineTo(x + s * 0.13, y - s * 0.8); c.lineTo(x - s * 0.32, y - s * 0.8); c.closePath(); },
              B.mix('#6E6558', TENT, 0.25 + lit * 0.75), INK, s * 0.07);
      // the shaded face away from the fire
      ctx.fillStyle = B.rgba(INK, 0.3 + (1 - lit) * 0.2);
      ctx.beginPath();
      if (side > 0) { ctx.moveTo(x - s * 0.65, y); ctx.lineTo(x - s * 0.32, y - s * 0.8); ctx.lineTo(x - s * 0.1, y - s * 0.4); ctx.closePath(); }
      else { ctx.moveTo(x + s * 0.65, y); ctx.lineTo(x + s * 0.13, y - s * 0.8); ctx.lineTo(x + s * 0.32, y - s * 0.4); ctx.closePath(); }
      ctx.fill();
    }
    // horse lines along one side
    for (let i = 0; i < (opts.horses || 6); i++) horse(ctx, w * 0.1 + i * s * 0.9, h * 0.9, s * 0.8, (i * 0.17 + t) % 1, false, i % 2 ? HORSE : HORSE2);
    // pickets: stakes round the edge
    ctx.strokeStyle = INK; ctx.lineWidth = s * 0.08; ctx.lineCap = 'round';
    for (let i = 0; i < 26; i++) {
      const a = i / 26 * TAU, x = w / 2 + Math.cos(a) * w * 0.47, y = h * 0.56 + Math.sin(a) * h * 0.41;
      ctx.beginPath(); ctx.moveTo(x, y); ctx.lineTo(x + s * 0.1, y - s * 0.5); ctx.stroke();
    }
    for (const f of fires) { fire(ctx, f[0], f[1], s * 0.42 * f[2], t + f[2], false); ctx.fillStyle = B.rgba('#FFF6D0', 0.9); ctx.beginPath(); ctx.arc(f[0], f[1] - s * 0.12, s * 0.07, 0, TAU); ctx.fill(); }
    // figures walking between the fires
    const n = opts.walkers == null ? 8 : opts.walkers;
    for (let i = 0; i < n; i++) {
      const a = fires[i % fires.length], b = fires[(i * 3 + 1) % fires.length], u = (t * (i % 2 ? 1 : -1) + i / n + 1) % 1;
      soldier(ctx, a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u + s * 0.3, s * 0.9, (t * 4 + i * 0.3) % 1, { shield: 'none', tunic: i % 3 ? RED : TENT_SHADE });
    }
  }

  const pictures = [];
  // ---- the ground pictures ----------------------------------------------------------------------------
  for (const [name, box] of [['ground-wide', T.wide], ['ground-close', T.close]]) {
    // (the wide picture 3780 pixels across, 16 m a pixel, its rows just under the globe's limit of 4096;
    // the close one 3072, 4.5 m: more than the deepest zoom shows)
    const w = box === T.wide ? 3780 : 3072, h = pictureHeight(box, w);
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
    for (let k = 0; k < 16; k++) { const x = w * (0.12 + r() * 0.76), y = h * (0.15 + r() * 0.7); B.glow(ctx, x, y, 2.2 + Math.sin(t * TAU * 2 + k) * 0.5, FIRE, 0.95, 0); }
    for (let k = 0; k < 10; k++) tent(ctx, w * (0.15 + r() * 0.7), h * (0.2 + r() * 0.65), 4, true);
  } });
  pictures.push({ name: 'camp-a', scale: 2, width: 230, height: 160, frames: 8, draw(ctx, w, h, i, n) { camp(ctx, w, h, i / n, 11, { fires: 16, rows: 4, cols: 8, horses: 7, walkers: 10 }); } });
  // the camp left behind: its fires kept burning, a few figures still walking among them
  pictures.push({ name: 'camp-decoy', scale: 2, width: 230, height: 160, frames: 8, draw(ctx, w, h, i, n) { camp(ctx, w, h, i / n, 11, { fires: 16, rows: 4, cols: 8, horses: 2, walkers: 4 }); } });
  pictures.push({ name: 'camp-p', scale: 2, width: 250, height: 170, frames: 8, draw(ctx, w, h, i, n) { camp(ctx, w, h, i / n, 23, { fires: 18, rows: 4, cols: 9, horses: 5, walkers: 8 }); } });
  // the elephants at the water, watching the far bank (toward the viewer's left, the river); by day they turn
  pictures.push({ name: 'elephants-watch', width: 190, height: 44, frames: 8, draw(ctx, w, h, i, n) {
    const t = i / n;
    for (let k = 0; k < 7; k++) {
      const x = w * (0.1 + k * 0.133), raise = k === 3 && i >= 2 && i <= 5 ? 1 : 0;
      ctx.save(); ctx.translate(x, 0); ctx.scale(-1, 1); ctx.translate(-x, 0);      // they face west, towards the far camp
      elephant(ctx, x, h * 0.95 + Math.sin(t * TAU + k) * 0.4, h * 0.66, t + k * 0.17, 1, false);
      if (raise) { ctx.strokeStyle = GREY; ctx.lineWidth = h * 0.06; ctx.lineCap = 'round'; ctx.beginPath(); ctx.moveTo(x + h * 0.34, h * 0.55); ctx.quadraticCurveTo(x + h * 0.5, h * 0.25, x + h * 0.42, h * 0.08); ctx.stroke(); }
      ctx.restore();
    }
  } });
  pictures.push({ name: 'elephants-mid', width: 40, height: 10, frames: 4, draw(ctx, w, h, i, n) {
    for (let k = 0; k < 7; k++) { ctx.fillStyle = GREY; ctx.beginPath(); ctx.ellipse(w * (0.08 + k * 0.14), h * 0.5 + Math.sin(i / n * TAU + k) * 0.4, 2.4, 2, 0, 0, TAU); ctx.fill(); }
  } });
  // the column on the march: ranks of foot with their spears, horse at its head, no fires
  pictures.push({ name: 'column-far', width: 24, height: 6, frames: 4, draw(ctx, w, h, i, n) {
    // what you would see of a column in the dark from across the river: a thin line of pale points, creeping
    for (let k = 0; k < 11; k++) { const x = 1.5 + k * 2.1 + ((i + k) % 4) * 0.12; ctx.fillStyle = B.rgba('#D8CFB8', 0.55 + ((k + i) % 3) * 0.15); ctx.beginPath(); ctx.arc(x, h / 2 + Math.sin(k * 1.3) * 0.7, 0.75, 0, TAU); ctx.fill(); }
  } });
  pictures.push({ name: 'column', width: 190, height: 34, frames: 8, draw(ctx, w, h, i, n) {
    // a file, not a block: two ranks, the horse at its head
    const t = i / n, s = 11;
    for (let k = 0; k < 3; k++) horse(ctx, w * 0.88 + k * 7 - k * 2, h * 0.62 + k * 5, s * 0.9, (t + k * 0.3) % 1, true);
    for (let row = 0; row < 2; row++) for (let k = 0; k < 17; k++) {
      soldier(ctx, w * 0.04 + k * 9.2 + row * 3, h * 0.6 + row * 9, s, (t + k * 0.09 + row * 0.2) % 1, { tunic: (k + row) % 2 ? RED : TENT_SHADE });
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
  pictures.push({ name: 'waders', width: 230, height: 96, frames: 8, draw(ctx, w, h, i, n) {
    const t = i / n, s = 12, r = B.rand(29);
    // the water each body stands in: the river's own tone, laid over the figure's lower half with a soft
    // edge, so what shows is what rises out of the flood; the current drags a pale streak from each
    const dip = (x, y, wide) => {
      const g = ctx.createRadialGradient(x, y, 0, x, y, wide);
      g.addColorStop(0, B.rgba('#363034', 0.95)); g.addColorStop(0.7, B.rgba('#363034', 0.9)); g.addColorStop(1, B.rgba('#363034', 0));
      ctx.fillStyle = g; ctx.beginPath(); ctx.ellipse(x, y + wide * 0.35, wide, wide * 0.55, 0, 0, TAU); ctx.fill();
    };
    const items = [];
    // the ones not yet in: dry on the island's shore (the picture's left end lies on the island)
    for (let k = 0; k < 5; k++) {
      const x = 6 + k * 9 + (r() - 0.5) * 4, y = 40 + k * 12 + (r() - 0.5) * 6;
      items.push({ y, draw: () => soldier(ctx, x, y, s, (t + k * 0.2) % 1, { tunic: k % 2 ? RED : TENT_SHADE }) });
    }
    for (let row = 0; row < 3; row++) for (let k = 2; k < 13; k++) {
      if (r() < 0.12) continue;
      const x = 8 + k * 17.5 + row * 5 + (r() - 0.5) * 6, y = 34 + row * 22 + (r() - 0.5) * 7 + Math.sin(t * TAU + k * 1.7 + row) * 1.2;
      const depth = row === 2 ? 0.52 + r() * 0.1 : row === 1 ? 0.6 + r() * 0.15 : 0.7 + r() * 0.22;   // the far rank deeper
      items.push({ y, draw: () => {
        const top = y - s * 1.4, water = y - s * depth, tilt = (r() - 0.5) * 0.3;
        ctx.save(); ctx.beginPath(); ctx.rect(x - s, top, s * 2, water - top + s * 0.3); ctx.clip();
        ctx.translate(x, y); ctx.rotate(tilt); ctx.translate(-x, -y);
        soldier(ctx, x, y, s, 0, { shield: r() < 0.9 ? 'up' : 'none', tunic: (k + row) % 2 ? RED : TENT_SHADE });
        ctx.restore();
        dip(x, water + s * 0.05, s * 0.75);
        ctx.strokeStyle = B.rgba(FOAM, 0.5); ctx.lineWidth = 0.7; ctx.lineCap = 'round';
        ctx.beginPath(); ctx.moveTo(x + s * 0.2, water + 1); ctx.quadraticCurveTo(x + s * 0.9, water + 1.5, x + s * 1.5, water + 0.5); ctx.stroke();
      } });
    }
    for (let k = 0; k < 4; k++) {
      const x = 30 + k * 55 + (r() - 0.5) * 10, y = 60 + (r() - 0.5) * 20 + Math.sin(t * TAU + k * 2) * 1.5;
      items.push({ y, draw: () => { horseHead(ctx, x, y, s * 1.15, k % 2 === 0, k % 2 ? HORSE2 : HORSE); dip(x + s * 0.55, y + s * 0.1, s * 0.8); } });
    }
    items.sort((a, b) => a.y - b.y).forEach((o) => o.draw());
    // a shield gone, turning in the current
    ctx.save(); ctx.translate(w - 14, 50 + Math.sin(t * TAU) * 1.5); ctx.rotate(0.7 + t * 0.6); B.inked(ctx, (c) => c.ellipse(0, 0, s * 0.3, s * 0.11, 0, 0, TAU), BRONZE, INK, 0.8); ctx.restore();
  } });
  pictures.push({ name: 'waders-mid', width: 40, height: 14, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    ctx.fillStyle = WATER_NIGHT; ctx.fillRect(0, 0, w, h);
    for (let k = 0; k < 9; k++) { const x = 3 + k * 4.2, y = 4 + Math.sin(t * TAU + k) * 0.6; ctx.fillStyle = BRONZE; ctx.beginPath(); ctx.ellipse(x, y, 1.8, 0.7, 0, 0, TAU); ctx.fill(); ctx.fillStyle = INK; ctx.beginPath(); ctx.arc(x, y + 1.6, 0.9, 0, TAU); ctx.fill(); }
  } });
  // the horse at the gallop, the Companions (Alexander's) and Coenus' regiments
  /** The dust a body of horse raises: a plume trailing behind (to the left, the picture facing east). */
  function dust(ctx, x, y, w, h, t, seed) {
    const r = B.rand(seed);
    for (let k = 0; k < 7; k++) {
      const u = (t + k / 7) % 1, px = x - u * w, py = y - u * h * 0.5 + (r() - 0.5) * h * 0.3;
      B.glow(ctx, px, py, h * (0.25 + u * 0.5), '#B8A98C', 0.28 * (1 - u), 0);
    }
  }
  pictures.push({ name: 'cavalry', width: 130, height: 44, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    for (let row = 0; row < 3; row++) dust(ctx, w * 0.3, h * 0.62 + row * 9, w * 0.3, h * 0.6, t + row * 0.3, 5 + row);
    for (let row = 0; row < 3; row++) for (let k = 0; k < 6; k++) horse(ctx, w * 0.25 + k * 16 + row * 5, h * 0.5 + row * 9, 9, (t + k * 0.15 + row * 0.3) % 1, true, (k + row) % 2 ? HORSE : HORSE2);
  } });
  pictures.push({ name: 'cavalry-mid', width: 30, height: 10, frames: 4, draw(ctx, w, h, i, n) {
    for (let k = 0; k < 7; k++) { ctx.fillStyle = (k + i) % 2 ? HORSE : RED; ctx.beginPath(); ctx.ellipse(3 + k * 4, 5 + Math.sin(i / n * TAU + k) * 1.2, 2, 1.3, 0, 0, TAU); ctx.fill(); }
  } });
  pictures.push({ name: 'archers', width: 86, height: 36, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    for (let row = 0; row < 2; row++) dust(ctx, w * 0.3, h * 0.65 + row * 11, w * 0.3, h * 0.6, t + row * 0.4, 9 + row);
    for (let row = 0; row < 2; row++) for (let k = 0; k < 5; k++) horse(ctx, w * 0.27 + k * 13 + row * 6, h * 0.55 + row * 11, 8, (t + k * 0.2) % 1, true, HORSE2);
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
  pictures.push({ name: 'chariots', width: 130, height: 40, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    for (let row = 0; row < 2; row++) dust(ctx, w * 0.3, h * 0.7 + row * 12, w * 0.3, h * 0.6, t + row * 0.5, 13 + row);
    for (let row = 0; row < 2; row++) for (let k = 0; k < 4; k++) chariot(ctx, w * 0.26 + k * 26 + row * 8, h * 0.55 + row * 12, 11, (t + k * 0.25) % 1);
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
    // curtains of rain drifting over the plain: soft slanting bands with no edge of their own, and a few
    // longer threads in them
    ctx.save(); ctx.translate(w / 2, h / 2); ctx.scale(w / 2, h / 2);
    const g = ctx.createRadialGradient(0, 0, 0, 0, 0, 1);
    g.addColorStop(0, 'rgba(130,140,170,0.14)'); g.addColorStop(0.7, 'rgba(130,140,170,0.06)'); g.addColorStop(1, 'rgba(130,140,170,0)');
    ctx.fillStyle = g; ctx.beginPath(); ctx.arc(0, 0, 1, 0, TAU); ctx.fill(); ctx.restore();
    const falloff = (x, y) => Math.pow(Math.max(0, 1 - Math.hypot((x - w / 2) / (w / 2), (y - h / 2) / (h / 2))), 2);
    for (let k = 0; k < 7; k++) {
      // a band: a slanting strip of faint light, drifting with the frames
      const x0 = ((k / 7 + t * 0.35) % 1) * w * 1.4 - w * 0.2, width = 10 + r() * 16;
      for (let y = 0; y < h; y += 4) {
        const x = x0 - y * 0.32, a = falloff(x, y) * 0.16;
        if (a <= 0.005) continue;
        ctx.fillStyle = B.rgba('#C8D0E6', a); ctx.fillRect(x - width / 2, y, width, 4);
      }
    }
    ctx.strokeStyle = 'rgba(205,215,240,0.5)'; ctx.lineWidth = 1.1; ctx.lineCap = 'round';
    for (let k = 0; k < 90; k++) {
      const x = r() * w, y0 = (r() * h + t * h * 1.3) % h, len = 10 + r() * 16;
      ctx.globalAlpha = falloff(x, y0) * (0.35 + r() * 0.5);
      ctx.beginPath(); ctx.moveTo(x, y0); ctx.lineTo(x - len * 0.32, y0 + len); ctx.stroke();
    }
    ctx.globalAlpha = 1;
  } });
  pictures.push({ name: 'lightning', width: 400, height: 400, frames: 3, scale: 1.5, draw(ctx, w, h, i, n) {
    // a wash of light over the whole field, then the bolt's after-image
    // (a faint wash: a flash is brief, and the bolt is what the eye keeps; a strong disc of light read as a
    // moon laid on the ground)
    const g = ctx.createRadialGradient(w / 2, h * 0.4, 0, w / 2, h * 0.4, w * 0.5);
    g.addColorStop(0, B.rgba('#DCE6FF', 0.2 - i * 0.06)); g.addColorStop(0.5, B.rgba('#DCE6FF', 0.07 - i * 0.02)); g.addColorStop(1, B.rgba('#DCE6FF', 0));
    ctx.fillStyle = g; ctx.beginPath(); ctx.arc(w / 2, h * 0.4, w * 0.5, 0, TAU); ctx.fill();
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
    // the fires on the water: long broken streaks of their light, running with the current, flickering
    const r = B.rand(3), t = i / n;
    ctx.save(); ctx.translate(w / 2, h / 2); ctx.scale(1, h / w);
    B.glow(ctx, 0, 0, w / 2, FIRE, 0.16, 0); ctx.restore();
    ctx.lineCap = 'round';
    for (let k = 0; k < 26; k++) {
      const x = w * (0.08 + r() * 0.84), y = h * (0.15 + r() * 0.7), len = 8 + r() * 22, a = 0.5 + r() * 0.5, ph = r();
      const f = 0.7 + 0.3 * Math.sin((t + ph) * TAU);
      ctx.strokeStyle = B.rgba(k % 3 ? FIRE : FLAME, a * f * 0.7); ctx.lineWidth = 1 + r() * 1.4;
      ctx.beginPath(); ctx.moveTo(x - len / 2, y); ctx.lineTo(x + len / 2, y + (r() - 0.5) * 2); ctx.stroke();
    }
  } });
  // dawn's light: a warm wash from the east, laid over the ground as the night thins
  pictures.push({ name: 'dawn', width: 400, height: 400, scale: 1, draw(ctx, w, h) {
    const g = ctx.createRadialGradient(w * 0.9, h * 0.5, 0, w * 0.9, h * 0.5, w * 0.9);
    g.addColorStop(0, 'rgba(255,190,120,0.45)'); g.addColorStop(0.5, 'rgba(255,170,110,0.18)'); g.addColorStop(1, 'rgba(255,160,110,0)');
    ctx.fillStyle = g; ctx.beginPath(); ctx.ellipse(w / 2, h / 2, w / 2, h / 2, 0, 0, TAU); ctx.fill();
  } });
  // the doorway: no badge, no glow, nothing that says "tap here": an elephant standing at the water on
  // the Jhelum's bank, ordinary life that doesn't quite belong on this bank today (it faces east as
  // drawn; the stage mirrors it as it stirs along the bank)
  pictures.push({ name: 'doorway', width: 30, height: 20, frames: 2, draw(ctx, w, h, i, n) {
    B.haloed(ctx, w, h, (c) => { c.lineCap = 'round'; c.lineJoin = 'round'; elephant(c, w * 0.5, h * 0.96, h * 0.86, i / n * 0.5, 1, false); }, '#F4ECD8', 0.9);
  } });
  window.SHEET = { pictures };
})();

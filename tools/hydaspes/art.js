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
  const NIGHT = { plain: '#141827', plain2: '#181D2E', plain3: '#111526', hill: '#101422', hillRim: '#242A42',
                  water: '#262220', water2: '#2E2A28', sheen: '#8E8C96', bank: '#0A0A12', foam: '#A6A4AE', wood: '#0E1620', wood2: '#172430',
                  kas: '#1F2436', sand: '#4C4A50', sand2: '#3A383E', edge: '#141827', moon: '#8E99B8' };
  const DAY = { plain: '#7A7E6E', plain2: '#878B78', plain3: '#6B6F60', hill: '#6E7266', hillRim: '#A8AB98',
                water: '#4E3A2C', water2: '#5C4634', sheen: '#C8AE8E', bank: '#40322A', foam: '#E8DCC8', wood: '#3C4E3A', wood2: '#4A5E46',
                kas: '#66707E', sand: '#A69C88', sand2: '#8A8070', edge: '#6E7684', moon: '#F0C890' };
  const INK = '#15110C', FIRE = '#FFB347', FLAME = '#FFE08A', EMBER = '#E2552B', TENT = '#E8DCC2', TENT_SHADE = '#B9AC90';
  const SKIN = '#D9B48C', BRONZE = '#C9A24B', IRON = '#8A8D92', RED = '#9E2B25', HORSE = '#5A3D28', HORSE2 = '#8C6A48';
  const GREY = '#5A5C62', GREY2 = '#43454B', WATER_NIGHT = '#2E2C35', FOAM = '#8A8793';

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
        const f = k / n, jitter = (r() - 0.5) * 0.012;
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
    trees: scatter(105, 260, T.wide),        // trees (kept where the woods are)
    water: scatter(106, 0.5, T.wide),        // streaks of light on the water: which stretch, which offset
    fields: scatter(107, 1.4, T.wide),       // the plain's fields, long strips near the river
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
    if (close) for (const m of SCATTER.fine) if (within(m, 0.01)) {
      const [x, y] = P(m), rx = Math.max(1, px(0.03 + m[2] * 0.12)), ry = rx * (0.4 + m[3] * 0.5);
      ctx.fillStyle = B.rgba(m[4] < 0.5 ? pal.plain2 : pal.plain3, 0.03 + m[5] * 0.05);
      ctx.beginPath(); ctx.ellipse(x, y, rx, ry, m[5] * TAU, 0, TAU); ctx.fill();
    }
    // fields: long strips, faint, lying near the river the way the plain is farmed, each a little off the last
    for (const m of SCATTER.fields) {
      if (!within(m, 0.02)) continue;
      let bd = Infinity; for (let i = 0; i < T.centre.length; i += 6) { const d = Math.hypot((T.centre[i][0] - m[0]) * KM_LON, (T.centre[i][1] - m[1]) * KM_LAT); if (d < bd) bd = d; }
      if (bd > 9 || insidePoly(m, HILLS)) continue;
      const [x, y] = P(m), L = px(0.25 + m[2] * 0.45), W2 = px(0.06 + m[3] * 0.1), ang = (m[4] - 0.5) * 0.5 + 0.6;
      ctx.save(); ctx.translate(x, y); ctx.rotate(ang); ctx.fillStyle = B.rgba(m[5] < 0.5 ? pal.plain2 : pal.plain3, (night ? 0.08 : 0.12) * (1 - bd / 9)); ctx.fillRect(-L / 2, -W2 / 2, L, W2); ctx.restore();
    }
    // the hills' skirt: a mass with a soft foot, and the ridges as short strokes
    for (const [alpha, width] of [[0.04, px(4.5)], [0.06, px(3)], [0.08, px(2)], [0.1, px(1.2)], [0.14, px(0.6)]]) {
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
    bed(flood * 1.78, B.rgba(pal.bank, 0.5));         // the wet margin: mud the flood has just left, a little darker than the water
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
      streak(best, 6 + Math.floor(m[2] * 24), (m[3] - 0.5) * flood * 1.3, Math.max(0.8, px(0.01 + m[4] * 0.016)), (night ? 0.08 : 0.1) + m[5] * 0.08, pal.sheen);
      if (m[5] < 0.35) streak(best + 2, 4 + Math.floor(m[2] * 10), (m[4] < 0.5 ? -1 : 1) * flood * (0.72 + m[3] * 0.12), Math.max(0.8, px(0.01)), 0.35 + m[2] * 0.3, pal.foam);
    }
    ctx.restore();
    // the riverine woods as mapped; the headland, its ground above the flood with the grove on it;
    // the island, sand above the flood, pale against the water, a strand of foam round it, the grove in
    // its middle (so both channels read as water and the island as ground to be stranded on)
    const trees = (pts, dense) => {
      ctx.save(); poly(ctx, pts, P); ctx.clip();
      poly(ctx, pts, P); ctx.fillStyle = B.rgba(pal.wood, dense ? 0.9 : 0.4); ctx.fill();
      for (const m of SCATTER.trees) {
        if (!within(m, 0.01) || !insidePoly(m, pts) || (!dense && m[5] > 0.45)) continue;
        const [x, y] = P(m), rr = Math.max(1.2, px((dense ? 0.035 : 0.012) + m[2] * (dense ? 0.06 : 0.025)));
        ctx.fillStyle = B.rgba(m[3] < 0.6 ? pal.wood2 : pal.wood, 0.75);
        ctx.beginPath(); ctx.arc(x, y, rr, 0, TAU); ctx.fill();
        ctx.fillStyle = B.rgba(INK, 0.5); ctx.beginPath(); ctx.arc(x + rr * 0.3, y + rr * 0.35, rr * 0.5, 0, TAU); ctx.fill();
        if (dense) { ctx.fillStyle = B.rgba(pal.hillRim, night ? 0.35 : 0.5); ctx.beginPath(); ctx.arc(x - rr * 0.3, y - rr * 0.35, rr * 0.35, 0, TAU); ctx.fill(); }
      }
      ctx.restore();
    };
    for (const wd of T.woods) trees(wavy(wd.c, 11), false);
    poly(ctx, head, P); ctx.fillStyle = pal.plain2; ctx.fill();
    trees(head, true);
    poly(ctx, isl, P); ctx.strokeStyle = B.rgba(pal.bank, 0.45); ctx.lineWidth = Math.max(1, px(0.05)); ctx.stroke();   // the water's edge, a little darker
    poly(ctx, isl, P); ctx.fillStyle = pal.sand; ctx.fill();
    ctx.save(); poly(ctx, isl, P); ctx.clip();                                                                        // a sandbar: mottled, scrub in patches
    for (const m of SCATTER.fine) if (insidePoly(m, isl)) { const [x, y] = P(m); ctx.fillStyle = B.rgba(m[4] < 0.5 ? pal.sand2 : pal.plain2, 0.25 + m[5] * 0.3); ctx.beginPath(); ctx.ellipse(x, y, Math.max(1, px(0.02 + m[2] * 0.05)), Math.max(1, px(0.012 + m[3] * 0.03)), m[5] * TAU, 0, TAU); ctx.fill(); }
    ctx.restore();
    const mid = isl.reduce((acc, c) => [acc[0] + c[0] / isl.length, acc[1] + c[1] / isl.length], [0, 0]);
    trees(isl.map((c) => [mid[0] + (c[0] - mid[0]) * 0.94, mid[1] + (c[1] - mid[1]) * 0.94]), true);
    // the second channel: its own light, threads of foam between the island and the far bank
    {
      const A2 = T.authored, r2 = B.rand(61);
      for (let k = 0; k < 14; k++) {
        const f = r2(), o = 0.2 + r2() * 0.7;
        const p0 = [A2.islandFar[0] + (A2.landing[0] - A2.islandFar[0]) * o, A2.islandFar[1] + (A2.landing[1] - A2.islandFar[1]) * o];
        const dx = (A2.bend[0] - A2.islandNear[0]), dy = (A2.bend[1] - A2.islandNear[1]);      // roughly along the flow
        const q0 = [p0[0] - dx * (0.15 + f * 0.3), p0[1] - dy * (0.15 + f * 0.3)], q1 = [p0[0] + dx * (0.15 + f * 0.2), p0[1] + dy * (0.15 + f * 0.2)];
        ctx.strokeStyle = B.rgba(pal.foam, 0.25 + r2() * 0.3); ctx.lineWidth = Math.max(0.8, px(0.012)); ctx.lineCap = 'round';
        line(ctx, [q0, p0, q1], P); ctx.stroke();
      }
    }
    if (!night) {
      // (from the wide box's eastern edge to its western, whichever picture: so the two agree)
      const g = ctx.createLinearGradient(P([T.wide[2], 0])[0], 0, P([T.wide[0], 0])[0], 0);
      g.addColorStop(0, 'rgba(255,196,130,0.28)'); g.addColorStop(0.45, 'rgba(255,190,130,0.1)'); g.addColorStop(1, 'rgba(200,210,235,0.08)');
      ctx.fillStyle = g; ctx.fillRect(0, 0, w, h);
    }
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
      ellipse(box, (c) => { const g = c.createRadialGradient(0, 0, 0, 0, 0, 1); g.addColorStop(0, 'rgba(0,0,0,1)'); g.addColorStop(0.62, 'rgba(0,0,0,1)'); g.addColorStop(1, 'rgba(0,0,0,0)'); return g; });
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
    ctx.fillStyle = opts.dim ? '#6E6A74' : BRONZE; ctx.beginPath(); ctx.arc(x, y - s * 0.86, s * 0.11, 0, TAU); ctx.fill();
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
  /** A tent `s` wide at (x, y) its foot, in a high three-quarter view: a ridge, two faces (the one towards
      the fire lit), its shadow on the ground. `kind` 'ridge' (Macedonian) or 'pavilion' (a domed Indian tent). */
  function tent(ctx, x, y, s, lit, side, kind, cloth) {
    cloth = cloth || TENT;
    ctx.fillStyle = B.rgba(INK, 0.35); ctx.beginPath(); ctx.ellipse(x + s * 0.1, y + s * 0.08, s * 0.6, s * 0.16, 0, 0, TAU); ctx.fill();
    const lightFace = B.mix('#6E6558', cloth, 0.3 + lit * 0.7), darkFace = B.mix('#3A3530', cloth, 0.15 + lit * 0.35);
    if (kind === 'pavilion') {
      B.inked(ctx, (c) => { c.moveTo(x - s * 0.5, y); c.quadraticCurveTo(x - s * 0.5, y - s * 0.75, x, y - s * 0.8); c.quadraticCurveTo(x + s * 0.5, y - s * 0.75, x + s * 0.5, y); c.closePath(); }, side > 0 ? lightFace : darkFace, INK, s * 0.07);
      ctx.fillStyle = B.rgba(side > 0 ? darkFace : lightFace, 0.9); ctx.beginPath(); ctx.moveTo(x, y - s * 0.8); ctx.quadraticCurveTo(x + (side > 0 ? -1 : 1) * s * 0.5, y - s * 0.75, x + (side > 0 ? -1 : 1) * s * 0.5, y); ctx.lineTo(x, y); ctx.closePath(); ctx.fill();
      ctx.strokeStyle = INK; ctx.lineWidth = s * 0.05; ctx.beginPath(); ctx.moveTo(x, y - s * 0.8); ctx.lineTo(x, y - s * 1.05); ctx.stroke();   // the pole and its pennant
      ctx.fillStyle = RED; ctx.beginPath(); ctx.moveTo(x, y - s * 1.05); ctx.lineTo(x + s * 0.18, y - s * 1.0); ctx.lineTo(x, y - s * 0.94); ctx.closePath(); ctx.fill();
      return;
    }
    // the ridge tent: the near face a long slope, the end face a triangle, the ridge line above
    B.inked(ctx, (c) => { c.moveTo(x - s * 0.55, y); c.lineTo(x + s * 0.35, y + s * 0.06); c.lineTo(x + s * 0.2, y - s * 0.62); c.lineTo(x - s * 0.4, y - s * 0.56); c.closePath(); }, side > 0 ? lightFace : darkFace, INK, s * 0.07);
    B.inked(ctx, (c) => { c.moveTo(x + s * 0.35, y + s * 0.06); c.lineTo(x + s * 0.62, y - s * 0.12); c.lineTo(x + s * 0.2, y - s * 0.62); c.closePath(); }, side > 0 ? darkFace : lightFace, INK, s * 0.06);
    ctx.strokeStyle = B.rgba('#FFF4DA', 0.5 + lit * 0.4); ctx.lineWidth = s * 0.05; ctx.beginPath(); ctx.moveTo(x - s * 0.4, y - s * 0.56); ctx.lineTo(x + s * 0.2, y - s * 0.62); ctx.stroke();
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
    // the hides: rounded bags in a row, each stitched, lashed to two poles
    for (let i = 0; i < 5; i++) {
      B.inked(ctx, (c) => c.ellipse(x - s * 0.36 + i * s * 0.18, y + bob, s * 0.1, s * 0.075, 0, 0, TAU), '#B8976A', INK, s * 0.03);
      ctx.strokeStyle = B.rgba(INK, 0.5); ctx.lineWidth = s * 0.012; ctx.beginPath(); ctx.moveTo(x - s * 0.4 + i * s * 0.18, y + bob - s * 0.02); ctx.lineTo(x - s * 0.32 + i * s * 0.18, y + bob + s * 0.03); ctx.stroke();
    }
    ctx.strokeStyle = '#5A4330'; ctx.lineWidth = s * 0.025;
    for (const dy of [-0.06, 0.06]) { ctx.beginPath(); ctx.moveTo(x - s * 0.46, y + bob + s * dy); ctx.lineTo(x + s * 0.4, y + bob + s * dy); ctx.stroke(); }
    // two paddlers at the ends, a horse standing between
    soldier(ctx, x - s * 0.44, y + bob + s * 0.04, s * 0.22, 0, { shield: 'none', tunic: RED });
    soldier(ctx, x + s * 0.42, y + bob + s * 0.04, s * 0.22, 0, { shield: 'none', tunic: TENT_SHADE });
    horse(ctx, x, y - s * 0.02 + bob, s * 0.3, 0, false, HORSE);
    ctx.strokeStyle = INK; ctx.lineWidth = s * 0.02; ctx.beginPath(); ctx.moveTo(x - s * 0.5, y + bob); ctx.lineTo(x - s * 0.6, y + bob + s * 0.14 + Math.sin(t * TAU) * s * 0.04); ctx.stroke();   // a paddle
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
    ctx.save(); ctx.translate(w / 2, h * 0.55); ctx.scale(w * 0.47, h * 0.4);
    const ground = ctx.createRadialGradient(0, 0, 0, 0, 0, 1); ground.addColorStop(0.5, B.rgba('#2A2420', 0.4)); ground.addColorStop(1, B.rgba('#2A2420', 0));
    ctx.fillStyle = ground; ctx.beginPath(); ctx.arc(0, 0, 1, 0, TAU); ctx.fill(); ctx.restore();
    // the fires: hot points, each with a small halo on the ground (not a blur over the camp)
    const fires = [];
    for (let i = 0; i < (opts.fires || 14); i++) fires.push([w * (0.12 + r() * 0.76), h * (0.22 + r() * 0.68), 0.6 + r() * 0.6]);
    for (const f of fires) B.glow(ctx, f[0], f[1], s * 1.3 * f[2], FIRE, 0.5, 0);
    // tents in staggered rows, no two the same size or angle, dimmed to the night, their fire-side faces lit
    const rows = opts.rows || 4, cols = opts.cols || 7, tents = [];
    for (let j = 0; j < rows; j++) for (let i = 0; i < cols; i++) {
      if (r() < 0.15) continue;
      const x = w * (0.12 + (i + (j % 2) * 0.5) * 0.76 / cols) + (r() - 0.5) * s * 0.9, y = h * (0.3 + j * 0.62 / rows) + (r() - 0.5) * s * 0.6;
      tents.push([x, y, 1.1 + r() * 0.5, opts.kind === 'pavilion' && r() < 0.6 ? 'pavilion' : 'ridge']);
    }
    tents.sort((a, b) => a[1] - b[1]);
    for (const [x, y, size, kind] of tents) {
      let near = Infinity, side = 1;
      for (const f of fires) { const d = Math.hypot(f[0] - x, f[1] - y); if (d < near) { near = d; side = f[0] < x ? -1 : 1; } }
      const lit = Math.max(0, 1 - near / (s * 3.2));
      tent(ctx, x, y, s * size, lit, side, kind, opts.cloth);
    }
    if (opts.chariots) for (let i = 0; i < opts.chariots; i++) chariot(ctx, w * 0.72 + i * s * 1.3, h * 0.93, s * 0.75, 0.5);
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

  // the dark over the second channel: a patch in the ground's own night tone joining the island to the far
  // bank, laid over the close picture until the column reaches the island's far shore; then it thins and
  // the river opens in front of them. Its bounds are the island's and the landing's, with a margin.
  {
    const A = T.authored, isl = A.island, box = A.veil;      // (the builder's box: the pack lays the picture in the same)
    const w = 1024, h = pictureHeight(box, 1024);
    pictures.push({ name: 'channel-veil', width: w, height: h, scale: 1, draw(ctx) {
      const P = projector(box, w, h), kpp = kmPerPx(box, w);
      // the far bank's side of the island, pulled out to the landing and beyond: one shape of ground
      const mid = isl.reduce((acc, c) => [acc[0] + c[0] / isl.length, acc[1] + c[1] / isl.length], [0, 0]);
      const dx = A.landing[0] - mid[0], dy = A.landing[1] - mid[1];
      const shape = isl.map((c) => [c[0] + dx * 1.05, c[1] + dy * 1.05]).concat(isl.slice().reverse().map((c) => [c[0] + dx * 0.25, c[1] + dy * 0.25]));
      const dusk = B.mix(NIGHT.plain, DAY.plain, 0.22);
      ctx.save(); ctx.shadowColor = dusk; ctx.shadowBlur = 0.12 / kpp;
      poly(ctx, shape, P); ctx.fillStyle = dusk; ctx.fill(); ctx.fill(); ctx.restore();
      // a few of the island's trees over it, so the join reads as the same ground
      ctx.save(); poly(ctx, shape, P); ctx.clip();
      for (const m of SCATTER.trees) if (insidePoly(m, shape) && m[5] < 0.2) { const [x, y] = P(m), rr = Math.max(1.2, 0.03 / kpp); ctx.fillStyle = B.rgba(NIGHT.wood2, 0.7); ctx.beginPath(); ctx.arc(x, y, rr, 0, TAU); ctx.fill(); }
      ctx.restore();
    } });
  }

  // ---- the sprites ------------------------------------------------------------------------------------
  const frames = (n) => n;
  function pinpricks(ctx, w, h, t, seed, n, size) {
    const r = B.rand(seed);
    for (let k = 0; k < n; k++) {
      const x = w * (0.1 + r() * 0.8), y = h * (0.12 + r() * 0.76), ph = r(), b = 0.7 + 0.3 * Math.sin((t + ph) * TAU);
      B.glow(ctx, x, y, size * (1.6 + r()) * b, FIRE, 0.5 * b, 0);
      ctx.fillStyle = B.rgba('#FFF1C4', 0.95 * b); ctx.beginPath(); ctx.arc(x, y, size * 0.32, 0, TAU); ctx.fill();
    }
  }
  pictures.push({ name: 'camp-far', width: 14, height: 12, frames: 4, draw(ctx, w, h, i, n) { pinpricks(ctx, w, h, i / n, 3, 9, 0.9); } });
  pictures.push({ name: 'camp-mid', width: 40, height: 30, frames: 6, draw(ctx, w, h, i, n) { pinpricks(ctx, w, h, i / n, 5, 34, 1.2); } });
  pictures.push({ name: 'camp-a', scale: 2, width: 230, height: 160, frames: 8, draw(ctx, w, h, i, n) { camp(ctx, w, h, i / n, 11, { fires: 16, rows: 4, cols: 8, horses: 7, walkers: 10 }); } });
  // the camp left behind: its fires kept burning, a few figures still walking among them
  pictures.push({ name: 'camp-decoy', scale: 2, width: 230, height: 160, frames: 8, draw(ctx, w, h, i, n) { camp(ctx, w, h, i / n, 11, { fires: 16, rows: 4, cols: 8, horses: 2, walkers: 4 }); } });
  pictures.push({ name: 'camp-p', scale: 2, width: 250, height: 170, frames: 8, draw(ctx, w, h, i, n) { camp(ctx, w, h, i / n, 23, { fires: 18, rows: 4, cols: 9, horses: 4, walkers: 8, kind: 'pavilion', cloth: '#D9B57A', chariots: 3 }); } });
  // the elephants at the water, watching the far bank (toward the viewer's left, the river); by day they turn
  pictures.push({ name: 'elephants-watch', width: 140, height: 34, frames: 8, draw(ctx, w, h, i, n) {
    const t = i / n, r = B.rand(17);
    for (let k = 0; k < 5; k++) {
      const x = w * (0.1 + k * 0.2) + (r() - 0.5) * 8, raise = k === 2 && i >= 2 && i <= 5 ? 1 : 0, size = h * (0.6 + r() * 0.2);
      const facing = k === 3 ? 0 : -1;                          // one turned to the viewer; the rest west, towards the far camp
      elephant(ctx, x, h * 0.95 + Math.sin(t * TAU + k) * 0.4, size, t + k * 0.17 + r(), facing, k === 1 || k === 4);
      if (raise) { ctx.save(); ctx.translate(x, 0); ctx.scale(-1, 1); ctx.translate(-x, 0); ctx.strokeStyle = GREY; ctx.lineWidth = h * 0.06; ctx.lineCap = 'round'; ctx.beginPath(); ctx.moveTo(x + h * 0.34, h * 0.55); ctx.quadraticCurveTo(x + h * 0.5, h * 0.25, x + h * 0.42, h * 0.08); ctx.stroke(); ctx.restore(); }
    }
  } });
  pictures.push({ name: 'elephants-mid', width: 40, height: 10, frames: 4, draw(ctx, w, h, i, n) {
    const r = B.rand(23);
    for (let k = 0; k < 5; k++) { const rx = 2 + r() * 1.6; ctx.fillStyle = B.rgba(GREY, 0.85); ctx.beginPath(); ctx.ellipse(w * (0.1 + k * 0.2) + (r() - 0.5) * 4, h * 0.55 + Math.sin(i / n * TAU + k) * 0.4, rx, rx * 0.75, 0, 0, TAU); ctx.fill(); }
  } });
  // the column on the march: ranks of foot with their spears, horse at its head, no fires
  pictures.push({ name: 'column-far', width: 24, height: 6, frames: 4, draw(ctx, w, h, i, n) {
    // what you would see of a column in the dark from across the river: a thin line of pale points, creeping
    for (let k = 0; k < 11; k++) { const x = 1.5 + k * 2.1 + ((i + k) % 4) * 0.12; ctx.fillStyle = B.rgba('#8E8AA0', 0.5 + ((k + i) % 3) * 0.15); ctx.beginPath(); ctx.arc(x, h / 2 + Math.sin(k * 1.3) * 0.7, 0.75, 0, TAU); ctx.fill(); }
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
    ctx.globalAlpha = 0.8;
    for (let row = 0; row < 2; row++) for (let k = 0; k < 12; k++) soldier(ctx, 4 + k * 4.4 + row * 1.5, h * 0.62 + row * 5, 5.2, (t + k * 0.1 + row * 0.3) % 1, { shield: 'none', tunic: (k + row) % 2 ? '#4A4650' : '#6A6670', dim: true });
    horse(ctx, w * 0.92, h * 0.7, 5, t, true, '#3E3A42');
    ctx.globalAlpha = 1;
  } });
  // (the night march is a dim grey thread: cloaks over everything, no gold, barely more than the ground)
  pictures.push({ name: 'cluster', width: 30, height: 30, frames: 8, draw(ctx, w, h, i, n) {
    const t = i / n, s = 10, r = B.rand(31);
    ctx.globalAlpha = 0.75;
    for (let k = 0; k < 5; k++) { const x = 5 + k * 5 + (r() - 0.5) * 2, y = h * 0.55 + (k % 2) * 8 + (r() - 0.5) * 3; soldier(ctx, x, y, s, (t + k * 0.17) % 1, { tunic: k % 2 ? '#4A4650' : '#5A5660', shield: 'none', dim: true }); ctx.fillStyle = B.rgba('#C8CCE0', 0.7); ctx.beginPath(); ctx.arc(x - s * 0.03, y - s * 0.92, s * 0.035, 0, TAU); ctx.fill(); }
    ctx.globalAlpha = 1;
  } });
  pictures.push({ name: 'cluster-horse', width: 30, height: 30, frames: 8, draw(ctx, w, h, i, n) {
    const t = i / n;
    ctx.globalAlpha = 0.75;
    for (let k = 0; k < 2; k++) horse(ctx, 8 + k * 11, h * 0.6 + k * 7, 9, (t + k * 0.3) % 1, true, '#3E3A42');
    ctx.globalAlpha = 1;
  } });
  // the boats and the hides pushed across
  pictures.push({ name: 'boats', width: 120, height: 54, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    for (const [x, y, len] of [[w * 0.3, h * 0.28, 56], [w * 0.72, h * 0.88, 50], [w * 0.8, h * 0.4, 30], [w * 0.22, h * 0.74, 28]]) {   // their wakes
      ctx.strokeStyle = B.rgba(FOAM, 0.4); ctx.lineWidth = 1; ctx.beginPath(); ctx.moveTo(x - len * 0.55, y + 2); ctx.lineTo(x - len * 0.9, y + 5 + Math.sin(t * TAU) * 1.5); ctx.stroke();
    }
    galley(ctx, w * 0.3, h * 0.28, 56, t);
    raft(ctx, w * 0.8, h * 0.4, 30, t);
    raft(ctx, w * 0.22, h * 0.74, 28, t + 0.4);
    galley(ctx, w * 0.72, h * 0.88, 50, t + 0.5);
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
  pictures.push({ name: 'waders', width: 170, height: 130, frames: 8, draw(ctx, w, h, i, n) {
    const t = i / n, s = 15, r = B.rand(29);
    // the current, drawn: threads of pale water running the length of the channel, moving with the frames
    ctx.lineCap = 'round';
    for (let k = 0; k < 28; k++) {
      const x = 10 + r() * (w - 20), y0 = ((r() + t * 0.6) % 1) * h, len = 18 + r() * 34;
      ctx.strokeStyle = B.rgba(FOAM, 0.12 + r() * 0.2); ctx.lineWidth = 0.8 + r() * 1.2;
      ctx.beginPath(); ctx.moveTo(x + len * 0.35, y0); ctx.quadraticCurveTo(x + len * 0.15, y0 + len * 0.5, x - len * 0.35, y0 + len); ctx.stroke();
    }
    const dip = (x, y, wide) => {
      const g = ctx.createRadialGradient(x, y, 0, x, y, wide);
      g.addColorStop(0, B.rgba('#4A3E38', 0.8)); g.addColorStop(0.6, B.rgba('#4A3E38', 0.6)); g.addColorStop(1, B.rgba('#4A3E38', 0));
      ctx.fillStyle = g; ctx.beginPath(); ctx.ellipse(x, y + wide * 0.35, wide, wide * 0.55, 0, 0, TAU); ctx.fill();
    };
    const items = [];
    // a file crossing, bunched: a band from the picture's upper left (the island's shore) to its lower right
    // (the far bank), the way the ford runs; the first few at the knee, then the chest, the middle deepest
    for (let k = 0; k < 30; k++) {
      const u = k / 30, x = 16 + u * (w - 32) + (r() - 0.5) * 10, y = 22 + u * (h - 44) + (r() - 0.5) * 18 + Math.sin(t * TAU + k * 1.7) * 1.3;
      if (r() < 0.1) continue;
      const depth = u < 0.12 ? 0.3 + r() * 0.1 : u > 0.88 ? 0.5 + r() * 0.1 : 0.55 + Math.sin(u * Math.PI) * 0.3 + r() * 0.1;
      const row = k % 3;
      items.push({ y, draw: () => {
        const top = y - s * 1.4, water = y - s * depth, tilt = (r() - 0.5) * 0.3;
        ctx.save(); ctx.beginPath(); ctx.rect(x - s, top, s * 2, water - top); ctx.clip();
        ctx.translate(x, y); ctx.rotate(tilt); ctx.translate(-x, -y);
        soldier(ctx, x, y, s, 0, { shield: r() < 0.92 ? 'up' : 'none', tunic: (k + row) % 2 ? RED : TENT_SHADE });
        ctx.restore();
        dip(x, water + s * 0.05, s * 0.5);
        ctx.strokeStyle = B.rgba(FOAM, 0.6); ctx.lineWidth = 0.9; ctx.lineCap = 'round';
        ctx.beginPath(); ctx.moveTo(x + s * 0.2, water + 1); ctx.quadraticCurveTo(x + s * 1.0, water + 1.8, x + s * 1.8, water + 0.6); ctx.stroke();
        ctx.beginPath(); ctx.moveTo(x - s * 0.3, water + 2.5); ctx.quadraticCurveTo(x + s * 0.6, water + 3.2, x + s * 1.3, water + 2.2); ctx.stroke();
      } });
    }
    for (let k = 0; k < 3; k++) {
      const u = 0.25 + k * 0.25, x = 16 + u * (w - 32) + (r() - 0.5) * 12, y = 22 + u * (h - 44) + (r() - 0.5) * 14 + Math.sin(t * TAU + k * 2) * 1.5;
      items.push({ y, draw: () => { horseHead(ctx, x, y, s * 1.15, k % 2 === 0, k % 2 ? HORSE2 : HORSE); dip(x + s * 0.55, y + s * 0.1, s * 0.8); } });
    }
    items.sort((a, b) => a.y - b.y).forEach((o) => o.draw());
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
    sunShadow(ctx, w * 0.6, h * 0.8, w * 0.35, h * 0.3);
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
  pictures.push({ name: 'phalanx', width: 96, height: 40, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n, s = 8;
    sunShadow(ctx, w / 2, h * 0.75, w * 0.5, h * 0.3);
    for (let row = 0; row < 4; row++) for (let k = 0; k < 10; k++) {
      const x = w * 0.06 + k * 7.5 + row * 2.2, y = h * 0.42 + row * 8;
      soldier(ctx, x, y, s, (t + k * 0.05) % 1, { tunic: TENT_SHADE });
      ctx.strokeStyle = INK; ctx.lineWidth = 0.6; ctx.beginPath(); ctx.moveTo(x + 2, y - 3); ctx.lineTo(x + 16, y - 11); ctx.stroke();   // the sarissa
    }
  } });
  pictures.push({ name: 'phalanx-mid', width: 36, height: 12, frames: 4, draw(ctx, w, h, i, n) {
    for (let row = 0; row < 2; row++) for (let k = 0; k < 10; k++) { ctx.fillStyle = TENT_SHADE; ctx.fillRect(2 + k * 3.2 + row, 3 + row * 4 + ((i + k) % 2) * 0.4, 1.8, 2.4); }
  } });
  // Porus' son's chariots, and the same bogged in the clay
  pictures.push({ name: 'chariots', width: 130, height: 40, frames: 6, draw(ctx, w, h, i, n) {
    const t = i / n;
    sunShadow(ctx, w * 0.6, h * 0.8, w * 0.35, h * 0.3);
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
  pictures.push({ name: 'chariots-mid', width: 44, height: 14, frames: 4, draw(ctx, w, h, i, n) {
    for (let k = 0; k < 5; k++) { chariot(ctx, 5 + k * 8.5, 11 + ((i + k) % 2) * 0.6, 4.5, i / n + k * 0.2); }
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
    const rr = B.rand(43);
    for (let k = 0; k < 6; k++) order.push({ y: at(0, (k - 2.5) * 34)[1], draw: () => {
      const p = at(turning ? Math.sin(k + t * TAU) * 5 : (rr() - 0.5) * 6, (k - 2.5) * 34);
      elephant(ctx, p[0], p[1] + 9, 16 + rr() * 5, t + k * 0.13, turning ? (k % 3 === 0 ? 0 : (k % 3 === 1 ? -FACE : FACE)) : FACE, true);
    } });
    for (let row = 0; row < 2; row++) for (let k = 0; k < 22; k++) {
      const p = at(16 + row * 7 + (turning ? Math.cos(k + t * TAU) * 3 : 0), (k - 10.5) * 9 + row * 3);
      order.push({ y: p[1], draw: () => soldier(ctx, p[0], p[1], 6.5, turning ? (t + k * 0.1) % 1 : 0, { shield: 'none', tunic: (k + row) % 2 ? '#C77B3E' : TENT_SHADE }) });
    }
    for (const sgn of [-1, 1]) { const p = at(4, sgn * 110); order.push({ y: p[1], draw: () => { ctx.save(); if (FACE < 0) { ctx.translate(p[0], 0); ctx.scale(-1, 1); ctx.translate(-p[0], 0); } chariot(ctx, p[0], p[1], 11, t); ctx.restore(); } }); }
    order.sort((a, b) => a.y - b.y).forEach((o) => o.draw());   // the nearer drawn last, as in a high view
  }
  /** The morning's low sun from the east: a soft shadow laid a little to the west of a formation. */
  function sunShadow(ctx, x, y, rx, ry) { ctx.save(); ctx.translate(x - rx * 0.25, y); ctx.scale(rx, ry); const g = ctx.createRadialGradient(0, 0, 0, 0, 0, 1); g.addColorStop(0, 'rgba(20,18,30,0.35)'); g.addColorStop(1, 'rgba(20,18,30,0)'); ctx.fillStyle = g; ctx.beginPath(); ctx.arc(0, 0, 1, 0, TAU); ctx.fill(); ctx.restore(); }
  pictures.push({ name: 'line', width: 240, height: 240, frames: 6, draw(ctx, w, h, i, n) { sunShadow(ctx, w / 2, h / 2 + 10, 70, 110); lineOf(ctx, w, h, i / n, false); } });
  pictures.push({ name: 'line-turning', width: 240, height: 240, frames: 6, draw(ctx, w, h, i, n) { sunShadow(ctx, w / 2, h / 2 + 10, 70, 110); lineOf(ctx, w, h, i / n, true); } });
  pictures.push({ name: 'line-mid', width: 60, height: 16, frames: 4, draw(ctx, w, h, i, n) {
    for (let k = 0; k < 9; k++) { ctx.fillStyle = GREY; ctx.beginPath(); ctx.ellipse(4 + k * 6.4, 10 + Math.sin(i / n * TAU + k) * 0.5, 2.6, 2.2, 0, 0, TAU); ctx.fill(); }
    for (let k = 0; k < 20; k++) { ctx.fillStyle = '#C77B3E'; ctx.fillRect(2 + k * 2.9, 2 + (k % 2), 1.4, 2); }
  } });
  // the rain, drifting over the plain's edges; the lightning over the hills
  pictures.push({ name: 'rain', width: 200, height: 320, frames: 8, scale: 1.5, draw(ctx, w, h, i, n) {
    const t = i / n, r = B.rand(41);
    // curtains of rain drifting over the plain: soft slanting bands with no edge of their own, and a few
    // longer threads in them
    ctx.save(); ctx.translate(w / 2, h / 2); ctx.scale(w / 2, h / 2);
    const g = ctx.createRadialGradient(0, 0, 0, 0, 0, 1);
    g.addColorStop(0, 'rgba(130,140,170,0.07)'); g.addColorStop(0.7, 'rgba(130,140,170,0.03)'); g.addColorStop(1, 'rgba(130,140,170,0)');
    ctx.fillStyle = g; ctx.beginPath(); ctx.arc(0, 0, 1, 0, TAU); ctx.fill(); ctx.restore();
    const falloff = (x, y) => Math.pow(Math.max(0, 1 - Math.hypot((x - w / 2 + (y - h / 2) * 0.3) / (w / 2), (y - h / 2) / (h / 2))), 1.6);
    for (let k = 0; k < 7; k++) {
      // a band: a slanting strip of faint light, drifting with the frames
      // one slanting stroke, its light following the field's fall-off along it
      const x0 = ((k / 7 + t * 0.35) % 1) * w * 1.4 - w * 0.2, width = 10 + r() * 16, x1 = x0 - h * 0.32;
      const g2 = ctx.createLinearGradient(x0, 0, x1, h);
      for (let j = 0; j <= 10; j++) { const y = h * j / 10; g2.addColorStop(j / 10, B.rgba('#C8D0E6', falloff(x0 - y * 0.32, y) * 0.07)); }
      ctx.lineCap = 'butt'; ctx.lineWidth = width; ctx.strokeStyle = g2;
      ctx.beginPath(); ctx.moveTo(x0, 0); ctx.lineTo(x1, h); ctx.stroke();
    }
    ctx.strokeStyle = 'rgba(205,215,240,0.5)'; ctx.lineWidth = 1.1; ctx.lineCap = 'round';
    for (let k = 0; k < 150; k++) {
      const x = r() * w, y0 = (r() * h + t * h * 1.3) % h, len = 12 + r() * 20;
      ctx.globalAlpha = falloff(x, y0) * (0.25 + r() * 0.4);
      ctx.beginPath(); ctx.moveTo(x, y0); ctx.lineTo(x - len * 0.32, y0 + len); ctx.stroke();
    }
    ctx.globalAlpha = 1;
    // (and the whole field ends softly, whatever a band's width carried to the picture's edge)
    ctx.save(); ctx.globalCompositeOperation = 'destination-in';
    ctx.translate(w / 2, h / 2); ctx.scale(w / 2, h / 2);
    const m = ctx.createRadialGradient(0, 0, 0, 0, 0, 1); m.addColorStop(0.55, 'rgba(0,0,0,1)'); m.addColorStop(1, 'rgba(0,0,0,0)');
    ctx.fillStyle = m; ctx.fillRect(-1, -1, 2, 2); ctx.restore();
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
  pictures.push({ name: 'fireglow', width: 100, height: 60, frames: 4, scale: 1, draw(ctx, w, h, i, n) {
    // the fires on the water: long broken streaks of their light, running with the current, flickering
    const r = B.rand(3), t = i / n;
    ctx.save(); ctx.translate(w / 2, h / 2); ctx.scale(1, h / w);
    B.glow(ctx, 0, 0, w / 2, FIRE, 0.16, 0); ctx.restore();
    // (the streaks lie along the river's run at the camps: its angle on the screen, from the middle's line there)
    const A = T.authored, C = T.centre, i0 = C.reduce((b, c, i) => (Math.hypot(c[0] - A.campA[0], c[1] - A.campA[1]) < Math.hypot(C[b][0] - A.campA[0], C[b][1] - A.campA[1]) ? i : b), 0);
    const d = [(C[Math.min(C.length - 1, i0 + 3)][0] - C[Math.max(0, i0 - 3)][0]) * KM_LON, -(C[Math.min(C.length - 1, i0 + 3)][1] - C[Math.max(0, i0 - 3)][1]) * KM_LAT];
    const ang = Math.atan2(d[1], d[0]);
    ctx.lineCap = 'round';
    for (let k = 0; k < 16; k++) {
      const x = w * (0.08 + r() * 0.84), y = h * (0.15 + r() * 0.7), len = 6 + r() * 16, a = 0.3 + r() * 0.4, ph = r();
      const f = 0.7 + 0.3 * Math.sin((t + ph) * TAU);
      ctx.strokeStyle = B.rgba(k % 3 ? FIRE : FLAME, a * f * 0.7); ctx.lineWidth = 1 + r() * 1.4;
      ctx.beginPath(); ctx.moveTo(x - Math.cos(ang) * len / 2, y - Math.sin(ang) * len / 2); ctx.lineTo(x + Math.cos(ang) * len / 2, y + Math.sin(ang) * len / 2); ctx.stroke();
    }
  } });
  // the doorway: no badge, no glow, nothing that says "tap here": an elephant standing at the water on
  // the Jhelum's bank, ordinary life that doesn't quite belong on this bank today (it faces east as
  // drawn; the stage mirrors it as it stirs along the bank)
  pictures.push({ name: 'doorway', width: 30, height: 20, frames: 2, draw(ctx, w, h, i, n) {
    B.haloed(ctx, w, h, (c) => { c.lineCap = 'round'; c.lineJoin = 'round'; elephant(c, w * 0.5, h * 0.96, h * 0.86, i / n * 0.5, 1, false); }, '#F4ECD8', 0.9);
  } });
  window.SHEET = { pictures };
})();

// pressure-front: drawing. Paper, the ground's contours, the river; each army as a topography of
// its own density (contour lines, deeper = darker); the front where they touch as wet black ink
// with flecks thrown off it; and the ground keeping a stain where men fell.
(function () {
  const { GW, GH, riverX, RIVER_HALF } = window.PF;
  const PAPER = [236, 228, 210];
  const COL = { rome: [158, 44, 30], carthage: [34, 62, 104] };

  function Painter(canvas, world) {
    const ctx = canvas.getContext('2d');
    let W = 0, H = 0, dpr = 1, cs = 1, bg = null;
    const small = document.createElement('canvas'); small.width = GW; small.height = GH;
    const sctx = small.getContext('2d');
    const img = sctx.createImageData(GW, GH);
    const up = new Float32Array(GW * 2 * GH * 2);
    let seed = 7; const rnd = () => { seed = (seed * 16807) % 2147483647; return seed / 2147483647; };

    function resize() {
      dpr = window.devicePixelRatio || 1; W = window.innerWidth; H = window.innerHeight;
      canvas.width = Math.round(W * dpr); canvas.height = Math.round(H * dpr);
      canvas.style.width = W + 'px'; canvas.style.height = H + 'px';
      cs = Math.max(W / GW, H / GH); // cell size in css px (cover)
      bg = background();
    }
    this.cellSize = () => cs;
    this.toGrid = (px, py) => [px / cs, py / cs];

    // marching squares over a field upsampled 2x (bilinear), segments in css px
    function contours(field, level, out, mfield, mout) {
      const w = GW * 2, h = GH * 2;
      for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
        const fx = Math.min(GW - 1.001, Math.max(0, (x + 0.5) / 2 - 0.5)), fy = Math.min(GH - 1.001, Math.max(0, (y + 0.5) / 2 - 0.5));
        const ix = fx | 0, iy = fy | 0, tx = fx - ix, ty = fy - iy, i = iy * GW + ix;
        up[y * w + x] = (field[i] * (1 - tx) + field[i + 1] * tx) * (1 - ty) + (field[i + GW] * (1 - tx) + field[i + GW + 1] * tx) * ty;
      }
      const s = cs / 2;
      for (let y = 0; y < h - 1; y++) for (let x = 0; x < w - 1; x++) {
        const a = up[y * w + x], b = up[y * w + x + 1], c = up[(y + 1) * w + x + 1], d = up[(y + 1) * w + x];
        const k = (a > level) | ((b > level) << 1) | ((c > level) << 2) | ((d > level) << 3);
        if (k === 0 || k === 15) continue;
        const e = [];
        const lerp = (p, q) => (level - p) / (q - p);
        if ((k & 1) !== ((k >> 1) & 1)) e.push([x + lerp(a, b), y]);
        if (((k >> 1) & 1) !== ((k >> 2) & 1)) e.push([x + 1, y + lerp(b, c)]);
        if (((k >> 2) & 1) !== ((k >> 3) & 1)) e.push([x + 1 - lerp(c, d), y + 1]);
        if (((k >> 3) & 1) !== (k & 1)) e.push([x, y + 1 - lerp(d, a)]);
        let o = out;
        if (mfield) { const gi = Math.min(GH - 1, (y / 2) | 0) * GW + Math.min(GW - 1, (x / 2) | 0); if (mfield[gi] < 0.5) o = mout; }
        for (let j = 0; j + 1 < e.length; j += 2) o.push((e[j][0] + 0.5) * s, (e[j][1] + 0.5) * s, (e[j + 1][0] + 0.5) * s, (e[j + 1][1] + 0.5) * s);
      }
    }
    function strokeSegs(c, segs) { c.beginPath(); for (let i = 0; i < segs.length; i += 4) { c.moveTo(segs[i], segs[i + 1]); c.lineTo(segs[i + 2], segs[i + 3]); } c.stroke(); }

    function background() {
      const c = document.createElement('canvas'); c.width = canvas.width; c.height = canvas.height;
      const g = c.getContext('2d'); g.scale(dpr, dpr);
      g.fillStyle = `rgb(${PAPER})`; g.fillRect(0, 0, W, H);
      const T = world.G.T, tw = T.w, th = T.h, z = T.z;
      // soft shading of the ground (light from the upper left), as a wash
      const sh = document.createElement('canvas'); sh.width = tw; sh.height = th;
      const sg = sh.getContext('2d'), id = sg.createImageData(tw, th);
      for (let y = 0; y < th; y++) for (let x = 0; x < tw; x++) {
        const i = y * tw + x;
        const dx = z[y * tw + Math.min(tw - 1, x + 1)] - z[y * tw + Math.max(0, x - 1)];
        const dy = z[Math.min(th - 1, y + 1) * tw + x] - z[Math.max(0, y - 1) * tw + x];
        const s = (dx + dy) * 0.11;          // facing away from the light: shade
        const hgt = (z[i] - 5) / 70;
        id.data[4 * i] = 120 - hgt * 30; id.data[4 * i + 1] = 100 - hgt * 25; id.data[4 * i + 2] = 70 - hgt * 15;
        id.data[4 * i + 3] = Math.max(0, Math.min(90, s * 60 + hgt * 70));
      }
      sg.putImageData(id, 0, 0);
      g.imageSmoothingQuality = 'high'; g.drawImage(sh, 0, 0, GW * cs, GH * cs);
      // the ground's contours: every 2.5 m, every 10 m a little firmer
      const zf = world.G.elev; // sim resolution is enough with the 2x upsample
      for (let lv = 5; lv < 85; lv += 2.5) {
        const segs = []; contours(zf, lv, segs);
        const major = Math.abs(lv % 10) < 0.01;
        g.strokeStyle = major ? 'rgba(120,96,62,0.30)' : 'rgba(120,96,62,0.13)'; g.lineWidth = major ? 0.9 : 0.6;
        strokeSegs(g, segs);
      }
      // the river: a ribbon of pale water inked on both banks
      const pts = []; for (let y = -1; y <= GH + 1; y += 0.5) pts.push([riverX(y) * cs, y * cs]);
      const ribbon = (half, style, lw) => {
        g.beginPath(); pts.forEach((p, k) => k ? g.lineTo(p[0] - half, p[1]) : g.moveTo(p[0] - half, p[1]));
        for (let k = pts.length - 1; k >= 0; k--) g.lineTo(pts[k][0] + half, pts[k][1]);
        g.closePath(); if (style) { g.fillStyle = style; g.fill(); }
        if (lw) { g.lineWidth = lw; g.strokeStyle = 'rgba(64,92,110,0.75)'; g.stroke(); }
      };
      ribbon(RIVER_HALF * cs * 1.9, 'rgba(150,176,186,0.25)');
      ribbon(RIVER_HALF * cs, 'rgba(170,196,204,0.95)', 1.1);
      // water lines inside the river
      g.strokeStyle = 'rgba(64,92,110,0.35)'; g.lineWidth = 0.6;
      for (const o of [-0.35, 0.3]) { g.beginPath(); pts.forEach((p, k) => k ? g.lineTo(p[0] + o * RIVER_HALF * cs, p[1]) : g.moveTo(p[0] + o * RIVER_HALF * cs, p[1])); g.stroke(); }
      // a vignette of paper
      const vg = g.createRadialGradient(W / 2, H / 2, Math.min(W, H) * 0.45, W / 2, H / 2, Math.max(W, H) * 0.75);
      vg.addColorStop(0, 'rgba(236,228,210,0)'); vg.addColorStop(1, 'rgba(206,192,165,0.55)');
      g.fillStyle = vg; g.fillRect(0, 0, W, H);
      return c;
    }

    const mor = new Float32Array(GW * GH), tmpF = new Float32Array(GW * GH);
    this.draw = function (live) {
      ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.drawImage(bg, 0, 0);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      // where men fell
      const d = img.data;
      for (let i = 0; i < GW * GH; i++) { const v = Math.min(1, Math.sqrt(world.dead[i] * 2.2)); d[4 * i] = 112; d[4 * i + 1] = 74; d[4 * i + 2] = 44; d[4 * i + 3] = v * 75; }
      sctx.putImageData(img, 0, 0); ctx.imageSmoothingEnabled = true; ctx.imageSmoothingQuality = 'high';
      ctx.drawImage(small, 0, 0, GW * cs, GH * cs);
      { const segs = []; contours(world.dead, 0.12, segs); ctx.strokeStyle = 'rgba(112,74,44,0.55)'; ctx.lineWidth = 0.8; ctx.setLineDash([1, 2.5]); strokeSegs(ctx, segs); ctx.setLineDash([]); }
      // each army: a wash, then its own contours
      for (const A of world.armies) {
        const col = COL[A.name];
        for (let i = 0; i < GW * GH; i++) {
          const r = A.rho[i], m = r > 1e-4 ? A.q[i] / r : 1; mor[i] = r > 0.02 ? m : 1;
          const a = Math.pow(Math.min(1, r * 1.15), 0.85) * (0.30 + 0.25 * m);
          const pale = 1 - m;
          d[4 * i] = col[0] + (PAPER[0] - col[0]) * pale * 0.5; d[4 * i + 1] = col[1] + (PAPER[1] - col[1]) * pale * 0.5; d[4 * i + 2] = col[2] + (PAPER[2] - col[2]) * pale * 0.5;
          d[4 * i + 3] = a * 255;
        }
        sctx.putImageData(img, 0, 0); ctx.drawImage(small, 0, 0, GW * cs, GH * cs);
        const levels = [[0.08, 0.7, 0.55], [0.3, 0.9, 0.7], [0.55, 1.1, 0.85], [0.8, 1.5, 1]];
        for (const [lv, lw, al] of levels) {
          const firm = [], broken = []; contours(A.rho, lv, firm, mor, broken);
          ctx.lineWidth = lw; ctx.lineCap = 'round';
          ctx.strokeStyle = `rgba(${col},${al})`; ctx.setLineDash([]); strokeSegs(ctx, firm);
          ctx.strokeStyle = `rgba(${col},${al * 0.65})`; ctx.setLineDash([1.5, 3.5]); strokeSegs(ctx, broken);
        }
        ctx.setLineDash([]);
      }
      // the front: wet ink where the two touch
      const cf = world.contact;
      for (let i = 0; i < GW * GH; i++) tmpF[i] = Math.min(1, cf[i] * 2.5);
      for (const [lv, lw, al] of [[0.05, 3.4, 0.16], [0.14, 2.0, 0.6], [0.35, 1.3, 0.95]]) {
        const segs = []; contours(tmpF, lv, segs);
        ctx.strokeStyle = `rgba(24,20,16,${al})`; ctx.lineWidth = lw; strokeSegs(ctx, segs);
      }
      // flecks thrown off the front
      ctx.fillStyle = 'rgba(24,20,16,0.85)';
      for (let y = 1; y < GH - 1; y++) for (let x = 1; x < GW - 1; x++) {
        const v = tmpF[y * GW + x]; if (v < 0.08) continue;
        if (rnd() > v * 0.5) continue;
        const px = (x + rnd()) * cs, py = (y + rnd()) * cs, r = 0.4 + rnd() * 1.1 * Math.min(1, v * 2);
        ctx.beginPath(); ctx.arc(px, py, r, 0, 7); ctx.fill();
      }
      // will: the strokes of ink, and the pins
      for (const A of world.armies) {
        const col = COL[A.name];
        for (const s of A.strokes) {
          const age = world.t - s.born, al = Math.max(0, Math.min(1, (s.life - age) / 20)) * 0.75;
          if (al <= 0 || s.pts.length < 2) continue;
          ctx.strokeStyle = `rgba(${col},${al})`; ctx.lineWidth = 1.4; ctx.setLineDash([]);
          ctx.beginPath(); s.pts.forEach((p, k) => k ? ctx.lineTo(p[0] * cs, p[1] * cs) : ctx.moveTo(p[0] * cs, p[1] * cs)); ctx.stroke();
          // the wet end
          const e = s.pts[s.pts.length - 1], p = s.pts[Math.max(0, s.pts.length - 3)];
          const ang = Math.atan2(e[1] - p[1], e[0] - p[0]);
          ctx.beginPath(); ctx.moveTo(e[0] * cs + Math.cos(ang) * 5, e[1] * cs + Math.sin(ang) * 5);
          ctx.lineTo(e[0] * cs + Math.cos(ang + 2.5) * 6, e[1] * cs + Math.sin(ang + 2.5) * 6);
          ctx.lineTo(e[0] * cs + Math.cos(ang - 2.5) * 6, e[1] * cs + Math.sin(ang - 2.5) * 6); ctx.closePath();
          ctx.fillStyle = `rgba(${col},${al})`; ctx.fill();
        }
        for (const p of A.pins) {
          const fade = p.held ? 1 : Math.max(0, 1 - (world.t - p.released) / 30);
          const pulse = 1 + 0.08 * Math.sin(world.t * 1.3);
          ctx.strokeStyle = `rgba(${col},${0.8 * fade})`; ctx.lineWidth = 1.3;
          for (let k = 0; k < 18; k++) {
            const a0 = k / 18 * Math.PI * 2;
            ctx.beginPath(); ctx.arc(p.x * cs, p.y * cs, p.r * cs * 0.9 * pulse, a0, a0 + 0.18); ctx.stroke();
          }
          ctx.beginPath(); ctx.arc(p.x * cs, p.y * cs, 2.2, 0, 7); ctx.fillStyle = `rgba(${col},${0.9 * fade})`; ctx.fill();
        }
      }
    };
    resize();
    window.addEventListener('resize', resize);
  }
  window.PFPainter = Painter;
})();

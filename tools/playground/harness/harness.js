// The preview harness: a stand-in for the app's host, so a web prototype can be tried on a desktop.
// It plays the host's part of docs/playground.md: a globe with the camera semantics of terra.js
// (lon, lat, zoom; radius 0.42 x the shorter side x zoom), markers, an overlay told the camera and
// the taps, a screen with a Back button in its top-left 56 pt, and the reader's settings.
// The globe is only a calm picture (Natural Earth land, orthographic); the app draws its own.
//
// Runtime 2: a pack asks the host to draw geography on the globe (terra.geo), to show a date control and a caption
// at the foot of the screen (terra.time, terra.caption) and to quieten the globe (terra.globe). This host does the
// same on the stand-in: the layers are drawn on the globe's canvas with terra.project's own arithmetic, every
// frame, in the styles and sizes of Prototypes/_runtime/terra.js (points x the device's reading scale,
// Globe/type.json); the foot is DOM; the date is told to the page as 'time' the way a tap is told as 'tap'.
// A pack whose manifest says "runtime": 2 also gets what `globe.historical` and `globe.still` have to act on:
// today's borders and country names drawn on the stand-in, and the globe's idle turn (4 degrees a second, west
// to east, after 8 s alone; ?idleAfter=<seconds> shortens the wait). A runtime-1 pack is drawn exactly as before.
// Preview options (query): device=phone|ipad, size=<w>x<h> (points), textScale, reduceMotion=1, idleAfter.
(function () {
  'use strict';
  const P = window.__proto, M = P.manifest, ID = P.id;
  const SIZES = { phone: [390, 844], ipad: [1024, 1366] };
  const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
  const sizeQuery = /^(\d{3,4})x(\d{3,4})$/.exec(P.query.size || '');
  const [W, H] = sizeQuery ? [clamp(+sizeQuery[1], 200, 4000), clamp(+sizeQuery[2], 200, 4000)] : SIZES[P.device];
  const R2 = (M.runtime | 0) >= 2;
  const D = Math.PI / 180, TAU = 2 * Math.PI;
  const $ = (id) => document.getElementById(id);
  const t0 = performance.now();

  const device = $('device'), stage = $('stage'), canvas = $('globe'), markersEl = $('markers');
  device.style.width = W + 'px';
  device.style.height = H + 'px';
  $('frame').classList.add(P.device);
  $('title').textContent = M.title;
  $('meta').textContent = M.presentation + ' prototype, ' + (M.shelf || 'current') + ' shelf';
  for (const d of ['phone', 'ipad']) {
    const a = $('to-' + d);
    a.href = '?device=' + d;
    if (d === P.device) a.setAttribute('aria-current', 'page');
  }

  const settings = {
    reduceMotion: P.query.reduceMotion === '1',
    textScale: clamp(parseFloat(P.query.textScale) || 1, 1, 3.2),
    active: true,
  };
  const cam = M.anchor ? { lon: M.anchor.lon, lat: M.anchor.lat, zoom: M.anchor.zoom } : { lon: -40, lat: 22, zoom: 1 };
  let overlay = null, screen = null, markers = [], ended = false;

  // What a runtime-2 pack has asked the host for (all of it ends with the prototype)
  const host = {
    geo: new Map(),                                  // id -> layer, in the order the ids first arrived
    time: null,                                      // { unit, min, max, value, ticks, playable, rate, playing } while shown
    caption: { title: '', text: '' },
    globe: { historical: false, still: false },
  };

  const api = window.__playground = {
    ready: { overlay: false, screen: false },
    log: [],
    settings,
    camera: () => camera(),
    openScreen: (params) => openScreen(params || {}),
    setSettings: (patch) => applySettings(patch),
    flyTo: (lon, lat, zoom, seconds) => flyTo(lon, lat, zoom, seconds),
    closeScreen: () => closeScreen(),
    // runtime 2, for checks: what the host holds, the reading scale it draws at, and a promise for the fonts
    reading: () => rs,
    geo: () => [...host.geo.values()].map((l) => ({ id: l.id, style: l.style, options: l.options, lines: l.lines.length, polygons: l.polys.length, points: l.points.length })),
    time: () => host.time && Object.assign({}, host.time, { text: timeText(host.time) }),
    caption: () => Object.assign({}, host.caption),
    globe: () => Object.assign({}, host.globe),
    politics: () => !!(R2 && politics && !host.globe.historical),
    fonts: Promise.resolve(),
  };
  // the current value of the date the pack shows, or null: for a script to read
  window.harnessTime = () => (host.time ? host.time.value : null);

  // ---- reading scale (Globe/type.json `reading`: linear in the shorter side, clamped; 1.0 on a phone, 1.2 on a 13-inch iPad)
  let TYPE = { reading: { shortSide: [430, 1032], factor: [1.0, 1.2] } };
  const readingScale = () => {
    const [s0, s1] = TYPE.reading.shortSide, [f0, f1] = TYPE.reading.factor;
    return f0 + (f1 - f0) * clamp((Math.min(W, H) - s0) / (s1 - s0), 0, 1);
  };
  let rs = readingScale();
  if (R2) {
    fetch('/_harness/type.json').then((r) => r.json()).then((t) => {
      const r = t && t.reading;
      if (r && Array.isArray(r.shortSide) && Array.isArray(r.factor) && r.shortSide[1] > r.shortSide[0]) { TYPE = t; rs = readingScale(); schedule(); }
    }).catch((e) => log('harness', 'warn', 'type spec not loaded: ' + e.message));
    api.fonts = Promise.all(['400 15px', 'italic 400 15px', '600 15px'].map((f) => document.fonts.load(f + ' "Libertinus Serif"')))
      .then(() => schedule()).catch((e) => log('harness', 'warn', 'fonts not loaded: ' + e.message));
  }

  // ---- log
  function log(source, level, message) {
    const entry = { t: (performance.now() - t0) / 1000, source, level, message: String(message) };
    api.log.push(entry);
    const el = $('log'), li = document.createElement('li');
    li.className = level;
    li.textContent = '+' + entry.t.toFixed(1) + 's ' + source + (level === 'info' ? '' : ' ' + level) + ': ' + entry.message;
    el.appendChild(li);
    while (el.children.length > 200) el.removeChild(el.firstChild);
    el.scrollTop = el.scrollHeight;
  }
  window.addEventListener('error', (e) => log('harness', 'error', e.message));
  window.addEventListener('unhandledrejection', (e) => log('harness', 'error', e.reason));

  // ---- camera
  function camera() {
    return { lon: cam.lon, lat: cam.lat, zoom: cam.zoom, cx: W / 2, cy: H / 2,
             radius: 0.42 * Math.min(W, H) * cam.zoom, width: W, height: H };
  }
  function wrapLon(l) { return ((l + 540) % 360) - 180; }
  function cameraChanged() {
    updateMarkers();
    if (overlay && api.ready.overlay) post(overlay, { type: 'camera', camera: camera() });
    $('camera').textContent = 'lon ' + cam.lon.toFixed(1) + ', lat ' + cam.lat.toFixed(1) + ', zoom ' + cam.zoom.toFixed(2);
    schedule();
  }

  let flight = 0, flying = false, lastActivity = performance.now();
  function flyTo(lon, lat, zoom, seconds) {
    cancelAnimationFrame(flight);
    flying = false;
    const from = { lon: cam.lon, lat: cam.lat, zoom: cam.zoom };
    const to = { lon: +lon, lat: clamp(+lat, -85, 85), zoom: zoom == null ? cam.zoom : clamp(+zoom, 0.8, 8) };
    let dlon = wrapLon(to.lon - from.lon);
    const ms = (seconds == null ? 0.8 : +seconds) * 1000;
    if (settings.reduceMotion || ms <= 0) {           // Reduce Motion: arrive, don't travel
      Object.assign(cam, { lon: wrapLon(to.lon), lat: to.lat, zoom: to.zoom });
      return cameraChanged();
    }
    const start = performance.now();
    flying = true;
    (function step(now) {
      const k = clamp((now - start) / ms, 0, 1), e = k < .5 ? 2 * k * k : 1 - Math.pow(-2 * k + 2, 2) / 2;
      cam.lon = wrapLon(from.lon + dlon * e);
      cam.lat = from.lat + (to.lat - from.lat) * e;
      cam.zoom = from.zoom + (to.zoom - from.zoom) * e;
      cameraChanged();
      if (k < 1) flight = requestAnimationFrame(step); else flying = false;
    })(start);
  }

  // ---- the stand-in globe
  let rings = [], politics = null;
  fetch('/_harness/land-50m.json').then((r) => r.json()).then((topo) => {
    const arcs = decodeArcs(topo);
    for (const g of topo.objects.land.geometries) {
      for (const poly of g.type === 'Polygon' ? [g.arcs] : g.arcs) for (const r of poly) rings.push(ring(arcsPoints(arcs, r), true));
    }
    schedule();
  }).catch((e) => log('harness', 'warn', 'land data not loaded: ' + e.message));
  // Today's borders and names, for the packs that can ask to hide them (runtime 2)
  if (R2) {
    fetch('/_harness/countries-50m.json').then((r) => r.json()).then((topo) => { politics = decodePolitics(topo); schedule(); })
      .catch((e) => log('harness', 'warn', 'country data not loaded: ' + e.message));
  }

  // A ring or a line as unit vectors on the sphere, with room for the same points in view space.
  // `pts`: [lon, lat] pairs; `closed` drops a last point that repeats the first (a polygon's ring).
  function ring(pts, closed) {
    if (closed && pts.length > 1) {
      const last = pts[pts.length - 1], first = pts[0];
      if (last[0] === first[0] && last[1] === first[1]) pts = pts.slice(0, -1);
    }
    const n = pts.length, w = new Float32Array(3 * n);
    pts.forEach((p, i) => {
      const lo = p[0] * D, la = p[1] * D;
      w[3 * i] = Math.cos(la) * Math.sin(lo); w[3 * i + 1] = Math.sin(la); w[3 * i + 2] = Math.cos(la) * Math.cos(lo);
    });
    return { n, w, vx: new Float32Array(n), vy: new Float32Array(n), vz: new Float32Array(n) };
  }

  // TopoJSON (quantized, delta-encoded arcs) to arcs of [lon, lat]
  function decodeArcs(topo) {
    const [sx, sy] = topo.transform.scale, [tx, ty] = topo.transform.translate;
    return topo.arcs.map((arc) => {
      let x = 0, y = 0;
      return arc.map((p) => { x += p[0]; y += p[1]; return [x * sx + tx, y * sy + ty]; });
    });
  }
  function arcsPoints(arcs, indexes) {
    const pts = [];
    for (const i of indexes) {
      const a = i < 0 ? arcs[~i].slice().reverse() : arcs[i];
      for (let k = pts.length ? 1 : 0; k < a.length; k++) pts.push(a[k]);
    }
    return pts;
  }

  // Borders: the arcs two countries share. Names: where each country's largest piece has its middle, and
  // about how big it is (radians of arc), so a name is shown only where it fits.
  function decodePolitics(topo) {
    const arcs = decodeArcs(topo), used = new Map(), names = [];
    for (const g of topo.objects.countries.geometries) {
      if (!g.arcs) continue;
      let best = null;
      for (const poly of g.type === 'Polygon' ? [g.arcs] : g.arcs) {
        for (const r of poly) for (const i of r) used.set(i < 0 ? ~i : i, (used.get(i < 0 ? ~i : i) || 0) + 1);
        const pts = arcsPoints(arcs, poly[0]);
        let area = 0, cx = 0, cy = 0;
        for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) {
          const f = pts[j][0] * pts[i][1] - pts[i][0] * pts[j][1];
          area += f; cx += (pts[j][0] + pts[i][0]) * f; cy += (pts[j][1] + pts[i][1]) * f;
        }
        if (area && (!best || Math.abs(area) > best.area)) best = { area: Math.abs(area), lon: cx / (3 * area), lat: cy / (3 * area) };
      }
      const name = g.properties && g.properties.name;
      if (best && name) {
        const la = best.lat * D;
        names.push({ name, size: Math.sqrt(best.area * D * D * Math.max(Math.cos(la), 0.2)),
                     wx: Math.cos(la) * Math.sin(best.lon * D), wy: Math.sin(la), wz: Math.cos(la) * Math.cos(best.lon * D) });
      }
    }
    names.sort((a, b) => b.size - a.size);
    const borders = [];
    for (const [i, count] of used) if (count > 1) borders.push(ring(arcs[i], false));
    return { borders, names };
  }

  function limbArc(path, cx, cy, R, from, to) {       // along the horizon the short way round
    let d = to - from;
    while (d > Math.PI) d -= TAU;
    while (d < -Math.PI) d += TAU;
    path.arc(cx, cy, R, from, from + d, d < 0);
  }

  // The camera as a rotation (terra.project's, term for term) and a ring or line carried through it
  function rotation() {
    const a = cam.lon * D, b = -cam.lat * D;
    return { ca: Math.cos(a), sa: Math.sin(a), cb: Math.cos(b), sb: Math.sin(b) };
  }
  function rotate(r, m) {
    const { n, w, vx, vy, vz } = r;
    for (let i = 0; i < n; i++) {
      const x = w[3 * i], y = w[3 * i + 1], z = w[3 * i + 2];
      vx[i] = m.ca * x - m.sa * z;
      vy[i] = m.sa * m.sb * x + m.cb * y + m.ca * m.sb * z;
      vz[i] = m.sa * m.cb * x - m.sb * y + m.ca * m.cb * z;
    }
  }
  // where the edge i-j of a rotated ring meets the horizon, as a point on the unit circle
  function crossing(r, i, j) {
    const { vx, vy, vz } = r;
    const t = vz[i] / (vz[i] - vz[j]);
    const px = vx[i] + t * (vx[j] - vx[i]), py = vy[i] + t * (vy[j] - vy[i]), len = Math.hypot(px, py) || 1;
    return [px / len, py / len];
  }

  // One ring in view space (the same rotation as terra.project). Points behind the globe are pushed to
  // the horizon so the fill follows the limb; the coast is stroked only where it is really seen.
  function trace(r, m, cx, cy, R, fill, stroke) {
    const { n, vx, vy, vz } = r;
    rotate(r, m);
    let first = -1, seen = 0;
    for (let i = 0; i < n; i++) if (vz[i] > 0) { seen++; if (first < 0) first = i; }
    if (!seen) return;
    const X = (i) => cx + vx[i] * R, Y = (i) => cy - vy[i] * R;
    const cross = (i, j) => crossing(r, i, j);
    fill.moveTo(X(first), Y(first));
    stroke.moveTo(X(first), Y(first));
    if (seen === n) {
      for (let k = 1; k < n; k++) { const i = (first + k) % n; fill.lineTo(X(i), Y(i)); stroke.lineTo(X(i), Y(i)); }
      fill.closePath(); stroke.closePath();
      return;
    }
    let angle = 0;
    for (let k = 0; k < n; k++) {
      const i = (first + k) % n, j = (first + k + 1) % n, a = vz[i] > 0, b = vz[j] > 0;
      if (a && b) {
        fill.lineTo(X(j), Y(j)); stroke.lineTo(X(j), Y(j));
      } else if (a) {                                  // goes out of sight
        const [qx, qy] = cross(i, j);
        fill.lineTo(cx + qx * R, cy - qy * R); stroke.lineTo(cx + qx * R, cy - qy * R);
        angle = Math.atan2(-qy, qx);
      } else if (!b) {                                 // still hidden: walk the limb
        const to = Math.atan2(-vy[j], vx[j]);
        limbArc(fill, cx, cy, R, angle, to);
        angle = to;
      } else {                                         // comes back
        const [qx, qy] = cross(i, j), to = Math.atan2(-qy, qx);
        limbArc(fill, cx, cy, R, angle, to);
        fill.lineTo(cx + qx * R, cy - qy * R); fill.lineTo(X(j), Y(j));
        stroke.moveTo(cx + qx * R, cy - qy * R); stroke.lineTo(X(j), Y(j));
      }
    }
    fill.closePath();
  }

  // A line (or a closed ring) in view space, drawn only where it is on the near side: it breaks where it crosses behind.
  function traceLine(r, m, cx, cy, R, path, closed) {
    const { n, vx, vy, vz } = r;
    rotate(r, m);
    const X = (i) => cx + vx[i] * R, Y = (i) => cy - vy[i] * R;
    let drawing = false;
    for (let i = 0, edges = closed ? n : n - 1; i < edges; i++) {
      const j = (i + 1) % n, a = vz[i] > 0, b = vz[j] > 0;
      if (a && b) {
        if (!drawing) { path.moveTo(X(i), Y(i)); drawing = true; }
        path.lineTo(X(j), Y(j));
      } else if (a) {
        if (!drawing) path.moveTo(X(i), Y(i));
        const [qx, qy] = crossing(r, i, j);
        path.lineTo(cx + qx * R, cy - qy * R);
        drawing = false;
      } else if (b) {
        const [qx, qy] = crossing(r, i, j);
        path.moveTo(cx + qx * R, cy - qy * R); path.lineTo(X(j), Y(j));
        drawing = true;
      } else drawing = false;
    }
    if (n === 1 && vz[0] > 0) { path.moveTo(X(0), Y(0)); path.lineTo(X(0), Y(0)); }
  }

  let pending = 0;
  function schedule() { if (!pending) pending = requestAnimationFrame(() => { pending = 0; render(); }); }
  function render() {
    const dpr = window.devicePixelRatio || 1;
    if (canvas.width !== Math.round(W * dpr)) { canvas.width = Math.round(W * dpr); canvas.height = Math.round(H * dpr); }
    const g = canvas.getContext('2d'), c = camera();
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, W, H);
    g.save();
    g.beginPath(); g.arc(c.cx, c.cy, c.radius, 0, TAU); g.fillStyle = '#C6ECFF'; g.fill(); g.clip();
    const m = rotation();
    const fill = new Path2D(), stroke = new Path2D();
    for (const r of rings) trace(r, m, c.cx, c.cy, c.radius, fill, stroke);
    g.fillStyle = '#F6E1B9'; g.fill(fill, 'evenodd');
    if (R2 && politics && !host.globe.historical) drawBorders(g, c, m);
    g.lineWidth = 0.75; g.lineJoin = 'round'; g.strokeStyle = '#0978AB'; g.stroke(stroke);
    g.restore();
    g.beginPath(); g.arc(c.cx, c.cy, c.radius, 0, TAU); g.lineWidth = 1; g.strokeStyle = 'rgba(9, 120, 171, .55)'; g.stroke();
    if (R2 && politics && !host.globe.historical) drawCountryNames(g, c, m);
    if (host.geo.size) drawGeo(g, c, m);
  }

  // ---- what the host draws of today's politics (runtime 2 only): borders, and country names where they fit
  const BORDER_INK = '#646464', NAME_INK = '#5F5A51', NAME_HALO = '#F6E1B9';
  function drawBorders(g, c, m) {
    const path = new Path2D();
    for (const r of politics.borders) traceLine(r, m, c.cx, c.cy, c.radius, path, false);
    g.lineWidth = 0.75 * rs; g.lineJoin = 'round'; g.lineCap = 'round'; g.strokeStyle = BORDER_INK; g.stroke(path);
  }
  function drawCountryNames(g, c, m) {
    const size = 12 * rs, placed = [];
    g.font = '400 ' + size + 'px "Libertinus Serif"'; g.textAlign = 'center'; g.textBaseline = 'middle';
    if ('letterSpacing' in g) g.letterSpacing = (0.09 * size) + 'px';
    g.lineJoin = 'round'; g.lineWidth = 3 * rs;
    const o = { x: 0, y: 0, z: 0 };
    for (const n of politics.names) {
      projectUnit(n, m, c, o);
      if (o.z < 0.4 || n.size * c.radius < 12 * rs) continue;
      const text = n.name.toUpperCase(), w = g.measureText(text).width;
      if (n.size * c.radius < w * 0.7) continue;
      const box = [o.x - w / 2 - 2, o.y - size * 0.6 - 2, w + 4, size * 1.2 + 4];
      if (box[0] < 0 || box[1] < 0 || box[0] + box[2] > W || box[1] + box[3] > H || placed.some((q) => overlaps(box, q))) continue;
      placed.push(box);
      g.strokeStyle = NAME_HALO; g.strokeText(text, o.x, o.y);
      g.fillStyle = NAME_INK; g.fillText(text, o.x, o.y);
    }
    if ('letterSpacing' in g) g.letterSpacing = '0px';
  }
  const overlaps = (a, b) => a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3];
  // a place (its unit vector) through the rotation, as terra.project does: screen x, y and facing z
  function projectUnit(p, m, c, out) {
    const x = p.wx, y = p.wy, z = p.wz;
    out.x = c.cx + (m.ca * x - m.sa * z) * c.radius;
    out.y = c.cy - (m.sa * m.sb * x + m.cb * y + m.ca * m.sb * z) * c.radius;
    out.z = m.sa * m.cb * x - m.sb * y + m.ca * m.cb * z;
  }

  // ---- terra.geo: geography drawn on the globe, every frame, in layer order
  const STYLE_KIND = { border: 'line', ghost: 'line', track: 'line', area: 'area', dot: 'point', ring: 'point', label: 'point', symbol: 'point', sounding: 'point' };
  const INK = '#3B4A54', PAPER = '#F6F2E7', AREA_COLOR = '#C17846', SOUNDING_LINE = '#1C7AB0', SOUNDING_INK = '#1C5C8C';
  const FACE = 'Libertinus Serif';

  function parseColor(v) {
    const m = typeof v === 'string' && /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(v.trim());
    if (!m) return null;
    const h = m[1].length === 3 ? m[1].replace(/./g, (x) => x + x) : m[1];
    return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
  }
  const css = (rgb, k) => 'rgb(' + rgb.map((v) => Math.round(v * (k == null ? 1 : k))).join(',') + ')';
  const num = (v, fallback) => (typeof v === 'number' && Number.isFinite(v) ? v : fallback);

  // A layer's features as rings, lines and points (unit vectors, ready to be carried through the camera)
  function parseLayer(id, features, options, source) {
    const style = options.style, kind = STYLE_KIND[style];
    const color = parseColor(options.color);
    const L = { id, style, options, kind, color, lines: [], polys: [], points: [],
                minZoom: num(options.minZoom, -Infinity), maxZoom: num(options.maxZoom, Infinity), offset: [0, 0] };
    const bad = [];
    if (options.color != null && !color) bad.push('options.color must be "#rrggbb"');
    if (options.offset != null) {
      const o = options.offset;
      if (Array.isArray(o) && o.length === 2 && Number.isFinite(o[0]) && Number.isFinite(o[1])) L.offset = [o[0], o[1]];
      else bad.push('options.offset must be [dx, dy] in points');
    }
    if (style === 'symbol') {
      if (typeof options.image !== 'string' || !packFile(options.image)) bad.push('a symbol layer needs options.image, a path inside the pack');
      if (!(options.width > 0) || !(options.height > 0)) bad.push('a symbol layer needs options.width and options.height, in points');
    }
    if (options.font != null && !['regular', 'italic', 'caps'].includes(options.font)) bad.push('options.font is regular, italic or caps');
    for (const b of bad) log(source, 'warn', 'terra.geo.set("' + id + '"): ' + b);
    let skipped = 0;
    const coord = (c) => (Array.isArray(c) && Number.isFinite(c[0]) && Number.isFinite(c[1]) && Math.abs(c[1]) <= 90 ? [c[0], c[1]] : null);
    const coords = (list, min) => {
      const pts = Array.isArray(list) ? list.map(coord) : [];
      return pts.length >= min && pts.every(Boolean) ? pts : null;
    };
    const point = (c, props) => {
      const p = coord(c);
      if (!p) { skipped++; return; }
      const lo = p[0] * D, la = p[1] * D;
      L.points.push({ lon: p[0], lat: p[1], props, wx: Math.cos(la) * Math.sin(lo), wy: Math.sin(la), wz: Math.cos(la) * Math.cos(lo) });
    };
    const polygon = (rs_, props) => {
      const parts = Array.isArray(rs_) ? rs_.map((r) => coords(r, 3)) : [];
      if (!parts.length || !parts.every(Boolean)) { skipped++; return; }
      if (kind === 'area') L.polys.push(parts.map((r) => ring(r, true)));
      else for (const r of parts) L.lines.push({ ring: ring(r, true), closed: true });
    };
    const line = (c) => {
      const pts = coords(c, 2);
      if (!pts) { skipped++; return; }
      L.lines.push({ ring: ring(pts, false), closed: false });
    };
    const walk = (geom, props) => {
      if (!geom || typeof geom !== 'object') { skipped++; return; }
      const t = geom.type, c = geom.coordinates;
      if (t === 'GeometryCollection') return (geom.geometries || []).forEach((x) => walk(x, props));
      const fits = (kind === 'point' && (t === 'Point' || t === 'MultiPoint')) || (kind === 'line' && /^(Multi)?(LineString|Polygon)$/.test(t))
        || (kind === 'area' && /^(Multi)?Polygon$/.test(t));
      if (!fits) { skipped++; return; }
      if (t === 'Point') point(c, props);
      else if (t === 'MultiPoint') (Array.isArray(c) ? c : []).forEach((x) => point(x, props));
      else if (t === 'LineString') line(c);
      else if (t === 'MultiLineString') (Array.isArray(c) ? c : []).forEach(line);
      else if (t === 'Polygon') polygon(c, props);
      else (Array.isArray(c) ? c : []).forEach((x) => polygon(x, props));
    };
    const list = Array.isArray(features) ? features : features && features.type === 'FeatureCollection' ? features.features || [] : features ? [features] : [];
    for (const f of list) {
      if (f && f.type === 'Feature') walk(f.geometry, f.properties && typeof f.properties === 'object' ? f.properties : {});
      else walk(f, {});
    }
    if (skipped) log(source, 'warn', 'terra.geo.set("' + id + '"): ' + skipped + ' feature' + (skipped === 1 ? '' : 's') + ' skipped: a "' + style + '" layer draws '
      + (kind === 'point' ? 'Point and MultiPoint' : kind === 'area' ? 'Polygon and MultiPolygon' : 'LineString, MultiLineString, Polygon and MultiPolygon') + ' with finite [lon, lat] coordinates');
    return L;
  }

  function onGeo(msg, source) {
    if (msg.op === 'clear') { host.geo.clear(); return schedule(); }
    const id = msg.id == null ? '' : String(msg.id);
    if (msg.op === 'remove') { host.geo.delete(id); return schedule(); }
    if (msg.op !== 'set') return log(source, 'warn', 'terra.geo: unknown op "' + msg.op + '"; ignored');
    const options = msg.options && typeof msg.options === 'object' && !Array.isArray(msg.options) ? msg.options : {};
    if (!STYLE_KIND[options.style]) {
      return log(source, 'error', 'terra.geo.set("' + id + '"): options.style must be one of ' + Object.keys(STYLE_KIND).join(', ') + ' (got ' + JSON.stringify(options.style) + '); layer not drawn');
    }
    host.geo.set(id, parseLayer(id, msg.features, options, source));    // a known id keeps its place in the order
    schedule();
  }

  // a file of the pack, as the host would fetch it: a relative path inside the pack, nothing else
  function packFile(path) {
    if (typeof path !== 'string') return null;
    const p = path.replace(/^\.\//, '');
    if (!p || p.startsWith('/') || /^[a-z][a-z0-9+.-]*:/i.test(p) || p.split('/').some((x) => x === '..' || x === '')) return null;
    return '/pack/' + ID + '/' + p.split('/').map(encodeURIComponent).join('/');
  }
  const images = new Map();
  function symbolImage(path) {
    let e = images.get(path);
    if (e) return e;
    e = { img: new Image(), ok: false };
    images.set(path, e);
    e.img.onload = () => { e.ok = true; schedule(); };
    e.img.onerror = () => log('overlay', 'error', 'symbol image did not load: ' + path);
    e.img.src = packFile(path);
    return e;
  }

  function labelFont(font, size) {
    return (font === 'italic' ? 'italic 400 ' : font === 'caps' ? '600 ' : '400 ') + size + 'px "' + FACE + '"';
  }
  function halo(g, text, x, y, size) {
    g.lineJoin = 'round'; g.lineWidth = 3 * rs; g.strokeStyle = PAPER; g.strokeText(text, x, y);
  }

  function drawGeo(g, c, m) {
    const placed = [], o = { x: 0, y: 0, z: 0 };
    g.save();
    g.lineJoin = 'round'; g.lineCap = 'round';
    for (const L of host.geo.values()) {
      if (cam.zoom < L.minZoom - 1e-9 || cam.zoom > L.maxZoom + 1e-9) continue;
      g.save();
      const stroke = () => {
        const path = new Path2D();
        for (const l of L.lines) traceLine(l.ring, m, c.cx, c.cy, c.radius, path, l.closed);
        return path;
      };
      const clipDisc = () => { g.beginPath(); g.arc(c.cx, c.cy, c.radius, 0, TAU); g.clip(); };
      const dx = L.offset[0] * rs, dy = L.offset[1] * rs;
      switch (L.style) {
        case 'border':
          clipDisc();
          g.lineWidth = 1.1 * rs; g.strokeStyle = '#8C8478'; g.stroke(stroke());
          break;
        case 'area': {
          clipDisc();
          const rgb = L.color || parseColor(AREA_COLOR), edge = new Path2D();
          g.fillStyle = css(rgb); g.globalAlpha = 0.28;
          for (const poly of L.polys) {
            const fill = new Path2D();
            for (const r of poly) trace(r, m, c.cx, c.cy, c.radius, fill, edge);
            g.fill(fill, 'evenodd');
          }
          g.globalAlpha = 1; g.lineWidth = 2.2 * rs; g.strokeStyle = css(rgb, 0.7); g.stroke(edge);
          break;
        }
        case 'ghost':
          clipDisc();
          g.lineCap = 'butt'; g.setLineDash([5 * rs, 4 * rs]); g.lineWidth = 1.6 * rs; g.strokeStyle = INK; g.globalAlpha = 0.8; g.stroke(stroke());
          break;
        case 'track':
          clipDisc();
          g.lineCap = 'butt'; g.setLineDash([3 * rs, 4 * rs]); g.lineWidth = 1.2 * rs; g.strokeStyle = INK; g.globalAlpha = 0.47; g.stroke(stroke());
          break;
        case 'dot':
        case 'ring':
          g.fillStyle = L.style === 'dot' ? (L.color ? css(L.color) : INK) : PAPER; g.strokeStyle = INK; g.lineWidth = 1.3 * rs;
          for (const p of L.points) {
            projectUnit(p, m, c, o);
            if (o.z <= 0) continue;
            g.beginPath(); g.arc(o.x, o.y, 3.4 * rs, 0, TAU); g.fill();
            if (L.style === 'ring') g.stroke();
          }
          break;
        case 'label': {
          const size = num(L.options.size, 15) * rs, caps = L.options.font === 'caps';
          g.font = labelFont(L.options.font, size); g.textAlign = 'center'; g.textBaseline = 'middle';
          if ('letterSpacing' in g) g.letterSpacing = caps ? 0.08 * size + 'px' : '0px';
          const ink = L.color ? css(L.color) : INK, pad = 1.5 * rs;
          for (const p of L.points) {
            projectUnit(p, m, c, o);
            if (o.z <= 0) continue;
            let text = p.props.label == null ? '' : String(p.props.label);
            if (!text) continue;
            if (caps) text = text.toUpperCase();
            const x = o.x + dx, y = o.y + dy, w = g.measureText(text).width;
            const box = [x - w / 2 - pad, y - size * 0.6 - pad, w + 2 * pad, size * 1.2 + 2 * pad];
            if (box[0] > W || box[1] > H || box[0] + box[2] < 0 || box[1] + box[3] < 0) continue;
            if (placed.some((q) => overlaps(box, q))) continue;         // earlier features win
            placed.push(box);
            halo(g, text, x, y, size);
            g.fillStyle = ink; g.fillText(text, x, y);
          }
          break;
        }
        case 'symbol': {
          const e = symbolImage(L.options.image);
          if (!e.ok) break;
          const w = L.options.width * rs, h = L.options.height * rs;
          for (const p of L.points) {
            projectUnit(p, m, c, o);
            if (o.z <= 0) continue;
            g.drawImage(e.img, o.x + dx - w / 2, o.y + dy - h / 2, w, h);
          }
          break;
        }
        case 'sounding':
          g.textBaseline = 'middle'; g.textAlign = 'left';
          g.font = labelFont('regular', 14 * rs);
          for (const p of L.points) {
            projectUnit(p, m, c, o);
            if (o.z <= 0) continue;
            const length = num(p.props.length, 0) * rs, y = o.y + length;
            g.strokeStyle = SOUNDING_LINE; g.lineCap = 'butt';
            g.setLineDash([2 * rs, 3 * rs]); g.lineWidth = 1.4 * rs;
            g.beginPath(); g.moveTo(o.x, o.y); g.lineTo(o.x, y); g.stroke();
            g.setLineDash([]); g.lineWidth = 2 * rs;
            g.beginPath(); g.moveTo(o.x - 8 * rs, y); g.lineTo(o.x + 8 * rs, y); g.stroke();
            const text = p.props.label == null ? '' : String(p.props.label);
            if (text) {
              halo(g, text, o.x + 12 * rs, y, 14 * rs);
              g.fillStyle = SOUNDING_INK; g.fillText(text, o.x + 12 * rs, y);
            }
          }
          break;
      }
      g.restore();
    }
    g.restore();
  }

  // ---- the foot of the screen: caption and date (terra.caption, terra.time)
  const foot = $('foot'), strip = $('date-strip'), playBtn = $('date-play');
  const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
  const PLAY_ICON = '<svg viewBox="0 0 44 44" aria-hidden="true"><circle cx="22" cy="22" r="15" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M19 15.5 L29.5 22 L19 28.5 Z" fill="currentColor"/></svg>';
  const PAUSE_ICON = '<svg viewBox="0 0 44 44" aria-hidden="true"><circle cx="22" cy="22" r="15" fill="none" stroke="currentColor" stroke-width="1.3"/><rect x="17.5" y="16" width="3.4" height="12" fill="currentColor"/><rect x="23.1" y="16" width="3.4" height="12" fill="currentColor"/></svg>';
  let needle = null, playRaf = 0, lastPlay = 0, lastSent = -1e9;

  // 'year': the integer year, negative as "N BCE". 'day' (days since 1970-01-01 UTC): "26 February 1873".
  function timeText(t) {
    if (t.unit === 'day') {
      const d = new Date(Math.floor(t.value) * 86400000);
      if (isNaN(d.getTime())) return '';
      const y = d.getUTCFullYear();
      return d.getUTCDate() + ' ' + MONTHS[d.getUTCMonth()] + ' ' + (y > 0 ? y : (1 - y) + ' BCE');
    }
    const y = Math.floor(t.value);
    return y < 0 ? -y + ' BCE' : String(y);
  }

  function updateFoot() {
    const t = host.time, cap = host.caption;
    foot.hidden = !(t || cap.title || cap.text);
    if (foot.hidden) return;
    const set = (id, text) => { const el = $(id); el.textContent = text; el.hidden = !text; };
    set('foot-eyebrow', cap.title);
    set('foot-title', t ? timeText(t) : '');
    set('foot-text', cap.text);
    $('foot-time').hidden = !t;
    if (!t) return;
    needle.style.setProperty('--f', String((t.value - t.min) / (t.max - t.min)));
    strip.setAttribute('aria-valuemin', t.min); strip.setAttribute('aria-valuemax', t.max);
    strip.setAttribute('aria-valuenow', t.value); strip.setAttribute('aria-valuetext', timeText(t));
    playBtn.hidden = !t.playable || settings.reduceMotion;
    playBtn.innerHTML = t.playing ? PAUSE_ICON : PLAY_ICON;
    playBtn.setAttribute('aria-label', t.playing ? 'Pause' : 'Play');
    playBtn.dataset.playing = t.playing ? 'true' : 'false';
  }

  function buildStrip() {
    const t = host.time;
    strip.textContent = '';
    const line = document.createElement('div');
    line.className = 'line';
    strip.appendChild(line);
    for (const tick of t.ticks) {
      const el = document.createElement('div');
      el.className = 'at tick' + (tick.hollow ? ' hollow' : '');
      el.style.setProperty('--f', String((tick.at - t.min) / (t.max - t.min)));
      strip.appendChild(el);
    }
    needle = document.createElement('div');
    needle.className = 'at needle';
    needle.setAttribute('data-testid', 'date-needle');
    strip.appendChild(needle);
  }

  function showTime(o, source) {
    if (!o || !Number.isFinite(o.min) || !Number.isFinite(o.max) || !(o.max > o.min)) {
      return log(source, 'error', 'terra.time.show: min and max must be numbers, max above min (got ' + JSON.stringify(o && [o.min, o.max]) + '); not shown');
    }
    if (o.unit !== 'year' && o.unit !== 'day') log(source, 'warn', 'terra.time.show: unit is "year" or "day" (got ' + JSON.stringify(o.unit) + '); read as years');
    stopPlayback();
    if (Number.isFinite(o.value) && (o.value < o.min || o.value > o.max)) log(source, 'warn', 'terra.time.show: value ' + o.value + ' is outside min-max; clamped');
    const ticks = [];
    for (const tk of Array.isArray(o.ticks) ? o.ticks : []) {
      const at = typeof tk === 'number' ? tk : tk && tk.at;
      if (Number.isFinite(at) && at >= o.min && at <= o.max) ticks.push({ at, hollow: !!(tk && typeof tk === 'object' && tk.hollow) });
    }
    host.time = { unit: o.unit === 'day' ? 'day' : 'year', min: o.min, max: o.max, value: clamp(Number.isFinite(o.value) ? o.value : o.min, o.min, o.max),
                  ticks, playable: !!o.playable, rate: o.rate > 0 ? +o.rate : 1, playing: false };
    buildStrip();
    if (o.playing && !settings.reduceMotion) startPlayback();
    updateFoot();
    if (api.ready.overlay) sendTime('host');          // otherwise when the page says ready
  }

  // tell the page the date, the way a tap is told: by 'reader' (a drag, the button), 'play' (a tick) or 'host' (the first value)
  function sendTime(by) {
    const t = host.time;
    if (t && overlay && api.ready.overlay) post(overlay, { type: 'time', value: t.value, playing: t.playing, by });
  }

  function startPlayback() {
    const t = host.time;
    if (!t || t.playing || settings.reduceMotion) return;
    if (t.value >= t.max) t.value = t.min;           // played to the end: again from the start
    t.playing = true;
    lastPlay = performance.now(); lastSent = -1e9;
    playRaf = requestAnimationFrame(playStep);
  }
  function stopPlayback() {
    cancelAnimationFrame(playRaf); playRaf = 0;
    if (host.time) host.time.playing = false;
  }
  function playStep(ts) {
    playRaf = 0;
    const t = host.time;
    if (!t || !t.playing) return;
    const dt = Math.min(Math.max(ts - lastPlay, 0) / 1000, 0.25);
    lastPlay = ts;
    t.value = Math.min(t.value + t.rate * dt, t.max);
    const done = t.value >= t.max;
    if (done) t.playing = false;
    updateFoot();
    if (done || ts - lastSent >= 1000 / 30) { lastSent = ts; sendTime('play'); }      // at most 30 a second, and always the last
    if (!done) playRaf = requestAnimationFrame(playStep);
  }

  // dragging or clicking anywhere on the strip sets the date in proportion; the button plays and pauses
  let scrubbing = false;
  function scrub(e) {
    const t = host.time;
    if (!t) return;
    const r = strip.getBoundingClientRect(), inset = 14;
    const f = clamp((e.clientX - r.left - inset) / Math.max(r.width - 2 * inset, 1), 0, 1);
    setValueByReader(t.min + f * (t.max - t.min));
  }
  function setValueByReader(v) {
    const t = host.time;
    stopPlayback();                                  // a hand on the date takes it from the clock
    const changed = v !== t.value;
    t.value = clamp(v, t.min, t.max);
    updateFoot();
    if (changed) sendTime('reader');
  }
  strip.addEventListener('pointerdown', (e) => {
    if (!host.time || (e.pointerType === 'mouse' && e.button !== 0)) return;
    scrubbing = true;
    strip.setPointerCapture(e.pointerId);
    strip.focus({ preventScroll: true });
    scrub(e);
    e.preventDefault();
  });
  strip.addEventListener('pointermove', (e) => { if (scrubbing) scrub(e); });
  const endScrub = () => { scrubbing = false; };
  strip.addEventListener('pointerup', endScrub);
  strip.addEventListener('pointercancel', endScrub);
  strip.addEventListener('keydown', (e) => {
    const t = host.time;
    if (!t) return;
    const step = (t.max - t.min) / (e.shiftKey ? 10 : 100);
    if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') setValueByReader(t.value - step);
    else if (e.key === 'ArrowRight' || e.key === 'ArrowUp') setValueByReader(t.value + step);
    else if (e.key === 'Home') setValueByReader(t.min);
    else if (e.key === 'End') setValueByReader(t.max);
    else return;
    e.preventDefault();
  });
  playBtn.addEventListener('click', () => {
    const t = host.time;
    if (!t) return;
    if (t.playing) stopPlayback(); else startPlayback();
    updateFoot();
    sendTime('reader');
  });

  function onTime(msg, source) {
    switch (msg.op) {
      case 'show': return showTime(msg.options, source);
      case 'hide': stopPlayback(); host.time = null; return updateFoot();
      case 'set':
        if (!host.time) return log(source, 'warn', 'terra.time.set before terra.time.show; ignored');
        host.time.value = clamp(Number.isFinite(msg.value) ? msg.value : host.time.value, host.time.min, host.time.max);    // not echoed: the page knows
        return updateFoot();
      case 'play':
        if (!host.time) return log(source, 'warn', 'terra.time.play before terra.time.show; ignored');
        if (msg.playing) startPlayback(); else stopPlayback();
        return updateFoot();
      default: log(source, 'warn', 'terra.time: unknown op "' + msg.op + '"; ignored');
    }
  }

  // ---- the globe's idle turn (runtime 2 only: the app's globe turns when left alone; terra.globe.set({ still: true }) stops it)
  const idleAfter = clamp(P.query.idleAfter === undefined ? 8 : parseFloat(P.query.idleAfter) || 0, 0, 3600);
  let lastTick = performance.now();
  function idleTick(now) {
    requestAnimationFrame(idleTick);
    const dt = Math.min(Math.max(now - lastTick, 0) / 1000, 0.25);
    lastTick = now;
    if (ended || screen || M.presentation !== 'globe' || host.globe.still || settings.reduceMotion || flying) return;
    if (pointers.size || now - lastActivity < idleAfter * 1000) return;
    cam.lon = wrapLon(cam.lon - 4 * dt);
    cameraChanged();
  }
  if (R2) requestAnimationFrame(idleTick);
  for (const type of ['pointerdown', 'pointermove', 'wheel', 'keydown']) stage.addEventListener(type, () => { lastActivity = performance.now(); }, { capture: true, passive: true });

  // everything a pack asked of the host ends with it
  function resetHost() {
    host.geo.clear();
    stopPlayback();
    host.time = null;
    host.caption = { title: '', text: '' };
    host.globe = { historical: false, still: false };
    updateFoot();
    schedule();
  }

  // ---- markers
  function makeMarkers() {
    for (const mk of M.markers || []) {
      const b = document.createElement('button');
      b.type = 'button'; b.className = 'marker'; b.setAttribute('aria-label', mk.label);
      b.style.width = mk.width + 'px'; b.style.height = mk.height + 'px';
      const img = document.createElement('img');
      img.alt = ''; img.width = mk.width; img.height = mk.height;
      img.src = '/pack/' + ID + '/' + mk.image.split('/').map(encodeURIComponent).join('/');
      img.addEventListener('error', () => log('harness', 'error', 'marker image did not load: ' + mk.image));
      b.appendChild(img);
      b.addEventListener('click', () => { if (mk.opens) openScreen({ marker: mk.id }); else log('harness', 'info', 'marker "' + mk.id + '" has no screen (opens is not true)'); });
      markersEl.appendChild(b);
      markers.push({ mk, el: b });
    }
    updateMarkers();
  }
  function updateMarkers() {
    const c = camera();
    for (const { mk, el } of markers) {
      const p = terra.project(mk.lon, mk.lat, c);
      const lo = mk.minZoom == null ? 0.8 : mk.minZoom, hi = mk.maxZoom == null ? 8 : mk.maxZoom;
      const show = p && p.facing >= 0.35 && cam.zoom >= lo - 1e-9 && cam.zoom <= hi + 1e-9;
      el.hidden = !show;
      if (show) el.style.transform = 'translate(' + (p.x - mk.width / 2) + 'px, ' + (p.y - mk.height / 2) + 'px)';
    }
  }

  // ---- the pack's frames
  function packUrl(file, mode, extra) {
    const q = new URLSearchParams(Object.assign({ mode, textScale: String(settings.textScale), reduceMotion: settings.reduceMotion ? '1' : '0' }, extra));
    return '/pack/' + ID + '/' + file.split('/').map(encodeURIComponent).join('/') + '?' + q;
  }
  function post(frame, message) { if (frame && frame.contentWindow) frame.contentWindow.postMessage({ terraHost: message }, '*'); }
  function makeFrame(kind, url) {
    const f = document.createElement('iframe');
    f.className = 'pack ' + kind; f.src = url;
    f.title = kind === 'overlay' ? 'Overlay (transparent, over the globe)' : 'Prototype screen';
    if (kind === 'overlay') f.setAttribute('aria-hidden', 'true');
    return f;
  }
  function expectReady(kind, frame) {
    setTimeout(() => {
      if (!api.ready[kind] && (kind === 'overlay' ? overlay : screen) === frame) {
        log(kind, 'error', 'no "ready" after 5 s: the page did not load, or does not finish loading');
      }
    }, 5000);
  }

  function makeOverlay() {
    overlay = makeFrame('overlay', packUrl(M.overlay, 'overlay'));
    api.ready.overlay = false;
    stage.insertBefore(overlay, markersEl);
    expectReady('overlay', overlay);
  }
  function openScreen(params) {
    if (ended) return;
    if (screen) return;
    const entry = M.entry || 'index.html';
    screen = makeFrame('screen', packUrl(entry, 'screen', params));
    screen.style.background = M.background || '#fff';
    api.ready.screen = false;
    device.insertBefore(screen, $('back'));
    device.classList.add('has-screen');
    $('back').focus();
    expectReady('screen', screen);
    log('harness', 'info', 'screen opened' + (params.marker ? ' from marker "' + params.marker + '"' : ''));
  }
  function closeScreen() {
    if (!screen) return;
    screen.remove(); screen = null; api.ready.screen = false;
    device.classList.remove('has-screen');
    log('harness', 'info', 'screen closed');
    if (M.presentation === 'screen') endPrototype(); else stage.focus();
  }
  function endPrototype() {
    if (overlay) { overlay.remove(); overlay = null; api.ready.overlay = false; }
    if (screen) { screen.remove(); screen = null; api.ready.screen = false; device.classList.remove('has-screen'); }
    markers.forEach((m) => m.el.remove()); markers = [];
    resetHost();
    ended = true;
    $('ended').hidden = false;
    $('again').focus();
    log('harness', 'info', 'prototype ended; the plain globe is as it was');
  }
  function start() {
    resetHost();
    warned.clear();
    ended = false;
    $('ended').hidden = true;
    if (M.presentation === 'globe') { makeMarkers(); if (M.overlay) makeOverlay(); }
    else openScreen({});
    cameraChanged();
  }

  function applySettings(patch) {
    Object.assign(settings, patch);
    settings.textScale = clamp(+settings.textScale || 1, 1, 3.2);
    $('scale').value = settings.textScale;
    $('scale-out').textContent = settings.textScale.toFixed(1);
    $('motion').checked = !!settings.reduceMotion;
    foot.style.setProperty('--ts', String(settings.textScale));
    if (settings.reduceMotion && host.time && host.time.playing) { stopPlayback(); sendTime('host'); }    // never played under Reduce Motion
    updateFoot();
    for (const f of [overlay, screen]) post(f, { type: 'settings', settings: Object.assign({}, settings) });
  }

  // messages from the pack's pages: terra.js posts { terra: message } to its parent
  const warned = new Set();
  window.addEventListener('message', (e) => {
    const msg = e.data && e.data.terra;
    if (!msg || typeof msg !== 'object') return;
    const from = overlay && e.source === overlay.contentWindow ? 'overlay' : screen && e.source === screen.contentWindow ? 'screen' : null;
    if (!from) return;
    switch (msg.type) {
      case 'ready':
        api.ready[from] = true;
        post(from === 'overlay' ? overlay : screen, { type: 'settings', settings: Object.assign({}, settings) });
        if (from === 'overlay') { post(overlay, { type: 'camera', camera: camera() }); sendTime('host'); }
        break;
      case 'close':
        if (from === 'screen') closeScreen(); else endPrototype();
        break;
      case 'open':
        if (from === 'overlay') openScreen(msg.params || {}); else log(from, 'warn', 'terra.open() is for overlays; ignored');
        break;
      case 'flyTo':
        if (from === 'overlay') flyTo(msg.lon, msg.lat, msg.zoom, msg.duration); else log(from, 'warn', 'terra.flyTo() is for overlays; ignored');
        break;
      case 'geo':
      case 'time':
      case 'caption':
      case 'globe':
        if (from !== 'overlay') { log(from, 'warn', 'terra.' + (msg.type === 'geo' ? 'geo' : msg.type) + ' is for overlays; ignored'); break; }
        if (!R2 && !warned.has(msg.type)) { warned.add(msg.type); log(from, 'warn', 'terra.' + msg.type + ' is runtime 2: prototype.json should say "runtime": 2 (it says ' + M.runtime + ')'); }
        if (msg.type === 'geo') onGeo(msg, from);
        else if (msg.type === 'time') onTime(msg, from);
        else if (msg.type === 'caption') { host.caption = { title: String(msg.title || ''), text: String(msg.text || '') }; updateFoot(); }
        else { host.globe = { historical: !!msg.historical, still: !!msg.still }; schedule(); }
        break;
      case 'log':
        log(from, msg.level === 'error' ? 'error' : 'info', msg.message);
        break;
    }
  });

  // ---- touching the stand-in globe: drag turns, wheel and pinch zoom, a tap goes to the overlay
  const pointers = new Map();
  let gesture = null;
  const local = (e) => { const r = stage.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
  stage.addEventListener('pointerdown', (e) => {
    if (e.target.closest('.marker')) return;
    stage.setPointerCapture(e.pointerId);
    pointers.set(e.pointerId, local(e));
    gesture = { moved: pointers.size > 1, start: performance.now(), x: pointers.get(e.pointerId)[0], y: pointers.get(e.pointerId)[1] };
  });
  stage.addEventListener('pointermove', (e) => {
    if (!pointers.has(e.pointerId)) return;
    const prev = pointers.get(e.pointerId), now = local(e);
    if (pointers.size === 2) {
      const [a, b] = [...pointers.values()], before = Math.hypot(a[0] - b[0], a[1] - b[1]);
      pointers.set(e.pointerId, now);
      const [c, d] = [...pointers.values()], after = Math.hypot(c[0] - d[0], c[1] - d[1]);
      if (before > 0) cam.zoom = clamp(cam.zoom * after / before, 0.8, 8);
      gesture.moved = true;
    } else {
      pointers.set(e.pointerId, now);
      const dx = now[0] - prev[0], dy = now[1] - prev[1], R = camera().radius;
      if (Math.hypot(now[0] - gesture.x, now[1] - gesture.y) > 6) gesture.moved = true;
      if (gesture.moved) {
        cam.lon = wrapLon(cam.lon - dx / R / D / Math.max(Math.cos(cam.lat * D), 0.2));
        cam.lat = clamp(cam.lat + dy / R / D, -85, 85);
      }
    }
    cameraChanged();
  });
  const release = (e) => {
    if (!pointers.has(e.pointerId)) return;
    const wasTap = gesture && !gesture.moved && pointers.size === 1 && e.type === 'pointerup' && performance.now() - gesture.start < 600;
    const at = pointers.get(e.pointerId);
    pointers.delete(e.pointerId);
    if (wasTap) tap(at[0], at[1]);
  };
  stage.addEventListener('pointerup', release);
  stage.addEventListener('pointercancel', release);
  stage.addEventListener('wheel', (e) => {
    e.preventDefault();
    cam.zoom = clamp(cam.zoom * Math.exp(-e.deltaY * 0.002), 0.8, 8);
    cameraChanged();
  }, { passive: false });
  stage.addEventListener('keydown', (e) => {
    const step = 10 / cam.zoom, keys = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] };
    if (keys[e.key]) { cam.lon = wrapLon(cam.lon + keys[e.key][0]); cam.lat = clamp(cam.lat + keys[e.key][1], -85, 85); }
    else if (e.key === '+' || e.key === '=') cam.zoom = clamp(cam.zoom * 1.25, 0.8, 8);
    else if (e.key === '-') cam.zoom = clamp(cam.zoom / 1.25, 0.8, 8);
    else return;
    e.preventDefault();
    cameraChanged();
  });
  function tap(x, y) {
    if (!(M.taps && overlay && api.ready.overlay)) return;      // otherwise the app would choose a country: not drawn here
    const at = terra.unproject(x, y, camera());
    post(overlay, { type: 'tap', x, y, lon: at ? at.lon : null, lat: at ? at.lat : null });
  }

  // ---- controls
  $('scale').addEventListener('input', (e) => applySettings({ textScale: +e.target.value }));
  $('motion').addEventListener('change', (e) => applySettings({ reduceMotion: e.target.checked }));
  $('back').addEventListener('click', closeScreen);
  $('end').addEventListener('click', endPrototype);
  $('restart').addEventListener('click', () => { endPrototype(); start(); });
  $('again').addEventListener('click', start);

  applySettings({});
  start();
})();

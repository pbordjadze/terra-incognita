// The desktop preview's host: a stand-in for the app around one runtime-3 pack (docs/runtime3.md), so a pack
// can be seen and touched without a Mac. It plays the app's part: a globe (Natural Earth land and borders,
// orthographic, in the app's Wikipedia colours: tools/globe/preview.py's palette and layout), the camera with the
// numbers terra.js uses, the geography, pictures and sprites a pack asks the globe to draw (as
// App/TerraIncognita/Globe/GeoLayers.swift draws them: its inks, dashes, halos and reading scale), the foot (the
// caption and the date's strip, as GlobeFoot), the panel, the chip (Open, ✕), the doorway, and touch.
// The pack's scripts run in a worker (worker.js) with only what JavaScriptCore gives them; this page is their host:
// the worker posts what `__terraSend` is given, and this page answers with `terra._receive` messages.
// Close to the app, not pixel-identical: the globe is drawn from vectors on a canvas, not by Globe.metal.
// Query: device=iphone|ipad, start=1 (past the doorway), bare=1 (the screen alone), textScale, reduceMotion=1,
// idleAfter=<seconds> (the idle turn's wait, 8 by default), timers=host (terra.js's own timers, as in JavaScriptCore).
(function () {
  'use strict';
  const P = window.__proto, M = P.manifest || {}, ID = P.id, Q = P.query || {};
  const DEVICES = {
    iphone: { size: [393, 852], safe: [59, 34], label: 'iPhone' },
    ipad: { size: [1032, 1376], safe: [24, 20], label: 'iPad' },
  };
  const DEV = DEVICES[P.device] || DEVICES.iphone;
  const [W, H] = DEV.size, [SAFE_TOP, SAFE_BOTTOM] = DEV.safe;
  const BARE = Q.bare === '1';
  const FILES = new Set(P.files || []);
  const D = Math.PI / 180, TAU = 2 * Math.PI;
  const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
  const num = (v, fallback) => (typeof v === 'number' && Number.isFinite(v) ? v : fallback);
  const $ = (id) => document.getElementById(id);
  const now = () => performance.now();
  const later = window.setTimeout.bind(window), cancel = window.clearTimeout.bind(window);
  const t0 = now();

  // The palette: the app's Wikipedia style (tools/globe/preview.py, Globe.metal), and GeoLayers.swift's inks
  const PAPER = '#F6F2E7', SEA = '#DDF3FD', LAND = '#F5EED8', COAST = '#1C7AB0', BORDER = '#8C8478', GOLD = '#B08A3E';
  const INK = [59, 74, 84], LABEL_TONE = 'rgba(246, 242, 231, 0.92)', DOT_PAPER = '#F6F2E7';
  const AREA = [193, 120, 70], SOUNDING_LINE = 'rgba(28, 122, 176, 0.85)', SOUNDING_FOOT = '#1C7AB0', SOUNDING_INK = '#1C5C8C';
  const FACE = '"Libertinus Serif", Georgia, serif';

  // ---- the page
  const device = $('device'), stage = $('stage'), canvas = $('globe'), screenCtx = canvas.getContext('2d');
  let g = screenCtx;      // where the drawing functions draw: the screen, or the cached layer under the sprites
  device.style.width = W + 'px';
  device.style.height = H + 'px';
  device.style.setProperty('--safe-bottom', SAFE_BOTTOM + 'px');
  if (BARE) document.body.classList.add('bare');
  $('frame').classList.add(P.device === 'ipad' ? 'ipad' : 'iphone');
  $('title').textContent = M.title || ID;
  $('meta').textContent = ID + ', runtime ' + M.runtime + ', ' + DEV.label + ' (' + W + ' × ' + H + ' pt)' + (P.where && !/^Prototypes\//.test(P.where) ? ', from ' + P.where : '');
  for (const d of ['iphone', 'ipad']) {
    const a = $('to-' + d), q = new URLSearchParams(Q);
    q.set('device', d);
    a.href = '?' + q;
    if (d === P.device) a.setAttribute('aria-current', 'page');
  }
  $('chip').style.top = $('start').style.top = (SAFE_TOP + 4) + 'px';
  $('chip-name').textContent = M.title || ID;
  $('foot').style.paddingBottom = (14 + SAFE_BOTTOM) + 'px';

  const settings = {
    reduceMotion: Q.reduceMotion === '1' || Q.reduceMotion === 'true',
    textScale: clamp(parseFloat(Q.textScale) || 1, 1, 3.2),
    active: true,
  };
  const idleAfter = clamp(Q.idleAfter === undefined ? 8 : parseFloat(Q.idleAfter) || 0, 0, 1e6);

  // ---- for checks (tools/playground/check.cjs, harness_check.cjs) and the curious
  const api = window.__playground = {
    ready: false, running: false, started: false, ended: false, frame: 0,
    device: { name: P.device, width: W, height: H },
    log: [], touches: [], delivered: {},
  };

  // ---- the log
  const shorten = (s) => String(s).split(location.origin + '/pack/' + ID + '/').join('').split(location.origin).join('');
  function log(source, level, message) {
    const entry = { t: (now() - t0) / 1000, source, level, message: shorten(message) };
    api.log.push(entry);
    if (api.log.length > 2000) api.log.shift();
    const el = $('log'), li = document.createElement('li');
    li.className = level;
    li.textContent = '+' + entry.t.toFixed(1) + 's ' + source + (level === 'info' ? '' : ' ' + level) + ': ' + entry.message;
    el.appendChild(li);
    while (el.children.length > 300) el.removeChild(el.firstChild);
    el.scrollTop = el.scrollHeight;
    console.log('[' + source + ' ' + level + '] ' + entry.message);   // (never console.error: that is the harness's own failing)
  }
  const hostError = (m) => log('host', 'error', m);
  const hostWarn = (m) => log('host', 'warn', m);
  const warnedOnce = new Set();
  const warnOnce = (key, m) => { if (!warnedOnce.has(key)) { warnedOnce.add(key); hostWarn(m); } };
  window.addEventListener('error', (e) => log('harness', 'error', e.message + (e.filename ? ' (' + shorten(e.filename) + ':' + e.lineno + ')' : '')));
  window.addEventListener('unhandledrejection', (e) => log('harness', 'error', 'unhandled rejection: ' + (e.reason && e.reason.message || e.reason)));

  // ---- the reading scale (Globe/type.json `reading`: 1.0 on a phone, 1.2 on the 13-inch iPad) and the globe's room
  let TYPE = { reading: { shortSide: [430, 1032], factor: [1.0, 1.2] } };
  let rs = 1, layout = null;
  function measure() {
    const [s0, s1] = TYPE.reading.shortSide, [f0, f1] = TYPE.reading.factor;
    rs = f0 + (f1 - f0) * clamp((Math.min(W, H) - s0) / (s1 - s0), 0, 1);
    // GlobeView.place, as tools/globe/preview.py's layout: between the buttons and the caption, its gold ring round it
    const top = SAFE_TOP + 30 + 48, bottom = H - SAFE_BOTTOM - 150;
    const half = Math.min(W, bottom - top) / 2 - 14;
    layout = { cx: W / 2, cy: (top + bottom) / 2, r0: Math.max(half - 11 * rs, 40) };
  }
  measure();

  // ---- the camera (terra.camera: lon, lat, zoom 0.8-8; radius = the resting radius x zoom)
  let zoomLimit = 8;   // the deepest zoom: 8, or what the pack asked for (terra.globe.set({ maxZoom }))
  const cam = { lon: -40, lat: 22, zoom: 1 };
  let flight = null, lastTouch = -1e9, camVersion = 0, lastCamMove = -1e9, needsRender = true;
  // something under the sprites changed: the cached layer is drawn again (camera moves are in its key)
  let layerVersion = 0;
  const dirty = () => { layerVersion++; needsRender = true; };
  const wrapLon = (l) => ((((l + 180) % 360) + 360) % 360) - 180;
  function camera() {
    return { lon: cam.lon, lat: cam.lat, zoom: cam.zoom, cx: layout.cx, cy: layout.cy, radius: layout.r0 * cam.zoom, width: W, height: H };
  }
  function cameraMoved() {
    camVersion++;
    lastCamMove = now();
    dirty();
    cameraEvent();
  }
  function flyTo(lon, lat, zoom, seconds) {
    if (!Number.isFinite(+lon) || !Number.isFinite(+lat)) return hostError('terra.flyTo(' + lon + ', ' + lat + '): longitude and latitude must be numbers');
    const to = { lon: wrapLon(+lon), lat: clamp(+lat, -80, 80), zoom: zoom == null || !Number.isFinite(+zoom) ? cam.zoom : clamp(+zoom, 0.8, zoomLimit) };
    const ms = (seconds == null || !Number.isFinite(+seconds) ? 0.8 : Math.max(0, +seconds)) * 1000;
    if (settings.reduceMotion || ms <= 0) {           // Reduce Motion: arrive, don't travel
      flight = null;
      Object.assign(cam, to);
      return cameraMoved();
    }
    flight = { from: { lon: cam.lon, lat: cam.lat, zoom: cam.zoom }, to, start: now(), ms };
  }
  function stepFlight(t) {
    const f = flight, u = clamp((t - f.start) / f.ms, 0, 1), e = u * u * (3 - 2 * u);
    let dl = f.to.lon - f.from.lon;
    if (dl > 180) dl -= 360; else if (dl < -180) dl += 360;
    cam.lon = wrapLon(f.from.lon + dl * e);
    cam.lat = f.from.lat + (f.to.lat - f.from.lat) * e;
    cam.zoom = Math.exp(Math.log(f.from.zoom) + (Math.log(f.to.zoom) - Math.log(f.from.zoom)) * e);
    if (u >= 1) flight = null;
    cameraMoved();
  }
  // a drag turns the globe under the hand (GlobeView.turn)
  function turn(dx, dy) {
    const R = layout.r0 * cam.zoom;
    cam.lon = wrapLon(cam.lon - dx / R / D);
    cam.lat = clamp(cam.lat + dy / R / D, -80, 80);
    cameraMoved();
  }
  function zoomBy(k) {
    cam.zoom = clamp(cam.zoom * k, 0.8, zoomLimit);
    cameraMoved();
  }

  // the camera told to the script: at most 20 times a second while it moves, and once when it stops
  let camSentAt = -1e9, camTimer = 0, camStopTimer = 0;
  function cameraEvent() {
    if (!api.started) return;
    const t = now();
    if (t - camSentAt >= 50) sendCameraEvent();
    else if (!camTimer) camTimer = later(() => { camTimer = 0; sendCameraEvent(); }, 50 - (t - camSentAt));
    cancel(camStopTimer);
    camStopTimer = later(stopped, 160);
  }
  function stopped() {
    // (timers that came due together, behind a busy frame, still keep 50 ms apart)
    const wait = 50 - (now() - camSentAt);
    if (wait > 0) { camStopTimer = later(stopped, wait); return; }
    camStopTimer = 0;
    sendCameraEvent();
  }
  api.cameraSends = [];          // when each was told (the log's clock), for checks
  function sendCameraEvent() {
    cancel(camTimer);
    camTimer = 0;
    camSentAt = now();
    api.cameraSends.push(+((camSentAt - t0) / 1000).toFixed(4));
    if (api.cameraSends.length > 400) api.cameraSends.shift();
    deliver({ type: 'camera' });
  }

  // ---- the stand-in globe: land, borders and names from Natural Earth (tools/plates/data), a graticule
  let land = [], politics = null;
  const graticule = [];
  for (let lat = -75; lat <= 75; lat += 15) {
    const pts = [];
    for (let lon = -180; lon <= 180; lon += 3) pts.push([lon, lat]);
    graticule.push(ring(pts, false));
  }
  for (let lon = -180; lon < 180; lon += 15) {
    const pts = [];
    for (let lat = -75; lat <= 75; lat += 3) pts.push([lon, lat]);
    graticule.push(ring(pts, false));
  }

  // A ring or a line as unit vectors on the sphere, with room for the same points in view space.
  function ring(pts, closed) {
    if (closed && pts.length > 1) {
      const last = pts[pts.length - 1], first = pts[0];
      if (last[0] === first[0] && last[1] === first[1]) pts = pts.slice(0, -1);
    }
    const n = pts.length, w = new Float64Array(3 * n);
    let sx = 0, sy = 0, sz = 0;
    pts.forEach((p, i) => {
      const lo = p[0] * D, la = p[1] * D;
      w[3 * i] = Math.cos(la) * Math.sin(lo); w[3 * i + 1] = Math.sin(la); w[3 * i + 2] = Math.cos(la) * Math.cos(lo);
      sx += w[3 * i]; sy += w[3 * i + 1]; sz += w[3 * i + 2];
    });
    // the cap round it (its middle and the cosine of its angular radius), so a ring off screen or behind is skipped whole
    const len = Math.hypot(sx, sy, sz) || 1, ux = sx / len, uy = sy / len, uz = sz / len;
    let cosR = 1;
    for (let i = 0; i < n; i++) cosR = Math.min(cosR, ux * w[3 * i] + uy * w[3 * i + 1] + uz * w[3 * i + 2]);
    return { n, w, vx: new Float64Array(n), vy: new Float64Array(n), vz: new Float64Array(n), ux, uy, uz, cosR,
             chord: Math.sqrt(Math.max(0, 2 - 2 * cosR)), sinR: Math.sqrt(Math.max(0, 1 - cosR * cosR)) };
  }
  // Could any of the ring be on screen? (Orthographic projection never lengthens a chord, so its picture lies within
  // the chord round its middle's.)
  // (points: finer at rest, coarser while the camera moves, when a frame must be quick)
  const MARGIN = 40, THIN_AT_REST = 0.4, THIN_MOVING = 1;
  let thin = THIN_AT_REST;
  function onScreen(r, m, cx, cy, R) {
    if (!(r.cosR > 0)) return true;
    const x = r.ux, y = r.uy, z = r.uz;
    const vz = m.sa * m.cb * x - m.sb * y + m.ca * m.cb * z;
    if (vz < -r.sinR) return false;
    const px = cx + (m.ca * x - m.sa * z) * R, py = cy - (m.sa * m.sb * x + m.cb * y + m.ca * m.sb * z) * R, rad = r.chord * R;
    return !(px + rad < -MARGIN || px - rad > W + MARGIN || py + rad < -MARGIN || py - rad > H + MARGIN);
  }
  // beyond which edges of the screen (and its margin) a point lies
  const outcode = (x, y) => (x < -MARGIN ? 1 : x > W + MARGIN ? 2 : 0) | (y < -MARGIN ? 4 : y > H + MARGIN ? 8 : 0);
  const unit = (lon, lat) => { const lo = lon * D, la = lat * D; return [Math.cos(la) * Math.sin(lo), Math.sin(la), Math.cos(la) * Math.cos(lo)]; };

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
  // Borders: the arcs two countries share. Names: the middle of each country's largest piece, and about how big it is.
  function decodePolitics(topo) {
    const arcs = decodeArcs(topo), used = new Map(), names = [];
    for (const geom of topo.objects.countries.geometries) {
      if (!geom.arcs) continue;
      let best = null;
      for (const poly of geom.type === 'Polygon' ? [geom.arcs] : geom.arcs) {
        for (const r of poly) for (const i of r) used.set(i < 0 ? ~i : i, (used.get(i < 0 ? ~i : i) || 0) + 1);
        const pts = arcsPoints(arcs, poly[0]);
        let area = 0, cx = 0, cy = 0;
        for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) {
          const f = pts[j][0] * pts[i][1] - pts[i][0] * pts[j][1];
          area += f; cx += (pts[j][0] + pts[i][0]) * f; cy += (pts[j][1] + pts[i][1]) * f;
        }
        if (area && (!best || Math.abs(area) > best.area)) best = { area: Math.abs(area), lon: cx / (3 * area), lat: cy / (3 * area) };
      }
      const name = geom.properties && geom.properties.name;
      if (best && name) {
        const v = unit(best.lon, best.lat);
        names.push({ name: name === 'United States of America' ? 'United States' : name,
                     size: Math.sqrt(best.area * D * D * Math.max(Math.cos(best.lat * D), 0.2)), wx: v[0], wy: v[1], wz: v[2] });
      }
    }
    names.sort((a, b) => b.size - a.size);
    const borders = [];
    for (const [i, count] of used) if (count > 1) borders.push(ring(arcs[i], false));
    return { borders, names };
  }

  // The camera as a rotation (terra.project's, term for term) and a ring carried through it
  function rotation(c) {
    const a = c.lon * D, b = -c.lat * D;
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
  // a place (its unit vector) through the rotation: screen x, y and facing z
  function projectUnit(x, y, z, m, c, out) {
    out.x = c.cx + (m.ca * x - m.sa * z) * c.radius;
    out.y = c.cy - (m.sa * m.sb * x + m.cb * y + m.ca * m.sb * z) * c.radius;
    out.z = m.sa * m.cb * x - m.sb * y + m.ca * m.cb * z;
    return out;
  }
  function unproject(x, y, c) {
    const vx = (x - c.cx) / c.radius, vy = (c.cy - y) / c.radius, r2 = vx * vx + vy * vy;
    if (r2 >= 1) return null;
    const vz = Math.sqrt(1 - r2), a = c.lon * D, b = -c.lat * D;
    const wx = Math.cos(a) * vx + Math.sin(a) * Math.sin(b) * vy + Math.sin(a) * Math.cos(b) * vz;
    const wy = Math.cos(b) * vy - Math.sin(b) * vz;
    const wz = -Math.sin(a) * vx + Math.cos(a) * Math.sin(b) * vy + Math.cos(a) * Math.cos(b) * vz;
    return { lon: Math.atan2(wx, wz) / D, lat: Math.asin(clamp(wy, -1, 1)) / D };
  }
  function crossing(r, i, j) {
    const { vx, vy, vz } = r;
    const t = vz[i] / (vz[i] - vz[j]);
    const px = vx[i] + t * (vx[j] - vx[i]), py = vy[i] + t * (vy[j] - vy[i]), len = Math.hypot(px, py) || 1;
    return [px / len, py / len];
  }
  // A path of points in view space, thinned as it goes: a point closer than `thin` to the last one drawn is left out,
  // and so is one that lies, with the last one drawn and the next, beyond the same edge of the screen (the shortcut
  // stays beyond that edge, so nothing on screen changes). Rasterizing every vertex is what makes a canvas slow.
  function thinner(r, cx, cy, R) {
    const { vx, vy, vz, n } = r;
    let lx = 0, ly = 0, lc = 0;
    return {
      at(x, y) { lx = x; ly = y; lc = outcode(x, y); },
      // whether to draw vertex j, when a visible vertex `next` (or -1) follows it
      keep(j, next) {
        const x = cx + vx[j] * R, y = cy - vy[j] * R;
        if (next >= 0 && next < n && vz[next] > 0) {
          if (Math.abs(x - lx) + Math.abs(y - ly) < thin) return false;
          const c = outcode(x, y);
          if (lc & c && lc & c & outcode(cx + vx[next] * R, cy - vy[next] * R)) return false;
        }
        lx = x; ly = y; lc = outcode(x, y);
        return true;
      },
    };
  }
  // One ring in view space: points behind the globe are carried along the limb so a fill follows it; the
  // outline (`stroke`) is drawn only where it is really seen.
  function trace(r, m, cx, cy, R, fill, stroke) {
    if (!onScreen(r, m, cx, cy, R)) return false;
    const { n, vx, vy, vz } = r;
    rotate(r, m);
    let first = -1, seen = 0;
    for (let i = 0; i < n; i++) if (vz[i] > 0) { seen++; if (first < 0) first = i; }
    if (!seen) return false;
    const X = (i) => cx + vx[i] * R, Y = (i) => cy - vy[i] * R, th = thinner(r, cx, cy, R);
    fill.moveTo(X(first), Y(first));
    if (stroke) stroke.moveTo(X(first), Y(first));
    th.at(X(first), Y(first));
    if (seen === n) {
      for (let k = 1; k < n; k++) {
        const i = (first + k) % n;
        if (th.keep(i, k < n - 1 ? (i + 1) % n : -1)) { fill.lineTo(X(i), Y(i)); if (stroke) stroke.lineTo(X(i), Y(i)); }
      }
      fill.closePath(); if (stroke) stroke.closePath();
      return true;
    }
    // behind the globe, the limb's angle is followed the short way from point to point and drawn as one arc on the way back
    let from = 0, at = 0, sweep = 0;
    for (let k = 0; k < n; k++) {
      const i = (first + k) % n, j = (first + k + 1) % n, a = vz[i] > 0, b = vz[j] > 0;
      if (a && b) {
        if (th.keep(j, k < n - 1 ? (j + 1) % n : -1)) { fill.lineTo(X(j), Y(j)); if (stroke) stroke.lineTo(X(j), Y(j)); }
      } else if (a) {
        // (the last point before the limb was drawn: a point is left out only when a visible one follows it)
        const [qx, qy] = crossing(r, i, j);
        fill.lineTo(cx + qx * R, cy - qy * R); if (stroke) stroke.lineTo(cx + qx * R, cy - qy * R);
        from = at = Math.atan2(-qy, qx);
        sweep = 0;
      } else if (!b) {
        const to = Math.atan2(-vy[j], vx[j]);
        sweep += shortTurn(at, to);
        at = to;
      } else {
        const [qx, qy] = crossing(r, i, j), to = Math.atan2(-qy, qx);
        sweep += shortTurn(at, to);
        const d = clamp(sweep, -TAU, TAU);
        fill.arc(cx, cy, R, from, from + d, d < 0);
        fill.lineTo(cx + qx * R, cy - qy * R); fill.lineTo(X(j), Y(j));
        if (stroke) { stroke.moveTo(cx + qx * R, cy - qy * R); stroke.lineTo(X(j), Y(j)); }
        th.at(X(j), Y(j));
      }
    }
    fill.closePath();
    return true;
  }
  function shortTurn(a, b) {
    let d = b - a;
    while (d > Math.PI) d -= TAU;
    while (d < -Math.PI) d += TAU;
    return d;
  }
  // A line (or a closed ring) in view space, broken where it goes behind the globe.
  function traceLine(r, m, cx, cy, R, path, closed) {
    if (!onScreen(r, m, cx, cy, R)) return;
    const { n, vx, vy, vz } = r;
    rotate(r, m);
    const X = (i) => cx + vx[i] * R, Y = (i) => cy - vy[i] * R, th = thinner(r, cx, cy, R);
    let drawing = false;
    for (let i = 0, edges = closed ? n : n - 1; i < edges; i++) {
      const j = (i + 1) % n, a = vz[i] > 0, b = vz[j] > 0;
      if (a && b) {
        if (!drawing) { path.moveTo(X(i), Y(i)); th.at(X(i), Y(i)); drawing = true; }
        if (th.keep(j, i + 1 < edges ? (j + 1) % n : -1)) path.lineTo(X(j), Y(j));
      } else if (a) {
        if (!drawing) path.moveTo(X(i), Y(i));
        const [qx, qy] = crossing(r, i, j);
        path.lineTo(cx + qx * R, cy - qy * R);
        drawing = false;
      } else if (b) {
        const [qx, qy] = crossing(r, i, j);
        path.moveTo(cx + qx * R, cy - qy * R); path.lineTo(X(j), Y(j));
        th.at(X(j), Y(j));
        drawing = true;
      } else drawing = false;
    }
  }

  const smooth = (a, b, x) => { const t = clamp((x - a) / (b - a), 0, 1); return t * t * (3 - 2 * t); };
  // the gold ring round the whole globe (it leaves as the globe outgrows the screen)
  function drawRing(c) {
    const on = 1 - smooth(1.25, 1.7, c.zoom);
    if (on <= 0.01) return;
    g.save();
    g.globalAlpha = on;
    g.strokeStyle = GOLD;
    g.lineWidth = 1.1 * rs; g.beginPath(); g.arc(c.cx, c.cy, c.radius + 6.5 * rs, 0, TAU); g.stroke();
    g.lineWidth = 0.7 * rs; g.beginPath(); g.arc(c.cx, c.cy, c.radius + 10 * rs, 0, TAU); g.stroke();
    g.globalAlpha = on * 0.8; g.lineWidth = 0.9 * rs; g.beginPath();
    for (let a = 5; a < 360; a += 10) {
      const s = Math.sin(a * D), k = -Math.cos(a * D);
      g.moveTo(c.cx + s * (c.radius + 7 * rs), c.cy + k * (c.radius + 7 * rs));
      g.lineTo(c.cx + s * (c.radius + 9.6 * rs), c.cy + k * (c.radius + 9.6 * rs));
    }
    g.stroke();
    g.globalAlpha = on; g.fillStyle = GOLD;
    for (let a = 0; a < 360; a += 90) for (const da of [-1.5, 0, 1.5]) {
      const t = (a + da) * D;
      g.beginPath(); g.arc(c.cx + Math.sin(t) * (c.radius + 8.25 * rs), c.cy - Math.cos(t) * (c.radius + 8.25 * rs), 1.4 * rs, 0, TAU); g.fill();
    }
    g.restore();
  }
  function drawGlobe(c, m) {
    g.save();
    g.beginPath(); g.arc(c.cx, c.cy, c.radius, 0, TAU); g.fillStyle = SEA; g.fill(); g.clip();
    const fill = new Path2D(), coast = new Path2D();
    for (const r of land) trace(r, m, c.cx, c.cy, c.radius, fill, coast);
    g.fillStyle = LAND; g.fill(fill, 'evenodd');
    const grat = new Path2D();
    for (const r of graticule) traceLine(r, m, c.cx, c.cy, c.radius, grat, false);
    g.lineJoin = 'round'; g.lineCap = 'round';
    g.globalAlpha = 0.13; g.strokeStyle = COAST; g.lineWidth = 0.6 * rs; g.stroke(grat);
    if (politics && !host.globe.historical) {
      const borders = new Path2D();
      for (const r of politics.borders) traceLine(r, m, c.cx, c.cy, c.radius, borders, false);
      g.globalAlpha = 0.8; g.strokeStyle = BORDER; g.lineWidth = 0.7 * rs; g.stroke(borders);
    }
    g.globalAlpha = 1; g.strokeStyle = COAST; g.lineWidth = 0.9 * rs * Math.min(Math.pow(clamp(c.zoom, 1, 8), 0.25), 1.4); g.stroke(coast);
    // roundness: the limb a little shaded (Globe.metal: 7% at the edge)
    const shade = g.createRadialGradient(c.cx, c.cy, c.radius * 0.6, c.cx, c.cy, c.radius);
    shade.addColorStop(0, 'rgba(0,0,0,0)'); shade.addColorStop(0.75, 'rgba(0,0,0,0.012)'); shade.addColorStop(1, 'rgba(0,0,0,0.07)');
    g.fillStyle = shade; g.fillRect(c.cx - c.radius, c.cy - c.radius, 2 * c.radius, 2 * c.radius);
    g.restore();
    g.beginPath(); g.arc(c.cx, c.cy, c.radius, 0, TAU); g.lineWidth = 1.2 * rs; g.strokeStyle = COAST; g.stroke();
  }
  // today's country names, where they fit (Globe/type.json's country1), hidden by globe.historical
  function drawCountryNames(c, m, placed) {
    const face = (TYPE.styles && TYPE.styles.wikipedia && TYPE.styles.wikipedia.names && TYPE.styles.wikipedia.names.country1) || {};
    const size = num(face.size, 12) * rs, o = { x: 0, y: 0, z: 0 };
    g.save();
    g.font = '400 ' + size + 'px ' + FACE; g.textAlign = 'center'; g.textBaseline = 'middle';
    if ('letterSpacing' in g) g.letterSpacing = (num(face.tracking, 0.09) * size) + 'px';
    g.lineJoin = 'round'; g.lineWidth = 3 * rs;
    for (const n of politics.names) {
      projectUnit(n.wx, n.wy, n.wz, m, c, o);
      if (o.z < 0.4 || n.size * c.radius < 12 * rs) continue;
      const text = n.name.toUpperCase(), w = g.measureText(text).width;
      if (n.size * c.radius < w * 0.7) continue;
      const box = [o.x - w / 2 - 2, o.y - size * 0.6 - 2, w + 4, size * 1.2 + 4];
      if (!inside(box, [0, 0, W, H]) || placed.some((q) => overlaps(box, q))) continue;
      placed.push(grow(box, 0.4 * size));
      g.strokeStyle = face.halo || '#F5EED8'; g.strokeText(text, o.x, o.y);
      g.fillStyle = face.ink || '#5F5A51'; g.fillText(text, o.x, o.y);
    }
    g.restore();
  }
  const overlaps = (a, b) => a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3];
  const inside = (a, b) => a[0] >= b[0] && a[1] >= b[1] && a[0] + a[2] <= b[0] + b[2] && a[1] + a[3] <= b[1] + b[3];
  const grow = (a, d) => [a[0] - d, a[1] - d, a[2] + 2 * d, a[3] + 2 * d];
  const within = (x, y, r) => x >= r[0] && y >= r[1] && x <= r[0] + r[2] && y <= r[1] + r[3];

  // ---- what the running pack has asked for (all of it ends with the pack)
  const host = {
    geo: new Map(), images: new Map(), sprites: new Map(),
    time: null, caption: { title: '', text: '' }, globe: { historical: false, still: false },
    panel: null, scheme: 'touchables', listensOpen: false,
  };
  let imagesVersion = 0;

  // ---- the pack's files and pictures
  function cleanPath(path) {
    if (typeof path !== 'string') return null;
    const p = path.replace(/^\.\//, '');
    if (!p || p.startsWith('/') || p.includes('\\') || /^[a-z][a-z0-9+.-]*:/i.test(p) || p.split('/').some((x) => x === '' || x === '.' || x === '..')) return null;
    return p;
  }
  const packUrl = (p) => '/pack/' + ID + '/' + p.split('/').map(encodeURIComponent).join('/');
  const pictures = new Map();
  let pendingPictures = 0;
  function picture(path, who) {
    const p = cleanPath(path);
    if (!p || !FILES.has(p)) { hostError(who + ': "' + path + '" is not a file of the pack'); return null; }
    let e = pictures.get(p);
    if (e) return e;
    e = { path: p, img: new Image(), ok: false, failed: false, data: null, w: 0, h: 0, tooBig: false };
    pictures.set(p, e);
    pendingPictures++;
    e.img.onload = () => {
      pendingPictures--;
      e.ok = true; e.w = e.img.naturalWidth; e.h = e.img.naturalHeight;
      imagesVersion++; dirty();
    };
    e.img.onerror = () => { pendingPictures--; e.failed = true; hostError(who + ': the picture "' + p + '" could not be loaded or decoded'); };
    e.img.src = packUrl(p);
    return e;
  }
  // a picture's pixels, for warping it onto the globe (once)
  function pixels(e) {
    if (e.data || e.tooBig) return e.data;
    if (e.w > 4096 || e.h > 4096) { e.tooBig = true; hostError('terra.images: "' + e.path + '" is ' + e.w + ' × ' + e.h + ': at most 4096 pixels a side; not drawn'); return null; }
    const cv = document.createElement('canvas');
    cv.width = e.w; cv.height = e.h;
    const cx = cv.getContext('2d', { willReadFrequently: true });
    cx.drawImage(e.img, 0, 0);
    e.data = cx.getImageData(0, 0, e.w, e.h).data;
    return e.data;
  }

  // ---- terra.geo: geography drawn by the globe (GeoLayers.swift), with touchable features
  const STYLE_KIND = { border: 'line', ghost: 'line', track: 'line', area: 'area', dot: 'point', ring: 'point', label: 'point', symbol: 'point', sounding: 'point' };
  const VERTEX_BUDGET = 100000, LAYER_BUDGET = 64, PLACES = 400;
  // as the app reads them (UIColor(hex:)): #rrggbb, or #rrggbbaa with its alpha
  function parseColor(v) {
    const m = typeof v === 'string' && /^#?([0-9a-f]{6}|[0-9a-f]{8})$/i.exec(v.trim());
    if (!m) return null;
    const h = m[1];
    const c = [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
    c.alpha = h.length === 8 ? parseInt(h.slice(6, 8), 16) / 255 : 1;
    return c;
  }
  const rgba = (c, a) => 'rgba(' + c[0] + ',' + c[1] + ',' + c[2] + ',' + (a == null ? 1 : a) * (c.alpha == null ? 1 : c.alpha) + ')';
  function darker(c) {        // GeoLayers.swift: saturation x1.15, brightness x0.7
    const [r, gg, b] = c.map((v) => v / 255), mx = Math.max(r, gg, b), mn = Math.min(r, gg, b), d = mx - mn;
    let h = 0;
    if (d) h = mx === r ? ((gg - b) / d) % 6 : mx === gg ? (b - r) / d + 2 : (r - gg) / d + 4;
    h = (h * 60 + 360) % 360;
    const s = mx ? Math.min(1, (d / mx) * 1.15) : 0, v = mx * 0.7, C = v * s, X = C * (1 - Math.abs(((h / 60) % 2) - 1)), m0 = v - C;
    const [r1, g1, b1] = h < 60 ? [C, X, 0] : h < 120 ? [X, C, 0] : h < 180 ? [0, C, X] : h < 240 ? [0, X, C] : h < 300 ? [X, 0, C] : [C, 0, X];
    return [r1, g1, b1].map((x) => Math.round((x + m0) * 255));
  }

  function parseLayer(id, features, options) {
    const style = options.style == null ? 'border' : options.style, kind = STYLE_KIND[style];
    const color = parseColor(options.color);
    const where = 'terra.geo.set("' + id + '")';
    if (options.color != null && !color) hostWarn(where + ': color must be "#rrggbb" or "#rrggbbaa" (got ' + JSON.stringify(options.color) + ')');
    if (options.font != null && !['regular', 'italic', 'caps'].includes(options.font)) hostWarn(where + ': font is regular, italic or caps');
    let offset = [0, 0];
    if (options.offset != null) {
      const o = options.offset;
      if (Array.isArray(o) && o.length === 2 && Number.isFinite(o[0]) && Number.isFinite(o[1])) offset = [clamp(o[0], -200, 200), clamp(o[1], -200, 200)];
      else hostWarn(where + ': offset must be [dx, dy] in points');
    }
    const L = { id, style, kind, options, color, lines: [], polys: [], points: [], vertices: 0, features: 0, skipped: 0,
      minZoom: num(options.minZoom, 0), maxZoom: num(options.maxZoom, Infinity), touchable: options.touchable === true,
      font: ['regular', 'italic', 'caps'].includes(options.font) ? options.font : 'regular', size: clamp(num(options.size, 15), 10, 40),
      width: clamp(num(options.width, 28), 4, 200), height: clamp(num(options.height, 28), 4, 200), offset, pic: null, drawn: [] };
    if (style === 'symbol') {
      if (typeof options.image !== 'string') hostError(where + ': a symbol layer needs options.image, a picture of the pack');
      else L.pic = picture(options.image, where);
    }
    const coord = (c) => (Array.isArray(c) && c.length >= 2 && Number.isFinite(c[0]) && Number.isFinite(c[1]) && Math.abs(c[1]) <= 90 && Math.abs(c[0]) <= 540 ? [c[0], c[1]] : null);
    const coords = (list, min) => {
      const pts = Array.isArray(list) ? list.map(coord) : [];
      return pts.length >= min && pts.every(Boolean) ? pts : null;
    };
    const walk = (geom, props, index, depth) => {
      if (!geom || typeof geom !== 'object' || depth > 4) { L.skipped++; return; }
      const t = geom.type, c = geom.coordinates;
      if (t === 'GeometryCollection') return (geom.geometries || []).forEach((x) => walk(x, props, index, depth + 1));
      const fits = (kind === 'point' && (t === 'Point' || t === 'MultiPoint')) || (kind === 'line' && /^(Multi)?(LineString|Polygon)$/.test(t))
        || (kind === 'area' && /^(Multi)?Polygon$/.test(t));
      if (!fits) { L.skipped++; return; }
      if (t === 'Point' || t === 'MultiPoint') {
        for (const p of t === 'Point' ? [c] : Array.isArray(c) ? c : []) {
          const q = coord(p);
          if (!q) { L.skipped++; continue; }
          const v = unit(q[0], q[1]);
          L.points.push({ index, props, lon: q[0], lat: q[1], wx: v[0], wy: v[1], wz: v[2] });
          L.vertices++;
        }
      } else if (t === 'LineString' || t === 'MultiLineString') {
        for (const part of t === 'LineString' ? [c] : Array.isArray(c) ? c : []) {
          const pts = coords(part, 2);
          if (!pts) { L.skipped++; continue; }
          L.lines.push({ index, ring: ring(pts, false), closed: false });
          L.vertices += pts.length;
        }
      } else {
        for (const poly of t === 'Polygon' ? [c] : Array.isArray(c) ? c : []) {
          const parts = Array.isArray(poly) ? poly.map((r) => coords(r, 3)) : [];
          if (!parts.length || !parts.every(Boolean)) { L.skipped++; continue; }
          if (kind === 'area') L.polys.push({ index, rings: parts.map((r) => ring(r, true)) });
          else for (const r of parts) L.lines.push({ index, ring: ring(r, true), closed: true });
          for (const r of parts) L.vertices += r.length;
        }
      }
    };
    let list = [];
    if (Array.isArray(features)) list = features;
    else if (features && typeof features === 'object') list = Array.isArray(features.features) ? features.features : [features];
    L.features = list.length;
    list.forEach((f, index) => {
      if (f && f.type === 'Feature') walk(f.geometry, f.properties && typeof f.properties === 'object' ? f.properties : {}, index, 0);
      else walk(f, f && typeof f.properties === 'object' && f.properties ? f.properties : {}, index, 0);
    });
    if (L.skipped) hostWarn(where + ': ' + L.skipped + ' geometr' + (L.skipped === 1 ? 'y' : 'ies') + ' skipped: a "' + style + '" layer draws '
      + (kind === 'point' ? 'Point and MultiPoint' : kind === 'area' ? 'Polygon and MultiPolygon' : 'LineString, MultiLineString, Polygon and MultiPolygon')
      + ' with finite [lon, lat] positions');
    if (['label', 'symbol', 'sounding'].includes(style) && L.points.length > PLACES) {
      hostWarn(where + ': ' + L.points.length + ' places; a layer draws at most ' + PLACES + ' (the rest are dropped)');
      L.points.length = PLACES;
    }
    if (L.touchable && style === 'label' && L.points.some((p) => p.props.label == null)) hostWarn(where + ': touchable labels need properties.label');
    return L;
  }

  function onGeo(msg) {
    if (msg.op === 'clear') { host.geo.clear(); dirty(); return; }
    const id = String(msg.id == null ? '' : msg.id);
    if (msg.op === 'remove') { host.geo.delete(id); dirty(); return; }
    if (msg.op !== 'set') return hostWarn('terra.geo: unknown op "' + msg.op + '"; ignored');
    const options = msg.options && typeof msg.options === 'object' && !Array.isArray(msg.options) ? msg.options : {};
    if (options.style != null && !STYLE_KIND[options.style]) {
      return hostError('terra.geo.set("' + id + '"): style must be one of ' + Object.keys(STYLE_KIND).join(', ') + ' (got ' + JSON.stringify(options.style) + '); not drawn');
    }
    const L = parseLayer(id, msg.features, options);
    let others = 0;
    for (const [k, v] of host.geo) if (k !== id) others += v.vertices;
    if (others + L.vertices > VERTEX_BUDGET) return hostError('terra.geo.set("' + id + '"): ' + L.vertices + ' vertices with the others\' ' + others + ' is over the budget of ' + VERTEX_BUDGET + '; not drawn (nothing changed)');
    if (!host.geo.has(id) && host.geo.size >= LAYER_BUDGET) return hostError('terra.geo.set("' + id + '"): over the budget of ' + LAYER_BUDGET + ' layers; not drawn');
    host.geo.set(id, L);                // a known id keeps its place in the order
    dirty();
  }

  const zoomHolds = (o, z) => z >= o.minZoom - 1e-9 && z <= o.maxZoom + 1e-9;
  // a name's picture in GeoLayers.swift: letters over a round-joined halo of the paper's tone, 3 reading points a side
  function labelFont(font, size) { return (font === 'italic' ? 'italic 400 ' : font === 'caps' ? '600 ' : '400 ') + size + 'px ' + FACE; }
  function nameBox(text, x, y, size, font) {
    g.font = labelFont(font, size);
    if ('letterSpacing' in g) g.letterSpacing = font === 'caps' ? 0.08 * size + 'px' : '0px';
    const mt = g.measureText(text), halo = 3 * rs;
    const asc = mt.fontBoundingBoxAscent || size * 0.9, desc = mt.fontBoundingBoxDescent || size * 0.25;
    const w = mt.width + 2 * halo + 2, h = asc + desc + 2 * halo + 2;
    return { frame: [x - w / 2, y - h / 2, w, h], baseline: y + (asc - desc) / 2 };
  }
  function drawName(text, x, baseline, ink) {
    g.textAlign = 'center'; g.textBaseline = 'alphabetic';
    g.lineJoin = 'round'; g.lineWidth = 6 * rs; g.strokeStyle = LABEL_TONE; g.strokeText(text, x, baseline);
    g.fillStyle = ink; g.fillText(text, x, baseline);
  }

  function drawGeo(c, m, placed) {
    const o = { x: 0, y: 0, z: 0 }, screen = [0, 0, W, H];
    for (const L of host.geo.values()) {
      L.drawn = [];
      if (!zoomHolds(L, c.zoom)) continue;
      g.save();
      g.lineJoin = 'round'; g.lineCap = 'round';
      const k = rs, ink = L.color || INK;
      const strokePath = () => {
        const path = new Path2D();
        for (const l of L.lines) traceLine(l.ring, m, c.cx, c.cy, c.radius, path, l.closed);
        return path;
      };
      switch (L.style) {
        case 'border':
          g.strokeStyle = rgba(L.color || [140, 132, 120]); g.lineWidth = 1.1 * k; g.stroke(strokePath());
          break;
        case 'ghost':
          g.setLineDash([5 * k, 4 * k]); g.strokeStyle = rgba(ink, 0.8); g.lineWidth = 1.6 * k; g.stroke(strokePath());
          break;
        case 'track':
          g.setLineDash([3 * k, 4 * k]); g.strokeStyle = rgba(ink, 0.47); g.lineWidth = 1.2 * k; g.stroke(strokePath());
          break;
        case 'area': {
          const base = L.color || AREA, fill = new Path2D(), edge = new Path2D();
          g.save(); g.beginPath(); g.arc(c.cx, c.cy, c.radius, 0, TAU); g.clip();
          for (const poly of L.polys) for (const r of poly.rings) trace(r, m, c.cx, c.cy, c.radius, fill, edge);
          g.fillStyle = rgba(base, 0.28); g.fill(fill, 'evenodd');
          g.restore();
          g.strokeStyle = rgba(darker(base)); g.lineWidth = 2.2 * k; g.stroke(edge);
          break;
        }
        case 'dot':
        case 'ring': {
          const r = 3.4 * k, dots = new Path2D();
          for (const p of L.points) {
            projectUnit(p.wx, p.wy, p.wz, m, c, o);
            if (o.z <= 0.1) continue;
            dots.moveTo(o.x + r, o.y); dots.arc(o.x, o.y, r, 0, TAU);
            L.drawn.push({ index: p.index, x: o.x, y: o.y });
          }
          if (L.style === 'dot') { g.fillStyle = rgba(ink); g.fill(dots); }
          else { g.fillStyle = DOT_PAPER; g.fill(dots); g.strokeStyle = rgba(ink); g.lineWidth = 1.3 * k; g.stroke(dots); }
          break;
        }
        case 'label': {
          const size = L.size * k, reach = [-120, -60, W + 240, H + 120];
          for (const p of L.points) {
            let text = p.props.label == null ? '' : String(p.props.label).slice(0, 80);
            if (!text) continue;
            projectUnit(p.wx, p.wy, p.wz, m, c, o);
            if (o.z <= 0.3 || !within(o.x, o.y, reach)) continue;
            if (L.font === 'caps') text = text.toUpperCase();
            const x = o.x + L.offset[0] * k, y = o.y + L.offset[1] * k, box = nameBox(text, x, y, size, L.font);
            const room = grow(box.frame, 0.2 * L.size * k);
            if (!inside(box.frame, screen) || placed.some((q) => overlaps(room, q))) continue;    // earlier ones win
            placed.push(room);
            drawName(text, x, box.baseline, rgba(ink));
            L.drawn.push({ index: p.index, x, y, rect: box.frame });
          }
          break;
        }
        case 'symbol': {
          const pic = L.pic;
          if (!pic || !pic.ok) break;
          const w = L.width * k, h = L.height * k;
          for (const p of L.points) {
            projectUnit(p.wx, p.wy, p.wz, m, c, o);
            if (o.z <= 0.2 || !within(o.x, o.y, [-w, -h, W + 2 * w, H + 2 * h])) continue;
            const x = o.x + L.offset[0] * k, y = o.y + L.offset[1] * k;
            // (aspect-fit, as the layer's contentsGravity)
            const s = Math.min(w / pic.w, h / pic.h), dw = pic.w * s, dh = pic.h * s;
            g.drawImage(pic.img, x - dw / 2, y - dh / 2, dw, dh);
            const rect = [x - w / 2, y - h / 2, w, h];
            placed.push(rect);
            L.drawn.push({ index: p.index, x, y, rect });
          }
          break;
        }
        case 'sounding': {
          for (const p of L.points) {
            projectUnit(p.wx, p.wy, p.wz, m, c, o);
            if (o.z <= 0.2 || !within(o.x, o.y, [-40, -40, W + 80, H + 80])) continue;
            const length = clamp(num(p.props.length, 0), 0, 2000) * k, y = o.y + length;
            g.setLineDash([2 * k, 3 * k]); g.lineCap = 'butt'; g.strokeStyle = SOUNDING_LINE; g.lineWidth = 1.4 * k;
            g.beginPath(); g.moveTo(o.x, o.y); g.lineTo(o.x, y); g.stroke();
            g.setLineDash([]); g.lineCap = 'round'; g.strokeStyle = SOUNDING_FOOT; g.lineWidth = 2 * k;
            g.beginPath(); g.moveTo(o.x - 8 * k, y); g.lineTo(o.x + 8 * k, y); g.stroke();
            const text = p.props.label == null ? '' : String(p.props.label).slice(0, 80);
            let rect = [o.x - 8 * k, o.y, 16 * k, length];
            if (text) {
              const box = nameBox(text, 0, y, 14 * k, 'regular'), fw = box.frame[2];
              drawName(text, o.x + 10 * k + fw / 2, box.baseline, SOUNDING_INK);
              const lr = [o.x + 10 * k, y - box.frame[3] / 2, fw, box.frame[3]];
              placed.push(lr);
              rect = [rect[0], rect[1], Math.max(rect[2], lr[0] + lr[2] - rect[0]), Math.max(rect[3], lr[1] + lr[3] - rect[1])];
            }
            L.drawn.push({ index: p.index, x: o.x, y: o.y, foot: [o.x, y], rect });
          }
          break;
        }
      }
      g.restore();
    }
  }

  // ---- terra.images: pictures in longitude and latitude, lying on the globe (warped pixel by pixel)
  const IMAGE_BUDGET = 8;
  function onImage(msg) {
    if (msg.op === 'clear') { host.images.clear(); imagesVersion++; dirty(); return; }
    const id = String(msg.id == null ? '' : msg.id), where = 'terra.images.set("' + id + '")';
    if (msg.op === 'remove') { host.images.delete(id); imagesVersion++; dirty(); return; }
    if (msg.op !== 'set') return hostWarn('terra.images: unknown op "' + msg.op + '"; ignored');
    const o = msg.options || {}, b = o.bounds;
    if (typeof o.image !== 'string') return hostError(where + ': image must be a picture of the pack; not drawn');
    if (!Array.isArray(b) || b.length !== 4 || !b.every(Number.isFinite) || !(b[3] > b[1]) || Math.abs(b[1]) > 90 || Math.abs(b[3]) > 90 || b[0] === b[2]) {
      return hostError(where + ': bounds must be [west, south, east, north] in degrees, north above south (got ' + JSON.stringify(b) + '); not drawn');
    }
    if (!host.images.has(id) && host.images.size >= IMAGE_BUDGET) return hostError(where + ': at most ' + IMAGE_BUDGET + ' pictures; not drawn');
    const pic = picture(o.image, where);
    if (!pic) return;
    host.images.set(id, { id, path: pic.path, pic, bounds: b.slice(), opacity: clamp(num(o.opacity, 1), 0, 1),
                          minZoom: num(o.minZoom, 0), maxZoom: num(o.maxZoom, Infinity), fade: Math.max(0, num(o.fade, 0)) });
    imagesVersion++;
    dirty();
  }
  function imageAlpha(im, z) {
    if (!zoomHolds(im, z)) return 0;
    return im.opacity * (im.fade > 0 ? clamp((z - im.minZoom) / im.fade, 0, 1) : 1);
  }
  // where on screen a picture can be: the box round the samples of it that face the reader, or the disc when it reaches the limb
  function screenBox(im, c, m) {
    const [w, s, e, n] = im.bounds, span = e > w ? e - w : e + 360 - w, o = { x: 0, y: 0, z: 0 }, N = 16;
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity, seen = 0, hidden = 0, step = 0;
    let prev = null;
    for (let j = 0; j <= N; j++) {
      for (let i = 0; i <= N; i++) {
        const v = unit(w + span * i / N, s + (n - s) * j / N);
        projectUnit(v[0], v[1], v[2], m, c, o);
        if (o.z <= 0) { hidden++; prev = null; continue; }
        seen++;
        x0 = Math.min(x0, o.x); y0 = Math.min(y0, o.y); x1 = Math.max(x1, o.x); y1 = Math.max(y1, o.y);
        if (prev && i > 0) step = Math.max(step, Math.hypot(o.x - prev[0], o.y - prev[1]));
        prev = [o.x, o.y];
      }
      prev = null;
    }
    if (!seen) return null;
    if (hidden) { x0 = Math.min(x0, c.cx - c.radius); y0 = Math.min(y0, c.cy - c.radius); x1 = Math.max(x1, c.cx + c.radius); y1 = Math.max(y1, c.cy + c.radius); }
    const pad = step * 0.5 + 2;
    x0 = Math.max(0, x0 - pad); y0 = Math.max(0, y0 - pad); x1 = Math.min(W, x1 + pad); y1 = Math.min(H, y1 + pad);
    return x1 > x0 && y1 > y0 ? [x0, y0, x1, y1] : null;
  }
  const warp = { canvas: document.createElement('canvas'), key: '', img: null };
  function drawImages(c, m, dpr, moving) {
    const list = [];
    for (const im of host.images.values()) if (im.pic.ok && imageAlpha(im, c.zoom) > 0.001 && pixels(im.pic)) list.push(im);
    if (!list.length) return;
    const s = moving ? 1 : dpr;          // half the pixels while the camera moves; sharp again when it stops
    const key = [c.lon, c.lat, c.zoom, c.radius, s, imagesVersion].join('|');
    if (warp.key !== key) { renderWarp(list, c, m, s); warp.key = key; }
    g.drawImage(warp.canvas, 0, 0, W, H);
  }
  function renderWarp(list, c, m, s) {
    const pw = Math.round(W * s), ph = Math.round(H * s);
    if (warp.canvas.width !== pw || warp.canvas.height !== ph) { warp.canvas.width = pw; warp.canvas.height = ph; warp.img = null; }
    const wctx = warp.canvas.getContext('2d');
    if (!warp.img) warp.img = wctx.createImageData(pw, ph);
    const out = warp.img.data;
    out.fill(0);
    const R = c.radius * s, cx = c.cx * s, cy = c.cy * s;
    const a = c.lon * D, b = -c.lat * D, ca = Math.cos(a), sa = Math.sin(a), cb = Math.cos(b), sb = Math.sin(b);
    for (const im of list) {
      const box = screenBox(im, c, m);
      if (!box) continue;
      const alpha = imageAlpha(im, c.zoom), src = im.pic.data, sw = im.pic.w, sh = im.pic.h;
      const [west, south, east, north] = im.bounds, span = east > west ? east - west : east + 360 - west, latSpan = north - south;
      const X0 = Math.floor(box[0] * s), Y0 = Math.floor(box[1] * s), X1 = Math.min(pw, Math.ceil(box[2] * s)), Y1 = Math.min(ph, Math.ceil(box[3] * s));
      for (let py = Y0; py < Y1; py++) {
        const vy = (cy - (py + 0.5)) / R;
        for (let px = X0; px < X1; px++) {
          const vx = (px + 0.5 - cx) / R, r2 = vx * vx + vy * vy;
          if (r2 >= 1) continue;
          const vz = Math.sqrt(1 - r2);
          const wy = cb * vy - sb * vz;
          const lat = Math.asin(wy > 1 ? 1 : wy < -1 ? -1 : wy) / D;
          if (lat < south || lat > north) continue;
          const wx = ca * vx + sa * sb * vy + sa * cb * vz, wz = -sa * vx + ca * sb * vy + ca * cb * vz;
          let du = (Math.atan2(wx, wz) / D - west) % 360;
          if (du < 0) du += 360;
          if (du > span) continue;
          // bilinear, edges clamped
          const u = (du / span) * sw - 0.5, v = ((north - lat) / latSpan) * sh - 0.5;
          let ix = Math.floor(u), iy = Math.floor(v);
          const fx = u - ix, fy = v - iy;
          const xa = ix < 0 ? 0 : ix >= sw ? sw - 1 : ix, xb = ix + 1 >= sw ? sw - 1 : ix + 1 < 0 ? 0 : ix + 1;
          const ya = iy < 0 ? 0 : iy >= sh ? sh - 1 : iy, yb = iy + 1 >= sh ? sh - 1 : iy + 1 < 0 ? 0 : iy + 1;
          const p00 = (ya * sw + xa) * 4, p10 = (ya * sw + xb) * 4, p01 = (yb * sw + xa) * 4, p11 = (yb * sw + xb) * 4;
          const w00 = (1 - fx) * (1 - fy), w10 = fx * (1 - fy), w01 = (1 - fx) * fy, w11 = fx * fy;
          const A = (src[p00 + 3] * w00 + src[p10 + 3] * w10 + src[p01 + 3] * w01 + src[p11 + 3] * w11) / 255 * alpha;
          if (A <= 0.002) continue;
          // (premultiplied while mixing, so a transparent pixel's colour doesn't bleed)
          const k00 = w00 * src[p00 + 3], k10 = w10 * src[p10 + 3], k01 = w01 * src[p01 + 3], k11 = w11 * src[p11 + 3], kk = k00 + k10 + k01 + k11;
          const r = (src[p00] * k00 + src[p10] * k10 + src[p01] * k01 + src[p11] * k11) / kk;
          const gg = (src[p00 + 1] * k00 + src[p10 + 1] * k10 + src[p01 + 1] * k01 + src[p11 + 1] * k11) / kk;
          const bb = (src[p00 + 2] * k00 + src[p10 + 2] * k10 + src[p01 + 2] * k01 + src[p11 + 2] * k11) / kk;
          const q = (py * pw + px) * 4, dA = out[q + 3] / 255, oA = A + dA * (1 - A);
          out[q] = (r * A + out[q] * dA * (1 - A)) / oA;
          out[q + 1] = (gg * A + out[q + 1] * dA * (1 - A)) / oA;
          out[q + 2] = (bb * A + out[q + 2] * dA * (1 - A)) / oA;
          out[q + 3] = oA * 255;
        }
      }
    }
    wctx.putImageData(warp.img, 0, 0);
  }

  // ---- terra.sprites (and the doorway's look): things on the globe, moving on great circles, their look chosen by the zoom
  const SPRITE_BUDGET = 64, FRAME_BUDGET = 24;
  function parseLook(look, where) {
    if (!look || typeof look !== 'object') { hostError(where + ': a look must be an object'); return null; }
    let names = null;
    if (Array.isArray(look.frames)) {
      names = look.frames;
      if (!names.length) { hostError(where + ': frames is empty'); return null; }
      if (names.length > FRAME_BUDGET) { hostError(where + ': ' + names.length + ' frames; at most ' + FRAME_BUDGET + ' (the rest are dropped)'); names = names.slice(0, FRAME_BUDGET); }
    } else if (typeof look.image === 'string') names = [look.image];
    else { hostError(where + ': a look needs image (a picture) or frames (pictures)'); return null; }
    const pics = names.map((n) => picture(n, where));
    if (pics.some((p) => !p)) return null;
    let size = look.size;
    if (!Array.isArray(size) || size.length !== 2 || !(size[0] > 0) || !(size[1] > 0)) {
      hostWarn(where + ': size must be [width, height] in points (got ' + JSON.stringify(size) + '); drawn at 28 × 28');
      size = [28, 28];
    }
    return { pics, fps: clamp(num(look.fps, 8), 0.1, 60), w: clamp(size[0], 1, 400), h: clamp(size[1], 1, 400),
             minZoom: num(look.minZoom, 0), maxZoom: num(look.maxZoom, Infinity),
             every: num(look.every, 0) > 0 && num(look.for, 0) > 0 ? look.every : 0, for: num(look.for, 0), face: look.face === true };
  }
  const validPos = (c) => Array.isArray(c) && c.length >= 2 && Number.isFinite(c[0]) && Number.isFinite(c[1]) && Math.abs(c[1]) <= 90;
  function buildPath(coords) {
    const pts = coords.map((c) => unit(c[0], c[1])), cum = [0];
    for (let i = 1; i < pts.length; i++) {
      const a = pts[i - 1], b = pts[i], d = clamp(a[0] * b[0] + a[1] * b[1] + a[2] * b[2], -1, 1);
      cum.push(cum[i - 1] + Math.acos(d));
    }
    return { pts, cum, total: cum[cum.length - 1] };
  }
  function along(path, f) {           // the place a fraction f of the way, on great circles
    const { pts, cum, total } = path;
    if (!(total > 0)) return pts[0];
    const d = clamp(f, 0, 1) * total;
    let i = 0;
    while (i < cum.length - 2 && cum[i + 1] < d) i++;
    const len = cum[i + 1] - cum[i], t = len > 0 ? (d - cum[i]) / len : 0, a = pts[i], b = pts[i + 1];
    if (len < 1e-9) return a;
    const s = Math.sin(len), ka = Math.sin((1 - t) * len) / s, kb = Math.sin(t * len) / s;
    return [a[0] * ka + b[0] * kb, a[1] * ka + b[1] * kb, a[2] * ka + b[2] * kb];
  }
  function onSprite(msg) {
    if (msg.op === 'clear') { host.sprites.clear(); dirty(); return; }
    const id = String(msg.id == null ? '' : msg.id), where = 'terra.sprites.set("' + id + '")';
    if (msg.op === 'remove') { host.sprites.delete(id); dirty(); return; }
    if (msg.op !== 'set') return hostWarn('terra.sprites: unknown op "' + msg.op + '"; ignored');
    const o = msg.options || {};
    const s = { id, at: null, path: null, seconds: 60, loop: true, key: '', looks: [], label: typeof o.label === 'string' ? o.label : '', touchable: o.touchable === true, t0: now(), flip: false, drawn: null };
    if (o.path != null) {
      const p = o.path, coords = p && Array.isArray(p.coordinates) ? p.coordinates : null;
      if (!coords || !coords.length || !coords.every(validPos)) return hostError(where + ': path.coordinates must be [[lon, lat], …]; not drawn');
      if (coords.length === 1) s.at = unit(coords[0][0], coords[0][1]);
      else {
        s.path = buildPath(coords);
        if (p.seconds != null && !(p.seconds > 0)) hostWarn(where + ': path.seconds must be above 0; 60 used');
        s.seconds = p.seconds > 0 ? +p.seconds : 60;
        if (p.loop != null && ![true, false, 'back'].includes(p.loop)) hostWarn(where + ': path.loop is true, "back" or false; true used');
        s.loop = p.loop === false ? false : p.loop === 'back' ? 'back' : true;
        s.key = JSON.stringify([coords, s.seconds, s.loop]);
      }
    } else if (validPos(o.at)) s.at = unit(o.at[0], o.at[1]);
    else return hostError(where + ': needs at: [lon, lat] or path: { coordinates, seconds, loop }; not drawn');
    const looks = Array.isArray(o.looks) ? o.looks : o.look ? [o.look] : [];
    if (!looks.length) return hostError(where + ': needs looks: [{ image | frames, size, … }]; not drawn');
    for (let i = 0; i < looks.length; i++) {
      const l = parseLook(looks[i], where + ' look ' + i);
      if (l) s.looks.push(l);
    }
    if (!s.looks.length) return;
    if (s.touchable && !s.label) hostWarn(where + ': a touchable sprite needs a label (its VoiceOver name)');
    const old = host.sprites.get(id);
    if (!old && host.sprites.size >= SPRITE_BUDGET) return hostError(where + ': at most ' + SPRITE_BUDGET + ' sprites; not drawn');
    if (old && s.key && old.key === s.key) { s.t0 = old.t0; s.flip = old.flip; }      // the same path keeps its place along it
    host.sprites.set(id, s);
    dirty();
  }
  // where a sprite is now, and whether it is heading west on screen
  function spritePlace(s, t, c, m) {
    if (s.at) return { v: s.at, moving: false };
    if (settings.reduceMotion) return { v: s.path.pts[0], moving: false };
    const T = (t - s.t0) / 1000 / s.seconds;
    let f, dir = 1;
    if (s.loop === true) f = T - Math.floor(T);
    else if (s.loop === 'back') { const k = T % 2; f = k <= 1 ? k : 2 - k; dir = k <= 1 ? 1 : -1; }
    else { f = Math.min(T, 1); if (f >= 1) dir = 0; }
    const v = along(s.path, f);
    if (dir) {
      const ahead = along(s.path, clamp(f + dir * 0.002, 0, 1)), o1 = projectUnit(v[0], v[1], v[2], m, c, {}), o2 = projectUnit(ahead[0], ahead[1], ahead[2], m, c, {});
      const dx = o2.x - o1.x;
      if (Math.abs(dx) > 1e-4 * c.radius) s.flip = dx < 0;
    }
    return { v, moving: dir !== 0 };
  }
  function chooseLook(looks, z) {
    for (let i = 0; i < looks.length; i++) if (zoomHolds(looks[i], z)) return i;
    return -1;
  }
  // draws a look at a place; returns what was drawn, or null (hidden: behind, off screen, under what the app keeps clear)
  function drawLook(look, v, flip, T, c, m, clear) {
    if (look.every && !settings.reduceMotion && (T % look.every) >= look.for) return null;
    const o = projectUnit(v[0], v[1], v[2], m, c, {});
    if (o.z < 0.2) return null;
    const w = look.w * rs, h = look.h * rs, rect = [o.x - w / 2, o.y - h / 2, w, h];
    if (!overlaps(rect, [0, 0, W, H]) || clear.some((q) => overlaps(rect, q))) return null;
    const pic = look.pics[settings.reduceMotion ? 0 : Math.floor(Math.max(0, T) * look.fps) % look.pics.length];
    if (!pic || !pic.ok) return null;
    g.save();
    g.translate(o.x, o.y);
    if (flip) g.scale(-1, 1);
    g.drawImage(pic.img, -w / 2, -h / 2, w, h);
    g.restore();
    return { x: o.x, y: o.y, rect };
  }
  function drawSprites(t, c, m, clear) {
    for (const s of host.sprites.values()) {
      s.drawn = null;
      const li = chooseLook(s.looks, c.zoom);
      if (li < 0) continue;
      const look = s.looks[li], place = spritePlace(s, t, c, m), T = (t - s.t0) / 1000;
      const d = drawLook(look, place.v, look.face && s.flip, T, c, m, clear);
      if (d) s.drawn = Object.assign(d, { look: li });
    }
  }
  function spritesAnimate() {
    for (const s of host.sprites.values()) {
      if (settings.reduceMotion) continue;
      if (s.path && (s.loop !== false || (now() - s.t0) / 1000 < s.seconds)) return true;
      if (s.looks.some((l) => l.pics.length > 1 || l.every)) return true;
    }
    return false;
  }

  // ---- the doorway: the pack on the globe while it isn't running; touching it starts the pack
  let doorway = null;
  function readDoorway() {
    const d = M.doorway;
    if (!d || M.shelf === 'parked') return null;
    if (!validPos([d.lon, d.lat])) { hostWarn('prototype.json doorway: lon and lat must be numbers; no doorway'); return null; }
    if (!d.label) hostWarn('prototype.json doorway: needs a label (its VoiceOver name)');
    const look = parseLook(d.look, 'prototype.json doorway look');
    if (!look) return null;
    return { lon: d.lon, lat: d.lat, v: unit(d.lon, d.lat), label: String(d.label || M.title || ID), look, t0: now(), drawn: null };
  }

  // ---- what the app keeps clear: the chip (or Start), the foot, the panel
  function keepClear() {
    const dr = device.getBoundingClientRect(), out = [];
    for (const el of [$('chip'), $('start'), $('foot'), $('panel')]) {
      if (el.hidden) continue;
      const r = el.getBoundingClientRect();
      out.push([r.left - dr.left, r.top - dr.top, r.width, r.height]);
    }
    return out;
  }

  // ---- drawing, each frame something changed or moves: the globe, its pictures and geography are kept in a layer
  // drawn again only when the camera or they change; sprites (and the doorway) go over it every frame
  let cameraMoving = false;
  const layer = { canvas: document.createElement('canvas'), key: '' };
  api.layerDraws = 0;
  function render(t) {
    const began = now(), dpr = window.devicePixelRatio || 1, pw = Math.round(W * dpr), ph = Math.round(H * dpr);
    if (canvas.width !== pw || canvas.height !== ph) { canvas.width = pw; canvas.height = ph; }
    const c = camera(), m = rotation(c), clear = keepClear();
    cameraMoving = !!flight || pointers.size > 0 || t - lastCamMove < 180;
    const key = [c.lon, c.lat, c.zoom, c.radius, rs, dpr, layerVersion, imagesVersion, cameraMoving,
                 clear.map((r) => r.map(Math.round).join(',')).join(';')].join('|');
    if (layer.key !== key) {
      layer.key = key;
      if (layer.canvas.width !== pw || layer.canvas.height !== ph) { layer.canvas.width = pw; layer.canvas.height = ph; }
      g = layer.canvas.getContext('2d');
      thin = cameraMoving ? THIN_MOVING : THIN_AT_REST;
      g.setTransform(dpr, 0, 0, dpr, 0, 0);
      g.fillStyle = PAPER; g.fillRect(0, 0, W, H);
      drawRing(c);
      drawGlobe(c, m);
      drawImages(c, m, dpr, cameraMoving);
      if (politics && !host.globe.historical) drawCountryNames(c, m, clear.slice());
      drawGeo(c, m, clear.slice());
      g = screenCtx;
      api.layerDraws++;
    }
    g.setTransform(1, 0, 0, 1, 0, 0);
    g.drawImage(layer.canvas, 0, 0);
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    drawSprites(t, c, m, clear);
    if (doorway) {
      doorway.drawn = null;
      if (!api.running) {
        const look = doorway.look;
        if (zoomHolds(look, c.zoom)) doorway.drawn = drawLook(look, doorway.v, false, (t - doorway.t0) / 1000, c, m, clear);
      }
    }
    api.frame++;
    api.renderMs = +((api.renderMs || 0) * 0.9 + (now() - began) * 0.1).toFixed(2);
  }

  // ---- touch: what is under a point (sprites first, then touchable geography, topmost first)
  const hitCtx = document.createElement('canvas').getContext('2d');
  function segDist(px, py, ax, ay, bx, by) {
    const dx = bx - ax, dy = by - ay, l2 = dx * dx + dy * dy;
    const t = l2 ? clamp(((px - ax) * dx + (py - ay) * dy) / l2, 0, 1) : 0;
    return Math.hypot(px - ax - t * dx, py - ay - t * dy);
  }
  function hitTest(x, y) {
    const sprites = [...host.sprites.values()].reverse();
    for (const s of sprites) {
      if (!s.touchable || !s.drawn) continue;
      const r = s.drawn.rect, w = Math.max(r[2], 44), h = Math.max(r[3], 44);
      if (within(x, y, [s.drawn.x - w / 2, s.drawn.y - h / 2, w, h])) return { sprite: s.id };
    }
    const c = camera(), m = rotation(c);
    for (const L of [...host.geo.values()].reverse()) {
      if (!L.touchable || !zoomHolds(L, c.zoom)) continue;
      let best = null, bestD = Infinity;
      if (L.kind === 'point') {
        for (const d of L.drawn) {
          let dist = Math.hypot(x - d.x, y - d.y);
          if (d.foot) dist = Math.min(dist, Math.hypot(x - d.foot[0], y - d.foot[1]));
          if (d.rect && within(x, y, d.rect)) dist = 0;
          if (dist <= 22 && dist < bestD) { bestD = dist; best = d.index; }
        }
      } else if (L.kind === 'line') {
        for (const l of L.lines) {
          const r = l.ring;
          rotate(r, m);
          for (let i = 0, edges = l.closed ? r.n : r.n - 1; i < edges; i++) {
            const j = (i + 1) % r.n;
            if (r.vz[i] <= 0 || r.vz[j] <= 0) continue;
            const dist = segDist(x, y, c.cx + r.vx[i] * c.radius, c.cy - r.vy[i] * c.radius, c.cx + r.vx[j] * c.radius, c.cy - r.vy[j] * c.radius);
            if (dist <= 14 && dist < bestD) { bestD = dist; best = l.index; }
          }
        }
      } else {
        if (Math.hypot(x - c.cx, y - c.cy) >= c.radius) continue;
        for (const poly of L.polys) {
          const path = new Path2D();
          let any = false;
          for (const r of poly.rings) any = trace(r, m, c.cx, c.cy, c.radius, path, null) || any;
          if (any && hitCtx.isPointInPath(path, x, y, 'evenodd')) { best = poly.index; bestD = 0; }
        }
      }
      if (best != null) return { layer: L.id, index: best };
    }
    return null;
  }
  const sameTarget = (a, b) => !!a && !!b && a.sprite === b.sprite && a.layer === b.layer && a.index === b.index;
  // every touchable thing on screen now, and a point that touches it (for checks)
  function touchables() {
    const c = camera(), m = rotation(c), clear = keepClear(), out = [], o = {};
    const ok = (x, y) => x > 6 && y > 6 && x < W - 6 && y < H - 6 && !clear.some((r) => within(x, y, r));
    const add = (target, x, y, what) => { if (ok(x, y) && sameTarget(hitTest(x, y), target)) out.push({ target, x, y, what }); };
    for (const s of host.sprites.values()) if (s.touchable && s.drawn) add({ sprite: s.id }, s.drawn.x, s.drawn.y, 'sprite "' + s.id + '"' + (s.label ? ' (' + s.label + ')' : ''));
    for (const L of host.geo.values()) {
      if (!L.touchable || !zoomHolds(L, c.zoom)) continue;
      const what = (i) => L.style + ' layer "' + L.id + '" feature ' + i;
      if (L.kind === 'point') for (const d of L.drawn) add({ layer: L.id, index: d.index }, d.x, d.y, what(d.index));
      else if (L.kind === 'line') {
        for (const l of L.lines) {
          const r = l.ring;
          rotate(r, m);
          const mid = Math.floor(r.n / 2);
          for (let k = 0; k < r.n; k++) {
            const i = (mid + (k % 2 ? -1 : 1) * Math.ceil(k / 2) + r.n) % r.n;
            if (r.vz[i] > 0.05) { add({ layer: L.id, index: l.index }, c.cx + r.vx[i] * c.radius, c.cy - r.vy[i] * c.radius, what(l.index)); break; }
          }
        }
      } else {
        for (const poly of L.polys) {
          const r = poly.rings[0];
          rotate(r, m);
          let sx = 0, sy = 0, n = 0;
          const pts = [];
          for (let i = 0; i < r.n; i++) if (r.vz[i] > 0) { const x = c.cx + r.vx[i] * c.radius, y = c.cy - r.vy[i] * c.radius; sx += x; sy += y; n++; pts.push([x, y]); }
          if (!n) continue;
          const cands = [[sx / n, sy / n]];
          for (let k = 0; k < Math.min(pts.length, 12); k++) { const p = pts[Math.floor(k * pts.length / 12)]; cands.push([(p[0] + sx / n) / 2, (p[1] + sy / n) / 2]); }
          const before = out.length;
          for (const [x, y] of cands) { add({ layer: L.id, index: poly.index }, x, y, what(poly.index)); if (out.length > before) break; }
        }
      }
    }
    return out;
  }

  // ---- the foot: the caption and the date (GlobeFoot), in Globe/type.json's caption type
  const foot = $('foot'), strip = $('date-strip'), playBtn = $('date-play');
  const PLAY = '<svg viewBox="0 0 44 44" aria-hidden="true"><path d="M17 14.5 L30 22 L17 29.5 Z" fill="currentColor" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg>';
  const PAUSE = '<svg viewBox="0 0 44 44" aria-hidden="true"><rect x="16" y="14.5" width="4.2" height="15" rx="1" fill="currentColor"/><rect x="23.8" y="14.5" width="4.2" height="15" rx="1" fill="currentColor"/></svg>';
  const DAYS = new Intl.DateTimeFormat('en-GB', { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' });
  function timeLabel(t) {
    if (t.unit === 'day') {
      const d = new Date(Math.floor(t.value) * 86400000);
      return isNaN(d.getTime()) ? '' : DAYS.format(d);
    }
    const y = Math.round(t.value);
    return y < 1 ? (1 - y) + ' BCE' : String(y);
  }
  function applyCaptionType() {
    const cap = TYPE.styles && TYPE.styles.wikipedia && TYPE.styles.wikipedia.caption;
    if (!cap) return;
    const set = (el, s, base) => {
      if (!s) return;
      if (s.size) el.style.fontSize = 'calc(' + s.size + 'px * var(--ts))';
      if (s.color) el.style.color = s.color;
      if (s.tracking != null) el.style.letterSpacing = s.tracking + 'px';
    };
    set($('foot-eyebrow'), cap.eyebrow);
    set($('foot-title'), cap.title);
    set($('foot-text'), cap.text);
  }
  function updateFoot() {
    const t = host.time, cap = host.caption;
    foot.hidden = !(t || cap.title || cap.text);
    if (foot.hidden) return;
    const set = (id, text) => { const el = $(id); el.textContent = text; el.hidden = !text; };
    set('foot-eyebrow', cap.title);
    set('foot-title', t ? timeLabel(t) : '');
    set('foot-text', cap.text);
    $('foot-date').hidden = !t;
    if (!t) return;
    playBtn.hidden = !t.playable;
    playBtn.innerHTML = t.playing ? PAUSE : PLAY;
    playBtn.setAttribute('aria-label', t.playing ? 'Pause' : 'Play');
    strip.setAttribute('aria-valuemin', t.min); strip.setAttribute('aria-valuemax', t.max);
    strip.setAttribute('aria-valuenow', t.value); strip.setAttribute('aria-valuetext', timeLabel(t));
    drawStrip();
  }
  function drawStrip() {
    const t = host.time;
    if (!t || $('foot-date').hidden) return;
    const dpr = window.devicePixelRatio || 1, cw = Math.max(1, strip.clientWidth), ch = 44;
    if (strip.width !== Math.round(cw * dpr) || strip.height !== Math.round(ch * dpr)) { strip.width = Math.round(cw * dpr); strip.height = Math.round(ch * dpr); }
    const s = strip.getContext('2d'), y = ch / 2, at = (v) => ((v - t.min) / (t.max - t.min)) * cw;
    s.setTransform(dpr, 0, 0, dpr, 0, 0);
    s.clearRect(0, 0, cw, ch);
    s.strokeStyle = 'rgba(37, 52, 64, 0.35)'; s.lineWidth = 1;
    s.beginPath(); s.moveTo(0, y); s.lineTo(cw, y); s.stroke();
    const fine = t.ticks.length * 7 > cw;
    for (const tk of t.ticks) {
      const x = at(tk.at);
      if (fine) {
        s.strokeStyle = tk.hollow ? 'rgba(37, 52, 64, 0.35)' : 'rgba(37, 52, 64, 0.8)';
        s.beginPath(); s.moveTo(x, y - 4); s.lineTo(x, y + 4); s.stroke();
        continue;
      }
      s.beginPath(); s.arc(x, y, 2.6, 0, TAU);
      if (tk.hollow) { s.fillStyle = '#F6F2E7'; s.fill(); s.strokeStyle = '#253440'; s.stroke(); }
      else { s.fillStyle = '#253440'; s.fill(); }
    }
    const x = at(t.value);
    s.strokeStyle = '#B04234'; s.lineWidth = 2; s.lineCap = 'round';
    s.beginPath(); s.moveTo(x, y - 12); s.lineTo(x, y + 8); s.stroke();
  }
  function showTime(o) {
    if (!o || typeof o !== 'object') o = {};
    const unit = o.unit === 'day' ? 'day' : 'year';
    if (o.unit != null && o.unit !== 'year' && o.unit !== 'day') hostWarn('terra.time.show: unit is "year" or "day" (got ' + JSON.stringify(o.unit) + '); years used');
    const limit = unit === 'year' ? 1e7 : 3.66e9;
    if (!Number.isFinite(o.min) || !Number.isFinite(o.max)) return hostError('terra.time.show: min and max must be numbers (got ' + JSON.stringify([o.min, o.max]) + '); not shown');
    const lo = Math.max(o.min, -limit), hi = Math.min(o.max, limit);
    if (!(hi > lo)) return hostError('terra.time.show: max must be above min; not shown');
    if (o.carry) warnOnce('carry', 'terra.time.show: carry has no earlier prototype\'s date in the preview; value used');
    const ticks = [];
    for (const tk of (Array.isArray(o.ticks) ? o.ticks : []).slice(0, 2000)) {
      const at = typeof tk === 'number' ? tk : tk && typeof tk === 'object' ? tk.at : NaN;
      if (Number.isFinite(at) && at >= lo && at <= hi) ticks.push({ at, hollow: !!(tk && typeof tk === 'object' && tk.hollow) });
    }
    stopPlayback(false);
    host.time = { unit, min: lo, max: hi, value: clamp(num(o.value, lo), lo, hi), ticks, playable: !!o.playable,
                  rate: clamp(num(o.rate, 1), 0.001, (hi - lo) / 2), playing: false };
    if (o.playing && host.time.playable && !settings.reduceMotion) host.time.playing = true;
    updateFoot();
    sendTime('host');                       // the first value
  }
  function sendTime(by) {
    const t = host.time;
    if (t) deliver({ type: 'time', value: t.value, playing: t.playing, by });
  }
  let lastPlaySent = -1e9;
  function stepPlayback(t, dt) {
    const tm = host.time;
    if (!tm || !tm.playing) return;
    if (settings.reduceMotion) { tm.playing = false; updateFoot(); sendTime('host'); return; }
    tm.value = Math.min(tm.value + tm.rate * dt, tm.max);
    const done = tm.value >= tm.max;
    if (done) tm.playing = false;
    updateFoot();
    if (done || t - lastPlaySent >= 1000 / 30) { lastPlaySent = t; sendTime('play'); }      // at most 30 a second, and always the last
  }
  function stopPlayback(update) {
    if (host.time) host.time.playing = false;
    if (update) updateFoot();
  }
  function setByReader(v) {
    const tm = host.time;
    if (!tm) return;
    const was = tm.playing;
    tm.playing = false;                      // a hand on the date takes it from the clock
    const value = clamp(v, tm.min, tm.max), changed = value !== tm.value || was;
    tm.value = value;
    updateFoot();
    if (changed) sendTime('reader');
  }
  function stepTime(dir) {                   // GlobeTime.step: the next tick, or a hundredth of the span
    const tm = host.time, ats = tm.ticks.map((x) => x.at);
    const ahead = dir > 0 ? Math.min(...ats.filter((a) => a > tm.value + 1e-6)) : Math.max(...ats.filter((a) => a < tm.value - 1e-6));
    return Number.isFinite(ahead) ? ahead : tm.value + dir * Math.max(1, (tm.max - tm.min) / 100);
  }
  let scrubbing = false;
  const scrub = (e) => { const r = strip.getBoundingClientRect(), tm = host.time; if (tm) setByReader(tm.min + clamp((e.clientX - r.left) / Math.max(r.width, 1), 0, 1) * (tm.max - tm.min)); };
  strip.addEventListener('pointerdown', (e) => {
    if (!host.time || (e.pointerType === 'mouse' && e.button !== 0)) return;
    scrubbing = true;
    strip.setPointerCapture(e.pointerId);
    strip.focus({ preventScroll: true });
    scrub(e);
    e.preventDefault();
  });
  strip.addEventListener('pointermove', (e) => { if (scrubbing) scrub(e); });
  strip.addEventListener('pointerup', () => { scrubbing = false; });
  strip.addEventListener('pointercancel', () => { scrubbing = false; });
  strip.addEventListener('keydown', (e) => {
    const tm = host.time;
    if (!tm) return;
    if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') setByReader(stepTime(-1));
    else if (e.key === 'ArrowRight' || e.key === 'ArrowUp') setByReader(stepTime(1));
    else if (e.key === 'Home') setByReader(tm.min);
    else if (e.key === 'End') setByReader(tm.max);
    else return;
    e.preventDefault();
  });
  playBtn.addEventListener('click', () => {
    const tm = host.time;
    if (!tm || settings.reduceMotion) return;
    if (tm.playing) tm.playing = false;
    else { if (tm.value >= tm.max) tm.value = tm.min; tm.playing = true; lastPlaySent = -1e9; }
    updateFoot();
    sendTime('reader');
  });
  function onTime(msg) {
    switch (msg.op) {
      case 'show': return showTime(msg.options);
      case 'hide': host.time = null; return updateFoot();
      case 'set':
        if (!host.time) return hostWarn('terra.time.set before terra.time.show; ignored');
        if (!Number.isFinite(msg.value)) return hostWarn('terra.time.set: the value must be a number; ignored');
        host.time.value = clamp(msg.value, host.time.min, host.time.max);       // not echoed: the script knows
        return updateFoot();
      case 'play':
        if (!host.time) return hostWarn('terra.time.play before terra.time.show; ignored');
        if (msg.playing && host.time.playable && !settings.reduceMotion) {
          if (host.time.value >= host.time.max) host.time.value = host.time.min;
          host.time.playing = true; lastPlaySent = -1e9;
        } else host.time.playing = false;
        return updateFoot();
      default: hostWarn('terra.time: unknown op "' + msg.op + '"; ignored');
    }
  }

  // ---- the panel: longer words and their sources, a sheet over the live globe
  const panel = $('panel'), panelBody = $('panel-body');
  function paragraphs(parent, text) {
    for (const para of String(text || '').split(/\n\s*\n/)) {
      const t = para.replace(/\s+/g, ' ').trim();
      if (!t) continue;
      const p = document.createElement('p');
      p.textContent = t;
      parent.appendChild(p);
    }
  }
  function showPanel(content) {
    const o = content && typeof content === 'object' ? content : {};
    panelBody.textContent = '';
    const h = document.createElement('h2');
    h.id = 'panel-title';
    h.textContent = typeof o.title === 'string' ? o.title : '';
    panelBody.appendChild(h);
    if (!o.title) hostWarn('terra.panel.show: give it a title (what this is)');
    paragraphs(panelBody, o.text);
    const sections = Array.isArray(o.sections) ? o.sections : [];
    for (const s of sections) {
      if (!s || typeof s !== 'object') continue;
      if (s.title) { const h3 = document.createElement('h3'); h3.textContent = String(s.title); panelBody.appendChild(h3); }
      paragraphs(panelBody, s.text);
    }
    const sources = Array.isArray(o.sources) ? o.sources : [];
    if (sources.length) {
      const h3 = document.createElement('h3'), ul = document.createElement('ul');
      h3.textContent = 'Sources';
      for (const s of sources) {
        if (!s || typeof s !== 'object') continue;
        const li = document.createElement('li');
        const url = typeof s.url === 'string' && /^https?:\/\//i.test(s.url) ? s.url : null;
        if (s.url && !url) hostWarn('terra.panel.show: a source\'s url must be http(s) (got ' + JSON.stringify(s.url) + ')');
        const a = document.createElement(url ? 'a' : 'span');
        a.textContent = String(s.title || s.url || 'Source');
        if (url) { a.href = url; a.target = '_blank'; a.rel = 'noopener noreferrer'; }
        li.appendChild(a);
        if (s.note) { const n = document.createElement('div'); n.className = 'note'; n.textContent = String(s.note); li.appendChild(n); }
        ul.appendChild(li);
      }
      panelBody.appendChild(h3);
      panelBody.appendChild(ul);
    }
    host.panel = { title: h.textContent, sections: sections.length, sources: sources.length };
    panel.hidden = false;
    panel.classList.remove('tall');
    panelBody.scrollTop = 0;
    dirty();
  }
  function hidePanel(byReader) {
    if (panel.hidden) return;
    panel.hidden = true;
    host.panel = null;
    dirty();
    if (byReader) deliver({ type: 'panel', open: false });
  }
  $('panel-close').addEventListener('click', () => hidePanel(true));
  $('panel-grab').addEventListener('click', () => { panel.classList.toggle('tall'); $('panel-grab').setAttribute('aria-label', panel.classList.contains('tall') ? 'Pull down' : 'Pull up'); });

  // ---- the pack's script context (worker.js) and the host's side of the messages
  let worker = null, closing = null, timers = new Map();
  api.delivered = { settings: 0, camera: 0, touch: 0, time: 0, timer: 0, open: 0, panel: 0, close: 0 };
  // (what the host says while the scripts are still loading, as the date's first value, waits for them to finish)
  let early = [];
  function deliver(message) {
    if (!worker) return;
    if (!api.started) { early.push(message); return; }
    api.delivered[message.type] = (api.delivered[message.type] || 0) + 1;
    worker.postMessage({ kind: 'receive', message, camera: camera() });
    postedCam = camVersion;
  }
  let postedCam = -1;
  function postCamera() {
    if (worker && postedCam !== camVersion) { postedCam = camVersion; worker.postMessage({ kind: 'camera', camera: camera() }); }
  }

  function handle(msg) {
    if (!msg || typeof msg !== 'object') return;
    switch (msg.type) {
      case 'log': return log('script', msg.level === 'error' ? 'error' : msg.level === 'warn' ? 'warn' : 'info', msg.message);
      case 'close': return endPack('script');
      case 'flyTo': return flyTo(msg.lon, msg.lat, msg.zoom, msg.duration);
      case 'geo': return onGeo(msg);
      case 'image': return onImage(msg);
      case 'sprite': return onSprite(msg);
      case 'touch':
        if (msg.op !== 'scheme') return hostWarn('terra.touch: unknown op "' + msg.op + '"');
        if (msg.scheme !== 'touchables' && msg.scheme !== 'one-finger') return hostWarn('terra.touch.scheme: "touchables" or "one-finger" (got ' + JSON.stringify(msg.scheme) + '); unchanged');
        host.scheme = msg.scheme;
        return updateReadout();
      case 'time': return onTime(msg);
      case 'caption':
        host.caption = { title: String(msg.title || ''), text: String(msg.text || '') };
        updateFoot();
        dirty();
        return;
      case 'globe':
        host.globe = { historical: !!msg.historical, still: !!msg.still };
        // (a pack may let the reader zoom deeper than 8, to 32, for its close views: as the app does)
        if (msg.maxZoom != null && Number.isFinite(+msg.maxZoom)) zoomLimit = clamp(+msg.maxZoom, 8, 32);
        dirty();
        return;
      case 'panel':
        if (msg.op === 'show') return showPanel(msg.content);
        if (msg.op === 'hide') return hidePanel(false);
        return hostWarn('terra.panel: unknown op "' + msg.op + '"');
      case 'listen':
        if (msg.event === 'open') { host.listensOpen = true; updateChrome(); }
        return;
      case 'timer': {
        // terra.js's own timers, where the context has none (?timers=host, as in JavaScriptCore)
        const id = msg.id;
        if (msg.op === 'clear') { const t = timers.get(id); if (t) { (t.repeat ? clearInterval : cancel)(t.handle); timers.delete(id); } return; }
        if (msg.op !== 'set') return;
        const fire = () => { if (!msg.repeat) timers.delete(id); deliver({ type: 'timer', id }); };
        timers.set(id, { repeat: !!msg.repeat, handle: msg.repeat ? setInterval(fire, Math.max(1, msg.ms)) : later(fire, msg.ms) });
        return;
      }
      default: hostWarn('a message the host does not know: ' + JSON.stringify(msg).slice(0, 200));
    }
  }

  function resetHost() {
    host.geo.clear(); host.images.clear(); host.sprites.clear();
    host.time = null;
    host.caption = { title: '', text: '' };
    host.globe = { historical: false, still: false };
    zoomLimit = 8;
    host.scheme = 'touchables';
    host.listensOpen = false;
    imagesVersion++;
    hidePanel(false);
    for (const t of timers.values()) (t.repeat ? clearInterval : cancel)(t.handle);
    timers.clear();
    updateFoot();
    dirty();
  }

  function startPack() {
    if (api.running) return;
    const where = 'prototype.json';
    if (M.runtime !== 3) { hostError(where + ': runtime is ' + JSON.stringify(M.runtime) + '; this preview runs runtime-3 packs'); return; }
    if (M.presentation !== 'globe') hostWarn(where + ': presentation is always "globe" (got ' + JSON.stringify(M.presentation) + ')');
    const scripts = Array.isArray(M.scripts) ? M.scripts : [];
    if (!scripts.length) hostError(where + ': scripts must list the pack\'s scripts, in order');
    const urls = [];
    for (const s of scripts) {
      const p = cleanPath(s);
      if (!p || !FILES.has(p)) hostError(where + ': scripts lists "' + s + '", which is not a file of the pack');
      else urls.push(packUrl(p));
    }
    resetHost();
    api.running = true; api.started = false; api.ended = false;
    early = [];
    log('harness', 'info', 'starting ' + ID + ': ' + urls.length + ' script' + (urls.length === 1 ? '' : 's') + (Q.timers === 'host' ? ' (timers through the host)' : ''));
    const a = M.anchor;
    if (a && validPos([a.lon, a.lat])) flyTo(a.lon, a.lat, a.zoom == null ? cam.zoom : a.zoom, 1.0);
    else if (a) hostWarn(where + ': anchor needs lon and lat (and zoom 0.8–8)');
    const w = worker = new Worker('/_harness/worker.js');
    w.onmessage = (e) => {
      const m = e.data || {};
      if (w !== worker) {                       // ending: only what the close listeners say is heard
        if (w === closing && m.kind === 'send' && m.message && m.message.type === 'log') handle(m.message);
        return;
      }
      if (m.kind === 'send') handle(m.message);
      else if (m.kind === 'loaded') {
        api.started = true;
        log('harness', 'info', 'started: ' + m.ok + ' of ' + m.scripts + ' script' + (m.scripts === 1 ? '' : 's') + ' ran without an error');
        deliver({ type: 'settings', settings: Object.assign({}, settings) });
        const waiting = early;
        early = [];
        waiting.forEach(deliver);
        updateChrome();
      }
    };
    w.onerror = (e) => { e.preventDefault(); log('script', 'error', e.message || 'the script context failed'); };
    w.postMessage({ kind: 'boot', runtime: P.runtime, scripts: urls, files: [...FILES], base: '/pack/' + ID + '/',
                    timers: Q.timers === 'host' ? 'host' : 'native',
                    boot: { pack: { id: ID, revision: 0, origin: 'preview' }, settings: Object.assign({}, settings), camera: camera() } });
    updateChrome();
  }
  function endPack(by) {
    if (!api.running) return;
    deliver({ type: 'close' });
    const w = worker;
    closing = w;
    worker = null;
    early = [];
    later(() => { w.terminate(); if (closing === w) closing = null; }, 150);
    api.running = false; api.started = false; api.ended = true;
    cancel(camTimer); cancel(camStopTimer); camTimer = camStopTimer = 0;
    resetHost();
    log('harness', 'info', 'ended' + (by === 'script' ? ' by terra.close()' : '') + '; the globe is as it was');
    updateChrome();
  }
  function pressOpen() {
    if (api.started && host.listensOpen) deliver({ type: 'open', params: {} });
  }

  // ---- the chip, the controls, the readout
  function updateChrome() {
    $('chip').hidden = !api.running;
    $('chip-open').hidden = !host.listensOpen;
    $('start').hidden = api.running;
    $('start-button').textContent = api.ended ? 'Start again' : 'Start';
    $('start-button').setAttribute('aria-label', (api.ended ? 'Start again: ' : 'Start: ') + (M.title || ID));
    $('ctl-start').disabled = api.running;
    $('ctl-end').disabled = !api.running;
    dirty();
    updateReadout();
  }
  let readoutAt = 0;
  function updateReadout() {
    if (BARE) return;
    const t = now();
    if (t - readoutAt < 150) return;
    readoutAt = t;
    $('readout').textContent = 'lon ' + cam.lon.toFixed(2) + ', lat ' + cam.lat.toFixed(2) + ', zoom ' + cam.zoom.toFixed(2) + ' · reading ' + rs.toFixed(2);
    $('state').textContent = !api.running ? (doorway ? 'Not running: its doorway is on the globe.' : 'Not running.')
      : [host.geo.size + ' geo layer' + (host.geo.size === 1 ? '' : 's') + ', ' + host.images.size + ' picture' + (host.images.size === 1 ? '' : 's') + ', ' + host.sprites.size + ' sprite' + (host.sprites.size === 1 ? '' : 's'),
         'touch: ' + host.scheme + (M.taps ? ', taps' : '') + (host.globe.still ? ' · still' : '') + (host.globe.historical ? ' · historical' : '')].join('\n');
  }
  function applySettings(patch) {
    Object.assign(settings, patch || {});
    settings.textScale = clamp(+settings.textScale || 1, 1, 3.2);
    $('scale').value = settings.textScale;
    $('scale-out').textContent = settings.textScale.toFixed(1);
    $('motion').checked = !!settings.reduceMotion;
    document.documentElement.style.setProperty('--ts', String(settings.textScale));
    if (settings.reduceMotion && host.time && host.time.playing) { host.time.playing = false; sendTime('host'); }
    updateFoot();
    dirty();
    if (api.started) deliver({ type: 'settings', settings: Object.assign({}, settings) });
  }
  $('scale').addEventListener('input', (e) => applySettings({ textScale: +e.target.value }));
  $('motion').addEventListener('change', (e) => applySettings({ reduceMotion: e.target.checked }));
  document.addEventListener('visibilitychange', () => applySettings({ active: !document.hidden }));
  $('start-button').addEventListener('click', startPack);
  $('ctl-start').addEventListener('click', startPack);
  $('ctl-end').addEventListener('click', () => endPack('reader'));
  $('ctl-restart').addEventListener('click', () => { endPack('reader'); later(startPack, 200); });
  $('chip-close').addEventListener('click', () => endPack('reader'));
  $('chip-open').addEventListener('click', pressOpen);

  // ---- the hands: drags turn the globe or are the pack's, wheel and pinch zoom, clicks are taps
  const pointers = new Map();
  let gesture = null;
  const local = (e) => { const r = stage.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
  function touchOut(phase, at, target) {
    const ll = unproject(at[0], at[1], camera());
    const t = { type: 'touch', phase, target: target || null, x: +at[0].toFixed(2), y: +at[1].toFixed(2), lon: ll ? ll.lon : null, lat: ll ? ll.lat : null };
    api.touches.push({ phase, target: t.target, x: t.x, y: t.y, lon: t.lon, lat: t.lat });
    if (api.touches.length > 500) api.touches.shift();
    deliver(t);
  }
  function tapAt(x, y, hit) {
    if (hit) return touchOut('tap', [x, y], hit);
    if (M.taps === true) return touchOut('tap', [x, y], null);
    log('harness', 'info', 'a tap on the globe: in the app it chooses a country ("taps" is not true and nothing touchable is here)');
  }
  stage.addEventListener('contextmenu', (e) => e.preventDefault());
  stage.addEventListener('pointerdown', (e) => {
    if (e.pointerType === 'mouse' && e.button !== 0 && e.button !== 2) return;
    stage.setPointerCapture(e.pointerId);
    stage.focus({ preventScroll: true });
    const at = local(e);
    pointers.set(e.pointerId, at);
    lastTouch = now();
    flight = null;
    if (pointers.size === 2) {
      if (gesture && gesture.kind === 'pack' && gesture.began) touchOut('ended', gesture.last, gesture.target);
      gesture = { kind: 'pinch' };
      return;
    }
    if (pointers.size > 2) return;
    const turnKey = e.shiftKey || e.altKey || e.button === 2;
    const hit = api.started ? hitTest(at[0], at[1]) : null;
    const door = !api.running && doorway && doorway.drawn && within(at[0], at[1], grow(doorway.drawn.rect, Math.max(0, (44 - doorway.drawn.rect[2]) / 2)));
    const pack = api.started && !turnKey && (host.scheme === 'one-finger' || !!hit);
    gesture = { kind: pack ? 'pack' : 'globe', target: hit, door, x0: at[0], y0: at[1], last: at, start: now(), moved: false, began: false };
    stage.classList.add('dragging');
  });
  stage.addEventListener('pointermove', (e) => {
    const at = local(e);
    if (!pointers.has(e.pointerId)) { hover(at); return; }
    const prev = pointers.get(e.pointerId);
    pointers.set(e.pointerId, at);
    lastTouch = now();
    if (!gesture) return;
    if (gesture.kind === 'pinch') {
      if (pointers.size !== 2) return;
      const [a, b] = [...pointers.values()], other = [...pointers.entries()].find(([id]) => id !== e.pointerId)[1];
      const before = Math.hypot(prev[0] - other[0], prev[1] - other[1]), after = Math.hypot(a[0] - b[0], a[1] - b[1]);
      if (before > 0) zoomBy(after / before);
      return;
    }
    const far = Math.hypot(at[0] - gesture.x0, at[1] - gesture.y0) > 6;
    if (gesture.kind === 'pack') {
      if (!gesture.began && far) { gesture.began = true; touchOut('began', [gesture.x0, gesture.y0], gesture.target); }
      if (gesture.began) touchOut('moved', at, gesture.target);
      gesture.last = at;
      return;
    }
    if (far) gesture.moved = true;
    if (gesture.moved) turn(at[0] - prev[0], at[1] - prev[1]);
  });
  const release = (e) => {
    if (!pointers.has(e.pointerId)) return;
    const at = pointers.get(e.pointerId);
    pointers.delete(e.pointerId);
    if (!pointers.size) stage.classList.remove('dragging');
    const gs = gesture;
    if (!gs) return;
    if (gs.kind === 'pinch') { if (!pointers.size) gesture = null; return; }
    gesture = null;
    if (e.type === 'pointercancel') { if (gs.kind === 'pack' && gs.began) touchOut('ended', gs.last, gs.target); return; }
    const tap = !gs.moved && !gs.began && now() - gs.start < 700;
    if (gs.kind === 'pack') {
      if (gs.began) touchOut('ended', at, gs.target);
      else if (tap) tapAt(at[0], at[1], gs.target);
    } else if (tap) {
      if (!api.running) { if (gs.door) startPack(); }
      else if (api.started) tapAt(at[0], at[1], gs.target);
    }
  };
  stage.addEventListener('pointerup', release);
  stage.addEventListener('pointercancel', release);
  stage.addEventListener('wheel', (e) => {
    e.preventDefault();
    lastTouch = now();
    flight = null;
    zoomBy(Math.exp(-e.deltaY * (e.ctrlKey ? 0.01 : 0.0015)));
  }, { passive: false });
  stage.addEventListener('keydown', (e) => {
    const step = 10 / cam.zoom, keys = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] };
    lastTouch = now();
    if (keys[e.key]) { flight = null; cam.lon = wrapLon(cam.lon + keys[e.key][0]); cam.lat = clamp(cam.lat + keys[e.key][1], -80, 80); cameraMoved(); }
    else if (e.key === '+' || e.key === '=') zoomBy(1.25);
    else if (e.key === '-') zoomBy(1 / 1.25);
    else if ((e.key === 'Enter' || e.key === ' ') && !api.running) startPack();
    else return;
    e.preventDefault();
  });
  let hoverAt = 0;
  function hover(at) {
    const t = now();
    if (t - hoverAt < 60) return;
    hoverAt = t;
    const hit = api.started ? hitTest(at[0], at[1]) : null;
    const door = !api.running && doorway && doorway.drawn && within(at[0], at[1], grow(doorway.drawn.rect, 8));
    stage.classList.toggle('pointing', !!(hit || door));
    stage.classList.toggle('reach', !hit && !door && api.started && host.scheme === 'one-finger');
  }

  // ---- the frame loop: the flight, the idle turn, the date playing, sprites moving; drawn when something changed
  let lastFrame = now();
  function frame(t) {
    requestAnimationFrame(frame);
    const dt = clamp((t - lastFrame) / 1000, 0, 0.1);
    lastFrame = t;
    if (flight) stepFlight(t);
    else if (!host.globe.still && !settings.reduceMotion && !pointers.size && t - lastTouch > idleAfter * 1000) {
      // left alone, it turns as the Earth does, west to east (GlobeView: 4 degrees a second)
      cam.lon = wrapLon(cam.lon - 4 * dt);
      cameraMoved();
    }
    stepPlayback(t, dt);
    postCamera();
    // (the last drawing was a quick one while the camera moved: drawn again, finely, once it is at rest)
    const sharpen = cameraMoving && !(flight || pointers.size || t - lastCamMove < 180);
    if (needsRender || sharpen || spritesAnimate() || (doorway && !api.running && (doorway.look.pics.length > 1 || doorway.look.every))) {
      needsRender = false;
      render(t);
      updateReadout();
    }
  }

  // ---- checks' handles
  Object.assign(api, {
    camera,
    flyTo: (lon, lat, zoom, seconds) => { lastTouch = now(); flyTo(lon, lat, zoom, seconds == null ? 0 : seconds); },
    flying: () => !!flight,
    redraw: () => { dirty(); },
    clock: () => (now() - t0) / 1000,          // the log's clock
    lastMove: () => (lastCamMove - t0) / 1000,
    start: startPack,
    end: () => endPack('reader'),
    open: pressOpen,
    setSettings: applySettings,
    reading: () => rs,
    touchables,
    hitTest,
    project: (lon, lat) => { const c = camera(), v = unit(lon, lat), o = projectUnit(v[0], v[1], v[2], rotation(c), c, {}); return { x: o.x, y: o.y, facing: o.z }; },
    unproject: (x, y) => unproject(x, y, camera()),
    settled: () => api.ready && !flight && pendingPictures === 0 && (!api.running || api.started),
    doorway: () => (doorway ? { label: doorway.label, lon: doorway.lon, lat: doorway.lat, visible: !!doorway.drawn, x: doorway.drawn && doorway.drawn.x, y: doorway.drawn && doorway.drawn.y } : null),
    state: () => ({
      running: api.running, started: api.started, ended: api.ended, scheme: host.scheme, listensOpen: host.listensOpen,
      globe: Object.assign({}, host.globe), caption: Object.assign({}, host.caption),
      time: host.time && Object.assign({}, host.time, { ticks: host.time.ticks.length, label: timeLabel(host.time) }),
      panel: host.panel && Object.assign({}, host.panel), camera: camera(), reading: rs,
      geo: [...host.geo.values()].map((L) => ({ id: L.id, style: L.style, touchable: L.touchable, features: L.features, vertices: L.vertices, minZoom: L.minZoom, maxZoom: L.maxZoom, drawn: L.drawn.length })),
      images: [...host.images.values()].map((im) => ({ id: im.id, image: im.path, bounds: im.bounds, ok: im.pic.ok, minZoom: im.minZoom, maxZoom: im.maxZoom, alpha: imageAlpha(im, cam.zoom) })),
      sprites: [...host.sprites.values()].map((s) => ({ id: s.id, label: s.label, touchable: s.touchable, moving: !!s.path, looks: s.looks.map((l) => ({ minZoom: l.minZoom, maxZoom: l.maxZoom, frames: l.pics.length })),
        drawn: s.drawn && { x: s.drawn.x, y: s.drawn.y, look: s.drawn.look, flipped: s.flip } })),
    }),
  });

  // ---- off we go: the globe's data, type and fonts, then the doorway or the pack
  const loads = [
    fetch('/_harness/land-50m.json').then((r) => r.json()).then((topo) => {
      const arcs = decodeArcs(topo);
      for (const geom of topo.objects.land.geometries) {
        for (const poly of geom.type === 'Polygon' ? [geom.arcs] : geom.arcs) for (const r of poly) land.push(ring(arcsPoints(arcs, r), true));
      }
    }),
    fetch('/_harness/countries-50m.json').then((r) => r.json()).then((topo) => { politics = decodePolitics(topo); }),
    fetch('/_harness/type.json').then((r) => r.json()).then((t) => {
      const r = t && t.reading;
      if (r && Array.isArray(r.shortSide) && Array.isArray(r.factor) && r.shortSide[1] > r.shortSide[0]) { TYPE = t; measure(); applyCaptionType(); }
    }),
    Promise.all(['400 15px', 'italic 400 15px', '600 15px'].map((f) => document.fonts.load(f + ' "Libertinus Serif"'))),
  ];
  Promise.allSettled(loads).then((results) => {
    results.forEach((r, i) => { if (r.status === 'rejected') log('harness', 'warn', ['land', 'countries', 'type spec', 'fonts'][i] + ' not loaded: ' + (r.reason && r.reason.message)); });
    doorway = readDoorway();
    const autostart = Q.start === '1' || !doorway;
    if (doorway && !autostart) {
      // the atlas round the doorway, close enough for its look to show
      Object.assign(cam, { lon: wrapLon(doorway.lon), lat: clamp(doorway.lat, -80, 80), zoom: clamp(Math.max(1, doorway.look.minZoom + 0.25), 0.8, 8) });
      if (!zoomHolds(doorway.look, cam.zoom)) cam.zoom = clamp(doorway.look.maxZoom, 0.8, 8);
      cameraMoved();
      log('harness', 'info', 'the doorway "' + doorway.label + '" is on the globe: touch it, or Start');
    }
    api.ready = true;
    applySettings({});
    updateChrome();
    if (autostart) startPack();
    requestAnimationFrame(frame);
  });
})();

// The preview harness: a stand-in for the app's host, so a web prototype can be tried on a desktop.
// It plays the host's part of docs/playground.md: a globe with the camera semantics of terra.js
// (lon, lat, zoom; radius 0.42 x the shorter side x zoom), markers, an overlay told the camera and
// the taps, a screen with a Back button in its top-left 56 pt, and the reader's settings.
// The globe is only a calm picture (Natural Earth land, orthographic); the app draws its own.
(function () {
  'use strict';
  const P = window.__proto, M = P.manifest, ID = P.id;
  const SIZES = { phone: [390, 844], ipad: [1024, 1366] };
  const [W, H] = SIZES[P.device];
  const D = Math.PI / 180, TAU = 2 * Math.PI;
  const $ = (id) => document.getElementById(id);
  const t0 = performance.now();
  const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

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

  const api = window.__playground = {
    ready: { overlay: false, screen: false },
    log: [],
    settings,
    camera: () => camera(),
    openScreen: (params) => openScreen(params || {}),
    setSettings: (patch) => applySettings(patch),
    flyTo: (lon, lat, zoom, seconds) => flyTo(lon, lat, zoom, seconds),
    closeScreen: () => closeScreen(),
  };

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

  let flight = 0;
  function flyTo(lon, lat, zoom, seconds) {
    cancelAnimationFrame(flight);
    const from = { lon: cam.lon, lat: cam.lat, zoom: cam.zoom };
    const to = { lon: +lon, lat: clamp(+lat, -85, 85), zoom: zoom == null ? cam.zoom : clamp(+zoom, 0.8, 8) };
    let dlon = wrapLon(to.lon - from.lon);
    const ms = (seconds == null ? 0.8 : +seconds) * 1000;
    if (settings.reduceMotion || ms <= 0) {           // Reduce Motion: arrive, don't travel
      Object.assign(cam, { lon: wrapLon(to.lon), lat: to.lat, zoom: to.zoom });
      return cameraChanged();
    }
    const start = performance.now();
    (function step(now) {
      const k = clamp((now - start) / ms, 0, 1), e = k < .5 ? 2 * k * k : 1 - Math.pow(-2 * k + 2, 2) / 2;
      cam.lon = wrapLon(from.lon + dlon * e);
      cam.lat = from.lat + (to.lat - from.lat) * e;
      cam.zoom = from.zoom + (to.zoom - from.zoom) * e;
      cameraChanged();
      if (k < 1) flight = requestAnimationFrame(step);
    })(start);
  }

  // ---- the stand-in globe
  let rings = [];
  fetch('/_harness/land-50m.json').then((r) => r.json()).then((topo) => { rings = decode(topo); schedule(); })
    .catch((e) => log('harness', 'warn', 'land data not loaded: ' + e.message));

  // TopoJSON (quantized, delta-encoded arcs) to rings of unit vectors on the sphere
  function decode(topo) {
    const [sx, sy] = topo.transform.scale, [tx, ty] = topo.transform.translate;
    const arcs = topo.arcs.map((arc) => {
      let x = 0, y = 0;
      return arc.map((p) => { x += p[0]; y += p[1]; return [x * sx + tx, y * sy + ty]; });
    });
    const out = [];
    const ring = (indexes) => {
      const pts = [];
      for (const i of indexes) {
        const a = i < 0 ? arcs[~i].slice().reverse() : arcs[i];
        for (let k = pts.length ? 1 : 0; k < a.length; k++) pts.push(a[k]);
      }
      const last = pts[pts.length - 1], first = pts[0];
      if (last[0] === first[0] && last[1] === first[1]) pts.pop();
      const n = pts.length, w = new Float32Array(3 * n);
      pts.forEach((p, i) => {
        const lo = p[0] * D, la = p[1] * D;
        w[3 * i] = Math.cos(la) * Math.sin(lo); w[3 * i + 1] = Math.sin(la); w[3 * i + 2] = Math.cos(la) * Math.cos(lo);
      });
      return { n, w, vx: new Float32Array(n), vy: new Float32Array(n), vz: new Float32Array(n) };
    };
    for (const g of topo.objects.land.geometries) {
      for (const poly of g.type === 'Polygon' ? [g.arcs] : g.arcs) for (const r of poly) out.push(ring(r));
    }
    return out;
  }

  function limbArc(path, cx, cy, R, from, to) {       // along the horizon the short way round
    let d = to - from;
    while (d > Math.PI) d -= TAU;
    while (d < -Math.PI) d += TAU;
    path.arc(cx, cy, R, from, from + d, d < 0);
  }

  // One ring in view space (the same rotation as terra.project). Points behind the globe are pushed to
  // the horizon so the fill follows the limb; the coast is stroked only where it is really seen.
  function trace(r, m, cx, cy, R, fill, stroke) {
    const { n, w, vx, vy, vz } = r;
    let first = -1, seen = 0;
    for (let i = 0; i < n; i++) {
      const x = w[3 * i], y = w[3 * i + 1], z = w[3 * i + 2];
      vx[i] = m.ca * x - m.sa * z;
      vy[i] = m.sa * m.sb * x + m.cb * y + m.ca * m.sb * z;
      vz[i] = m.sa * m.cb * x - m.sb * y + m.ca * m.cb * z;
      if (vz[i] > 0) { seen++; if (first < 0) first = i; }
    }
    if (!seen) return;
    const X = (i) => cx + vx[i] * R, Y = (i) => cy - vy[i] * R;
    const cross = (i, j) => {                         // where edge i-j meets the horizon
      const t = vz[i] / (vz[i] - vz[j]);
      const px = vx[i] + t * (vx[j] - vx[i]), py = vy[i] + t * (vy[j] - vy[i]), len = Math.hypot(px, py) || 1;
      return [px / len, py / len];
    };
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
    const a = cam.lon * D, b = -cam.lat * D;
    const m = { ca: Math.cos(a), sa: Math.sin(a), cb: Math.cos(b), sb: Math.sin(b) };
    const fill = new Path2D(), stroke = new Path2D();
    for (const r of rings) trace(r, m, c.cx, c.cy, c.radius, fill, stroke);
    g.fillStyle = '#F6E1B9'; g.fill(fill, 'evenodd');
    g.lineWidth = 0.75; g.lineJoin = 'round'; g.strokeStyle = '#0978AB'; g.stroke(stroke);
    g.restore();
    g.beginPath(); g.arc(c.cx, c.cy, c.radius, 0, TAU); g.lineWidth = 1; g.strokeStyle = 'rgba(9, 120, 171, .55)'; g.stroke();
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
    ended = true;
    $('ended').hidden = false;
    $('again').focus();
    log('harness', 'info', 'prototype ended; the plain globe is as it was');
  }
  function start() {
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
    for (const f of [overlay, screen]) post(f, { type: 'settings', settings: Object.assign({}, settings) });
  }

  // messages from the pack's pages: terra.js posts { terra: message } to its parent
  window.addEventListener('message', (e) => {
    const msg = e.data && e.data.terra;
    if (!msg || typeof msg !== 'object') return;
    const from = overlay && e.source === overlay.contentWindow ? 'overlay' : screen && e.source === screen.contentWindow ? 'screen' : null;
    if (!from) return;
    switch (msg.type) {
      case 'ready':
        api.ready[from] = true;
        post(from === 'overlay' ? overlay : screen, { type: 'settings', settings: Object.assign({}, settings) });
        if (from === 'overlay') post(overlay, { type: 'camera', camera: camera() });
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

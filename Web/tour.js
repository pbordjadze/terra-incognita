// The tour: a film of about six minutes, scripted as beats on one clock. Camera beats fly
// between looks (lon, lat, zoom), along a great circle with a hop out and back in for long
// distances; style beats change an ink, a projection, a grain or the mountains through a
// transition between two whole frames; tween beats ease numbers (the inking-in, the medallion,
// the sun); captions come and go. Touch the map and the film waits: the reader has the map in
// hand until they tap Resume, and the camera flies back to where the film left off.
'use strict';
(() => {
const { clamp, lerp, slerpLL, smooth01, wrap180, inverse, forward, projParams, layoutFor } = TI;
const E = {
  inOut: t => t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2,
  out: t => 1 - Math.pow(1 - t, 3), sine: t => 0.5 - 0.5 * Math.cos(Math.PI * t), lin: t => t,
};
const phone = () => Math.min(innerWidth, innerHeight) < 600;
const GRAIN = { coarse: 3.4, fine: 2.5, finest: 1.9 };
const grainPx = name => GRAIN[name] * (phone() ? 1 : 1.25);
const angDist = (a, b) => {
  const r = Math.PI / 180, p1 = a[1] * r, p2 = b[1] * r;
  return Math.acos(clamp(Math.sin(p1) * Math.sin(p2) + Math.cos(p1) * Math.cos(p2) * Math.cos((b[0] - a[0]) * r), -1, 1)) / r;
};
const cells = () => { const g = grainPx('finest'), dpr = Math.min(devicePixelRatio || 1, 3), cw = Math.max(4, Math.round(g * dpr)) / dpr; return Math.round(innerWidth / cw) * Math.round(innerHeight / (cw * 2)); };

// ------------------------------------------------------------------ the script
// cam: fly to a look; set: change styles through a transition (kind: 0 dissolve, 1 wipe, 2 iris);
// tw: ease numbers; cap: a caption (eyebrow, title, text) for d seconds.
const CHAPTERS = [
  { t: 0, n: 'I', name: 'Terra Incognita' }, { t: 30, n: 'II', name: 'The Sea' }, { t: 96, n: 'III', name: 'Mountains' },
  { t: 138, n: 'IV', name: 'Density' }, { t: 168, n: 'V', name: 'Inks' }, { t: 206, n: 'VI', name: 'Hibernia' },
  { t: 266, n: 'VII', name: 'Projections' }, { t: 318, n: 'VIII', name: 'Hidden Things' }, { t: 344, n: 'IX', name: 'Yours' },
];
const END = 360;
const SCRIPT = [
  // I — the atlas inks itself in
  { t: 0, d: 0, cam: { lon: 20, lat: 0, z: 0.92, ov: 1 } },
  { t: 1.2, d: 9, tw: { reveal: [0, 1] }, ease: 'sine' },
  { t: 3.5, d: 7, cap: { eyebrow: 'A demo', title: 'Terra Incognita', text: 'An atlas drawn in letters.', big: true } },
  { t: 2, d: 26, cam: { lon: 20, lat: 0, z: 1.06, ov: 1 }, ease: 'sine' },
  { t: 12, d: 8, cap: { eyebrow: 'Every mark is a character', text: 'A coast is a row of - _ / \\ | chosen for the way the line runs through each cell. Woods are ♣, sand is stippled, the sea is ruled.' } },
  { t: 21, d: 8, cap: { eyebrow: 'The two hemispheres', text: 'The world as the mapmakers of the 1600s laid it out, split at 20° west, with the winds blowing in from the margins.' } },
  // II — the sea
  { t: 30, d: 7, cam: { lon: -40, lat: 27, z: 2.7, ov: 0 } },
  { t: 37, d: 8, cap: { eyebrow: 'Portolan charts', text: 'Compass roses, and the rhumb lines that sailors steered by, ruled across the open sea.' } },
  { t: 45, d: 7, cam: { lon: -30.2, lat: 38.6, z: 6.5, ov: 0 } },
  { t: 52, d: 10, cap: { eyebrow: 'A legend of the sea', title: 'The island that swam away', text: 'In the Voyage of Saint Brendan (by about 900) the monks light a fire on an island. The island is a whale.' } },
  { t: 62, d: 9, cam: { lon: -9.5, lat: 65.2, z: 5.2, ov: 0 } },
  { t: 71, d: 10, cap: { eyebrow: 'Carta Marina, 1539', title: 'The sea serpent off Norway', text: 'Olaus Magnus drew what sailors told him. He wrote that it came ashore near Bergen on summer nights.' } },
  { t: 82, d: 5, cam: { lon: -15.5, lat: 52.2, z: 2.3, ov: 0 } },
  { t: 87, d: 6, cam: { lon: -15.5, lat: 52.0, z: 6.5, ov: 0 }, ease: 'inOut' },
  { t: 86, d: 9, cap: { eyebrow: 'A phantom island', title: 'Hy-Brasil', text: 'On the charts from 1325 for five hundred years. Here it shows itself only from far away: come closer and it is gone.' } },
  // III — mountains, two ways
  { t: 96, d: 9, cam: { lon: 86, lat: 31, z: 4.6, ov: 0 } },
  { t: 104, d: 8, cap: { eyebrow: 'Mountains, the old way', text: 'Little hills on a lattice tied to the map, snow on the high ones, kept where the named ranges are.' } },
  { t: 113, d: 2.5, set: { relief: 1 }, kind: 0 },
  { t: 113, d: 22, tw: { sunAz: [0.6, 4.2] }, ease: 'lin' },
  { t: 115, d: 11, cap: { eyebrow: 'Or the other way', text: 'Shaded relief from the GEBCO elevation grid, in a ramp of characters, lit by a sun that moves.' } },
  { t: 116, d: 18, cam: { lon: 80, lat: 33, z: 6.5, ov: 0 }, ease: 'sine' },
  { t: 134, d: 2, set: { relief: 0 }, kind: 0 },
  // IV — density
  { t: 138, d: 5, cam: { lon: 16, lat: 39, z: 2.1, ov: 0 } },
  { t: 143, d: 1.6, set: { grain: 'coarse' }, kind: 0 },
  { t: 143, d: 6, cap: { eyebrow: 'Density', title: 'Coarse' } },
  { t: 150, d: 1.6, set: { grain: 'fine' }, kind: 0 },
  { t: 150, d: 6, cap: { eyebrow: 'Density', title: 'Fine', text: 'The demo’s default.' } },
  { t: 157, d: 1.6, set: { grain: 'finest' }, kind: 0 },
  { t: 157, d: 8, cap: { eyebrow: 'Density', title: 'Finest', text: () => `About ${cells().toLocaleString('en-US')} characters on this screen, every one chosen afresh each frame.` } },
  { t: 165, d: 1.6, set: { grain: 'fine' }, kind: 0 },
  // V — inks
  { t: 168, d: 5, cam: { lon: 14, lat: 26, z: 1.5, ov: 0 } },
  { t: 171, d: 2.4, set: { ink: 'vellum' }, kind: 1 },
  { t: 172, d: 9, cap: { eyebrow: 'Ink', title: 'Vellum', text: 'A woodcut printed on parchment: the same map, inked dark on light.' } },
  { t: 183, d: 2.6, set: { ink: 'illum' }, kind: 2 },
  { t: 184, d: 11, cap: { eyebrow: 'Ink', title: 'Illuminated', text: 'A manuscript page: gold leaf that catches the light, lapis seas, vermilion and verdigris, red dots along the coasts.' } },
  { t: 186, d: 18, cam: { lon: 8, lat: 42, z: 2.6, ov: 0 }, ease: 'sine' },
  // VI — Hibernia, after Kells
  { t: 202, d: 2.4, set: { ink: 'night' }, kind: 0 },
  { t: 204, d: 8, cam: { lon: -7.9, lat: 53.5, z: 12, ov: 0 } },
  { t: 212, d: 5, tw: { kells: [0, 1] }, ease: 'inOut' },
  { t: 213, d: 11, cap: { eyebrow: 'Hibernia', title: 'After the Book of Kells', text: 'Ireland in a medallion, in the pigments of a gospel book made around 800: verdigris, orpiment, red lead and gold.' } },
  { t: 218, d: 16, cam: { lon: -7.8, lat: 53.4, z: 24, ov: 0 }, ease: 'sine' },
  { t: 225, d: 10, cap: { eyebrow: 'Hibernia', text: 'Triskele spirals turn slowly over the land, round trees as the film draws them, a plait along the coast, red dots standing off it.' } },
  { t: 237, d: 7, cam: { lon: 6, lat: 50, z: 3.2, ov: 0 } },
  { t: 238, d: 2.8, set: { ink: 'kells' }, kind: 2 },
  { t: 238, d: 4, tw: { kells: [1, 0] }, ease: 'inOut' },
  { t: 241, d: 11, cap: { eyebrow: 'The other path', title: 'The whole world, after Kells', text: 'Every country laid flat in its own pigment, four colours so no two neighbours share one, spirals over all of it.' } },
  { t: 246, d: 16, cam: { lon: 20, lat: 10, z: 1.25, ov: 0.6 }, ease: 'sine' },
  // VII — projections
  { t: 262, d: 2.4, set: { ink: 'night' }, kind: 0 },
  { t: 266, d: 3, set: { proj: 1 }, kind: 0 },
  { t: 266, d: 3, cam: { lon: 10, lat: 0, z: 0.95, ov: 1 } },
  { t: 268, d: 10, cap: { eyebrow: 'Projections', title: 'The oval', text: 'One world in an oval, the shape Ortelius gave it in 1570.' } },
  { t: 281, d: 3, set: { proj: 2 }, kind: 2 },
  { t: 281, d: 0, cam: { lon: -30, lat: 22, z: 1, ov: 0 } },
  { t: 282, d: 4, tw: { sunW: [0, 1] }, ease: 'inOut' },
  { t: 284, d: 34, cam: { lon: 115, lat: 22, z: 1.15, ov: 0 }, ease: 'sine' },
  { t: 284, d: 34, tw: { sunLon: [40, -150] }, ease: 'lin' },
  { t: 285, d: 11, cap: { eyebrow: 'Projections', title: 'A globe', text: 'Turning, with the night crossing it and the lights of cities from NASA’s Black Marble.' } },
  { t: 300, d: 10, cap: { eyebrow: 'Projections', text: 'Each is drawn by inverting its projection at every character cell, so any of them can be zoomed, turned or tilted.' } },
  // VIII — hidden things
  { t: 318, d: 2.6, set: { proj: 0 }, kind: 0 },
  { t: 318, d: 1, tw: { sunW: [1, 0] } },
  { t: 318, d: 6, cam: { lon: -3, lat: 17.6, z: 5.5, ov: 0 } },
  { t: 323, d: 7, cap: { eyebrow: 'Hidden things', title: 'Mansa Musa', text: 'The Catalan Atlas (1375) puts a king in West Africa, holding up a nugget of gold.' } },
  { t: 330, d: 6, cam: { lon: 138.5, lat: 16, z: 4.2, ov: 0 } },
  { t: 335, d: 7, cap: { eyebrow: 'Hidden things', title: 'HC SVNT DRACONES', text: '“Here are dragons” is on just two old globes, both near the coast of East Asia.' } },
  // IX — yours
  { t: 344, d: 7, cam: { lon: 20, lat: 0, z: 1, ov: 1 } },
  { t: 346, d: 2.8, set: { ink: 'illum' }, kind: 2 },
  { t: 349, d: 11, cap: { eyebrow: 'Terra Incognita', title: 'Yours to wander', text: 'Pinch, drag, tap what glints. The styles are all in the menu. Nothing keeps score.', big: true } },
];

// ------------------------------------------------------------------ compiling it
function compile() {
  const cam = [], tw = {}, sets = {}, caps = [];
  let last = null;
  for (const b of SCRIPT.slice().sort((a, b) => a.t - b.t)) {
    if (b.cam) { cam.push({ t0: b.t, t1: b.t + b.d, to: b.cam, ease: E[b.ease || 'inOut'], from: last }); last = b.cam; }
    if (b.tw) for (const [k, [a, z]] of Object.entries(b.tw)) (tw[k] = tw[k] || []).push({ t0: b.t, t1: b.t + Math.max(b.d, 1e-3), a, z, ease: E[b.ease || 'inOut'] });
    if (b.set) for (const [k, v] of Object.entries(b.set)) (sets[k] = sets[k] || []).push({ t: b.t, d: b.d, v, kind: b.kind || 0 });
    if (b.cap) caps.push(Object.assign({ t0: b.t, t1: b.t + b.d }, b.cap));
  }
  for (const s of cam) if (!s.from) s.from = s.to;
  return { cam, tw, sets, caps };
}
const BASE = { proj: 0, ink: 'night', grain: 'fine', relief: 0, reveal: 0, kells: 0, sunW: 0, sunLon: 40, sunAz: 2.3, labels: 1, ornaments: 1 };

function lookAt(seg, T) {
  const u = clamp((T - seg.t0) / Math.max(seg.t1 - seg.t0, 1e-6)), e = seg.ease(u);
  const a = seg.from, b = seg.to;
  const ll = slerpLL([a.lon, a.lat], [b.lon, b.lat], e);
  const la = Math.log(a.z), lb = Math.log(b.z);
  const hop = Math.max(0, Math.min(la, lb) - 0.05) * smooth01(10, 55, angDist([a.lon, a.lat], [b.lon, b.lat]));
  return { lon: ll[0], lat: ll[1], z: Math.exp(lerp(la, lb, e) - hop * Math.sin(Math.PI * e)), ov: lerp(a.ov ?? 0, b.ov ?? 0, e) };
}
function lerpLook(a, b, e) {
  const ll = slerpLL([a.lon, a.lat], [b.lon, b.lat], e);
  const hop = Math.max(0, Math.min(Math.log(a.z), Math.log(b.z)) - 0.05) * smooth01(10, 55, angDist([a.lon, a.lat], [b.lon, b.lat]));
  return { lon: ll[0], lat: ll[1], z: Math.exp(lerp(Math.log(a.z), Math.log(b.z), e) - hop * Math.sin(Math.PI * e)), ov: lerp(a.ov ?? 0, b.ov ?? 0, e) };
}
// The sun right now: the point it stands over, as a unit vector (x to 0°E, y to 90°E, z north).
function sunVec(lonOverride) {
  const d = new Date(), doy = (Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate()) - Date.UTC(d.getUTCFullYear(), 0, 0)) / 864e5;
  const dec = -23.44 * Math.cos(2 * Math.PI / 365 * (doy + 10)) * Math.PI / 180;
  const lon = (lonOverride ?? -15 * (d.getUTCHours() + d.getUTCMinutes() / 60 - 12)) * Math.PI / 180;
  return [Math.cos(dec) * Math.cos(lon), Math.cos(dec) * Math.sin(lon), Math.sin(dec)];
}

class Tour {
  constructor(app) {
    this.app = app;
    this.tl = compile();
    this.T = 0; this.playing = true; this.mode = 'film';   // film | free | return
    this.lastNow = null;
    this.free = null;                 // the reader's state while the film waits
    this.ui = new UI(this);
    this.input = new Input(this);
    this.capShown = null;
    const m = /t(\d+(?:\.\d+)?)(p?)/.exec(location.hash);
    if (m) { this.T = Math.min(+m[1], END); this.playing = !m[2]; }
  }

  // the film's state at time T: one state, or two with a transition between them
  at(T) {
    const tl = this.tl, st = Object.assign({}, BASE);
    for (const [k, list] of Object.entries(tl.tw)) {
      let v = BASE[k];
      for (const s of list) { if (T >= s.t1) v = s.z; else if (T >= s.t0) { v = lerp(s.a, s.z, s.ease((T - s.t0) / (s.t1 - s.t0))); break; } else break; }
      st[k] = v;
    }
    let seg = null;
    for (const s of tl.cam) { if (s.t0 <= T) seg = s; else break; }
    st.look = seg ? lookAt(seg, T) : Object.assign({}, tl.cam[0].to);
    let trans = null;
    for (const [k, list] of Object.entries(tl.sets)) {
      let v = BASE[k], prev = BASE[k];
      for (const s of list) {
        if (s.t > T) break;
        prev = v; v = s.v;
        if (T < s.t + s.d && (!trans || s.t > trans.t)) trans = { k, prev, s };
        else if (trans && trans.k === k && T >= s.t + s.d) trans = null;
      }
      st[k] = v;
    }
    const a = this.toRender(st);
    if (!trans) return { a, b: null, mix: 0, kind: 0 };
    const before = Object.assign({}, st, { [trans.k]: trans.prev });
    return { a: this.toRender(before), b: a, mix: E.inOut((T - trans.s.t) / trans.s.d), kind: trans.s.kind };
  }
  // tour fields -> what the renderer reads
  toRender(st) {
    const r = Object.assign(TI.defaultState(), {
      proj: st.proj, ink: st.ink, grain: grainPx(st.grain), grainName: st.grain, relief: st.relief, kells: st.kells,
      sunAz: st.sunAz, labels: st.labels, ornaments: st.ornaments, look: Object.assign({}, st.look),
    });
    const W = this.app.W, H = this.app.H;
    r.reveal = [st.reveal, W / 2, H / 2, Math.hypot(W, H) * 0.62];
    const sv = sunVec(st.sunW > 0 ? st.sunLon : undefined);
    r.sun = [sv[0], sv[1], sv[2], st.sunW];
    r.title = 'TERRA INCOGNITA';
    return r;
  }

  frame(time, now) {
    const dt = this.lastNow == null ? 0 : Math.min(0.1, (now - this.lastNow) / 1000);
    this.lastNow = now;
    if (this.mode === 'film' && this.playing) {
      this.T += dt;
      if (this.T >= END) { this.T = END; this.enterFree(true); }
    }
    if (this.mode === 'film') {
      const f = this.at(this.T);
      this.ui.update(this.T, this.capAt(this.T));
      return f;
    }
    if (this.mode === 'return') {
      const u = clamp((now - this.ret.t0) / this.ret.d), e = E.inOut(u);
      const target = this.at(this.ret.T);
      const tgt = target.b && target.mix > 0.5 ? target.b : target.a;
      const st = Object.assign({}, tgt, { look: lerpLook(this.ret.from.look, tgt.look, e) });
      let out;
      if (this.ret.styleDiff) {
        const from = Object.assign({}, this.ret.from, { look: st.look });
        out = { a: from, b: st, mix: smooth01(0.15, 0.85, u), kind: 0 };
      } else out = { a: st, b: null, mix: 0, kind: 0 };
      if (u >= 1) { this.mode = 'film'; this.T = this.ret.T; this.playing = true; this.ui.setMode('film'); }
      this.ui.update(this.ret.T, null);
      return out;
    }
    // free: the reader's own state, with an optional style transition
    const fs = this.free;
    // zoomed right out, the sheet settles back into its frame
    const L = fs.st.look;
    if (fs.st.proj !== 2 && L.z < 1.25 && this.input.pointers.size === 0) {
      const r = 1 - Math.exp(-dt * 4);
      L.ov += (1 - L.ov) * r; L.dx = (L.dx || 0) * (1 - r); L.dy = (L.dy || 0) * (1 - r);
    }
    if (fs.trans) {
      const u = clamp((now - fs.trans.t0) / fs.trans.d);
      const a = Object.assign({}, fs.trans.from, { look: fs.st.look, reveal: fs.st.reveal });
      if (u >= 1) fs.trans = null;
      return { a, b: fs.st, mix: E.inOut(u), kind: fs.trans ? fs.trans.kind : 0 };
    }
    return { a: fs.st, b: null, mix: 0, kind: 0 };
  }
  capAt(T) { let c = null; for (const k of this.tl.caps) if (T >= k.t0 && T < k.t1) c = k; return c; }

  // the reader takes the map: the film waits where it is
  enterFree(atEnd) {
    if (this.mode === 'free') return;
    const f = this.mode === 'return' ? null : this.at(this.T);
    const cur = f ? (f.b && f.mix > 0.5 ? f.b : f.a) : this.at(this.ret.T).a;
    const st = Object.assign({}, cur, { look: Object.assign({}, cur.look), reveal: [1, 0, 0, 1] });
    this.free = { st, trans: null };
    this.mode = 'free'; this.playing = false;
    this.ui.setMode('free', atEnd);
  }
  resume() {
    if (this.mode !== 'free') return;
    if (this.T >= END) { this.restart(); return; }
    this.flyBackTo(this.T);
  }
  flyBackTo(T) {
    const from = this.free ? this.free.st : this.at(this.T).a;
    const tgtF = this.at(T), tgt = tgtF.b && tgtF.mix > 0.5 ? tgtF.b : tgtF.a;
    const styleDiff = ['ink', 'proj', 'grain', 'relief'].some(k => from[k] !== tgt[k]);
    const d = clamp(0.9 + angDist([from.look.lon, from.look.lat], [tgt.look.lon, tgt.look.lat]) / 120 + Math.abs(Math.log(from.look.z / tgt.look.z)) * 0.25, 0.9, 2.2);
    this.ret = { from, T, t0: performance.now(), d: d * 1000, styleDiff };
    this.mode = 'return'; this.free = null;
    this.ui.setMode('return');
  }
  jump(T) {
    if (this.mode === 'film') { this.enterFree(false); }
    this.T = T; this.flyBackTo(T);
  }
  restart() {
    this.T = 0; this.free = null; this.mode = 'film'; this.playing = true;
    this.ui.setMode('film');
  }
  // a style change while exploring
  setStyle(k, v) {
    const fs = this.free; if (!fs) return;
    const from = Object.assign({}, fs.st);
    if (k === 'grain') { fs.st.grain = grainPx(v); fs.st.grainName = v; }
    else if (k === 'kellsRoundel') { fs.st.kells = v ? 1 : 0; }
    else fs.st[k] = v;
    if (k === 'proj') fs.st.look.ov = fs.st.look.z < 1.3 ? 1 : fs.st.look.ov;
    if (k === 'sun') fs.st.sun = v ? (() => { const s = sunVec(); return [s[0], s[1], s[2], 1]; })() : [1, 0, 0, 0];
    fs.trans = { from, t0: performance.now(), d: k === 'kellsRoundel' || k === 'sun' ? 900 : 1400, kind: k === 'ink' ? 2 : 0 };
  }
}

// ------------------------------------------------------------------ the reader's fingers
class Input {
  constructor(tour) {
    this.tour = tour; this.app = tour.app;
    this.pointers = new Map(); this.g = null;
    const el = document.getElementById('stage');
    el.addEventListener('pointerdown', e => this.down(e));
    el.addEventListener('pointermove', e => this.move(e));
    el.addEventListener('pointerup', e => this.up(e));
    el.addEventListener('pointercancel', e => this.up(e));
    el.addEventListener('wheel', e => { e.preventDefault(); this.take(); this.zoomAt(Math.exp(-e.deltaY * (e.ctrlKey ? 0.01 : 0.002)), e.clientX, e.clientY); }, { passive: false });
  }
  take() { if (this.tour.mode !== 'free') this.tour.enterFree(false); }
  st() { return this.tour.free.st; }
  pp(st) { const lay = layoutFor(st.proj, this.app.W, this.app.H); return { pp: projParams(st, lay), lay }; }
  // the plane point under a screen point, for the current look
  plane(st, sx, sy) {
    const v = this.app.lastView;
    return [v.cam.x + (sx - this.app.W / 2) / v.scale, v.cam.y + (sy - this.app.H / 2) / v.scale];
  }
  down(e) {
    this.take();
    this.tour.ui.hideCard();
    e.currentTarget.setPointerCapture(e.pointerId);
    this.pointers.set(e.pointerId, { x: e.clientX, y: e.clientY, x0: e.clientX, y0: e.clientY, t0: performance.now() });
    this.begin();
  }
  begin() {
    const ps = [...this.pointers.values()], st = this.st();
    if (ps.length === 1) this.g = { kind: 'pan', moved: false, x: ps[0].x, y: ps[0].y };
    else if (ps.length >= 2) { const [a, b] = ps; this.g = { kind: 'pinch', d: Math.hypot(a.x - b.x, a.y - b.y), mx: (a.x + b.x) / 2, my: (a.y + b.y) / 2, moved: true }; }
  }
  move(e) {
    const p = this.pointers.get(e.pointerId); if (!p || !this.g) return;
    p.x = e.clientX; p.y = e.clientY;
    const ps = [...this.pointers.values()];
    if (this.g.kind === 'pan' && ps.length === 1) {
      const dx = p.x - this.g.x, dy = p.y - this.g.y;
      if (!this.g.moved && Math.hypot(p.x - p.x0, p.y - p.y0) < 6) return;
      this.g.moved = true; this.g.x = p.x; this.g.y = p.y;
      this.panBy(dx, dy);
    } else if (this.g.kind === 'pinch' && ps.length >= 2) {
      const [a, b] = ps, d = Math.hypot(a.x - b.x, a.y - b.y), mx = (a.x + b.x) / 2, my = (a.y + b.y) / 2;
      this.panBy(mx - this.g.mx, my - this.g.my);
      this.zoomAt(d / this.g.d, mx, my);
      Object.assign(this.g, { d, mx, my });
    }
  }
  up(e) {
    const p = this.pointers.get(e.pointerId);
    this.pointers.delete(e.pointerId);
    if (this.g && this.g.kind === 'pan' && !this.g.moved && p && performance.now() - p.t0 < 400) this.tap(p.x, p.y);
    this.g = null;
    if (this.pointers.size) this.begin();
  }
  tap(x, y) {
    const v = this.app.lastView, f = this.app.scene.findAt(v, x, y);
    if (f) this.tour.ui.showCard(f);
  }
  // move the look so the map follows the finger by (dx, dy) screen px
  panBy(dx, dy) {
    const st = this.st(), L = st.look, v = this.app.lastView;
    if (st.proj !== 2 && L.ov > 0.02) {
      // far out: slide the sheet; the overview framing gives way as soon as it is moved
      L.dx = (L.dx || 0) - dx / v.scale; L.dy = (L.dy || 0) - dy / v.scale;
      return;
    }
    const { pp } = this.pp(st);
    const c = [v.cam.x - dx / v.scale, v.cam.y - dy / v.scale];
    const ll = inverse(pp, c);
    if (ll) { L.lon = ll[0]; L.lat = clamp(ll[1], -84, 84); L.dx = 0; L.dy = 0; }
    else { L.dx = (L.dx || 0) - dx / v.scale; L.dy = (L.dy || 0) - dy / v.scale; }
  }
  zoomAt(f, sx, sy) {
    const st = this.st(), L = st.look, z0 = L.z;
    L.z = clamp(L.z * f, 0.85, 26);
    const k = L.z / z0;
    // keep the point under the fingers where it is
    this.panBy((sx - this.app.W / 2) * (1 - k), (sy - this.app.H / 2) * (1 - k));
    if (st.proj !== 2) {
      if (L.z > 1.35 && L.ov > 0) {
        // leaving the overview: hand the framing over to a look at the place in the middle
        const v = this.app.lastView, { pp } = this.pp(st);
        const ll = inverse(pp, [v.cam.x, v.cam.y]);
        if (ll) { L.lon = ll[0]; L.lat = ll[1]; L.ov = 0; L.dx = 0; L.dy = 0; }
      } else if (L.z < 1.15) { L.ov = Math.min(1, L.ov + 0.2); L.dx = (L.dx || 0) * 0.8; L.dy = (L.dy || 0) * 0.8; }
    }
  }
}

// ------------------------------------------------------------------ captions and controls
class UI {
  constructor(tour) {
    this.tour = tour;
    const $ = id => document.getElementById(id);
    this.cap = $('cap'); this.capEyebrow = $('capEyebrow'); this.capTitle = $('capTitle'); this.capText = $('capText');
    this.chap = $('chap'); this.bar = $('bar'); this.fill = $('barFill'); this.resumeBtn = $('resume'); this.menuBtn = $('menuBtn');
    this.panel = $('panel'); this.card = $('card');
    this.resumeBtn.addEventListener('click', () => this.tour.resume());
    this.menuBtn.addEventListener('click', () => this.togglePanel());
    $('cardClose').addEventListener('click', () => this.hideCard());
    $('panelClose').addEventListener('click', () => this.togglePanel(false));
    // chapter ticks
    for (const c of CHAPTERS) {
      const b = document.createElement('button');
      b.className = 'tick'; b.style.left = (c.t / END * 100) + '%'; b.type = 'button';
      b.setAttribute('aria-label', `Chapter ${c.n}: ${c.name}`);
      b.addEventListener('click', e => { e.stopPropagation(); this.tour.jump(c.t + 0.01); });
      this.bar.appendChild(b);
    }
    this.panel.addEventListener('click', e => {
      const b = e.target.closest('button[data-k]'); if (!b) return;
      const k = b.dataset.k, raw = b.dataset.v, v = /^\d+$/.test(raw) ? +raw : raw === 'on' ? true : raw === 'off' ? false : raw;
      this.panel.querySelectorAll(`button[data-k="${k}"]`).forEach(x => x.setAttribute('aria-pressed', String(x === b)));
      this.tour.setStyle(k, v);
    });
    this.lastChap = null;
  }
  syncPanel() {
    const st = this.tour.free && this.tour.free.st; if (!st) return;
    const cur = { ink: st.ink, proj: String(st.proj), relief: String(st.relief), grain: st.grainName, kellsRoundel: st.kells > 0.5 ? 'on' : 'off', sun: st.sun[3] > 0.5 ? 'on' : 'off' };
    this.panel.querySelectorAll('button[data-k]').forEach(b => b.setAttribute('aria-pressed', String(cur[b.dataset.k] === b.dataset.v)));
  }
  togglePanel(force) {
    const open = force ?? this.panel.hidden;
    if (open && this.tour.mode !== 'free') this.tour.enterFree(false);
    this.panel.hidden = !open;
    if (open) { this.syncPanel(); this.hideCard(); }
  }
  setMode(m, atEnd) {
    document.body.dataset.mode = m;
    this.resumeBtn.hidden = m !== 'free';
    this.resumeBtn.textContent = atEnd || this.tour.T >= END ? 'Play the tour again' : 'Resume the tour';
    if (m !== 'free') { this.panel.hidden = true; this.hideCard(); }
    if (m !== 'film') { this.capShown = null; this.cap.classList.remove('on'); document.body.classList.remove('capOn'); }
    if (m === 'free' && atEnd) this.togglePanel(true);
  }
  update(T, cap) {
    this.fill.style.transform = `scaleX(${T / END})`;
    let ch = CHAPTERS[0]; for (const c of CHAPTERS) if (T >= c.t) ch = c;
    if (ch !== this.lastChap) { this.chap.textContent = `${ch.n} · ${ch.name}`; this.lastChap = ch; }
    if (cap !== this.capShown) {
      this.capShown = cap;
      this.cap.classList.remove('on');
      document.body.classList.toggle('capOn', !!cap);
      clearTimeout(this.capTimer);
      if (cap) this.capTimer = setTimeout(() => {
        this.capEyebrow.textContent = cap.eyebrow || '';
        this.capTitle.textContent = cap.title || ''; this.capTitle.hidden = !cap.title;
        const text = typeof cap.text === 'function' ? cap.text() : cap.text;
        this.capText.textContent = text || ''; this.capText.hidden = !text;
        this.cap.classList.toggle('big', !!cap.big);
        this.cap.classList.add('on');
      }, 380);
    }
  }
  showCard(f) {
    const $ = id => document.getElementById(id);
    $('cardEyebrow').textContent = f.eyebrow; $('cardTitle').textContent = f.title;
    const body = $('cardBody'); body.replaceChildren();
    for (const para of f.text) { const p = document.createElement('p'); p.innerHTML = para; body.append(p); }
    $('cardSource').textContent = f.source; $('cardLink').href = f.link;
    this.panel.hidden = true;
    this.card.hidden = false; this.card.classList.remove('rise'); void this.card.offsetWidth; this.card.classList.add('rise');
  }
  hideCard() { this.card.hidden = true; }
}

TI.Tour = Tour;
TI.END = END;
})();

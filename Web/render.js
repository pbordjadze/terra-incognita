// Terra Incognita's renderer. Two WebGL2 passes: cell.glsl decides every character cell (one
// fragment per cell, into two small textures: colour and strength, ground and character), then
// the glyph pass paints each cell's character from an atlas measured from the font. A third pass
// blends two whole frames for transitions (grain, ink or projection changing).
'use strict';

const GLYPHS = [];
for (let c = 32; c < 127; c++) GLYPHS.push(String.fromCharCode(c));
const EXTRA = ['·', '♣', '♠', '≈', '¯', '•', '°', '†', '✦'];
GLYPHS.push(...EXTRA);
const GI = Object.fromEntries(GLYPHS.map((g, i) => [g, i]));
const NAMED = { SP: ' ', DOT: '.', COMMA: ',', QUOTE: "'", BTICK: '`', COLON: ':', SEMI: ';', DASH: '-', US: '_', MACRON: '¯', PIPE: '|',
  SLASH: '/', BSLASH: '\\', TILDE: '~', APPROX: '≈', PLUS: '+', STAR: '*', MIDDOT: '·', BULLET: '•', CLUB: '♣', SPADE: '♠', O: 'o' };
const RAMP_CAND = " .`:-=+*#%@";

const PAL_KEYS = ['SKY', 'STAR', 'SEABG', 'SEA', 'SEAHI', 'COAST', 'GLINT', 'LANDBG', 'VELLUM', 'VELLUMHI', 'FOREST', 'FORESTHI', 'GRASS', 'GRASSHI',
  'DESERT', 'DESERTHI', 'ICE', 'ICEHI', 'MOUNT', 'MOUNTHI', 'SNOW', 'RIVER', 'BORDER', 'GOLD', 'GOLDHI', 'VERM', 'VERMHI', 'VERD', 'VERDHI', 'LAPIS',
  'HULL', 'SAIL', 'WHALE', 'WHALEHI', 'SPRAY', 'FIRE', 'GHOST', 'MOON', 'CLOUD', 'CLOUDHI', 'RHUMB0', 'RHUMB1', 'RHUMB2', 'WASH0', 'WASH1', 'WASH2', 'WASH3',
  'LIGHTS', 'INK', 'PURPLE', 'ORPIMENT', 'KVELLUM'];
const hex = h => [parseInt(h.slice(1, 3), 16) / 255, parseInt(h.slice(3, 5), 16) / 255, parseInt(h.slice(5, 7), 16) / 255];
// The inks. Night is scratchboard (light lines on a dark ground); vellum is a woodcut on parchment;
// illuminated is a manuscript page: vellum, gold leaf, lapis, vermilion and verdigris, ink outlines.
const INKS = {
  night: { dark: 1, illum: 0, gain: 1, label: { continent: '#e8c35f', ocean: '#97b8ff', sea: '#86a6ea', country: '#efe3c4', capital: '#cbbf9f', line: '#c99a52', find: '#ff8a6e', title: '#f3d27a', halo: 'rgba(6,8,13,0.72)' },
    c: { SKY: '#06080d', STAR: '#d6deff', SEABG: '#050a18', SEA: '#345ab2', SEAHI: '#96bcff', COAST: '#608ce2', GLINT: '#ffecaa', LANDBG: '#22190e',
      VELLUM: '#b8a274', VELLUMHI: '#fae8ba', FOREST: '#3ea05c', FORESTHI: '#96ea8c', GRASS: '#769e42', GRASSHI: '#c4de78', DESERT: '#c4944a', DESERTHI: '#f8d68c',
      ICE: '#c4d8ec', ICEHI: '#f0f8ff', MOUNT: '#ba6c3e', MOUNTHI: '#eca068', SNOW: '#f6f2e6', RIVER: '#5c8ef0', BORDER: '#e25840', GOLD: '#d6b050', GOLDHI: '#ffe896',
      VERM: '#de523c', VERMHI: '#ff8c6e', VERD: '#3eac84', VERDHI: '#96ecc4', LAPIS: '#547ade', HULL: '#96603a', SAIL: '#ecdeba', WHALE: '#607696', WHALEHI: '#b0c4de',
      SPRAY: '#d6e8ff', FIRE: '#ffa03c', GHOST: '#aac4ff', MOON: '#f4e2a0', CLOUD: '#7680a8', CLOUDHI: '#bec6e4', RHUMB0: '#968c70', RHUMB1: '#40966c', RHUMB2: '#b2483a',
      WASH0: '#e66e80', WASH1: '#ecba46', WASH2: '#80c868', WASH3: '#68aae6', LIGHTS: '#ffd88a', INK: '#1a120a', PURPLE: '#7a4f88', ORPIMENT: '#e2b23e', KVELLUM: '#ead8b0' } },
  vellum: { dark: 0, illum: 0, gain: 1.8, label: { continent: '#8a5a12', ocean: '#2e4f8c', sea: '#3c5c96', country: '#3a2814', capital: '#5a4428', line: '#9a5a22', find: '#b0301c', title: '#7a4e0c', halo: 'rgba(232,218,184,0.72)' },
    c: { SKY: '#e2d3b0', STAR: '#96784e', SEABG: '#cbc8b6', SEA: '#385c96', SEAHI: '#1c3a78', COAST: '#2c508c', GLINT: '#aa781e', LANDBG: '#eee0ba',
      VELLUM: '#6e5638', VELLUMHI: '#342416', FOREST: '#2e703e', FORESTHI: '#145028', GRASS: '#607832', GRASSHI: '#3c5a1e', DESERT: '#aa7832', DESERTHI: '#82541a',
      ICE: '#607896', ICEHI: '#3c5478', MOUNT: '#8c4824', MOUNTHI: '#643014', SNOW: '#5a5a64', RIVER: '#325aaa', BORDER: '#be3824', GOLD: '#aa7c1e', GOLDHI: '#825a0a',
      VERM: '#ba3420', VERMHI: '#961e10', VERD: '#28785a', VERDHI: '#145a40', LAPIS: '#3254aa', HULL: '#643c1e', SAIL: '#968260', WHALE: '#46566e', WHALEHI: '#283850',
      SPRAY: '#506e96', FIRE: '#c85a14', GHOST: '#506eaa', MOON: '#a07828', CLOUD: '#8c7c60', CLOUDHI: '#5a4e3c', RHUMB0: '#6e5a3c', RHUMB1: '#3c7850', RHUMB2: '#aa3c28',
      WASH0: '#c84658', WASH1: '#c88c1e', WASH2: '#5a9646', WASH3: '#4678be', LIGHTS: '#c8961e', INK: '#1e140a', PURPLE: '#6a3f73', ORPIMENT: '#c8961e', KVELLUM: '#ecdcb6' } },
  illum: { dark: 0, illum: 1, gain: 1.9, label: { continent: '#9a6a0e', ocean: '#24448c', sea: '#2c4c94', country: '#2a1c0e', capital: '#4a3418', line: '#a8321e', find: '#b0301c', title: '#9a6a0e', halo: 'rgba(240,228,194,0.74)' },
    c: { SKY: '#e6d5ad', STAR: '#a8781e', SEABG: '#cbd6d0', SEA: '#2a4ea8', SEAHI: '#16348a', COAST: '#24449c', GLINT: '#c8961e', LANDBG: '#f2e5c0',
      VELLUM: '#5c4428', VELLUMHI: '#22160a', FOREST: '#2c8050', FORESTHI: '#126036', GRASS: '#5e8a34', GRASSHI: '#3a661c', DESERT: '#c88a24', DESERTHI: '#9a6210',
      ICE: '#5a78a8', ICEHI: '#34507e', MOUNT: '#9a4a1e', MOUNTHI: '#6e2c0c', SNOW: '#4a4a5a', RIVER: '#2a50b0', BORDER: '#c43a2a', GOLD: '#c8961e', GOLDHI: '#a87408',
      VERM: '#c43a2a', VERMHI: '#a82416', VERD: '#2e8a62', VERDHI: '#166646', LAPIS: '#2f4f9f', HULL: '#5a3418', SAIL: '#8a7650', WHALE: '#3a4e7a', WHALEHI: '#22325a',
      SPRAY: '#3a5a9a', FIRE: '#d0641a', GHOST: '#3a5ab0', MOON: '#b08218', CLOUD: '#7a6c52', CLOUDHI: '#4a3e2c', RHUMB0: '#8a6a2a', RHUMB1: '#2e8a5a', RHUMB2: '#c43a2a',
      WASH0: '#d2465a', WASH1: '#d8a028', WASH2: '#4e9a42', WASH3: '#3a70c0', LIGHTS: '#c8961e', INK: '#1a1008', PURPLE: '#6b3f73', ORPIMENT: '#d9a83a', KVELLUM: '#efe0b8' } },
  // the whole world after the Book of Kells: a carpet page on a dark ground
  kells: { dark: 1, illum: 1, kells: 1, gain: 1.15, label: { continent: '#f0c64a', ocean: '#9fb8ff', sea: '#93a8e8', country: '#f3e6c4', capital: '#e0d0a8', line: '#e2573f', find: '#ff8a6e', title: '#f6d470', halo: 'rgba(8,14,11,0.72)' },
    c: { SKY: '#070b09', STAR: '#e8d590', SEABG: '#0c1712', SEA: '#2b4da8', SEAHI: '#7f9cf0', COAST: '#f2cf62', GLINT: '#ffe08a', LANDBG: '#0f1d15',
      VELLUM: '#cdb47c', VELLUMHI: '#f8e7b6', FOREST: '#2e8a52', FORESTHI: '#7fd27a', GRASS: '#5a9c3c', GRASSHI: '#b8de6c', DESERT: '#e2a92a', DESERTHI: '#ffd36a',
      ICE: '#a8bde6', ICEHI: '#eef4ff', MOUNT: '#c8763c', MOUNTHI: '#f2a86a', SNOW: '#f6f0e0', RIVER: '#6d94f2', BORDER: '#e2573f', GOLD: '#d9a52a', GOLDHI: '#ffdc6e',
      VERM: '#d2442e', VERMHI: '#ff7a5c', VERD: '#2e9a6a', VERDHI: '#8be4b4', LAPIS: '#4a6ce0', HULL: '#a0643a', SAIL: '#f0e0b8', WHALE: '#6a7ea8', WHALEHI: '#b8c8ea',
      SPRAY: '#dce8ff', FIRE: '#ffa03c', GHOST: '#aac4ff', MOON: '#f6e2a0', CLOUD: '#7c8a80', CLOUDHI: '#c8d2c4', RHUMB0: '#b08a3a', RHUMB1: '#3a9a68', RHUMB2: '#c24a36',
      WASH0: '#e66e80', WASH1: '#ecba46', WASH2: '#80c868', WASH3: '#68aae6', LIGHTS: '#ffd88a', INK: '#f2cf62', PURPLE: '#8a5a98', ORPIMENT: '#e8b43a', KVELLUM: '#0f1d15' } },
};
for (const ink of Object.values(INKS)) ink.pal = new Float32Array(PAL_KEYS.flatMap(k => hex(ink.c[k])));

const LIMITS = { NKNOCK: 40, NSPR: 16, NROSE: 5, NMARK: 32, NART: 16, NASPR: 12 };

class Renderer {
  constructor(canvas, textCanvas) {
    this.canvas = canvas; this.text = textCanvas; this.tctx = textCanvas.getContext('2d');
    const gl = canvas.getContext('webgl2', { antialias: false, alpha: false, premultipliedAlpha: false, powerPreference: 'high-performance' });
    if (!gl) throw new Error('This device has no WebGL2.');
    this.gl = gl;
    this.grids = new Map();      // per cell size: the cell-pass framebuffer
    this.frames = [];            // two full-size frames, for transitions
  }

  async init(base = '') {
    const gl = this.gl;
    const [cellSrc] = await Promise.all([fetch(base + 'cell.glsl').then(r => r.text())]);
    const load = src => new Promise((res, rej) => { const im = new Image(); im.onload = () => res(im); im.onerror = () => rej(new Error('missing ' + src)); im.src = base + src; });
    const names = ['geoA', 'geoB', 'lo1', 'lo2', 'lo3', 'lo4'];
    const [imgs, sprites] = await Promise.all([Promise.all(names.map(n => load(`data/${n}.png`))), fetch(base + 'sprites.json').then(r => r.json())]);
    this.buildArt(sprites);
    this.tex = Object.fromEntries(names.map((n, i) => [n, this.texture(imgs[i], true)]));
    this.measureFont();
    const defines = [
      ...Object.entries(NAMED).map(([k, v]) => `#define G_${k} ${GI[v]}`),
      ...PAL_KEYS.map((k, i) => `#define P_${k} ${i}`),
      `#define NPAL ${PAL_KEYS.length}`,
      ...Object.entries(LIMITS).map(([k, v]) => `#define ${k} ${v}`),
      `#define NRAMP ${this.ramp.length}`,
      `const int RAMP[${this.ramp.length}] = int[${this.ramp.length}](${this.ramp.join(',')});`,
    ].join('\n');
    this.cellProg = this.program(QUAD_VS, cellSrc.replace('//%DEFINES%', defines));
    this.glyphProg = this.program(QUAD_VS, GLYPH_FS);
    this.mixProg = this.program(QUAD_VS, MIX_FS);
    const buf = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    this.vao = gl.createVertexArray(); gl.bindVertexArray(this.vao);
    gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    this.uloc = new Map();
  }

  // The creatures' drawings in one integer texture: R the character, G its colour (palette index
  // + 1; 0 see-through), B 1 for an opaque blank. Frames of a drawing are stacked under each other.
  buildArt(data) {
    const names = Object.keys(data.art), W = Math.max(...names.map(n => data.art[n].w));
    let H = 0; const meta = {};
    names.forEach((n, i) => { const a = data.art[n]; meta[n] = { i, y0: H, w: a.w, h: a.h, frames: a.frames.length, anchor: a.anchor }; H += a.h * a.frames.length; });
    const px = new Uint8Array(W * H * 4);
    for (const n of names) {
      const a = data.art[n], m = meta[n];
      a.frames.forEach((f, fi) => f.g.forEach((line, y) => [...line].forEach((ch, x) => {
        const o = ((m.y0 + fi * a.h + y) * W + x) * 4, code = f.m[y][x];
        if (code === ':') { px[o + 2] = 1; return; }
        if (ch === ' ' || code === ' ') return;
        px[o] = GI[ch] ?? GI['?'];
        px[o + 1] = PAL_KEYS.indexOf(data.colours[code]) + 1;
      })));
    }
    const gl = this.gl, t = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, t);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8UI, W, H, 0, gl.RGBA_INTEGER, gl.UNSIGNED_BYTE, px);
    for (const p of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, p, gl.NEAREST);
    this.artTex = t; this.artMeta = meta; this.artData = data; this.artCache = new Map();
    this.artTable = new Float32Array(LIMITS.NART * 4); this.artAnchor = new Float32Array(LIMITS.NART * 2);
    names.forEach((n, i) => { const m = meta[n]; this.artTable.set([m.y0, m.w, m.h, m.frames], i * 4); this.artAnchor.set(m.anchor, i * 2); });
    TI.ART = meta;
  }

  // A creature's frame as a bitmap of characters at k times the cell, for the lettering canvas.
  creatureBitmap(name, frame, k, cwDev, chDev, ink) {
    const key = [name, frame, k, cwDev, ink].join('|');
    let c = this.artCache.get(key);
    if (c) return c;
    const a = this.artData.art[name], f = a.frames[frame];
    c = document.createElement('canvas'); c.width = a.w * k * cwDev; c.height = a.h * k * chDev;
    const x = c.getContext('2d'); x.textAlign = 'center'; x.textBaseline = 'middle';
    const pal = INKS[ink].c, dark = INKS[ink].dark;
    f.g.forEach((line, y) => [...line].forEach((ch, xx) => {
      const code = f.m[y][xx]; if (ch === ' ' || code === ' ' || code === ':') return;
      x.fillStyle = pal[this.artData.colours[code]];
      if (!dark) x.globalAlpha = 0.95;
      this.drawGlyph(x, ch, (xx + 0.5) * k * cwDev, (y + 0.53) * k * chDev, k * chDev / 1.2);
    }));
    if (this.artCache.size > 80) this.artCache.clear();
    this.artCache.set(key, c);
    return c;
  }

  texture(img, linear) {
    const gl = this.gl, t = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, t);
    gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.NONE);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE, img);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, linear ? gl.LINEAR : gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, linear ? gl.LINEAR : gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    return t;
  }

  program(vs, fs) {
    const gl = this.gl;
    const sh = (type, src) => { const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s)); return s; };
    const p = gl.createProgram();
    gl.attachShader(p, sh(gl.VERTEX_SHADER, vs)); gl.attachShader(p, sh(gl.FRAGMENT_SHADER, fs));
    gl.bindAttribLocation(p, 0, 'aPos');
    gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
    return p;
  }

  u(p, name) {
    let m = this.uloc.get(p); if (!m) { m = new Map(); this.uloc.set(p, m); }
    if (!m.has(name)) m.set(name, this.gl.getUniformLocation(p, name));
    return m.get(name);
  }

  // The density ramp: candidate characters sorted by the ink they carry in this font.
  measureFont() {
    const c = document.createElement('canvas'); c.width = 40; c.height = 80;
    const x = c.getContext('2d', { willReadFrequently: true });
    x.textAlign = 'center'; x.textBaseline = 'middle';
    const ink = ch => {
      x.fillStyle = '#000'; x.fillRect(0, 0, 40, 80); x.fillStyle = '#fff';
      this.drawGlyph(x, ch, 20, 42, 66);
      const d = x.getImageData(0, 0, 40, 80).data; let s = 0;
      for (let i = 0; i < d.length; i += 4) s += d[i];
      return s;
    };
    const cand = [...RAMP_CAND].map(ch => ({ ch, v: ink(ch) })).sort((a, b) => a.v - b.v);
    const vmax = cand[cand.length - 1].v, n = 16, out = [];
    for (let j = 0; j < n; j++) {
      const want = vmax * j / (n - 1);
      let best = cand[0]; for (const q of cand) if (Math.abs(q.v - want) < Math.abs(best.v - want)) best = q;
      out.push(GI[best.ch]);
    }
    this.ramp = out;
  }
  drawGlyph(x, g, cx, cy, size) {
    const sym = EXTRA.includes(g) && g !== '¯' && g !== '°';
    x.font = `${sym ? 400 : 700} ${size}px ${sym ? '"Noto Sans Symbols 2", "JetBrains Mono", monospace' : '"JetBrains Mono", monospace'}`;
    x.fillText(sym ? g + '︎' : g, cx, cy);
  }
  atlasFor(cwDev, chDev) {
    const key = cwDev + 'x' + chDev;
    this.atlases = this.atlases || new Map();
    if (this.atlases.has(key)) return this.atlases.get(key);
    const per = 16, rows = Math.ceil(GLYPHS.length / per);
    const c = document.createElement('canvas'); c.width = cwDev * per; c.height = chDev * rows;
    const x = c.getContext('2d');
    x.fillStyle = '#000'; x.fillRect(0, 0, c.width, c.height);
    x.fillStyle = '#fff'; x.textAlign = 'center'; x.textBaseline = 'middle';
    GLYPHS.forEach((g, i) => {
      const ox = (i % per) * cwDev, oy = Math.floor(i / per) * chDev;
      x.save(); x.beginPath(); x.rect(ox, oy, cwDev, chDev); x.clip();
      this.drawGlyph(x, g, ox + cwDev / 2, oy + chDev * 0.53, chDev / 1.2);
      x.restore();
    });
    if (this.atlases.size >= 4) { for (const a of this.atlases.values()) this.gl.deleteTexture(a.tex); this.atlases.clear(); }
    const atlas = { key, tex: this.texture(c, false) };
    this.atlases.set(key, atlas);
    return atlas;
  }

  grid(cols, rows) {
    const key = cols + 'x' + rows;
    let g = this.grids.get(key);
    if (g) return g;
    const gl = this.gl;
    const mk = () => { const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, cols, rows, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      for (const p of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, p, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      return t; };
    const fg = mk(), bg = mk(), fb = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, fb);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, fg, 0);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT1, gl.TEXTURE_2D, bg, 0);
    gl.drawBuffers([gl.COLOR_ATTACHMENT0, gl.COLOR_ATTACHMENT1]);
    g = { cols, rows, fg, bg, fb };
    if (this.grids.size > 6) { for (const old of this.grids.values()) { gl.deleteTexture(old.fg); gl.deleteTexture(old.bg); gl.deleteFramebuffer(old.fb); } this.grids.clear(); }
    this.grids.set(key, g);
    return g;
  }
  frame(i) {
    const gl = this.gl, w = this.canvas.width, h = this.canvas.height;
    let f = this.frames[i];
    if (f && f.w === w && f.h === h) return f;
    if (f) { gl.deleteTexture(f.tex); gl.deleteFramebuffer(f.fb); }
    const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    for (const p of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, p, gl.NEAREST);
    const fb = gl.createFramebuffer(); gl.bindFramebuffer(gl.FRAMEBUFFER, fb);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, t, 0);
    f = this.frames[i] = { w, h, tex: t, fb };
    return f;
  }

  // Draw one view (v, from scene.js) into `target` (null: the screen).
  draw(v, target) {
    const gl = this.gl, dpr = v.dpr;
    const cwDev = Math.max(4, Math.round(v.grain * dpr)), chDev = cwDev * 2;
    const cw = cwDev / dpr, ch = chDev / dpr;
    const cols = Math.ceil(v.W / cw), rows = Math.ceil(v.H / ch);
    const g = this.grid(cols, rows), atlas = this.atlasFor(cwDev, chDev);
    v.cw = cw; v.ch = ch;
    // pass 1: every cell decides
    gl.bindFramebuffer(gl.FRAMEBUFFER, g.fb);
    gl.viewport(0, 0, cols, rows);
    const p = this.cellProg; gl.useProgram(p);
    const ink = INKS[v.ink];
    const T = (unit, tex, name) => { gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, tex); gl.uniform1i(this.u(p, name), unit); };
    T(0, this.tex.geoA, 'uGeoA'); T(1, this.tex.geoB, 'uGeoB'); T(2, this.tex.lo1, 'uLo1'); T(3, this.tex.lo2, 'uLo2');
    T(4, this.tex.lo3, 'uLo3'); T(5, this.tex.lo4, 'uLo4'); T(6, this.artTex, 'uArtTex');
    gl.uniform4fv(this.u(p, 'uArt[0]'), this.artTable); gl.uniform2fv(this.u(p, 'uArtA[0]'), this.artAnchor);
    const cr = v.creatures || [], ca = new Float32Array(LIMITS.NASPR * 4);
    cr.slice(0, LIMITS.NASPR).forEach((c, i) => ca.set([c.x, c.y, this.artMeta[c.art].i, (c.frame % this.artMeta[c.art].frames) * 8 + c.k], i * 4));
    gl.uniform4fv(this.u(p, 'uAS[0]'), ca); gl.uniform1i(this.u(p, 'uASN'), Math.min(cr.length, LIMITS.NASPR));
    gl.uniform2f(this.u(p, 'uGrid'), cols, rows); gl.uniform2f(this.u(p, 'uCell'), cw, ch); gl.uniform2f(this.u(p, 'uView'), v.W, v.H);
    gl.uniform2f(this.u(p, 'uCam'), v.cam.x, v.cam.y); gl.uniform1f(this.u(p, 'uScale'), v.scale); gl.uniform1f(this.u(p, 'uDelta'), cw * 0.5);
    gl.uniform1f(this.u(p, 'uTime'), v.time); gl.uniform1f(this.u(p, 'uDetail'), v.detail);
    gl.uniform1i(this.u(p, 'uProj'), v.proj); gl.uniform2f(this.u(p, 'uHemi'), v.hemi[0], v.hemi[1]); gl.uniform2f(this.u(p, 'uHemiC'), v.hc[0], v.hc[1]);
    gl.uniform1f(this.u(p, 'uOvalLon'), v.ovalLon); gl.uniform2f(this.u(p, 'uGlobe'), v.globe[0], v.globe[1]);
    gl.uniform3fv(this.u(p, 'uPal[0]'), ink.pal);
    gl.uniform1f(this.u(p, 'uDark'), ink.dark); gl.uniform1f(this.u(p, 'uIllum'), ink.illum); gl.uniform1f(this.u(p, 'uRelief'), v.relief);
    gl.uniform1f(this.u(p, 'uKellsAll'), ink.kells ? 1 : 0);
    gl.uniform4f(this.u(p, 'uKells'), v.kells[0], v.kells[1], v.kells[2], v.kells[3]);
    gl.uniform4f(this.u(p, 'uSun'), v.sun[0], v.sun[1], v.sun[2], v.sun[3]); gl.uniform1f(this.u(p, 'uSunAz'), v.sunAz);
    gl.uniform4f(this.u(p, 'uReveal'), v.reveal[0], v.reveal[1], v.reveal[2], v.reveal[3]);
    gl.uniform2f(this.u(p, 'uStars'), v.stars[0], v.stars[1]);
    const arr = (name, list, n, count) => {
      const a = new Float32Array(n * 4); list.slice(0, n).forEach((q, i) => a.set(q, i * 4));
      gl.uniform4fv(this.u(p, name + '[0]'), a); if (count) gl.uniform1i(this.u(p, count), Math.min(n, list.length));
    };
    arr('uKnock', v.knock, LIMITS.NKNOCK, 'uKnockN');
    arr('uSpr', v.sprites.map(s => s.a), LIMITS.NSPR, 'uSprN');
    arr('uSprP', v.sprites.map(s => s.b), LIMITS.NSPR, null);
    arr('uRose', v.roses, LIMITS.NROSE, 'uRoseN');
    arr('uMark', v.marks, LIMITS.NMARK, 'uMarkN');
    gl.bindVertexArray(this.vao);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    // pass 2: paint the characters
    gl.bindFramebuffer(gl.FRAMEBUFFER, target ? target.fb : null);
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    const q = this.glyphProg; gl.useProgram(q);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, g.fg); gl.uniform1i(this.u(q, 'uFg'), 0);
    gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, g.bg); gl.uniform1i(this.u(q, 'uBg'), 1);
    gl.activeTexture(gl.TEXTURE2); gl.bindTexture(gl.TEXTURE_2D, atlas.tex); gl.uniform1i(this.u(q, 'uAtlas'), 2);
    gl.uniform2f(this.u(q, 'uCell'), cwDev, chDev); gl.uniform2f(this.u(q, 'uGrid'), cols, rows);
    gl.uniform2f(this.u(q, 'uRes'), this.canvas.width, this.canvas.height);
    gl.uniform3fv(this.u(q, 'uSky'), ink.pal.subarray(0, 3));
    gl.uniform1f(this.u(q, 'uShine'), ink.illum || ink.kells ? v.time : -1);
    gl.uniform1f(this.u(q, 'uGain'), ink.gain);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    return { cw, ch, cols, rows };
  }

  // A frame, or two blended: kind 0 dissolve (character by character), 1 wipe, 2 iris.
  render(a, b, t, kind) {
    const gl = this.gl;
    if (!b || t <= 0) return this.draw(a, null);
    if (t >= 1) return this.draw(b, null);
    const fa = this.frame(0), fb = this.frame(1);
    this.draw(a, fa); const info = this.draw(b, fb);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    const p = this.mixProg; gl.useProgram(p);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, fa.tex); gl.uniform1i(this.u(p, 'uA'), 0);
    gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, fb.tex); gl.uniform1i(this.u(p, 'uB'), 1);
    gl.uniform1f(this.u(p, 'uT'), t); gl.uniform1i(this.u(p, 'uKind'), kind || 0);
    gl.uniform2f(this.u(p, 'uRes'), this.canvas.width, this.canvas.height);
    gl.uniform2f(this.u(p, 'uCell'), Math.round(Math.max(a.grain, b.grain) * a.dpr), Math.round(Math.max(a.grain, b.grain) * a.dpr) * 2);
    gl.bindVertexArray(this.vao);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    return info;
  }
}

const QUAD_VS = `#version 300 es
in vec2 aPos; void main() { gl_Position = vec4(aPos, 0.0, 1.0); }`;

const GLYPH_FS = `#version 300 es
precision highp float;
uniform sampler2D uFg, uBg, uAtlas;
uniform vec2 uCell, uGrid, uRes; uniform vec3 uSky; uniform float uShine, uGain;
out vec4 o;
void main() {
  vec2 p = vec2(gl_FragCoord.x, uRes.y - gl_FragCoord.y);
  vec2 c = floor(p / uCell);
  if (c.x >= uGrid.x || c.y >= uGrid.y) { o = vec4(uSky, 1.0); return; }
  vec2 f = floor(p - c * uCell);
  ivec2 ci = ivec2(c.x, uGrid.y - 1.0 - c.y);
  vec4 fg = texelFetch(uFg, ci, 0), bg = texelFetch(uBg, ci, 0);
  int g = int(bg.a * 255.0 + 0.5);
  ivec2 ap = ivec2(g % 16, g / 16) * ivec2(uCell) + ivec2(min(f, uCell - 1.0));
  float a = texelFetch(uAtlas, ap, 0).r;
  float s = fg.a * 2.0 * uGain;
  vec3 ink = fg.rgb * max(1.0, uGain > 1.0 ? 1.0 : s);
  // gold leaf catches the light: a slow sheen travelling across the page
  if (uShine >= 0.0) {
    float gold = step(0.5, fg.r) * step(fg.b, fg.r * 0.55) * step(fg.g, fg.r * 0.95);
    float band = smoothstep(0.08, 0.0, abs(fract((p.x * 0.6 + p.y) / (uRes.x * 1.4) - uShine * 0.08) - 0.5) - 0.42);
    ink = mix(ink, vec3(1.0, 0.93, 0.62), gold * band * 0.75);
  }
  o = vec4(mix(bg.rgb, min(ink, vec3(1.0)), a * min(s, 1.0)), 1.0);
}`;

const MIX_FS = `#version 300 es
precision highp float;
uniform sampler2D uA, uB; uniform float uT; uniform int uKind; uniform vec2 uRes, uCell;
out vec4 o;
float h12(vec2 p) { vec3 p3 = fract(vec3(p.xyx) * 0.1031); p3 += dot(p3, p3.yzx + 33.33); return fract((p3.x + p3.y) * p3.z); }
void main() {
  vec2 uv = gl_FragCoord.xy / uRes;
  vec4 a = texture(uA, uv), b = texture(uB, uv);
  float m;
  if (uKind == 0) { vec2 c = floor(gl_FragCoord.xy / uCell); m = step(h12(c), uT * 1.02); }
  else if (uKind == 1) { float x = (uv.x * 0.7 + (1.0 - uv.y) * 0.3); m = smoothstep(x - 0.04, x + 0.04, uT * 1.1 - 0.05); vec2 c = floor(gl_FragCoord.xy / uCell); m = step(h12(c) * 0.25 + 0.75 * (1.0 - m), 0.75 + 0.25 * m); }
  else { float d = length((uv - 0.5) * vec2(uRes.x / uRes.y, 1.0)); float r = uT * 1.05; m = smoothstep(r, r - 0.04, d); vec2 c = floor(gl_FragCoord.xy / uCell); if (abs(d - r) < 0.03) m = step(h12(c), 0.5); }
  o = mix(a, b, m);
}`;

window.TI = Object.assign(window.TI || {}, { Renderer, INKS, GLYPHS, GI });

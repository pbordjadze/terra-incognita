// The Paper Almanac's materials, in plate pixels. A plate is a sheet of uncoated cream stock; water is
// a watercolour wash soaked into it, stopping short of every coast; land is torn coloured paper laid
// nearly flat, its white fibrous core showing at the tear; names are letterpressed; small plants and
// trees are stamped in two colours a touch off register; living things are painted in gouache and cut
// out with scissors. Every value is seeded, so a plate draws the same on every run.
//
// Sizes are tuned for the phone: at rest one point of the screen is about U plate pixels, so a tear,
// a hairline or a letter is the same size on screen at every scale. SVG filters do the tearing and the
// watercolour; they stay in user space, so a pass plate (the exporter's magnified copy) re-renders
// them sharp.
'use strict';
const PA = (() => {
  const W = Kit.W, H = Kit.H, U = 2.9;
  const C = {
    paper: '#efe8d8', fibreDark: '#b9a37c', fibreLight: '#fbf8f1',
    ink: '#2e2a20', inkSoft: '#4a4334', pencil: '#5d594f',
    seaA: '#628e9b', seaB: '#8aa27e', seaDeep: '#416c7b', seaInk: '#24434c',
    kraft: '#c4a882', kraftLight: '#cfb792', kraftDark: '#b39670',
    sage: '#8e9c70', sageDeep: '#6f7d55', sageLight: '#a4ae86',
    washi: '#eeeee8', ochre: '#c8a45a', ochreDeep: '#a9843c',
    clay: '#c49478', heather: '#95776f',
    rust: '#b0603e', rustDeep: '#8e4228', haw: '#a8402c',
  };

  // ---------- the view ----------
  // A plate is drawn as it is (s = 1) or, for a pass plate, magnified s times about (cx, cy) by the
  // page itself. Either way the large filters only work over what the plate can show (plus a margin
  // for the tears that reach in from outside): window() is that rectangle in the plate's own units.
  const VIEW = { s: 1, cx: W / 2, cy: H / 2 };
  function setView(s, cx, cy) { VIEW.s = s; VIEW.cx = cx; VIEW.cy = cy; }
  // (Chromium renders a filter, or an image a filter reads, no larger than 4096 pixels a side, so the
  // margin is kept to 40 pixels of the plate as drawn, whatever the magnification.)
  function window_(margin = 40 / VIEW.s) {
    const w = W / VIEW.s, h = H / VIEW.s;
    return [VIEW.cx - w / 2 - margin, VIEW.cy - h / 2 - margin, w + 2 * margin, h + 2 * margin];
  }
  const regionAttr = r => `x="${f1(r[0])}" y="${f1(r[1])}" width="${f1(r[2])}" height="${f1(r[3])}" filterUnits="userSpaceOnUse"`;
  const R = () => regionAttr(window_());
  // The bounding box of an SVG path with absolute commands (M, L, H, V, Q, C, Z).
  function pathBox(d) {
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity, cmd = 'M', i = 0;
    const tok = d.match(/[MLHVQCZmlhvqcz]|-?\d*\.?\d+(?:e-?\d+)?/g) || [];
    const add = (x, y) => { if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; };
    let cx = 0, cy = 0;
    while (i < tok.length) {
      if (/[A-Za-z]/.test(tok[i])) { cmd = tok[i++]; continue; }
      if (cmd === 'H') { cx = +tok[i++]; add(cx, cy); }
      else if (cmd === 'V') { cy = +tok[i++]; add(cx, cy); }
      else { cx = +tok[i++]; cy = +tok[i++]; add(cx, cy); }
    }
    return [x0, y0, x1 - x0, y1 - y0];
  }
  function intersect(a, b) {
    const x0 = Math.max(a[0], b[0]), y0 = Math.max(a[1], b[1]), x1 = Math.min(a[0] + a[2], b[0] + b[2]), y1 = Math.min(a[1] + a[3], b[1] + b[3]);
    return x1 > x0 && y1 > y0 ? [x0, y0, x1 - x0, y1 - y0] : null;
  }

  // ---------- seeded randomness ----------
  function rng(seed) {
    let s = seed | 0;
    return () => { s = s + 0x6D2B79F5 | 0; let t = Math.imul(s ^ s >>> 15, 1 | s); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; };
  }
  const f1 = v => Math.round(v * 10) / 10, f2 = v => Math.round(v * 100) / 100;
  const esc = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;');

  // ---------- filters ----------
  // A torn piece: the narrowest inlets closed (no one tears a fjord), a coarse warp for the tear, fine
  // fuzz for its fibres, the white core showing past the coloured face, the paper's own mottle and
  // grain, and a shadow barely a point deep.
  function tornFilter(id, o) {
    const s = o.seed, k = o.k || 1;
    const close = (o.close ?? 1.1) * U * k, warp = (o.warp ?? 4.2) * U * k, fuzz = (o.fuzz ?? 1.3) * U * k, rim = (o.rim ?? 0.8) * U * k;
    return `<filter id="${id}" ${o.region || 'x="-3%" y="-3%" width="106%" height="106%"'} color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="${f2(0.05 / U / k * 1000) / 1000}" numOctaves="2" seed="${s}" result="warp"/>
    <feGaussianBlur in="SourceGraphic" stdDeviation="${f2(close)}" result="cl1"/>
    <feComponentTransfer in="cl1" result="cl2"><feFuncA type="linear" slope="8" intercept="-3.6"/></feComponentTransfer>
    <feDisplacementMap in="cl2" in2="warp" scale="${f2(warp)}" xChannelSelector="R" yChannelSelector="G" result="shape"/>
    <feTurbulence type="fractalNoise" baseFrequency="${f2(1.2 / U / k * 100) / 100}" numOctaves="1" seed="${s + 1}" result="fuzzN"/>
    <feDisplacementMap in="shape" in2="fuzzN" scale="${f2(fuzz)}" xChannelSelector="R" yChannelSelector="G" result="face"/>
    <feGaussianBlur in="shape" stdDeviation="${f2(rim * 1.2)}" result="fat0"/>
    <feComponentTransfer in="fat0" result="fat"><feFuncA type="linear" slope="8" intercept="-1"/></feComponentTransfer>
    <feDisplacementMap in="fat" in2="warp" scale="${f2(rim * 2.6)}" xChannelSelector="G" yChannelSelector="R" result="fat2"/>
    <feDisplacementMap in="fat2" in2="fuzzN" scale="${f2(2.2 * U * k)}" xChannelSelector="G" yChannelSelector="R" result="core"/>
    <feFlood flood-color="#fdfbf4"/>
    <feComposite in2="core" operator="in" result="rimC"/>
    <feGaussianBlur in="core" stdDeviation="${f2(0.9 * U)}" result="sh0"/>
    <feOffset in="sh0" dx="${f2(0.5 * U)}" dy="${f2(0.8 * U)}" result="sh1"/>
    <feFlood flood-color="#3a2d18" flood-opacity="${o.shadow || 0.28}"/>
    <feComposite in2="sh1" operator="in" result="shadow"/>
    <feTurbulence type="fractalNoise" baseFrequency="${f2((o.mottleF || 0.055) / U * 1000) / 1000}" numOctaves="2" seed="${s + 4}" result="mot"/>
    <feTurbulence type="fractalNoise" baseFrequency="${o.grainF || '0.24 0.12'}" numOctaves="1" seed="${s + 5}" result="grn"/>
    <feComposite in="mot" in2="grn" operator="arithmetic" k1="0" k2="${o.mottle || 0.55}" k3="${o.grain || 0.45}" k4="0" result="tx"/>
    <feColorMatrix in="tx" type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  ${o.texK || -0.9} 0 0 0 ${o.texB || 0.5}" result="txA"/>
    <feFlood flood-color="${o.texColor || '#2e2210'}"/>
    <feComposite in2="txA" operator="in" result="darkTx"/>
    <feColorMatrix in="tx" type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  ${o.hiK || 0.9} 0 0 0 ${o.hiB || -0.52}" result="hiA"/>
    <feFlood flood-color="#fffdf6"/>
    <feComposite in2="hiA" operator="in" result="hiTx"/>
    <feMerge result="faceTx"><feMergeNode in="face"/><feMergeNode in="darkTx"/><feMergeNode in="hiTx"/></feMerge>
    <feComposite in="faceTx" in2="face" operator="in" result="faceOut"/>
    <feComponentTransfer in="faceOut" result="faceA"><feFuncA type="linear" slope="${o.opacity || 1}"/></feComponentTransfer>
    <feMerge><feMergeNode in="shadow"/><feMergeNode in="rimC"/><feMergeNode in="faceA"/></feMerge>
  </filter>`;
  }

  // The papers: a fill and how its stock looks.
  const PAPERS = {
    kraft: { fill: C.kraft, f: { grainF: '0.38 0.21', grain: 0.55, mottle: 0.3, mottleF: 0.08, texK: -0.6, texB: 0.36, hiK: 0.7, hiB: -0.43 } },
    kraftLight: { fill: C.kraftLight, f: { grainF: '0.38 0.21', grain: 0.5, mottle: 0.4, mottleF: 0.05, texK: -0.6, texB: 0.36, hiK: 0.7, hiB: -0.43 } },
    city: { fill: C.kraft, f: { grainF: '0.38 0.21', grain: 0.55, mottle: 0.3, mottleF: 0.08, texK: -0.6, texB: 0.36, hiK: 0.7, hiB: -0.43, close: 2.2, warp: 2.4, rim: 0.7 } },
    sage: { fill: C.sage, f: { texColor: '#2c3418', mottle: 0.32, grain: 0.48, texK: -0.6, texB: 0.34, mottleF: 0.08 } },
    sageLight: { fill: C.sageLight, f: { texColor: '#2c3418', mottle: 0.66, grain: 0.32, texK: -0.8, texB: 0.45 } },
    washi: { fill: C.washi, f: { opacity: 0.9, texColor: '#4a4a40', mottle: 0.5, grain: 0.5, texK: -0.7, texB: 0.36, rim: 1.0, warp: 3.2, close: 0.6 } },
    ochre: { fill: C.ochre, f: { texColor: '#3a2a08', mottle: 0.7, grain: 0.35 } },
    clay: { fill: C.clay, f: { texColor: '#3a1e10', mottle: 0.62, grain: 0.38, grainF: '0.28 0.19' } },
    heather: { fill: C.heather, f: { texColor: '#2a1a18', mottle: 0.7, grain: 0.3 } },
    kraftDark: { fill: C.kraftDark, f: { grainF: '0.38 0.21', grain: 0.42, mottle: 0.6, texK: -0.75, texB: 0.42, hiK: 0.7, hiB: -0.42 } },
  };

  function defs(extra = '') {
    return `<defs>
  <!-- a seam's tear: its edge ragged in the paper's fibres -->
  <filter id="seamTear" ${R()}>
    <feTurbulence type="fractalNoise" baseFrequency="${f2(0.6 / U * 100) / 100}" numOctaves="2" seed="44" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="${f2(1.8 * U)}" xChannelSelector="R" yChannelSelector="G"/>
  </filter>
  <!-- where the pigment gathered at the wash's edge, softening into the open water -->
  <filter id="soften" ${R()}><feGaussianBlur stdDeviation="${f2(1.6 * U)}"/></filter>

  <!-- a river or a lake's thread: the same pigment, laid with one stroke of the brush -->
  <filter id="brush" ${R()} color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="0.05" numOctaves="2" seed="27" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="${f2(1.6 * U)}" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feGaussianBlur in="d" stdDeviation="${f2(0.5 * U)}" result="b"/>
    <feComposite in="d" in2="b" operator="arithmetic" k1="0" k2="1.7" k3="-0.7" k4="0"/>
  </filter>

  <filter id="pencil" ${R()}>
    <feTurbulence type="fractalNoise" baseFrequency="0.31" numOctaves="1" seed="31" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="${f2(1.1 * U)}" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feColorMatrix in="n" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  1.8 0 0 0 -0.45" result="skip"/>
    <feComposite in="d" in2="skip" operator="in"/>
  </filter>

  <!-- letterpress: the type bites a little ragged, the ink sits unevenly and squeezes darker at its edges -->
  <filter id="press" ${R()} color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="0.41" numOctaves="2" seed="17" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="${f2(0.55 * U)}" xChannelSelector="R" yChannelSelector="G" result="rough"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.33" numOctaves="2" seed="19" result="inkN"/>
    <feColorMatrix in="inkN" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  -1.1 0 0 0 1.5" result="cover"/>
    <feComposite in="rough" in2="cover" operator="in" result="inked"/>
    <feMorphology in="rough" operator="erode" radius="${f2(0.32 * U)}" result="er"/>
    <feComposite in="rough" in2="er" operator="out" result="rim"/>
    <feColorMatrix in="rim" type="matrix" values="0.55 0 0 0 0  0 0.55 0 0 0  0 0 0.55 0 0  0 0 0 0.6 0" result="rimD"/>
    <feMerge><feMergeNode in="inked"/><feMergeNode in="rimD"/></feMerge>
  </filter>
  <!-- the impression: a faint light lip below and right of each letter, where the paper was pressed down -->
  <filter id="deboss" ${R()}>
    <feGaussianBlur in="SourceAlpha" stdDeviation="${f2(0.35 * U)}" result="b"/>
    <feOffset in="b" dx="${f2(0.45 * U)}" dy="${f2(0.55 * U)}" result="o"/>
    <feComposite in="o" in2="SourceAlpha" operator="out" result="lip"/>
    <feFlood flood-color="#fffdf5" flood-opacity="0.85"/>
    <feComposite in2="lip" operator="in" result="lit"/>
    <feOffset in="b" dx="${f2(-0.35 * U)}" dy="${f2(-0.45 * U)}" result="o2"/>
    <feComposite in="o2" in2="SourceAlpha" operator="out" result="lip2"/>
    <feFlood flood-color="#3a2d18" flood-opacity="0.22"/>
    <feComposite in2="lip2" operator="in" result="dark"/>
    <feMerge><feMergeNode in="lit"/><feMergeNode in="dark"/></feMerge>
  </filter>
  <!-- gouache on paper: the pigment a little uneven, darker where the brush stopped -->
  <filter id="gouache" x="-5%" y="-5%" width="110%" height="110%" color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="0.21" numOctaves="2" seed="23" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="${f2(0.7 * U)}" xChannelSelector="R" yChannelSelector="G" result="g"/>
    <feGaussianBlur in="g" stdDeviation="${f2(0.7 * U)}" result="gb"/>
    <feComposite in="g" in2="gb" operator="arithmetic" k1="0" k2="1.5" k3="-0.5" k4="0" result="edged"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.031" numOctaves="3" seed="29" result="m"/>
    <feColorMatrix in="m" type="matrix" values="0 0 0 0 1  0 0 0 0 0.97  0 0 0 0 0.9  1.1 0 0 0 -0.5" result="mA"/>
    <feComposite in="mA" in2="g" operator="in" result="light"/>
    <feMerge result="painted"><feMergeNode in="edged"/><feMergeNode in="light"/></feMerge>
    <feComposite in="painted" in2="g" operator="in"/>
  </filter>
  <!-- brush ink: a slightly wavering line that runs thin in places -->
  <filter id="inkline" ${R()}>
    <feTurbulence type="fractalNoise" baseFrequency="0.24" numOctaves="2" seed="37" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="${f2(0.8 * U)}" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.086" numOctaves="2" seed="38" result="w"/>
    <feColorMatrix in="w" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  -2 0 0 0 1.9" result="wA"/>
    <feComposite in="d" in2="wA" operator="in"/>
  </filter>
  <filter id="inklineS" x="-5%" y="-5%" width="110%" height="110%">
    <feTurbulence type="fractalNoise" baseFrequency="0.24" numOctaves="2" seed="37" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="${f2(0.8 * U)}" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.086" numOctaves="2" seed="38" result="w"/>
    <feColorMatrix in="w" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  -2 0 0 0 1.9" result="wA"/>
    <feComposite in="d" in2="wA" operator="in"/>
  </filter>
  <!-- a painted thing cut out with scissors, a hair of paper left around it, glued down -->
  <filter id="cutout" x="-10%" y="-10%" width="120%" height="120%" color-interpolation-filters="sRGB">
    <feMorphology in="SourceAlpha" operator="dilate" radius="${f2(1.05 * U)}" result="m0"/>
    <feGaussianBlur in="m0" stdDeviation="${f2(0.5 * U)}" result="m1"/>
    <feComponentTransfer in="m1" result="m2"><feFuncA type="linear" slope="3" intercept="-1"/></feComponentTransfer>
    <feFlood flood-color="#f1e9d6"/>
    <feComposite in2="m2" operator="in" result="margin"/>
    <feGaussianBlur in="m2" stdDeviation="${f2(0.9 * U)}" result="s0"/>
    <feOffset in="s0" dx="${f2(0.6 * U)}" dy="${f2(1 * U)}" result="s1"/>
    <feFlood flood-color="#2e2210" flood-opacity="0.32"/>
    <feComposite in2="s1" operator="in" result="shadow"/>
    <feMerge><feMergeNode in="shadow"/><feMergeNode in="margin"/><feMergeNode in="SourceGraphic"/></feMerge>
  </filter>
  <!-- riso: a flat spot colour with the printer's grain in it -->
  <filter id="riso" ${R()} color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="0.45" numOctaves="1" seed="41" result="n"/>
    <feColorMatrix in="n" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  -1.3 0 0 0 1.5" result="nA"/>
    <feComposite in="SourceGraphic" in2="nA" operator="in"/>
  </filter>
  <!-- card, cut clean with a knife: a soft shadow and the stock's mottle -->
  <filter id="card" x="-5%" y="-5%" width="110%" height="110%" color-interpolation-filters="sRGB">
    <feGaussianBlur in="SourceAlpha" stdDeviation="${f2(1.1 * U)}" result="b"/><feOffset in="b" dx="${f2(0.7 * U)}" dy="${f2(1.2 * U)}" result="o"/>
    <feFlood flood-color="#2e2210" flood-opacity="0.34"/><feComposite in2="o" operator="in" result="sh"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.017" numOctaves="3" seed="61" result="m"/>
    <feColorMatrix in="m" type="matrix" values="0 0 0 0 0.35  0 0 0 0 0.27  0 0 0 0 0.15  0.5 0 0 0 -0.18" result="mA"/>
    <feComposite in="mA" in2="SourceAlpha" operator="in" result="mot"/>
    <feMerge><feMergeNode in="sh"/><feMergeNode in="SourceGraphic"/><feMergeNode in="mot"/></feMerge>
  </filter>
  <!-- a street cut through the paper: the cream beneath, shadowed under the cut's upper edge -->
  <filter id="cutStreet" ${R()} color-interpolation-filters="sRGB">
    <feOffset in="SourceAlpha" dx="${f2(0.45 * U)}" dy="${f2(0.65 * U)}" result="o"/>
    <feComposite in="SourceAlpha" in2="o" operator="out" result="band"/>
    <feGaussianBlur in="band" stdDeviation="${f2(0.35 * U)}" result="bb"/>
    <feFlood flood-color="#3a2d18" flood-opacity="0.38"/>
    <feComposite in2="bb" operator="in" result="sh0"/>
    <feComposite in="sh0" in2="SourceAlpha" operator="in" result="sh"/>
    <feMerge><feMergeNode in="SourceGraphic"/><feMergeNode in="sh"/></feMerge>
  </filter>
  <!-- small pieces of card laid on the paper: a shadow a point deep and the card's mottle -->
  <filter id="cards" ${R()} color-interpolation-filters="sRGB">
    <feGaussianBlur in="SourceAlpha" stdDeviation="${f2(0.55 * U)}" result="b"/><feOffset in="b" dx="${f2(0.5 * U)}" dy="${f2(0.75 * U)}" result="o"/>
    <feFlood flood-color="#2e2210" flood-opacity="0.42"/><feComposite in2="o" operator="in" result="sh"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.035" numOctaves="2" seed="63" result="m"/>
    <feColorMatrix in="m" type="matrix" values="0 0 0 0 0.35  0 0 0 0 0.27  0 0 0 0 0.15  0.5 0 0 0 -0.2" result="mA"/>
    <feComposite in="mA" in2="SourceAlpha" operator="in" result="mot"/>
    <feMorphology in="SourceAlpha" operator="erode" radius="0.6" result="er"/>
    <feComposite in="SourceAlpha" in2="er" operator="out" result="edge"/>
    <feFlood flood-color="#fffaf0" flood-opacity="0.55"/><feComposite in2="edge" operator="in" result="lip"/>
    <feMerge><feMergeNode in="sh"/><feMergeNode in="SourceGraphic"/><feMergeNode in="mot"/><feMergeNode in="lip"/></feMerge>
  </filter>
  ${extra}
</defs>`;
  }

  // ---------- low-frequency noise, painted once into a small image ----------
  // The large soft variations (the paper's formation, where a wash pooled or ran green) carry no
  // detail, so they are drawn at a sixth of the plate's resolution and stretched over it: cheap to
  // render, and still smooth when the exporter magnifies a plate for a pass.
  function fbm(seed) {
    const r = rng(seed), perm = new Uint8Array(512), val = new Float32Array(256);
    for (let i = 0; i < 256; i++) { perm[i] = i; val[i] = r(); }
    for (let i = 255; i > 0; i--) { const j = Math.floor(r() * (i + 1)); const t = perm[i]; perm[i] = perm[j]; perm[j] = t; }
    for (let i = 0; i < 256; i++) perm[256 + i] = perm[i];
    const lat = (x, y) => val[perm[perm[x & 255] + (y & 255)]];
    const sm = t => t * t * (3 - 2 * t);
    const n = (x, y) => {
      const xi = Math.floor(x), yi = Math.floor(y), u = sm(x - xi), v = sm(y - yi);
      const a = lat(xi, yi), b = lat(xi + 1, yi), c = lat(xi, yi + 1), d = lat(xi + 1, yi + 1);
      return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
    };
    return (x, y, oct = 4) => { let s = 0, amp = 0.5, f = 1, tot = 0; for (let o = 0; o < oct; o++) { s += amp * n(x * f, y * f); tot += amp; amp *= 0.5; f *= 2.03; } return s / tot; };
  }
  // fn(x, y) in plate pixels gives [r, g, b, a], 0..255
  function noiseImage(fn, scale = 6) {
    const w = Math.ceil(W / scale), h = Math.ceil(H / scale), cv = document.createElement('canvas');
    cv.width = w; cv.height = h;
    const ctx = cv.getContext('2d'), img = ctx.createImageData(w, h);
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
      const c = fn(x * scale, y * scale), i = (y * w + x) * 4;
      img.data[i] = c[0]; img.data[i + 1] = c[1]; img.data[i + 2] = c[2]; img.data[i + 3] = c[3];
    }
    ctx.putImageData(img, 0, 0);
    return `<image href="${cv.toDataURL()}" x="0" y="0" width="${W}" height="${H}" preserveAspectRatio="none"/>`;
  }
  const hex = h => [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)];
  const mix = (a, b, t) => a.map((v, i) => v + (b[i] - v) * t);
  // The paper's formation: soft clouds where the pulp lay thicker.
  let cloudsCache = null;
  function clouds() {
    if (cloudsCache) return cloudsCache;
    const n = fbm(11);
    return cloudsCache = noiseImage((x, y) => { const v = n(x / 520, y / 520); return [115, 92, 52, Math.max(0, Math.min(255, (v - 0.52) * 90))]; });
  }
  // A wash's pigment over the whole plate: two pigments running together, deeper where o.deep says,
  // paler where the water pooled and dried thin.
  function washImage(o = {}) {
    const a = hex(C.seaA), b = hex(C.seaB), dp = hex(C.seaDeep), white = [255, 255, 255];
    const nMix = fbm(13), nPool = fbm(8), deep = o.deep || ((x, y) => 0.25 + 0.35 * (1 - (x / W * 0.45 + y / H * 0.55)));
    return noiseImage((x, y) => {
      const g = Math.max(0, Math.min(1, (nMix(x / 900, y / 900, 3) - 0.42) * 2.4));
      const p = nPool(x / 430, y / 430, 4);
      let c = mix(a, b, g * 0.85);
      c = mix(c, dp, Math.max(0, Math.min(1, deep(x, y) + (p - 0.5) * 0.5)));
      c = mix(c, white, Math.max(0, (p - 0.52) * 1.1));
      return [c[0], c[1], c[2], 255];
    });
  }

  // ---------- the sheet ----------
  function sheet(rand, n = 1900) {
    let fib = '', fibD = '', fleck = '';
    for (let i = 0; i < n; i++) {
      const x = rand() * W, y = rand() * H, L = (2.5 + rand() * 6.5) * U, a = rand() * Math.PI, bend = (rand() * 4 - 2) * U;
      const x2 = x + Math.cos(a) * L, y2 = y + Math.sin(a) * L;
      const cx = (x + x2) / 2 + Math.cos(a + Math.PI / 2) * bend, cy = (y + y2) / 2 + Math.sin(a + Math.PI / 2) * bend;
      const d = `M${f1(x)} ${f1(y)}Q${f1(cx)} ${f1(cy)} ${f1(x2)} ${f1(y2)}`;
      if (rand() < 0.62) fib += d; else fibD += d;
    }
    const cols = [C.rust, C.sageDeep, '#7a5a3a', C.ochreDeep];
    for (let i = 0; i < n / 34; i++) {
      const x = rand() * W, y = rand() * H, L = (0.8 + rand() * 1.6) * U, a = rand() * Math.PI;
      fleck += `<path d="M${f1(x)} ${f1(y)}l${f2(Math.cos(a) * L)} ${f2(Math.sin(a) * L)}" stroke="${cols[i % 4]}" stroke-width="${f2((0.35 + rand() * 0.25) * U)}" opacity="${f2(0.25 + rand() * 0.25)}"/>`;
    }
    return `<rect width="${W}" height="${H}" fill="${C.paper}"/>${clouds()}
      <g fill="none" stroke-linecap="round"><path d="${fib}" stroke="${C.fibreLight}" stroke-width="${f2(0.4 * U)}" opacity="0.55"/><path d="${fibD}" stroke="${C.fibreDark}" stroke-width="${f2(0.3 * U)}" opacity="0.35"/>${fleck}</g>`;
  }

  // Watercolour over an area (white) with the land (black, stroked for the reserve) kept dry.
  // Watercolour over an area, the land kept dry: the group multiplies onto the paper, so the white
  // painted over the land and the reserve along its shore leaves the paper as it was. Just outside
  // the reserve the pigment gathers into the darker line a wash leaves where it dried.
  let washN = 0;
  function wash(areaD, landD, o = {}) {
    const reserve = (o.reserve ?? 4.5) * U, id = 'washClip' + (washN++), coast = (landD || '') + (o.shore || '');
    return `<clipPath id="${id}"><path d="${areaD}"/></clipPath>
    <g style="mix-blend-mode:multiply" opacity="${o.opacity || 0.55}"><g clip-path="url(#${id})">
      ${washImage(o)}
      <path d="${coast}" fill="none" stroke="${C.seaDeep}" stroke-opacity="0.7" stroke-width="${f2(reserve + 4.5 * U)}" stroke-linejoin="round" filter="url(#soften)"/>
      ${landD ? `<path d="${landD}" fill="#fff"/>` : ''}
      <path d="${coast}" fill="none" stroke="#fff" stroke-width="${f2(reserve)}" stroke-linejoin="round"/>
    </g></g>`;
  }

  // Torn pieces: [{ d, paper, small }], each paper's pieces torn together, so where two pieces of one
  // paper meet there is no seam.
  let tornN = 0;
  function torn(pieces, clip) {
    const byKey = new Map();
    for (const p of pieces) {
      if (!p.d) continue;
      // small pieces far apart are torn in separate groups, so no filter works over the whole plate
      const cell = p.small && p.cell ? '#' + p.cell : '';
      const key = (p.small ? 's' : 'b') + '|' + p.paper + cell + '|' + (p.clip || '');
      if (!byKey.has(key)) byKey.set(key, []);
      byKey.get(key).push(p.d);
    }
    let out = '';
    for (const [key, ds] of byKey) {
      const [size, paperKey, pieceClip] = key.split('|');
      const name = paperKey.replace(/#.*/, ''), paper = PAPERS[name], d = ds.join('');
      // each group gets its own filter, working only where the group and the view overlap
      const box = pathBox(d), m = 30 * U, region = intersect([box[0] - m, box[1] - m, box[2] + 2 * m, box[3] + 2 * m], window_());
      if (!region) continue;
      const id = `torn${tornN++}`;
      const f = size === 's' ? { ...paper.f, warp: 1.5, rim: 0.5, fuzz: 0.8, close: 0.3 } : paper.f;
      out += `<defs>${tornFilter(id, { seed: 40, ...f, region: regionAttr(region) })}</defs>`;
      out += `<g filter="url(#${id})"><path d="${d}" fill="${paper.fill}"${pieceClip ? ` clip-path="url(#${pieceClip})"` : ''}/></g>`;
    }
    return clip ? `<g clip-path="url(#${clip})">${out}</g>` : out;
  }

  // The base papers of a plate torn in one pass (one filter over the view, however many papers):
  // the coasts get the white core and the shadow; where two papers meet inland, seam() draws the tear.
  function tornBase(pieces, o = {}) {
    const box = window_(), id = `tornBase${tornN++}`;
    const byKey = new Map();
    for (const p of pieces) {
      if (!p.d) continue;
      const key = p.paper + '|' + (p.clip || '');
      if (!byKey.has(key)) byKey.set(key, []);
      byKey.get(key).push(p.d);
    }
    let paths = '';
    for (const [key, ds] of byKey) {
      const [name, clip] = key.split('|');
      paths += `<path d="${ds.join('')}" fill="${PAPERS[name].fill}"${clip ? ` clip-path="url(#${clip})"` : ''}/>`;
    }
    const f = { ...PAPERS.kraft.f, texColor: '#3a3020', mottle: 0.3, grain: 0.55, mottleF: 0.08, texK: -0.5, texB: 0.29, hiK: 0.75, hiB: -0.45, ...(o.f || {}) };
    return `<defs>${tornFilter(id, { seed: 41, ...f, region: regionAttr(box) })}</defs><g filter="url(#${id})">${paths}</g>`;
  }
  // A tear inland, where one paper meets another: the white core showing along it, a hair of shadow.
  function seam(d) {
    if (!d) return '';
    return `<g fill="none" stroke-linecap="round" stroke-linejoin="round" filter="url(#seamTear)">
      <path d="${d}" stroke="#3a2d18" stroke-opacity="0.28" stroke-width="${f2(1.6 * U)}" transform="translate(${f2(0.45 * U)} ${f2(0.7 * U)})"/>
      <path d="${d}" stroke="#fdfbf4" stroke-width="${f2(1.25 * U)}"/></g>`;
  }

  function pencil(d, o = {}) {
    return `<path d="${d}" transform="translate(${f2((o.dx ?? -1.4) * U)} ${f2((o.dy ?? 1.1) * U)})" fill="none" stroke="${C.pencil}" stroke-width="${f2((o.width || 0.45) * U)}" opacity="${o.opacity || 0.5}" filter="url(#pencil)"${o.dash ? ` stroke-dasharray="${o.dash}"` : ''}/>`;
  }

  // ---------- letterpress ----------
  // One label: x, y, the string and its setting, in points at rest (size, tracking in em).
  function textEl(x, y, str, o = {}) {
    const size = (o.size || 10) * U, track = o.track || 0, anchor = o.anchor || 'middle';
    const dx = anchor === 'middle' ? track * size / 2 : 0;
    const fam = o.family || 'CG';
    const tr = o.rotate ? ` transform="rotate(${f2(o.rotate)} ${f1(x)} ${f1(y)})"` : '';
    return `<text x="${f2(x + dx)}" y="${f2(y)}" font-family="${fam}" font-size="${f2(size)}"${o.italic ? ' font-style="italic"' : ''} letter-spacing="${f2(track * size)}" text-anchor="${anchor}"${o.fill ? ` fill="${o.fill}"` : ''}${o.opacity ? ` opacity="${o.opacity}"` : ''}${tr}>${esc(str)}</text>`;
  }
  // Labels set together: the impression first, then the inked type.
  function letterpress(items) {
    const t = items.map(i => textEl(...i)).join('');
    return `<g filter="url(#deboss)" fill="#000">${t}</g><g filter="url(#press)" fill="${C.ink}" style="mix-blend-mode:multiply">${t}</g>`;
  }

  // ---------- stamps: a spot colour with grain, the key line printed a touch off register ----------
  function stamp(colour, ink, dx = 0.5, dy = 0.4) {
    return `<g style="mix-blend-mode:multiply" filter="url(#riso)">${colour}</g>
      <g style="mix-blend-mode:multiply" filter="url(#inkline)" transform="translate(${f2(dx * U)} ${f2(dy * U)})">${ink}</g>`;
  }

  return { W, H, U, C, PAPERS, VIEW, setView, rng, f1, f2, esc, defs, sheet, wash, torn, tornBase, seam, pencil, textEl, letterpress, stamp };
})();
window.PA = PA;

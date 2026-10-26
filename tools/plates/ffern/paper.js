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
    return `<filter id="${id}" x="-3%" y="-3%" width="106%" height="106%" color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="${f2(0.05 / U / k * 100) / 100}" numOctaves="3" seed="${s}" result="warp"/>
    <feMorphology in="SourceGraphic" operator="dilate" radius="${f2(close)}" result="cl1"/>
    <feMorphology in="cl1" operator="erode" radius="${f2(close)}" result="cl2"/>
    <feDisplacementMap in="cl2" in2="warp" scale="${f2(warp)}" xChannelSelector="R" yChannelSelector="G" result="shape"/>
    <feTurbulence type="fractalNoise" baseFrequency="${f2(0.9 / U / k * 100) / 100}" numOctaves="2" seed="${s + 1}" result="fuzzN"/>
    <feDisplacementMap in="shape" in2="fuzzN" scale="${f2(fuzz)}" xChannelSelector="R" yChannelSelector="G" result="face"/>
    <feMorphology in="shape" operator="dilate" radius="${f2(rim)}" result="fat"/>
    <feTurbulence type="fractalNoise" baseFrequency="${f2(0.035 / U / k * 1000) / 1000}" numOctaves="2" seed="${s + 2}" result="rimN"/>
    <feDisplacementMap in="fat" in2="rimN" scale="${f2(rim * 2.6)}" xChannelSelector="G" yChannelSelector="R" result="fat2"/>
    <feTurbulence type="fractalNoise" baseFrequency="${f2(1.6 / U / k * 100) / 100}" numOctaves="1" seed="${s + 3}" result="hairN"/>
    <feDisplacementMap in="fat2" in2="hairN" scale="${f2(2.2 * U * k)}" xChannelSelector="R" yChannelSelector="G" result="core"/>
    <feFlood flood-color="#fdfbf4"/>
    <feComposite in2="core" operator="in" result="rimC"/>
    <feGaussianBlur in="core" stdDeviation="${f2(0.9 * U)}" result="sh0"/>
    <feOffset in="sh0" dx="${f2(0.5 * U)}" dy="${f2(0.8 * U)}" result="sh1"/>
    <feFlood flood-color="#3a2d18" flood-opacity="${o.shadow || 0.28}"/>
    <feComposite in2="sh1" operator="in" result="shadow"/>
    <feTurbulence type="fractalNoise" baseFrequency="${f2((o.mottleF || 0.03) / U * 1000) / 1000}" numOctaves="3" seed="${s + 4}" result="mot"/>
    <feTurbulence type="fractalNoise" baseFrequency="${o.grainF || '0.24 0.12'}" numOctaves="2" seed="${s + 5}" result="grn"/>
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
    kraft: { fill: C.kraft, f: { grainF: '0.38 0.21', grain: 0.42, mottle: 0.58, texK: -0.7, texB: 0.4, hiK: 0.7, hiB: -0.42 } },
    kraftLight: { fill: C.kraftLight, f: { grainF: '0.38 0.21', grain: 0.42, mottle: 0.58, texK: -0.65, texB: 0.37, hiK: 0.7, hiB: -0.42 } },
    sage: { fill: C.sage, f: { texColor: '#2c3418', mottle: 0.7, grain: 0.3 } },
    sageLight: { fill: C.sageLight, f: { texColor: '#2c3418', mottle: 0.66, grain: 0.32, texK: -0.8, texB: 0.45 } },
    washi: { fill: C.washi, f: { opacity: 0.9, texColor: '#4a4a40', mottle: 0.5, grain: 0.5, texK: -0.7, texB: 0.36, rim: 1.0, warp: 3.2, close: 0.6 } },
    ochre: { fill: C.ochre, f: { texColor: '#3a2a08', mottle: 0.7, grain: 0.35 } },
    clay: { fill: C.clay, f: { texColor: '#3a1e10', mottle: 0.62, grain: 0.38, grainF: '0.28 0.19' } },
    heather: { fill: C.heather, f: { texColor: '#2a1a18', mottle: 0.7, grain: 0.3 } },
  };

  function defs(extra = '') {
    let torn = '', seed = 40;
    for (const [name, p] of Object.entries(PAPERS)) {
      torn += tornFilter(`torn-${name}`, { seed, ...p.f });
      torn += tornFilter(`tornS-${name}`, { seed: seed + 7, ...p.f, warp: 1.5, rim: 0.5, fuzz: 0.8, close: 0.3 });
      seed += 10;
    }
    return `<defs>
  <filter id="paperTex" x="0" y="0" width="${W}" height="${H}" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="0.0022" numOctaves="4" seed="11" result="cloud"/>
    <feColorMatrix in="cloud" type="matrix" values="0 0 0 0 0.45  0 0 0 0 0.36  0 0 0 0 0.2  0.5 0 0 0 -0.19" result="cloudA"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.11" numOctaves="2" seed="4" result="tooth"/>
    <feDiffuseLighting in="tooth" surfaceScale="${f2(0.9 * U)}" diffuseConstant="1.05" lighting-color="#fff" result="lit"><feDistantLight azimuth="235" elevation="58"/></feDiffuseLighting>
    <feColorMatrix in="lit" type="matrix" values="0 0 0 0 0.3  0 0 0 0 0.24  0 0 0 0 0.14  -0.2 0 0 0 0.13" result="toothA"/>
    <feMerge><feMergeNode in="cloudA"/><feMergeNode in="toothA"/></feMerge>
  </filter>

  <!-- watercolour, laid on a group whose white is water and whose black is land: the wash stops a
       few points short of each coast, unevenly; two pigments run together wet in wet; it deepens away
       from the shore, pools, dries darker at its edges and granulates in the paper's tooth -->
  <filter id="wash" x="0" y="0" width="${W}" height="${H}" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
    <feColorMatrix in="SourceGraphic" type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  1 0 0 0 0" result="A0"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.014" numOctaves="3" seed="21" result="rn"/>
    <feDisplacementMap in="A0" in2="rn" scale="${f2(5 * U)}" xChannelSelector="R" yChannelSelector="G" result="A1"/>
    <feGaussianBlur in="A1" stdDeviation="0.8" result="A"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.0011" numOctaves="2" seed="13" result="mixN"/>
    <feColorMatrix in="mixN" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 2.6 0 0 -1.05" result="mixA"/>
    <feGaussianBlur in="A" stdDeviation="${f2(26 * U)}" result="farB"/>
    <feComponentTransfer in="farB" result="far"><feFuncA type="linear" slope="1.6" intercept="-0.45"/></feComponentTransfer>
    <feFlood flood-color="${C.seaA}" result="pigA"/>
    <feFlood flood-color="${C.seaB}"/>
    <feComposite in2="mixA" operator="in" result="pigB"/>
    <feFlood flood-color="${C.seaDeep}"/>
    <feComposite in2="far" operator="in" result="pigD"/>
    <feMerge result="pig"><feMergeNode in="pigA"/><feMergeNode in="pigB"/><feMergeNode in="pigD"/></feMerge>
    <feTurbulence type="fractalNoise" baseFrequency="0.0022" numOctaves="4" seed="8" result="pool"/>
    <feColorMatrix in="pool" type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  1.5 0 0 0 -0.22" result="poolA"/>
    <feComposite in="pig" in2="A" operator="in" result="pigS"/>
    <feComposite in="pigS" in2="poolA" operator="arithmetic" k1="0.55" k2="0.45" k3="0" k4="0" result="body"/>
    <feGaussianBlur in="A" stdDeviation="${f2(0.7 * U)}" result="blurN"/>
    <feComposite in="A" in2="blurN" operator="arithmetic" k1="0" k2="3" k3="-3" k4="0" result="edgeN"/>
    <feGaussianBlur in="A" stdDeviation="${f2(3.5 * U)}" result="blurW"/>
    <feComposite in="A" in2="blurW" operator="arithmetic" k1="0" k2="0.6" k3="-0.6" k4="0" result="edgeW"/>
    <feMerge result="edgeA0"><feMergeNode in="edgeN"/><feMergeNode in="edgeW"/></feMerge>
    <feComposite in="edgeA0" in2="A" operator="in" result="edgeA"/>
    <feFlood flood-color="${C.seaDeep}" flood-opacity="0.8"/>
    <feComposite in2="edgeA" operator="in" result="edge"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.28" numOctaves="2" seed="5" result="gr"/>
    <feColorMatrix in="gr" type="matrix" values="0 0 0 0 0.2  0 0 0 0 0.3  0 0 0 0 0.34  1.6 0 0 0 -0.92" result="grA"/>
    <feComposite in="grA" in2="A" operator="in" result="gran"/>
    <feMerge><feMergeNode in="body"/><feMergeNode in="edge"/><feMergeNode in="gran"/></feMerge>
  </filter>

  <!-- a river or a lake's thread: the same pigment, laid with one stroke of the brush -->
  <filter id="brush" x="-2%" y="-2%" width="104%" height="104%" color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="0.05" numOctaves="2" seed="27" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="${f2(1.6 * U)}" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feGaussianBlur in="d" stdDeviation="${f2(0.5 * U)}" result="b"/>
    <feComposite in="d" in2="b" operator="arithmetic" k1="0" k2="1.7" k3="-0.7" k4="0"/>
  </filter>

  <filter id="pencil" x="-2%" y="-2%" width="104%" height="104%">
    <feTurbulence type="fractalNoise" baseFrequency="0.31" numOctaves="1" seed="31" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="${f2(1.1 * U)}" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feColorMatrix in="n" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  1.8 0 0 0 -0.45" result="skip"/>
    <feComposite in="d" in2="skip" operator="in"/>
  </filter>

  <!-- letterpress: the type bites a little ragged, the ink sits unevenly and squeezes darker at its edges -->
  <filter id="press" x="-2%" y="-2%" width="104%" height="104%" color-interpolation-filters="sRGB">
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
  <filter id="deboss" x="-2%" y="-2%" width="104%" height="104%">
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
  <filter id="inkline" x="-5%" y="-5%" width="110%" height="110%">
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
  <filter id="riso" x="-2%" y="-2%" width="104%" height="104%" color-interpolation-filters="sRGB">
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
  ${torn}
  ${extra}
</defs>`;
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
    return `<rect width="${W}" height="${H}" fill="${C.paper}"/><rect width="${W}" height="${H}" filter="url(#paperTex)"/>
      <g fill="none" stroke-linecap="round"><path d="${fib}" stroke="${C.fibreLight}" stroke-width="${f2(0.4 * U)}" opacity="0.7"/><path d="${fibD}" stroke="${C.fibreDark}" stroke-width="${f2(0.3 * U)}" opacity="0.4"/>${fleck}</g>`;
  }

  // Watercolour over an area (white) with the land (black, stroked for the reserve) kept dry.
  function wash(areaD, landD, o = {}) {
    const reserve = (o.reserve ?? 4.5) * U;
    return `<g style="mix-blend-mode:multiply" opacity="${o.opacity || 0.55}"><g filter="url(#wash)">
      <path d="${areaD}" fill="#fff"/>
      ${landD ? `<path d="${landD}" fill="#000" stroke="#000" stroke-width="${f2(reserve)}" stroke-linejoin="round"/>` : ''}
    </g></g>`;
  }

  // Torn pieces: [{ d, paper, small }], each paper's pieces torn together, so where two pieces of one
  // paper meet there is no seam.
  function torn(pieces) {
    const byKey = new Map();
    for (const p of pieces) {
      if (!p.d) continue;
      const key = (p.small ? 'tornS-' : 'torn-') + p.paper + (p.clip ? '|' + p.clip : '');
      if (!byKey.has(key)) byKey.set(key, []);
      byKey.get(key).push(p.d);
    }
    let out = '';
    for (const [key, ds] of byKey) {
      const [filter, clip] = key.split('|');
      const paper = PAPERS[filter.replace(/^tornS?-/, '')];
      out += `<g filter="url(#${filter})"><path d="${ds.join('')}" fill="${paper.fill}"${clip ? ` clip-path="url(#${clip})"` : ''}/></g>`;
    }
    return out;
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

  return { W, H, U, C, PAPERS, rng, f1, f2, esc, defs, sheet, wash, torn, pencil, textEl, letterpress, stamp };
})();
window.PA = PA;

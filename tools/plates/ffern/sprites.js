// The Paper Almanac's living things: painted in gouache on cream paper and cut out with scissors.
// Each is drawn in points, in its own frame, and placed on a plate with a transform; its filters are
// the point-sized ones defined here, so a sprite looks the same at any scale it is placed at.
'use strict';
const Sprites = (() => {
  const { C, f1, f2 } = PA;
  const ptDefs = () => `<defs>
  <filter id="gouache-pt" x="-10%" y="-10%" width="120%" height="120%" color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="0.6" numOctaves="2" seed="23" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="0.7" xChannelSelector="R" yChannelSelector="G" result="g"/>
    <feGaussianBlur in="g" stdDeviation="0.7" result="gb"/>
    <feComposite in="g" in2="gb" operator="arithmetic" k1="0" k2="1.5" k3="-0.5" k4="0" result="edged"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.09" numOctaves="3" seed="29" result="m"/>
    <feColorMatrix in="m" type="matrix" values="0 0 0 0 1  0 0 0 0 0.97  0 0 0 0 0.9  1.1 0 0 0 -0.5" result="mA"/>
    <feComposite in="mA" in2="g" operator="in" result="light"/>
    <feMerge result="painted"><feMergeNode in="edged"/><feMergeNode in="light"/></feMerge>
    <feComposite in="painted" in2="g" operator="in"/>
  </filter>
  <filter id="inkline-pt" x="-10%" y="-10%" width="120%" height="120%">
    <feTurbulence type="fractalNoise" baseFrequency="0.7" numOctaves="2" seed="37" result="n"/>
    <feDisplacementMap in="SourceGraphic" in2="n" scale="0.8" xChannelSelector="R" yChannelSelector="G" result="d"/>
    <feTurbulence type="fractalNoise" baseFrequency="0.25" numOctaves="2" seed="38" result="w"/>
    <feColorMatrix in="w" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  -2 0 0 0 1.9" result="wA"/>
    <feComposite in="d" in2="wA" operator="in"/>
  </filter>
  <filter id="cutout-pt" x="-15%" y="-15%" width="130%" height="130%" color-interpolation-filters="sRGB">
    <feMorphology in="SourceAlpha" operator="dilate" radius="1.05" result="m0"/>
    <feGaussianBlur in="m0" stdDeviation="0.5" result="m1"/>
    <feComponentTransfer in="m1" result="m2"><feFuncA type="linear" slope="3" intercept="-1"/></feComponentTransfer>
    <feFlood flood-color="#f1e9d6"/>
    <feComposite in2="m2" operator="in" result="margin"/>
    <feGaussianBlur in="m2" stdDeviation="0.9" result="s0"/>
    <feOffset in="s0" dx="0.6" dy="1" result="s1"/>
    <feFlood flood-color="#2e2210" flood-opacity="0.32"/>
    <feComposite in2="s1" operator="in" result="shadow"/>
    <feMerge><feMergeNode in="shadow"/><feMergeNode in="margin"/><feMergeNode in="SourceGraphic"/></feMerge>
  </filter>
  <filter id="riso-pt" x="-10%" y="-10%" width="120%" height="120%" color-interpolation-filters="sRGB">
    <feTurbulence type="fractalNoise" baseFrequency="1.3" numOctaves="1" seed="41" result="n"/>
    <feColorMatrix in="n" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  -1.3 0 0 0 1.5" result="nA"/>
    <feComposite in="SourceGraphic" in2="nA" operator="in"/>
  </filter>
</defs>`;

  // Places art drawn in points: its local point (cx, cy) lands at plate (x, y), scaled by s (plate
  // pixels per point).
  const place = (art, x, y, s, cx, cy, extra = '') => `<g transform="translate(${f2(x)} ${f2(y)}) scale(${f2(s)}) translate(${f2(-cx)} ${f2(-cy)})"${extra}>${art}</g>`;

  // ---------- curves ----------
  function cubicPts(segs, per = 40) {
    const out = [];
    for (const [a, b, c, d] of segs) {
      for (let i = out.length ? 1 : 0; i <= per; i++) {
        const t = i / per, u = 1 - t;
        out.push([u * u * u * a[0] + 3 * u * u * t * b[0] + 3 * u * t * t * c[0] + t * t * t * d[0],
                  u * u * u * a[1] + 3 * u * u * t * b[1] + 3 * u * t * t * c[1] + t * t * t * d[1]]);
      }
    }
    const L = [0];
    for (let i = 1; i < out.length; i++) L.push(L[i - 1] + Math.hypot(out[i][0] - out[i - 1][0], out[i][1] - out[i - 1][1]));
    const total = L[L.length - 1], n = Math.ceil(total / 0.8), res = [];
    let j = 0;
    for (let k = 0; k <= n; k++) {
      const s = total * k / n;
      while (j < L.length - 2 && L[j + 1] < s) j++;
      const t = (s - L[j]) / Math.max(1e-6, L[j + 1] - L[j]);
      res.push([out[j][0] + (out[j + 1][0] - out[j][0]) * t, out[j][1] + (out[j + 1][1] - out[j][1]) * t]);
    }
    return { pts: res, length: total };
  }
  const ptsD = (arr, close) => 'M' + arr.map(p => f2(p[0]) + ' ' + f2(p[1])).join('L') + (close ? 'Z' : '');
  const leafD = (bx, by, ang, L, Wd) => {
    const c = Math.cos(ang), s = Math.sin(ang), Q = (u, v) => [bx + c * u - s * v, by + s * u + c * v];
    const tip = Q(L, 0), a1 = Q(L * 0.42, -Wd), a2 = Q(L * 0.42, Wd), mid = Q(L * 0.8, 0);
    return { d: `M${f2(bx)} ${f2(by)}Q${f2(a1[0])} ${f2(a1[1])} ${f2(tip[0])} ${f2(tip[1])}Q${f2(a2[0])} ${f2(a2[1])} ${f2(bx)} ${f2(by)}Z`, vein: `M${f2(bx)} ${f2(by)}L${f2(mid[0])} ${f2(mid[1])}` };
  };

  // ---------- the sea serpent, after the Carta Marina, as a folk painting ----------
  // Local frame: about x 118..330, y 158..270; the waterline is y 249.
  function serpent(id = 'serp') {
    const WL = 249;
    const segs = [
      [[156, 188], [168, 188], [178.5, 198], [180, 215]],
      [[180, 215], [181.5, 231], [180, 243], [183, 257]],
      [[183, 257], [186, 267], [197, 267], [199.5, 254]],
      [[199.5, 254], [200, 212], [236, 201], [247.5, 254]],
      [[247.5, 254], [250, 267], [261, 267], [263.5, 254]],
      [[263.5, 254], [264, 228], [285, 222], [291.5, 254]],
      [[291.5, 254], [294, 265], [302, 265], [304.5, 253]],
      [[304.5, 253], [306.5, 236], [312, 228], [319, 228.5]],
      [[319, 228.5], [327.5, 229.5], [327.5, 241.5], [319.5, 241.5]],
      [[319.5, 241.5], [313.5, 241.5], [313.8, 234], [319, 234.6]],
    ];
    const { pts } = cubicPts(segs);
    const n = pts.length;
    const width = i => { const t = i / (n - 1); return t < 0.08 ? 10.4 : t < 0.7 ? 11.2 - (t - 0.08) * 1.6 : Math.max(1.3, 10.2 * Math.max(0, 1 - (t - 0.7) / 0.3) ** 0.85); };
    const nor = pts.map((p, i) => { const a = pts[Math.max(0, i - 2)], b = pts[Math.min(n - 1, i + 2)]; const dx = b[0] - a[0], dy = b[1] - a[1], l = Math.hypot(dx, dy) || 1; return [-dy / l, dx / l]; });
    const tan = pts.map((p, i) => [nor[i][1], -nor[i][0]]);
    const off = (i, k) => [pts[i][0] + nor[i][0] * k, pts[i][1] + nor[i][1] * k];
    const side = k => pts.map((p, i) => off(i, width(i) / 2 * k));
    const belly = side(1), back = side(-1);
    const bodyD = ptsD(belly.concat(back.slice().reverse()), true);
    const bellyLine = side(0.3);
    const bellyD = ptsD(belly.concat(bellyLine.slice().reverse()), true);
    const ridgeD = ptsD(back.concat(side(-0.42).reverse()), true);
    // the crest: a scalloped fin like sea-kale, swept back, tallest where the back faces the sky
    const crestH = i => { const up = Math.max(0, nor[i][1] - 0.15) / 0.85, ph = (i % 11) / 11; return (1.4 + 5.2 * Math.sin(ph * Math.PI) ** 0.8) * Math.min(1, up * 1.5); };
    const crestOut = [], crestIn = [];
    let veins = '';
    for (let i = 8; i < n * 0.88; i++) {
      const w = width(i) / 2, h = crestH(i);
      const o = off(i, -(w + h)); crestOut.push([o[0] + tan[i][0] * h * 0.55, o[1] + tan[i][1] * h * 0.55]);
      crestIn.push(off(i, -w * 0.85));
      if (i % 11 === 5 && h > 1.2) { const a = off(i, -w * 0.9), e = crestOut[crestOut.length - 1]; veins += `M${f2(a[0])} ${f2(a[1])}L${f2(a[0] + (e[0] - a[0]) * 0.82)} ${f2(a[1] + (e[1] - a[1]) * 0.82)}`; }
    }
    const crestD = ptsD(crestOut.concat(crestIn.reverse()), true);
    let scales = '';
    for (let i = 10, row = 0; i < n * 0.9; i += 4, row++) {
      for (const k of (row % 2 ? [-0.66, -0.2] : [-0.43])) {
        const w = width(i) / 2, c = off(i, k * w), r = w * 0.26, t = tan[i], q = nor[i];
        const a = [c[0] + q[0] * r, c[1] + q[1] * r], b = [c[0] - q[0] * r, c[1] - q[1] * r], m = [c[0] + t[0] * r * 2, c[1] + t[1] * r * 2];
        scales += `M${f2(a[0])} ${f2(a[1])}Q${f2(m[0])} ${f2(m[1])} ${f2(b[0])} ${f2(b[1])}`;
      }
    }
    let dots = '';
    for (let i = 16; i < n * 0.84; i += 26) {
      if (pts[i][1] > WL - 3) continue;
      const c = off(i, -width(i) * 0.04), t = tan[i], r = width(i) * 0.07 + 0.25;
      for (const k of [-1.15, 0, 1.15]) dots += `<circle cx="${f2(c[0] + t[0] * k * 2.1)}" cy="${f2(c[1] + t[1] * k * 2.1)}" r="${f2(r)}"/>`;
    }
    let ticks = '';
    for (let i = 8; i < n - 10; i += 4) {
      const w = width(i) / 2, a = off(i, w * 0.45), b = off(i, w * 0.86), m = off(i, w * 0.66), t = tan[i];
      ticks += `M${f2(a[0])} ${f2(a[1])}Q${f2(m[0] + t[0] * 1.1)} ${f2(m[1] + t[1] * 1.1)} ${f2(b[0])} ${f2(b[1])}`;
    }
    const headD = `M4 -5.2C-2 -9.5 -10 -11 -16 -9C-20 -7.8 -23 -6.2 -27 -6.4C-31 -6.6 -34 -5.4 -34.5 -3.4C-35 -1.8 -33.6 -0.6 -31.5 -0.8C-27 -1.2 -21 -0.2 -15 0.4C-21 1.6 -27 2.4 -30 3.6C-29.5 6.5 -22 8.6 -12 8.2C-5 7.9 0 6.6 4 5.2Z`;
    const headInk = headD.slice(0, -1);
    const jawD = `M-15 0.4C-21 1.6 -27 2.4 -30 3.6C-29.5 6.5 -22 8.6 -12 8.2C-5 7.9 0 6.6 4 5.2L4 1.6C-3 1.3 -9 0.9 -15 0.4Z`;
    const brow = `M4 -5.2C-2 -9.5 -10 -11 -16 -9C-20 -7.8 -23 -6.2 -27 -6.4L-27 -4.6C-22 -4.6 -17 -6.4 -12 -6.6C-6 -6.8 0 -4.4 4 -2.6Z`;
    const mane = [leafD(-9, -8.6, -2.05, 9.5, 2.5), leafD(-5, -8.2, -1.7, 11, 2.7), leafD(-1, -7, -1.32, 10.5, 2.6), leafD(2.5, -5.4, -0.98, 9, 2.4), leafD(5, -3.2, -0.66, 7.2, 2.1)];
    const headG = inner => `<g transform="translate(156 188)">${inner}</g>`;
    const clip = `<clipPath id="${id}-water"><rect x="0" y="0" width="400" height="${WL}"/></clipPath>`;
    const colour = `${clip}<g filter="url(#gouache-pt)">
        <g clip-path="url(#${id}-water)"><path d="${crestD}" fill="${C.sage}"/><path d="${bodyD}" fill="${C.rust}"/>
          <path d="${ridgeD}" fill="${C.rustDeep}" opacity="0.45"/><path d="${scales}" fill="none" stroke="${C.rustDeep}" stroke-width="0.42" opacity="0.75"/>
          <path d="${bellyD}" fill="#e2c28a"/></g>
        ${headG(`<g fill="${C.sage}">${mane.map(m => `<path d="${m.d}"/>`).join('')}</g><path d="${headD}" fill="${C.rust}"/><path d="${brow}" fill="${C.rustDeep}" opacity="0.5"/><path d="${jawD}" fill="#e2c28a"/>`)}
      </g>`;
    const ink = `<g fill="none" stroke="${C.ink}" stroke-width="0.55" stroke-linejoin="round" stroke-linecap="round" filter="url(#inkline-pt)" transform="translate(0.5 0.4)">
        <g clip-path="url(#${id}-water)"><path d="${ptsD(belly)}"/><path d="${ptsD(back)}"/><path d="${ptsD(bellyLine)}" stroke-width="0.35"/>
          <path d="${ticks}" stroke-width="0.3" opacity="0.55"/><path d="${ptsD(crestOut)}" stroke-width="0.38"/><path d="${veins}" stroke-width="0.3"/></g>
        ${headG(`<path d="${headInk}"/><path d="M-15 0.4C-12.6 0.7 -10.6 1.1 -9.2 1.8" stroke-width="0.45"/>${mane.map(m => `<path d="${m.d}" stroke-width="0.38"/><path d="${m.vein}" stroke-width="0.32"/>`).join('')}`)}
      </g>
      <g clip-path="url(#${id}-water)" fill="#f7efdc">${dots}</g>
      ${headG(`<circle cx="-17.6" cy="-4.4" r="2.5" fill="#f7efdc"/><circle cx="-18.1" cy="-4.1" r="1.25" fill="${C.ink}"/><circle cx="-18.5" cy="-4.6" r="0.35" fill="#fff"/>
        <path d="M-20.4 -5.6C-18.8 -7.6 -16.2 -7.4 -15 -5.6C-16.6 -6.2 -18.6 -6.3 -20.4 -5.6Z" fill="${C.rust}"/>
        <g fill="none" stroke="${C.ink}" stroke-linecap="round"><path d="M-20.4 -5.6C-18.8 -7.6 -16.2 -7.4 -15 -5.6" stroke-width="0.5"/><path d="M-20.1 -4.2A2.5 2.5 0 0 0 -15.2 -3.8" stroke-width="0.35"/><path d="M-21 -7.6l-0.8 -0.9M-19 -8.2l-0.3 -1.1M-16.8 -8l0.2 -1.1" stroke-width="0.32"/></g>
        <circle cx="-31.6" cy="-3.6" r="0.5" fill="${C.ink}"/>`)}`;
    // where it breaks the water: curling crests, and a few swell marks
    const crestLocal = `M0 0.4C2.5 -0.8 5 -4.4 8.4 -3.8C10.8 -3.4 11 -0.6 9 -0.8C7.9 -0.9 8 -2.3 8.9 -2.1`;
    const foamLocal = `M0 0.4C2.5 -0.8 5 -4.4 8.4 -3.8C10.8 -3.4 11 -0.6 9 -0.8L12 1.2Z`;
    let crests = '';
    [183, 199.5, 247.5, 263.5, 291.5, 304.5].forEach((x, k) => {
      const sg = k % 2 ? 1 : -1;
      crests += `<g transform="translate(${f1(x + sg * 1.6)} ${WL}) scale(${sg * 1.25} 1.25)"><path d="${foamLocal}" fill="#fbf8f0" stroke="none"/><path d="${crestLocal}"/><path d="M13 2.2q1.8 -1.2 3.6 0"/></g>`;
    });
    let swell = '';
    for (const [x, y] of [[166, 263], [223, 266], [277, 264], [316, 257], [146, 252]]) swell += `M${x - 4.5} ${y}q2.25 -1.7 4.5 0t4.5 0`;
    return `<g filter="url(#cutout-pt)">${colour}${ink}</g>
      <g fill="none" stroke="${C.ink}" stroke-width="0.5" stroke-linecap="round" filter="url(#inkline-pt)" opacity="0.85">${crests}<path d="${swell}" stroke="#34525c" stroke-width="0.45" opacity="0.7"/></g>`;
  }

  // ---------- a little cog in cut paper, its sail marked with Brendan's cross ----------
  // Local frame: origin at the hull's keel, about x -26..36, y -40..7.
  function ship() {
    const hull = `M-19 -9.5C-16 -2 -9 1.2 0 1.2C9 1.2 15 -2.5 17.5 -8.5L14.5 -7C9 -4.8 -9 -4.8 -16.5 -7Z`;
    const sail = `M-10 -33C-12.5 -25 -12.5 -17 -10 -10L10 -10C8 -17 8 -25 10 -33Z`;
    const flag = `M0.3 -38.5C-3 -39.5 -6 -37.5 -9 -38.3C-6.5 -36.6 -3 -36.4 0.3 -36.2Z`;
    const colour = `<g filter="url(#gouache-pt)"><path d="${sail}" fill="#f1e8d4"/><path d="${hull}" fill="#7d5c3e"/>
      <path d="M-15.5 -5.6C-9 -3.6 9 -3.6 14 -5.6L13.6 -4.2C8 -2.4 -9 -2.4 -14.6 -4.2Z" fill="#a98a60"/><path d="${flag}" fill="${C.rust}"/>
      <g fill="${C.rust}"><rect x="-1.15" y="-29.5" width="2.3" height="15.5"/><rect x="-6.5" y="-24.2" width="13" height="2.3"/></g>
      <circle cx="0" cy="-23.05" r="3.6" fill="none" stroke="${C.rust}" stroke-width="1.1"/></g>`;
    const ink = `<g fill="none" stroke="${C.ink}" stroke-width="0.5" stroke-linejoin="round" stroke-linecap="round" filter="url(#inkline-pt)" transform="translate(0.45 0.35)">
      <path d="${hull}"/><path d="${sail}" stroke-width="0.42"/><path d="M0 -6V-38.5" stroke-width="0.7"/><path d="M-11 -33.3H11M-10.5 -10H10.5" stroke-width="0.55"/>
      <path d="M-16 -3.6C-8 -1.6 8 -1.6 15 -3.8" stroke-width="0.35"/><path d="${flag}" stroke-width="0.35"/></g>`;
    const sea = `<g fill="none" stroke="${C.ink}" stroke-width="0.45" stroke-linecap="round" filter="url(#inkline-pt)" opacity="0.75">
      <path d="M-24 3q3 -2.4 6 -0.8q2 1 4 0M12 3.2q3 -2.2 6 -0.6q1.6 0.8 3.4 0M-6 5.6q2.4 -1.6 4.8 0t4.8 0"/><path d="M20 -2.2q7 1.2 15 3.4M21 0.6q6 1.4 12 3.6M19 3q4 1.2 8 2.8" stroke-width="0.35" opacity="0.7"/></g>`;
    return `<g filter="url(#cutout-pt)">${colour}${ink}</g>${sea}`;
  }



  // ---------- Canada geese, flying south in a V ----------
  // One goose, facing west (its local x runs back to its tail), wings up or down.
  function goose(up) {
    const body = `M-6 0C-3 -2.6 5 -2.8 8 -0.6C9 0.2 9 1 8 1.6C4 3 -3 2.6 -6 0Z`;
    const belly = `M-5 0.9C-2 2.4 4 2.6 7.6 1.4C4 2.9 -3 2.7 -5 0.9Z`;
    const far = up ? `M0.5 -1.8C2.6 -6 6 -10.6 9.4 -13.6C7.6 -9 5.6 -5 4.4 -1.6Z` : `M0.5 1C2.6 4 5.6 7.4 8.6 9.2C7 6.2 5.4 3.2 4.4 0.8Z`;
    const near = up ? `M-1.2 -1.6C-1.2 -6 0.8 -11 3.8 -15.6C4.4 -10.2 4 -5.4 3.2 -1.4Z` : `M-1 0.8C-0.4 4.6 1.6 8.6 4.2 11.4C4.4 7.6 3.8 4 3 0.6Z`;
    const neck = `M-5.4 -0.6C-8.4 -1.2 -11 -1.6 -13.4 -1.8`;
    const colour = `<path d="${far}" fill="#6a5d4e"/><path d="${body}" fill="#8b7b67"/><path d="${belly}" fill="#ddd1ba"/><path d="${near}" fill="#8f806b"/>
      <path d="M8 -0.4L11 -1L11.2 1.2L8 1.4Z" fill="#2e2a20"/><path d="${neck}" stroke="#2e2a20" stroke-width="1.9" fill="none" stroke-linecap="round"/>
      <ellipse cx="-14.6" cy="-1.9" rx="1.9" ry="1.35" fill="#2e2a20"/><ellipse cx="-14.2" cy="-1.3" rx="0.9" ry="0.55" fill="#f4eee0"/><path d="M-16.4 -2.2L-17.9 -1.7L-16.3 -1.3Z" fill="#3a3326"/>`;
    const fl = up ? `M1.4 -3.4L5.4 -8.6M2.2 -2.6L7 -7.2M-0.4 -3.6L1.8 -10M0.8 -3L3 -11.4` : `M1.6 1.8L5.6 6M2.4 1.4L7 4.6M-0.2 2L1.6 7.6M0.8 1.8L2.8 8.8`;
    const ink = `<path d="${far}"/><path d="${body}"/><path d="${near}"/><path d="${fl}" stroke-width="0.25"/>`;
    return { colour, ink };
  }
  // The V: the leader at the origin, flying south-west; about x -20..44, y -42..16 in points.
  function geese() {
    const birds = [[0, 0, 1], [14, -12, 0], [28, -24, 1], [18, -2, 0], [35, -5, 1]];
    let colour = '', ink = '';
    for (const [x, y, up] of birds) {
      const g = goose(up), t = `transform="translate(${x} ${y}) rotate(-16)"`;
      colour += `<g ${t}>${g.colour}</g>`; ink += `<g ${t}>${g.ink}</g>`;
    }
    return `<g filter="url(#cutout-pt)"><g filter="url(#gouache-pt)">${colour}</g>
      <g fill="none" stroke="${C.ink}" stroke-width="0.4" stroke-linejoin="round" filter="url(#inkline-pt)" transform="translate(0.4 0.3)">${ink}</g></g>`;
  }


  // ---------- a sprig of Ohio buckeye: two palmate leaves turning, a split husk and its nut ----------
  // Local frame: the twig's foot at the origin, rising up and to the right; about x -34..62, y -62..6.
  function buckeyeSprig() {
    const leafletD = (L, Wd) => `M0 0C${f2(L * 0.22)} ${f2(-Wd * 0.8)} ${f2(L * 0.62)} ${f2(-Wd)} ${L} 0C${f2(L * 0.62)} ${f2(Wd)} ${f2(L * 0.22)} ${f2(Wd * 0.8)} 0 0Z`;
    const palm = (x, y, ang, size, cols) => {
      let c = '', k = '';
      [-1.1, -0.56, 0, 0.56, 1.1].forEach((da, i) => {
        const L = size * (1 - Math.abs(da) * 0.3), Wd = L * 0.19, t = `transform="translate(${f2(x)} ${f2(y)}) rotate(${f2((ang + da) * 180 / Math.PI)})"`;
        c += `<path ${t} d="${leafletD(L, Wd)}" fill="${cols[i % cols.length]}"/>`;
        k += `<path ${t} d="${leafletD(L, Wd)}"/><path ${t} d="M0 0L${f2(L * 0.88)} 0" stroke-width="0.3"/>`;
        for (let j = 1; j < 5; j++) k += `<path ${t} d="M${f2(L * j / 5.6)} 0l${f2(L * 0.07)} ${f2(-Wd * 0.62)}M${f2(L * j / 5.6)} 0l${f2(L * 0.07)} ${f2(Wd * 0.62)}" stroke-width="0.2"/>`;
      });
      return { c, k };
    };
    const twig = `M0 0C5 -7 12 -14 20 -19C25 -22 29 -25 32 -28`;
    const petA = `M6 -8C2 -11 -2 -14 -6 -16`, petB = `M20 -19C23 -25 25 -30 26 -36`;
    const A = palm(-6, -16, -2.55, 27, [C.ochre, '#b98a3a', '#c9a24e', C.sage, C.ochre]);
    const B = palm(26, -36, -1.2, 25, [C.sage, '#c08a3e', C.sageDeep, C.ochre, C.sage]);
    // two husks at the tip: one closed and warty, one split in three around its glossy nut
    const husk1 = `M36 -26C33 -29 33 -34 37 -36C41 -38 46 -36 47 -32C48 -28 45 -25 41 -24.5C39 -24.3 37.4 -25 36 -26Z`;
    const huskOpen = `M41 -36C40 -41 43 -45 47 -45.6C51.6 -46 55.4 -43 55 -38.6L52.6 -40.2C52 -42.6 49 -43.6 46.6 -42.8C44.4 -42 43.2 -39.8 43.6 -37.4Z`;
    const huskLow = `M43.6 -37.4C44.8 -33.6 49.6 -32.6 52.6 -35L55 -38.6C56 -34 52.6 -30.6 48 -30.8C44.4 -31 42 -33.4 41 -36Z`;
    const nut = `M44.4 -38.6C44.2 -41.6 47 -43.4 49.8 -42.6C52.6 -41.8 53.6 -38.8 52 -36.6C50.4 -34.4 46.8 -34.4 45.4 -36.2C44.8 -36.9 44.5 -37.7 44.4 -38.6Z`;
    const colour = `<g filter="url(#gouache-pt)"><path d="${twig}${petA}${petB}" stroke="#6b5236" stroke-width="1.6" fill="none" stroke-linecap="round"/>${A.c}${B.c}
      <path d="${husk1}" fill="#a49a5e"/><path d="${huskOpen}" fill="#b2a66a"/><path d="${huskLow}" fill="#9a9058"/><path d="${nut}" fill="#5a3620"/>
      <ellipse cx="48.6" cy="-37.6" rx="2.6" ry="1.9" fill="#d6c099"/><ellipse cx="46.6" cy="-40.6" rx="1.1" ry="0.7" fill="#f2e6cc" opacity="0.85"/></g>`;
    let warts = '';
    for (const [x, y] of [[38.6, -33], [42.4, -35.2], [44.6, -30.6], [39.6, -28.2], [44, -27], [36.4, -30], [47.2, -44], [51.8, -44], [53.4, -34]]) warts += `<circle cx="${x}" cy="${y}" r="0.4"/>`;
    const ink = `<g fill="none" stroke="${C.ink}" stroke-width="0.4" stroke-linecap="round" stroke-linejoin="round" filter="url(#inkline-pt)" transform="translate(0.4 0.3)">${A.k}${B.k}
      <path d="${twig}" stroke-width="0.42"/><path d="${husk1}"/><path d="${huskOpen}"/><path d="${huskLow}"/><path d="${nut}" stroke-width="0.32"/></g>
      <g fill="${C.ink}" opacity="0.7">${warts}</g>`;
    return `<g filter="url(#cutout-pt)">${colour}${ink}</g>`;
  }
  // ---------- a cardinal, perched, facing west ----------
  // Local frame: its feet at the origin; about x -16..12, y -18..2.
  function cardinal() {
    const body = `M-6 -6.5C-7.6 -10 -6 -13.6 -2.4 -14.2C-1.2 -16.6 1.4 -17.6 2.2 -15.6C3.6 -14.4 4.2 -12 3.4 -9.6C5.2 -8.4 7 -6.4 9 -3.4L11.4 0.8L9.4 1.2L6.6 -2.4C4.4 -1.2 1 -0.6 -1.6 -1.4C-4.6 -2.4 -5.8 -4.2 -6 -6.5Z`;
    const wing = `M-0.6 -9.4C2.4 -9.6 5.4 -7.8 7.6 -4.2C4.8 -4.4 1.6 -5.2 -0.6 -7Z`;
    const mask = `M-6.2 -11.6C-5 -12.4 -3.4 -12.4 -2.4 -11.4C-3 -10 -4.4 -9.2 -6 -9.6Z`;
    const beak = `M-6.1 -11.6L-8.8 -10.6L-6 -9.6Z`;
    const colour = `<g filter="url(#gouache-pt)"><path d="${body}" fill="#b5392b"/><path d="${wing}" fill="#8e2a22"/><path d="${mask}" fill="#2e2a20"/><path d="${beak}" fill="#e08a4a"/>
      <path d="M-1 -1.2L-1.6 1.6M1.6 -1L1.8 1.6" stroke="#5a4630" stroke-width="0.5"/></g>
      <circle cx="-4.4" cy="-11.6" r="0.55" fill="#f4eee0"/>`;
    const ink = `<g fill="none" stroke="${C.ink}" stroke-width="0.4" stroke-linejoin="round" filter="url(#inkline-pt)" transform="translate(0.35 0.3)"><path d="${body}"/><path d="${wing}" stroke-width="0.3"/><path d="M1 -8.4L5.6 -5M1.4 -7.2L4.6 -5" stroke-width="0.22"/></g>`;
    return `<g filter="url(#cutout-pt)">${colour}${ink}</g>`;
  }


  // ---------- fallen leaves: a maple and an oak, painted and cut out ----------
  // Local frames: the stalk's end at the origin, the leaf about 40 points long.
  function mapleLeaf() {
    const d = `M0 0L0.6 -6C-3 -5 -8 -6.6 -12 -5.4C-10.6 -8 -11.6 -10.4 -13.6 -12C-10.8 -12.4 -9 -13.4 -8.4 -15.4C-6.4 -14.2 -4.4 -14.4 -3.4 -16C-4.6 -19.4 -4.2 -23.6 -2.8 -26.4C-1.6 -24.6 -0.6 -24.6 0.4 -26C1.2 -30 2.4 -33.4 4.4 -36C6 -33 7 -29.6 7.4 -26C8.6 -27 9.8 -26.8 10.8 -25.8C11.8 -22.6 11.8 -19.2 10.6 -16.4C12 -15.4 13.6 -15.6 15 -16.6C15.2 -14.6 16.6 -13.2 18.6 -12.6C16.6 -11 15.8 -8.6 16.8 -6.2C12.8 -7.8 8.6 -6.8 5.2 -6.2L1.8 -6Z`;
    const veins = `M0.6 -6L4.4 -33M1.6 -7.6L-10.6 -12.2M2 -8.4L-1.6 -23.6M3.4 -8.8L9.8 -22.6M3 -7.4L15.4 -12.4`;
    return `<g filter="url(#cutout-pt)"><g filter="url(#gouache-pt)"><path d="${d}" fill="#c4502c"/><path d="M4.4 -33C2 -24 1 -14 1.6 -7L4 -7C6 -14 6.6 -24 4.4 -33Z" fill="#d8803a" opacity="0.55"/></g>
      <g fill="none" stroke="${C.ink}" stroke-width="0.4" stroke-linejoin="round" filter="url(#inkline-pt)" transform="translate(0.4 0.3)"><path d="${d}"/><path d="${veins}" stroke-width="0.28" opacity="0.8"/><path d="M0.6 -6L0 0" stroke-width="0.7"/></g></g>`;
  }
  function oakLeaf() {
    const d = `M0 0L0.4 -4C-2.6 -4.6 -4.6 -6.6 -3.6 -8.6C-6 -9.6 -7.2 -12 -5.4 -13.6C-7.2 -15.6 -6.8 -18.4 -4.4 -19C-5.6 -21.6 -4.4 -24.4 -1.8 -24.4C-2 -27.4 -0.4 -30 1.6 -30.6C3.8 -30 5 -27.4 4.8 -24.4C7.4 -24.4 8.6 -21.6 7.4 -19C9.8 -18.4 10.2 -15.6 8.4 -13.6C10.2 -12 9 -9.6 6.6 -8.6C7.6 -6.6 5.6 -4.6 2.6 -4L2 -4Z`;
    const veins = `M1.2 -4L1.6 -28.6M1.3 -9L-3.4 -11.4M1.4 -14L-3.6 -16.6M1.5 -19L-2.6 -21.8M1.3 -9L5.8 -11.2M1.4 -14L6.4 -16.4M1.5 -19L5.2 -21.6`;
    return `<g filter="url(#cutout-pt)"><g filter="url(#gouache-pt)"><path d="${d}" fill="#b98a3a"/><path d="M-3 -12C-1 -10 2 -12 4 -16C2 -20 -1 -19 -3 -12Z" fill="#9a6a2c" opacity="0.4"/></g>
      <g fill="none" stroke="${C.ink}" stroke-width="0.4" stroke-linejoin="round" filter="url(#inkline-pt)" transform="translate(0.4 0.3)"><path d="${d}"/><path d="${veins}" stroke-width="0.28" opacity="0.8"/><path d="M0.4 -4L0 0" stroke-width="0.7"/></g></g>`;
  }


  // ---------- a great blue heron, standing in the shallows, facing west ----------
  // Local frame: its feet at the origin; about x -14..16, y -46..4.
  function heron() {
    const body = `M-6 -17C-4 -23 5.6 -25 10 -20.4C12.2 -18.2 13.6 -14.8 11.6 -12.4C7 -11 -1.4 -12 -6 -17Z`;
    const wing = `M-3 -19.6C1 -22.6 7 -22.4 10.4 -19C11.6 -17.4 12 -15.2 11 -13.6C6.6 -14.6 1.4 -16.4 -3 -19.6Z`;
    const tail = `M10.6 -19.4L15.8 -16.2L11.6 -14.6Z`;
    const neck = `M-4.6 -19C-8.6 -23.4 -2.6 -29.8 -5.2 -34.2C-6.8 -37 -6.2 -40 -3.2 -41.4`;
    const head = `M-5.6 -41.8C-5.6 -43.6 -2.4 -44.6 -0.6 -43.2C0.6 -42.2 0.2 -40.6 -1.4 -40.2C-3 -39.8 -5.6 -40.2 -5.6 -41.8Z`;
    const bill = `M-5.4 -42.4L-13.4 -40.8L-5.2 -40.8Z`;
    const plume = `M-2.6 -43.6C0 -44.8 3 -45 5.8 -44.2`;
    const legs = `M1 -12.2L0.4 -6.6L0.8 0M3.4 -12.4L3.8 -6.6L3.2 0`;
    const colour = `<g filter="url(#gouache-pt)"><path d="${body}" fill="#8597a3"/><path d="${wing}" fill="#6b7f8c"/><path d="${tail}" fill="#5f707c"/>
      <path d="M0.4 -13.4C1.6 -14.6 3.4 -14.6 4.4 -13.2L3.8 -11.8L1 -11.6Z" fill="#9a5a3c"/>
      <path d="${neck}" stroke="#c9cfcb" stroke-width="2.7" fill="none" stroke-linecap="round"/><path d="M-6.4 -36C-7 -31 -4 -27 -5.6 -22" stroke="#7d8d96" stroke-width="0.7" fill="none"/>
      <path d="${head}" fill="#f2efe6"/><path d="M-4.8 -43.2C-3 -44 -1 -44 0.2 -43C-1.6 -42.6 -3.4 -42.6 -4.8 -43.2Z" fill="#2e2a20"/><path d="${bill}" fill="#d9a441"/>
      <path d="${legs}" stroke="#6b5a40" stroke-width="0.75" fill="none"/></g>
      <circle cx="-3.6" cy="-41.6" r="0.45" fill="#2e2a20"/>`;
    const ink = `<g fill="none" stroke="${C.ink}" stroke-width="0.4" stroke-linejoin="round" stroke-linecap="round" filter="url(#inkline-pt)" transform="translate(0.35 0.3)">
      <path d="${body}"/><path d="${wing}" stroke-width="0.3"/><path d="${head}" stroke-width="0.3"/><path d="${bill}" stroke-width="0.3"/><path d="${plume}" stroke-width="0.45"/>
      <path d="M-5.9 -20.6C-9.9 -24.8 -3.9 -30.4 -6.5 -34.6C-8 -37.2 -7.6 -39.8 -5.4 -41M-3.4 -18.2C-7.2 -22.6 -1.2 -29.2 -3.8 -33.6C-5.2 -36 -4.8 -38.8 -2.4 -40.4" stroke-width="0.32"/>
      <path d="M2.6 -20.4L7.8 -17.4M3.4 -21.4L9 -18.8" stroke-width="0.22"/></g>`;
    const water = `<g fill="none" stroke="${C.ink}" stroke-width="0.4" stroke-linecap="round" filter="url(#inkline-pt)" opacity="0.8">
      <path d="M-6 1.2q3 -1.6 6 0t6 0M-11 3.6q2.6 -1.3 5.2 0M7 3.4q2.6 -1.3 5.2 0"/></g>`;
    return `<g filter="url(#cutout-pt)">${colour}${ink}</g>${water}`;
  }

  // ---------- stamps: small plants and buildings, a spot-colour block and its key line ----------
  // Each kind returns [colour, ink] in points, its origin at the foot; put() places one.
  const STAMP = {
    spruce: (o) => {
      const body = `M0 -12L3.2 -6.6L1.6 -6.8L4.4 -2.4L-4.4 -2.4L-1.6 -6.8L-3.2 -6.6Z`;
      return [`<path d="${body}" fill="${o.fill || C.sageDeep}"/>`, `<path d="${body}" fill="none" stroke="${C.ink}" stroke-width="0.4"/><path d="M0 -2.4V0.4" stroke="${C.ink}" stroke-width="0.6"/>`];
    },
    oak: (o) => {
      const crown = `M-4.4 -4.6C-6.4 -6.2 -5.4 -9.6 -2.8 -9.6C-2 -11.8 2.2 -11.8 3 -9.6C5.6 -9.8 6.6 -6.2 4.6 -4.6C3.4 -3.2 -3.2 -3.2 -4.4 -4.6Z`;
      return [`<path d="${crown}" fill="${o.fill || C.sageDeep}"/><rect x="-0.55" y="-4.4" width="1.1" height="4.4" fill="#5a4630"/>`,
        `<path d="${crown}" fill="none" stroke="${C.ink}" stroke-width="0.4"/><path d="M0 0V-4.2" stroke="${C.ink}" stroke-width="0.45"/>${o.ground === false ? '' : `<path d="M-2.6 0.2H2.6" stroke="${C.ink}" stroke-width="0.45"/>`}`];
    },
    hawthorn: () => {
      const crown = `M-7.5 -9C-9.5 -12 -7 -15.5 -3.5 -15C-2 -18 3 -18.2 4.6 -15.4C8 -16 10.6 -12.6 9 -9.6C10.6 -7.4 8 -5 5 -6C3 -4.6 -1 -4.6 -2.6 -6.2C-5.6 -5 -9 -6.6 -7.5 -9Z`;
      const haws = [[-5, -10], [-1.5, -13.5], [3, -11], [6.5, -9.5], [1, -8], [-4, -7.4], [5.4, -13.6], [-6.2, -12.6]].map(([x, y]) => `<circle cx="${x}" cy="${y}" r="0.85"/>`).join('');
      return [`<path d="${crown}" fill="${C.sageDeep}"/><path d="M-0.5 0C0 -2 -0.6 -4 0.6 -6.4" stroke="#5a4630" stroke-width="1.3" fill="none"/><g fill="${C.haw}">${haws}</g>`,
        `<path d="${crown}" fill="none" stroke="${C.ink}" stroke-width="0.45"/><path d="M-0.5 0C0 -2 -0.6 -4 0.6 -6.4M0 -4.2L-3 -6.6M0.3 -5L3.4 -7" stroke="${C.ink}" stroke-width="0.5" fill="none"/><path d="M-6 0.3H6" stroke="${C.ink}" stroke-width="0.4"/>`];
    },
    heather: () => {
      const bl = [[-2.6, -5], [-3, -3.4], [-2.2, -6.6], [0.2, -7.6], [0.4, -5.8], [0.2, -4.2], [2.6, -6], [2.8, -4.4], [2.2, -7.4]].map(([x, y]) => `<ellipse cx="${x}" cy="${y}" rx="0.9" ry="0.7"/>`).join('');
      return [`<g fill="#8a4f6a">${bl}</g>`, `<path d="M0 0C-0.4 -2 -1.8 -3 -2.4 -6.8M0 0C0.2 -2.4 0.2 -4.6 0.2 -8M0 0C0.6 -2 2 -3.4 2.4 -7.4" stroke="${C.ink}" stroke-width="0.35" fill="none"/>`];
    },
    wheat: () => {
      let ears = '', stalks = '';
      for (const [dx, lean] of [[-2.2, -0.35], [0, 0], [2.2, 0.35]]) {
        const tx = dx + lean * 6, ty = -9;
        stalks += `M${dx * 0.3} 0L${f2(tx)} ${ty}`;
        for (let k = 0; k < 3; k++) { const yy = ty + 0.6 + k * 1.3, xx = tx - lean * k * 1.2; ears += `<ellipse cx="${f2(xx - 0.6)}" cy="${f2(yy)}" rx="0.45" ry="0.9" transform="rotate(-25 ${f2(xx - 0.6)} ${f2(yy)})"/><ellipse cx="${f2(xx + 0.6)}" cy="${f2(yy)}" rx="0.45" ry="0.9" transform="rotate(25 ${f2(xx + 0.6)} ${f2(yy)})"/>`; }
      }
      return [`<g fill="${C.ochre}">${ears}</g><path d="M-2.2 -3.2Q0 -2.4 2.2 -3.2" stroke="${C.rust}" stroke-width="0.8" fill="none"/>`, `<path d="${stalks}" stroke="${C.ink}" stroke-width="0.35" fill="none"/>`];
    },
    cactus: () => {
      const body = `M-0.9 0V-9.4Q0 -10.6 0.9 -9.4V0ZM0.9 -5.6H2.4V-7.6Q3.1 -8.4 3.6 -7.6V-4.6H0.9ZM-0.9 -4.2H-2.6V-6.2Q-3.2 -7 -3.8 -6.2V-3.2H-0.9Z`;
      return [`<path d="${body}" fill="${C.sageDeep}"/>`, `<path d="${body}" fill="none" stroke="${C.ink}" stroke-width="0.35"/><path d="M-4 0.2H4" stroke="${C.ink}" stroke-width="0.4"/>`];
    },
    // a street tree's crown, seen from above: a lobed round, its key line broken
    crown: (o) => {
      const r = o.r || 4, n = 7, ph = o.ph || 0;
      let d = '';
      for (let i = 0; i <= n; i++) {
        const a = ph + i / n * 2 * Math.PI, a2 = ph + (i + 0.5) / n * 2 * Math.PI;
        const p = [Math.cos(a) * r, Math.sin(a) * r], q = [Math.cos(a2) * r * 1.22, Math.sin(a2) * r * 1.22];
        d += i === 0 ? `M${f2(p[0])} ${f2(p[1])}` : `Q${f2(q0x)} ${f2(q0y)} ${f2(p[0])} ${f2(p[1])}`;
        var q0x = q[0], q0y = q[1];
      }
      return [`<path d="${d}Z" fill="${o.fill || C.sageDeep}"/>`, `<path d="${d}" fill="none" stroke="${C.ink}" stroke-width="0.26" stroke-opacity="0.75" stroke-dasharray="${f2(r * 2.2)} ${f2(r * 0.9)}"/><circle r="0.4" fill="${C.ink}" opacity="0.7"/>`];
    },
    // a town: three gabled houses and a spire, in cut card
    town: (o) => {
      const h = [[-5.2, 0, 3.6, 3.6], [-1.4, 0, 3.2, 4.6], [2.2, 0, 3.4, 3.2]];
      let fill = '', ink = '';
      const cols = o.cols || [C.kraftLight, '#e9dfc9', C.clay];
      h.forEach(([x, y, w, hh], i) => {
        const d = `M${x} ${y}V${y - hh}L${x + w / 2} ${f2(y - hh - w * 0.5)}L${x + w} ${y - hh}V${y}Z`;
        fill += `<path d="${d}" fill="${cols[i % cols.length]}"/>`;
        ink += `<path d="${d}" fill="none" stroke="${C.ink}" stroke-width="0.32"/>`;
      });
      const spire = `M0.2 -4.6L1 -9.4L1.8 -4.6Z`;
      fill += `<path d="${spire}" fill="${C.rust}"/>`; ink += `<path d="${spire}" fill="none" stroke="${C.ink}" stroke-width="0.3"/>`;
      return [fill, ink];
    },
  };
  function put(kind, x, y, s, o = {}) {
    const [a, b] = STAMP[kind](o);
    const t = `transform="translate(${f1(x)} ${f1(y)}) scale(${f2(s)})${o.rotate ? ` rotate(${o.rotate})` : ''}"`;
    return [`<g ${t}>${a}</g>`, `<g ${t}>${b}</g>`];
  }
  // Many stamps printed together: colours with the riso grain, then the key lines off register.
  function stamps(list) { return PA.stamp(list.map(l => l[0]).join(''), list.map(l => l[1]).join('')); }

  return { ptDefs, place, cubicPts, ptsD, leafD, serpent, ship, geese, buckeyeSprig, cardinal, heron, mapleLeaf, oakLeaf, STAMP, put, stamps };
})();
window.Sprites = Sprites;

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

  return { ptDefs, place, cubicPts, ptsD, leafD, serpent, ship };
})();
window.Sprites = Sprites;

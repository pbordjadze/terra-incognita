#!/usr/bin/env node
// Draws the herds' animals (Prototypes/herds/art/): one camelid, drawn from parameters, in each of the
// family's forms, walking and grazing, at the sizes the pack shows them. Rebuild, never hand-edit:
//
//   node tools/herds/draw.cjs [--sheet out.png] [--only kind,kind]
//
// The animal is built from a few parts in its own frame (x forward, y up, the height at the withers 1):
// a body whose back carries the humps, a tapered neck along a curve, a head, a tail, and four legs
// whose feet follow a gait and whose knees are found from the feet (two-bone inverse kinematics). The
// family paces: the two legs of one side swing together, as camels and llamas walk. Every part's ink
// is laid down first, wider, and the fills over it, so only the silhouette is outlined, as an
// engraver's line; a few fine strokes add the eye, the mouth, the ears, the knees' calluses and the
// wool. The pictures face east (the runtime mirrors them for a sprite heading west).
//
// What is drawn is an illustration, not a reconstruction: every kind's proportions and coat follow
// the sources in Prototypes/herds/SOURCES.md broadly, and where the fossils say nothing (the extinct
// animals' coats, colours and whether they had humps) the drawing says so there.
'use strict';
const fs = require('fs');
const path = require('path');
const { chromium } = require('playwright');

const ROOT = path.resolve(__dirname, '..', '..');
const OUT = path.join(ROOT, 'Prototypes', 'herds', 'art');
const INK = '#3B2C16';          // the atlas's text ink
const HALO = '#F4ECD8';         // the paper's, round a tiny figure so it reads on any ground

// ---- the kinds -------------------------------------------------------------------------------------
// size: the height at the withers against the dromedary's (about 2 m); leg: the legs' share of it;
// body: shoulder to hip; depth: the barrel; neck: { len, lean (degrees from upright, forward), dip (how
// far a camel's neck first falls forward before it rises) }; head: { len, depth, tilt }; ears: shape;
// humps: [{ at (0 hip, 1 withers), h, w }]; tail; coat: colours; wool: the outline's curl; mane: the
// Bactrian's long hair under the neck and on the forearms.
const KINDS = {
  early: {
    name: 'An early camelid', size: 0.42, leg: 0.6, body: 0.92, depth: 0.36, slope: 0.06,
    neck: { len: 0.5, lean: 38, dip: 0.02, base: 0.2, top: 0.11 }, head: { len: 0.33, depth: 0.13, tilt: -24, snout: 0.5 },
    ears: 'deer', humps: [], tail: { len: 0.22, tuft: 0.04 },
    coat: { base: '#9C6B3C', light: '#C08C55', shade: '#6E4724', belly: '#D9B88A', far: '#7C5330' }, wool: 0, calluses: false
  },
  giraffe: {
    name: 'A giraffe-camel', size: 1.15, leg: 0.68, body: 0.7, depth: 0.27, slope: 0.12,
    neck: { len: 1.05, lean: 14, dip: 0.0, base: 0.15, top: 0.075 }, head: { len: 0.24, depth: 0.09, tilt: -30, snout: 0.45 },
    ears: 'llama-short', humps: [], tail: { len: 0.17, tuft: 0.04 },
    coat: { base: '#A9763E', light: '#CC9A5E', shade: '#77502A', belly: '#E0C39A', far: '#86592F' }, wool: 0, calluses: true
  },
  paracamelus: {
    name: 'Paracamelus, a camel of the north', size: 1.08, leg: 0.6, body: 0.95, depth: 0.37, slope: 0.0,
    neck: { len: 0.68, lean: 58, dip: 0.12, base: 0.24, top: 0.115 }, head: { len: 0.33, depth: 0.13, tilt: -14, snout: 0.55 },
    ears: 'camel', humps: [], tail: { len: 0.24, tuft: 0.06 },
    coat: { base: '#8E6A45', light: '#B18B62', shade: '#5E4429', belly: '#C9AB84', far: '#6C4F33' }, wool: 0.012, calluses: true
  },
  arctic: {
    name: 'The giant camel of the High Arctic', size: 1.3, leg: 0.6, body: 0.98, depth: 0.39, slope: 0.0,
    neck: { len: 0.66, lean: 60, dip: 0.12, base: 0.26, top: 0.13 }, head: { len: 0.33, depth: 0.14, tilt: -12, snout: 0.55 },
    ears: 'camel', humps: [{ at: 0.62, h: 0.1, w: 0.42 }], tail: { len: 0.24, tuft: 0.07 },
    coat: { base: '#7A5B3F', light: '#A0805F', shade: '#4E3825', belly: '#B79A79', far: '#5C442E' }, wool: 0.03, mane: 0.6, calluses: true
  },
  camelops: {
    name: 'Camelops, the camel of the American West', size: 1.12, leg: 0.62, body: 0.93, depth: 0.36, slope: 0.02,
    neck: { len: 0.66, lean: 52, dip: 0.08, base: 0.23, top: 0.115 }, head: { len: 0.34, depth: 0.13, tilt: -16, snout: 0.55 },
    ears: 'camel', humps: [], tail: { len: 0.24, tuft: 0.05 },
    coat: { base: '#9A7148', light: '#BF956A', shade: '#68492B', belly: '#D2B48C', far: '#765535' }, wool: 0, calluses: true
  },
  bactrian: {
    name: 'A Bactrian camel', size: 0.98, leg: 0.53, body: 1.0, depth: 0.4, slope: 0.0,
    neck: { len: 0.62, lean: 62, dip: 0.13, base: 0.25, top: 0.12 }, head: { len: 0.32, depth: 0.135, tilt: -12, snout: 0.55 },
    ears: 'camel', humps: [{ at: 0.27, h: 0.2, w: 0.27 }, { at: 0.74, h: 0.21, w: 0.27 }], tail: { len: 0.24, tuft: 0.06 },
    coat: { base: '#7C5634', light: '#A57B52', shade: '#4F3520', belly: '#B59572', far: '#5F4128' }, wool: 0.022, mane: 1, calluses: true
  },
  dromedary: {
    name: 'A dromedary', size: 1.0, leg: 0.6, body: 0.96, depth: 0.37, slope: 0.0,
    neck: { len: 0.7, lean: 62, dip: 0.14, base: 0.23, top: 0.11 }, head: { len: 0.33, depth: 0.125, tilt: -10, snout: 0.55 },
    ears: 'camel', humps: [{ at: 0.52, h: 0.27, w: 0.46 }], tail: { len: 0.25, tuft: 0.05 },
    coat: { base: '#C49A62', light: '#DDB985', shade: '#8E6A3F', belly: '#E6CDA4', far: '#A07B4C' }, wool: 0, calluses: true
  },
  camelus: {
    name: 'Camelus, an early true camel', size: 1.0, leg: 0.6, body: 0.96, depth: 0.37, slope: 0.0,
    neck: { len: 0.69, lean: 60, dip: 0.13, base: 0.23, top: 0.11 }, head: { len: 0.33, depth: 0.125, tilt: -12, snout: 0.55 },
    ears: 'camel', humps: [{ at: 0.52, h: 0.13, w: 0.42 }], tail: { len: 0.25, tuft: 0.05 },
    coat: { base: '#A88457', light: '#C9A777', shade: '#745834', belly: '#DCC39C', far: '#87683F' }, wool: 0, calluses: true
  },
  hemiauchenia: {
    name: 'Hemiauchenia, a long-legged llama of the Americas', size: 0.7, leg: 0.68, body: 0.84, depth: 0.32, slope: 0.04,
    neck: { len: 0.78, lean: 18, dip: 0.0, base: 0.17, top: 0.09 }, head: { len: 0.27, depth: 0.105, tilt: -24, snout: 0.5 },
    ears: 'llama', humps: [], tail: { len: 0.16, tuft: 0.04 },
    coat: { base: '#A67447', light: '#C8986A', shade: '#714C2C', belly: '#E4CBA8', far: '#82593A' }, wool: 0.004, calluses: false
  },
  guanaco: {
    name: 'A guanaco', size: 0.58, leg: 0.6, body: 0.88, depth: 0.34, slope: 0.03,
    neck: { len: 0.74, lean: 20, dip: 0.0, base: 0.17, top: 0.095 }, head: { len: 0.27, depth: 0.11, tilt: -20, snout: 0.5 },
    ears: 'llama', humps: [], tail: { len: 0.17, tuft: 0.05 },
    coat: { base: '#B06F3C', light: '#CF8F58', shade: '#7A4A24', belly: '#F2E8D8', far: '#8E5A31' }, wool: 0.006, twoTone: true, calluses: false
  },
  vicuna: {
    name: 'A vicuña', size: 0.45, leg: 0.62, body: 0.82, depth: 0.31, slope: 0.03,
    neck: { len: 0.72, lean: 20, dip: 0.0, base: 0.15, top: 0.085 }, head: { len: 0.25, depth: 0.1, tilt: -22, snout: 0.48 },
    ears: 'llama', humps: [], tail: { len: 0.15, tuft: 0.04 },
    coat: { base: '#C8915A', light: '#E0AF7B', shade: '#91653B', belly: '#F7F0E2', far: '#A57447' }, wool: 0.006, twoTone: true, bib: true, calluses: false
  },
  llama: {
    name: 'A llama', size: 0.6, leg: 0.58, body: 0.9, depth: 0.36, slope: 0.03,
    neck: { len: 0.78, lean: 16, dip: 0.0, base: 0.19, top: 0.105 }, head: { len: 0.29, depth: 0.12, tilt: -20, snout: 0.5 },
    ears: 'banana', humps: [], tail: { len: 0.17, tuft: 0.06 },
    coat: { base: '#EFE6D6', light: '#FBF6EC', shade: '#C4B49B', belly: '#F4EDE1', far: '#CDBFA8', patch: '#7A4E2C' }, wool: 0.02, patches: true, calluses: false
  },
  alpaca: {
    name: 'An alpaca', size: 0.44, leg: 0.5, body: 0.86, depth: 0.42, slope: 0.0,
    neck: { len: 0.62, lean: 14, dip: 0.0, base: 0.24, top: 0.17 }, head: { len: 0.23, depth: 0.12, tilt: -14, snout: 0.42 },
    ears: 'alpaca', humps: [], tail: { len: 0.13, tuft: 0.08 },
    coat: { base: '#E9D9BC', light: '#F7EEDC', shade: '#BFA67F', belly: '#EFE2CB', far: '#CCB592' }, wool: 0.055, topknot: true, calluses: false
  }
};

// the herded camels carry a saddle blanket (striped, fringed); a herded llama wears the yarn tassels
// Andean herders tie in its ears
KINDS['dromedary-domestic'] = Object.assign({}, KINDS.dromedary, { name: 'A dromedary, herded',
  blanket: { from: 0.3, to: 0.74, drop: 0.15, colours: ['#8E3328', '#D7A23E', '#2D4672'] } });
KINDS['bactrian-domestic'] = Object.assign({}, KINDS.bactrian, { name: 'A Bactrian camel, herded',
  blanket: { from: 0.36, to: 0.66, drop: 0.2, colours: ['#2D4672', '#C9473A', '#E2C27A'] } });
KINDS.llama.tassels = ['#C2384B', '#E07A2E'];

// ---- geometry ----------------------------------------------------------------------------------------
const rad = (d) => (d * Math.PI) / 180;
const add = (a, b) => [a[0] + b[0], a[1] + b[1]];
const sub = (a, b) => [a[0] - b[0], a[1] - b[1]];
const mul = (a, k) => [a[0] * k, a[1] * k];
const len = (a) => Math.hypot(a[0], a[1]);
const norm = (a) => { const l = len(a) || 1; return [a[0] / l, a[1] / l]; };
const perp = (a) => [-a[1], a[0]];
const lerp = (a, b, t) => a + (b - a) * t;
const smooth = (t) => t * t * (3 - 2 * t);

function bezier(p0, p1, p2, p3, n) {
  const out = [];
  for (let i = 0; i <= n; i++) {
    const t = i / n, u = 1 - t;
    out.push([u * u * u * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t * t * t * p3[0],
              u * u * u * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t * t * t * p3[1]]);
  }
  return out;
}

/** A closed Catmull-Rom curve through the points, sampled finely. */
function closedCurve(pts, per = 8) {
  const out = [], n = pts.length;
  for (let i = 0; i < n; i++) {
    const p0 = pts[(i - 1 + n) % n], p1 = pts[i], p2 = pts[(i + 1) % n], p3 = pts[(i + 2) % n];
    for (let k = 0; k < per; k++) {
      const t = k / per, t2 = t * t, t3 = t2 * t;
      out.push([0, 1].map((j) => 0.5 * ((2 * p1[j]) + (-p0[j] + p2[j]) * t + (2 * p0[j] - 5 * p1[j] + 4 * p2[j] - p3[j]) * t2
        + (-p0[j] + 3 * p1[j] - 3 * p2[j] + p3[j]) * t3)));
    }
  }
  return out;
}

/** A tapered limb along points with widths: its outline (up its left side, down its right), with round ends. */
function tapered(pts, widths) {
  const left = [], right = [], n = pts.length;
  for (let i = 0; i < n; i++) {
    const d = norm(sub(pts[Math.min(n - 1, i + 1)], pts[Math.max(0, i - 1)]));
    const nrm = perp(d), w = widths[i] / 2;
    left.push(add(pts[i], mul(nrm, w)));
    right.push(add(pts[i], mul(nrm, -w)));
  }
  // a half circle, clockwise from the side `from` points to
  const cap = (c, from, w, steps = 6) => {
    const out = [], a0 = Math.atan2(from[1], from[0]);
    for (let k = 1; k < steps; k++) { const a = a0 - (Math.PI * k) / steps; out.push([c[0] + Math.cos(a) * w, c[1] + Math.sin(a) * w]); }
    return out;
  };
  const endDir = norm(sub(pts[n - 1], pts[n - 2])), startDir = norm(sub(pts[1], pts[0]));
  return [...left, ...cap(pts[n - 1], perp(endDir), widths[n - 1] / 2), ...right.reverse(),
          ...cap(pts[0], mul(perp(startDir), -1), widths[0] / 2)];
}

/** A closed outline made woolly: pushed out and in along its normals in small curls. */
function curl(pts, amp, waves) {
  if (!amp) return pts;
  const n = pts.length;
  return pts.map((p, i) => {
    const d = norm(sub(pts[(i + 1) % n], pts[(i - 1 + n) % n])), nrm = perp(d);
    const k = Math.abs(Math.sin((i / n) * Math.PI * waves));
    return add(p, mul(nrm, amp * (0.35 + 0.65 * k)));
  });
}

/** Two-bone inverse kinematics: the knee between a joint and a foot; `forward` bends it forward. */
function knee(j, f, l1, l2, forward) {
  const d = Math.min(len(sub(f, j)), (l1 + l2) * 0.999), a = Math.atan2(f[1] - j[1], f[0] - j[0]);
  const c = Math.max(-1, Math.min(1, (l1 * l1 + d * d - l2 * l2) / (2 * l1 * d)));
  const b = Math.acos(c) * (forward ? 1 : -1);
  return [j[0] + l1 * Math.cos(a + b), j[1] + l1 * Math.sin(a + b)];
}

// ---- one animal, one moment ------------------------------------------------------------------------
// pose: { gait: 'walk' | 'graze' | 'stand', phase (0–1 of a stride), head (0 up – 1 down, grazing), chew }
function build(k, pose) {
  const L = k.body, D = k.depth, leg = k.leg, H = 1;
  const bob = pose.gait === 'walk' ? 0.012 * Math.cos(4 * Math.PI * pose.phase) : 0;
  const bottom = leg + bob, top = leg + D + bob;
  const rumpX = -L / 2, chestX = L / 2;
  // the back, with its humps (at: 0 over the hip, 1 over the withers)
  const back = [];
  for (let i = 0; i <= 14; i++) {
    const t = i / 14, x = lerp(rumpX + 0.06, chestX - 0.04, t);
    let y = top + k.slope * (t - 0.5) * 2 * 0.5 - 0.02 * Math.pow(Math.abs(t - 0.45) * 2, 2);
    for (const h of k.humps) y += h.h * Math.exp(-Math.pow((t - h.at) / (h.w / 2), 2) * 1.6);
    back.push([x, y]);
  }
  const body = [
    ...back,
    [chestX + 0.03, top - 0.06],
    [chestX + 0.06, bottom + D * 0.5],
    [chestX - 0.02, bottom + 0.03],
    [chestX - 0.18, bottom - 0.005],
    [0, bottom - 0.025],
    [rumpX + 0.2, bottom + 0.005],
    [rumpX + 0.06, bottom + D * 0.25],
    [rumpX - 0.04, bottom + D * 0.62],
    [rumpX - 0.01, top - 0.05]
  ];
  const bodyOutline = curl(closedCurve(body, 6), k.wool, 48);

  // the legs: joints, feet by the gait, knees by IK
  const foreJ = [chestX - 0.13, bottom + D * 0.42], hindJ = [rumpX + 0.15, bottom + D * 0.5];
  const stride = 0.42 * leg / 0.6, lift = 0.11;
  const legs = [];
  const foot = (j, phase, extra) => {
    if (pose.gait !== 'walk') return [j[0] + extra, 0];
    const q = ((phase % 1) + 1) % 1, duty = 0.62;
    if (q < duty) return [j[0] + extra + stride / 2 - stride * (q / duty), 0];
    const s = (q - duty) / (1 - duty);
    return [j[0] + extra - stride / 2 + stride * smooth(s), lift * Math.sin(Math.PI * s)];
  };
  for (const [side, off] of [['far', 0.5], ['near', 0]]) {
    for (const [which, j, fwd] of [['hind', hindJ, false], ['fore', foreJ, true]]) {
      // grazing, the feet stand a little apart, the forefeet splayed forward
      const extra = pose.gait === 'walk' ? 0 : (which === 'fore' ? (side === 'near' ? 0.07 : -0.02) : (side === 'near' ? -0.03 : 0.05));
      const f = foot(j, pose.phase + off + (which === 'hind' ? 0.04 : 0), extra);
      const fh = [f[0], f[1] + 0.045];      // the fetlock, above the pad
      const l1 = (j[1] - 0.045) * (which === 'fore' ? 0.5 : 0.56), l2 = (j[1] - 0.045) * (which === 'fore' ? 0.53 : 0.5);
      const kn = knee(j, fh, l1, l2, fwd);
      const thigh = which === 'hind' ? 0.2 : 0.13;
      const pts = [j, add(j, mul(sub(kn, j), 0.5)), kn, add(kn, mul(sub(fh, kn), 0.55)), fh, [f[0] + 0.03, f[1] + 0.012]];
      const ws = [thigh, thigh * 0.62, 0.062, 0.05, 0.045, 0.04].map((w) => w * (k.size < 0.5 ? 1.05 : 1) * (1 + k.wool * 3));
      legs.push({ side, which, outline: tapered(pts, ws), knee: kn, foot: f, fetlock: fh });
    }
  }

  // the neck: from the chest's top, a curve to the head; a camel's first falls forward, then rises
  const graze = pose.gait === 'graze' || pose.gait === 'stand' ? pose.head || 0 : 0;
  const base = [chestX - 0.1, top - 0.08];
  const lean = rad(lerp(k.neck.lean, 135, graze)), nl = k.neck.len * (1 - graze * 0.08);
  const headJ = add(base, [Math.sin(lean) * nl, Math.cos(lean) * nl + (pose.gait === 'walk' ? -bob * 0.6 : 0)]);
  const dip = k.neck.dip;
  // upright, a camel's neck first falls forward (its dip) and rises to the head; grazing, it arches down to it
  const c1 = add(base, [lerp(0.06 + 1.8 * dip, 0.3 * nl, graze), lerp(0.32 * nl - 2.4 * dip, 0.12 * nl, graze)]);
  const c2 = add(headJ, [lerp(-0.03 - 0.6 * dip, -0.02, graze), lerp(-0.32 * nl, 0.3 * nl, graze)]);
  const neckPts = bezier(base, c1, c2, headJ, 14);
  const neckW = neckPts.map((_, i) => lerp(k.neck.base, k.neck.top, smooth(i / 14)) * (1 + k.wool * 2.5));
  const neckOutline = curl(tapered(neckPts, neckW), k.wool * 0.6, 22);

  // the head: a long skull and its muzzle, tipped by the tilt (and down further, grazing)
  const tilt = rad(k.head.tilt - graze * 70 + (pose.chew ? 4 : 0));
  const hd = [Math.cos(tilt), Math.sin(tilt)], hn = perp(hd), hl = k.head.len, hdp = k.head.depth;
  const at = (u, v) => add(headJ, add(mul(hd, u * hl), mul(hn, v * hdp)));
  const headShape = closedCurve([at(-0.12, 0.55), at(0.25, 0.62), at(0.6, 0.42), at(0.98, 0.25), at(1.04, -0.05),
                                 at(0.95, -0.38), at(0.55, -0.5), at(0.15, -0.55), at(-0.15, -0.3)], 6);
  const eye = at(0.22, 0.2), nostril = at(0.93, 0.12), mouth = [at(0.99, -0.22), at(0.72, -0.3)];
  // the ears
  const ears = [];
  const ear = (u, l, w, bend, angle) => {
    const b = at(u, 0.5), dir = [Math.cos(tilt + rad(angle)), Math.sin(tilt + rad(angle))];
    const tip = add(b, mul(dir, l)), side = perp(dir);
    const mid = add(add(b, mul(dir, l * 0.55)), mul(side, bend));
    return { outline: closedCurve([add(b, mul(side, w / 2)), add(mid, mul(side, w * 0.45)), tip, add(mid, mul(side, -w * 0.45)),
                                   add(b, mul(side, -w / 2))], 5), base: b, tip };
  };
  switch (k.ears) {
    case 'camel': ears.push(ear(0.05, 0.07, 0.05, 0, 120)); break;
    case 'deer': ears.push(ear(0.02, 0.1, 0.055, 0, 125)); break;
    case 'llama-short': ears.push(ear(0.04, 0.09, 0.04, 0, 105)); break;
    case 'llama': ears.push(ear(0.02, 0.12, 0.045, 0.012, 100)); break;
    case 'banana': ears.push(ear(0.0, 0.15, 0.05, 0.03, 98)); break;
    case 'alpaca': ears.push(ear(0.05, 0.085, 0.045, 0, 100)); break;
  }
  // an alpaca's topknot: a tuft of wool on the crown
  const topknot = k.topknot ? curl(closedCurve([at(-0.1, 0.4), at(0.05, 1.25), at(0.3, 1.3), at(0.42, 0.55)], 8), 0.012, 9) : null;
  // the tail
  const tailBase = [rumpX - 0.02, top - 0.07];
  const sway = pose.gait === 'walk' ? 0.02 * Math.sin(2 * Math.PI * pose.phase) : 0;
  const tailEnd = add(tailBase, [-0.05 + sway, -k.tail.len]);
  const tailPts = bezier(tailBase, add(tailBase, [-0.06, 0.0]), add(tailEnd, [-0.02, 0.06]), tailEnd, 8);
  const tailOutline = tapered(tailPts, tailPts.map((_, i) => lerp(0.05, 0.025 + k.tail.tuft, i / 8)));

  return { k, body: bodyOutline, back, legs, neck: neckOutline, neckPts, head: closedCurve(headShape.filter((_, i) => i % 1 === 0), 1),
           headJ, eye, nostril, mouth, ears, tail: tailOutline, topknot, top, bottom, foreJ, hindJ, chestX, rumpX, tilt, at };
}

// ---- drawing ---------------------------------------------------------------------------------------
const poly = (pts) => 'M' + pts.map((p) => p[0].toFixed(4) + ' ' + p[1].toFixed(4)).join('L') + 'Z';
const line = (pts) => 'M' + pts.map((p) => p[0].toFixed(4) + ' ' + p[1].toFixed(4)).join('L');

/** The SVG body of one animal: `detail` 'close' (the illustration), 'small' (a coloured figure) or
    'tiny' (a silhouette in ink on a halo). Units are the animal's own; `ink` is the outline's width. */
function draw(a, detail, ink) {
  const k = a.k, c = k.coat, parts = [];
  const shapes = [
    ...a.legs.filter((l) => l.side === 'far').map((l) => ({ d: poly(l.outline), fill: c.far })),
    { d: poly(a.tail), fill: c.shade },
    { d: poly(a.body), fill: 'url(#coat)' },
    { d: poly(a.neck), fill: 'url(#coat)' },
    ...a.ears.map((e) => ({ d: poly(e.outline), fill: c.shade })),
    { d: poly(a.head), fill: 'url(#coat)' },
    ...(k.topknot ? [{ d: poly(a.topknot), fill: c.light }] : []),
    ...a.legs.filter((l) => l.side === 'near').map((l) => ({ d: poly(l.outline), fill: 'url(#coat)' }))
  ];
  if (detail === 'tiny') {
    // (no wool on a figure this small)
    // a silhouette: the halo, then the ink
    for (const s of shapes) parts.push(`<path d="${s.d}" fill="${HALO}" stroke="${HALO}" stroke-width="${ink * 2.6}" stroke-linejoin="round"/>`);
    for (const s of shapes) parts.push(`<path d="${s.d}" fill="${INK}" stroke="${INK}" stroke-width="${ink * 0.5}" stroke-linejoin="round"/>`);
    return parts.join('');
  }
  // the ink of the whole silhouette first, wider; then the fills over it
  // (close up, the line wavers a little, drawn by hand)
  parts.push(detail === 'close' ? '<g filter="url(#hand)">' : '<g>');
  for (const s of shapes) parts.push(`<path d="${s.d}" fill="${INK}" stroke="${INK}" stroke-width="${ink * 2}" stroke-linejoin="round"/>`);
  parts.push('</g>');
  for (const s of shapes) parts.push(`<path d="${s.d}" fill="${s.fill}"/>`);
  // the far legs' edge where they pass behind the near ones, and the near legs' fronts, finely
  const fine = ink * 0.55;
  for (const l of a.legs.filter((x) => x.side === 'near')) {
    parts.push(`<path d="${line([l.knee, l.fetlock])}" stroke="${c.shade}" stroke-width="${fine * 1.4}" fill="none" stroke-linecap="round" opacity="0.55"/>`);
  }
  if (k.twoTone) {
    // the guanaco's and vicuña's pale underside, cut along the flank in a gentle curve, and the inside
    // of the far legs
    const yb = a.bottom + (a.top - a.bottom) * 0.38, under = [[-2, -1]];
    for (let i = 0; i <= 10; i++) { const x = lerp(a.rumpX - 0.1, a.chestX + 0.1, i / 10); under.push([x, yb + 0.05 * Math.sin(Math.PI * i / 10) - 0.02 * (i / 10)]); }
    under.push([2, -1]);
    parts.push(`<clipPath id="under"><path d="${poly(under)}"/></clipPath>`);
    parts.push(`<path d="${poly(a.body)}" fill="${c.belly}" clip-path="url(#under)"/>`);
  }
  if (k.bib) {
    // the vicuña's long white bib, hanging from the base of the throat over the chest
    const b0 = a.neckPts[3], d = norm(sub(a.neckPts[4], a.neckPts[2])), front = mul(perp(d), -1);
    const w0 = lerp(k.neck.base, k.neck.top, smooth(3 / 14)) / 2;
    const root = add(b0, mul(front, w0 * 0.6));
    const bib = tapered([root, add(root, [0.03, -0.08]), add(root, [0.035, -0.16]), add(root, [0.02, -0.22])], [0.07, 0.075, 0.06, 0.03]);
    parts.push(`<path d="${poly(curl(bib, 0.006, 7))}" fill="${c.belly}" stroke="${INK}" stroke-width="${fine}" opacity="0.97"/>`);
  }
  if (k.patches) {
    // a llama's patches: brown over its pale coat
    parts.push(`<clipPath id="bodyclip"><path d="${poly(a.body)}"/></clipPath>`);
    parts.push(`<g clip-path="url(#bodyclip)" fill="${c.patch}" opacity="0.92"><ellipse cx="${(a.rumpX + 0.22).toFixed(3)}" cy="${(a.top - 0.06).toFixed(3)}" rx="0.2" ry="0.13"/>`
      + `<ellipse cx="${(a.chestX - 0.18).toFixed(3)}" cy="${(a.bottom + 0.12).toFixed(3)}" rx="0.13" ry="0.09"/></g>`);
    parts.push(`<clipPath id="neckclip"><path d="${poly(a.neck)}"/></clipPath>`);
    const n = a.neckPts[11];
    parts.push(`<g clip-path="url(#neckclip)" fill="${c.patch}" opacity="0.92"><ellipse cx="${n[0].toFixed(3)}" cy="${n[1].toFixed(3)}" rx="0.12" ry="0.2"/></g>`);
    parts.push(`<path d="${poly(a.head)}" fill="${c.patch}" opacity="0.92"/>`);
  }
  if (k.blanket) {
    // a saddle blanket over the back, following it, striped, its lower edge fringed
    const b = k.blanket, n = a.back.length - 1;
    const topEdge = a.back.slice(Math.round(b.from * n), Math.round(b.to * n) + 1).map((p) => [p[0], p[1] + 0.012]);
    // it hangs below the back (the back without its humps) by its drop, a little lower in the middle
    const bottomEdge = topEdge.map((p, i) => [p[0], a.top - 0.03 - b.drop - 0.03 * Math.sin(Math.PI * i / (topEdge.length - 1))]).reverse();
    const shape = [...topEdge, ...bottomEdge];
    parts.push(`<clipPath id="blanket"><path d="${poly(shape)}"/></clipPath>`);
    parts.push(`<path d="${poly(shape)}" fill="${b.colours[0]}" stroke="${INK}" stroke-width="${ink * 0.9}" stroke-linejoin="round"/>`);
    const y0 = Math.min(...bottomEdge.map((p) => p[1]));
    parts.push(`<g clip-path="url(#blanket)">`
      + [0.32, 0.6].map((f, i) => `<rect x="-3" y="${(y0 + f * b.drop).toFixed(4)}" width="6" height="${(b.drop * 0.13).toFixed(4)}" fill="${b.colours[1 + i]}"/>`).join('')
      + `</g>`);
    const fringe = bottomEdge.filter((_, i) => i % 1 === 0).map((p) => `<path d="${line([p, add(p, [0, -0.035])])}"/>`).join('');
    parts.push(`<g stroke="${b.colours[1]}" stroke-width="${fine * 1.3}" stroke-linecap="round">${fringe}</g>`);
  }
  if (k.tassels) {
    // yarn tassels in the ears
    for (const [i, e] of a.ears.entries()) {
      const t = add(e.base, mul(sub(e.tip, e.base), 0.55));
      for (const [j, colour] of k.tassels.entries()) {
        const q = add(t, [0.012 - j * 0.03, -0.025 - j * 0.012]);
        parts.push(`<circle cx="${q[0].toFixed(4)}" cy="${q[1].toFixed(4)}" r="0.028" fill="${colour}" stroke="${INK}" stroke-width="${fine}"/>`);
      }
    }
  }
  if (k.mane) {
    // long hair under the neck and on the forearms (the Bactrian's winter coat; the Arctic giant's, guessed)
    const hair = [];
    for (let i = 2; i < 12; i++) {
      const p = a.neckPts[i], q = a.neckPts[i + 1], dn = norm(perp(sub(q, p)));
      const under = add(p, mul(dn, -(lerp(k.neck.base, k.neck.top, i / 14) / 2) * (1 + k.wool * 2.5)));
      hair.push(`<path d="${line([under, add(under, [0.01, -0.06 * k.mane - 0.02 * (i % 2)])])}"/>`);
    }
    for (const l of a.legs.filter((x) => x.which === 'fore')) {
      const p = add(l.knee, mul(sub(l.knee, l.fetlock), -0.0));
      hair.push(`<path d="${line([add(p, [0.02, 0.12]), add(p, [0.04, 0.0])])}"/>`);
    }
    parts.push(`<g stroke="${c.shade}" stroke-width="${fine * 2.2}" fill="none" stroke-linecap="round">${hair.join('')}</g>`);
  }
  if (detail === 'close') {
    if (k.calluses) {
      for (const l of a.legs.filter((x) => x.which === 'fore')) {
        parts.push(`<ellipse cx="${l.knee[0].toFixed(4)}" cy="${l.knee[1].toFixed(4)}" rx="0.028" ry="0.022" fill="${c.shade}" opacity="${l.side === 'near' ? 0.8 : 0.5}"/>`);
      }
    }
    // the face
    parts.push(`<circle cx="${a.eye[0].toFixed(4)}" cy="${a.eye[1].toFixed(4)}" r="${(k.head.depth * 0.13).toFixed(4)}" fill="${INK}"/>`);
    parts.push(`<path d="${line([add(a.eye, [-0.02, 0.022]), add(a.eye, [0.025, 0.026])])}" stroke="${INK}" stroke-width="${fine}" fill="none" stroke-linecap="round"/>`);
    parts.push(`<path d="${line(a.mouth)}" stroke="${INK}" stroke-width="${fine}" fill="none" stroke-linecap="round"/>`);
    parts.push(`<circle cx="${a.nostril[0].toFixed(4)}" cy="${a.nostril[1].toFixed(4)}" r="${(k.head.depth * 0.07).toFixed(4)}" fill="${INK}" opacity="0.8"/>`);
    for (const e of a.ears) parts.push(`<path d="${line([e.base, add(e.base, mul(sub(e.tip, e.base), 0.7))])}" stroke="${INK}" stroke-width="${fine}" opacity="0.6" fill="none" stroke-linecap="round"/>`);
    // a few hatchings along the flank's shadow, as an engraver shades (above a pale underside)
    const hatch = [], hy = a.bottom + (k.twoTone ? (a.top - a.bottom) * 0.45 : 0.035);
    for (let i = 0; i < 5; i++) {
      const x = lerp(a.rumpX + 0.26, a.chestX - 0.26, i / 4) + 0.012 * Math.sin(i * 2.3), l = 0.04 + 0.018 * Math.sin(i * 1.7 + 1);
      hatch.push(`<path d="${line([[x, hy], [x + l * 0.55, hy + l]])}"/>`);
    }
    parts.push(`<g stroke="${c.shade}" stroke-width="${fine}" opacity="0.6" stroke-linecap="round">${hatch.join('')}</g>`);
    // the feet: a camel's broad pad, a llama's two toes
    for (const l of a.legs) {
      const f = l.foot, w = k.calluses ? 0.05 : 0.035;
      parts.push(`<path d="${line([[f[0] - w * 0.6, f[1] + 0.005], [f[0] + w, f[1] + 0.005]])}" stroke="${INK}" stroke-width="${fine * 1.2}" stroke-linecap="round" opacity="${l.side === 'near' ? 0.9 : 0.6}"/>`);
    }
  }
  return parts.join('');
}

/** Two colours mixed, `t` of the way from a to b. */
function mix(a, b, t) {
  const p = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
  const x = p(a), y = p(b);
  return '#' + x.map((v, i) => Math.round(lerp(v, y[i], t)).toString(16).padStart(2, '0')).join('');
}

// ---- frames ----------------------------------------------------------------------------------------
const WALK = 8;   // frames in a stride
const poses = {
  walk: Array.from({ length: WALK }, (_, i) => ({ gait: 'walk', phase: i / WALK })),
  // grazing: head down, chewing, now and then looking up (four drawings, shown in GRAZE's order)
  graze: [{ gait: 'graze', phase: 0, head: 1 }, { gait: 'graze', phase: 0, head: 1, chew: 1 },
          { gait: 'graze', phase: 0, head: 0.45 }, { gait: 'stand', phase: 0, head: 0 }]
};
const GRAZE = [0, 1, 0, 1, 2, 3, 3, 2];

/** The animal's extent over every pose, so its frames share one frame. */
function extent(k) {
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (const set of Object.values(poses)) for (const p of set) {
    const a = build(k, p);
    for (const s of [a.body, a.neck, a.head, a.tail, ...a.legs.map((l) => l.outline), ...a.ears.map((e) => e.outline)]) {
      for (const q of s) { x0 = Math.min(x0, q[0]); x1 = Math.max(x1, q[0]); y0 = Math.min(y0, q[1]); y1 = Math.max(y1, q[1]); }
    }
  }
  const m = 0.06;
  return [x0 - m, Math.min(y0, 0) - m, x1 + m, y1 + m];
}

// how each look is drawn: points per unit of withers height (the dromedary's), the outline in points
const LOOKS = {
  close: { ptPerH: 80, ink: 1.4, detail: 'close', scale: 3 },
  small: { ptPerH: 21, ink: 1.15, detail: 'small', scale: 3 },
  tiny: { ptPerH: 10.5, ink: 0.8, detail: 'tiny', scale: 3 }
};

function svgFor(k, pose, look, box) {
  const a = build(k, pose), [x0, y0, x1, y1] = box;
  const unit = look.ptPerH * Math.sqrt(k.size);       // smaller kinds drawn a little larger than true, to stay legible
  const w = (x1 - x0) * unit, h = (y1 - y0) * unit;
  const ink = look.ink / unit;
  const c = k.coat;
  // the coat, one gradient in the animal's own space for every part, so the parts meet without a seam:
  // the legs as the coat, paler under the belly (countershaded), the back a little darker
  const back = k.leg + k.depth, top = back + Math.max(0, ...k.humps.map((h) => h.h)), span = top + 1.2;
  const stop = (y, colour, opacity = 1) => `<stop offset="${Math.max(0, Math.min(1, y / span)).toFixed(3)}" stop-color="${colour}" stop-opacity="${opacity}"/>`;
  const defs = `<defs><linearGradient id="coat" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2="${span.toFixed(3)}">`
    + stop(0, c.base) + stop(k.leg * 0.8, c.base) + stop(k.leg + 0.04, c.light) + stop(back - 0.08, c.base)
    + stop(back + 0.02, mix(c.base, c.shade, 0.35)) + (top > back + 0.05 ? stop(top, mix(c.base, c.shade, 0.5)) : '')
    + stop(top + 0.18, c.base) + stop(span, c.base) + `</linearGradient>`
    + `<filter id="hand" x="-5%" y="-5%" width="110%" height="110%"><feTurbulence type="fractalNoise" baseFrequency="${(look.ptPerH * Math.sqrt(k.size) / 9).toFixed(3)}" numOctaves="2" seed="3"/>`
    + `<feDisplacementMap in="SourceGraphic" scale="${(1.1 / (look.ptPerH * Math.sqrt(k.size))).toFixed(4)}"/></filter></defs>`;
  // y up in the animal's frame, down in SVG's
  return {
    w, h,
    svg: `<svg xmlns="http://www.w3.org/2000/svg" width="${w * look.scale}" height="${h * look.scale}" viewBox="${x0} ${-y1} ${x1 - x0} ${y1 - y0}">${defs}`
      + (look.detail === 'close' ? `<ellipse cx="${((a.rumpX + a.chestX) / 2 + 0.05).toFixed(3)}" cy="-0.005" rx="${(k.body * 0.62).toFixed(3)}" ry="0.035" fill="${INK}" opacity="0.13"/>` : '')
      + `<g transform="scale(1,-1)">${draw(a, look.detail, ink)}</g></svg>`
  };
}

/** The herd's mark from afar: three tiny figures walking together, out of step. */
function herdSvg(k, frame, frames, look, box, graze) {
  const [x0, y0, x1, y1] = box, wU = x1 - x0, hU = y1 - y0;
  const unit = look.ptPerH * Math.sqrt(k.size);
  const places = [[0.0, 0.0, 1.0, 0.0], [-0.62, 0.42, 0.82, 0.37], [0.5, 0.52, 0.78, 0.71]];   // dx, dy (of the width), scale, phase
  // the frame round all three
  let bx0 = Infinity, by0 = Infinity, bx1 = -Infinity, by1 = -Infinity;
  for (const [dx, dy, sc] of places) {
    bx0 = Math.min(bx0, dx * wU + sc * x0); bx1 = Math.max(bx1, dx * wU + sc * x1);
    by0 = Math.min(by0, -dy * hU - sc * y1); by1 = Math.max(by1, -dy * hU - sc * y0);
  }
  const W = bx1 - bx0, Hh = by1 - by0;
  let body = '';
  for (const [dx, dy, sc, ph] of places.slice().reverse()) {
    const pose = graze ? poses.graze[0] : { gait: 'walk', phase: (frame / frames + ph) % 1 };
    const a = build(k, pose);
    body += `<g transform="translate(${(dx * wU).toFixed(3)},${(-dy * hU).toFixed(3)}) scale(${sc})"><g transform="scale(1,-1)">${draw(a, 'tiny', look.ink / unit)}</g></g>`;
  }
  const vx = bx0, vy = by0;
  return { w: W * unit, h: Hh * unit,
           svg: `<svg xmlns="http://www.w3.org/2000/svg" width="${W * unit * look.scale}" height="${Hh * unit * look.scale}" viewBox="${vx} ${vy} ${W} ${Hh}">${body}</svg>` };
}

async function main() {
  const args = process.argv.slice(2);
  const only = args.includes('--only') ? args[args.indexOf('--only') + 1].split(',') : null;
  const sheet = args.includes('--sheet') ? args[args.indexOf('--sheet') + 1] : null;
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch();
  const page = await browser.newPage({ deviceScaleFactor: 1 });
  const art = {};
  const sheetRows = [];
  for (const [id, k] of Object.entries(KINDS)) {
    if (only && !only.includes(id)) continue;
    const box = extent(k);
    art[id] = { name: k.name };
    for (const [lookId, look] of Object.entries(LOOKS)) {
      for (const [poseId, set] of Object.entries(poses)) {
        // a tiny figure grazes still (one drawing, head down); a tiny or small one walks in four
        const frames = lookId === 'tiny' && poseId === 'graze' ? [set[0]] : lookId !== 'close' && poseId === 'walk' ? set.filter((_, i) => i % 2 === 0) : set;
        const names = [];
        let size = null;
        for (let i = 0; i < frames.length; i++) {
          const { w, h, svg } = svgFor(k, frames[i], look, box);
          size = [Math.round(w * 10) / 10, Math.round(h * 10) / 10];
          const name = `${id}-${lookId}-${poseId}-${i + 1}.png`;
          await page.setViewportSize({ width: Math.ceil(w * look.scale), height: Math.ceil(h * look.scale) });
          await page.setContent(`<html><body style="margin:0;background:transparent">${svg}</body></html>`);
          await page.locator('svg').screenshot({ path: path.join(OUT, name), omitBackground: true });
          names.push('art/' + name);
          if (sheet && lookId === 'close') sheetRows.push({ id, poseId, i, svg, w: w * look.scale, h: h * look.scale });
        }
        art[id][lookId] = art[id][lookId] || {};
        // (the pack's file budget: a cycle shows its drawings again rather than drawing them twice)
        art[id][lookId][poseId] = { frames: poseId === 'graze' && names.length === 4 ? GRAZE.map((i) => names[i]) : names, size };
      }
    }
    // the herd's mark, from afar
    art[id].herd = {};
    for (const [poseId, count] of [['walk', 4], ['graze', 1]]) {
      const names = [];
      let size = null;
      for (let i = 0; i < count; i++) {
        const { w, h, svg } = herdSvg(k, i, count, LOOKS.tiny, box, poseId === 'graze');
        size = [Math.round(w * 10) / 10, Math.round(h * 10) / 10];
        const name = `${id}-herd-${poseId}-${i + 1}.png`;
        await page.setViewportSize({ width: Math.ceil(w * 3), height: Math.ceil(h * 3) });
        await page.setContent(`<html><body style="margin:0;background:transparent">${svg}</body></html>`);
        await page.locator('svg').screenshot({ path: path.join(OUT, name), omitBackground: true });
        names.push('art/' + name);
      }
      art[id].herd[poseId] = { frames: names, size };
    }
    console.log(id, JSON.stringify(art[id].close.walk.size), JSON.stringify(art[id].small.walk.size), JSON.stringify(art[id].tiny.walk.size));
  }
  // the index the pack reads: each kind's looks, their frames and sizes in points
  if (!only) {
    fs.writeFileSync(path.join(ROOT, 'Prototypes', 'herds', 'art.js'),
      '// Written by tools/herds/draw.cjs: the animals\' pictures and their sizes in points. Never hand-edit.\n'
      + 'window.HERD_ART = ' + JSON.stringify(art) + ';\n');
  }
  if (sheet) {
    // a contact sheet of the close illustrations, for looking at them together
    const html = `<html><body style="margin:0;background:#E8DFC8;display:flex;flex-wrap:wrap;gap:6px;padding:8px;width:2400px">`
      + sheetRows.map((r) => `<div style="width:${r.w / 2}px;height:${r.h / 2}px">${r.svg.replace(/width="[^"]+" height="[^"]+"/, `width="${r.w / 2}" height="${r.h / 2}"`)}</div>`).join('') + '</body></html>';
    await page.setViewportSize({ width: 2416, height: 800 });
    await page.setContent(html);
    await page.screenshot({ path: sheet, fullPage: true });
  }
  await browser.close();
}

if (require.main === module) main().catch((e) => { console.error(e); process.exit(1); });
module.exports = { KINDS, build };

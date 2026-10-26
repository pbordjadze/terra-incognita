// What every look's page shares: the plate size, the data loader, and where the next plate sits.
//
// A look's page (tools/plates/<look>/index.html) loads d3 and this kit, then defines
//   window.Plates = { levels, style, info(level) }
// and, loaded at ?level=N&layer=name, draws that layer into the page (one <svg> or <canvas> filling
// the viewport, after DOMContentLoaded) and sets window.platesReady = true (or a string saying what
// failed). The exporter (tools/plates/export.cjs) screenshots it: an opaque layer as JPEG, the others
// as PNG with their transparency, cropped to the layer's frame if it has one.
//
// info(level) returns
//   { id, child: [x, y, w, h] | null, caption: { eyebrow, title, text }, layers: [layer, ...] }
// child is where the next level's plate sits in this one (Kit.childRect), and a layer is
//   { name,
//     opaque: true,                 // the ground; one per plate, first
//     depth: 0.05,                  // parallax: how much further than the ground it moves with the
//                                   // camera, as a fraction of the camera's offset from the centre
//     frame: [x, y, w, h],          // a sprite: only this part of the plate (a creature, a ship)
//     motion: { kind, ... } }       // gentle life, in plate pixels and seconds:
//        bob     { amp: [x, y], period, phase }        moves back and forth
//        sway    { angle, period, phase, anchor: [x, y] }  turns a sprite about a point, degrees
//        breathe { min, max, period, phase }           opacity rises and falls
//        sheen   { period, width, angle, color, opacity }  this layer is a mask (gold, say): a band
//                                                      of light crosses it once a period
// Plates.style is the look's captions (the app sets them in native text over the map):
//   { fonts: ['fonts/Name.ttf', ...],                 // TrueType or OpenType files beside the page
//     eyebrow: { font, size, color, tracking, uppercase }, title: { font, size, color },
//     text: { font, size, color, italic }, scrim: '#00000088', accent: '#9c4a32' }
// font is a PostScript name from those files (or 'serif' for the system's); size is in points.
// Draw no caption into the plates, and keep the lowest fifth of the screen calm: the caption sits
// there.
//
// Plates are portrait, PLATE_W x PLATE_H pixels: about 1.4 phone screens across at 3x. At rest the
// screen shows the middle 1/1.4 of the plate each way, so compose for the middle and fill the whole.
'use strict';
const PLATE_W = 1638, PLATE_H = 3545;

const Kit = {
  W: PLATE_W, H: PLATE_H,
  // magnify is set for a pass plate: the level drawn magnified by that much about (cx, cy) of its own
  // plate. By default the exporter does it with a CSS transform on the page, which keeps SVG sharp
  // but blurs a canvas; a page that paints the magnification itself sets window.platesMagnifies.
  params() {
    const q = new URLSearchParams(location.search);
    return { level: +(q.get('level') || 1), layer: q.get('layer') || 'all',
             magnify: +(q.get('magnify') || 1), cx: +(q.get('cx') || PLATE_W / 2), cy: +(q.get('cy') || PLATE_H / 2) };
  },
  async json(path) { return (await fetch(path)).json(); },
  // The lens every level draws through: one orthographic projection centred on where the film ends
  // (High Street, Short North), its scale the level's. Plates drawn through it nest exactly, so as the
  // camera dives the closer plate fades up over the same coasts and streets, only drawn finer.
  // lens.km(width) is the scale at which the plate spans that many kilometres across.
  TARGET: [-83.005, 39.983],
  lens(scale) {
    return d3.geoOrthographic().rotate([-Kit.TARGET[0], -Kit.TARGET[1]]).translate([PLATE_W / 2, PLATE_H / 2])
      .scale(scale).clipAngle(90).precision(0.1);
  },
  km(width) { return PLATE_W * 6371 / width; },
  // (At the street scale d3's spherical clipping merges vertices closer than about 1e-6 radians,
  // some 6 m, folding small buildings into slivers: project such geometry point by point through the
  // lens instead of d3.geoPath, as ffern's flatPath does.)
  // The next plate's footprint in this plate's pixels: the child's centre where this projection puts
  // it, its width from the child's left and right edges, and the plate's aspect.
  childRect(parent, child) {
    const c = child.invert([PLATE_W / 2, PLATE_H / 2]);
    const l = child.invert([0, PLATE_H / 2]), r = child.invert([PLATE_W, PLATE_H / 2]);
    const pc = parent(c), pl = parent(l), pr = parent(r);
    const w = Math.hypot(pr[0] - pl[0], pr[1] - pl[1]), h = w * PLATE_H / PLATE_W;
    return [pc[0] - w / 2, pc[1] - h / 2, w, h].map(v => Math.round(v * 10) / 10);
  },
  // An SVG filling the plate, for d3 to draw into.
  svg() {
    document.body.style.margin = '0';
    return d3.select('body').append('svg').attr('width', PLATE_W).attr('height', PLATE_H)
      .attr('viewBox', `0 0 ${PLATE_W} ${PLATE_H}`).style('display', 'block');
  },
};
window.Kit = Kit;

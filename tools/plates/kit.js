// What every look's page shares: the plate size, the data loader, and where the next plate sits.
//
// A look's page (tools/plates/<look>/index.html) loads d3 and this kit, then defines
//   window.Plates = { levels, info(level), draw(level, layer) }
// where info(level) returns { id, child: [x, y, w, h] | null, caption: { eyebrow, title, text },
// layers: [{ name, depth, opaque }] } and draw(level, layer) draws that layer into the page (one
// <svg> or <canvas> filling the viewport) and resolves when it is done. The exporter
// (tools/plates/export.cjs) opens the page at ?level=N&layer=name, waits for window.platesReady and
// screenshots the viewport: an opaque layer as JPEG, the others as PNG with their transparency.
//
// Plates are portrait, PLATE_W x PLATE_H pixels: about 1.4 phone screens wide at 3x, so the camera
// can drift and pan without running off the picture.
'use strict';
const PLATE_W = 1638, PLATE_H = 3545;

const Kit = {
  W: PLATE_W, H: PLATE_H,
  params() {
    const q = new URLSearchParams(location.search);
    return { level: +(q.get('level') || 1), layer: q.get('layer') || 'all' };
  },
  async json(path) { return (await fetch(path)).json(); },
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

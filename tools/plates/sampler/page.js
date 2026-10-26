// The page around the stitching: one canvas filling the plate, painted from a scene through a view.
//
// A pass plate is the level magnified by s about a point (cx, cy): the exporter asks for it with
// ?magnify=s&cx=&cy=, and the page stitches the scene through that view at full resolution and says so
// (window.platesMagnifies), so the stitches come out larger and sharp, each thread's twist visible,
// rather than a canvas blown up. An older exporter instead magnifies the page with a CSS transform on
// <html>: the page watches for that, stitches again through it and takes the transform off.
'use strict';
const Page = {
  canvas: null, scene: null,
  paint(view) {
    const t0 = performance.now();
    const img = Stitch.render(this.scene, view, Kit.W, Kit.H);
    const ctx = this.canvas.getContext('2d');
    ctx.putImageData(img, 0, 0);
    if (this.scene.after) this.scene.after(ctx, view);
    this.ms = performance.now() - t0;
  },
  // build(level, layer) resolves to a scene, or null for a layer with nothing on it at this level
  async run(build) {
    const { level, layer } = Kit.params();
    if (layer === 'none') return;
    if (document.readyState === 'loading') await new Promise(r => addEventListener('DOMContentLoaded', r));
    document.body.style.margin = '0';
    const c = this.canvas = document.createElement('canvas');
    c.width = Kit.W; c.height = Kit.H;
    c.style.display = 'block';
    document.body.appendChild(c);
    try {
      this.scene = await build(level, layer);
      if (this.scene.after) {
        const face = new FontFace('Caslon', 'url(fonts/LibreCaslonText-Italic.ttf)', { style: 'italic' });
        document.fonts.add(await face.load());
      }
      const q = new URLSearchParams(location.search), s = +(q.get('magnify') || 1);
      const view = s === 1 ? { s: 1, tx: 0, ty: 0 } : { s, tx: Kit.W / 2 - +q.get('cx') * s, ty: Kit.H / 2 - +q.get('cy') * s };
      this.paint(view);
      const root = document.documentElement;
      new MutationObserver(() => {
        const tr = root.style.transform;
        if (!tr || tr === 'none') return;
        const m = new DOMMatrix(tr);
        this.paint({ s: m.a, tx: m.e, ty: m.f });
        root.style.transform = '';
        root.style.transformOrigin = '';
      }).observe(root, { attributes: true, attributeFilter: ['style'] });
      window.platesMagnifies = true;      // pass plates are stitched magnified here, not blown up by the exporter
      window.platesReady = true;
    } catch (e) {
      console.error(e);
      window.platesReady = 'failed: ' + (e && e.stack || e);
    }
  },
};
window.Page = Page;

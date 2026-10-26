// The app: one canvas of characters, a canvas of lettering over it, the tour (tour.js) driving
// the camera and the styles, and the reader free to take the map in hand at any moment.
'use strict';

const App = {
  async start() {
    this.gl = document.getElementById('gl');
    this.textCanvas = document.getElementById('text');
    this.tctx = this.textCanvas.getContext('2d');
    await Promise.race([document.fonts.ready, new Promise(r => setTimeout(r, 3000))]);
    await Promise.all(['700 20px "JetBrains Mono"', '20px "IM Fell English SC"', 'italic 20px "IM Fell English"', '20px "Noto Sans Symbols 2"']
      .map(f => document.fonts.load(f, f.includes('Symbols') ? '♣♠' : 'A').catch(() => {})));
    TI.log('fonts');
    // iOS may take the GPU context away while the app is in the background: come back by
    // reloading (the address keeps the film's place, see Tour.remember)
    this.gl.addEventListener('webglcontextlost', e => { e.preventDefault(); this.lost = true; TI.log('webgl context lost'); });
    this.gl.addEventListener('webglcontextrestored', () => { TI.log('webgl context restored: reloading'); location.reload(); });
    this.renderer = new TI.Renderer(this.gl, this.textCanvas);
    TI.log('webgl2 context');
    await this.renderer.init('');
    // the glyphs at the opening grain, drawn and uploaded before the first frame
    const g0 = TI.defaultState().grain, cw0 = Math.max(4, Math.round(g0 * Math.min(devicePixelRatio || 1, 3)));
    this.renderer.atlasFor(cw0, cw0 * 2);
    TI.log('glyph atlas');
    const places = await fetch('data/places.json').then(r => r.json());
    TI.log('places');
    this.scene = new TI.Scene(places);
    this.state = TI.defaultState();
    this.resize();
    addEventListener('resize', () => this.resize());
    this.t0 = performance.now();
    this.frames = 0; this.fpsAt = performance.now(); this.fps = 60;
    if (TI.Tour) this.tour = new TI.Tour(this);
    TI.log('tour');
    this.n = 0; this.fpsLogAt = performance.now();
    requestAnimationFrame(t => this.loop(t));
    this.ready = true;
  },
  resize() {
    this.W = innerWidth; this.H = innerHeight;
    this.dpr = Math.min(devicePixelRatio || 1, 3);
    for (const c of [this.gl, this.textCanvas]) { c.width = Math.round(this.W * this.dpr); c.height = Math.round(this.H * this.dpr); }
  },
  loop(now) {
    if (this.tour && this.tour.still && now - this.t0 > 2000) { if (this.n < 5) { this.n = 5; TI.log('still, ready'); } return; }
    requestAnimationFrame(t => this.loop(t));
    if (this.lost) return;
    // a frame's timestamp can fall a little before the moment the loop started
    const time = Math.max(0, (now - this.t0) / 1000);
    let a = this.state, b = null, mix = 0, kind = 0;
    if (this.tour) ({ a, b, mix, kind } = this.tour.frame(time, now));
    const va = this.scene.view(a, this.W, this.H, this.dpr, time, this.tctx);
    let vb = null;
    if (b && mix > 0 && mix < 1) vb = this.scene.view(b, this.W, this.H, this.dpr, time, this.tctx);
    const t1 = performance.now();
    if (TI.diag !== 'nogl') this.renderer.render(va, vb, mix, kind);
    const lv = vb && mix > 0.5 ? vb : va;
    const la = vb ? Math.abs(mix - 0.5) * 2 : 1;
    const t2 = performance.now();
    if (TI.diag !== 'no2d') this.scene.drawLabels(lv, this.tctx, (lv === vb ? (b.labels ?? 1) : (a.labels ?? 1)) * la, this.renderer, this.tour && this.tour.ui.veil(now));
    this.lastView = lv;
    this.frames++;
    // the first frames, timed (on the CPU: the GPU's work shows in when the next frame comes)
    if (++this.n <= 5) TI.log(`frame ${this.n}: map ${Math.round(t2 - t1)} ms, lettering ${Math.round(performance.now() - t2)} ms`);
    if (this.n === 5) TI.log('ready');
    if (now - this.fpsLogAt > 15000) { this.fpsLogAt = now; TI.log(`${this.fps.toFixed(0)} frames a second at ${this.tour ? this.tour.T.toFixed(0) : 0} s`); }
    if (now - this.fpsAt > 1000) { this.fps = this.frames * 1000 / (now - this.fpsAt); this.frames = 0; this.fpsAt = now; }
  },
};

TI.defaultState = () => ({
  proj: 0, look: { lon: 10, lat: 20, z: 1, ov: 1 }, ovalLon: 10,
  grain: innerWidth < 600 ? 2.6 : 3.2, ink: 'night', relief: 0, kells: 0,
  reveal: [1, 0, 0, 1], sun: [1, 0, 0, 0], sunAz: 2.3, labels: 1, ornaments: 1, date: Date.now(),
});

window.App = App;
App.start().catch(e => { console.error(e); TI.log('could not start: ' + e.message); const f = document.getElementById('fail'); f.hidden = false; f.textContent = 'Terra Incognita could not start: ' + e.message; });

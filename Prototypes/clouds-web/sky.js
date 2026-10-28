// Clouds over the globe: where they usually are in this month, as satellites have seen them
// (ISCCP-H 2000-2016: sky.png, made by tools/playground/clouds/build.py), drawn as weather that
// could be (shader.js). This file is the host: the camera, the month, the drift (a simulated day a
// minute; none under Reduce Motion, none while the app is in the background) and the drawing.
(function () {
  'use strict';

  var query = new URLSearchParams(location.search);      // the desktop preview's knobs (the app passes none)
  var canvas = document.getElementById('sky');
  var gl = canvas.getContext('webgl2', {
    alpha: true, premultipliedAlpha: true, antialias: false, depth: false, stencil: false,
    powerPreference: 'low-power',
  });
  if (!gl) { terra.log('clouds: no WebGL 2, so no clouds'); return; }

  var DAY_SECONDS = 60;                                    // a simulated day passes in a minute
  var FPS = 15;                                            // the drift is slow; the camera draws at its own pace
  var MAX_PIXELS = 1.6e6;                                  // the canvas's budget; clouds are soft

  // ---- the shader: shader.js's, over a full-screen triangle -----------------------------------------

  var VERT = '#version 300 es\n' +
    'in vec2 a; void main() { gl_Position = vec4(a, 0.0, 1.0); }';
  var FRAG = window.CLOUDS_FRAG;


  // ---- WebGL set-up -------------------------------------------------------------------------------

  var program, loc = {}, sky = null, land = null, noise = null, ready = false;

  function compile(type, src) {
    var s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
    return s;
  }

  function build() {
    program = gl.createProgram();
    gl.attachShader(program, compile(gl.VERTEX_SHADER, VERT));
    gl.attachShader(program, compile(gl.FRAGMENT_SHADER, FRAG));
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
    ['uSize', 'uPx', 'uDisc', 'uRot', 'uZoom', 'uReading', 'uDays', 'uMonth', 'uSky', 'uLand', 'uNoise', 'uSun', 'uSeaShadow', 'uLandShadow', 'uTone', 'uCyclone'].forEach(function (n) {
      loc[n] = gl.getUniformLocation(program, n);
    });
    var buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    var a = gl.getAttribLocation(program, 'a');
    gl.enableVertexAttribArray(a);
    gl.vertexAttribPointer(a, 2, gl.FLOAT, false, 0, 0);
  }

  // the measured sky (sky.png) and the land (land.png): data, not pictures, so taken as they are
  function texture(src, internal, format, wrapS, done) {
    var img = new Image();
    img.onload = function () {
      var t = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, t);
      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
      gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.NONE);
      gl.texImage2D(gl.TEXTURE_2D, 0, internal, format, gl.UNSIGNED_BYTE, img);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, wrapS);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      done(t);
    };
    img.onerror = function () { terra.log('clouds: ' + src + ' did not load'); };
    img.src = src;
  }

  // the noise's tile: gradient noise (shader.js's comment; calibrate.py), 16 cells a side wrapping
  // round, six samples a cell, -1..1 as 0..255
  var CELLS = 16, PER = 6, N = CELLS * PER;
  function pcg3d(x, y, z) {
    x = (Math.imul(x, 1664525) + 1013904223) >>> 0;
    y = (Math.imul(y, 1664525) + 1013904223) >>> 0;
    z = (Math.imul(z, 1664525) + 1013904223) >>> 0;
    x = (x + Math.imul(y, z)) >>> 0; y = (y + Math.imul(z, x)) >>> 0; z = (z + Math.imul(x, y)) >>> 0;
    x = (x ^ (x >>> 16)) >>> 0; y = (y ^ (y >>> 16)) >>> 0; z = (z ^ (z >>> 16)) >>> 0;
    x = (x + Math.imul(y, z)) >>> 0; y = (y + Math.imul(z, x)) >>> 0; z = (z + Math.imul(x, y)) >>> 0;
    return [x / 4294967295, y / 4294967295, z / 4294967295];
  }
  function noiseTile() {
    var g = new Float32Array(CELLS * CELLS * CELLS * 3), i, x, y, z;
    for (i = 0, z = 0; z < CELLS; z++) for (y = 0; y < CELLS; y++) for (x = 0; x < CELLS; x++, i += 3) {
      var h = pcg3d(x, y, z);
      g[i] = h[0] * 2 - 1; g[i + 1] = h[1] * 2 - 1; g[i + 2] = h[2] * 2 - 1;
    }
    var out = new Uint8Array(N * N * N), q = function (t) { return t * t * t * (t * (t * 6 - 15) + 10); };
    for (var k = 0, sz = 0; sz < N; sz++) for (var sy = 0; sy < N; sy++) for (var sx = 0; sx < N; sx++, k++) {
      var px = (sx + 0.5) / PER, py = (sy + 0.5) / PER, pz = (sz + 0.5) / PER;
      var ix = Math.floor(px), iy = Math.floor(py), iz = Math.floor(pz);
      var fx = px - ix, fy = py - iy, fz = pz - iz, v = 0;
      var ux = q(fx), uy = q(fy), uz = q(fz);
      for (var c = 0; c < 8; c++) {
        var dx = c & 1, dy = (c >> 1) & 1, dz = c >> 2;
        var j = ((((iz + dz) % CELLS) * CELLS + (iy + dy) % CELLS) * CELLS + (ix + dx) % CELLS) * 3;
        var dot = g[j] * (fx - dx) + g[j + 1] * (fy - dy) + g[j + 2] * (fz - dz);
        v += dot * (dx ? ux : 1 - ux) * (dy ? uy : 1 - uy) * (dz ? uz : 1 - uz);
      }
      out[k] = Math.max(0, Math.min(255, Math.round((v * 0.5 + 0.5) * 255)));
    }
    var t = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_3D, t);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage3D(gl.TEXTURE_3D, 0, gl.R8, N, N, N, 0, gl.RED, gl.UNSIGNED_BYTE, out);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T, gl.TEXTURE_WRAP_R].forEach(function (w) { gl.texParameteri(gl.TEXTURE_3D, w, gl.REPEAT); });
    return t;
  }

  function load(done) {
    noise = noiseTile();
    texture('sky.png', gl.RGB8, gl.RGB, gl.CLAMP_TO_EDGE, function (t) {
      sky = t;
      texture('land.png', gl.R8, gl.RED, gl.REPEAT, function (t) { land = t; done(); });
    });
  }

  // the cyclones (shader.js's cyclones()): where each is and how hard it turns, the same for every
  // pixel, so worked out once a frame. Two vec4s each: its centre and size, its turn and reach.
  var cycloneData = new Float32Array(80);
  function smooth(a, b, x) { var t = Math.min(Math.max((x - a) / (b - a), 0), 1); return t * t * (3 - 2 * t); }
  function cyclones(d) {
    for (var k = 0; k < 10; k++) {
      var hemi = k < 5 ? 1 : -1;
      var t = d / 6 + k * 0.2 + (hemi < 0 ? 0.1 : 0), n = Math.floor(t), age = t - n;
      var r = pcg3d(k, n, 977);
      var lat = hemi * (38 + 22 * r[1]) * Math.PI / 180, lon = 2 * Math.PI * r[0];
      var size = 0.09 + 0.07 * r[2];
      var wind = 2.6 * smooth(0, 0.55, age) * (1 - smooth(0.78, 1, age));
      cycloneData.set([Math.cos(lat) * Math.sin(lon), Math.sin(lat), Math.cos(lat) * Math.cos(lon), size,
                       -hemi * wind, Math.cos(3 * size), 0, 0], k * 8);
    }
    return cycloneData;
  }

  // ---- time: this month, and the drift --------------------------------------------------------------

  var started = performance.now();
  var heldDays = query.has('days') ? +query.get('days') : null;

  function monthNow() {
    // the day of the year, as months: mid-January is 0, mid-February 1 ...
    if (query.has('month')) return ((+query.get('month') - 1) % 12 + 12) % 12;
    var d = new Date(), start = new Date(d.getFullYear(), 0, 1);
    var day = (d - start) / 864e5;
    var lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31], m = 0;
    while (m < 11 && day >= lengths[m]) { day -= lengths[m]; m++; }
    return ((m + (day - lengths[m] / 2) / lengths[m]) % 12 + 12) % 12;
  }

  function days() {
    if (heldDays != null) return heldDays;
    if (terra.settings.reduceMotion) return 0;
    return (performance.now() - started) / 1000 / DAY_SECONDS;
  }

  // ---- drawing --------------------------------------------------------------------------------------

  var cam = null, px = 1, dirty = true, timer = null;

  function reading(w, h) {
    // Globe/type.json's reading scale: 1 at a short side of 430 pt, 1.2 at 1032
    return 1 + 0.2 * Math.min(Math.max((Math.min(w, h) - 430) / (1032 - 430), 0), 1);
  }

  function resize() {
    var w = window.innerWidth, h = window.innerHeight;
    px = Math.min(window.devicePixelRatio || 1, 1.5, Math.sqrt(MAX_PIXELS / Math.max(w * h, 1)));
    canvas.width = Math.max(1, Math.round(w * px));
    canvas.height = Math.max(1, Math.round(h * px));
    dirty = true;
    schedule();
  }

  function rotation(lon, lat) {
    // world from view, Ry(lon) . Rx(-lat) (GlobeView.rotation), column-major
    var a = lon * Math.PI / 180, b = -lat * Math.PI / 180;
    var ca = Math.cos(a), sa = Math.sin(a), cb = Math.cos(b), sb = Math.sin(b);
    // rows: [ca, sa*sb, sa*cb], [0, cb, -sb], [-sa, ca*sb, ca*cb]
    return new Float32Array([ca, 0, -sa, sa * sb, cb, ca * sb, sa * cb, -sb, ca * cb]);
  }

  function draw() {
    dirty = false;
    if (!ready || !cam) return;
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.useProgram(program);
    var m = monthNow();
    gl.uniform2f(loc.uSize, canvas.width, canvas.height);
    gl.uniform1f(loc.uPx, px);
    gl.uniform3f(loc.uDisc, cam.cx, cam.cy, cam.radius);
    gl.uniformMatrix3fv(loc.uRot, false, rotation(cam.lon, cam.lat));
    gl.uniform1f(loc.uZoom, cam.zoom);
    gl.uniform1f(loc.uReading, reading(cam.width || window.innerWidth, cam.height || window.innerHeight));
    var d = days();
    gl.uniform1f(loc.uDays, d);
    gl.uniform4fv(loc.uCyclone, cyclones(d));
    gl.uniform3f(loc.uMonth, Math.floor(m), (Math.floor(m) + 1) % 12, m - Math.floor(m));
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, sky);
    gl.uniform1i(loc.uSky, 0);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, land);
    gl.uniform1i(loc.uLand, 1);
    gl.activeTexture(gl.TEXTURE2);
    gl.bindTexture(gl.TEXTURE_3D, noise);
    gl.uniform1i(loc.uNoise, 2);
    // the Wikipedia map's look (CloudsLook in the app): shadows blue-grey on the sea, the globe's own
    // warm grey on land, lit from the north-west (no sun)
    gl.uniform4f(loc.uSun, 0, 0, 0, 0);
    gl.uniform4f(loc.uSeaShadow, 0.20, 0.26, 0.38, 1);
    gl.uniform4f(loc.uLandShadow, 0.43, 0.39, 0.32, 0.8);
    gl.uniform4f(loc.uTone, 1, 0.75, 0, 0);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  var frameRequested = false;
  function schedule() {
    if (frameRequested) return;
    frameRequested = true;
    requestAnimationFrame(function () { frameRequested = false; draw(); });
  }

  function tick() {
    // the drift: a frame every 1/FPS s while active and moving; the camera asks for its own
    clearInterval(timer);
    timer = null;
    if (!terra.settings.active || terra.settings.reduceMotion || heldDays != null) return;
    timer = setInterval(schedule, 1000 / FPS);
  }

  terra.on('camera', function (c) { cam = c; schedule(); });
  terra.on('settings', function () { tick(); schedule(); });
  window.addEventListener('resize', resize);
  canvas.addEventListener('webglcontextlost', function (e) { e.preventDefault(); ready = false; });
  canvas.addEventListener('webglcontextrestored', function () { start(); });

  function start() {
    try { build(); } catch (e) { terra.log('clouds: ' + e.message); return; }
    load(function () {
      ready = true;
      document.documentElement.setAttribute('data-clouds', 'ready');
      schedule();
    });
  }

  // the desktop preview's pictures and films draw on demand, at a chosen moment
  window.__clouds = { draw: draw, hold: function (d) { heldDays = d; tick(); } };
  resize();
  setTimeout(start, 0);                                   // after the page has said it is ready
  tick();
})();

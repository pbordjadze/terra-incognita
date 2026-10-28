// The ocean over the globe, the desktop twin of App/TerraIncognita/Ocean/OceanLayer.swift: the
// camera, the month, the sea's own time, the touches and the drawing (the shader is shader.js's,
// Ocean.metal line for line). For the preview's pictures and films only: the app draws natively.
(function () {
  'use strict';

  var query = new URLSearchParams(location.search);
  var canvas = document.getElementById('sea');
  var gl = canvas.getContext('webgl2', {
    alpha: true, premultipliedAlpha: true, antialias: false, depth: false, stencil: false,
  });
  if (!gl) { terra.log('ocean: no WebGL 2'); return; }

  var MAX_PIXELS = 4.5e6;
  var GLOBE = '../../../Globe/', SEA = '../../../App/TerraIncognita/Ocean/';

  var VERT = '#version 300 es\nin vec2 a; void main() { gl_Position = vec4(a, 0.0, 1.0); }';
  var program, loc = {}, tex = {}, ready = false;

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
    gl.attachShader(program, compile(gl.FRAGMENT_SHADER, window.OCEAN_FRAG));
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
    ['uSize', 'uPx', 'uDisc', 'uRot', 'uZoom', 'uReading', 'uTime', 'uMonth', 'uTone', 'uRipple',
     'uCurrents', 'uStream', 'uGeo', 'uIce', 'uNoise'].forEach(function (n) {
      loc[n] = gl.getUniformLocation(program, n);
    });
    var buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    var a = gl.getAttribLocation(program, 'a');
    gl.enableVertexAttribArray(a);
    gl.vertexAttribPointer(a, 2, gl.FLOAT, false, 0, 0);
  }

  // data, not pictures: taken as they are; 'nearest' for the stream function's joined bytes
  function texture(src, internal, format, filter, wrapS, done) {
    var img = new Image();
    img.onload = function () {
      var t = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, t);
      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
      gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.NONE);
      gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
      gl.texImage2D(gl.TEXTURE_2D, 0, internal, format, gl.UNSIGNED_BYTE, img);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, wrapS);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      done(t);
    };
    img.onerror = function () { terra.log('ocean: ' + src + ' did not load'); };
    img.src = src;
  }

  // the clouds' noise tile (tools/clouds/web/sky.js), the same numbers
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

  function month1() { return query.has('month') ? +query.get('month') : new Date().getMonth() + 1; }

  function load(done) {
    tex.noise = noiseTile();
    var m = ('0' + month1()).slice(-2);
    var jobs = [
      ['currents', SEA + 'OceanCurrents.png', gl.RGB8, gl.RGB, gl.LINEAR, gl.CLAMP_TO_EDGE],
      ['stream', SEA + 'OceanStream.png', gl.RGB8, gl.RGB, gl.NEAREST, gl.CLAMP_TO_EDGE],
      ['geo', GLOBE + 'geo.png', gl.RGBA8, gl.RGBA, gl.LINEAR, gl.REPEAT],
      ['ice', GLOBE + 'ground/ice-' + m + '.png', gl.R8, gl.RED, gl.LINEAR, gl.REPEAT],
    ];
    var left = jobs.length;
    jobs.forEach(function (j) {
      texture(j[1], j[2], j[3], j[4], j[5], function (t) { tex[j[0]] = t; if (--left === 0) done(); });
    });
  }

  // ---- time, touches, drawing ---------------------------------------------------------------------

  var started = performance.now();
  var heldTime = query.has('time') ? +query.get('time') : null;
  function time() { return heldTime != null ? heldTime : (performance.now() - started) / 1000; }
  var ripples = new Float32Array(32);                     // four: centre and when, then strength

  var cam = null, px = 1;
  function reading(w, h) {
    return 1 + 0.2 * Math.min(Math.max((Math.min(w, h) - 430) / (1032 - 430), 0), 1);
  }
  function resize() {
    var w = window.innerWidth, h = window.innerHeight;
    px = Math.min(window.devicePixelRatio || 1, 3, Math.sqrt(MAX_PIXELS / Math.max(w * h, 1)));
    canvas.width = Math.max(1, Math.round(w * px));
    canvas.height = Math.max(1, Math.round(h * px));
  }
  function rotation(lon, lat) {
    var a = lon * Math.PI / 180, b = -lat * Math.PI / 180;
    var ca = Math.cos(a), sa = Math.sin(a), cb = Math.cos(b), sb = Math.sin(b);
    return new Float32Array([ca, 0, -sa, sa * sb, cb, ca * sb, sa * cb, -sb, ca * cb]);
  }

  function draw() {
    if (!ready || !cam) return;
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.useProgram(program);
    var m = month1() - 1;
    gl.uniform2f(loc.uSize, canvas.width, canvas.height);
    gl.uniform1f(loc.uPx, px);
    gl.uniform3f(loc.uDisc, cam.cx, cam.cy, cam.radius);
    gl.uniformMatrix3fv(loc.uRot, false, rotation(cam.lon, cam.lat));
    gl.uniform1f(loc.uZoom, cam.zoom);
    gl.uniform1f(loc.uReading, reading(cam.width || window.innerWidth, cam.height || window.innerHeight));
    gl.uniform1f(loc.uTime, time());
    gl.uniform3f(loc.uMonth, m, m, 0);
    gl.uniform4f(loc.uTone, 1, query.get('still') ? 0 : 1, query.get('still') ? 0 : time(), 0);
    gl.uniform4fv(loc.uRipple, ripples);
    [['uCurrents', 'currents'], ['uStream', 'stream'], ['uGeo', 'geo'], ['uIce', 'ice']].forEach(function (p, i) {
      gl.activeTexture(gl.TEXTURE0 + i);
      gl.bindTexture(gl.TEXTURE_2D, tex[p[1]]);
      gl.uniform1i(loc[p[0]], i);
    });
    gl.activeTexture(gl.TEXTURE4);
    gl.bindTexture(gl.TEXTURE_3D, tex.noise);
    gl.uniform1i(loc.uNoise, 4);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  // a touch on the water at (lon, lat), at the sea's time 'when'
  function touch(lon, lat, when, k) {
    var a = lon * Math.PI / 180, b = lat * Math.PI / 180, i = (k || 0) * 8;
    ripples.set([Math.cos(b) * Math.sin(a), Math.sin(b), Math.cos(b) * Math.cos(a), when, 1, 0, 0, 0], i);
  }

  terra.on('camera', function (c) { cam = c; requestAnimationFrame(draw); });
  window.addEventListener('resize', function () { resize(); requestAnimationFrame(draw); });

  function start() {
    try { build(); } catch (e) { terra.log('ocean: ' + e.message); return; }
    load(function () {
      ready = true;
      document.documentElement.setAttribute('data-ocean', 'ready');
    });
  }

  window.__ocean = {
    draw: draw,
    hold: function (s) { heldTime = s; },
    touch: touch,
  };
  resize();
  setTimeout(start, 0);
})();

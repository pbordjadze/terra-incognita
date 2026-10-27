// A quiet ring grows where the sea is tapped. It stays on its place as the globe turns.
// Under Reduce Motion there is one static ring that fades, with nothing growing.
(function () {
  'use strict';
  var INK = '#3B2C16';
  var GROW_MS = 1600, FADE_MS = 1200, REACH = 64, STILL = 30;   // points

  var canvas = document.getElementById('rings');
  var g = canvas.getContext('2d');
  var rings = [];            // { lon, lat, born, still }
  var running = false;

  function resize() {
    var dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(window.innerWidth * dpr);
    canvas.height = Math.round(window.innerHeight * dpr);
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    draw(performance.now());
  }

  function draw(now) {
    g.clearRect(0, 0, window.innerWidth, window.innerHeight);
    rings = rings.filter(function (r) { return now - r.born < (r.still ? FADE_MS : GROW_MS); });
    rings.forEach(function (r) {
      var p = terra.camera ? terra.project(r.lon, r.lat) : null;
      if (!p || p.facing < 0.05) return;                    // behind the globe: nothing to draw
      var t = (now - r.born) / (r.still ? FADE_MS : GROW_MS);
      var radius = r.still ? STILL : REACH * (1 - Math.pow(1 - t, 3));
      g.globalAlpha = 0.7 * (1 - t);
      g.strokeStyle = INK;
      g.lineWidth = 1.5;
      g.beginPath();
      g.arc(p.x, p.y, Math.max(radius, 1), 0, 2 * Math.PI);
      g.stroke();
    });
  }

  function frame(now) {
    draw(now);
    if (rings.length) requestAnimationFrame(frame); else running = false;
  }

  terra.on('camera', function () { if (!running) draw(performance.now()); });

  terra.on('tap', function (tap) {
    if (tap.lon == null) return;                            // the tap missed the globe
    var still = terra.settings.reduceMotion;
    rings = still ? [] : rings;                             // reduced motion: exactly one ring at a time
    rings.push({ lon: tap.lon, lat: tap.lat, born: performance.now(), still: still });
    if (!running) { running = true; requestAnimationFrame(frame); }
  });

  window.addEventListener('resize', resize);
  resize();
})();

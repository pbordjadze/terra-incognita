/* The packs' shared script (docs/runtime3.md › Shared script): small helpers every pack would otherwise
   write again. A pack that uses it carries a copy, `shared.js`, first in its "scripts" (a pack reaches
   nothing outside its folder, so the copy ships with it and needs no new app); `playground.py share`
   writes the copies and `playground.py validate` fails a copy that differs from this file. Change it
   compatibly: every pack that carries it takes the change with its next revision.

     Shared.along([lon, lat], [lon, lat], t)   the place t (0–1) of the way along the great circle
     Shared.arc([lon, lat], [lon, lat])        the angle between two places, in radians
     Shared.unit([lon, lat])                   a place's unit vector [x, y, z] (the runtime's axes)
     Shared.frames('whale', 8)                 ['whale-1.png', …, 'whale-8.png'], the art drawer's names */
(function (global) {
  'use strict';
  var D = Math.PI / 180;
  function unit(p) { var la = p[1] * D, lo = p[0] * D; return [Math.cos(la) * Math.sin(lo), Math.sin(la), Math.cos(la) * Math.cos(lo)]; }
  function arc(a, b) { var u = unit(a), v = unit(b); return Math.acos(Math.max(-1, Math.min(1, u[0] * v[0] + u[1] * v[1] + u[2] * v[2]))); }
  function along(a, b, t) {
    var u = unit(a), v = unit(b), w = arc(a, b);
    if (w < 1e-9) return [a[0], a[1]];
    var sa = Math.sin((1 - t) * w) / Math.sin(w), sb = Math.sin(t * w) / Math.sin(w);
    var p = [u[0] * sa + v[0] * sb, u[1] * sa + v[1] * sb, u[2] * sa + v[2] * sb];
    return [Math.atan2(p[0], p[2]) / D, Math.asin(Math.max(-1, Math.min(1, p[1]))) / D];
  }
  function frames(name, n) { var out = []; for (var i = 1; i <= n; i++) out.push(name + '-' + i + '.png'); return out; }
  global.Shared = { unit: unit, arc: arc, along: along, frames: frames };
})(typeof globalThis !== 'undefined' ? globalThis : this);

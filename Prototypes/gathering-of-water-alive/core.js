/* A gathering of water, alive: the river network as a graph, as gathering-of-water/core.js (a pack
   carries its own scripts), with its stretches' lengths. Geographic connectivity only: no runoff,
   velocity, rainfall or flood model. Ported from the study's Water/core.js (its nearest line, its way
   downstream and its tree upstream), on the globe's longitude and latitude instead of a flat map,
   with the drawing levels and nearest-line grid that tools/water/build_pack.py prepares in
   network.json. It runs interpreted (JavaScriptCore in the app has no JIT), so it decodes nothing at
   start and reuses the parsed [lon, lat] pairs for every line it hands the globe. */
(function (root) {
  'use strict';
  var D = Math.PI / 180;

  function build(data) {
    var n = data.next.length, edges = new Array(n), inland = {};
    var finest = data.levels.length, grid = data.grid.cells, CELL = data.grid.cell;
    data.inland.forEach(function (i) { inland[i] = true; });
    for (var i = 0; i < n; i++) {
      var c = data.lines[i], w = Infinity, s = Infinity, e = -Infinity, nn = -Infinity;
      for (var j = 0; j < c.length; j++) {
        var lon = c[j][0], lat = c[j][1];
        if (lon < w) w = lon; if (lon > e) e = lon; if (lat < s) s = lat; if (lat > nn) nn = lat;
      }
      edges[i] = { i: i, next: data.next[i], order: data.order[i], km: data.km10[i] / 10, area: data.area[i],
                   c: c, lod: data.lod[i], drawn: [], cum: null, length: 0, bounds: [w, s, e, nn], up: [], outlet: -1 };
    }
    edges.forEach(function (e) { if (e.next >= 0) edges[e.next].up.push(e.i); });
    edges.forEach(function (e) {
      if (e.outlet >= 0) return;
      var chain = [], j = e.i;
      while (edges[j].next >= 0 && edges[j].outlet < 0) { chain.push(j); j = edges[j].next; }
      var o = edges[j].outlet >= 0 ? edges[j].outlet : j;
      edges[j].outlet = o;
      chain.forEach(function (k) { edges[k].outlet = o; });
    });
    edges.forEach(function (e) { e.inland = !!inland[e.outlet]; });

    /** The nearest point on a line of at least minOrder within radius degrees of lon, lat. */
    function nearest(lon, lat, radius, minOrder) {
      var k = Math.max(0.05, Math.cos(lat * D)), rx = radius / k, seen = {}, hit = null, best = radius * radius;
      for (var gx = Math.floor((lon - rx) / CELL); gx <= Math.floor((lon + rx) / CELL); gx++) {
        for (var gy = Math.floor((lat - radius) / CELL); gy <= Math.floor((lat + radius) / CELL); gy++) {
          var cell = grid[(gx + 500) * 1000 + gy + 500];
          if (!cell) continue;
          for (var q = 0; q < cell.length; q++) {
            var id = cell[q];
            if (seen[id]) continue;
            seen[id] = true;
            var e = edges[id];
            if (e.order < (minOrder || 1)) continue;
            var h = onEdge(e, lon, lat, k);
            if (h.d2 <= best) { best = h.d2; hit = h; }
          }
        }
      }
      if (hit) hit.distance = Math.sqrt(best);
      return hit;
    }

    /** Where on this stretch a place is nearest: { id, lon, lat, segment (its end vertex), t, d2 }. */
    function onEdge(e, lon, lat, k) {
      k = k || Math.cos(lat * D);
      var c = e.c, best = Infinity, hit = null;
      for (var j = 1; j < c.length; j++) {
        var ax = c[j - 1][0], ay = c[j - 1][1], dx = (c[j][0] - ax) * k, dy = c[j][1] - ay, dd = dx * dx + dy * dy;
        var px = (lon - ax) * k, py = lat - ay;
        var t = dd ? Math.max(0, Math.min(1, (px * dx + py * dy) / dd)) : 0;
        var d = (px - t * dx) * (px - t * dx) + (py - t * dy) * (py - t * dy);
        if (d <= best) {
          best = d;
          hit = { id: e.i, lon: ax + t * (c[j][0] - ax), lat: ay + t * (c[j][1] - ay), segment: j, t: t, d2: d };
        }
      }
      return hit;
    }

    /** A stretch's length along its line, vertex by vertex (worked out only for stretches touched). */
    function measured(e) {
      if (e.cum) return e;
      var c = e.c, cum = [0];
      for (var j = 1; j < c.length; j++) cum.push(cum[j - 1] + span(c[j - 1][0], c[j - 1][1], c[j][0], c[j][1]));
      e.cum = cum;
      e.length = cum[cum.length - 1];
      return e;
    }

    /** How far along its stretch a hit lies, in the stretch's own measure (to compare two hits). */
    function along(hit) {
      var e = measured(edges[hit.id]);
      return e.cum[hit.segment - 1] + hit.t * (e.cum[hit.segment] - e.cum[hit.segment - 1]);
    }

    /** The way down from a hit to its outlet: the stretches, km along mapped channels, the outlet. */
    function downstream(hit) {
      var e = measured(edges[hit.id]);
      // the first, partial stretch is estimated along its line; every later one is its summed reach lengths
      var km = e.km * (1 - along(hit) / Math.max(e.length, 1e-9));
      var ids = [e.i], seen = {}, j = e.next;
      seen[e.i] = true;
      while (j >= 0 && !seen[j]) { seen[j] = true; ids.push(j); km += edges[j].km; j = edges[j].next; }
      return { hit: hit, ids: ids, km: km, end: ids[ids.length - 1] };
    }

    /** The stretches gathered above a hit (every one whose water passes it). */
    function upstream(hit) {
      var ids = [], stack = edges[hit.id].up.slice(), seen = {};
      seen[hit.id] = true;
      while (stack.length) {
        var i = stack.pop();
        if (seen[i]) continue;
        seen[i] = true;
        ids.push(i);
        Array.prototype.push.apply(stack, edges[i].up);
      }
      return { hit: hit, ids: ids };
    }

    /** A stretch's line at a drawing level, as [lon, lat] pairs (kept once made: the pairs are the
        parsed data's own, never changed); from/to cut it at a hit. */
    function line(e, lod, from, to) {
      var c = e.c, m = c.length, j;
      if (!from && !to) {
        if (e.drawn[lod]) return e.drawn[lod];
        var whole = c;
        if (lod < finest) {
          whole = [];
          for (j = 0; j < m; j++) if (e.lod.charCodeAt(j) - 48 <= lod) whole.push(c[j]);
        }
        e.drawn[lod] = whole;
        return whole;
      }
      var out = [], a = from ? from.segment : 0, b = to ? to.segment - 1 : m - 1;
      if (from) out.push([from.lon, from.lat]);
      for (j = a; j <= b; j++) if (e.lod.charCodeAt(j) - 48 <= lod) out.push(c[j]);
      if (to) out.push([to.lon, to.lat]);
      if (out.length === 1) out.push(out[0]);
      return out;
    }

    /** The way down as one line at a drawing level, from the hit to the outlet. */
    function routeLine(route, lod) {
      var out = line(edges[route.ids[0]], lod, route.hit, null);
      for (var r = 1; r < route.ids.length; r++) {
        var l = line(edges[route.ids[r]], lod);
        for (var q = 1; q < l.length; q++) out.push(l[q]);
      }
      return out;
    }

    /** A stretch's whole length along its line, in degrees of arc (as `along` measures). */
    function length(i) { return measured(edges[i]).length; }

    return { edges: edges, nearest: nearest, onEdge: onEdge, along: along, downstream: downstream,
             upstream: upstream, line: line, routeLine: routeLine, length: length };
  }

  /** Degrees of arc, near enough, between two places a stretch apart (for proportions along a line). */
  function span(lon0, lat0, lon1, lat1) {
    var k = Math.cos((lat0 + lat1) / 2 * D);
    return Math.sqrt((lon1 - lon0) * (lon1 - lon0) * k * k + (lat1 - lat0) * (lat1 - lat0));
  }

  root.WaterNet = { build: build, span: span };
})(typeof globalThis !== 'undefined' ? globalThis : this);

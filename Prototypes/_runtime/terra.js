// The playground's runtime, version 6 (5 with sprite sheets, and dated geography: lines and points with a
// date at each place, drawn as far as the stage's date; 5 is 4 with sprites on dated ways and the date's
// pace; 4 is 3 with the stage shared: docs/runtime3.md › Many at once):
// the whole API a pack's script gets from its host (docs/runtime3.md is the contract). The app runs it in JavaScriptCore before a pack's scripts
// (App/TerraIncognita/Playground/PackScript.swift), and so does the desktop preview in a browser
// (tools/playground/harness/): a pack never includes it itself.
// The host provides `__terraSend(message)` (it may answer with a value) and `__terraBoot`, and calls
// `terra._receive(message)`. Change it compatibly, or raise `runtime` here, in PackStore.runtime
// (Swift) and in the docs, together.
(function (global) {
  'use strict';
  if (global.terra) return;
  var boot = global.__terraBoot || {};
  var host = global.__terraSend;
  var listeners = {};
  var D = Math.PI / 180;

  if (!global.window) global.window = global;

  function send(message) {
    try {
      return host ? host(message) : undefined;
    } catch (e) {
      return undefined; // a closed host has nothing to hear
    }
  }

  function report(e) {
    send({ type: 'log', level: 'error', message: String((e && (e.stack ? e.message + '\n' + e.stack : e.message)) || e) });
  }

  function emit(type, value) {
    (listeners[type] || []).slice().forEach(function (fn) {
      try { fn(value); } catch (e) { report(e); }
    });
  }

  // Timers, where the host has none of its own (JavaScriptCore): the host keeps the clock.
  if (typeof global.setTimeout !== 'function') {
    var timers = {}, nextTimer = 1;
    var start = function (fn, ms, repeat, args) {
      var id = nextTimer++;
      timers[id] = { fn: fn, args: args, repeat: repeat };
      send({ type: 'timer', op: 'set', id: id, ms: Math.max(0, +ms || 0), repeat: repeat });
      return id;
    };
    var stop = function (id) {
      if (timers[id]) { delete timers[id]; send({ type: 'timer', op: 'clear', id: id }); }
    };
    global.setTimeout = function (fn, ms) { return start(fn, ms, false, Array.prototype.slice.call(arguments, 2)); };
    global.setInterval = function (fn, ms) { return start(fn, ms, true, Array.prototype.slice.call(arguments, 2)); };
    global.clearTimeout = stop;
    global.clearInterval = stop;
    global.__terraTimer = function (id) {
      var t = timers[id];
      if (!t) return;
      if (!t.repeat) delete timers[id];
      try { if (typeof t.fn === 'function') t.fn.apply(global, t.args || []); } catch (e) { report(e); }
    };
  }

  // A clock and animation frames, where the host has none (JavaScriptCore). Frames ride the timers:
  // fine for a short eased change (a line let down); a pack never animates the globe frame by frame.
  if (!global.performance || typeof global.performance.now !== 'function') {
    var t0 = Date.now();
    global.performance = { now: function () { return Date.now() - t0; } };
  }
  if (typeof global.requestAnimationFrame !== 'function') {
    global.requestAnimationFrame = function (fn) {
      return global.setTimeout(function () { fn(global.performance.now()); }, 1000 / 60);
    };
    global.cancelAnimationFrame = function (id) { global.clearTimeout(id); };
  }

  function log(level, args) {
    send({ type: 'log', level: level, message: Array.prototype.map.call(args, function (a) {
      if (typeof a === 'string') return a;
      try { return JSON.stringify(a); } catch (e) { return String(a); }
    }).join(' ') });
  }
  if (!global.console || !global.console.__terra) {
    var previous = global.console || {};
    global.console = {
      __terra: true,
      log: function () { log('info', arguments); if (previous.log) previous.log.apply(previous, arguments); },
      info: function () { log('info', arguments); if (previous.info) previous.info.apply(previous, arguments); },
      warn: function () { log('warn', arguments); if (previous.warn) previous.warn.apply(previous, arguments); },
      error: function () { log('error', arguments); if (previous.error) previous.error.apply(previous, arguments); }
    };
  }

  function layerAPI(kind) {
    return {
      set: function (id, a, b) {
        if (kind === 'geo') send({ type: 'geo', op: 'set', id: String(id), features: a, options: b || {} });
        else send({ type: kind, op: 'set', id: String(id), options: a || {} });
      },
      remove: function (id) { send({ type: kind, op: 'remove', id: String(id) }); },
      clear: function () { send({ type: kind, op: 'clear' }); }
    };
  }

  var terra = {
    runtime: 6,
    /** { id, revision, origin: 'bundled' | 'downloaded' | 'preview' } */
    pack: boot.pack || { id: 'preview', revision: 0, origin: 'preview' },
    /** { reduceMotion, textScale (1 = the default iOS size, up to 3.2), active (false while backgrounded),
        wikipedia (the reader wants Wikipedia's words in place of the atlas's: keep your own lines to data) } */
    settings: Object.assign({ reduceMotion: false, textScale: 1, active: true, wikipedia: false }, boot.settings || {}),
    /** True while this is the experience in front, the one the reader is with (runtime 4: several run at
        once; the front one's date, caption, scheme and flights are the globe's). */
    focus: boot.focus !== false,
    /** How the reader came in: { by: 'doorway' | 'workbench' | 'link' | 'launch', lon, lat, zoom } (where
        the reader was). By a doorway, the pack starts where the reader is: no flight to its anchor. */
    entry: boot.entry || { by: 'workbench', lon: null, lat: null, zoom: null },

    /** 'settings', 'camera', 'touch' (and 'tap', its taps alone), 'time', 'open', 'panel', 'focus', 'close' */
    on: function (type, fn) {
      (listeners[type] = listeners[type] || []).push(fn);
      if (type === 'settings') { try { fn(terra.settings); } catch (e) { report(e); } }
      if (type === 'open') send({ type: 'listen', event: 'open' });
      return function () { terra.off(type, fn); };
    },
    off: function (type, fn) {
      listeners[type] = (listeners[type] || []).filter(function (f) { return f !== fn; });
    },
    /** End the prototype: back to the atlas. */
    close: function () { send({ type: 'close' }); },
    /** Move the globe's camera there (degrees; zoom 0.8–8; seconds). */
    flyTo: function (lon, lat, zoom, duration) {
      send({ type: 'flyTo', lon: +lon, lat: +lat, zoom: zoom == null ? null : +zoom, duration: duration == null ? 0.8 : +duration });
    },
    /** A file of this pack, as text (null if it has none by that name). */
    read: function (path) {
      var text = send({ type: 'read', path: String(path) });
      return typeof text === 'string' ? text : null;
    },
    readJSON: function (path) {
      var text = terra.read(path);
      if (text == null) return null;
      try { return JSON.parse(text); } catch (e) { report(e); return null; }
    },
    /** Where a place is on screen: { x, y, facing (1 at the centre, 0 at the limb, < 0 behind), visible }. */
    project: function (lon, lat, camera) {
      var c = camera || terra.camera;
      if (!c) return null;
      var a = c.lon * D, b = -c.lat * D, la = lat * D, lo = lon * D;
      var wx = Math.cos(la) * Math.sin(lo), wy = Math.sin(la), wz = Math.cos(la) * Math.cos(lo);
      var vx = Math.cos(a) * wx - Math.sin(a) * wz;
      var vy = Math.sin(a) * Math.sin(b) * wx + Math.cos(b) * wy + Math.cos(a) * Math.sin(b) * wz;
      var vz = Math.sin(a) * Math.cos(b) * wx - Math.sin(b) * wy + Math.cos(a) * Math.cos(b) * wz;
      return { x: c.cx + vx * c.radius, y: c.cy - vy * c.radius, facing: vz, visible: vz > 0 };
    },
    /** The place under a point of the screen, or null off the globe: { lon, lat }. */
    unproject: function (x, y, camera) {
      var c = camera || terra.camera;
      if (!c) return null;
      var vx = (x - c.cx) / c.radius, vy = (c.cy - y) / c.radius, r2 = vx * vx + vy * vy;
      if (r2 >= 1) return null;
      var vz = Math.sqrt(1 - r2), a = c.lon * D, b = -c.lat * D;
      var wx = Math.cos(a) * vx + Math.sin(a) * Math.sin(b) * vy + Math.sin(a) * Math.cos(b) * vz;
      var wy = Math.cos(b) * vy - Math.sin(b) * vz;
      var wz = -Math.sin(a) * vx + Math.cos(a) * Math.sin(b) * vy + Math.cos(a) * Math.cos(b) * vz;
      return { lon: Math.atan2(wx, wz) / D, lat: Math.asin(Math.max(-1, Math.min(1, wy))) / D };
    },
    log: function () { log('info', arguments); },

    /** Geography drawn by the globe: set(id, features, options), remove(id), clear(). */
    geo: layerAPI('geo'),
    /** Pictures lying on the globe: set(id, { image, bounds: [w, s, e, n], opacity, minZoom, maxZoom, fade }). */
    images: layerAPI('image'),
    /** Things that live and move on the globe: set(id, { at | path, looks, label, touchable }); a path with
        dates (runtime 5) is moved by the globe's date; a look's frame may be a frame of a sprite sheet,
        'sheet.png#x,y,w,h' (runtime 6). */
    sprites: layerAPI('sprite'),
    /** 'touchables' (drags that begin on a touchable thing are the pack's), 'one-finger' (all one-finger drags), or
        'hold' (a drag turns the globe; a finger that rests a moment first, then moves, is the pack's). */
    touch: {
      scheme: function (scheme) { send({ type: 'touch', op: 'scheme', scheme: String(scheme || 'touchables') }); }
    },
    /** The globe's date at the foot of the screen (show: unit, min, max or a pace (runtime 5), value, ticks, playable). */
    time: {
      show: function (options) { send({ type: 'time', op: 'show', options: options || {} }); },
      set: function (value) { send({ type: 'time', op: 'set', value: +value }); },
      play: function (playing) { send({ type: 'time', op: 'play', playing: playing !== false }); },
      /** How fast playback runs, 1 as written (runtime 5): slower as the reader comes close, say. */
      speed: function (factor) { send({ type: 'time', op: 'speed', speed: +factor }); },
      hide: function () { send({ type: 'time', op: 'hide' }); }
    },
    /** The words at the foot: what this is (title), then a line or two (text). */
    caption: {
      set: function (c) { send({ type: 'caption', title: (c && c.title) || '', text: (c && c.text) || '', wikipedia: (c && c.wikipedia) || '' }); }
    },
    /** { historical: hide today's borders and names, still: no idle turn, maxZoom: deeper close views (8–32) } */
    globe: {
      set: function (o) {
        var m = { type: 'globe', historical: !!(o && o.historical), still: !!(o && o.still) };
        if (o && o.maxZoom != null) m.maxZoom = +o.maxZoom;
        send(m);
      }
    },
    /** Longer words and their sources, in a sheet over the globe. */
    panel: {
      show: function (content) { send({ type: 'panel', op: 'show', content: content || {} }); },
      hide: function () { send({ type: 'panel', op: 'hide' }); }
    },

    /** The host's side: settings, camera, touch, time, timers, open, panel, focus, close. */
    _receive: function (message) {
      if (!message || typeof message !== 'object') return;
      switch (message.type) {
        case 'settings':
          Object.assign(terra.settings, message.settings || {});
          emit('settings', terra.settings);
          break;
        case 'focus':
          terra.focus = !!message.front;
          emit('focus', { front: terra.focus, by: message.by || 'host' });
          break;
        case 'camera':
          emit('camera', terra.camera);
          break;
        case 'touch':
          var touch = { phase: message.phase, target: message.target || null, x: message.x, y: message.y,
                        lon: message.lon == null ? null : message.lon, lat: message.lat == null ? null : message.lat };
          emit('touch', touch);
          // (taps alone, for a script that wants only those)
          if (touch.phase === 'tap') emit('tap', touch);
          break;
        case 'time':
          emit('time', { value: message.value, playing: !!message.playing, by: message.by });
          break;
        case 'timer':
          if (global.__terraTimer) global.__terraTimer(message.id);
          break;
        case 'open':
          emit('open', message.params || {});
          break;
        case 'panel':
          emit('panel', { open: !!message.open });
          break;
        case 'close':
          emit('close');
          break;
      }
    }
  };

  /** The camera now: { lon, lat, zoom, cx, cy, radius, width, height }, in points. */
  Object.defineProperty(terra, 'camera', {
    enumerable: true,
    get: function () {
      var c = send({ type: 'camera' });
      return c && typeof c === 'object' ? c : (boot.camera || null);
    }
  });

  global.terra = terra;
})(typeof globalThis !== 'undefined' ? globalThis : this);

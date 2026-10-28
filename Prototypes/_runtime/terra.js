// The playground's runtime, version 2: the whole API a web prototype gets from its host (version 1's,
// plus geo, time, caption and globe for packs drawn on the globe itself).
// The app injects this file before a pack's own scripts (App/TerraIncognita/Playground/WebPrototype.swift),
// and so does the desktop preview (tools/playground/serve.cjs): a page never includes it itself.
// Change it compatibly, or raise `runtime` here, in Playground.runtime (Swift) and in the docs, together:
// a pack declares the runtime it needs, and an app too old for it lists the pack as needing a new build.
(function () {
  'use strict';
  if (window.terra) return;
  var boot = window.__terraBoot || {};
  var handler = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.terra;
  var listeners = {};
  var D = Math.PI / 180;

  function send(message) {
    try {
      if (handler) handler.postMessage(message);
      else if (window.parent !== window) window.parent.postMessage({ terra: message }, '*');
      else console.log('[terra]', message);
    } catch (e) { /* a closed host has nothing to hear */ }
  }

  function emit(type, value) {
    (listeners[type] || []).slice().forEach(function (fn) {
      try { fn(value); } catch (e) { report(e); }
    });
  }

  function report(e) {
    send({ type: 'log', level: 'error', message: String(e && (e.stack || e.message) || e) });
  }

  function reflect(s) {
    // the reader's settings as CSS: `calc(17px * var(--terra-text-scale))`, `[data-reduce-motion]`
    var root = document.documentElement;
    if (!root) return;
    root.style.setProperty('--terra-text-scale', String(s.textScale));
    if (s.reduceMotion) root.setAttribute('data-reduce-motion', '');
    else root.removeAttribute('data-reduce-motion');
  }

  var terra = {
    runtime: 2,
    /** { id, revision, origin: 'bundled' | 'downloaded' | 'preview' } */
    pack: boot.pack || { id: 'preview', revision: 0, origin: 'preview' },
    /** 'screen' (its own surface over the paused globe) or 'overlay' (drawn over the live globe) */
    mode: boot.mode || 'screen',
    /** What opened it, e.g. { marker: 'shell' } */
    params: boot.params || {},
    /** { reduceMotion, textScale (1 = the default iOS size, up to 3.2), active (false while backgrounded) } */
    settings: Object.assign({ reduceMotion: false, textScale: 1, active: true }, boot.settings || {}),
    /** Overlay only: { lon, lat, zoom, cx, cy, radius, width, height }, in CSS pixels (= iOS points). */
    camera: boot.camera || null,

    /** 'settings', 'camera' (overlay), 'tap' (overlay with taps: { x, y, lon, lat }), 'time' (overlay,
     *  the globe's date: { value, playing, by: 'reader' | 'play' | 'host' }), 'close' */
    on: function (type, fn) {
      (listeners[type] = listeners[type] || []).push(fn);
      if (type === 'settings') fn(terra.settings);
      if (type === 'camera' && terra.camera) fn(terra.camera);
      return function () { terra.off(type, fn); };
    },
    off: function (type, fn) {
      listeners[type] = (listeners[type] || []).filter(function (f) { return f !== fn; });
    },
    /** Back to the globe (a screen), or end the prototype (an overlay). */
    close: function () { send({ type: 'close' }); },
    /** An overlay opens its pack's screen (prototype.json `entry`), with params. */
    open: function (params) { send({ type: 'open', params: params || {} }); },
    /** Overlay only: move the globe's camera there (degrees; zoom 0.8–8; seconds). */
    flyTo: function (lon, lat, zoom, duration) {
      send({ type: 'flyTo', lon: +lon, lat: +lat, zoom: zoom == null ? null : +zoom, duration: duration == null ? 0.8 : +duration });
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
    /** Runtime 2, overlay only: geography the app draws on the globe itself, in the globe's own style
     *  (its line widths, type and halos, at the reader's scale), moving with it at full frame rate.
     *  A layer is a list of GeoJSON features or bare geometries ([lon, lat] coordinates: Point,
     *  MultiPoint, LineString, MultiLineString, Polygon, MultiPolygon) and one style:
     *    'border'   a thin line in the globe's border ink         'area'   a soft fill and its outline
     *    'ghost'    a dashed outline (another map's, another time) 'track'  a fine dashed line
     *    'dot'      filled dots at points                          'ring'   hollow dots at points
     *    'label'    a name at each point (properties.label), on a halo; earlier features win room
     *    'symbol'   options.image (a file of the pack) at each point, options.width × options.height pt
     *    'sounding' a line hanging below each point, properties.length pt long, properties.label at its foot
     *  options: { style, color: '#rrggbb' (fills, dots, labels), minZoom, maxZoom (shown only between),
     *  font: 'regular' | 'italic' | 'caps' (labels), size (labels, pt), image, width, height (symbols),
     *  offset: [dx, dy] (labels and symbols: points from their place, right and down) }.
     *  Setting an id again replaces that layer; a later id draws above an earlier one. */
    geo: {
      set: function (id, features, options) {
        send({ type: 'geo', op: 'set', id: String(id), features: features || [], options: options || {} });
      },
      remove: function (id) { send({ type: 'geo', op: 'remove', id: String(id) }); },
      clear: function () { send({ type: 'geo', op: 'clear' }); }
    },

    /** Runtime 2, overlay only: the globe's date, at the foot of the screen, which the reader drags.
     *  show({ unit: 'year' | 'day', min, max, value, ticks: [at | { at, hollow }], playable, rate,
     *  playing, carry }): 'day' values count days since 1 January 1970 (UTC, negative before);
     *  ticks mark where the evidence is; rate is units a second while playing (playing starts it;
     *  never under Reduce Motion); carry starts from the date another prototype left, when it lies
     *  within min–max. The page hears every move by the reader, playback or the host as 'time' and
     *  redraws what it shows; its own set() is not echoed back. */
    time: {
      value: null,
      show: function (options) {
        var o = Object.assign({ unit: 'year', ticks: [], playable: false, rate: 1, playing: false, carry: false }, options || {});
        terra.time.value = o.value;
        send({ type: 'time', op: 'show', options: o });
      },
      set: function (value) { terra.time.value = +value; send({ type: 'time', op: 'set', value: +value }); },
      play: function (playing) { send({ type: 'time', op: 'play', playing: playing !== false }); },
      hide: function () { send({ type: 'time', op: 'hide' }); }
    },

    /** Runtime 2, overlay only: the words at the foot of the screen, in the globe's caption type
     *  (title above the date, text below it). Say what the reader is looking at before why it matters. */
    caption: {
      set: function (caption) {
        var c = caption || {};
        send({ type: 'caption', title: String(c.title || ''), text: String(c.text || '') });
      },
      clear: function () { send({ type: 'caption', title: '', text: '' }); }
    },

    /** Runtime 2, overlay only: how the globe itself behaves while this runs.
     *  { historical: true } hides today's borders and names (coasts stay, as a reference);
     *  { still: true } stops its slow idle turn. Both end with the prototype. */
    globe: {
      set: function (options) {
        var o = options || {};
        send({ type: 'globe', historical: !!o.historical, still: !!o.still });
      }
    },

    /** To the workbench's log (and the Xcode console). */
    log: function () {
      send({ type: 'log', level: 'info', message: Array.prototype.map.call(arguments, String).join(' ') });
    },

    /** The host's side: settings, camera, tap, close. */
    _receive: function (message) {
      if (!message || typeof message !== 'object') return;
      switch (message.type) {
        case 'settings':
          Object.assign(terra.settings, message.settings || {});
          reflect(terra.settings);
          emit('settings', terra.settings);
          break;
        case 'camera':
          terra.camera = message.camera;
          emit('camera', terra.camera);
          break;
        case 'tap':
          emit('tap', { x: message.x, y: message.y, lon: message.lon, lat: message.lat });
          break;
        case 'time':
          terra.time.value = message.value;
          emit('time', { value: message.value, playing: !!message.playing, by: message.by || 'host' });
          break;
        case 'close':
          emit('close');
          break;
      }
    }
  };

  window.terra = terra;
  window.addEventListener('message', function (e) {
    if (e.data && e.data.terraHost) terra._receive(e.data.terraHost);
  });
  window.addEventListener('error', function (e) { report(e.error || e.message); });
  window.addEventListener('unhandledrejection', function (e) { report(e.reason); });
  if (document.documentElement) reflect(terra.settings);
  document.addEventListener('DOMContentLoaded', function () {
    reflect(terra.settings);
    send({ type: 'ready' });
  });
})();

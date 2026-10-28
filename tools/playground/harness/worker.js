// The pack's script context in the desktop preview: a dedicated worker standing in for the app's
// JavaScriptCore context (docs/runtime3.md › The script's world). terra.js and then the pack's scripts run in
// order in this one global (`window` and `globalThis` are it), with no DOM, no fetch, no network and no storage:
// what a browser worker has beyond JavaScriptCore is taken away before the pack's first script, so a pack that
// reaches for `document` or `fetch` fails here as it would on the phone.
// The host (harness.js) is a postMessage away: `__terraSend` posts every message to it, except the two that
// answer at once: `read` (a synchronous request for the pack's file) and `camera` (the camera the host last told).
(function (global) {
  'use strict';
  var post = global.postMessage.bind(global);
  var load = global.importScripts.bind(global);
  var listen = global.addEventListener.bind(global);
  var XHR = global.XMLHttpRequest;
  var camera = null, files = {}, base = '', texts = {};

  // What a browser worker has and JavaScriptCore does not (ECMAScript's own globals stay).
  var ABSENT = ['fetch', 'XMLHttpRequest', 'WebSocket', 'WebSocketStream', 'EventSource', 'WebTransport', 'importScripts',
    'indexedDB', 'caches', 'cookieStore', 'navigator', 'location', 'origin', 'isSecureContext', 'crossOriginIsolated',
    'Worker', 'SharedWorker', 'BroadcastChannel', 'MessageChannel', 'MessagePort', 'requestAnimationFrame',
    'cancelAnimationFrame', 'postMessage', 'close', 'onmessage', 'onmessageerror', 'onerror', 'onunhandledrejection',
    'onrejectionhandled', 'addEventListener', 'removeEventListener', 'dispatchEvent', 'OffscreenCanvas',
    'createImageBitmap', 'ImageData', 'ImageBitmap', 'atob', 'btoa', 'structuredClone', 'TextEncoder', 'TextDecoder',
    'URL', 'URLSearchParams', 'Blob', 'File', 'FileReader', 'FileReaderSync', 'FormData', 'Request', 'Response',
    'Headers', 'AbortController', 'AbortSignal', 'crypto', 'performance', 'queueMicrotask', 'reportError', 'self',
    'Event', 'EventTarget', 'CustomEvent', 'Notification', 'scheduler', 'trustedTypes'];

  function strip() {
    ABSENT.forEach(function (name) {
      try { Object.defineProperty(global, name, { value: undefined, writable: true, configurable: true, enumerable: false }); } catch (e) { /* kept */ }
    });
  }

  // "main.js:12:5: " for a place in one of the pack's files
  function where(file, line, col) {
    var s = String(file || ''), i = base ? s.indexOf(base) : -1;
    var name = i >= 0 ? s.slice(i + base.length) : '';
    if (!name) return '';
    try { name = decodeURIComponent(name); } catch (e) { /* as it is */ }
    return name + (line ? ':' + line + (col ? ':' + col : '') : '') + ': ';
  }
  function describe(err) {
    if (!err || typeof err !== 'object') return String(err);
    var text = (err.name || 'Error') + ': ' + err.message, stack = String(err.stack || '');
    var at = base ? stack.indexOf(base) : -1;
    if (at >= 0) {
      var m = /^([^\s):]+):(\d+):(\d+)/.exec(stack.slice(at + base.length));
      if (m) text = where(base + m[1], m[2], m[3]) + text;
    }
    return text;
  }
  function error(message) {
    post({ kind: 'send', message: { type: 'log', level: 'error', message: String(message) } });
  }
  // an error nobody caught (a timer, a promise): logged, and the pack keeps running
  listen('error', function (e) {
    e.preventDefault();
    error(e.error ? describe(e.error) : where(e.filename, e.lineno, e.colno) + String(e.message || 'error').replace(/^Uncaught /, ''));
  });
  listen('unhandledrejection', function (e) {
    e.preventDefault();
    error('unhandled promise rejection: ' + describe(e.reason));
  });

  function clean(path) {
    var p = String(path).replace(/^\.\//, '');
    if (!p || p.charAt(0) === '/' || p.indexOf('\\') >= 0 || /^[a-z][a-z0-9+.-]*:/i.test(p)) return null;
    var parts = p.split('/');
    if (parts.some(function (x) { return x === '' || x === '.' || x === '..'; })) return null;
    return parts.join('/');
  }
  function read(path) {
    var p = clean(path);
    if (!p || !files[p]) return null;
    if (Object.prototype.hasOwnProperty.call(texts, p)) return texts[p];
    var x = new XHR(), text = null;
    try {
      x.open('GET', base + p.split('/').map(encodeURIComponent).join('/'), false);
      x.overrideMimeType('text/plain; charset=utf-8');
      x.send();
      if (x.status === 200) text = x.responseText;
    } catch (e) { text = null; }
    texts[p] = text;
    return text;
  }

  function send(message) {
    if (!message || typeof message !== 'object') return undefined;
    if (message.type === 'camera') return camera ? JSON.parse(JSON.stringify(camera)) : null;
    if (message.type === 'read') return read(message.path);
    try {
      post({ kind: 'send', message: message });
    } catch (e) {
      error('terra.' + (message.type === 'image' ? 'images' : message.type === 'sprite' ? 'sprites' : message.type)
        + ': what was passed cannot reach the host (' + e.message + ')');
    }
    return undefined;
  }

  function boot(m) {
    m.files.forEach(function (f) { files[f] = true; });
    base = m.base;
    camera = m.boot.camera || null;
    global.__terraBoot = m.boot;
    global.__terraSend = send;
    if (m.timers === 'host') {
      // as in JavaScriptCore: no timers of its own, so terra.js keeps them through the host
      ['setTimeout', 'setInterval', 'clearTimeout', 'clearInterval'].forEach(function (n) { global[n] = undefined; });
    }
    // (before terra.js, so it finds the world it finds in JavaScriptCore and fills in what it fills in there)
    strip();
    try {
      load(m.runtime);
    } catch (e) {
      error('terra.js did not load: ' + describe(e));
      post({ kind: 'loaded', scripts: m.scripts.length, ok: 0 });
      return;
    }
    var ok = 0;
    m.scripts.forEach(function (url) {
      try { load(url); ok++; } catch (e) { error(describe(e)); }
    });
    post({ kind: 'loaded', scripts: m.scripts.length, ok: ok });
  }

  listen('message', function (e) {
    var m = e.data || {};
    if (m.kind === 'boot') return boot(m);
    if (m.camera) camera = m.camera;
    if (m.kind === 'receive' && global.terra && typeof global.terra._receive === 'function') {
      try { global.terra._receive(m.message); } catch (err) { error(describe(err)); }
    }
  });
})(globalThis);

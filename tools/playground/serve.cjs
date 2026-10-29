'use strict';
// The playground's desktop preview: every runtime-3 and runtime-4 pack (docs/runtime3.md) run with the same
// terra.js the app runs, over a stand-in globe, so a pack can be seen without a Mac.
//   node tools/playground/serve.cjs [--port 8790] [--root <dir>]... [--packs build/packs]
//   /                              every pack in Prototypes/, then in each --root (a folder of pack folders)
//   /preview/<id>                  the preview: ?device=iphone|ipad, &start=1 (past the doorway), &bare=1 (the screen
//                                  alone, for screenshots), &textScale=<1-3.2>, &reduceMotion=1, &idleAfter=<seconds>,
//                                  &timers=host (terra.js's own timers, as in JavaScriptCore), &with=<id>,<id> (more
//                                  packs on the same globe, their doorways alive: docs/runtime3.md › Many at once)
//   /atlas                         the living globe: every current pack's doorway on the one globe, each alive; touch
//                                  one to start its pack beside the others (the same query as /preview)
//   /Prototypes/_runtime/terra.js  the runtime (also at /_runtime/terra.js)
//   /pack/<id>/<path>              a pack's files
//   /_harness/...                  the preview's own files (harness/), the stand-in globe's land and countries, the
//                                  globe's type spec (type.json) and fonts (fonts/<file>, from Globe/fonts/)
//   /packs/...                     with --packs: a packed folder (catalog.json, packs/, open/), for the simulator
// Binds 127.0.0.1 only and makes no outgoing request.
const http = require('http'), fs = require('fs'), path = require('path');

const ROOT = path.resolve(__dirname, '../..');
const HARNESS = path.join(__dirname, 'harness');
const TYPES = {
  '.html': 'text/html; charset=utf-8', '.htm': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.json': 'application/json',
  '.geojson': 'application/geo+json', '.txt': 'text/plain; charset=utf-8', '.md': 'text/markdown; charset=utf-8',
  '.csv': 'text/csv; charset=utf-8', '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg', '.gif': 'image/gif', '.webp': 'image/webp', '.avif': 'image/avif', '.woff': 'font/woff',
  '.woff2': 'font/woff2', '.otf': 'font/otf', '.ttf': 'font/ttf', '.mp3': 'audio/mpeg', '.m4a': 'audio/mp4',
  '.wav': 'audio/wav', '.ogg': 'audio/ogg', '.mp4': 'video/mp4', '.webm': 'video/webm', '.wasm': 'application/wasm',
};
// The preview page loads only from this server; a pack's own files are never pages.
const PAGE_CSP = "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; "
  + "img-src 'self' data: blob:; font-src 'self'; connect-src 'self'; worker-src 'self'; base-uri 'self'; form-action 'none'";
const FILE_CSP = "default-src 'none'; sandbox";
const ID_RE = /^[a-z0-9](?:[a-z0-9-]{0,38}[a-z0-9])$/;
const DEVICES = ['iphone', 'ipad'];
const MAX_FILES = 4000;

const typeOf = (file) => TYPES[path.extname(file).toLowerCase()] || 'application/octet-stream';
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const jsonForScript = (v) => JSON.stringify(v).replace(/</g, '\\u003c').replace(/[\u2028\u2029]/g, (c) => '\\u' + c.charCodeAt(0).toString(16));

// A URL path below `root`, or null: no `..`, no hidden segment, no backslash, no NUL, no symlink.
function resolveIn(root, rel) {
  let decoded;
  try { decoded = decodeURIComponent(rel); } catch (e) { return null; }
  if (decoded.includes('\0') || decoded.includes('\\')) return null;
  const parts = decoded.split('/').filter((p) => p !== '');
  if (parts.some((p) => p === '..' || p.startsWith('.'))) return null;
  const full = path.join(root, ...parts);
  if (full !== root && !full.startsWith(root + path.sep)) return null;
  try {
    let at = root;
    for (const p of parts) { at = path.join(at, p); if (fs.lstatSync(at).isSymbolicLink()) return null; }
    return fs.statSync(full).isFile() ? full : null;
  } catch (e) { return null; }
}

// Every pack under the roots, the first root's first: [{ id, dir, root, manifest | error }]
function listPacks(roots) {
  const seen = new Map();
  for (const root of roots) {
    let names = [];
    try { names = fs.readdirSync(root, { withFileTypes: true }).filter((d) => d.isDirectory() && !/^[_.]/.test(d.name)).map((d) => d.name).sort(); } catch (e) { continue; }
    for (const id of names) {
      if (seen.has(id)) continue;
      const dir = path.join(root, id), entry = { id, dir, root };
      if (!fs.existsSync(path.join(dir, 'prototype.json'))) continue;
      if (!ID_RE.test(id)) entry.error = 'not a valid prototype id';
      else {
        try { entry.manifest = JSON.parse(fs.readFileSync(path.join(dir, 'prototype.json'), 'utf8')); }
        catch (e) { entry.error = 'prototype.json cannot be read: ' + e.message; }
      }
      seen.set(id, entry);
    }
  }
  return [...seen.values()];
}

// A pack's files, as relative paths with '/' (no hidden files, no symlinks)
function packFiles(dir) {
  const out = [];
  (function walk(at, rel, depth) {
    let entries = [];
    try { entries = fs.readdirSync(at, { withFileTypes: true }); } catch (e) { return; }
    for (const d of entries.sort((a, b) => a.name.localeCompare(b.name))) {
      if (d.name.startsWith('.') || d.isSymbolicLink() || out.length >= MAX_FILES) continue;
      const r = rel ? rel + '/' + d.name : d.name;
      if (d.isDirectory() && depth < 8) walk(path.join(at, d.name), r, depth + 1);
      else if (d.isFile()) out.push(r);
    }
  })(dir, '', 0);
  return out;
}

function send(res, status, body, headers) {
  res.writeHead(status, Object.assign({ 'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff' }, headers));
  res.end(body);
}
const sendText = (res, status, text) => send(res, status, text, { 'Content-Type': 'text/plain; charset=utf-8' });

function indexPage(packs, extraRoots, packsDir) {
  const rows = packs.map((p) => {
    const m = p.manifest;
    if (p.error) return `<li><strong>${esc(p.id)}</strong> <span class="bad">${esc(p.error)}</span></li>`;
    const where = p.root === path.join(ROOT, 'Prototypes') ? '' : ` &middot; ${esc(path.relative(ROOT, p.dir) || p.dir)}`;
    const meta = `${esc(p.id)} &middot; runtime ${esc(m.runtime)} &middot; ${esc(m.shelf || 'current')}${where}`;
    const links = m.runtime === 3 || m.runtime === 4
      ? `<a href="/preview/${esc(p.id)}?device=iphone">iPhone</a> <a href="/preview/${esc(p.id)}?device=ipad">iPad</a>`
      : `<span class="note">A runtime-${esc(m.runtime)} web page: this preview runs runtime-3 and runtime-4 packs (scripts drawn by the globe).</span>`;
    return `<li><strong>${esc(m.title || p.id)}</strong> <span class="meta">${meta}</span><p>${esc(m.summary || '')}</p>${links}</li>`;
  });
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Playground preview</title>
<style>
  :root { color-scheme: light; }
  body { margin: 0; background: #F6F2E7; color: #2A2113; font: 17px/1.5 ui-serif, "New York", Georgia, serif; }
  main { max-width: 42rem; margin: 0 auto; padding: 32px 16px; }
  h1 { font-size: 28px; margin: 0 0 4px; }
  ul { list-style: none; padding: 0; } li { padding: 14px 0; border-top: 1px solid #D9D2C0; }
  p { margin: 4px 0 8px; } a { color: #0A5A80; margin-right: 14px; }
  .meta, .note { color: #5A4A33; font-size: 15px; } .bad { color: #8C1D18; }
</style></head><body><main>
<h1>Playground preview</h1>
<p class="meta">Each pack over a stand-in globe, run with the app's terra.js. Edit a file and reload.${extraRoots.length ? ' Also from: ' + extraRoots.map((r) => esc(path.relative(ROOT, r) || r)).join(', ') + '.' : ''}${packsDir ? ' Packed output is served at <a href="/packs/catalog.json">/packs/catalog.json</a>.' : ''}</p>
<p><strong>The living globe</strong> <span class="meta">every current pack's doorway on the one globe, each alive; touch one to start it beside the others</span><br>
<a href="/atlas?device=iphone">iPhone</a> <a href="/atlas?device=ipad">iPad</a></p>
<ul>${rows.join('') || '<li>No packs yet.</li>'}</ul>
</main></body></html>`;
}

function start(options) {
  const opts = Object.assign({ port: 8790, packs: null, prototypes: path.join(ROOT, 'Prototypes'), roots: [] }, options);
  const protos = path.resolve(opts.prototypes);
  const extraRoots = (opts.roots || []).map((r) => path.resolve(r));
  const roots = [protos, ...extraRoots];
  const packsDir = opts.packs ? path.resolve(opts.packs) : null;
  // the runtime comes with the prototypes it serves, or (a folder of packs has none) from the repository's
  const own = path.join(protos, '_runtime', 'terra.js');
  const runtime = fs.existsSync(own) ? own : path.join(ROOT, 'Prototypes', '_runtime', 'terra.js');
  const globe = path.join(ROOT, 'Globe');
  const DATA = {
    'land-50m.json': path.join(ROOT, 'tools/plates/data/land-50m.json'),
    'countries-50m.json': path.join(ROOT, 'tools/plates/data/countries-50m.json'),
    'type.json': path.join(globe, 'type.json'),
  };
  const findPack = (id) => (ID_RE.test(id) ? listPacks(roots).find((p) => p.id === id) : null);

  const server = http.createServer((req, res) => {
    if (req.method !== 'GET' && req.method !== 'HEAD') return sendText(res, 405, 'GET only');
    let url;
    try { url = new URL(req.url, 'http://127.0.0.1'); } catch (e) { return sendText(res, 400, 'Bad request'); }
    const p = url.pathname;
    let m;
    const file = (full, type, extra) => fs.readFile(full, (err, data) => err ? sendText(res, 404, 'Not found')
      : send(res, 200, req.method === 'HEAD' ? '' : data, Object.assign({ 'Content-Type': type || typeOf(full) }, extra)));

    if (p === '/') return send(res, 200, indexPage(listPacks(roots), extraRoots, packsDir), { 'Content-Type': 'text/html; charset=utf-8' });
    if (p === '/Prototypes/_runtime/terra.js' || p === '/_runtime/terra.js') return file(runtime, 'text/javascript; charset=utf-8');
    m = /^\/_harness\/([a-z0-9-]+\.(?:js|css|json))$/.exec(p);
    if (m) {
      if (DATA[m[1]]) return file(DATA[m[1]], 'application/json');
      const own = resolveIn(HARNESS, m[1]);
      return own ? file(own) : sendText(res, 404, 'Not found');
    }
    m = /^\/_harness\/fonts\/([^/]+\.(?:otf|ttf))$/.exec(p);
    if (m) {
      const font = resolveIn(path.join(globe, 'fonts'), m[1]);
      return font ? file(font) : sendText(res, 404, 'Not found');
    }

    // the preview of one pack (with others beside it), or the living globe: every current pack at once
    m = /^\/preview\/([^/]+)\/?$/.exec(p) || (/^\/atlas\/?$/.test(p) ? [p, null] : null);
    if (m) {
      const asked = url.searchParams.get('device');
      const device = asked === 'phone' ? 'iphone' : DEVICES.includes(asked) ? asked : 'iphone';
      const query = Object.fromEntries(url.searchParams);
      const entry = (pack) => ({ id: pack.id, manifest: pack.manifest, files: packFiles(pack.dir), where: path.relative(ROOT, pack.dir) || pack.dir });
      const runs = (pack) => pack && !pack.error && (pack.manifest.runtime === 3 || pack.manifest.runtime === 4);
      let packs, primary, title;
      if (m[1]) {
        const pack = findPack(m[1]);
        if (!pack) return sendText(res, 404, `${m[1]}: no such pack (no ${m[1]}/prototype.json in ${roots.map((r) => path.relative(ROOT, r) || r).join(', ')})`);
        if (pack.error) return sendText(res, 404, `${m[1]}: ${pack.error}`);
        packs = [entry(pack)];
        primary = pack.id;
        title = pack.manifest.title || m[1];
        // ?with=<id>,<id>: more packs on the same globe (unknown or unrunnable ones are left out, and said so)
        for (const id of String(url.searchParams.get('with') || '').split(',').map((x) => x.trim()).filter(Boolean)) {
          const other = findPack(id);
          if (id !== pack.id && runs(other) && !packs.some((k) => k.id === id)) packs.push(entry(other));
        }
      } else {
        packs = listPacks(roots).filter((k) => runs(k) && (k.manifest.shelf || 'current') !== 'parked').map(entry);
        primary = null;
        title = 'The living globe';
      }
      const first = packs[0] || {};
      const page = fs.readFileSync(path.join(HARNESS, 'harness.html'), 'utf8')
        .replace('__TITLE__', () => esc(title))
        .replace('__PROTO__', () => jsonForScript({ id: primary || first.id || null, manifest: primary ? first.manifest : null, files: primary ? first.files : [],
          where: primary ? first.where : null, packs, primary, device, query, runtime: '/Prototypes/_runtime/terra.js' }));
      return send(res, 200, page, { 'Content-Type': 'text/html; charset=utf-8', 'Content-Security-Policy': PAGE_CSP });
    }

    m = /^\/pack\/([^/]+)\/(.+)$/.exec(p);
    if (m) {
      const pack = findPack(m[1]);
      const full = pack && resolveIn(pack.dir, m[2]);
      if (!full) return sendText(res, 404, 'Not found');
      return file(full, null, { 'Content-Security-Policy': FILE_CSP });
    }

    if (packsDir && p.startsWith('/packs/')) {
      const full = resolveIn(packsDir, p.slice('/packs/'.length));
      if (!full) return sendText(res, 404, 'Not found');
      return file(full, null, { 'Cache-Control': 'no-cache' });
    }
    return sendText(res, 404, 'Not found');
  });

  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(opts.port, '127.0.0.1', () => {
      server.off('error', reject);
      const port = server.address().port;
      resolve({ server, port, url: 'http://127.0.0.1:' + port, roots, close: () => new Promise((r) => server.close(r)) });
    });
  });
}

module.exports = { start, resolveIn, listPacks, packFiles };

if (require.main === module) {
  const args = process.argv.slice(2), opt = (name) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : undefined; };
  const port = opt('--port') === undefined ? 8790 : Number(opt('--port'));
  if (!Number.isInteger(port) || port < 0 || port > 65535) { console.error('--port takes a number from 0 to 65535'); process.exit(2); }
  if (args.includes('--packs') && !opt('--packs')) { console.error('--packs takes a folder'); process.exit(2); }
  const roots = [];
  args.forEach((a, i) => { if (a === '--root') roots.push(args[i + 1]); });
  if (roots.some((r) => !r || !fs.existsSync(r))) { console.error('--root takes an existing folder of pack folders'); process.exit(2); }
  start({ port, packs: opt('--packs'), roots }).then((s) => {
    console.log('Playground preview: ' + s.url + '/' + (opt('--packs') ? '   packs: ' + s.url + '/packs/catalog.json' : ''));
  }).catch((e) => {
    console.error(e.code === 'EADDRINUSE' ? `Port ${port} is in use: pass --port <n>` : e.message);
    process.exit(1);
  });
}

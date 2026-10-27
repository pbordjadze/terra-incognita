'use strict';
// The playground's local preview: every prototype in Prototypes/ over a stand-in globe, served with
// the host's rules (a pack may load only its own files; terra.js is injected, never included).
//   node tools/playground/serve.cjs [--port 8790] [--packs build/packs]
//   /                       the prototypes
//   /preview/<id>           the harness, ?device=phone|ipad
//   /pack/<id>/<path>       a pack's files, html with terra.js injected (?mode=overlay, ?marker=x)
//   /packs/...              with --packs: a packed folder (catalog.json, packs/, open/), for the simulator
// Binds 127.0.0.1 only and makes no outgoing request.
const http = require('http'), fs = require('fs'), path = require('path');

const ROOT = path.resolve(__dirname, '../..');
const HARNESS = path.join(__dirname, 'harness');
const TYPES = {
  '.html': 'text/html; charset=utf-8', '.htm': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.json': 'application/json',
  '.txt': 'text/plain; charset=utf-8', '.md': 'text/markdown; charset=utf-8', '.svg': 'image/svg+xml',
  '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.gif': 'image/gif', '.webp': 'image/webp',
  '.avif': 'image/avif', '.woff': 'font/woff', '.woff2': 'font/woff2', '.otf': 'font/otf', '.ttf': 'font/ttf',
  '.mp3': 'audio/mpeg', '.m4a': 'audio/mp4', '.wav': 'audio/wav', '.ogg': 'audio/ogg', '.mp4': 'video/mp4',
  '.webm': 'video/webm', '.wasm': 'application/wasm',
};
// What the app enforces by content rule: nothing is loaded from outside the pack. (eval stays allowed:
// the app's web view does not forbid it, so the preview should not either.)
const PACK_CSP = "default-src 'self' 'unsafe-inline' 'unsafe-eval' data: blob:; base-uri 'self'; form-action 'self'";
const ID_RE = /^[a-z0-9](?:[a-z0-9-]{0,38}[a-z0-9])$/;
const DEVICES = ['phone', 'ipad'];

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

function send(res, status, body, headers) {
  res.writeHead(status, Object.assign({ 'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff' }, headers));
  res.end(body);
}
const sendText = (res, status, text) => send(res, status, text, { 'Content-Type': 'text/plain; charset=utf-8' });

function readManifest(protos, id) {
  if (!ID_RE.test(id)) return { error: 'not a valid prototype id' };
  try {
    return { manifest: JSON.parse(fs.readFileSync(path.join(protos, id, 'prototype.json'), 'utf8')) };
  } catch (e) {
    return { error: e.code === 'ENOENT' ? 'no such prototype (no prototype.json)' : 'prototype.json cannot be read: ' + e.message };
  }
}

function listPrototypes(protos) {
  let names = [];
  try { names = fs.readdirSync(protos, { withFileTypes: true }).filter((d) => d.isDirectory() && !/^[_.]/.test(d.name)).map((d) => d.name).sort(); } catch (e) { /* none */ }
  return names;
}

function indexPage(protos, packs) {
  const rows = listPrototypes(protos).map((id) => {
    const { manifest: m, error } = readManifest(protos, id);
    if (error) return `<li><strong>${esc(id)}</strong> <span class="bad">${esc(error)}</span></li>`;
    return `<li><strong>${esc(m.title || id)}</strong> <span class="meta">${esc(id)} &middot; ${esc(m.presentation)} &middot; ${esc(m.shelf || 'current')}</span>
      <p>${esc(m.summary || '')}</p>
      <a href="/preview/${esc(id)}?device=phone">Phone</a> <a href="/preview/${esc(id)}?device=ipad">iPad</a></li>`;
  });
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Playground preview</title>
<style>
  :root { color-scheme: light; }
  body { margin: 0; background: #F6F2E7; color: #2A2113; font: 17px/1.5 ui-serif, "New York", Georgia, serif; }
  main { max-width: 40rem; margin: 0 auto; padding: 32px 20px; }
  h1 { font-size: 28px; margin: 0 0 4px; }
  ul { list-style: none; padding: 0; } li { padding: 14px 0; border-top: 1px solid #D9D2C0; }
  p { margin: 4px 0 8px; } a { color: #0A5A80; margin-right: 14px; }
  .meta { color: #5A4A33; font-size: 15px; } .bad { color: #8C1D18; }
</style></head><body><main>
<h1>Playground preview</h1>
<p class="meta">Prototypes/ over a stand-in globe. Edit a file and reload.${packs ? ' Packed output is served at <a href="/packs/catalog.json">/packs/catalog.json</a>.' : ''}</p>
<ul>${rows.join('') || '<li>No prototypes yet: <code>python3 tools/playground/playground.py new &lt;id&gt; --globe</code></li>'}</ul>
</main></body></html>`;
}

// terra.js is injected by the host, before the page's own scripts. In the preview that is a pair of
// script tags at the top of <head> (after the doctype if there is no <head>, to keep standards mode).
function inject(html, id, url) {
  const q = url.searchParams, params = {};
  for (const [k, v] of q) if (!['mode', 'textScale', 'reduceMotion', 'active'].includes(k)) params[k] = v;
  const boot = {
    pack: { id, revision: 0, origin: 'preview' },
    mode: q.get('mode') === 'overlay' ? 'overlay' : 'screen',
    params,
    settings: {
      reduceMotion: q.get('reduceMotion') === '1' || q.get('reduceMotion') === 'true',
      textScale: Math.min(3.2, Math.max(1, parseFloat(q.get('textScale')) || 1)),
      active: q.get('active') !== '0',
    },
  };
  const tags = `<script>window.__terraBoot=${jsonForScript(boot)}</script><script src="/_runtime/terra.js"></script>`;
  const head = /<head(?:\s[^>]*)?>/i.exec(html);
  if (head) return html.slice(0, head.index + head[0].length) + tags + html.slice(head.index + head[0].length);
  const doctype = /^\s*<!doctype[^>]*>/i.exec(html);
  return doctype ? html.slice(0, doctype[0].length) + tags + html.slice(doctype[0].length) : tags + html;
}

function start(options) {
  const opts = Object.assign({ port: 8790, packs: null, prototypes: path.join(ROOT, 'Prototypes') }, options);
  const protos = path.resolve(opts.prototypes);
  const packsDir = opts.packs ? path.resolve(opts.packs) : null;
  const runtime = path.join(protos, '_runtime', 'terra.js');
  const land = path.join(ROOT, 'tools/plates/data/land-50m.json');

  const server = http.createServer((req, res) => {
    if (req.method !== 'GET' && req.method !== 'HEAD') return sendText(res, 405, 'GET only');
    let url;
    try { url = new URL(req.url, 'http://127.0.0.1'); } catch (e) { return sendText(res, 400, 'Bad request'); }
    const p = url.pathname;
    const file = (full, type, extra) => fs.readFile(full, (err, data) => err ? sendText(res, 404, 'Not found')
      : send(res, 200, req.method === 'HEAD' ? '' : data, Object.assign({ 'Content-Type': type || typeOf(full) }, extra)));

    if (p === '/') return send(res, 200, indexPage(protos, packsDir), { 'Content-Type': 'text/html; charset=utf-8' });
    if (p === '/_runtime/terra.js') return file(runtime, 'text/javascript; charset=utf-8');
    if (p === '/_harness/harness.js') return file(path.join(HARNESS, 'harness.js'), 'text/javascript; charset=utf-8');
    if (p === '/_harness/land-50m.json') return file(land, 'application/json');

    let m = /^\/preview\/([^/]+)\/?$/.exec(p);
    if (m) {
      const { manifest, error } = readManifest(protos, m[1]);
      if (error) return sendText(res, 404, `${m[1]}: ${error}`);
      const device = DEVICES.includes(url.searchParams.get('device')) ? url.searchParams.get('device') : 'phone';
      const query = Object.fromEntries(url.searchParams);
      const page = fs.readFileSync(path.join(HARNESS, 'harness.html'), 'utf8')
        .replace('__TITLE__', () => esc(manifest.title || m[1]))
        .replace('__PROTO__', () => jsonForScript({ id: m[1], manifest, device, query }));
      return send(res, 200, page, { 'Content-Type': 'text/html; charset=utf-8' });
    }

    m = /^\/pack\/([^/]+)\/(.+)$/.exec(p);
    if (m) {
      if (!ID_RE.test(m[1])) return sendText(res, 404, 'Not found');
      const full = resolveIn(path.join(protos, m[1]), m[2]);
      if (!full) return sendText(res, 404, 'Not found');
      if (/\.html?$/i.test(full)) {
        return fs.readFile(full, 'utf8', (err, html) => err ? sendText(res, 404, 'Not found')
          : send(res, 200, req.method === 'HEAD' ? '' : inject(html, m[1], url), { 'Content-Type': typeOf(full), 'Content-Security-Policy': PACK_CSP }));
      }
      return file(full, null, { 'Content-Security-Policy': PACK_CSP });
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
      resolve({ server, port, url: 'http://127.0.0.1:' + port, close: () => new Promise((r) => server.close(r)) });
    });
  });
}

module.exports = { start, resolveIn, inject };

if (require.main === module) {
  const args = process.argv.slice(2), opt = (name) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : undefined; };
  const port = opt('--port') === undefined ? 8790 : Number(opt('--port'));
  if (!Number.isInteger(port) || port < 0 || port > 65535) { console.error('--port takes a number from 0 to 65535'); process.exit(2); }
  if (args.includes('--packs') && !opt('--packs')) { console.error('--packs takes a folder'); process.exit(2); }
  start({ port, packs: opt('--packs') }).then((s) => {
    console.log('Playground: ' + s.url + '/' + (opt('--packs') ? '   packs: ' + s.url + '/packs/catalog.json' : ''));
  }).catch((e) => {
    console.error(e.code === 'EADDRINUSE' ? `Port ${port} is in use: pass --port <n>` : e.message);
    process.exit(1);
  });
}

// Loads the page in headless Chromium (software WebGL), fails on any page error or console
// error, and screenshots the tour at the times given.
//   node ci/web_check.cjs <Web dir> <out dir> <width>x<height> <t1> <t2> ...
const http = require('http'), fs = require('fs'), path = require('path');
const { chromium } = require('playwright');
const [root, out, size, ...times] = process.argv.slice(2);
const [W, H] = size.split('x').map(Number);
const types = { '.html': 'text/html', '.js': 'text/javascript', '.json': 'application/json', '.png': 'image/png', '.css': 'text/css', '.woff2': 'font/woff2', '.glsl': 'text/plain' };
const server = http.createServer((req, res) => {
  const f = path.join(root, decodeURIComponent(req.url.split('?')[0]).replace(/\/$/, '/index.html'));
  fs.readFile(f, (e, d) => { if (e) { res.writeHead(404); return res.end(); } res.writeHead(200, { 'Content-Type': types[path.extname(f)] || 'application/octet-stream' }); res.end(d); });
}).listen(0, async () => {
  const port = server.address().port;
  const b = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  const p = await (await b.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 2 })).newPage();
  const errs = [];
  p.on('pageerror', e => errs.push('page error: ' + e));
  p.on('console', m => { if (m.type() === 'error') errs.push('console error: ' + m.text()); });
  fs.mkdirSync(out, { recursive: true });
  await p.goto(`http://127.0.0.1:${port}/index.html`);
  try { await p.waitForFunction(() => window.App && App.ready, null, { timeout: 90000 }); }
  catch (e) { errs.push('the page never became ready'); }
  for (const t of times) {
    await p.evaluate(T => { if (App.tour) { App.tour.T = +T; App.tour.playing = false; } }, t);
    await p.waitForTimeout(1500);
    await p.screenshot({ path: path.join(out, `web-${W}x${H}-t${String(t).padStart(3, '0')}.png`) });
  }
  await b.close(); server.close();
  if (errs.length) { console.error(errs.join('\n')); process.exit(1); }
  console.log('ok:', times.length, 'screenshots');
});

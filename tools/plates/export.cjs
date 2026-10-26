#!/usr/bin/env node
// Renders a look's plates for the app.
//   node tools/plates/export.cjs <look> "<Look name>"
// opens tools/plates/<look>/index.html (the contract is in tools/plates/kit.js), renders every layer
// of every level and writes Atlas/<look>/<level id>/<layer>.{jpg,png} and Atlas/<look>/plates.json,
// then lists the look in Atlas/looks.json. Needs Playwright (in CI's image, or `npm i playwright`).
'use strict';
const fs = require('fs'), path = require('path'), http = require('http');
const { chromium } = require('playwright');

const [look, name] = process.argv.slice(2);
if (!look || !name) { console.error('usage: export.cjs <look> "<Look name>"'); process.exit(2); }
const root = path.join(__dirname, '..', '..');
const out = path.join(root, 'Atlas', look);
const types = { '.html': 'text/html', '.js': 'text/javascript', '.json': 'application/json', '.geojson': 'application/json',
  '.png': 'image/png', '.jpg': 'image/jpeg', '.css': 'text/css', '.woff2': 'font/woff2', '.svg': 'image/svg+xml' };

const server = http.createServer((req, res) => {
  const f = path.join(__dirname, decodeURIComponent(req.url.split('?')[0]));
  if (!f.startsWith(__dirname)) { res.writeHead(403); return res.end(); }
  fs.readFile(f, (e, d) => {
    if (e) { res.writeHead(404); return res.end(); }
    res.writeHead(200, { 'Content-Type': types[path.extname(f)] || 'application/octet-stream' }); res.end(d);
  });
}).listen(0, '127.0.0.1', async () => {
  const base = `http://127.0.0.1:${server.address().port}/${look}/index.html`;
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  await page.goto(`${base}?level=1&layer=none`);
  await page.waitForFunction(() => window.Plates && window.Kit, null, { timeout: 60000 });
  const { W, H, levels } = await page.evaluate(() => ({ W: Kit.W, H: Kit.H, levels: Plates.levels }));
  await page.setViewportSize({ width: W, height: H });
  fs.rmSync(out, { recursive: true, force: true });
  // Each level's plate, and between two levels whose zoom differs more than MAX_STEP, plates that
  // pass through: the wider level drawn again magnified about the closer one's centre (vector work
  // stays sharp magnified), so no hand-over shows a picture blown up past MAX_STEP. The film dives
  // straight through them, with no drift and no caption.
  const MAX_STEP = 4;
  const infos = [];
  for (let level = 1; level <= levels; level++) infos.push(await page.evaluate(l => Plates.info(l), level));
  const plan = [];
  infos.forEach((info, n) => {
    plan.push({ level: n + 1, info, id: info.id, magnify: 1 });
    if (!info.child) return;
    const [x, y, w] = info.child, cx = x + w / 2, cy = y + info.child[3] / 2, ratio = W / w;
    const steps = Math.ceil(Math.log(ratio) / Math.log(MAX_STEP));
    for (let i = 1; i < steps; i++) plan.push({ level: n + 1, info, id: `${info.id}-${i}`, magnify: Math.pow(ratio, i / steps), cx, cy, pass: true });
  });
  // where each plate sits in the one before: a level's child is the next level, or a magnified pass
  // plate centred on it; a pass plate's child is the next pass plate or the next level, magnified alike
  const childOf = (a, b) => {
    if (!b) return null;
    if (b.magnify === 1) {
      const [x, y, w, h] = a.info.child;
      if (a.magnify === 1) return a.info.child;
      return [W / 2 + (x - a.cx) * a.magnify, H / 2 + (y - a.cy) * a.magnify, w * a.magnify, h * a.magnify];
    }
    const k = (a.magnify === 1 ? 1 : a.magnify) / b.magnify, w = W * k, h = H * k;
    const [cx, cy] = a.magnify === 1 ? [b.cx, b.cy] : [W / 2, H / 2];
    return [cx - w / 2, cy - h / 2, w, h];
  };
  const plates = [];
  for (const [i, item] of plan.entries()) {
    const { level, info, id } = item;
    fs.mkdirSync(path.join(out, id), { recursive: true });
    const layers = [];
    for (const layer of info.layers) {
      await page.goto(`${base}?level=${level}&layer=${layer.name}`);
      await page.waitForFunction(() => window.platesReady !== undefined, null, { timeout: 120000 });
      const ready = await page.evaluate(() => window.platesReady);
      if (ready !== true) throw new Error(`${id} ${layer.name}: ${ready}\n${errors.join('\n')}`);
      if (item.magnify !== 1) {
        await page.evaluate(({ s, cx, cy, W, H }) => {
          const r = document.documentElement;
          r.style.transformOrigin = '0 0';
          r.style.transform = `translate(${W / 2 - cx * s}px, ${H / 2 - cy * s}px) scale(${s})`;
        }, { s: item.magnify, cx: item.cx, cy: item.cy, W, H });
        await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
      }
      const file = `${id}/${layer.name}.${layer.opaque ? 'jpg' : 'png'}`;
      await page.screenshot({ path: path.join(out, file), type: layer.opaque ? 'jpeg' : 'png', quality: layer.opaque ? 88 : undefined,
        omitBackground: !layer.opaque, clip: { x: 0, y: 0, width: W, height: H } });
      layers.push({ file, depth: layer.depth || 0 });
      console.log(`${look} ${id} ${layer.name}`);
    }
    const plate = { id, size: [W, H], layers, child: childOf(item, plan[i + 1]), caption: info.caption };
    if (plate.child) plate.child = plate.child.map(v => Math.round(v * 10) / 10);
    if (item.pass) plate.pass = true;
    plates.push(plate);
  }
  fs.writeFileSync(path.join(out, 'plates.json'), JSON.stringify({ name, plates }, null, 1) + '\n');
  const listPath = path.join(root, 'Atlas', 'looks.json');
  const list = fs.existsSync(listPath) ? JSON.parse(fs.readFileSync(listPath, 'utf8')) : [];
  const entry = { id: look, name, plates: `${look}/plates.json` };
  const i = list.findIndex(l => l.id === look);
  if (i >= 0) list[i] = entry; else list.push(entry);
  fs.writeFileSync(listPath, JSON.stringify(list, null, 1) + '\n');
  await browser.close(); server.close();
  if (errors.length) { console.error('page errors:\n' + errors.join('\n')); process.exit(1); }
});

// Opens a web build of Adrift in headless Chromium (WebGPU on Mesa's software Vulkan) at a phone's
// size (headed under xvfb-run: headless Chromium shoots a WebGPU canvas blank), lets it run, touches it as a plan says, and screenshots: a look at the shell on Linux, no
// device needed (slow: a frame is about a second on the software driver).
//
//     python3 -m http.server -d <web-build-dir> 8765 &
//     xvfb-run -a node tools/adrift/web-look.cjs http://localhost:8765/ <out-dir> [plan.json]
//
// A plan is a list of steps: {"wait": s}, {"shot": "name"}, {"down": [x, y], "id": n},
// {"up": [x, y], "id": n}, {"move": [x, y], "id": n} (CSS pixels), {"key": "."}, {"keyup": "."},
// {"reload": true}. Without one: wait 8 s and shoot. The console is printed.
const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
  const [url, out, planPath] = process.argv.slice(2);
  fs.mkdirSync(out, { recursive: true });
  const plan = planPath ? JSON.parse(fs.readFileSync(planPath, 'utf8')) : [{ wait: 8 }, { shot: 'open' }];
  const browser = await chromium.launch({ headless: !process.env.DISPLAY, args: (process.env.CHROME_ARGS || '--enable-unsafe-webgpu --enable-features=Vulkan --ignore-gpu-blocklist').split(' ') });
  const ctx = await browser.newContext({ viewport: { width: 393, height: 852 }, deviceScaleFactor: 2, hasTouch: true });
  const page = await ctx.newPage();
  page.on('console', m => console.log('console:', m.text()));
  page.on('pageerror', e => console.log('pageerror:', e.message));
  await page.goto(url);
  const cdp = await ctx.newCDPSession(page);
  const touches = new Map();
  const send = async (type) => {
    await cdp.send('Input.dispatchTouchEvent', { type, touchPoints: [...touches.entries()].map(([id, [x, y]]) => ({ x, y, id })) });
  };
  for (const s of plan) {
    if (s.wait) await page.waitForTimeout(s.wait * 1000);
    if (s.shot) { await page.screenshot({ path: `${out}/${s.shot}.png` }); console.log('shot', s.shot); }
    if (s.down) { const had = touches.size; touches.set(s.id || 1, s.down); await send(had ? 'touchMove' : 'touchStart'); if (had) await send('touchStart'); }
    if (s.move) { touches.set(s.id || 1, s.move); await send('touchMove'); }
    if (s.up) { touches.delete(s.id || 1); await send(touches.size ? 'touchEnd' : 'touchEnd'); }
    if (s.key) await page.keyboard.down(s.key);
    if (s.keyup) await page.keyboard.up(s.keyup);
    if (s.reload) await page.reload();
    if (s.eval) console.log('eval:', JSON.stringify(await page.evaluate(s.eval)));
  }
  await browser.close();
})();

'use strict';
// Real browser checks for Prototypes/one-ship-two-rhythms, through the playground's injected runtime.
// node tools/playground/ship/check.cjs [--engine chromium|webkit|all]
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { chromium, webkit } = require('playwright');
const { start } = require('../serve.cjs');

const ROOT = path.resolve(__dirname, '../../..');
const OUT = path.join(ROOT, 'build/ship-report');
const sizes = [
  { name: 'phone', width: 390, height: 844 },
  { name: 'ipad-landscape', width: 1366, height: 1024 },
  { name: 'ipad-portrait', width: 1024, height: 1366 },
];
const index = process.argv.indexOf('--engine');
const engineChoice = index < 0 ? 'all' : process.argv[index + 1];
if (!['all', 'chromium', 'webkit'].includes(engineChoice)) {
  throw new Error('--engine must be chromium, webkit, or all');
}
fs.mkdirSync(OUT, { recursive: true });
const report = {
  passed: false,
  tests: [],
  failures: [],
  screenshots: [],
  requests: [],
  notes: [
    'Chromium pinch checks use CDP native touch input, not synthetic PointerEvents.',
    'WebKit pinch checks dispatch PointerEvents with a temporary pointer-capture shim; they verify handler mathematics, not physical-device multitouch delivery.',
    'Simulator/browser checks do not establish physical iPad or iPhone performance.',
  ],
};
const state = page => page.evaluate(() => window.shipAtlasState());
const settle = page => page.evaluate(() => new Promise(resolve =>
  requestAnimationFrame(() => requestAnimationFrame(resolve))));
const settings = (page, next) => page.evaluate(settings => {
  window.terra._receive({ type: 'settings', settings });
}, next);
const setDay = async (page, day) => {
  await page.locator('#year-slider').evaluate((el, value) => {
    el.value = value;
    el.dispatchEvent(new Event('input', { bubbles: true }));
  }, String(day));
  await page.waitForFunction(d => Math.abs(window.shipAtlasState().day-d)<.011, day);
  await settle(page);
};
const pixels = page => page.locator('#map').evaluate(canvas => {
  const { data } = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height);
  let hash = 2166136261 >>> 0, warm = 0, total = 0;
  // Sample actual painted pixels, including RGB channels; do not infer rendering from state.
  for (let i = 0; i < data.length; i += 16) {
    hash = Math.imul(hash ^ data[i], 16777619) >>> 0;
    hash = Math.imul(hash ^ data[i + 1], 16777619) >>> 0;
    hash = Math.imul(hash ^ data[i + 2], 16777619) >>> 0;
    if (data[i] > data[i + 1] + 8 && data[i + 1] > data[i + 2] + 8 && data[i + 3] > 200) warm++;
    total++;
  }
  return { hash, warm, total, width: canvas.width, height: canvas.height };
});
async function mouseDrag(page, locator, from, to) {
  const box = await locator.boundingBox();
  assert(box && box.width > 0 && box.height > 0, 'gesture surface has a visible box');
  const a = { x: box.x + box.width * from[0], y: box.y + box.height * from[1] };
  const b = { x: box.x + box.width * to[0], y: box.y + box.height * to[1] };
  await page.mouse.move(a.x, a.y);
  await page.mouse.down();
  await page.mouse.move(b.x, b.y, { steps: 8 });
  await page.mouse.up();
  await settle(page);
}
async function pinch(page, context, engine) {
  const box = await page.locator('#ruler').boundingBox();
  assert(box && box.width > 100 && box.height > 30, 'timeline is large enough for two fingers');
  const center = box.x + box.width / 2;
  const y = box.y + box.height * .45;
  const start = Math.min(48, box.width * .14), end = Math.min(125, box.width * .35);
  if (engine === 'chromium') {
    const cdp = await context.newCDPSession(page);
    const points = spread => [
      { x: center - spread, y, id: 11, radiusX: 5, radiusY: 5, force: 1 },
      { x: center + spread, y, id: 12, radiusX: 5, radiusY: 5, force: 1 },
    ];
    try {
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: points(start) });
      for (let i = 1; i <= 6; i++) {
        await cdp.send('Input.dispatchTouchEvent', {
          type: 'touchMove', touchPoints: points(start + (end - start) * i / 6),
        });
      }
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    } finally {
      await cdp.detach();
    }
  } else {
    await page.locator('#ruler').evaluate((canvas, values) => {
      const [center, y, start, end] = values;
      // Synthetic PointerEvents cannot acquire a real browser pointer capture.
      const old = {
        set: canvas.setPointerCapture, release: canvas.releasePointerCapture, has: canvas.hasPointerCapture,
      };
      canvas.setPointerCapture = () => {};
      canvas.releasePointerCapture = () => {};
      canvas.hasPointerCapture = () => false;
      const send = (type, id, x) => canvas.dispatchEvent(new PointerEvent(type, {
        pointerId: id, pointerType: 'touch', isPrimary: id === 11,
        clientX: x, clientY: y, bubbles: true, cancelable: true, buttons: type === 'pointerup' ? 0 : 1,
      }));
      try {
        send('pointerdown', 11, center - start);
        send('pointerdown', 12, center + start);
        for (let i = 1; i <= 6; i++) {
          const spread = start + (end - start) * i / 6;
          send('pointermove', 11, center - spread);
          send('pointermove', 12, center + spread);
        }
        send('pointerup', 11, center - end);
        send('pointerup', 12, center + end);
      } finally {
        canvas.setPointerCapture = old.set;
        canvas.releasePointerCapture = old.release;
        canvas.hasPointerCapture = old.has;
      }
    }, [center, y, start, end]);
  }
  await settle(page);
}


(async () => {
  let server;
  try {
    server=await start({port:0});
    for(const [engine,launcher] of [['chromium',chromium],['webkit',webkit]]){
      if(engineChoice!=='all'&&engineChoice!==engine)continue;
      let browser;
      try{
        browser=await launcher.launch(engine==='chromium'&&process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{});
        for(const size of sizes){
          const label=engine+'/'+size.name,result={label,passed:false,steps:[],errors:[],screenshots:[]};
          report.tests.push(result);
          const context=await browser.newContext({viewport:{width:size.width,height:size.height},deviceScaleFactor:1,hasTouch:true});
          const page=await context.newPage();
          let phase='load',observations=[],initialDay=0;
          const step=async(name,fn)=>{phase=name;await fn();result.steps.push(name);};
          const shot=async name=>{
            const file=engine+'-'+size.name+'-'+name+'.png';
            await page.screenshot({path:path.join(OUT,file),fullPage:true});
            const rel=path.relative(ROOT,path.join(OUT,file));report.screenshots.push(rel);result.screenshots.push(rel);
          };
          page.on('pageerror',e=>result.errors.push('pageerror: '+e.message));
          page.on('console',m=>{if(m.type()==='error')result.errors.push('console: '+m.text());});
          page.on('response',r=>{if(r.status()>=400)result.errors.push(r.status()+' '+r.url());});
          await context.route('**/*',async route=>{
            const url=new URL(route.request().url());report.requests.push({test:label,url:url.href});
            if(url.origin!==server.url&&!['data:','blob:','about:'].includes(url.protocol)){
              result.errors.push('external request blocked: '+url.href);await route.abort();return;
            }
            if(url.pathname==='/favicon.ico'){await route.fulfill({status:204});return;}
            await route.continue();
          });
          const unfollow=async()=>{
            if((await state(page)).follow)await page.locator('#follow-ship').click();
            assert.equal((await state(page)).follow,false);
          };
          try{
            await step('injected runtime and real map surface',async()=>{
              await page.goto(server.url+'/pack/one-ship-two-rhythms/index.html?mode=screen',{waitUntil:'networkidle'});
              await page.waitForFunction(()=>window.terra&&window.shipAtlasState);
              await settle(page);
              assert.equal(await page.evaluate(()=>terra.pack.id),'one-ship-two-rhythms');
              const s=await state(page);observations=s.observations;initialDay=s.initialDay;
              assert(observations.length>=3,'dated ship observations are available');
              assert(Number.isFinite(initialDay),'initial day is numeric');
              const p=await pixels(page);assert(p.width>100&&p.height>100,'map has real backing pixels');
              await shot('opening');
            });
            await step('selection changes time scale without rewinding the world',async()=>{
              await unfollow();
              const d=(observations[0].day+observations[1].day)/2;
              await setDay(page,d);
              const before=await state(page);
              await page.locator('#select-empire').click();
              const empire=await state(page);
              assert.equal(empire.selection,'empire');
              assert(Math.abs(empire.day-before.day)<.011,'empire selection preserves current day');
              assert.deepEqual(empire.camera,before.camera,'empire selection preserves geographic camera');
              assert(empire.span>1000,'empire scale covers years');
              await page.locator('#select-ship').click();
              const ship=await state(page);
              assert.equal(ship.selection,'ship');
              assert(Math.abs(ship.day-empire.day)<.011,'ship selection preserves current day');
              assert.deepEqual(ship.camera,empire.camera,'ship selection preserves geographic camera');
              assert(ship.span<empire.span/20,'ship scale is substantially narrower');
              assert.equal(await page.locator('#date-toggle').getAttribute('aria-expanded'),'true');
              result.scaleComparison={ship:ship.span,empire:empire.span};
            });
            await step('dated positions and reversible movement share one clock',async()=>{
              await setDay(page,observations[0].day);
              const first=await state(page);assert.equal(first.position?.kind,'recorded');
              assert(Math.abs(first.position.lon-observations[0].lon)<1e-5);
              assert(Math.abs(first.position.lat-observations[0].lat)<1e-5);
              const a=observations[0].day+1,b=observations[0].day+2;
              await setDay(page,a);const earlier=await state(page),pa=await pixels(page);
              await setDay(page,b);const later=await state(page),pb=await pixels(page);
              assert.equal(earlier.position?.kind,'interpolated');
              assert.equal(later.position?.kind,'interpolated');
              assert(Math.hypot(later.position.lon-earlier.position.lon,later.position.lat-earlier.position.lat)>1e-5,'ship moves between dates');
              assert.notEqual(pa.hash,pb.hash,'painted map changes as the ship moves');
              await setDay(page,a);const reversed=await state(page);
              assert(Math.abs(reversed.position.lon-earlier.position.lon)<1e-5,'rewinding retraces longitude');
              assert(Math.abs(reversed.position.lat-earlier.position.lat)<1e-5,'rewinding retraces latitude');
              result.motionHashes={earlier:pa.hash,later:pb.hash};
              await shot('ship-scale');
            });
            await step('clicking the visible ship selects it at the current date',async()=>{
              await setDay(page,initialDay);
              await page.locator('#clear-selection').click();
              const before=await state(page);
              assert.equal(before.selection,null);
              const hit=await page.locator('#map').evaluate((canvas,s)=>{
                const r=canvas.getBoundingClientRect(),m=lat=>Math.log(Math.tan(Math.PI/4+Math.max(-80,Math.min(80,lat))*Math.PI/360));
                const x=r.x+r.width/2+(s.position.lon-s.camera.lon)*Math.PI/180*s.camera.scale;
                const y=r.y+r.height/2-(m(s.position.lat)-m(s.camera.lat))*s.camera.scale;
                return {x,y,visible:x>r.left&&x<r.right&&y>r.top&&y<r.bottom&&document.elementFromPoint(x,y)===canvas};
              },before);
              assert(before.position&&hit.visible,'the initial ship is directly touchable on the map');
              await page.mouse.click(hit.x,hit.y);await settle(page);
              const after=await state(page);
              assert.equal(after.selection,'ship');
              assert.equal(after.day,before.day,'touching the ship does not reset the date');
            });
            await step('sparse evidence stays a gap instead of an invented route',async()=>{
              const gaps=observations.slice(1).map((b,i)=>({a:observations[i],b,span:b.day-observations[i].day})).sort((a,b)=>b.span-a.span);
              const gap=gaps[0];
              await setDay(page,(gap.a.day+gap.b.day)/2);
              const s=await state(page),text=await page.locator('#position-status').textContent();
              assert(s.position?.kind==='gap'||(s.position===null&&/gap|unrecorded|unknown|not known|no position/i.test(text)),'largest observation interval is explicitly unlocated');
              result.evidenceGap={days:gap.span,status:text};
            });
            await step('outside the voyage has an explicit return without resetting attention',async()=>{
              await setDay(page,observations[0].day-10);
              const outside=await state(page);
              assert(!outside.position||outside.position.kind==='outside','no observed ship outside the voyage');
              const returnButton=page.locator('#return-voyage');
              assert(await returnButton.isVisible(),'return is offered outside voyage');
              await returnButton.click();await settle(page);
              const returned=await state(page);
              assert(Math.abs(returned.day-initialDay)<.011);
              assert.equal(returned.selection,outside.selection);
              assert.equal(returned.span,outside.span);
              assert.equal(await returnButton.isVisible(),false,'return is hidden inside voyage');
            });
            await step('follow control is explicit and can be turned off',async()=>{
              const before=await state(page);
              await page.locator('#follow-ship').click();
              assert.equal((await state(page)).follow,!before.follow);
              await unfollow();
            });
            await step('timeline drag changes day while map pan keeps it',async()=>{
              await setDay(page,initialDay);
              const a=await state(page);
              await mouseDrag(page,page.locator('#ruler'),[.45,.42],[.65,.42]);
              const b=await state(page);assert(Math.abs(b.day-a.day)>.1,'ruler drag changes world time');
              await mouseDrag(page,page.locator('#map'),[.40,.48],[.60,.50]);
              const c=await state(page);assert.equal(c.day,b.day);assert.notEqual(c.camera.lon,b.camera.lon,'map pans');
            });
            await step('timeline zoom keeps map camera and current day',async()=>{
              await setDay(page,initialDay);
              const a=await state(page);
              await page.locator('#timeline-tools summary').click();
              await page.locator('#zoom-in').click();
              const b=await state(page);
              assert(b.span<a.span);assert.deepEqual(b.camera,a.camera);assert.equal(b.day,a.day);
              await page.locator('#timeline-tools summary').click();
            });
            await step(engine==='chromium'?'actual CDP two-finger time pinch':'WebKit pointer pinch handler fallback',async()=>{
              const a=await state(page);await pinch(page,context,engine);const b=await state(page);
              assert(b.span<a.span*.9);assert.deepEqual(b.camera,a.camera);
            });
            await step('keyboard map movement pauses playback',async()=>{
              await setDay(page,initialDay);
              await page.locator('#play-toggle').click();
              await page.waitForFunction(()=>window.shipAtlasState().playing);
              const a=await state(page);await page.locator('#map').focus();await page.keyboard.press('ArrowLeft');
              const b=await state(page);assert.equal(b.playing,false);assert.notEqual(b.camera.lon,a.camera.lon);
            });
            await step('backgrounding and Reduce Motion hold the same clock',async()=>{
              await settings(page,{active:true,reduceMotion:false,textScale:1});
              const a=await state(page);await page.locator('#play-toggle').click();
              await page.waitForFunction(d=>window.shipAtlasState().playing&&window.shipAtlasState().day>d,a.day);
              await settings(page,{active:false});const paused=await state(page);
              assert.equal(paused.active,false);assert.equal(paused.playing,false);
              await page.waitForTimeout(140);assert.equal((await state(page)).day,paused.day);
              await settings(page,{active:true,reduceMotion:true});const reduced=await state(page);
              await page.locator('#play-toggle').click();await page.waitForTimeout(100);
              assert.equal((await state(page)).playing,false);assert.equal((await state(page)).day,reduced.day);
            });
            await step('large text keeps controls reachable',async()=>{
              await settings(page,{active:true,reduceMotion:false,textScale:3.2});
              await setDay(page,observations[Math.floor(observations.length/2)].day);await settle(page);
              const overflow=await page.evaluate(()=>({viewport:innerWidth,html:document.documentElement.scrollWidth,body:document.body.scrollWidth,scale:getComputedStyle(document.documentElement).getPropertyValue('--text-scale')}));
              assert(Number(overflow.scale)>=3.19);assert(overflow.html<=overflow.viewport+1,JSON.stringify(overflow));assert(overflow.body<=overflow.viewport+1,JSON.stringify(overflow));
              for(const id of ['date-toggle','year-slider','sources-open','map-zoom-in','map-zoom-out','select-ship','select-empire','follow-ship','clear-selection']){
                const control=page.locator('#'+id);await control.scrollIntoViewIfNeeded();await control.click({trial:true,timeout:3000});
              }
              const summary=page.locator('#timeline-tools summary');await summary.scrollIntoViewIfNeeded();
              if(!await page.locator('#timeline-tools').evaluate(el=>el.open))await summary.click();
              for(const id of ['zoom-in','zoom-out','previous-snapshot','next-snapshot']){
                const c=page.locator('#'+id);await c.scrollIntoViewIfNeeded();await c.click({trial:true,timeout:3000});
              }
              await summary.click();await shot('large-text');
              await settings(page,{textScale:1});await settle(page);
              await page.locator('#sources-open').click();assert(await page.locator('#sources-dialog').evaluate(el=>el.open));
              await page.locator('#sources-close').click();
            });
            await step('empire overview is the same date at a different scale',async()=>{
              const a=await state(page);await page.locator('#select-empire').click();const b=await state(page);
              assert.equal(b.day,a.day);assert(b.span>a.span*20);assert.deepEqual(b.camera,a.camera);
              await shot('empire-scale');
            });
            await step('runtime close removes handlers and animation',async()=>{
              await page.evaluate(()=>terra._receive({type:'close'}));
              const a=await state(page);assert.equal(a.disposed,true);assert.equal(a.playing,false);
              await page.evaluate(()=>{
                const slider=document.getElementById('year-slider');slider.value=0;slider.dispatchEvent(new Event('input',{bubbles:true}));
                document.getElementById('select-ship').click();document.getElementById('follow-ship').click();
                document.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowLeft',bubbles:true}));
              });
              await page.waitForTimeout(120);const b=await state(page);
              assert.equal(b.day,a.day);assert.equal(b.span,a.span);assert.equal(b.selection,a.selection);assert.equal(b.follow,a.follow);assert.deepEqual(b.camera,a.camera);
            });
            assert.deepEqual(result.errors,[],'no browser errors or external requests');result.passed=true;
          }catch(e){
            result.errors.push(phase+': '+e.message);
            try{result.state=await state(page);}catch(_){}
            try{await shot('failure');}catch(_){}
          }finally{await context.close();}
          if(!result.passed)report.failures.push({test:label,errors:result.errors});
        }
      }catch(e){report.failures.push({test:engine,errors:[e.stack||String(e)]});}
      finally{if(browser)await browser.close();}
    }
  }catch(e){report.failures.push({test:'setup',errors:[e.stack||String(e)]});}
  finally{
    if(server)await server.close();
    report.passed=report.failures.length===0&&report.tests.length>0&&report.tests.every(t=>t.passed);
    fs.writeFileSync(path.join(OUT,'results.json'),JSON.stringify(report,null,2)+'\n');
    console.log(JSON.stringify(report,null,2));process.exitCode=report.passed?0:1;
  }
})().catch(e=>{console.error(e);process.exitCode=1;});

'use strict';
const {chromium}=require('playwright'),assert=require('node:assert/strict');
const fs=require('node:fs');
const close=(a,b,epsilon=.001)=>Math.abs(a-b)<epsilon;
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'/usr/bin/chromium',args:['--no-sandbox']});
 const context=await browser.newContext({viewport:{width:1366,height:1024},hasTouch:true});
 const p=await context.newPage(),errors=[],requests=[];
 p.on('pageerror',e=>errors.push(e.message));p.on('request',r=>requests.push(r.url()));
 await p.goto(process.env.LEAF_URL||'http://127.0.0.1:8787');await p.waitForFunction(()=>window.leafReady);await p.waitForTimeout(950);
 const state=()=>p.evaluate(()=>leafDiagnostics());
 const chart=await p.locator('#chart').boundingBox();
 const point=async(lon,lat)=>{const a=await p.evaluate(([a,b])=>leafPoint(a,b),[lon,lat]);return [a[0]+chart.x,a[1]+chart.y];};
 let s=await state();assert.equal(s.soundOn,false);assert.equal(s.note,null);
 await p.screenshot({path:'out/leaf-ipad.png'});
 // Pan and preserve that position through a note and its source disclosure.
 await p.mouse.move(chart.x+180,chart.y+400);await p.mouse.down();await p.mouse.move(chart.x+230,chart.y+410,{steps:8});await p.mouse.up();await p.waitForTimeout(80);
 s=await state();assert.ok(Math.abs(s.cam.x-s.home.x)>20);const before={...s.cam};
 await p.locator('#menu-button').click();await p.locator('#note-index button').nth(1).click();
 assert.equal((await state()).note,'santorini');await p.locator('#note summary').click();assert.ok(await p.locator('#note-sources').isVisible());await p.screenshot({path:'out/leaf-ipad-note.png'});
 await p.locator('.return-to-water').click();s=await state();assert.deepEqual(s.cam,before);
 // Touch target supports starting a pan without opening a note.
 let [sx,sy]=await point(25.4,36.4);await p.mouse.move(sx,sy);await p.mouse.down();await p.mouse.move(sx+65,sy,{steps:6});await p.mouse.up();await p.waitForTimeout(100);assert.equal((await state()).note,null);
 await p.locator('#home').click();
 // Brief water tap does not start the glass. A long press does; cancel clears it.
 let [x,y]=await point(24.0,35.9);await p.mouse.click(x,y);await p.waitForTimeout(70);assert.equal((await state()).lens,undefined);
 await p.mouse.move(x,y);await p.mouse.down();await p.waitForTimeout(480);assert.ok((await state()).lens);await p.screenshot({path:'out/leaf-ipad-glass.png'});await p.mouse.up();assert.equal((await state()).lens,undefined);
 // Pinch around an arbitrary fixed geographic point; ending one finger must not jump the camera.
 await p.locator('#home').click();const session=await context.newCDPSession(p);const ax=chart.x+chart.width*.57,ay=chart.y+chart.height*.55;
 await session.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:ax-45,y:ay,id:1},{x:ax+45,y:ay,id:2}]});
 for(let i=1;i<=8;i++)await session.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:ax-45-i*5,y:ay,id:1},{x:ax+45+i*5,y:ay,id:2}]});
 await p.waitForTimeout(80);const pinched=await state();assert.ok(pinched.cam.s>pinched.home.s*1.7);
 await session.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});s=await state();assert.ok(close(s.cam.x,pinched.cam.x)&&close(s.cam.y,pinched.cam.y));
 // Stillness is respected, including pausing the surprise rather than hiding its accessible note.
 await p.locator('#menu-button').click();await p.locator('#motion').click();await p.locator('.close-menu').click();const t=(await state()).time;await p.waitForTimeout(150);assert.equal((await state()).time,t);await p.evaluate(()=>{leafSettings({active:false});leafSettings({active:true});});assert.equal((await state()).still,true);
 await p.locator('#light').fill('100');await p.locator('#light').dispatchEvent('input');await p.waitForTimeout(100);assert.equal((await state()).light,1);await p.screenshot({path:'out/leaf-ipad-blue-hour.png'});
 // The menu provides a non-timed and keyboard-operable alternative to the long press.
 await p.locator('#menu-button').click();await p.locator('#glass').click();assert.ok((await state()).lens);await p.keyboard.press('ArrowRight');let lens=(await state()).lens;assert.ok(lens[0]>chart.width/2);await p.locator('#lens-close').click();
 await p.locator('#menu-button').click();await p.locator('#handle-skiff').click();await p.waitForFunction(()=>document.activeElement.id==='skiff');const sail=(await state()).boat.sail;await p.keyboard.press('Enter');assert.notEqual((await state()).boat.sail,sail);const bx=(await state()).boat.x;await p.keyboard.press('ArrowRight');assert.ok((await state()).boat.x>bx);
 // Labels should never overlap each other, at base size and larger native text settings.
 for(const factor of [1,1.6]){
   await p.evaluate(f=>leafSettings({textSize:f,reduceMotion:true}),factor);await p.locator('#home').click();await p.waitForTimeout(80);s=await state();
   for(let i=0;i<s.labels.length;i++)for(let j=i+1;j<s.labels.length;j++){let a=s.labels[i],b=s.labels[j];assert.ok(!(a.x<b.x+b.w&&a.x+a.w>b.x&&a.y<b.y+b.h&&a.y+a.h>b.y),`${a.name} overlaps ${b.name}`);}
 }
 await context.close();
 // Fresh phone view, Reduce Motion, and oversized reading text in the actual scrolled source panel.
 const phone=await browser.newContext({viewport:{width:402,height:874},hasTouch:true,reducedMotion:'reduce'});const q=await phone.newPage();q.on('pageerror',e=>errors.push(e.message));await q.goto(process.env.LEAF_URL||'http://127.0.0.1:8787');await q.waitForFunction(()=>window.leafReady);await q.waitForTimeout(900);assert.equal((await q.evaluate(()=>leafDiagnostics())).still,true);await q.screenshot({path:'out/leaf-iphone.png'});
 await q.evaluate(()=>leafSettings({textSize:3.12,reduceMotion:true}));await q.locator('#menu-button').click();await q.locator('#note-index button').nth(3).click();await q.locator('#note summary').click();await q.screenshot({path:'out/leaf-iphone-large-text.png'});
 const overflow=await q.locator('#note').evaluate(el=>el.scrollWidth>el.clientWidth+1);assert.equal(overflow,false);await q.locator('.return-to-water').click();await q.waitForTimeout(100);assert.equal((await q.evaluate(()=>leafDiagnostics())).note,null);
 assert.deepEqual(errors,[]);assert.ok(requests.every(u=>u.startsWith('http://127.0.0.1:8787/')));console.log('PASS: bundled resources, pan/note return, gesture conflict, long press, pinch, camera continuity, stillness, blue hour, keyboard glass and skiff, label collisions, phone large text, local requests only.');
 fs.writeFileSync('out/leaf-check.json',JSON.stringify({passed:true,checks:14,browser:'Chromium',requests:requests.length,errors},null,2)+'\n');await browser.close();
})().catch(e=>{console.error(e);process.exit(1);});

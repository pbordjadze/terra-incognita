/* Runs on the exact bundled surface. Needs Playwright + a local static server.
   node tools/water/interaction_check.cjs [url] [output-directory] */
const {chromium}=require('playwright'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
(async()=>{
 const url=process.argv[2]||'http://127.0.0.1:8765',out=process.argv[3]||'/tmp/water-check';fs.mkdirSync(out,{recursive:true});
 const b=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'/usr/bin/chromium',args:['--no-sandbox']});
 const results=[];
 try{for(const config of [{name:'phone',width:402,height:874},{name:'small-phone',width:320,height:568},{name:'ipad',width:1032,height:1376},{name:'large-text',width:402,height:874,large:true},{name:'reduced-motion',width:402,height:874,reduced:true}]){
  const p=await b.newPage({viewport:{width:config.width,height:config.height},deviceScaleFactor:2,hasTouch:true,isMobile:true,reducedMotion:config.reduced?'reduce':'no-preference'});
  const errors=[],external=[];p.on('pageerror',e=>errors.push(e.message));p.on('request',r=>{if(!r.url().startsWith(url))external.push(r.url());});
  const start=Date.now();await p.goto(url);await p.waitForFunction(()=>window.waterDiagnostics?.().ready);const startup=Date.now()-start;
  if(config.large)await p.evaluate(()=>setReaderScale(1.8));
  await p.waitForTimeout(180);assert.equal((await p.evaluate(()=>document.documentElement.scrollWidth)),config.width,'No horizontal document overflow');
  await p.screenshot({path:path.join(out,config.name+'-01-open.png')});
  const box=await p.locator('#map').boundingBox();assert(box.height>=160,'Map remains usable');
  // A real drag brushes multiple rivers; this is not a JS-injected state fixture.
  await p.mouse.move(box.x+box.width*.3,box.y+box.height*.25);await p.mouse.down();await p.waitForTimeout(150);
  await p.mouse.move(box.x+box.width*.72,box.y+box.height*.4,{steps:38});await p.mouse.up();await p.waitForTimeout(120);
  const afterBrush=await p.locator('#map').boundingBox();assert(Math.abs(afterBrush.height-box.height)<1,'Changing a caption must not shift the map under the hand');
  let d=await p.evaluate(()=>waterDiagnostics());assert(d.traces>1,'Direct brush must leave multiple traced routes');assert.equal(d.pointers,0);
  await p.screenshot({path:path.join(out,config.name+'-02-brush.png')});
  await p.click('#gather');await p.selectOption('#river-menu','0');await p.waitForTimeout(1050);
  d=await p.evaluate(()=>waterDiagnostics());assert(d.treeEdges>1500,'Amazon gather reveals the data-derived network');
  await p.screenshot({path:path.join(out,config.name+'-03-gather.png')});
  // Controls for users who do not use two-finger navigation.
  const z=d.zoom;await p.click('#zoom-in');assert((await p.evaluate(()=>waterDiagnostics().zoom))>z);
  await p.click('#move');await p.mouse.move(box.x+box.width*.45,box.y+box.height*.5);await p.mouse.down();await p.mouse.move(box.x+box.width*.6,box.y+box.height*.55,{steps:12});await p.mouse.up();
  assert.equal((await p.evaluate(()=>waterDiagnostics().pointers)),0);await p.click('#move');
  await p.click('#pause');assert.equal(await p.getAttribute('#pause','aria-pressed'),'true');await p.click('#pause');
  await p.click('#about');assert(await p.locator('#evidence').evaluate(e=>e.open));
  await p.screenshot({path:path.join(out,config.name+'-04-notes.png')});
  // Scroll to the bottom. The close button must remain in the viewport.
  await p.locator('#evidence').evaluate(e=>e.scrollTop=e.scrollHeight);
  assert(await p.locator('#close-about').isVisible());await p.click('#close-about');assert(!(await p.locator('#evidence').evaluate(e=>e.open)));
  assert.equal(await p.evaluate(()=>document.activeElement.id),'about','Focus returns to the invoking control');
  await p.click('#clear');d=await p.evaluate(()=>waterDiagnostics());assert.equal(d.traces,0);assert.equal(d.treeEdges,0);
  if(config.reduced)assert.equal(d.reduced,true);
  assert.deepEqual(errors,[]);assert.deepEqual(external,[],'Bundled map must make no external requests');
  results.push({device:config.name,startupMs:startup,mapHeight:box.height,passed:true});await p.close();
 }
 // Native touch events include cancellation of a one-finger brush when a pinch begins.
 const p=await b.newPage({viewport:{width:402,height:874},hasTouch:true,isMobile:true});await p.goto(url);await p.waitForFunction(()=>window.waterDiagnostics?.().ready);
 const cdp=await p.context().newCDPSession(p),box=await p.locator('#map').boundingBox(),y=box.y+box.height*.4;
 const a={x:150,y,id:1},c={x:250,y,id:2};
 await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[a]});await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[a,c]});
 for(let i=0;i<15;i++)await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{...a,x:150-i*3},{...c,x:250+i*3}]});
 await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[{...a,x:108}]});
 await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{...a,x:122,y:y+35}]});
 await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
 const d=await p.evaluate(()=>waterDiagnostics());assert(d.zoom>1.5);assert.equal(d.pointers,0);assert.equal(d.traces,0,'Pinch does not leave accidental ink');
 await p.close();results.push({device:'two-finger-pinch',zoom:d.zoom,passed:true});
 fs.writeFileSync(path.join(out,'results.json'),JSON.stringify(results,null,2));console.log(JSON.stringify(results,null,2));
 }finally{await b.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});

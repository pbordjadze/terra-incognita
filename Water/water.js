/* An offline instrument for playing with a mapped river graph. */
'use strict';
(() => {
const $=id=>document.getElementById(id), canvas=$('map'), ctx=canvas.getContext('2d',{alpha:false});
const core=window.WaterCore,{C,W,H,project,invert}=core;
const net=core.build(window.WATER_DATA), edges=net.edges;
const base=document.createElement('canvas'), bctx=base.getContext('2d',{alpha:false});
const ink=document.createElement('canvas'), ictx=ink.getContext('2d');
const media=matchMedia('(prefers-reduced-motion: reduce)');
let reduced=media.matches||!!window.__nativeReduceMotion,paused=false,mode='follow',moving=false,ready=false;
let width=0,height=0,dpr=1,scale=1,tx=0,ty=0,fitScale=1,zoom=1,baseDirty=true,inkDirty=true;
let traces=[],tree=null,contact=null,focus=null,treeBorn=0,lastSample=0,lastHit=-1,lastFrame=0,raf=0,now=performance.now();
let touched=false,downTimer=0,statusTimer=0,dragged=false,keyboard=false;
const pointers=new Map();let gesture=null;
const palette=['#244e9b','#a24e39','#38685a','#62518d','#826423'];
const named=new Map(),starts=[];
const colors=new Map();
function color(basin){if(!colors.has(basin))colors.set(basin,palette[Math.abs(basin*37)%palette.length]);return colors.get(basin);}
function linePath(xy){const p=new Path2D();if(xy.length){p.moveTo(xy[0],xy[1]);for(let j=2;j<xy.length;j+=2)p.lineTo(xy[j],xy[j+1]);}return p;}
const paths=edges.map(e=>linePath(e.xy));
const tierPaths=Array.from({length:12},()=>new Path2D());
edges.forEach(e=>tierPaths[Math.min(11,e.order)].addPath(paths[e.i]));
const landPaths=(window.WATER_LAND||[]).map(poly=>{const p=new Path2D();poly.forEach(ring=>{ring.forEach((pt,i)=>{const q=project(pt[0],pt[1]);if(i)p.lineTo(...q);else p.moveTo(...q);});p.closePath();});return p;});
const terrain=new Image();terrain.src='terrain.jpg';
const places=window.WATER_PLACES||{cities:[],rivers:[]};
for(const r of places.rivers){
 const p=project(...r.at),hit=net.nearest(...p,.55,3);
 if(!hit)continue;
 const e=edges[hit.id];if(r.system)named.set(e.basin,r.system);
 starts.push({...r,hit});const o=document.createElement('option');o.value=String(starts.length-1);o.textContent=r.name;$('river-menu').appendChild(o);
}
// Named systems use their own quiet ink; all branches retain that same identity.
starts.forEach((r,i)=>{const k=edges[r.hit.id].basin;if(!colors.has(k))colors.set(k,palette[i%palette.length]);});
function minOrder(){return zoom<1.65?4:zoom<3.5?3:1;}
function screen(x,y){return [x*scale+tx,y*scale+ty];}
function world(x,y){return [(x-tx)/scale,(y-ty)/scale];}
function transform(c){c.setTransform(dpr*scale,0,0,dpr*scale,dpr*tx,dpr*ty);}
function wake(){if(!raf&&!document.hidden)raf=requestAnimationFrame(frame);}
function resetCamera(){
 fitScale=Math.min((width-42)/W,(height-22)/H);scale=fitScale;zoom=1;tx=(width-W*scale)/2;ty=(height-H*scale)/2;
 baseDirty=true;inkDirty=true;wake();
}
function clampCamera(){
 // Keep some of the continent in reach; never lose the page to unbounded panning.
 tx=Math.max(-W*scale+width*.25,Math.min(width*.75,tx));
 ty=Math.max(-H*scale+height*.2,Math.min(height*.8,ty));
}
function resize(){
 const rect=canvas.getBoundingClientRect(), old=[width,height];width=Math.round(rect.width);height=Math.round(rect.height);dpr=Math.min(devicePixelRatio||1,2.5);
 for(const c of [canvas,base,ink]){c.width=Math.round(width*dpr);c.height=Math.round(height*dpr);}
 if(!ready||!old[0])resetCamera();else{tx+=(width-old[0])/2;ty+=(height-old[1])/2;fitScale=Math.min((width-42)/W,(height-22)/H);zoom=scale/fitScale;clampCamera();baseDirty=inkDirty=true;wake();}
}
function drawBase(){
 const c=bctx;c.setTransform(dpr,0,0,dpr,0,0);c.fillStyle='#e1eaeb';c.fillRect(0,0,width,height);transform(c);
 if(terrain.complete&&terrain.naturalWidth)c.drawImage(terrain,0,0,W,H);
 c.strokeStyle='#849ba345';c.lineWidth=.55/scale;
 for(let lon=-80;lon<=-40;lon+=10){const p=project(lon,0);c.beginPath();c.moveTo(p[0],0);c.lineTo(p[0],H);c.stroke();}
 for(let lat=-50;lat<=10;lat+=10){const p=project(0,lat);c.beginPath();c.moveTo(0,p[1]);c.lineTo(W,p[1]);c.stroke();}
 c.strokeStyle='#6c85866b';c.lineWidth=.65/scale;landPaths.forEach(p=>c.stroke(p));
 c.lineCap='round';c.lineJoin='round';
 for(let o=minOrder();o<tierPaths.length;o++){
  const active=!!(traces.length||tree);
  c.strokeStyle=active?(o>=7?'#71909b88':'#8a9da35f'):(o>=7?'#3d638bc0':o>=5?'#4b70958a':'#597a935b');
  c.lineWidth=(o>=8?1.15:o>=6?.8:o>=4?.55:.4)*Math.min(1.35,Math.pow(zoom,.12))/scale;c.stroke(tierPaths[o]);
 }
 c.setTransform(dpr,0,0,dpr,0,0);
 drawNames(c);baseDirty=false;
}
function text(c,label,x,y,size,ink,italic=false){
 c.font=`${italic?'italic ':''}${size}px Libertinus, Georgia, serif`;c.textAlign='center';c.textBaseline='middle';c.lineJoin='round';c.lineWidth=3.5;c.strokeStyle='#efeee4e8';c.strokeText(label,x,y);c.fillStyle=ink;c.fillText(label,x,y);
}
function drawNames(c){
 const occupied=[];const reader=Math.min(1.5,Number(getComputedStyle(document.documentElement).getPropertyValue('--reader'))||1);
 const sizes=width>=700?1.15:1;
 function place(label,p,size,ink,italic=false){
  const [x,y]=screen(...project(...p));if(x<52||x>width-35||y<22||y>height-18)return;
  c.font=`${size}px Libertinus`;const w=c.measureText(label).width+14,h=size+12;
  if(x-w/2<4||x+w/2>width-4)return;
  const r=[x-w/2,y-h/2,x+w/2,y+h/2];if(occupied.some(a=>r[0]<a[2]&&r[2]>a[0]&&r[1]<a[3]&&r[3]>a[1]))return;
  occupied.push(r);text(c,label,x,y,size,ink,italic);
 }
 if(zoom<2.2){
  place('Pacific Ocean',[-81,-23],14*sizes*reader,'#526e82',true);
  place('Atlantic Ocean',[-38,-23],14*sizes*reader,'#526e82',true);
 }
 for(const r of places.rivers){if(r.label&&(zoom>=1.4||r.major))place(r.label,r.labelAt||r.at,(r.major?14:12)*sizes*reader,'#3c638a',true);}
 for(const city of places.cities){
  if(zoom<(city.min||1.6))continue;
  const [x,y]=screen(...project(...city.at));if(x<55||x>width-50||y<20||y>height-25)continue;
  c.fillStyle='#354e59';c.beginPath();c.arc(x,y,1.7,0,Math.PI*2);c.fill();
  // A city dot and its offset name remain anchored to the same geography.
  const p=city.at.slice();p[1]-=10/scale;
  place(city.name,p,11.5*sizes*reader,'#354e59');
 }
}
function fullInkPath(){
 if(!tree)return null;const p=new Path2D();tree.ids.forEach(i=>p.addPath(paths[i]));p.addPath(linePath(tree.partial));return p;
}
function drawInkCache(){
 const c=ictx;c.setTransform(1,0,0,1,0,0);c.clearRect(0,0,ink.width,ink.height);transform(c);
 if(tree){const p=fullInkPath();c.strokeStyle=tree.color;c.lineWidth=1.3/scale;c.lineCap='round';c.lineJoin='round';c.stroke(p);}
 inkDirty=false;
}
function paint(t){
 if(baseDirty)drawBase();if(inkDirty)drawInkCache();
 ctx.setTransform(1,0,0,1,0,0);ctx.globalAlpha=1;ctx.drawImage(base,0,0);
 if(tree){
  const age=t-treeBorn;
  if(reduced||paused||age>950){ctx.drawImage(ink,0,0);}
  else{
   ctx.globalAlpha=.18;ctx.drawImage(ink,0,0);ctx.globalAlpha=1;transform(ctx);ctx.strokeStyle=tree.color;ctx.lineCap='round';ctx.lineJoin='round';ctx.lineWidth=1.3/scale;
   const stop=Math.floor(age/950*24);for(let k=0;k<=Math.min(23,stop);k++)ctx.stroke(tree.buckets[k]);
  }
 }
 transform(ctx);ctx.lineCap='round';ctx.lineJoin='round';
 for(const tr of traces){
  const age=(paused?(tr.frozenAt??t):t)-tr.born;
  const alpha=paused||reduced?1:Math.min(1,(24000-age)/5000);
  if(alpha<=0)continue;
  ctx.globalAlpha=alpha;ctx.strokeStyle=tr.color;ctx.lineWidth=1.45/scale;
  if(reduced||paused||age>1700){ctx.stroke(tr.path);}else{
   ctx.save();ctx.setLineDash([tr.metrics.length, tr.metrics.length]);ctx.lineDashOffset=tr.metrics.length*(1-Math.min(1,age/1700));ctx.stroke(tr.path);ctx.restore();
  }
  if(!reduced&&!paused){
   const distance=((t-tr.born)/1000*tr.speed)%Math.max(tr.metrics.length,.001);
   ctx.beginPath();
   for(let n=0;n<=9;n++){
    const q=core.pointAt(tr.metrics,Math.max(0,distance-(9-n)*.1/Math.max(.4,zoom)));
    if(n)ctx.lineTo(...q);else ctx.moveTo(...q);
   }
   ctx.strokeStyle='#f9f8ec';ctx.lineWidth=1.7/scale;ctx.stroke();
   const q=core.pointAt(tr.metrics,distance);ctx.fillStyle=tr.color;ctx.beginPath();ctx.arc(...q,1.7/scale,0,Math.PI*2);ctx.fill();
  }
 }
 ctx.globalAlpha=1;
 if(contact){
  const p=contact;ctx.strokeStyle='#354f7db3';ctx.lineWidth=1/scale;
  if(p.finger&&Math.hypot(p.finger[0]-p.x,p.finger[1]-p.y)*scale>5){ctx.setLineDash([2/scale,3/scale]);ctx.beginPath();ctx.moveTo(...p.finger);ctx.lineTo(p.x,p.y);ctx.stroke();ctx.setLineDash([]);}
  ctx.fillStyle='#f5f1e7';ctx.beginPath();ctx.arc(p.x,p.y,4.5/scale,0,Math.PI*2);ctx.fill();ctx.stroke();ctx.fillStyle=color(edges[p.id].basin);ctx.beginPath();ctx.arc(p.x,p.y,1.8/scale,0,Math.PI*2);ctx.fill();
 }
 if(keyboard&&focus){ctx.strokeStyle='#354f7d';ctx.lineWidth=1.2/scale;ctx.beginPath();ctx.arc(...focus,12/scale,0,Math.PI*2);ctx.stroke();}
 ctx.setTransform(dpr,0,0,dpr,0,0);
}
function frame(t){
 raf=0;if(document.hidden)return;
 if(t-lastFrame<30){wake();return;}lastFrame=t;now=t;
 if(!paused&&!reduced){const count=traces.length;traces=traces.filter(tr=>t-tr.born<24000);if(count&&traces.length===0)baseDirty=true;}
 paint(t);
 if(!paused&&!reduced&&(traces.length||tree&&t-treeBorn<1000))wake();
}
function describe(hit,route){
 const e=edges[hit.id], name=named.get(e.basin),km=Math.max(1,Math.round(route.km/10)*10);
 const coord=invert(...route.end.xy.slice(-2));
 const ending=route.end.inland?'an inland ending':(coord[0]<-68&&coord[1]>-48?'the Pacific coast':coord[0]>-60?'the Atlantic coast':'the coast');
 const title=mode==='gather'?(name?`The ${name} system, gathered at your touch`:'Tributaries, gathered at your touch'):(name?`Toward the ${name} system`:'A river finding its ending');
 const detail=mode==='gather'?'Connected channels upstream · larger rivers only':`~${km.toLocaleString()} km along mapped channels to ${ending}`;
 clearTimeout(statusTimer);statusTimer=setTimeout(()=>{$('river-title').textContent=title;$('river-detail').textContent=detail;},80);
 canvas.setAttribute('aria-description',`${title}. ${detail}.`);
}
function traceAt(x,y,force=false){
 const hit=net.nearest(x,y,24/scale,minOrder());
 if(!hit){contact=null;wake();return false;}
 contact={...hit,finger:[x,y]};
 const t=performance.now();if(!force&&(t-lastSample<(mode==='gather'?85:55)||hit.id===lastHit)){wake();return true;}
 lastSample=t;lastHit=hit.id;touched=true;baseDirty=true;const route=net.downstream(hit),e=edges[hit.id];
 if(mode==='gather'){
  tree={...net.upstream(hit),color:color(e.basin),hit};treeBorn=t;inkDirty=true;
  const dist=new Map(),queue=edges[hit.id].up.map(i=>[i,core.pathMetrics(tree.partial).length]);let farthest=.001;
  while(queue.length){const [i,d]=queue.pop();dist.set(i,d);farthest=Math.max(farthest,d+edges[i].length);edges[i].up.forEach(j=>queue.push([j,d+edges[i].length]));}
  tree.buckets=Array.from({length:24},()=>new Path2D());tree.buckets[0].addPath(linePath(tree.partial));
  for(const i of tree.ids)tree.buckets[Math.min(23,Math.floor(dist.get(i)/farthest*24))].addPath(paths[i]);
 }else{
  const metrics=core.pathMetrics(route.xy);
  traces.push({path:linePath(route.xy),metrics,born:t,color:color(e.basin),id:hit.id,speed:Math.max(.35,metrics.length/9),frozenAt:paused?t:null});
  if(traces.length>36)traces.shift();
 }
 describe(hit,route);wake();return true;
}
function atPointer(e){const r=canvas.getBoundingClientRect();return [e.clientX-r.left,e.clientY-r.top];}
function stopDelay(){clearTimeout(downTimer);downTimer=0;}
canvas.addEventListener('pointerdown',e=>{
 if(e.button&&e.button!==0)return;
 e.preventDefault();keyboard=false;canvas.setPointerCapture(e.pointerId);const p=atPointer(e);pointers.set(e.pointerId,p);dragged=false;
 if(pointers.size===1){
  lastHit=-1;gesture={start:p,tx,ty,scale};
  if(!moving)downTimer=setTimeout(()=>{if(pointers.size===1)traceAt(...world(...p),true);},100);
 }else{
  stopDelay();contact=null;const a=[...pointers.values()];
  gesture={distance:Math.hypot(a[0][0]-a[1][0],a[0][1]-a[1][1]),center:[(a[0][0]+a[1][0])/2,(a[0][1]+a[1][1])/2],tx,ty,scale};
 }
 wake();
});
canvas.addEventListener('pointermove',e=>{
 if(!pointers.has(e.pointerId))return;e.preventDefault();const p=atPointer(e);pointers.set(e.pointerId,p);
 if(pointers.size>=2&&gesture?.distance){
  const a=[...pointers.values()],dist=Math.hypot(a[0][0]-a[1][0],a[0][1]-a[1][1]),center=[(a[0][0]+a[1][0])/2,(a[0][1]+a[1][1])/2];
  scale=Math.max(fitScale,Math.min(fitScale*7,gesture.scale*dist/Math.max(1,gesture.distance)));
  tx=center[0]-(gesture.center[0]-gesture.tx)*scale/gesture.scale;ty=center[1]-(gesture.center[1]-gesture.ty)*scale/gesture.scale;zoom=scale/fitScale;
  clampCamera();baseDirty=inkDirty=true;wake();
 }else if(pointers.size===1&&gesture?.start){
  if(Math.hypot(p[0]-gesture.start[0],p[1]-gesture.start[1])>3)dragged=true;
  if(moving){tx=gesture.tx+p[0]-gesture.start[0];ty=gesture.ty+p[1]-gesture.start[1];clampCamera();baseDirty=inkDirty=true;wake();}
  else if(dragged){stopDelay();traceAt(...world(...p));}
 }
});
function endPointer(e){
 if(!pointers.has(e.pointerId))return;const p=pointers.get(e.pointerId);
 if(pointers.size===1&&downTimer&&!moving)traceAt(...world(...p),true);
 stopDelay();pointers.delete(e.pointerId);
 if(pointers.size){const a=[...pointers.values()][0];gesture={start:a,tx,ty,scale};}else{gesture=null;contact=null;lastHit=-1;}
 wake();
}
canvas.addEventListener('pointerup',endPointer);
canvas.addEventListener('pointercancel',e=>{stopDelay();pointers.delete(e.pointerId);gesture=null;contact=null;wake();});
function zoomAt(factor,x=width/2,y=height/2){
 const old=scale;scale=Math.max(fitScale,Math.min(fitScale*7,scale*factor));tx=x-(x-tx)*scale/old;ty=y-(y-ty)*scale/old;zoom=scale/fitScale;clampCamera();baseDirty=inkDirty=true;wake();
}
canvas.addEventListener('wheel',e=>{e.preventDefault();zoomAt(Math.exp(-e.deltaY*.0015),...atPointer(e));},{passive:false});
canvas.addEventListener('keydown',e=>{
 if(['ArrowUp','ArrowDown','ArrowLeft','ArrowRight',' ','Enter','+','=','-','Escape'].includes(e.key))e.preventDefault();else return;
 if(e.key==='+'||e.key==='='){zoomAt(1.5);return;}if(e.key==='-'){zoomAt(1/1.5);return;}
 if(e.key==='Escape'){clearInk();return;}
 keyboard=true;if(!focus)focus=world(width/2,height/2);
 const step=18/scale;
 if(e.key==='ArrowUp')focus[1]-=step;if(e.key==='ArrowDown')focus[1]+=step;if(e.key==='ArrowLeft')focus[0]-=step;if(e.key==='ArrowRight')focus[0]+=step;
 if(e.key===' '||e.key==='Enter')traceAt(...focus,true);wake();
});
function setMode(value){mode=value;$('follow').setAttribute('aria-pressed',String(mode==='follow'));$('gather').setAttribute('aria-pressed',String(mode==='gather'));lastHit=-1;
 $('invitation').textContent=mode==='gather'?'Touch a river. Gather what flows into it.':'Brush a river. See where it takes you.';
 if(contact)traceAt(contact.x,contact.y,true);
}
function clearInk(){traces=[];tree=null;contact=null;baseDirty=inkDirty=true;lastHit=-1;clearTimeout(statusTimer);$('river-title').textContent='Every line has somewhere to go.';$('river-detail').textContent='Trace with one finger · Move & zoom with two';$('river-menu').value='';wake();}
$('follow').onclick=()=>setMode('follow');$('gather').onclick=()=>setMode('gather');$('clear').onclick=clearInk;
$('move').onclick=()=>{moving=!moving;$('move').setAttribute('aria-pressed',String(moving));canvas.classList.toggle('moving',moving);$('invitation').textContent=moving?'Drag to move. Switch this control off to trace.':mode==='gather'?'Touch a river. Gather what flows into it.':'Brush a river. See where it takes you.';};
$('zoom-in').onclick=()=>zoomAt(1.5);$('zoom-out').onclick=()=>zoomAt(1/1.5);$('fit').onclick=resetCamera;
$('river-menu').onchange=()=>{
 const r=starts[Number($('river-menu').value)];if(!r||$('river-menu').value==='')return;
 const p=project(...r.at),hit=r.hit;const points=mode==='gather'?net.upstream(hit):null;
 const xy=points?points.ids.flatMap(i=>edges[i].xy).concat(points.partial):net.downstream(hit).xy;
 let x0=Infinity,y0=Infinity,x1=-Infinity,y1=-Infinity;for(let j=0;j<xy.length;j+=2){x0=Math.min(x0,xy[j]);x1=Math.max(x1,xy[j]);y0=Math.min(y0,xy[j+1]);y1=Math.max(y1,xy[j+1]);}
 scale=Math.max(fitScale,Math.min(fitScale*5,Math.min((width-90)/Math.max(5,x1-x0),(height-75)/Math.max(7,y1-y0))));zoom=scale/fitScale;tx=width/2-(x0+x1)/2*scale;ty=height/2-(y0+y1)/2*scale;clampCamera();baseDirty=inkDirty=true;
 traceAt(hit.x,hit.y,true);contact=null;wake();
};
function setPaused(value){
 const t=performance.now();paused=value;
 for(const tr of traces){if(paused)tr.frozenAt=t;else if(tr.frozenAt!=null){tr.born+=t-tr.frozenAt;tr.frozenAt=null;}}
 $('pause').setAttribute('aria-pressed',String(paused));$('pause').textContent=paused?'Resume ink':'Pause ink';$('pause').setAttribute('aria-label',paused?'Resume flowing ink':'Pause flowing ink');wake();
}
$('pause').onclick=()=>setPaused(!paused);
$('about').onclick=()=>{$('evidence').showModal();};$('close-about').onclick=()=>{$('evidence').close();$('about').focus();};
$('evidence').addEventListener('click',e=>{if(e.target===$('evidence')){const r=e.target.getBoundingClientRect();if(e.clientX<r.left||e.clientX>r.right||e.clientY<r.top||e.clientY>r.bottom)e.target.close();}});
window.setReducedMotion=value=>{reduced=!!value;wake();};
media.addEventListener('change',e=>{reduced=e.matches;wake();});
document.addEventListener('visibilitychange',()=>{if(document.hidden){if(raf)cancelAnimationFrame(raf);raf=0;}else wake();});
window.setReaderScale=value=>{document.documentElement.style.setProperty('--reader',String(Math.max(1,Math.min(1.8,value))));baseDirty=inkDirty=true;requestAnimationFrame(resize);};
if(window.__nativeReaderScale)window.setReaderScale(window.__nativeReaderScale);
terrain.onload=()=>{baseDirty=true;wake();};
new ResizeObserver(resize).observe($('map-area'));
document.fonts.ready.then(()=>{ready=true;resize();baseDirty=true;$('loading').hidden=true;$('loading').style.display='none';wake();});
// Read-only diagnostics used by the interaction checks; no persisted user state.
window.waterDiagnostics=()=>({ready,mode,moving,paused,reduced,zoom,traces:traces.length,treeEdges:tree?tree.ids.length+1:0,pointers:pointers.size,framePixels:canvas.width*canvas.height,networkEdges:edges.length,title:$('river-title').textContent});
})();

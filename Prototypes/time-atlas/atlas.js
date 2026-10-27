/* Offline historical-map experiment. Data is dated evidence, never interpolated geometry. */
(function() {
'use strict';
const C=window.TimeAtlasCore,D=window.TIME_DATA,$=id=>document.getElementById(id);
if(!C||!D||!D.snapshots?.length) { $('selection-detail').textContent='The dated maps could not load. Return to the atlas and try again.';return; }
const snapshots=D.snapshots.slice().sort((a,b)=>a.year-b.year), dates=snapshots.map(s=>s.year);
const time=new C.TimeCamera({min:dates[0],max:dates.at(-1),year:1938,span:120});
const camera=new C.MapCamera();
const map=$('map'),ruler=$('ruler'),ctx=map.getContext('2d'),rc=ruler.getContext('2d');
const abort=new AbortController(),listen=(target,type,fn,options={})=>target.addEventListener(type,fn,{...options,signal:abort.signal});
let selection=null,playing=false,active=true,disposed=false,reduceMotion=matchMedia('(prefers-reduced-motion: reduce)').matches;
let mapSize=[0,0],rulerSize=[0,0],frame=0,lastFrame=0,dirty=true,textScale=1,lastSnapshot=null;
let initialFit=true,announcementTimer=0;
const mapPointers=new Map(),timePointers=new Map();
let mapGesture=null,timeGesture=null;
const hostUnsubscribe=[];
const current=()=>C.nearestSnapshot(snapshots,time.year);
const selectedFeatures=s=>selection?.kind==='entity'?s.features.filter(f=>C.subjectId(f)===selection.id):[];
function mark(){dirty=true;if(!frame&&!disposed)frame=requestAnimationFrame(render);}
function announce(text){clearTimeout(announcementTimer);announcementTimer=setTimeout(()=>{$('announcer').textContent=text;},220);}
function setPlaying(value){
  playing=!!value&&active&&!reduceMotion&&!document.hidden;
  $('play-toggle').setAttribute('aria-pressed',String(playing));
  $('play-toggle').setAttribute('aria-label',playing?'Pause time':'Play through time');
  $('play-toggle').firstElementChild.textContent=playing?'Ⅱ':'▶';
  lastFrame=0;mark();
}
function stop(){if(playing)setPlaying(false);}
function expand(value=true){
  $('time-panel').classList.toggle('collapsed',!value);
  $('date-toggle').setAttribute('aria-expanded',String(value));
  mark();
}
function selectEntity(id,name,{focus=false}={}){
  stop();selection={kind:'entity',id,name};
  const available=snapshots.filter(s=>s.features.some(f=>C.subjectId(f)===id));
  if(available.length){
    if(focus&&!available.includes(current()))time.setYear(available[Math.floor(available.length/2)].year);
    time.focus(available[0].year,available.at(-1).year);
    if(focus)camera.fit(C.bounds(selectedFeatures(current())),mapSize[0],mapSize[1]*.7);
  }
  expand();update();announce(name+' selected. Drag the timeline to explore its dated maps.');
}
function selectPlace(point){
  stop();selection={kind:'place',point:point||[camera.lon,camera.lat]};expand();update();
  announce('Place selected. The geographic location stays selected as you change time.');
}
function update(){
  const s=current(),year=C.yearLabel(time.year),matches=selectedFeatures(s);
  $('current-date').textContent=year;
  $('year-slider').min=time.min;$('year-slider').max=time.max;$('year-slider').value=Math.round(time.year);
  $('year-slider').setAttribute('aria-valuetext',year+'; map snapshot '+s.year);
  $('range-start').textContent=time.min;$('range-end').textContent=time.max;
  let title='Europe, through time',detail='Touch a country, then move through its dated maps.';
  if(selection?.kind==='entity'){
    title=selection.name;
    detail=matches.length?'Following this polity as drawn on the dated maps.':'No matching state is drawn in the '+s.year+' snapshot.';
  }else if(selection?.kind==='place'){
    title='This place';
    const [lon,lat]=selection.point,owner=s.features.find(f=>C.contains(f.geometry,selection.point));
    detail=Math.abs(lat).toFixed(1)+'°'+(lat<0?'S':'N')+' · '+Math.abs(lon).toFixed(1)+'°'+(lon<0?'W':'E')+
      (owner?' · '+C.featureName(owner):' · No state mapped here in this snapshot');
  }
  $('selection-title').textContent=title;$('selection-detail').textContent=detail;
  $('clear-selection').hidden=!selection;
  document.querySelectorAll('[data-subject]').forEach(b=>b.setAttribute('aria-pressed',String(b.dataset.subject==='place'?selection?.kind==='place':selection?.id===b.dataset.subject)));
  $('map-caption').textContent='Map snapshot: '+s.year+' · Generalized historical boundaries · Modern land reference';
  $('time-context').textContent='Drag to travel · Pinch to change scale · Dots are available maps';
  $('previous-snapshot').disabled=!dates.some(y=>y<time.year-.01);
  $('next-snapshot').disabled=!dates.some(y=>y>time.year+.01);
  $('zoom-in').disabled=time.span<=time.minSpan+.01;
  $('zoom-out').disabled=time.span>=time.max-time.min-.01;
  $('map').setAttribute('aria-label',title+'. Selected date '+year+'. Map snapshot '+s.year+'. '+detail);
  if(s!==lastSnapshot){announce('Map snapshot '+s.year);lastSnapshot=s;}
  mark();
}
function sizeCanvas(canvas,size){
  const rect=canvas.getBoundingClientRect(),dpr=Math.min(2,window.devicePixelRatio||1);
  size[0]=rect.width;size[1]=rect.height;
  canvas.width=Math.round(rect.width*dpr);canvas.height=Math.round(rect.height*dpr);
  canvas.getContext('2d').setTransform(dpr,0,0,dpr,0,0);
}
function pathGeometry(geometry){
  ctx.beginPath();
  for(const poly of C.rings(geometry))for(const ring of poly){
    ring.forEach((point,i)=>{const p=camera.project(point,...mapSize);i?ctx.lineTo(...p):ctx.moveTo(...p);});ctx.closePath();
  }
}
function drawMap(){
  const [w,h]=mapSize;if(!w||!h)return;
  ctx.fillStyle='#c6ecff';ctx.fillRect(0,0,w,h);
  // Quiet geographic reference. No modern political boundaries are drawn.
  const land=D.land?.features||D.background?.features||[];
  for(const f of land){pathGeometry(f.geometry);ctx.fillStyle='#f6e1b9';ctx.fill('evenodd');ctx.strokeStyle='#91a9a7';ctx.lineWidth=.6;ctx.stroke();}
  const s=current(),features=s.features;
  for(const f of features){
    pathGeometry(f.geometry);
    const selected=selection?.kind==='entity'&&C.subjectId(f)===selection.id;
    ctx.fillStyle=selected?'#d9ad69':'#e9d8b6';ctx.fill('evenodd');
    ctx.strokeStyle=selected?'#76502b':'#7e8985';ctx.lineWidth=selected?1.9:.8;ctx.stroke();
  }
  // Labels remain sparse; selection is also available as accessible HTML.
  ctx.textAlign='center';ctx.textBaseline='middle';ctx.lineJoin='round';
  const used=[];
  for(const f of features){
    const selected=selection?.kind==='entity'&&C.subjectId(f)===selection.id,b=C.bounds([f]);if(!b)continue;
    const a=camera.project([b[0],b[1]],w,h),z=camera.project([b[2],b[3]],w,h);
    if(!selected&&(Math.abs(z[0]-a[0])<95||Math.abs(z[1]-a[1])<60))continue;
    const center=[(b[0]+b[2])/2,(b[1]+b[3])/2];if(!C.contains(f.geometry,center))continue;
    const p=camera.project(center,w,h),name=C.featureName(f),font=Math.min(18,13*textScale);
    ctx.font=(selected?'600 ':'')+font+'px Georgia';
    const tw=ctx.measureText(name).width;
    if(p[0]<tw/2+10||p[0]>w-tw/2-10||p[1]<130||p[1]>h-110)continue;
    if(used.some(r=>Math.abs(r.x-p[0])<(r.w+tw)/2+16&&Math.abs(r.y-p[1])<font+14))continue;
    used.push({x:p[0],y:p[1],w:tw});
    ctx.strokeStyle=selected?'#d9ad69':'#e9d8b6';ctx.lineWidth=4;ctx.strokeText(name,...p);
    ctx.fillStyle='#334348';ctx.fillText(name,...p);
  }
  if(selection?.kind==='place'){
    const p=camera.project(selection.point,w,h);ctx.beginPath();ctx.arc(...p,7,0,Math.PI*2);
    ctx.fillStyle='#f5f6f2';ctx.fill();ctx.lineWidth=2;ctx.strokeStyle='#286e85';ctx.stroke();
    ctx.beginPath();ctx.arc(...p,2,0,Math.PI*2);ctx.fillStyle='#286e85';ctx.fill();
  }
}
function drawRuler(){
  const [w,h]=rulerSize;if(!w||!h)return;rc.clearRect(0,0,w,h);
  const x=y=>((y-time.year)/time.span+.5)*w,baseline=h*.42;
  const left=x(time.min),right=x(time.max);
  rc.fillStyle='#e5ece8';rc.fillRect(Math.max(0,left),baseline+25,Math.min(w,right)-Math.max(0,left),18);
  rc.strokeStyle='#aebfc1';rc.lineWidth=1;rc.beginPath();rc.moveTo(0,baseline);rc.lineTo(w,baseline);rc.stroke();
  rc.font=Math.min(18,12*textScale)+'px system-ui';rc.textAlign='center';rc.fillStyle='#435b64';
  for(const tick of time.ticks(w)){
    rc.beginPath();rc.moveTo(tick.x,baseline-7);rc.lineTo(tick.x,baseline+7);rc.stroke();
    rc.fillText(C.yearLabel(tick.year),tick.x,baseline-16);
  }
  for(const s of snapshots){
    const px=x(s.year);if(px<0||px>w)continue;
    rc.beginPath();rc.arc(px,baseline+34,s===current()?5:3.5,0,Math.PI*2);
    rc.fillStyle=s===current()?'#286e85':'#748e96';rc.fill();
  }
  rc.fillStyle='#536773';rc.font='11px system-ui';
  rc.fillText('Mapped '+current().year,w/2,h-5);
}
function render(timestamp){
  frame=0;if(disposed||!active)return;
  if(playing){
    if(lastFrame)time.setYear(time.year+Math.min(.05,(timestamp-lastFrame)/1000)*time.span/12);
    lastFrame=timestamp;if(time.year>=time.max)setPlaying(false);
    dirty=true;update();
  }
  if(dirty){dirty=false;drawMap();drawRuler();}
  if(playing&&!frame)frame=requestAnimationFrame(render);
}
function position(e,canvas){const r=canvas.getBoundingClientRect();return [e.clientX-r.left,e.clientY-r.top];}
function midpoint(points){return [(points[0][0]+points[1][0])/2,(points[0][1]+points[1][1])/2];}
function distance(points){return Math.hypot(points[1][0]-points[0][0],points[1][1]-points[0][1]);}
function bindGestures(canvas,pointers,isTime){
  function baseline(){
    const pts=[...pointers.values()];const g={points:pts.map(p=>p.slice()),moved:false};
    if(isTime)timeGesture=g;else mapGesture=g;
  }
  listen(canvas,'pointerdown',e=>{
    if(e.pointerType==='mouse'&&e.button!==0)return;
    stop();canvas.setPointerCapture(e.pointerId);pointers.set(e.pointerId,position(e,canvas));baseline();
  });
  listen(canvas,'pointermove',e=>{
    if(!pointers.has(e.pointerId))return;
    const previous=[...pointers.values()];pointers.set(e.pointerId,position(e,canvas));const next=[...pointers.values()];
    const g=isTime?timeGesture:mapGesture;
    if(Math.hypot(next[0][0]-g.points[0][0],next[0][1]-g.points[0][1])>5||next.length>1)g.moved=true;
    if(next.length===1){
      const dx=next[0][0]-previous[0][0],dy=next[0][1]-previous[0][1];
      if(isTime)time.drag(dx,rulerSize[0]);else camera.pan(dx,dy);
    }else if(next.length===2&&previous.length===2){
      const a=midpoint(previous),b=midpoint(next),oldDistance=distance(previous);
      const factor=oldDistance>8?distance(next)/oldDistance:1;
      if(isTime){time.zoom(factor,a[0],rulerSize[0]);time.drag(b[0]-a[0],rulerSize[0]);}
      else{camera.zoom(factor,...a,...mapSize);camera.pan(b[0]-a[0],b[1]-a[1]);}
    }
    update();
  });
  function up(e,cancelled){
    if(!pointers.has(e.pointerId))return;
    const g=isTime?timeGesture:mapGesture,p=position(e,canvas),tap=pointers.size===1&&!g.moved&&!cancelled;
    pointers.delete(e.pointerId);if(canvas.hasPointerCapture(e.pointerId))canvas.releasePointerCapture(e.pointerId);
    if(tap){
      if(isTime)time.setYear(time.at(p[0],rulerSize[0]));
      else{
        const point=camera.invert(p,...mapSize);
        if(selection?.kind==='place')selectPlace(point);
        else{const f=current().features.slice().reverse().find(f=>C.contains(f.geometry,point));
          if(f)selectEntity(C.subjectId(f),C.featureName(f));
        }
      }
    }
    if(pointers.size){baseline();const remaining=isTime?timeGesture:mapGesture;remaining.moved=true;}
    update();
  }
  listen(canvas,'pointerup',e=>up(e,false));listen(canvas,'pointercancel',e=>up(e,true));
  listen(canvas,'lostpointercapture',e=>{if(pointers.has(e.pointerId)){pointers.delete(e.pointerId);if(pointers.size)baseline();}});
  listen(canvas,'wheel',e=>{
    e.preventDefault();stop();const p=position(e,canvas);
    if(isTime){if(e.ctrlKey||e.metaKey)time.zoom(Math.exp(-e.deltaY*.005),p[0],rulerSize[0]);else time.drag(-e.deltaY-e.deltaX,rulerSize[0]);}
    else camera.zoom(Math.exp(-e.deltaY*.002),...p,...mapSize);
    update();
  },{passive:false});
}
bindGestures(map,mapPointers,false);bindGestures(ruler,timePointers,true);
listen($('date-toggle'),'click',()=>expand($('time-panel').classList.contains('collapsed')));
listen($('play-toggle'),'click',()=>{if(reduceMotion){announce('Automatic playback is off with Reduce Motion. Use the date slider or mapped-date buttons.');return;}if(time.year>=time.max)time.setYear(time.min);setPlaying(!playing);});
listen($('year-slider'),'input',e=>{stop();time.setYear(Number(e.target.value));update();});
listen($('zoom-in'),'click',()=>{stop();time.zoom(2,.5,1);update();});
listen($('zoom-out'),'click',()=>{stop();time.zoom(.5,.5,1);update();});
for(const [id,factor]of [['map-zoom-in',1.5],['map-zoom-out',1/1.5]])if($(id))listen($(id),'click',()=>{camera.zoom(factor,mapSize[0]/2,mapSize[1]/2,...mapSize);mark();});
listen($('previous-snapshot'),'click',()=>{stop();time.setYear(dates.filter(y=>y<time.year-.01).at(-1)??time.min);update();});
listen($('next-snapshot'),'click',()=>{stop();time.setYear(dates.find(y=>y>time.year+.01)??time.max);update();});
document.querySelectorAll('[data-subject]').forEach(b=>listen(b,'click',()=>{
  const id=b.dataset.subject;if(id==='place')selectPlace();
  else selectEntity(id,id==='poland'?'Poland':'Ottoman Empire',{focus:true});
}));
listen($('clear-selection'),'click',()=>{stop();selection=null;update();});
listen(document,'keydown',e=>{
  if(e.target===map&&e.key.startsWith('Arrow')){e.preventDefault();const dx=e.key==='ArrowLeft'?50:e.key==='ArrowRight'?-50:0,dy=e.key==='ArrowUp'?50:e.key==='ArrowDown'?-50:0;camera.pan(dx,dy);mark();return;}
  if($('sources-dialog').open||['INPUT','BUTTON','SUMMARY','A'].includes(e.target.tagName))return;
  if(e.key==='ArrowLeft'||e.key==='ArrowRight'){e.preventDefault();stop();time.setYear(time.year+(e.key==='ArrowLeft'?-1:1)*(e.shiftKey?10:1));update();}
  if(e.key==='Escape'){stop();selection=null;update();}
});
const sourceRepo='https://github.com/aourednik/historical-basemaps';
$('sources-content').innerHTML='<p>Explore how the states drawn on a set of historical maps differ. The date is continuous; the available maps are not. The nearest available snapshot is shown and always labeled. Borders switch between those maps; they are not reconstructed for the intervening years.</p>'+
  '<p><strong>Available map years:</strong> '+dates.join(', ')+'. Some years only cover the Ottoman Empire. A missing shape means this extract has no matching state, not that a people or identity disappeared.</p>'+
  '<p><strong>Historical geometry:</strong> A. Ourednik and contributors, Historical Basemaps (source URL: '+sourceRepo+'/tree/'+D.source.commit+'), GPL-3.0. Regional extract, generalized for this prototype. The source is a work in progress; boundaries are coarse, sometimes disputed, and not independently verified here. This is an interaction study, not an authoritative historical atlas.</p>'+
  '<p><strong>Physical map:</strong> Natural Earth, public domain. Present-day land is an orientation reference, not reconstructed historical coastlines.</p>'+
  '<p>Poland and the Ottoman Empire are independent subjects. “This place” follows a geographic point while the states around it change. Selecting an entity does not imply unbroken national identity.</p>'+
  '<p>Drag the map to pan and pinch to zoom. Drag the timeline to move time; pinch it to change scale. The date slider, mapped-date buttons and zoom buttons provide alternatives. Automatic playback pauses when you leave or open this panel; Reduce Motion keeps it off.</p>'+
  '<p>Exact source revision, transformations and licenses are bundled in SOURCES.md and the data license files. No accounts, analytics, network requests or saved browsing history.</p>';
listen($('sources-open'),'click',()=>{stop();$('sources-dialog').showModal();});
listen($('sources-close'),'click',()=>$('sources-dialog').close());
listen($('close-demo'),'click',()=>{
  stop();
  if(window.terra){window.terra.close();return;}
  announce('Standalone preview. Close this tab to return.');
});
function settings(s={}){
  if(typeof s.reduceMotion==='boolean')reduceMotion=s.reduceMotion;
  if(typeof s.textSize==='number')textScale=C.clamp(s.textSize,1,3.2);
  document.documentElement.style.setProperty('--text-scale',textScale);
  document.body.classList.toggle('reduce-motion',reduceMotion);
  document.body.classList.toggle('large-text',textScale>1.6);
  if(typeof s.active==='boolean')active=s.active;
  if(!active||reduceMotion||suspended())stop();
  if(!active&&frame){cancelAnimationFrame(frame);frame=0;}
  if(active)mark();
}
function suspended(){return document.hidden||$('sources-dialog').open;}
function dispose(){
  if(disposed)return;
  disposed=true;playing=false;for(const off of hostUnsubscribe)off();abort.abort();resize.disconnect();clearTimeout(announcementTimer);
  if(frame)cancelAnimationFrame(frame);frame=0;mapPointers.clear();timePointers.clear();
}
window.timelineSettings=settings;window.timelineDispose=dispose;
listen(document,'visibilitychange',()=>{if(document.hidden)stop();else mark();});
listen(window,'pagehide',dispose);
const motion=matchMedia('(prefers-reduced-motion: reduce)');
listen(motion,'change',e=>settings({reduceMotion:e.matches}));
const resize=new ResizeObserver(()=>{
  sizeCanvas(map,mapSize);sizeCanvas(ruler,rulerSize);
  if(initialFit&&mapSize[0]>0&&mapSize[1]>0){camera.fit([-8,36,40,62],mapSize[0],mapSize[1]*.7);initialFit=false;}
  mark();
});resize.observe($('map-view'));resize.observe(ruler);
if(window.terra){
  $('close-demo').style.visibility='hidden';
  hostUnsubscribe.push(terra.on('settings',s=>settings({...s,textSize:s.textScale})));
  hostUnsubscribe.push(terra.on('close',dispose));
}else settings({reduceMotion});
update();
// Read-only inspection surface for automated interaction checks.
window.timeAtlasState=()=>({year:time.year,span:time.span,snapshot:current().year,playing,active,disposed,
  selection:selection?JSON.parse(JSON.stringify(selection)):null,camera:{lon:camera.lon,lat:camera.lat,scale:camera.scale}});
})();

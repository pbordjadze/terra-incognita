/* One clock; selection changes its viewing scale, never its date. */
(function(){'use strict';
const C=window.TimeAtlasCore,S=window.ShipMath,D=window.SHIP_DATA,$=id=>document.getElementById(id);
const observations=S.prepare(D.observations,D.land.features,C.contains),initialDay=observations[0].day+2;
const time=new C.TimeCamera({min:S.day('1800-01-01'),max:S.day('1900-01-01'),year:initialDay,span:30,minSpan:2});
class OceanCamera extends C.MapCamera{
 pan(dx,dy){const lon=this.lon-dx/this.scale*180/Math.PI;super.pan(dx,dy);this.lon=lon;}
}
const camera=new OceanCamera(-30,28,650),map=$('map'),ruler=$('ruler'),ctx=map.getContext('2d'),rc=ruler.getContext('2d');
const abort=new AbortController(),listen=(el,type,fn,opt={})=>el.addEventListener(type,fn,{...opt,signal:abort.signal});
const mapSize=[0,0],rulerSize=[0,0],mapPointers=new Map(),timePointers=new Map(),hostUnsubscribe=[];
let mapGesture=null,timeGesture=null,selection=null,follow=false,playing=false,active=true,disposed=false,frame=0,lastFrame=0,dirty=true,textScale=1,initialFit=true,announcementTimer=0;
let reduceMotion=matchMedia('(prefers-reduced-motion: reduce)').matches,pulses=[];
const current=()=>S.position(observations,time.year);
function mark(){dirty=true;if(!frame&&!disposed&&active)frame=requestAnimationFrame(render);}
function announce(text){clearTimeout(announcementTimer);announcementTimer=setTimeout(()=>{$('announcer').textContent=text;},160);}
function stop(){if(playing)setPlaying(false);pulses=[];}
function setPlaying(on){
 playing=!!on&&active&&!disposed&&!reduceMotion&&!document.hidden&&!$('sources-dialog').open;
 $('play-toggle').setAttribute('aria-pressed',String(playing));
 $('play-toggle').setAttribute('aria-label',playing?'Pause time':'Play through time');
 $('play-toggle').firstElementChild.textContent=playing?'Ⅱ':'▶';lastFrame=0;mark();
}
function expand(on=true){$('time-panel').classList.toggle('collapsed',!on);$('date-toggle').setAttribute('aria-expanded',String(on));mark();}
function select(kind){
 selection=kind;if(kind)time.span=C.clamp(S.focusSpan(kind),time.minSpan,time.max-time.min);
 expand();update();announce(kind==='empire'?'British Empire selected. The date is unchanged; time now spans a century.':'Ship selected. The date is unchanged; time now spans thirty days.');
}
function update(){
 const pos=current(),d=S.label(time.year),years=time.span/365.25;
 let title='HMS Challenger',detail='An ocean survey, 1872–76. Touch the ship; draw time back and forth.';
 if(selection==='ship'){title='HMS Challenger';detail='Ship scale · dated sampling stations across the ocean.';}
 if(selection==='empire'){title='The voyage in its century';detail='Empire scale · a century around one British expedition. Same ship, same date.';}
 $('selection-title').textContent=title;$('selection-detail').textContent=detail;
 $('select-ship').setAttribute('aria-pressed',String(selection==='ship'));
 $('select-empire').setAttribute('aria-pressed',String(selection==='empire'));
 $('clear-selection').hidden=!selection;
 $('current-date').textContent=d;
 const slider=$('year-slider');slider.min=time.min;slider.max=time.max;slider.step=.01;slider.value=time.year;slider.setAttribute('aria-valuetext',d);
 $('range-start').textContent='1800';$('range-end').textContent='1900';
 const speed=time.span/24;
 $('rate-label').textContent=(years>=2?(years.toFixed(years>=10?0:1)+' years across'):(Math.round(time.span)+' days across'))+' · '+(speed>=365.25?(speed/365.25).toFixed(1)+' years':speed.toFixed(speed>=10?0:1)+' days')+' per second';
 let status;
 if(pos.kind==='recorded')status='Recorded station '+pos.station+' · '+S.label(pos.day)+(Number.isFinite(pos.depth)?' · '+pos.depth.toLocaleString('en-GB')+' m sampling depth':'');
 else if(pos.kind==='interpolated')status='Illustrated transit · between '+pos.previous+' and '+pos.next;
 else if(pos.kind==='gap')status='No included positions between '+pos.previousDate+' and '+pos.nextDate;
 else status='Outside the included observations · 1873–76';
 $('position-status').textContent=status;$('position-status').dataset.kind=pos.kind;
 $('return-voyage').hidden=pos.kind!=='outside'&&pos.kind!=='gap';
 $('follow-ship').setAttribute('aria-pressed',String(follow));
 $('follow-ship').textContent=follow?'Following ship':'Follow ship';
 $('map-caption').textContent='Dots are selected stations. Physical land is a modern reference; political borders are not shown.';
 $('time-context').textContent='Drag time · Pinch its scale · Dots mark included observations';
 $('previous-snapshot').disabled=!observations.some(o=>o.day<time.year-.01);
 $('next-snapshot').disabled=!observations.some(o=>o.day>time.year+.01);
 $('zoom-in').disabled=time.span<=time.minSpan+.001;$('zoom-out').disabled=time.span>=time.max-time.min-.001;
 map.setAttribute('aria-label',title+'. '+d+'. '+status+'. Arrow keys pan the map.');
 if(follow&&Number.isFinite(pos.lon)){let lon=pos.lon;while(lon-camera.lon>180)lon-=360;while(lon-camera.lon< -180)lon+=360;camera.lon=lon;camera.lat=pos.lat;}
 mark();
}
function sizeCanvas(canvas,size){
 const r=canvas.getBoundingClientRect(),dpr=Math.min(2,devicePixelRatio||1);size[0]=r.width;size[1]=r.height;
 canvas.width=Math.round(r.width*dpr);canvas.height=Math.round(r.height*dpr);canvas.getContext('2d').setTransform(dpr,0,0,dpr,0,0);
}
function project(point){let lon=point.lon;while(lon-camera.lon>180)lon-=360;while(lon-camera.lon< -180)lon+=360;return camera.project([lon,point.lat],...mapSize);}
function drawLand(){
 const world=Math.round(camera.lon/360);
 ctx.fillStyle='#f6e1b9';ctx.strokeStyle='#91a9a7';ctx.lineWidth=.7;
 for(let k=world-1;k<=world+1;k++)for(const f of D.land.features){
  ctx.beginPath();for(const poly of C.rings(f.geometry))for(const ring of poly){
   ring.forEach(([lon,lat],i)=>{const p=camera.project([lon+k*360,lat],...mapSize);i?ctx.lineTo(...p):ctx.moveTo(...p);});ctx.closePath();
  }ctx.fill('evenodd');ctx.stroke();
 }
}
function drawMap(now){
 const [w,h]=mapSize;if(!w||!h)return;ctx.fillStyle='#c6ecff';ctx.fillRect(0,0,w,h);drawLand();
 ctx.save();ctx.textAlign='center';ctx.font='italic '+Math.min(20,16*textScale)+'px Georgia';ctx.lineWidth=4;
 for(const label of [{lon:-36,lat:35,name:'Atlantic Ocean'},{lon:-140,lat:10,name:'Pacific Ocean'},{lon:80,lat:-20,name:'Indian Ocean'}]){const p=project(label);if(p[0]>75&&p[0]<w-75&&p[1]>140&&p[1]<h-240){ctx.strokeStyle='#c6ecff';ctx.strokeText(label.name,...p);ctx.fillStyle='#276987';ctx.fillText(label.name,...p);}}
 ctx.restore();
 // Only short ocean-only schematic links; no lines across missing months or years.
 ctx.strokeStyle='#668c9a';ctx.lineWidth=1.2;ctx.setLineDash([3,6]);
 for(let i=0;i<observations.length-1;i++){
  const a=observations[i],b=observations[i+1];if(!a.connect)continue;let last=null;
  ctx.beginPath();for(let n=0;n<=24;n++){const p=project(S.interpolate(a,b,n/24));if(last&&Math.abs(p[0]-last[0])<w*.8)ctx.lineTo(...p);else ctx.moveTo(...p);last=p;}ctx.stroke();
 }ctx.setLineDash([]);
 for(const o of observations){
  const p=project(o);if(p[0]<-10||p[0]>w+10||p[1]<-10||p[1]>h+10)continue;
  ctx.beginPath();ctx.arc(...p,3.5,0,Math.PI*2);ctx.fillStyle=o.day<=time.year?'#447286':'#d2e3e6';ctx.fill();ctx.strokeStyle='#447286';ctx.lineWidth=1;ctx.stroke();
 }
 for(const pulse of pulses){
  const age=(now-pulse.at)/700;if(age<0||age>1)continue;const p=project(pulse);
  ctx.beginPath();ctx.arc(...p,5+age*20,0,Math.PI*2);ctx.strokeStyle='rgba(40,110,133,'+(1-age)+')';ctx.lineWidth=2;ctx.stroke();
 }
 const pos=current();
 if(Number.isFinite(pos.lon)){
  const p=project(pos);let angle=0;
  if(pos.next){const b=observations.find(o=>o.station===pos.next);if(b){const q=project(b);angle=Math.atan2(q[1]-p[1],q[0]-p[0])+Math.PI/2;}}
  ctx.save();ctx.translate(...p);
  ctx.beginPath();ctx.arc(0,0,25,0,Math.PI*2);ctx.fillStyle='#f5f6f2cc';ctx.fill();ctx.strokeStyle='#286e85';ctx.lineWidth=1;ctx.setLineDash(pos.kind==='interpolated'?[3,4]:[]);ctx.stroke();ctx.setLineDash([]);
  ctx.rotate(angle);ctx.fillStyle='#294e60';ctx.strokeStyle='#f5f6f2';ctx.lineWidth=1;
  ctx.beginPath();ctx.moveTo(0,-19);ctx.bezierCurveTo(12,-10,11,12,0,19);ctx.bezierCurveTo(-11,12,-12,-10,0,-19);ctx.fill();ctx.stroke();
  ctx.strokeStyle='#e8dbc0';ctx.lineWidth=2;ctx.beginPath();ctx.moveTo(0,-12);ctx.lineTo(0,12);for(const y of [-9,0,9]){ctx.moveTo(-12,y);ctx.lineTo(12,y);}ctx.stroke();ctx.restore();
 }
}
function drawRuler(){
 const [w,h]=rulerSize;if(!w||!h)return;rc.clearRect(0,0,w,h);
 const x=d=>((d-time.year)/time.span+.5)*w,base=h*.42;
 rc.fillStyle='#d5e4e5';rc.fillRect(Math.max(0,x(observations[0].day)),base+24,Math.max(0,Math.min(w,x(observations.at(-1).day+1))-Math.max(0,x(observations[0].day))),16);
 rc.strokeStyle='#aebfc1';rc.beginPath();rc.moveTo(0,base);rc.lineTo(w,base);rc.stroke();
 rc.textAlign='center';rc.fillStyle='#435b64';rc.font=Math.min(18,12*textScale)+'px system-ui';
 let ticks=time.ticks(w);
 if(time.span>1500){
  const y1=S.date(time.at(0,w)).getUTCFullYear(),y2=S.date(time.at(w,w)).getUTCFullYear(),target=(y2-y1)/Math.max(2,w/90);
  const step=[1,2,5,10,20,50,100].find(n=>n>=target)||100;ticks=[];
  for(let y=Math.ceil(y1/step)*step;y<=y2;y+=step){const d=S.day(y+'-01-01');if(d>=time.min&&d<=time.max)ticks.push({year:d,x:x(d)});}
 }
 for(const t of ticks){rc.beginPath();rc.moveTo(t.x,base-6);rc.lineTo(t.x,base+6);rc.stroke();const dt=S.date(t.year);rc.fillText(time.span>1500?String(dt.getUTCFullYear()):dt.toLocaleDateString('en-GB',{day:'numeric',month:'short',timeZone:'UTC'}),t.x,base-16);}
 for(const o of observations){const px=x(o.day);if(px<0||px>w)continue;rc.beginPath();rc.arc(px,base+32,3.5,0,Math.PI*2);rc.fillStyle='#447286';rc.fill();}
 rc.fillStyle='#536773';rc.font='11px system-ui';rc.fillText(time.span>1500?'Included observations · 1873–76':'Selected station dates',w/2,h-5);
}
function render(now){
 frame=0;if(disposed||!active)return;
 if(playing){
  const before=time.year;if(lastFrame)time.setYear(time.year+Math.min(.05,(now-lastFrame)/1000)*time.span/24);
  if(!reduceMotion)for(const o of observations)if(o.day>before&&o.day<=time.year)pulses.push({...o,at:now});
  pulses=pulses.filter(p=>now-p.at<700);lastFrame=now;update();if(time.year>=time.max)setPlaying(false);
 }
 if(dirty){dirty=false;drawMap(now);drawRuler();}
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
    stop();if(!isTime)follow=false;canvas.setPointerCapture(e.pointerId);pointers.set(e.pointerId,position(e,canvas));baseline();
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
        const pos=current(),pixel=Number.isFinite(pos.lon)?project(pos):null;
        if(pixel&&Math.hypot(pixel[0]-p[0],pixel[1]-p[1])<=30)select('ship');
        else{
          const hit=observations.map(o=>({o,p:project(o)})).find(q=>Math.hypot(q.p[0]-p[0],q.p[1]-p[1])<14);
          if(hit){time.setYear(hit.o.day);select('ship');}
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
    else {follow=false;camera.zoom(Math.exp(-e.deltaY*.002),...p,...mapSize);}
    update();
  },{passive:false});
}

bindGestures(map,mapPointers,false);bindGestures(ruler,timePointers,true);
listen($('select-ship'),'click',()=>select('ship'));listen($('select-empire'),'click',()=>select('empire'));
listen($('clear-selection'),'click',()=>{selection=null;update();});
listen($('follow-ship'),'click',()=>{follow=!follow;update();});
listen($('return-voyage'),'click',()=>{stop();time.setYear(initialDay);update();announce('Returned to the Atlantic passage. Time scale is unchanged.');});
listen($('date-toggle'),'click',()=>expand($('time-panel').classList.contains('collapsed')));
listen($('play-toggle'),'click',()=>{if(reduceMotion){announce('Automatic playback is off with Reduce Motion. Scrub or use recorded-date controls.');return;}setPlaying(!playing);});
listen($('year-slider'),'input',e=>{stop();time.setYear(Number(e.target.value));update();});
listen($('zoom-in'),'click',()=>{stop();time.zoom(2,.5,1);update();});listen($('zoom-out'),'click',()=>{stop();time.zoom(.5,.5,1);update();});
for(const [id,factor]of [['map-zoom-in',1.5],['map-zoom-out',1/1.5]])listen($(id),'click',()=>{stop();follow=false;camera.zoom(factor,mapSize[0]/2,mapSize[1]/2,...mapSize);update();});
listen($('previous-snapshot'),'click',()=>{stop();time.setYear(observations.filter(o=>o.day<time.year-.01).at(-1)?.day??observations[0].day);update();});
listen($('next-snapshot'),'click',()=>{stop();time.setYear(observations.find(o=>o.day>time.year+.01)?.day??observations.at(-1).day);update();});
listen(document,'keydown',e=>{
 if($('sources-dialog').open||['INPUT','BUTTON','SUMMARY','A'].includes(e.target.tagName))return;
 if(e.target===map&&e.key.startsWith('Arrow')){e.preventDefault();stop();follow=false;camera.pan(e.key==='ArrowLeft'?50:e.key==='ArrowRight'?-50:0,e.key==='ArrowUp'?50:e.key==='ArrowDown'?-50:0);update();return;}
 if(e.key==='ArrowLeft'||e.key==='ArrowRight'){e.preventDefault();stop();time.setYear(time.year+(e.key==='ArrowLeft'?-1:1)*(e.shiftKey?30:1));update();}
});
const intro=document.createElement('p');intro.textContent='HMS Challenger was an oceanographic expedition (1872–1876). This map includes selected sampling stations, not a continuous ship log. Select the ship for a thirty-day window, or the British Empire for a century. Selection changes scale without changing the world date.';$('sources-content').append(intro);
for(const source of Object.values(D.sources)){
 const p=document.createElement('p');p.textContent=source.authors+' ('+source.year+'). '+source.title+'. '+source.license+'. '+source.url;$('sources-content').append(p);
}
for(const text of [...D.limitations,'Dashed links and moving ship positions between short, ocean-only station pairs are illustrative great-circle transits. They are not logged courses or measured speeds. Longer gaps and connections crossing modern land are left blank.',
'The ship symbol is original schematic artwork. Faint station dots are reference observations; brief rings at broad time scales indicate observations crossed by the same clock, not additional ships or repeated voyages.',
'Natural Earth 1:110m physical land is public domain and represents modern geography. Political and imperial borders are not reconstructed. Empire selection is a time-scale frame for this British expedition.',
'Holding the symbol on a dated station is a display convention, not evidence the ship stayed there all day. Dates have day precision. Exact sampling times are not asserted. Play advances one visible span in 24 seconds. All elements derive from the same date; no ship runs on an independent loop.']){
 const p=document.createElement('p');p.textContent=text;$('sources-content').append(p);
}
listen($('sources-open'),'click',()=>{stop();$('sources-dialog').showModal();});listen($('sources-close'),'click',()=>$('sources-dialog').close());
listen($('close-demo'),'click',()=>{stop();if(window.terra)terra.close();else announce('Standalone preview. Close this tab to return.');});
function settings(s={}){
 if(typeof s.reduceMotion==='boolean')reduceMotion=s.reduceMotion;
 if(typeof s.textScale==='number')textScale=C.clamp(s.textScale,1,3.2);
 if(typeof s.active==='boolean')active=s.active;
 document.documentElement.style.setProperty('--text-scale',textScale);document.body.classList.toggle('large-text',textScale>1.6);document.body.classList.toggle('reduce-motion',reduceMotion);
 if(!active||reduceMotion)stop();if(!active&&frame){cancelAnimationFrame(frame);frame=0;}if(active)mark();
}
function dispose(){if(disposed)return;disposed=true;playing=false;abort.abort();resize.disconnect();hostUnsubscribe.forEach(off=>off());clearTimeout(announcementTimer);if(frame)cancelAnimationFrame(frame);frame=0;pulses=[];mapPointers.clear();timePointers.clear();}
listen(document,'visibilitychange',()=>{if(document.hidden)stop();else mark();});listen(window,'pagehide',dispose);listen(matchMedia('(prefers-reduced-motion: reduce)'),'change',e=>settings({reduceMotion:e.matches}));
const resize=new ResizeObserver(()=>{sizeCanvas(map,mapSize);sizeCanvas(ruler,rulerSize);if(initialFit&&mapSize[0]&&mapSize[1]){camera.fit([-58,10,0,45],mapSize[0],mapSize[1]*.65);initialFit=false;}mark();});resize.observe($('map-view'));resize.observe(ruler);
if(window.terra){$('close-demo').style.visibility='hidden';hostUnsubscribe.push(terra.on('settings',settings));hostUnsubscribe.push(terra.on('close',dispose));}
else settings({reduceMotion});
update();
window.shipAtlasState=()=>({day:time.year,span:time.span,selection,playing,active,disposed,follow,camera:{lon:camera.lon,lat:camera.lat,scale:camera.scale},position:current(),observations:observations.map(o=>({day:o.day,lon:o.lon,lat:o.lat,station:o.station})),initialDay});
})();

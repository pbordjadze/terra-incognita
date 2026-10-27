'use strict';
(() => {
  const $ = id => document.getElementById(id);
  const canvas = $('map'), ctx = canvas.getContext('2d', {alpha:false});
  const chart = $('chart'), data = window.LeafChart, notes = window.LeafNotes;
  const clamp = (x,a,b) => Math.max(a,Math.min(b,x));
  const mix = (a,b,t) => a+(b-a)*t;
  const tau = Math.PI*2;
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  let manualStill=false, nativeReduce=false;
  let still = reduced.matches, w=1,h=1,dpr=1,ready=false, frame=0, last=0, time=0, dirty=true;
  let light=0, targetLight=0, activeNote=null, previousFocus=null, labelBoxes=[], nativeActive=true, suppressClick=false;
  const cam = {x:0,y:80,s:1}, home = {x:0,y:80,s:1};
  const fingers = new Map();
  let gesture=null, holdTimer=null, lastTap=0, lastTapPoint=null, held=false;
  let lens=null, lensPinned=false;
  let boat={x:-36,y:72,angle:-.22,sail:1,drag:false};
  let wakes=[], menuReturnFocus=null, pendingMenuFocus=null;
  const bounds=data.bounds;
  const fromLL = p => [(p[0]-data.projection.lon0)*data.projection.cos*120,(38-p[1])*120];
  const toScreen = p => [(p[0]-cam.x)*cam.s+w/2,(p[1]-cam.y)*cam.s+h/2];
  const toWorld = p => [(p[0]-w/2)/cam.s+cam.x,(p[1]-h/2)/cam.s+cam.y];
  const local = e => {const r=chart.getBoundingClientRect(); return [e.clientX-r.left,e.clientY-r.top];};
  const unpack = s => Uint8Array.from(atob(s), c=>c.charCodeAt(0));
  const land=unpack(data.raster.land), depths=unpack(data.raster.depth);
  function sample(p, a) {
    const x=Math.floor((p[0]-bounds[0])/bounds[2]*data.raster.width);
    const y=Math.floor((p[1]-bounds[1])/bounds[3]*data.raster.height);
    if(x<0||y<0||x>=data.raster.width||y>=data.raster.height)return 0;
    return a[y*data.raster.width+x];
  }
  const isLand = p => sample(p,land)>127;
  function path(points,close=false) {const p=new Path2D(); points.forEach((v,i)=>i?p.lineTo(...v):p.moveTo(...v));if(close)p.closePath();return p;}
  const coast = new Path2D();
  data.polygons.forEach(poly=>poly.forEach(r=>coast.addPath(path(r,true))));
  const relief=data.relief.map(([z,p])=>[z,path(p)]), soundings=data.soundings.map(([z,p])=>[z,path(p)]);
  const hatch=new Path2D();data.hachures.forEach(([x,y,dx,dy])=>{hatch.moveTo(x,y);hatch.lineTo(x+dx,y+dy);});
  const palette=[
    {water:'#dce8e3',paper:'#f4f1e8',land:'#eee5d1',coast:'#4b6a6b',relief:'#b68d78',hatch:'#6d8275',line:'#a9c1b7',ink:'#294652',faint:'#526b65',copper:'#9b4934'},
    {water:'#203b49',paper:'#1e3540',land:'#3e5358',coast:'#b1b8aa',relief:'#9a8884',hatch:'#a0b3a9',line:'#38525e',ink:'#e1dec5',faint:'#b1c3bd',copper:'#f0bda5'}
  ];
  const cacheScale=1.65;
  function makeMap(p) {
    const c=document.createElement('canvas');c.width=Math.ceil(bounds[2]*cacheScale);c.height=Math.ceil(bounds[3]*cacheScale);
    const g=c.getContext('2d');g.scale(cacheScale,cacheScale);g.translate(-bounds[0],-bounds[1]);
    g.fillStyle=p.water;g.fillRect(...bounds);
    // Sparse engraved water. A fixed seed makes the page stable, not a changing noise texture.
    let seed=23;const rand=()=>{seed=(Math.imul(seed,1664525)+1013904223)|0;return(seed>>>0)/4294967296;};
    g.strokeStyle=p.line;g.lineWidth=.5;g.globalAlpha=.6;g.beginPath();
    for(let i=0;i<2400;i++){const x=bounds[0]+rand()*bounds[2],y=bounds[1]+rand()*bounds[3];g.moveTo(x,y);g.bezierCurveTo(x+3,y-.7,x+6,y-.7,x+9+rand()*7,y);}
    g.stroke();g.globalAlpha=1;
    // Very fine coordinate ruling; no navigational precision is implied by the illustration.
    g.strokeStyle=p.line;g.lineWidth=.5;g.setLineDash([1,7]);g.beginPath();
    for(let lon=18;lon<=32;lon+=2){const [x]=fromLL([lon,38]);g.moveTo(x,bounds[1]);g.lineTo(x,bounds[1]+bounds[3]);}
    for(let lat=33;lat<=42;lat+=2){const [,y]=fromLL([24,lat]);g.moveTo(bounds[0],y);g.lineTo(bounds[0]+bounds[2],y);}
    g.stroke();g.setLineDash([]);
    g.strokeStyle=p.line;g.lineWidth=8;g.globalAlpha=.4;g.stroke(coast);g.globalAlpha=1;
    g.fillStyle=p.land;g.fill(coast,'evenodd');
    g.save();g.clip(coast,'evenodd');
    for(const [z,pth] of relief){g.strokeStyle=p.relief;g.lineWidth=z<1000?.65:.85;g.globalAlpha=z<1000?.34:.55;g.stroke(pth);}
    g.strokeStyle=p.hatch;g.lineWidth=.5;g.globalAlpha=.53;g.stroke(hatch);
    g.restore();g.globalAlpha=1;
    g.strokeStyle=p.coast;g.lineWidth=.75;g.stroke(coast);
    return c;
  }
  const maps=palette.map(makeMap);
  const geoLabels=[
    ['GREECE',22.35,40.18,17,'region'],['TÜRKİYE',29.14,39.55,17,'region'],['PELOPONNESE',22.25,37.56,12,'region'],
    ['CRETE',24.7,35.10,20,'region'],['Euboea',23.73,38.64,15,'land'],['Lesbos',26.25,39.16,15,'land'],['Chios',26.09,38.37,14,'land'],
    ['Rhodes',28.0,36.14,15,'land'],['Samos',26.83,37.71,14,'land'],['Naxos',25.47,37.02,14,'land'],['Santorini',25.40,36.40,14,'note'],
    ['Lemnos',25.22,39.93,13,'land'],['Thasos',24.66,40.68,13,'land'],['Samothrace',25.53,40.46,12,'land'],['Skyros',24.57,38.89,12,'land'],
    ['Andros',24.85,37.84,12,'land'],['Ikaria',26.16,37.60,12,'land'],['Kos',27.23,36.81,12,'land'],['Karpathos',27.17,35.57,12,'land'],
    ['Kythira',22.98,36.23,12,'land'],['Milos',24.43,36.71,12,'land'],['Paros',25.16,37.07,12,'land'],['Tinos',25.16,37.60,12,'land'],
    ['Mykonos',25.35,37.44,12,'land'],['Syros',24.90,37.44,12,'land'],['Patmos',26.55,37.32,12,'land'],['Astypalea',26.39,36.57,12,'land'],
    ['Athens',23.73,37.98,12,'city'],['İzmir',27.14,38.42,12,'city'],['Thessaloniki',22.94,40.64,12,'city'],
    ['Sea of Crete',25.0,35.83,17,'sea'],['Saronic Gulf',23.35,37.65,12,'sea'],['Myrtoan Sea',23.73,36.90,14,'sea'],
    ['Cyclades',24.66,37.14,19,'archipelago'],['Dodecanese',27.28,37.23,16,'archipelago']
  ].map(([name,lon,lat,size,kind])=>({name,point:fromLL([lon,lat]),size,kind}));
  const noteButtons=notes.map(n=>{
    n.world=fromLL(n.point); const b=document.createElement('button');b.setAttribute('aria-label',`${n.name}: ${n.title}`);b.dataset.note=n.id;
    b.addEventListener('click',e=>{if(e.detail===0||(!suppressClick&&!held))openNote(n.id);e.stopPropagation();});$('places').append(b);return b;
  });
  for(const n of notes){const b=document.createElement('button');const name=document.createElement('span');name.textContent=n.name;const small=document.createElement('small');small.textContent=n.title;b.append(name,small);b.onclick=()=>{$('leaf-menu').close();openNote(n.id);};$('note-index').append(b);}

  function resize() {
    const r=chart.getBoundingClientRect();if(!r.width||!r.height)return;
    const old={w,h,s:home.s};w=r.width;h=r.height;dpr=Math.min(devicePixelRatio||1,2);
    canvas.width=Math.round(w*dpr);canvas.height=Math.round(h*dpr);
    home.s=w<700?Math.min(w/620,h/830):Math.min(w/1030,h/880);
    if(!ready)Object.assign(cam,home);else cam.s*=home.s/old.s;
    constrain();dirty=true;
  }
  function constrain(){cam.s=clamp(cam.s,home.s*.75,home.s*6);cam.x=clamp(cam.x,bounds[0]+w/cam.s*.15,bounds[0]+bounds[2]-w/cam.s*.15);cam.y=clamp(cam.y,bounds[1]+h/cam.s*.15,bounds[1]+bounds[3]-h/cam.s*.15);}
  function zoom(f,anchor=[w/2,h/2]){const p=toWorld(anchor);cam.s*=f;constrain();cam.x=p[0]-(anchor[0]-w/2)/cam.s;cam.y=p[1]-(anchor[1]-h/2)/cam.s;constrain();dirty=true;}
  function worldTransform(g){g.translate(w/2,h/2);g.scale(cam.s,cam.s);g.translate(-cam.x,-cam.y);}
  function drawMap(g,night){g.save();worldTransform(g);g.drawImage(maps[night],...bounds);g.restore();}
  function label(g,t,x,y,size,options={}){
    g.font=`${options.italic?'italic ':''}${size}px Libertinus, Georgia, serif`;g.textAlign='center';g.textBaseline='middle';
    g.lineJoin='round';g.lineWidth=4;g.strokeStyle=options.ground||palette[light>.6?1:0].water;g.strokeText(t,x,y);g.fillStyle=options.color||ink();g.fillText(t,x,y);
  }
  function ink(){return palette[light>.6?1:0].ink;}
  function labels(g){
    labelBoxes=[];const mobile=w<700;
    const blocked=[];
    if(cam.s<home.s*1.5){const p=toScreen(fromLL([24.65,38.7]));blocked.push({x:p[0]-86,y:p[1]-50,w:175,h:100});}
    if($('note').open){const b=$('note').getBoundingClientRect(),r=chart.getBoundingClientRect();blocked.push({x:b.x-r.x-8,y:b.y-r.y-8,w:b.width+16,h:b.height+16});}
    for(const l of geoLabels){
      const [x0,y0]=toScreen(l.point);const size=l.size*(mobile?1:1.12)*textScale;
      const island=l.kind==='land'||l.kind==='note';const x=x0,y=y0+(island?12:0);
      g.font=`${l.kind==='sea'||l.kind==='archipelago'?'italic ':''}${size}px Libertinus`;
      const width=g.measureText(l.name).width+12,height=size+9;
      const b={x:x-width/2,y:y-height/2,w:width,h:height,name:l.name};
      if(x<30||x>w-30||b.x<17||b.x+b.w>w-17||b.y<26||b.y+b.h>h-42)continue;
      if((l.kind==='archipelago'||l.kind==='sea')&&cam.s>home.s*2)continue;
      if(labelBoxes.concat(blocked).some(a=>a.x<b.x+b.w&&a.x+a.w>b.x&&a.y<b.y+b.h&&a.y+a.h>b.y))continue;
      labelBoxes.push(b);
      if(l.kind==='city'){g.fillStyle=ink();g.beginPath();g.arc(x,y0-8,1.7,0,tau);g.fill();}
      label(g,l.name,x,y,size,{italic:l.kind==='sea'||l.kind==='archipelago',color:l.kind==='note'?palette[light>.6?1:0].copper:ink(),ground:island||l.kind==='region'||l.kind==='city'?palette[light>.6?1:0].land:undefined});
      if(l.kind==='note'){g.strokeStyle=palette[light>.6?1:0].copper;g.lineWidth=.6;g.beginPath();g.moveTo(x-width/2+8,y+size*.6);g.lineTo(x+width/2-8,y+size*.6);g.stroke();}
    }
  }
  function drawWind(g,x,y,s,t=0){
    g.save();g.translate(x,y);g.scale(s,s);g.strokeStyle=ink();g.lineWidth=.75;g.lineCap='round';
    for(let i=0;i<4;i++){g.beginPath();g.moveTo(-18+i*9,-23);g.bezierCurveTo(-25+i*9,0,20+i*3,5,8+i*8,28);g.stroke();}
    g.restore();
  }
  function palm(g,x,y,s,t=0){
    g.save();g.translate(x,y);g.scale(s,s);g.lineCap='round';g.strokeStyle=ink();g.lineWidth=1;
    g.beginPath();g.moveTo(-4,25);g.quadraticCurveTo(1,7,0,-9);g.moveTo(1,25);g.quadraticCurveTo(5,10,0,-9);g.stroke();
    for(let side of [-1,1])for(let i=0;i<4;i++){
      let endX=side*(13+i*4),endY=-20+i*10,bend=Math.sin(t*.7)*1.4;
      g.beginPath();g.moveTo(0,-9);g.quadraticCurveTo(endX*.6,-30+i*5,endX+bend,endY);g.stroke();
      for(let j=1;j<7;j++){const u=j/7;const xx=2*(1-u)*u*endX*.6+u*u*endX, yy=(1-u)*(1-u)*-9+2*(1-u)*u*(-30+i*5)+u*u*endY;g.beginPath();g.moveTo(xx,yy);g.lineTo(xx+side*2.5,yy+5);g.stroke();}
    }
    g.beginPath();g.ellipse(-2,27,12,2,0,0,tau);g.stroke();g.restore();
  }
  function whale(g,x,y,s,t=0){
    g.save();g.translate(x,y);g.rotate(-.12);g.scale(s,s);g.fillStyle=ink();g.strokeStyle=ink();g.lineWidth=.65;
    g.beginPath();g.moveTo(-43,-8);g.bezierCurveTo(-48,-23,-28,-24,-11,-18);g.bezierCurveTo(6,-16,19,-7,34,-2);g.quadraticCurveTo(43,-10,50,-9);g.quadraticCurveTo(46,-2,38,2);g.quadraticCurveTo(44,7,50,8);g.quadraticCurveTo(39,13,33,4);g.bezierCurveTo(10,6,-9,11,-30,5);g.quadraticCurveTo(-43,3,-43,-8);g.fill();
    g.strokeStyle=palette[light>.6?1:0].water;g.beginPath();g.moveTo(-38,-2);g.quadraticCurveTo(-25,4,-8,1);g.moveTo(-30,-14);g.quadraticCurveTo(-12,-12,2,-5);g.stroke();
    g.fillStyle=palette[light>.6?1:0].water;g.beginPath();g.arc(-35,-10,1,0,tau);g.fill();g.restore();
  }
  function drawSkiff(g,screen=true){
    const pos=toScreen([boat.x,boat.y]);const s=clamp(cam.s/home.s,.9,1.8)*(w<700?.9:1);
    g.save();g.translate(...pos);g.rotate(boat.angle+Math.sin(time*.6)*.035);g.scale(s,s);
    const bob=Math.sin(time*1.1)*1;g.translate(0,bob);
    g.strokeStyle=ink();g.lineWidth=.8;g.fillStyle=palette[light>.6?1:0].land;
    g.beginPath();g.moveTo(-12,5);g.quadraticCurveTo(0,13,16,4);g.lineTo(-12,5);g.fill();g.stroke();
    g.strokeStyle=palette[light>.6?1:0].copper;g.beginPath();g.moveTo(-4,6);g.lineTo(-4,-25);g.stroke();
    const sail=boat.sail;g.fillStyle=palette[light>.6?1:0].paper;g.strokeStyle=ink();g.beginPath();g.moveTo(-3,-24);g.quadraticCurveTo(6+Math.sin(time)*sail, -10,12*sail,2);g.lineTo(-3,2);g.closePath();g.fill();g.stroke();
    g.fillStyle=palette[light>.6?1:0].copper;g.beginPath();g.moveTo(-6,-19);g.lineTo(-14*sail,2);g.lineTo(-6,2);g.closePath();g.fill();
    g.strokeStyle=palette[light>.6?1:0].line;g.lineWidth=.6;
    for(let i=0;i<3;i++){g.beginPath();g.moveTo(-19,10+i*4);g.quadraticCurveTo(0,13+i*4,20,9+i*4);g.stroke();}
    g.restore();const b=$('skiff');b.style.left=(pos[0]-22)+'px';b.style.top=(pos[1]-22)+'px';b.hidden=pos[0]<22||pos[0]>w-22||pos[1]<22||pos[1]>h-22;return pos;
  }
  function ornaments(g){
    const [wx,wy]=toScreen(notes[0].world);if(wx>-50&&wx<w+50&&wy>-50&&wy<h+50){g.globalAlpha=.7;drawWind(g,wx,wy,.7,time);g.globalAlpha=1;}
    const [px,py]=toScreen(notes[2].world);if(px>-40&&px<w+40&&py>-40&&py<h+40){palm(g,px+5,py-12,.54,time);label(g,'Vai',px+17,py+17,12*textScale,{color:palette[light>.6?1:0].copper});}
    const wp=fromLL([23.15+Math.sin(time*.008)*.45,34.45]);const [dx,dy]=toScreen(wp);
    const phase=(time+4)%95;const alpha=still?.36:phase>29&&phase<63?Math.sin((phase-29)/34*Math.PI)*.46:0;
    if(alpha>0){g.globalAlpha=alpha;whale(g,dx,dy,clamp(cam.s/home.s*.8,.65,1.4),time);g.globalAlpha=1;}
    const poem=toScreen(fromLL([24.65,38.7]));$('poem').style.left=(poem[0]-70)+'px';$('poem').style.top=(poem[1]-35)+'px';$('poem').style.display=cam.s<home.s*1.5?'block':'none';
    noteButtons.forEach((b,i)=>{const p=i===3?toScreen(wp):toScreen(notes[i].world);b.style.left=(p[0]-22)+'px';b.style.top=(p[1]-22)+'px';const show=p[0]>22&&p[0]<w-22&&p[1]>28&&p[1]<h-35;b.hidden=!show;b.style.pointerEvents=(i===3&&alpha<.1)?'none':'auto';});
  }
  // A handful of moving ink strokes, not a flow simulation. They never claim measured weather.
  const streams=Array.from({length:18},(_,i)=>({x:-350+(i*137%850),y:-500+(i*113%950),seed:i*3.13}));
  function life(g,dt){
    g.strokeStyle=ink();g.lineWidth=.65;g.globalAlpha=.23;
    for(const p of streams){
      if(!still){p.y+=dt*4.5;p.x+=Math.sin(time*.12+p.seed)*dt*.7;if(p.y>560)p.y=-520;}
      const q=[p.x+Math.sin(p.y*.013+p.seed)*12,p.y];if(isLand(q))continue;
      const [x,y]=toScreen(q);if(x<0||x>w||y<0||y>h)continue;
      g.beginPath();g.moveTo(x,y);g.bezierCurveTo(x-4,y+4,x+4,y+10,x+1,y+18);g.stroke();
    }g.globalAlpha=1;
    wakes=wakes.filter(r=>time-r.t<3.5);
    for(const r of wakes){const age=time-r.t,p=toScreen(r.p);g.strokeStyle=ink();g.lineWidth=.65;g.globalAlpha=(1-age/3.5)*.22;g.beginPath();g.ellipse(p[0],p[1],7+age*14,3+age*6,-.12,0,tau);g.stroke();}g.globalAlpha=1;
    // The skiff drifts only a little on this page, and does not traverse a purported route.
    if(!still&&!boat.drag){const next=[boat.x+Math.sin(time*.16)*dt*.12,boat.y+dt*.24];if(!isLand(next)&&next[1]<200){boat.x=next[0];boat.y=next[1];}}
    drawSkiff(g);
  }
  function soundingGlass(g){
    if(!lens){$('lens-label').hidden=true;return;}
    const r=w<700?84:108,p=lens;
    g.save();g.beginPath();g.arc(p[0],p[1],r,0,tau);g.clip();g.fillStyle=palette[light>.6?1:0].paper;g.fillRect(p[0]-r,p[1]-r,r*2,r*2);
    g.save();worldTransform(g);g.fillStyle=palette[light>.6?1:0].land;g.fill(coast,'evenodd');g.strokeStyle=ink();
    for(const [z,pth] of soundings){g.globalAlpha=z>1000?.42:.22;g.lineWidth=(z%1000===0?1.0:.6)/cam.s;g.stroke(pth);}
    g.globalAlpha=1;g.lineWidth=.8/cam.s;g.stroke(coast);g.restore();
    const wp=toWorld(p);g.globalAlpha=.8;g.strokeStyle=ink();g.lineWidth=.6;g.beginPath();g.moveTo(p[0]-6,p[1]);g.lineTo(p[0]+6,p[1]);g.moveTo(p[0],p[1]-6);g.lineTo(p[0],p[1]+6);g.stroke();g.globalAlpha=1;g.restore();
    g.strokeStyle=palette[light>.6?1:0].copper;g.lineWidth=1.1;g.beginPath();g.arc(p[0],p[1],r+2,0,tau);g.stroke();g.strokeStyle=ink();g.lineWidth=.6;g.beginPath();g.arc(p[0],p[1],r+6,0,tau);g.stroke();
    const depth=Math.round(sample(wp,depths)*25/100)*100;
    $('depth-reading').textContent=isLand(wp)?'Land · generalised':`~${depth.toLocaleString()} m · generalised`;
    const el=$('lens-label');el.hidden=false;el.style.left=clamp(p[0]-90,12,w-220)+'px';el.style.top=clamp(p[1]+r+14,12,h-50)+'px';$('lens-close').hidden=!lensPinned;
  }
  function render(now){
    frame=requestAnimationFrame(render);
    const dt=Math.min((now-last)/1000,.05)||0;last=now;
    if(document.hidden||!nativeActive||!ready)return;
    if(!still&&!$('leaf-menu').open)time+=dt;
    if(Math.abs(light-targetLight)>.001){light=still?targetLight:mix(light,targetLight,.1);dirty=true;}
    if(still&&!dirty)return;
    // Idle animation is deliberately capped at 30Hz; input draws immediately on its next frame.
    if(!dirty&&now-lastDraw<32)return;lastDraw=now;dirty=false;
    ctx.setTransform(dpr,0,0,dpr,0,0);ctx.fillStyle=palette[light>.6?1:0].water;ctx.fillRect(0,0,w,h);
    drawMap(ctx,0);if(light>.001){ctx.globalAlpha=light;drawMap(ctx,1);ctx.globalAlpha=1;}
    life(ctx,dt);labels(ctx);ornaments(ctx);soundingGlass(ctx);
    const nw=toWorld([20,20]),ne=toWorld([w-20,20]);
    const lat=38-nw[1]/120,lon=24.6+ne[0]/(120*data.projection.cos);
    document.querySelector('.top-left').textContent=lat.toFixed(1)+'° N';document.querySelector('.top-right').textContent=lon.toFixed(1)+'° E';
    const km=[100,50,20,10].find(k=>k/111.2*120*cam.s<Math.min(135,w*.26))||10;const scale=km/111.2*120*cam.s;$('scale').firstElementChild.style.width=scale+'px';$('scale').lastElementChild.textContent=km+' km';
  }
  let lastDraw=0,textScale=1;
  function refreshType(){const px=parseFloat(getComputedStyle(document.body).fontSize)||16;textScale=clamp(px/16,1,1.6);dirty=true;}
  function stopHold(){clearTimeout(holdTimer);holdTimer=null;}
  function wake(p){if(still)return;wakes.push({p,t:time});if(wakes.length>4)wakes.shift();dirty=true;}
  chart.addEventListener('pointerdown', e=>{
    if(e.target.closest('button')&&!e.target.closest('[data-note],#skiff'))return;
    suppressClick=false;
    if(e.pointerType==='mouse'&&e.button!==0)return;
    chart.setPointerCapture(e.pointerId);const p=local(e);fingers.set(e.pointerId,p);stopHold();held=false;
    if(fingers.size===2){const ps=[...fingers.values()];gesture={kind:'pinch',distance:Math.hypot(ps[1][0]-ps[0][0],ps[1][1]-ps[0][1]),anchor:toWorld([(ps[0][0]+ps[1][0])/2,(ps[0][1]+ps[1][1])/2]),s:cam.s,moved:true};lens=null;return;}
    const bp=toScreen([boat.x,boat.y]);const skiff=Math.hypot(p[0]-bp[0],p[1]-bp[1])<28;
    gesture={kind:skiff?'boat':'pan',start:p,last:p,cam:{...cam},moved:false};boat.drag=skiff;
    if(!skiff&&!isLand(toWorld(p))){holdTimer=setTimeout(()=>{if(!gesture?.moved&&fingers.size===1){held=true;gesture.kind='lens';lens=p.slice();lensPinned=false;dirty=true;chime(174,.03);}},430);}
  });
  chart.addEventListener('pointermove',e=>{
    const p=local(e);if(!fingers.has(e.pointerId))return;
    fingers.set(e.pointerId,p);if(!gesture)return;
    if(fingers.size>=2){stopHold();const ps=[...fingers.values()],mid=[(ps[0][0]+ps[1][0])/2,(ps[0][1]+ps[1][1])/2];cam.s=gesture.s*Math.hypot(ps[1][0]-ps[0][0],ps[1][1]-ps[0][1])/gesture.distance;constrain();cam.x=gesture.anchor[0]-(mid[0]-w/2)/cam.s;cam.y=gesture.anchor[1]-(mid[1]-h/2)/cam.s;constrain();dirty=true;return;}
    const dx=p[0]-gesture.start[0],dy=p[1]-gesture.start[1];if(Math.hypot(dx,dy)>6){gesture.moved=true;stopHold();}
    if(gesture.kind==='lens'){lens=[clamp(p[0],18,w-18),clamp(p[1],18,h-18)];dirty=true;return;}
    if(gesture.kind==='boat'){const q=toWorld(p);if(!isLand(q)){boat.x=q[0];boat.y=q[1];boat.angle=clamp((p[0]-gesture.last[0])*.04,-.7,.7);if(Math.hypot(p[0]-gesture.last[0],p[1]-gesture.last[1])>4)wake(q);}dirty=true;}
    else if(gesture.moved){cam.x=gesture.cam.x-dx/cam.s;cam.y=gesture.cam.y-dy/cam.s;constrain();dirty=true;}
    gesture.last=p;
  });
  function pointerEnd(e,cancel=false){
    if(!fingers.has(e.pointerId))return;
    suppressClick=cancel||held||!!gesture?.moved;
    const p=fingers.get(e.pointerId);fingers.delete(e.pointerId);stopHold();boat.drag=false;
    if(fingers.size){const q=[...fingers.values()][0];gesture={kind:'pan',start:q,last:q,cam:{...cam},moved:true};return;}
    if(!cancel&&gesture&&!gesture.moved&&!held){
      if(gesture.kind==='boat'){boat.sail=boat.sail===1?.16:1;chime(261.6,.045);}
      else {const q=toWorld(p);if(!isLand(q)){wake(q);chime(196+Math.abs(q[0])*.1,.025);}if(lastTapPoint&&performance.now()-lastTap<320&&Math.hypot(p[0]-lastTapPoint[0],p[1]-lastTapPoint[1])<28)zoom(1.65,p);lastTap=performance.now();lastTapPoint=p;}
    }
    if(gesture?.kind==='lens'&&!lensPinned)lens=null;
    gesture=null;held=false;dirty=true;
    $('invitation').style.opacity='0';
  }
  chart.addEventListener('pointerup',e=>pointerEnd(e));chart.addEventListener('pointercancel',e=>pointerEnd(e,true));
  chart.addEventListener('lostpointercapture',e=>{if(fingers.has(e.pointerId))pointerEnd(e,true);});
  chart.addEventListener('contextmenu',e=>e.preventDefault());
  chart.addEventListener('wheel',e=>{e.preventDefault();zoom(Math.exp(-e.deltaY*.0015),local(e));},{passive:false});
  $('zoom-in').onclick=()=>zoom(1.35);$('zoom-out').onclick=()=>zoom(1/1.35);
  $('home').onclick=()=>{Object.assign(cam,home);lens=null;dirty=true;};
  $('lens-close').onclick=()=>{lens=null;lensPinned=false;dirty=true;};
  $('light').oninput=e=>{
    targetLight=Number(e.target.value)/100;$('light-name').textContent=targetLight>.62?'Blue hour':targetLight>.22?'Late light':'Daylight';
    const p=palette[targetLight>.6?1:0],r=document.documentElement;
    r.style.setProperty('--paper',p.paper);r.style.setProperty('--panel',targetLight>.6?'#233d49':'#f8f5ec');r.style.setProperty('--ink',p.ink);r.style.setProperty('--faint',p.faint);r.style.setProperty('--rule',targetLight>.6?'#718b8e':'#9eada7');r.style.setProperty('--copper',p.copper);r.style.setProperty('--water',p.water);dirty=true;
  };
  function setStill(value){const system=nativeReduce||reduced.matches;still=value||system;$('motion').disabled=system;$('motion').textContent=system?'Motion reduced by system':still?'Let the page move':'Still the page';$('motion').setAttribute('aria-pressed',String(still));dirty=true;}
  $('motion').onclick=()=>{manualStill=!manualStill;setStill(manualStill);};reduced.addEventListener('change',()=>setStill(manualStill));
  $('glass').onclick=()=>{pendingMenuFocus=$('lens-close');$('leaf-menu').close();lens=[w/2,h/2];lensPinned=true;dirty=true;$('announcement').textContent='Sounding glass open at the centre of the map. Use arrow keys to move it, or drag the water to move the map beneath it.';};
  $('skiff').onclick=e=>{if(e.detail===0){boat.sail=boat.sail===1?.16:1;dirty=true;$('announcement').textContent=boat.sail===1?'The paper sail is raised.':'The paper sail is folded.';chime(261.6,.035);}};
  $('handle-skiff').onclick=()=>{pendingMenuFocus=$('skiff');$('leaf-menu').close();const p=toScreen([boat.x,boat.y]);if(p[0]<22||p[0]>w-22||p[1]<22||p[1]>h-22){cam.x=boat.x;cam.y=boat.y;}$('skiff').hidden=false;dirty=true;$('announcement').textContent='Paper skiff. Use arrow keys to move it. Activate it to fold or raise its sail.';};
  $('retry').onclick=()=>location.reload();
  $('atlas').onclick=()=>{if(window.webkit?.messageHandlers?.leaf)window.webkit.messageHandlers.leaf.postMessage('close');else openMenu();};
  function openMenu(){menuReturnFocus=document.activeElement;$('leaf-menu').showModal();}
  $('menu-button').onclick=openMenu;
  $('leaf-menu').querySelector('.close-menu').onclick=()=>$('leaf-menu').close();
  $('leaf-menu').addEventListener('close',()=>{dirty=true;if(!$('note').open){const target=pendingMenuFocus||menuReturnFocus;pendingMenuFocus=null;requestAnimationFrame(()=>target?.focus());}});
  function openNote(id){
    const n=notes.find(n=>n.id===id);if(!n)return;
    previousFocus=document.activeElement.closest('#leaf-menu')?$('menu-button'):document.activeElement;activeNote=n;$('note-location').textContent=n.location;$('note-title').textContent=n.title;$('note-deck').textContent=n.deck;
    $('note-body').replaceChildren(...n.body.map(text=>{const p=document.createElement('p');p.textContent=text;return p;}));
    const sources=n.sources.flatMap(s=>{const p=document.createElement('p'),c=document.createElement('cite'),url=document.createElement('p');c.textContent=s.title;p.append(c,document.createElement('br'),document.createTextNode(s.text));url.className='source-url';url.textContent=s.url;return [p,url];});
    const limit=document.createElement('p');limit.className='qualification';limit.textContent=n.limit;$('note-sources').replaceChildren(...sources,limit);
    $('note').querySelector('details').open=false;$('note').scrollTop=0;$('note').showModal();drawNote(n);dirty=true;chime(293.66,.025);
  }
  function closeNote(){$('note').close();}
  $('note').querySelector('.close-note').onclick=closeNote;$('note').querySelector('.return-to-water').onclick=closeNote;
  $('note').addEventListener('close',()=>{activeNote=null;dirty=true;previousFocus?.focus();});
  function drawNote(n){
    const g=$('note-drawing').getContext('2d');g.clearRect(0,0,640,250);g.save();g.scale(2,2);g.strokeStyle=ink();g.lineWidth=.7;
    if(n.drawing==='palm')palm(g,160,63,1.7,0);
    if(n.drawing==='whale'){whale(g,155,64,1.65);g.globalAlpha=.5;for(let i=0;i<4;i++){g.beginPath();g.moveTo(38,95+i*6);g.bezierCurveTo(100,65+i*7,210,120+i*4,285,82+i*8);g.stroke();}}
    if(n.drawing==='wind'){for(let i=0;i<6;i++){g.beginPath();g.moveTo(35+i*22,12);g.bezierCurveTo(15+i*27,50,150+i*4,60,110+i*22,110);g.stroke();}g.fillStyle=palette[light>.6?1:0].land;g.beginPath();g.ellipse(130,53,10,20,.55,0,tau);g.fill();g.stroke();g.beginPath();g.ellipse(193,82,7,15,.8,0,tau);g.fill();g.stroke();}
    if(n.drawing==='caldera'){
      const p=n.world;g.translate(156,65);g.scale(5.6,5.6);g.translate(-p[0],-p[1]);g.fillStyle=palette[light>.6?1:0].water;g.fillRect(p[0]-30,p[1]-10,60,20);g.fillStyle=palette[light>.6?1:0].land;g.fill(coast,'evenodd');g.lineWidth=.16;g.stroke(coast);
      // Minor islands omitted by the general atlas dataset: deliberately schematic shapes.
      // Their inside/outside relationship is checked against NASA ISS017-E-5037 and GVP.
      for(const [lon,lat,rx,ry] of [[25.397,36.402,1.55,1.15],[25.380,36.400,.65,.9]]){
        const q=fromLL([lon,lat]);g.beginPath();g.ellipse(q[0],q[1],rx,ry,-.3,0,tau);g.fill();g.stroke();
      }
    }
    g.restore();
    if(n.drawing==='caldera'){g.font='20px Libertinus';g.fillStyle=ink();g.textAlign='center';g.fillText('Caldera · minor islands schematic',320,238);}
  }
  document.addEventListener('keydown',e=>{
    if($('note').open||$('leaf-menu').open||e.target.matches('input,summary')||(e.target.matches('button')&&e.target.id!=='skiff'&&!lensPinned))return;
    if(e.key==='Escape'){lens=null;dirty=true;return;}
    const moves={ArrowLeft:[-1,0],ArrowRight:[1,0],ArrowUp:[0,-1],ArrowDown:[0,1]};
    if(moves[e.key]){e.preventDefault();const [dx,dy]=moves[e.key];if(e.target.id==='skiff'){const q=[boat.x+dx*8/cam.s,boat.y+dy*8/cam.s];if(!isLand(q)){boat.x=q[0];boat.y=q[1];}}else if(lensPinned&&lens){lens[0]=clamp(lens[0]+dx*20,20,w-20);lens[1]=clamp(lens[1]+dy*20,20,h-20);}else{cam.x+=dx*32/cam.s;cam.y+=dy*32/cam.s;constrain();}dirty=true;}
    if(e.key==='+'||e.key==='=')zoom(1.25);if(e.key==='-')zoom(.8);
  });
  let audio=null, soundOn=false;
  function chime(freq,volume){if(!audio||!soundOn)return;const o=audio.createOscillator(),g=audio.createGain();o.type='sine';o.frequency.value=freq;g.gain.setValueAtTime(0,audio.currentTime);g.gain.linearRampToValueAtTime(volume,audio.currentTime+.025);g.gain.exponentialRampToValueAtTime(.0001,audio.currentTime+1.7);o.connect(g);g.connect(audio.destination);o.start();o.stop(audio.currentTime+1.8);}
  $('sound').onclick=async()=>{
    try {
      if(!audio)audio=new (window.AudioContext||window.webkitAudioContext)();
      soundOn=!soundOn;if(soundOn){await audio.resume();chime(196,.035);}else await audio.suspend();
      $('sound').setAttribute('aria-pressed',String(soundOn));$('sound').setAttribute('aria-label',soundOn?'Turn off the imagined soundscape':'Turn on the imagined soundscape');
    }catch(_){soundOn=false;$('sound').setAttribute('aria-pressed','false');$('announcement').textContent='Sound is unavailable. The leaf works silently.';}
  };
  document.addEventListener('visibilitychange',()=>{last=performance.now();if(document.hidden){stopHold();fingers.clear();gesture=null;boat.drag=false;lens=lensPinned?lens:null;audio?.suspend();}else{dirty=true;if(soundOn)audio?.resume();}});
  window.leafDispose=()=>{cancelAnimationFrame(frame);stopHold();audio?.close();};window.addEventListener('pagehide',window.leafDispose);
  // The native host supplies system text-size scaling, scene visibility and Reduce Motion.
  window.leafSettings=({textSize=1,reduceMotion=false,active=true}={})=>{
    document.documentElement.style.fontSize=16*clamp(textSize,1,2)+'px';textScale=clamp(textSize,1,1.6);nativeReduce=reduceMotion;setStill(manualStill);nativeActive=active;if(!active)audio?.suspend();else if(soundOn)audio?.resume();dirty=true;
  };
  // Read-only geometry diagnostics for the local interaction harness.
  window.leafDiagnostics=()=>({ready,cam:{...cam},home:{...home},size:[w,h],labels:labelBoxes.map(b=>({...b})),lens:lens?.slice(),still,light,soundOn,boat:{...boat},note:activeNote?.id||null,time});
  window.leafPoint=(lon,lat)=>toScreen(fromLL([lon,lat]));
  async function start(){
    try{await document.fonts.ready;resize();refreshType();setStill(manualStill);ready=true;dirty=true;$('loading').style.opacity='0';setTimeout(()=>$('loading').hidden=true,850);window.leafReady=true;}
    catch(e){$('loading').hidden=true;$('error').hidden=false;console.error(e);}
  }
  new ResizeObserver(resize).observe(chart);window.addEventListener('resize',refreshType);start();frame=requestAnimationFrame(render);
})();

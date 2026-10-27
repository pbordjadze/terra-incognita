/* Geographic connectivity only. No runoff, velocity, rainfall or flood model. */
(function(root){
'use strict';
const C=Math.cos(20*Math.PI/180), W=52*C, H=73;
function project(lon,lat){return [(lon+84)*C,15-lat];}
function invert(x,y){return [x/C-84,15-y];}
function build(data){
 const edges=data.rivers.map((r,i)=>({i,next:r[0],order:r[1],km:r[2],basin:r[3],inland:r[4],source:r[6],area:r[7],xy:[],cum:[0],up:[]}));
 const grid=new Map(), cell=.6;
 edges.forEach((e,i)=>{
  const raw=data.rivers[i][5];
  for(let j=0;j<raw.length;j+=2)e.xy.push(...project(raw[j],raw[j+1]));
  for(let j=2;j<e.xy.length;j+=2)e.cum.push(e.cum[e.cum.length-1]+Math.hypot(e.xy[j]-e.xy[j-2],e.xy[j+1]-e.xy[j-1]));
  e.length=e.cum[e.cum.length-1];
  if(e.next>=0)edges[e.next].up.push(i);
  const used=new Set();
  for(let j=2;j<e.xy.length;j+=2){
   const x0=Math.floor(Math.min(e.xy[j-2],e.xy[j])/cell),x1=Math.floor(Math.max(e.xy[j-2],e.xy[j])/cell);
   const y0=Math.floor(Math.min(e.xy[j-1],e.xy[j+1])/cell),y1=Math.floor(Math.max(e.xy[j-1],e.xy[j+1])/cell);
   for(let x=x0;x<=x1;x++)for(let y=y0;y<=y1;y++)used.add(x+','+y);
  }
  used.forEach(k=>{if(!grid.has(k))grid.set(k,[]);grid.get(k).push(i);});
 });
 function nearest(x,y,radius,minOrder=1){
  const candidates=new Set();
  for(let i=Math.floor((x-radius)/cell);i<=Math.floor((x+radius)/cell);i++)for(let j=Math.floor((y-radius)/cell);j<=Math.floor((y+radius)/cell);j++){
   const a=grid.get(i+','+j);if(a)for(const k of a)candidates.add(k);
  }
  let hit=null,best=radius*radius;
  for(const id of candidates){
   const e=edges[id];if(e.order<minOrder)continue;
   for(let j=2;j<e.xy.length;j+=2){
    const ax=e.xy[j-2],ay=e.xy[j-1],dx=e.xy[j]-ax,dy=e.xy[j+1]-ay,dd=dx*dx+dy*dy;
    const t=dd?Math.max(0,Math.min(1,((x-ax)*dx+(y-ay)*dy)/dd)):0;
    const px=ax+t*dx,py=ay+t*dy,d=(px-x)**2+(py-y)**2;
    if(d<=best){best=d;hit={id,x:px,y:py,segment:j/2,t,distance:Math.sqrt(d)};}
   }
  }
  return hit;
 }
 function downstream(hit){
  const e=edges[hit.id],xy=[hit.x,hit.y,...e.xy.slice(hit.segment*2)];
  const prefix=e.cum[hit.segment-1]+hit.t*(e.cum[hit.segment]-e.cum[hit.segment-1]);
  let km=e.km*(1-prefix/Math.max(e.length,1e-9));
  let j=e.next, end=e, ids=[e.i];const seen=new Set(ids);
  while(j>=0&&!seen.has(j)){
   seen.add(j);end=edges[j];ids.push(j);xy.push(...end.xy.slice(2));km+=end.km;j=end.next;
  }
  return {xy,km,end,ids};
 }
 function upstream(hit){
  const ids=[],stack=edges[hit.id].up.slice(),seen=new Set([hit.id]);
  while(stack.length){const i=stack.pop();if(seen.has(i))continue;seen.add(i);ids.push(i);stack.push(...edges[i].up);}
  const e=edges[hit.id],partial=[...e.xy.slice(0,hit.segment*2),hit.x,hit.y];
  return {ids,partial};
 }
 return {edges,nearest,downstream,upstream};
}
function pathMetrics(xy){let cum=[0];for(let j=2;j<xy.length;j+=2)cum.push(cum[cum.length-1]+Math.hypot(xy[j]-xy[j-2],xy[j+1]-xy[j-1]));return {xy,cum,length:cum[cum.length-1]};}
function pointAt(p,d){
 if(!p.length)return p.xy.slice(0,2);
 d=Math.max(0,Math.min(p.length,d));let lo=0,hi=p.cum.length-1;
 while(lo<hi){const m=(lo+hi)>>1;if(p.cum[m]<d)lo=m+1;else hi=m;}
 const j=Math.max(1,lo),a=p.cum[j-1],b=p.cum[j],t=b>a?(d-a)/(b-a):0;
 return [p.xy[(j-1)*2]+(p.xy[j*2]-p.xy[(j-1)*2])*t,p.xy[(j-1)*2+1]+(p.xy[j*2+1]-p.xy[(j-1)*2+1])*t];
}
const api={C,W,H,project,invert,build,pathMetrics,pointAt};
if(typeof module!=='undefined'&&module.exports)module.exports=api;else root.WaterCore=api;
})(typeof window!=='undefined'?window:globalThis);

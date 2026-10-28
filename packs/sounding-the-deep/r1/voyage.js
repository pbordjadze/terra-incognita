/* From "One ship, two rhythms" (the first version of this study, removed; in the repository's history). Day-resolution observations and an explicitly illustrative transit model. */
(function(root){'use strict';
const DAY=86400000,rad=Math.PI/180;
const day=s=>Date.parse(s+'T00:00:00Z')/DAY;
const date=d=>new Date(Math.floor(d)*DAY);
function label(d,span=0){const dt=date(d);return span>1500?String(dt.getUTCFullYear()):dt.toLocaleDateString('en-GB',{day:'numeric',month:'short',year:'numeric',timeZone:'UTC'});}
function interpolate(a,b,t){
 const vector=p=>{const la=p.lat*rad,lo=p.lon*rad;return [Math.cos(la)*Math.cos(lo),Math.cos(la)*Math.sin(lo),Math.sin(la)]};
 const u=vector(a),v=vector(b),dot=Math.max(-1,Math.min(1,u.reduce((s,x,i)=>s+x*v[i],0))),angle=Math.acos(dot);
 if(angle<1e-9)return {lon:a.lon,lat:a.lat};
 const sa=Math.sin((1-t)*angle)/Math.sin(angle),sb=Math.sin(t*angle)/Math.sin(angle),p=u.map((x,i)=>x*sa+v[i]*sb);
 return {lon:Math.atan2(p[1],p[0])/rad,lat:Math.atan2(p[2],Math.hypot(p[0],p[1]))/rad};
}
function prepare(observations,land,contains){
 const out=observations.map(o=>({...o,day:day(o.date)})).sort((a,b)=>a.day-b.day);
 for(let i=0;i<out.length-1;i++){
  const a=out[i],b=out[i+1],gap=b.day-a.day;
  // A short, ocean-only connection is an illustration, never a logged course.
  a.connect=gap>1&&gap<=14;
  if(a.connect&&land&&contains)for(let n=1;n<20;n++){
   const p=interpolate(a,b,n/20);
   if(land.some(f=>contains(f.geometry,[p.lon,p.lat]))){a.connect=false;break;}
  }
 }
 return out;
}
function position(observations,d){
 if(d<observations[0].day||d>=observations.at(-1).day+1)return {kind:'outside'};
 const exact=observations.find(o=>Math.floor(d)===o.day);
 if(exact)return {...exact,kind:'recorded'};
 const index=observations.findIndex(o=>o.day>d),a=observations[index-1],b=observations[index];
 if(!a||!b)return {kind:'outside'};
 if(!a.connect)return {kind:'gap',previous:a.station,next:b.station,previousDate:a.date,nextDate:b.date};
 const t=Math.max(0,Math.min(1,(d-(a.day+1))/(b.day-a.day-1)));
 return {...interpolate(a,b,t),kind:'interpolated',previous:a.station,next:b.station,previousDate:a.date,nextDate:b.date};
}
function focusSpan(kind){return kind==='empire'?36525:30;}
const api={DAY,day,date,label,interpolate,prepare,position,focusSpan};
if(typeof module==='object'&&module.exports)module.exports=api;else root.ShipMath=api;
})(typeof globalThis!=='undefined'?globalThis:this);

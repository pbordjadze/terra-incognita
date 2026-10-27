/* Time Atlas: numeric years, independent map and time cameras. No Date range limit. */
(function(root) {
'use strict';
const clamp=(x,a,b)=>Math.max(a,Math.min(b,x));
function finite(x,name){if(!Number.isFinite(x))throw new TypeError(name+' must be finite');return x;}
function yearLabel(year){const y=Math.round(year);return y<=0?(1-y)+' BCE':String(y);}
function nearestSnapshot(snapshots, year) {
  if(!snapshots.length)throw new Error('No dated maps');
  return snapshots.reduce((best,s)=>Math.abs(s.year-year)<Math.abs(best.year-year)?s:best);
}
class TimeCamera {
  constructor({min,max,year,span,minSpan=2}) {
    if(!(max>min))throw new RangeError('Invalid time extent');
    this.min=min;this.max=max;this.minSpan=Math.min(minSpan,max-min);
    this.year=clamp(finite(year,'year'),min,max);this.span=clamp(finite(span,'span'),this.minSpan,max-min);
  }
  at(x,width){return this.year+(x/width-.5)*this.span;}
  setYear(y){this.year=clamp(finite(y,'year'),this.min,this.max);}
  drag(pixels,width){if(width>0)this.setYear(this.year-pixels/width*this.span);}
  zoom(factor,x,width){
    if(!(factor>0&&width>0&&Number.isFinite(factor)))return;
    const anchor=this.at(x,width), next=clamp(this.span/factor,this.minSpan,this.max-this.min);
    this.setYear(anchor-(x/width-.5)*next);this.span=next;
  }
  focus(start,end){const span=Math.max(this.minSpan,end-start);this.span=clamp(span*1.15,this.minSpan,this.max-this.min);}
  ticks(width){
    const target=Math.max(1,this.span/Math.max(2,width/100)), power=10**Math.floor(Math.log10(target));
    const step=[1,2,5,10].map(x=>x*power).find(x=>x>=target)||10*power;
    const first=Math.ceil((this.year-this.span/2)/step)*step, ticks=[];
    for(let y=first;y<=this.year+this.span/2+step*.01;y+=step) {
      if(y>=this.min&&y<=this.max)ticks.push({year:y,x:((y-this.year)/this.span+.5)*width});
    }
    return ticks;
  }
}
function rings(geometry){return geometry.type==='Polygon'?[geometry.coordinates]:geometry.type==='MultiPolygon'?geometry.coordinates:[];}
function inRing(point,ring){
  let inside=false;const [x,y]=point;
  for(let i=0,j=ring.length-1;i<ring.length;j=i++){
    const a=ring[i],b=ring[j];
    if((a[1]>y)!==(b[1]>y)&&x<(b[0]-a[0])*(y-a[1])/(b[1]-a[1])+a[0])inside=!inside;
  }
  return inside;
}
function contains(geometry,point){return rings(geometry).some(poly=>inRing(point,poly[0])&&!poly.slice(1).some(r=>inRing(point,r)));}
function bounds(features){
  let west=Infinity,south=Infinity,east=-Infinity,north=-Infinity;
  for(const f of features)for(const p of rings(f.geometry))for(const r of p)for(const [x,y]of r){
    west=Math.min(west,x);east=Math.max(east,x);south=Math.min(south,y);north=Math.max(north,y);
  }
  return Number.isFinite(west)?[west,south,east,north]:null;
}
const mercator=lat=>Math.log(Math.tan(Math.PI/4+clamp(lat,-80,80)*Math.PI/360));
class MapCamera{
  constructor(lon=18,lat=50,scale=900){this.lon=lon;this.lat=lat;this.scale=scale;}
  project([lon,lat],width,height){return [width/2+(lon-this.lon)*Math.PI/180*this.scale,height/2-(mercator(lat)-mercator(this.lat))*this.scale];}
  invert([x,y],width,height){return [this.lon+(x-width/2)/this.scale*180/Math.PI, (2*Math.atan(Math.exp(mercator(this.lat)-(y-height/2)/this.scale))-Math.PI/2)*180/Math.PI];}
  pan(dx,dy){this.lon=clamp(this.lon-dx/this.scale*180/Math.PI,-180,180);this.lat=clamp((2*Math.atan(Math.exp(mercator(this.lat)+dy/this.scale))-Math.PI/2)*180/Math.PI,-75,75);}
  zoom(factor,x,y,width,height){
    const anchor=this.invert([x,y],width,height);
    this.scale=clamp(this.scale*factor,100,14000);
    const moved=this.project(anchor,width,height);
    this.pan(x-moved[0],y-moved[1]);
  }
  fit(box,width,height){
    if(!box||width<=0||height<=0)return;
    const [w,s,e,n]=box;
    this.lon=(w+e)/2;this.lat=(2*Math.atan(Math.exp((mercator(s)+mercator(n))/2))-Math.PI/2)*180/Math.PI;
    this.scale=clamp(Math.min(width/Math.max(.05,(e-w)*Math.PI/180),height/Math.max(.05,mercator(n)-mercator(s)))*.7,100,14000);
  }
}
function featureName(f){return f.properties.NAME || f.properties.name || 'Unnamed state';}
function subjectId(f){
  const name=featureName(f);
  if(/^(Poland|Poland-Lithuania|Polish.Lithuanian Commonwealth)$/i.test(name))return 'poland';
  if(/Ottoman/i.test(name))return 'ottoman';
  return name;
}
const api={clamp,yearLabel,nearestSnapshot,TimeCamera,MapCamera,rings,contains,bounds,featureName,subjectId};
if(typeof module==='object'&&module.exports)module.exports=api;
else root.TimeAtlasCore=api;
})(typeof globalThis!=='undefined'?globalThis:this);

'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const root=path.resolve(__dirname,'../../..'),C=require(path.join(root,'Prototypes/time-atlas/core.js'));
const sandbox={window:{}};vm.runInNewContext(fs.readFileSync(path.join(root,'Prototypes/time-atlas/data.js'),'utf8'),sandbox);
const D=sandbox.window.TIME_DATA;
test('timeline pinch preserves the date under its anchor',()=>{
 const t=new C.TimeCamera({min:0,max:1000,year:500,span:200}),anchor=t.at(750,1000);
 t.zoom(2,750,1000);assert.equal(t.span,100);assert.equal(t.at(750,1000),anchor);
 t.drag(100,1000);assert.equal(t.year,515);
});
test('toolbar zoom works independently of collapsed layout',()=>{
 const t=new C.TimeCamera({min:1530,max:1994,year:1938,span:120});
 t.zoom(2,.5,1);assert.equal(t.year,1938);assert.equal(t.span,60);
});
test('time remains bounded and yearly ticks never duplicate rounded dates',()=>{
 const t=new C.TimeCamera({min:1530,max:1994,year:1938,span:120});
 t.setYear(2500);assert.equal(t.year,1994);t.zoom(1e9,.5,1);assert.equal(t.span,2);
 const labels=t.ticks(1400).map(x=>C.yearLabel(x.year));assert.equal(new Set(labels).size,labels.length);
 assert.throws(()=>t.setYear(NaN));assert.throws(()=>new C.TimeCamera({min:10,max:1,year:5,span:2}));
});
test('numeric years support geological scale without JavaScript Date',()=>{
 const t=new C.TimeCamera({min:-4500000000,max:2026,year:-100000000,span:1000000});
 t.zoom(2,.5,1);assert.equal(t.span,500000);assert.equal(C.yearLabel(0),'1 BCE');assert.equal(C.yearLabel(-99),'100 BCE');
});
test('map projection roundtrips and pinch preserves its geographic anchor',()=>{
 const m=new C.MapCamera(20,45,1000),point=[21,52.23],screen=m.project(point,1000,800);
 const inverse=m.invert(screen,1000,800);assert.ok(Math.hypot(...inverse.map((v,i)=>v-point[i]))<1e-8);
 const before=m.invert([200,300],1000,800);m.zoom(2,200,300,1000,800);
 const after=m.invert([200,300],1000,800);assert.ok(Math.hypot(...after.map((v,i)=>v-before[i]))<1e-8);
});
test('country hit testing respects polygon holes',()=>{
 const g={type:'Polygon',coordinates:[[[0,0],[10,0],[10,10],[0,10],[0,0]],[[3,3],[7,3],[7,7],[3,7],[3,3]]]};
 assert.equal(C.contains(g,[1,1]),true);assert.equal(C.contains(g,[5,5]),false);assert.equal(C.contains(g,[20,5]),false);
});
test('dated evidence stays discrete and both Ottoman components are retained',()=>{
 assert.deepEqual(Array.from(D.snapshots,s=>s.year),[1530,1600,1815,1878,1920,1938,1945,1994]);
 assert.equal(C.nearestSnapshot(D.snapshots,1943).year,1945);
 for(const s of D.snapshots.filter(s=>s.year<=1600))assert.equal(s.features.filter(f=>C.subjectId(f)==='ottoman').length,2);
 assert.equal(D.snapshots.find(s=>s.year===1815).features.some(f=>C.subjectId(f)==='poland'),false);
});
test('Warsaw belongs to the four Polish extracts and all rings remain closed',()=>{
 for(const s of D.snapshots.filter(s=>s.year>=1920))assert.ok(s.features.some(f=>C.featureName(f)==='Poland'&&C.contains(f.geometry,[21,52.23])));
 for(const s of D.snapshots)for(const f of s.features)for(const polygon of C.rings(f.geometry))for(const ring of polygon){
  assert.ok(ring.length>=4);assert.deepEqual(Array.from(ring[0]),Array.from(ring.at(-1)));
  for(const point of ring)assert.ok(point.every(Number.isFinite));
 }
 assert.ok(D.land.features.length>0);
});
test('historical data keeps its provenance and separate GPL license',()=>{
 assert.equal(D.source.commit,'da7a4b735ecef70aebdc9c73e409d8a2500d50f3');assert.equal(D.source.license,'GPL-3.0');
 assert.ok(fs.readFileSync(path.join(root,'Prototypes/time-atlas/LICENSE-HISTORY.txt'),'utf8').includes('GNU GENERAL PUBLIC LICENSE'));
});

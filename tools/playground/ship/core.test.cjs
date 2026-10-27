'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const root=path.resolve(__dirname,'../../..'),folder=path.join(root,'Prototypes/one-ship-two-rhythms');
const S=require(path.join(folder,'voyage.js')),C=require(path.join(folder,'core.js'));
const env={window:{}};vm.runInNewContext(fs.readFileSync(path.join(folder,'data.js'),'utf8'),env);
const D=env.window.SHIP_DATA,obs=S.prepare(D.observations,D.land.features,C.contains);
test('source stations preserve dates, coordinates and reusable attribution',()=>{
 assert.equal(obs.length,94);const known=obs.find(o=>o.station==='CHA-3');assert.equal(known.date,'1873-02-18');
 assert.equal(known.lon,-20.233333);assert.equal(known.lat,25.75);
 assert.equal(D.sources.pangaea849072.license,'CC-BY-3.0');assert.equal(D.primaryExport.inputSha256,'b9597170e4c1494e83937d362b8cc0d5d076d9d53949421d853019c7e3b9fa13');
 assert.equal(D.ship.completeTrack,false);
});
test('date labels use UTC day precision',()=>{
 const d=S.day('1873-02-18');assert.equal(S.date(d).toISOString().slice(0,10),'1873-02-18');
 assert.equal(S.label(d+.9),S.label(d));assert.equal(S.position(obs,d+.9).kind,'recorded');
});
test('illustrated transit is reversible and distinct from a recorded position',()=>{
 const known=obs.find(o=>o.station==='CHA-3'),a=known.day,p=S.position(obs,a+1),q=S.position(obs,a+2);
 assert.equal(p.kind,'interpolated');assert.equal(q.kind,'interpolated');
 assert.ok(q.lon<p.lon);assert.equal(S.position(obs,a+1).lon,p.lon);
 const next=obs.find(o=>o.station==='CHA-7');assert.equal(S.position(obs,next.day).lon,next.lon);
 assert.equal(S.position(obs,a).kind,'recorded');
});
test('missing months have no invented location and no world-wrapping connector',()=>{
 const gaps=obs.slice(1).map((b,i)=>({a:obs[i],b,days:b.day-obs[i].day})).sort((a,b)=>b.days-a.days),gap=gaps[0];assert.ok(gap.days>14);
 const p=S.position(obs,gap.a.day+gap.days/2);assert.equal(p.kind,'gap');assert.equal(p.lon,undefined);
 assert.equal(gap.a.connect,false);assert.equal(S.position(obs,obs[0].day-1).kind,'outside');
 assert.equal(S.position(obs,obs.at(-1).day+1).kind,'outside');
});
test('great-circle interpolation takes the short path across the date line',()=>{
 const p=S.interpolate({lat:0,lon:170},{lat:0,lon:-170},.5);
 assert.ok(Math.abs(Math.abs(p.lon)-180)<1e-8);assert.ok(Math.abs(p.lat)<1e-8);
});
test('an illustrative connection that crosses land is rejected',()=>{
 const land=[{geometry:{type:'Polygon',coordinates:[[[-1,-1],[1,-1],[1,1],[-1,1],[-1,-1]]]}}];
 const rows=[{date:'1873-01-01',lon:-3,lat:0},{date:'1873-01-04',lon:3,lat:0}];
 assert.equal(S.prepare(rows,land,C.contains)[0].connect,false);
});
test('ship and empire scales share the exact same date',()=>{
 const t=new C.TimeCamera({min:S.day('1800-01-01'),max:S.day('1900-01-01'),year:obs[0].day+2,span:30,minSpan:2});
 const before=t.year;t.span=S.focusSpan('empire');assert.equal(t.year,before);assert.ok(t.span>1000*S.focusSpan('ship'));
 t.span=S.focusSpan('ship');assert.equal(t.year,before);assert.equal(t.span,30);
});
test('pinching time preserves the date under the fingers',()=>{
 const t=new C.TimeCamera({min:S.day('1800-01-01'),max:S.day('1900-01-01'),year:obs[0].day+2,span:30,minSpan:2}),anchor=t.at(200,390);
 t.zoom(2,200,390);assert.ok(Math.abs(t.at(200,390)-anchor)<1e-9);assert.equal(t.span,15);
});
test('real sparse data uses only short, explicitly illustrative links',()=>{
 for(let i=0;i<obs.length-1;i++)if(obs[i].connect)assert.ok(obs[i+1].day-obs[i].day<=14);
 assert.ok(obs.some(o=>o.connect));assert.ok(obs.some(o=>o.connect===false));
});

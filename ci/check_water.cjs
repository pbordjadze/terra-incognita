#!/usr/bin/env node
// Independent data invariants: graph termini, geography, clipping and no fabricated links.
const fs=require('fs'),vm=require('vm'),assert=require('assert/strict');
const context={window:{}};vm.createContext(context);
vm.runInContext(fs.readFileSync('Water/network.js','utf8'),context);
const data=context.window.WATER_DATA;
const core=require('../Water/core.js');const net=core.build(data);
const n=data.rivers.length;assert(n>10000&&n<30000);
let ends=0, sinks=0;const terminals=new Map();
for(let i=0;i<n;i++){
 const e=data.rivers[i];assert(Number.isInteger(e[0])&&e[0]>=-1&&e[0]<n);assert(e[2]>0&&e[5].length>=4&&e[5].length%2===0);
 for(let j=0;j<e[5].length;j+=2){assert(e[5][j]>=-84&&e[5][j]<=-32);assert(e[5][j+1]>=-58&&e[5][j+1]<=16);}
 if(e[0]<0){ends++;sinks+=e[4];assert(e[3]===e[6]||e[3]>0);}
 else{
  const dest=data.rivers[e[0]];assert.equal(dest[3],e[3],'A downstream link cannot jump systems');
  const dx=e[5].at(-2)-dest[5][0],dy=e[5].at(-1)-dest[5][1];assert(Math.hypot(dx,dy)<.065,'A join must be geographically continuous');
 }
 let j=i,seen=new Set();
 while(j>=0&&!terminals.has(j)){assert(!seen.has(j),'Cycle');seen.add(j);j=data.rivers[j][0];}
 const final=j<0?[...seen].at(-1):terminals.get(j);
 for(const k of seen)terminals.set(k,final);
 assert.equal(data.rivers[final][3],e[3]);
}
assert(ends>100&&sinks>0);
assert.equal(net.nearest(...core.project(-35,-50),.1),null,'Do not invent a river in the ocean');
const p=core.project(-60,-3.1),hit=net.nearest(...p,.3,4);assert(hit);
const down=net.downstream(hit),up=net.upstream(hit);assert(down.km>1000&&down.end.next<0);assert(up.ids.length>500);
assert.deepEqual(down.xy.slice(0,2),[hit.x,hit.y]);assert.deepEqual(up.partial.slice(-2),[hit.x,hit.y]);
assert(!up.ids.includes(hit.id));assert(!up.ids.includes(net.edges[hit.id].next),'Gather cannot include channels downstream of the touch');
for(const id of up.ids){let j=id,count=0;while(j!==hit.id&&j>=0&&count++<n)j=net.edges[j].next;assert.equal(j,hit.id);}
// Original geographic endpoints survive the display projection and inverse.
for(const pt of [[-60,-3.1],[-74,-12],[-68,-54],[0,0]]){const r=core.invert(...core.project(...pt));assert(Math.hypot(r[0]-pt[0],r[1]-pt[1])<1e-10);}
const metrics=core.pathMetrics(down.xy);assert.deepEqual(core.pointAt(metrics,0),down.xy.slice(0,2));assert.deepEqual(core.pointAt(metrics,metrics.length),down.xy.slice(-2));
console.log(`Water: ${n} merged reaches, ${ends} termini (${sinks} inland), all joins continuous, acyclic, basin-consistent; upstream/downstream clipping verified.`);

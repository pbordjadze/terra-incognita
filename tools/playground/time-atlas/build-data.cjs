#!/usr/bin/env node
'use strict';
// Rebuild the historical display extract. Node 18+; network used only at authoring time.
// The committed data.js source metadata is the explicit extraction configuration.
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const root = path.resolve(__dirname, '../../..');
const directory = path.join(root, 'Prototypes/time-atlas');
const inputSha = {
  1530:'743b3db1fd414891e5d9e9eb868c2aaf2b04810e',
  1600:'ea95bd8fcfb7cdf889739aa6222cbc62329d93ec',
  1815:'4aac1a9d1fa0ab1271b23dcf24eff8ba8298e48d',
  1878:'e7b4d0c6115f97dd541a32ac2f2d1f8663ddf999',
  1920:'be6722f403fc64af4f584b67a17ac59af617fa1e',
  1938:'27fa58024df898bfdc0e530ac6b856e0fa560cb4',
  1945:'08b7fb87faf31e534fc31cc7d8cc9e3d415fa133',
  1994:'931b7ed93d8c4fcf23a4e9a351b4e179d00ba451'
};
async function download(repository, commit, file, expected) {
  const url = 'https://raw.githubusercontent.com/'+repository+'/'+commit+'/'+file;
  const response = await fetch(url,{signal:AbortSignal.timeout(60000)});
  if (!response.ok) throw Error(response.status+' '+url);
  const bytes = Buffer.from(await response.arrayBuffer());
  const sha = crypto.createHash('sha1').update('blob '+bytes.length+'\0').update(bytes).digest('hex');
  if (sha !== expected) throw Error('Source hash mismatch: '+url);
  return bytes.toString('utf8');
}
function intersects(polygon, box) {
  let a=Infinity,b=Infinity,c=-Infinity,d=-Infinity;
  for (const ring of polygon) for (const [x,y] of ring) {
    a=Math.min(a,x);b=Math.min(b,y);c=Math.max(c,x);d=Math.max(d,y);
  }
  return c>=box[0] && a<=box[2] && d>=box[1] && b<=box[3];
}
function distance(p,a,b) {
  let x=a[0],y=a[1],dx=b[0]-x,dy=b[1]-y;
  if(dx||dy){
    const t=((p[0]-x)*dx+(p[1]-y)*dy)/(dx*dx+dy*dy);
    if(t>1){x=b[0];y=b[1];}else if(t>0){x+=dx*t;y+=dy*t;}
  }
  return (p[0]-x)**2+(p[1]-y)**2;
}
function simplify(ring,tolerance) {
  if(ring.length<6)return ring;
  const keep=new Uint8Array(ring.length), todo=[[0,ring.length-1]];
  keep[0]=keep[ring.length-1]=1;
  while(todo.length){
    const [a,b]=todo.pop();let max=tolerance*tolerance,index=-1;
    for(let i=a+1;i<b;i++){const d=distance(ring[i],ring[a],ring[b]);if(d>max){max=d;index=i;}}
    if(index>=0){keep[index]=1;todo.push([a,index],[index,b]);}
  }
  const result=ring.filter((_,i)=>keep[i]);
  return result.length>=4?result:ring;
}
function extract(collection,year,source) {
  const names=new Set(source.allowlists[year]), scale=10**source.coordinateDecimals,features=[];
  for(const f of collection.features){
    if(!names.has(f.properties?.NAME))continue;
    const g=f.geometry;
    const polygons=g?.type==='MultiPolygon'?g.coordinates:g?.type==='Polygon'?[g.coordinates]:[];
    const coordinates=(year<1800?polygons:polygons.filter(p=>intersects(p,source.bounds)))
      .map(p=>p.map(r=>simplify(r,source.coordinateToleranceDegrees).map(pt=>pt.map(x=>Math.round(x*scale)/scale))));
    if(!coordinates.length)continue;
    for(const p of coordinates)for(const r of p){
      if(r.length<4||r[0][0]!==r.at(-1)[0]||r[0][1]!==r.at(-1)[1]||
        r.some(pt=>pt.some(x=>!Number.isFinite(x))))throw Error('Invalid ring: '+year+' '+f.properties.NAME);
    }
    features.push({type:'Feature',properties:{NAME:f.properties.NAME,SUBJECTO:f.properties.SUBJECTO??null,
      BORDERPRECISION:f.properties.BORDERPRECISION??null},geometry:{type:'MultiPolygon',coordinates}});
  }
  const sourceFile='geojson/world_'+year+'.geojson';
  return {year,features,sourceFile,sourceUrl:'https://github.com/'+source.repository+'/blob/'+source.commit+'/'+sourceFile};
}
async function main(){
  const output=path.join(directory,'data.js'), text=await fs.readFile(output,'utf8');
  const marker='window.TIME_DATA=',index=text.indexOf(marker);
  if(index<0)throw Error('Missing TIME_DATA assignment');
  const current=JSON.parse(text.slice(index+marker.length).trim().replace(/;$/,''));
  const source=current.source, years=current.snapshots.map(s=>s.year);
  if(source.coordinateToleranceDegrees!==0.025||source.coordinateDecimals!==4)throw Error('Unexpected simplification configuration');
  const snapshots=await Promise.all(years.map(async year=>extract(JSON.parse(await download(
    source.repository,source.commit,'geojson/world_'+year+'.geojson',inputSha[year])),year,source)));
  const neRepo='nvkelso/natural-earth-vector', neCommit='ca96624a56bd078437bca8184e78163e5039ad19';
  const [land,historyLicense,landLicense]=await Promise.all([
    download(neRepo,neCommit,'geojson/ne_110m_land.geojson','04811d72fff2701ec67587e30ad8942675b511e3'),
    download(source.repository,source.commit,'LICENSE','94a9ed024d3859793618152ea559a168bbcbb5e2'),
    download(neRepo,neCommit,'LICENSE.md','9dfbc3d55a4ab1dd758c253b2ee339888f632889')
  ]);
  const rebuilt=text.slice(0,index+marker.length)+JSON.stringify({source,snapshots,land:JSON.parse(land)})+';\n';
  await fs.writeFile(output,rebuilt);
  await fs.writeFile(path.join(directory,'LICENSE-HISTORY.txt'),historyLicense);
  await fs.writeFile(path.join(directory,'LICENSE-NATURAL-EARTH.md'),landLicense);
  console.log('Rebuilt '+Buffer.byteLength(rebuilt)+' bytes; '+snapshots.length+' source snapshots.');
}
main().catch(error=>{console.error(error.message);process.exitCode=1;});

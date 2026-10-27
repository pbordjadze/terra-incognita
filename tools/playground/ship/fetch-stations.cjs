#!/usr/bin/env node
'use strict';
// Optional authoring-time extraction from the licensed primary station dataset. Node18+.
// Writes a build artifact; never changes the curated pack data.
// node tools/playground/ship-time/fetch-stations.cjs [--input cached.tsv] [--out build/challenger-stations.json]
// node tools/playground/ship-time/fetch-stations.cjs --self-test
const fs=require('node:fs/promises'),path=require('node:path'),crypto=require('node:crypto');
const URL='https://doi.pangaea.de/10.1594/PANGAEA.849072?format=textfile';
function parseEvents(text){
  const observations=[],seen=new Set();
  for(const line of text.split(/\r?\n/)){
    const name=/\b(CHA-[A-Za-z0-9-]+)\s*\*/.exec(line),
      lat=/Latitude:\s*(-?\d+(?:\.\d+)?)/i.exec(line),
      lon=/Longitude:\s*(-?\d+(?:\.\d+)?)/i.exec(line),
      date=/Date\/Time:\s*(\d{4}-\d{2}-\d{2})/i.exec(line);
    if(!name||!lat||!lon||!date)continue;
    const station=name[1],latitude=Number(lat[1]),longitude=Number(lon[1]),day=date[1];
    if(!Number.isFinite(latitude)||latitude < -90||latitude > 90||
       !Number.isFinite(longitude)||longitude < -180||longitude > 180||
       !Number.isFinite(Date.parse(day+'T00:00:00Z'))||
       new Date(day+'T00:00:00Z').toISOString().slice(0,10)!==day)
      throw Error('Invalid station metadata: '+station);
    const key=[station,day,latitude,longitude].join('|');
    if(seen.has(key))continue;seen.add(key);
    const elevation=/Elevation:\s*(-?\d+(?:\.\d+)?)/i.exec(line);
    observations.push({station,date:day,lat:latitude,lon:longitude,
      depth:elevation?Math.max(0,-Number(elevation[1])):null,source:'pangaea849072'});
  }
  return observations.sort((a,b)=>a.date.localeCompare(b.date)||a.station.localeCompare(b.station));
}
function selfTest(){
  const first='Event(s): CHA-3 * Latitude: 25.750000 * Longitude: -20.233333 * Date/Time: 1873-02-18T00:00:00 * Elevation: -2504.0 m',
    second='CHA-7 * Latitude: 23.383333 * Longitude: -31.516667 * Date/Time: 1873-02-24T00:00:00 * Elevation: -4516.0 m';
  const rows=parseEvents([second,first,first,'unrelated data'].join('\n'));
  if(rows.length!==2||rows[0].station!=='CHA-3'||rows[0].date!=='1873-02-18'||
     rows[0].depth!==2504||rows[1].lon!==-31.516667)throw Error('Station parser self-test failed');
  let rejected=false;try{parseEvents(first.replace('25.750000','125.750000'));}catch(_){rejected=true;}
  if(!rejected)throw Error('Invalid coordinate accepted');
  console.log('PASS: station parsing, sorting, deduplication, date precision and coordinate bounds');
}
async function main(){
  const args=process.argv.slice(2),value=k=>{const i=args.indexOf(k);return i<0?null:args[i+1];};
  if(args.includes('--self-test')){selfTest();return;}
  for(const key of ['--input','--out'])if(args.includes(key)&&!value(key))throw Error(key+' requires a path');
  let text;
  if(value('--input'))text=await fs.readFile(value('--input'),'utf8');
  else{
    const response=await fetch(URL,{signal:AbortSignal.timeout(45000),
      headers:{Accept:'*/*','User-Agent':'TerraIncognita-source-rebuild/1'}});
    if(!response.ok)throw Error('PANGAEA download failed: HTTP '+response.status+' '+(await response.text()).slice(0,1000));
    text=await response.text();
  }
  const observations=parseEvents(text);
  if(observations.length<12)throw Error('Only '+observations.length+' station records found. Check export/parser; curated pack remains unchanged.');
  const output=value('--out')||'build/challenger-stations.json',data={
    source:{id:'pangaea849072',url:URL,doi:'10.1594/PANGAEA.849072',
      citation:'Murray, John; Renard, A F (1891): Annotated record of the detailed examination of Mn deposits from the HMS Challenger stations. PANGAEA.',
      license:'CC-BY-3.0',licenseUrl:'https://creativecommons.org/licenses/by/3.0/',
      inputSha256:crypto.createHash('sha256').update(text).digest('hex')},
    observations,notes:[
      'Selected sampling-event metadata, not a complete ship log or documented route.',
      'Timestamps reduced to source dates. Midnight sampling times are not asserted.',
      'Depth is negative event elevation, not an interpolated seafloor profile.',
      'Gaps remain explicit. Connecting observations is illustrative, not navigation evidence.',
      'Optional export only: does not overwrite or automatically extend curated runtime data.'
    ]};
  await fs.mkdir(path.dirname(output),{recursive:true});
  await fs.writeFile(output,JSON.stringify(data,null,2)+'\n');
  console.log('Wrote '+observations.length+' dated sampling events to '+output);
}
main().catch(error=>{console.error(error.message);process.exitCode=1;});

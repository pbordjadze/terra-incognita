'use strict';
// Export the exact same static drawing used by the leaf. Run the local leaf server first.
// This keeps first-open WebKit work small; geometry remains available to the sounding glass.
const fs=require('fs'),{chromium}=require('playwright');
(async()=>{
 const b=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'/usr/bin/chromium',args:['--no-sandbox']});
 try{const p=await b.newPage();await p.goto((process.env.LEAF_URL||'http://127.0.0.1:8787')+'/index.html?paint=1');await p.waitForFunction(()=>window.leafReady);
 for(const [i,name]of ['day','night'].entries()){
   const encoded=await p.evaluate(i=>leafPaintedMap(i).split(',')[1],i);fs.writeFileSync(`Leaf/data/${name}.png`,Buffer.from(encoded,'base64'));console.log(`Painted ${name}`);
 }
 }finally{await b.close();}
})().catch(e=>{console.error(e);process.exit(1);});

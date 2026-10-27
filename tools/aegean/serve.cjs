'use strict';
const http=require('http'),fs=require('fs'),path=require('path');
const root=path.resolve(__dirname,'../../Leaf');
const mime={'.html':'text/html','.js':'text/javascript','.css':'text/css','.png':'image/png','.otf':'font/otf','.txt':'text/plain'};
const server=http.createServer((req,res)=>{const file=path.resolve(root,'.'+decodeURIComponent(req.url.split('?')[0]==='/'?'/index.html':req.url.split('?')[0]));if(!file.startsWith(root+path.sep)){res.writeHead(403);return res.end();}fs.readFile(file,(e,d)=>{res.writeHead(e?404:200,{'Content-Type':mime[path.extname(file)]||'application/octet-stream'});res.end(e?'Not found':d);});});
server.listen(Number(process.env.PORT||8787),'127.0.0.1',()=>console.log('Leaf: http://127.0.0.1:'+server.address().port));

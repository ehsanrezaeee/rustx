// Actual loopback HTTP + WebSocket adapters; permission-denial branches below are contract tests.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {HostNetwork,HostServices} from '../web/services.js';
const port=Number(process.env.RUSTX_SERVICE_PORT||3017),base=`http://127.0.0.1:${port}`;
const server=spawn(fileURLToPath(new URL('../target/debug/rustx-cli',import.meta.url)),['serve','dist',String(port)],{cwd:new URL('../',import.meta.url),stdio:['ignore','pipe','pipe']});
const deadline=(promise,label)=>Promise.race([promise,new Promise((_,reject)=>{const timer=setTimeout(()=>reject(new Error(`${label} timeout`)),5000);timer.unref();})]);
try {
 await deadline(new Promise((resolve,reject)=>{server.on('error',reject);server.on('exit',code=>reject(new Error(`server exit ${code}`)));server.stdout.on('data',chunk=>{if(String(chunk).includes('RustX serving'))resolve();});}),'startup');
 const request=async(method,body=[],headers=[])=>{
  let result;const host=new HostNetwork((id,status,bytes,error,version,responseHeaders)=>{result={id,status,bytes,error,version,responseHeaders};},fetch,base+'/');
  await host.request({kind:'Request',version:1,id:1,url:'/api/echo',max_bytes:1024,timeout_ms:2000,method,headers,body,credentials:'same-origin'});assert.equal(result.error,'');assert.equal(result.status,200);assert(result.responseHeaders.some(([name])=>name==='content-type'));return JSON.parse(new TextDecoder().decode(result.bytes));
 };
 for(const method of ['GET','POST','PUT','PATCH','DELETE','OPTIONS']){const body=method==='GET'?[]:Array.from(new TextEncoder().encode('payload'));const reply=await request(method,body,[['content-type','text/plain']]);assert.equal(reply.method,method);assert.equal(reply.bytes,body.length);}
 const env={fetch,AbortController,ReadableStream,WebSocket,setTimeout,clearTimeout,location:{href:base+'/',origin:base}};
 const replies=[],waiters=[];const host=new HostServices((id,json,version)=>{const result={id,...JSON.parse(json),version};replies.push(result);for(const resolve of waiters.splice(0))resolve();},env);
 const waitFor=async predicate=>{for(let i=0;i<100;i++){const result=replies.find(predicate);if(result)return result;await deadline(new Promise(resolve=>waiters.push(resolve)),'service result');}throw new Error('Service condition not met');};
 host.accept([{version:1,id:3,kind:'timer',delay_ms:5}]);await waitFor(r=>r.id===3&&r.value?.type==='unit');
 host.accept([{version:1,id:4,kind:'timer',delay_ms:20},{version:1,id:0,kind:'cancel',resource:4}]);await new Promise(resolve=>setTimeout(resolve,30));assert(!replies.some(r=>r.id===4));assert.equal(host.resources.size,0);
 host.accept([{version:1,id:1,kind:'stream_open',url:'/api/stream',max_bytes:1024,timeout_ms:3000}]);await waitFor(r=>r.id===1&&r.value?.type==='stream');
 let bytes=[];for(;;){const previous=replies.length;host.accept([{version:1,id:0,kind:'stream_read',resource:1}]);await waitFor(r=>replies.indexOf(r)>=previous&&r.id===1&&r.value?.type==='chunk');const chunk=replies.at(-1).value;bytes.push(...chunk.bytes);if(chunk.done)break;}
 assert.equal(new TextDecoder().decode(new Uint8Array(bytes)),'RustX streamed data with bounded pull backpressure.\n');assert.equal(host.resources.size,0);
 host.accept([{version:1,id:2,kind:'socket_open',url:'/api/socket',protocols:[]}]);await waitFor(r=>r.id===2&&r.value?.type==='socket_open');
 host.accept([{version:1,id:0,kind:'socket_send',resource:2,bytes:Array.from(new TextEncoder().encode('Persian فارسی')),text:true}]);const message=await waitFor(r=>r.id===2&&r.value?.type==='socket_message');assert.equal(new TextDecoder().decode(new Uint8Array(message.value.bytes)),'Persian فارسی');
 host.accept([{version:1,id:0,kind:'socket_close',resource:2,code:1000,reason:'done'}]);await waitFor(r=>r.id===2&&r.value?.type==='socket_closed');assert.equal(host.resources.size,0);
 const denied=new HostServices((id,json)=>replies.push({id,...JSON.parse(json)}),{navigator:{clipboard:{writeText:async()=>{throw Object.assign(new Error('Permission denied'),{name:'NotAllowedError'});}}}});
 denied.accept([{version:1,id:99,kind:'clipboard_write',text:'text'}]);await deadline(new Promise(resolve=>setTimeout(resolve,0)),'denial');assert.equal(replies.find(r=>r.id===99).error.kind,'denied');
 host.dispose();denied.dispose();
 console.log('PASS: real fixture HTTP methods/headers/bodies; actual Fetch pull streaming, WebSocket UTF-8 echo/close/cleanup and scoped timers/cancellation. Clipboard denial is a separate injected contract test; no browser/Windows/permission success claim.');
}finally{server.kill();}

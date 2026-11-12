import init,* as app from './pkg/rustx_lab.js';
import {HostNetwork,HostServices} from './services.js';
import {SemanticProjection} from './semantics.js';
let canvas=document.querySelector('#surface');const root=document.querySelector('#semantic'),status=document.querySelector('#status');
let activeDevice=null,queued=false,renderer='cpu',ctx,ready=false,draws=0;
const network=new HostNetwork((id,status,body,error,version,headers=[])=>{const accepted=app.response_meta(id,status,body,error,version,JSON.stringify(headers));if(accepted&&error&&error!=='cancelled')resourceError('http',['timeout','too_large','busy'].includes(error)?error:'transport',null);pump();schedule();});
const services=new HostServices((id,json,version)=>{const accepted=app.service_response(version,id,json),error=JSON.parse(json).error;if(accepted&&error&&error.kind!=='cancelled')resourceError('service',error.kind);pump();schedule();});
function fatal(e){ready=false;const panel=document.querySelector('#fatal');const message=document.createElement('p'),reload=document.createElement('button');message.textContent='RustX could not continue. Reload to restart the application.';reload.textContent='Reload application';reload.addEventListener('click',()=>location.reload());panel.replaceChildren(message,reload);reload.focus();console.error(e);}
function schedule(){if(!queued&&ready&&!document.hidden){queued=true;requestAnimationFrame(draw);}}
function reveal(id){const n=JSON.parse(app.inspect()).nodes.find(x=>x.node===Number(id));const r=n?.scroll_parent?JSON.parse(app.inspect()).nodes.find(x=>x.node===n.scroll_parent)?.rect:n?.rect;if(r){if(r[1]<0){app.scroll(r[1]-16);schedule();}else if(r[1]+r[3]>innerHeight){app.scroll(r[1]+r[3]-innerHeight+16);schedule();}}}
const projection=new SemanticProjection(root,app,schedule,pump,reveal);
function resourceError(source,code,status=null){if(ready)projection.hostEvent('resource_error',{resource_error:{source,code,status}});}
function resized(){if(ready)projection.hostEvent('resize',{width:innerWidth,height:innerHeight,scale:devicePixelRatio||1});schedule();}
function sync(){projection.sync(JSON.parse(app.semantics()),JSON.parse(app.inspect()));}
document.addEventListener('keydown',e=>{if(ready)projection.tab(e);});
function pump(){network.accept(JSON.parse(app.commands()));if(app.service_commands)services.accept(JSON.parse(app.service_commands()));}
function draw(){queued=false;try{const w=innerWidth,h=innerHeight;if(w<=0||h<=0)return;const requested=Math.min(devicePixelRatio||1,2),limit=Math.min(8192/w,8192/h,Math.sqrt(16777216/(w*h))),dpr=Math.min(requested,limit<requested?limit*.999:requested),pw=Math.round(w*dpr),ph=Math.round(h*dpr);if(canvas.width!==pw)canvas.width=pw;if(canvas.height!==ph)canvas.height=ph;
 if(app.frame(w,h,dpr)){draws++;if(renderer==='cpu')ctx.putImageData(new ImageData(new Uint8ClampedArray(app.pixels()),pw,ph),0,0);sync();}pump();if(app.needs_update())schedule();}catch(e){if(renderer==='gpu'){recoverCPU(String(e));}else fatal(e);}}
function recoverCPU(reason){resourceError('renderer','gpu_recovery');const device=activeDevice;activeDevice=null;app.drop_gpu();device?.destroy();const replacement=canvas.cloneNode();canvas.replaceWith(replacement);canvas=replacement;ctx=canvas.getContext('2d');if(!ctx){fatal('CPU canvas unavailable');return;}renderer='cpu';status.textContent='RustX · CPU recovery · '+reason;schedule();}
async function retryGPU(){if(!navigator.gpu)throw new Error('WebGPU unavailable');const replacement=canvas.cloneNode();canvas.replaceWith(replacement);canvas=replacement;try{await app.init_gpu(canvas);renderer='gpu';app.invalidate();status.textContent='RustX · WebGPU · Stage 3';schedule();}catch(error){recoverCPU('GPU initialization failed');throw error;}}
// Observe actual browser device loss without owning geometry, state or layout.
if(navigator.gpu){const request=navigator.gpu.requestAdapter.bind(navigator.gpu);navigator.gpu.requestAdapter=async options=>{const adapter=await request(options);if(adapter){const create=adapter.requestDevice.bind(adapter);adapter.requestDevice=async descriptor=>{const device=await create(descriptor);activeDevice=device;device.lost.then(info=>{if(activeDevice===device&&ready&&renderer==='gpu')recoverCPU('GPU device lost: '+info.reason);});device.addEventListener('uncapturederror',event=>{event.preventDefault();if(activeDevice===device&&ready&&renderer==='gpu')recoverCPU('GPU error');});return device;};}return adapter;};}
let dprQuery;function watchDPR(){dprQuery?.removeEventListener('change',changedDPR);dprQuery=matchMedia(`(resolution: ${devicePixelRatio}dppx)`);dprQuery.addEventListener('change',changedDPR,{once:true});}function changedDPR(){watchDPR();resized();}watchDPR();
document.addEventListener('visibilitychange',()=>{if(!document.hidden)schedule();});
window.addEventListener('resize',resized);
window.addEventListener('dragover',e=>{if(ready)e.preventDefault();});
window.addEventListener('drop',e=>{if(ready)projection.drop(null,e);});
function wheel(node,dy,dx=0,x=0,y=0,event){
 if(node&&projection.event(node,'wheel',{delta_y:dy,delta_x:dx,x,y},event))return;
 if(!node?.dataset.scrollNode||!app.scroll_node(Number(node.dataset.scrollNode),dy))app.scroll(dy);
 schedule();
}
window.addEventListener('wheel',e=>{if(!ready)return;const node=e.target.closest?.('[data-node]');
 if(e.ctrlKey||e.metaKey){if(node?.getAttribute('role')==='img'){e.preventDefault();app.canvas_zoom(Number(node.dataset.node),e.clientX,e.clientY,Math.exp(-e.deltaY*0.01));schedule();}return;}
 wheel(node,e.deltaY,e.deltaX,e.clientX,e.clientY,e);e.preventDefault();
},{passive:false});
let touch;window.addEventListener('touchstart',e=>{touch={y:e.touches[0]?.clientY,node:e.target.closest?.('[data-node]')};},{passive:true});
window.addEventListener('touchmove',e=>{const y=e.touches[0]?.clientY;if(ready&&y!==undefined&&touch?.y!==undefined){wheel(touch.node,touch.y-y,0,e.touches[0].clientX,y,e);touch.y=y;e.preventDefault();}},{passive:false});
window.addEventListener('pagehide',()=>{network.dispose();services.dispose();projection.dispose();});
try{await init();app.init_app(JSON.stringify(network.capabilities()));app.service_capabilities(JSON.stringify(services.capabilities()));const requested=new URLSearchParams(location.search).get('renderer')||'auto';let reason='';
 if(requested!=='cpu'&&navigator.gpu){try{await app.init_gpu(canvas);renderer='gpu';}catch(e){reason='GPU initialization failed; software mode active.';const replacement=canvas.cloneNode();canvas.replaceWith(replacement);canvas=replacement;}}
 if(renderer==='cpu'){ctx=canvas.getContext('2d');if(!ctx)throw new Error('Canvas context unavailable');}ready=true;status.textContent=renderer==='gpu'?'RustX · WebGPU · Stage 3':`RustX · Rust CPU renderer · Stage 3${reason?' · '+reason:''}`;
 window.rustx={inspect:()=>JSON.parse(app.inspect()),semantics:()=>JSON.parse(app.semantics()),renderer:()=>renderer,draws:()=>draws,retryGPU,destroyGPU:()=>app.destroy_gpu(),setTheme:light=>{app.set_theme(light);schedule();}};schedule();
 resized();if(reason)resourceError('renderer','initialization_failed');
}catch(e){fatal(e);}

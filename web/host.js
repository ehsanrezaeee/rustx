import init,* as app from './pkg/rustx_lab.js';
import {HostNetwork} from './services.js';
let canvas=document.querySelector('#surface');const root=document.querySelector('#semantic'),status=document.querySelector('#status');
let activeDevice=null,queued=false,renderer='cpu',ctx,ready=false,draws=0;const elements=new Map();
const network=new HostNetwork((id,status,body,error,version)=>{app.response(id,status,body,error,version);pump();schedule();});
function fatal(e){document.querySelector('#fatal').textContent=`RustX could not continue: ${String(e)}. Reload to restart.`;console.error(e);}
function schedule(){if(!queued&&ready&&!document.hidden){queued=true;requestAnimationFrame(draw);}}
function flatten(node,out=[],owner=null){node.owner=owner;out.push(node);if(['list','listitem'].includes(node.role))owner=node.id;for(const child of node.children)flatten(child,out,owner);return out;}
function reveal(id){const n=JSON.parse(app.inspect()).nodes.find(x=>x.node===Number(id));const r=n?.scroll_parent?JSON.parse(app.inspect()).nodes.find(x=>x.node===n.scroll_parent)?.rect:n?.rect;if(r){if(r[1]<0){app.scroll(r[1]-16);schedule();}else if(r[1]+r[3]>innerHeight){app.scroll(r[1]+r[3]-innerHeight+16);schedule();}}}
function sync(){const inspection=JSON.parse(app.inspect()),boxes=new Map(inspection.nodes.map(n=>[n.node,n])),seen=new Set(),ownership=new Map();
 for(const n of flatten(JSON.parse(app.semantics()))){if(!['button','textbox','text','list','listitem','img'].includes(n.role))continue;const geometry=boxes.get(n.id);if(!geometry)continue;const rect=geometry.rect;seen.add(n.id);let el=elements.get(n.id);const tag=n.role==='button'?'button':n.role==='textbox'?'input':'span';
  if(!el){el=document.createElement(tag);el.dataset.node=n.id;el.id=`rustx-${n.id}`;elements.set(n.id,el);root.append(el);
   if(tag==='button'){el.type='button';el.addEventListener('click',()=>{app.activate(n.id);pump();schedule();});}
   if(n.role==='img'){el.tabIndex=0;el.style.pointerEvents='auto';let pointer;el.addEventListener('pointerdown',e=>{pointer=[e.clientX,e.clientY,false];el.setPointerCapture(e.pointerId);el.focus({preventScroll:true});e.preventDefault();});el.addEventListener('pointermove',e=>{if(!pointer)return;const dx=e.clientX-pointer[0],dy=e.clientY-pointer[1];if(dx||dy){app.canvas_pan(n.id,dx,dy);pointer=[e.clientX,e.clientY,true];schedule();}});el.addEventListener('pointerup',e=>{if(pointer&&!pointer[2])app.canvas_pick(n.id,e.clientX,e.clientY);pointer=null;schedule();});el.addEventListener('pointercancel',()=>{pointer=null;});el.addEventListener('keydown',e=>{const d={ArrowLeft:[40,0],ArrowRight:[-40,0],ArrowUp:[0,40],ArrowDown:[0,-40]}[e.key];if(d){e.preventDefault();app.canvas_pan(n.id,...d);schedule();}});}
   if(tag==='input'){el.type='text';el.autocomplete='off';el.spellcheck=false;el.addEventListener('input',e=>{app.input(n.id,el.value,e.isComposing);schedule();});el.addEventListener('compositionend',()=>{app.input(n.id,el.value,false);schedule();});}
   if(tag!=='span')el.addEventListener('focus',()=>{app.focus(n.id);reveal(n.id);schedule();});else el.className='text';
  }
  if(n.app_id)el.dataset.testid=n.app_id;
  el.dataset.scrollNode=geometry.scroll_parent||'';if(n.owner){const list=ownership.get(n.owner)||[];list.push(el.id);ownership.set(n.owner,list);}
  const [vx,vy,vw,vh]=geometry.visible_rect;const [rx,ry,rw,rh]=rect;el.style.clipPath=vw>0&&vh>0?`inset(${Math.max(0,vy-ry)}px ${Math.max(0,rx+rw-vx-vw)}px ${Math.max(0,ry+rh-vy-vh)}px ${Math.max(0,vx-rx)}px)`: 'inset(100%)';
  if(n.role==='img'){el.setAttribute('role','img');el.setAttribute('aria-label',n.label);}
  if(['list','listitem'].includes(n.role)){el.setAttribute('role',n.role);el.setAttribute('aria-label',n.label);if(n.position){el.setAttribute('aria-posinset',n.position[0]);el.setAttribute('aria-setsize',n.position[1]);}}
  if(n.role==='list')el.style.pointerEvents='auto';
  if(tag==='input'){if(el.value!==n.value){const a=el.selectionStart,b=el.selectionEnd;el.value=n.value;if(document.activeElement===el)el.setSelectionRange(Math.min(a??0,el.value.length),Math.min(b??0,el.value.length));}el.setAttribute('aria-label',n.label);el.dir='auto';}else if(el.textContent!==n.label)el.textContent=n.label;
  const[x,y,w,h]=rect;Object.assign(el.style,{left:`${x}px`,top:`${y}px`,width:`${Math.max(1,w)}px`,height:`${Math.max(1,h)}px`});
 }
 for(const[id,el]of elements){const owned=ownership.get(id);if(owned)el.setAttribute('aria-owns',owned.join(' '));else el.removeAttribute('aria-owns');}
 for(const[id,el]of elements)if(!seen.has(id)){el.remove();elements.delete(id);}
 const order=inspection.nodes.map(n=>elements.get(n.node)).filter(Boolean);
 // Move siblings around the active element so keyed reorder never detaches
 // its native editing session, selection, or in-progress composition.
 const active=order.indexOf(document.activeElement);
 const previousActive=[...root.children].indexOf(document.activeElement);
 function arrange(items,anchor){for(let i=items.length-1;i>=0;i--){const el=items[i];if(el.nextElementSibling!==anchor)root.insertBefore(el,anchor);anchor=el;}}
 if(active>=0){arrange(order.slice(0,active),order[active]);arrange(order.slice(active+1),null);}else arrange(order,null);
 if(active>=0&&active!==previousActive)reveal(document.activeElement.dataset.node);
}
// Normalize the framework's declared focus order, including on macOS browsers
// whose native Tab preference skips buttons. Let Tab leave at either boundary.
document.addEventListener('keydown',e=>{if(!ready||e.key!=='Tab'||e.altKey||e.ctrlKey||e.metaKey)return;if(document.activeElement!==document.body&&!root.contains(document.activeElement))return;
 const controls=[...root.querySelectorAll('button,input,[role=img]')],index=controls.indexOf(document.activeElement),next=index<0?(e.shiftKey?controls.length-1:0):index+(e.shiftKey?-1:1);
 if(next>=0&&next<controls.length){e.preventDefault();controls[next].focus({preventScroll:true});}
});
function pump(){network.accept(JSON.parse(app.commands()));}
function draw(){queued=false;try{const w=innerWidth,h=innerHeight;if(w<=0||h<=0)return;const requested=Math.min(devicePixelRatio||1,2),limit=Math.min(8192/w,8192/h,Math.sqrt(16777216/(w*h))),dpr=Math.min(requested,limit<requested?limit*.999:requested),pw=Math.round(w*dpr),ph=Math.round(h*dpr);if(canvas.width!==pw)canvas.width=pw;if(canvas.height!==ph)canvas.height=ph;
 if(app.frame(w,h,dpr)){draws++;if(renderer==='cpu')ctx.putImageData(new ImageData(new Uint8ClampedArray(app.pixels()),pw,ph),0,0);sync();}pump();if(app.needs_update())schedule();}catch(e){if(renderer==='gpu'){recoverCPU(String(e));}else fatal(e);}}
function recoverCPU(reason){const device=activeDevice;activeDevice=null;app.drop_gpu();device?.destroy();const replacement=canvas.cloneNode();canvas.replaceWith(replacement);canvas=replacement;ctx=canvas.getContext('2d');if(!ctx){fatal('CPU canvas unavailable');return;}renderer='cpu';status.textContent='RustX · CPU recovery · '+reason;schedule();}
async function retryGPU(){if(!navigator.gpu)throw new Error('WebGPU unavailable');const replacement=canvas.cloneNode();canvas.replaceWith(replacement);canvas=replacement;try{await app.init_gpu(canvas);renderer='gpu';app.invalidate();status.textContent='RustX · WebGPU · Stage 2';schedule();}catch(error){recoverCPU('GPU initialization failed');throw error;}}
// Observe actual browser device loss without owning geometry, state or layout.
if(navigator.gpu){const request=navigator.gpu.requestAdapter.bind(navigator.gpu);navigator.gpu.requestAdapter=async options=>{const adapter=await request(options);if(adapter){const create=adapter.requestDevice.bind(adapter);adapter.requestDevice=async descriptor=>{const device=await create(descriptor);activeDevice=device;device.lost.then(info=>{if(activeDevice===device&&ready&&renderer==='gpu')recoverCPU('GPU device lost: '+info.reason);});device.addEventListener('uncapturederror',event=>{event.preventDefault();if(activeDevice===device&&ready&&renderer==='gpu')recoverCPU('GPU error');});return device;};}return adapter;};}
let dprQuery;function watchDPR(){dprQuery?.removeEventListener('change',changedDPR);dprQuery=matchMedia(`(resolution: ${devicePixelRatio}dppx)`);dprQuery.addEventListener('change',changedDPR,{once:true});}function changedDPR(){watchDPR();schedule();}watchDPR();
document.addEventListener('visibilitychange',()=>{if(!document.hidden)schedule();});
window.addEventListener('resize',schedule);window.addEventListener('wheel',e=>{if(!ready)return;e.preventDefault();const node=e.target.closest?.('[data-node]');if(node?.getAttribute('role')==='img'&&(e.ctrlKey||e.metaKey)){app.canvas_zoom(Number(node.dataset.node),e.clientX,e.clientY,Math.exp(-e.deltaY*0.01));}else if(!node?.dataset.scrollNode||!app.scroll_node(Number(node.dataset.scrollNode),e.deltaY))app.scroll(e.deltaY);schedule();},{passive:false});
let touchY;window.addEventListener('touchstart',e=>{touchY=e.touches[0]?.clientY;},{passive:true});window.addEventListener('touchmove',e=>{const y=e.touches[0]?.clientY;if(ready&&y!==undefined&&touchY!==undefined){app.scroll(touchY-y);touchY=y;schedule();e.preventDefault();}},{passive:false});window.addEventListener('pagehide',()=>network.dispose());
try{await init();app.init_app(JSON.stringify(network.capabilities()));const requested=new URLSearchParams(location.search).get('renderer')||'auto';let reason='';
 if(requested!=='cpu'&&navigator.gpu){try{await app.init_gpu(canvas);renderer='gpu';}catch(e){reason='GPU initialization failed; software mode active.';const replacement=canvas.cloneNode();canvas.replaceWith(replacement);canvas=replacement;}}
 if(renderer==='cpu'){ctx=canvas.getContext('2d');if(!ctx)throw new Error('Canvas context unavailable');}ready=true;status.textContent=renderer==='gpu'?'RustX · WebGPU · Stage 2':`RustX · Rust CPU renderer · Stage 2${reason?' · '+reason:''}`;
 window.rustx={inspect:()=>JSON.parse(app.inspect()),semantics:()=>JSON.parse(app.semantics()),renderer:()=>renderer,draws:()=>draws,retryGPU,destroyGPU:()=>app.destroy_gpu(),setTheme:light=>{app.set_theme(light);schedule();}};schedule();
}catch(e){fatal(e);}

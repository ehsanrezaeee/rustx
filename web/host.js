import init,* as app from './pkg/rustx_lab.js';
import {HostNetwork} from './services.js';
let canvas=document.querySelector('#surface');const root=document.querySelector('#semantic'),status=document.querySelector('#status');
let queued=false,renderer='cpu',ctx,ready=false,draws=0;const elements=new Map();
const network=new HostNetwork((id,status,body,error,version)=>{app.response(id,status,body,error,version);pump();schedule();});
function fatal(e){document.querySelector('#fatal').textContent=`RustX could not continue: ${String(e)}. Reload to restart.`;console.error(e);}
function schedule(){if(!queued&&ready){queued=true;requestAnimationFrame(draw);}}
function flatten(node,out=[]){out.push(node);for(const child of node.children)flatten(child,out);return out;}
function reveal(id){const r=JSON.parse(app.inspect()).nodes.find(x=>x.node===Number(id))?.rect;if(r){if(r[1]<0){app.scroll(r[1]-16);schedule();}else if(r[1]+r[3]>innerHeight){app.scroll(r[1]+r[3]-innerHeight+16);schedule();}}}
function sync(){const inspection=JSON.parse(app.inspect()),boxes=new Map(inspection.nodes.map(n=>[n.node,n.rect])),seen=new Set();
 for(const n of flatten(JSON.parse(app.semantics()))){if(!['button','textbox','text'].includes(n.role))continue;const rect=boxes.get(n.id);if(!rect)continue;seen.add(n.id);let el=elements.get(n.id);const tag=n.role==='button'?'button':n.role==='textbox'?'input':'span';
  if(!el){el=document.createElement(tag);el.dataset.node=n.id;elements.set(n.id,el);root.append(el);
   if(tag==='button'){el.type='button';el.addEventListener('click',()=>{app.activate(n.id);pump();schedule();});}
   if(tag==='input'){el.type='text';el.autocomplete='off';el.spellcheck=false;el.addEventListener('input',e=>{app.input(n.id,el.value,e.isComposing);schedule();});el.addEventListener('compositionend',()=>{app.input(n.id,el.value,false);schedule();});}
   if(tag!=='span')el.addEventListener('focus',()=>{app.focus(n.id);reveal(n.id);schedule();});else el.className='text';
  }
  if(n.app_id)el.dataset.testid=n.app_id;
  if(tag==='input'){if(el.value!==n.value){const a=el.selectionStart,b=el.selectionEnd;el.value=n.value;if(document.activeElement===el)el.setSelectionRange(Math.min(a??0,el.value.length),Math.min(b??0,el.value.length));}el.setAttribute('aria-label',n.label);el.dir='auto';}else if(el.textContent!==n.label)el.textContent=n.label;
  const[x,y,w,h]=rect;Object.assign(el.style,{left:`${x}px`,top:`${y}px`,width:`${Math.max(1,w)}px`,height:`${Math.max(1,h)}px`});
 }
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
 const controls=[...root.querySelectorAll('button,input')],index=controls.indexOf(document.activeElement),next=index<0?(e.shiftKey?controls.length-1:0):index+(e.shiftKey?-1:1);
 if(next>=0&&next<controls.length){e.preventDefault();controls[next].focus({preventScroll:true});}
});
function pump(){network.accept(JSON.parse(app.commands()));}
function draw(){queued=false;try{const dpr=Math.min(devicePixelRatio||1,2),w=innerWidth,h=innerHeight,pw=Math.round(w*dpr),ph=Math.round(h*dpr);if(canvas.width!==pw)canvas.width=pw;if(canvas.height!==ph)canvas.height=ph;
 if(app.frame(w,h,dpr)){draws++;if(renderer==='cpu')ctx.putImageData(new ImageData(new Uint8ClampedArray(app.pixels()),pw,ph),0,0);sync();}pump();if(app.needs_update())schedule();}catch(e){fatal(e);}}
window.addEventListener('resize',schedule);window.addEventListener('wheel',e=>{if(!ready)return;e.preventDefault();app.scroll(e.deltaY);schedule();},{passive:false});
let touchY;window.addEventListener('touchstart',e=>{touchY=e.touches[0]?.clientY;},{passive:true});window.addEventListener('touchmove',e=>{const y=e.touches[0]?.clientY;if(ready&&y!==undefined&&touchY!==undefined){app.scroll(touchY-y);touchY=y;schedule();e.preventDefault();}},{passive:false});window.addEventListener('pagehide',()=>network.dispose());
try{await init();app.init_app(JSON.stringify(network.capabilities()));const requested=new URLSearchParams(location.search).get('renderer')||'auto';let reason='';
 if(requested!=='cpu'&&navigator.gpu){try{await app.init_gpu(canvas);renderer='gpu';}catch(e){reason='GPU initialization failed; software mode active.';const replacement=canvas.cloneNode();canvas.replaceWith(replacement);canvas=replacement;}}
 if(renderer==='cpu'){ctx=canvas.getContext('2d');if(!ctx)throw new Error('Canvas context unavailable');}ready=true;status.textContent=renderer==='gpu'?'RustX · WebGPU · Stage 1':`RustX · Rust CPU renderer · Stage 1${reason?' · '+reason:''}`;
 window.rustx={inspect:()=>JSON.parse(app.inspect()),semantics:()=>JSON.parse(app.semantics()),renderer:()=>renderer,draws:()=>draws};schedule();
}catch(e){fatal(e);}

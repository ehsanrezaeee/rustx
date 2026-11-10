// Version 1 envelope with bounded HTTP and scoped capability adapters.
export const HOST_VERSION = 1;
export const HOST_LIMITS = Object.freeze({max_requests:16,max_response_bytes:1_048_576,timeout_ms:10_000});
export class HostNetwork {
  constructor(complete, fetcher = globalThis.fetch, base = globalThis.location?.href) {
    this.complete = complete;
    this.fetcher = fetcher;
    this.base = base;
    this.active = new Map();
    this.disposed = false;
  }
  capabilities() {
    return {version:HOST_VERSION,http_get:typeof this.fetcher === 'function'?'supported':'unavailable',
      http_cancel:typeof AbortController === 'function'?'supported':'unavailable',http_methods:true,...HOST_LIMITS};
  }
  validate(command) {
    if (!command || command.version !== HOST_VERSION) throw new Error('Host protocol version mismatch');
    if (!Number.isSafeInteger(command.id) || command.id < 1) throw new Error('Invalid host request ID');
    if (!['Get','Request','Cancel'].includes(command.kind)) throw new Error('Unsupported host command');
    if (['Get','Request'].includes(command.kind) && (typeof command.url !== 'string' || !command.url.length || new TextEncoder().encode(command.url).length > 8192 ||
      !Number.isInteger(command.max_bytes) || command.max_bytes < 1 || command.max_bytes > HOST_LIMITS.max_response_bytes ||
      !Number.isInteger(command.timeout_ms) || command.timeout_ms < 1 || command.timeout_ms > HOST_LIMITS.timeout_ms)) {
      throw new Error('Invalid host request limits');
    }
    if (command.kind === 'Request' && (!['GET','HEAD','POST','PUT','PATCH','DELETE','OPTIONS'].includes(command.method) || !Array.isArray(command.headers) || command.headers.length>32 || !Array.isArray(command.body) || command.body.length>HOST_LIMITS.max_response_bytes || !['omit','same-origin','include'].includes(command.credentials))) throw new Error('Invalid HTTP request');
    if (command.kind === 'Request' && command.headers.some(h=>!Array.isArray(h)||h.length!==2||typeof h[0]!=='string'||typeof h[1]!=='string'||!/^[-!#$%&'*+.^_`|~A-Za-z0-9]+$/.test(h[0])||/[\r\n\0]/.test(h[1]))) throw new Error('Invalid HTTP headers');
    if(command.kind==='Request'&&(command.body.some(b=>!Number.isInteger(b)||b<0||b>255)||(command.body.length&&['GET','HEAD'].includes(command.method))))throw new Error('Invalid HTTP body');
  }
  accept(commands) {
    if (!Array.isArray(commands) || commands.length > 32) throw new Error('Host command batch exceeds limit');
    for (const command of commands) this.validate(command);
    for (const command of commands) {
      if (command.kind !== 'Cancel') void this.request(command);
      else this.active.get(command.id)?.abort();
    }
  }
  async request(command) {
    this.validate(command.kind ? command : {...command,kind:'Get'});
    if (this.disposed) return;
    if (this.active.has(command.id)) throw new Error('Duplicate active host request ID');
    if (this.active.size >= HOST_LIMITS.max_requests) {
      this.complete(command.id,0,new Uint8Array(),'busy',HOST_VERSION);
      return;
    }
    const controller = new AbortController();
    this.active.set(command.id,controller);
    let error='',status=0,body=new Uint8Array(),headers=[];
    const timer=setTimeout(()=>{error='timeout';controller.abort();},command.timeout_ms);
    try {
      const url=this.base?new URL(command.url,this.base):command.url;
      const response=await this.fetcher.call(globalThis,url,{signal:controller.signal,credentials:command.credentials||'same-origin', method:command.method||'GET', headers:command.headers||[], body:command.body?.length?new Uint8Array(command.body):undefined});
      const reader=response.body?.getReader(),chunks=[];
      let size=0;
      if (reader) {
        try {
          for (;;) {
            const {done,value}=await reader.read();
            if (done) break;
            size+=value.byteLength;
            if (size>command.max_bytes) {error='too_large';controller.abort();await reader.cancel();throw new Error(error);}
            chunks.push(value);
          }
        } finally {reader.releaseLock();}
      }
      if (controller.signal.aborted) throw new Error('Aborted');
      body=new Uint8Array(size);
      let offset=0;
      for (const chunk of chunks) {body.set(chunk,offset);offset+=chunk.length;}
      status=response.status;
      headers=Array.from(response.headers.entries());
      if(headers.length>64||headers.reduce((sum,[k,v])=>sum+k.length+v.length,0)>32_768){error='too_large';throw new Error(error);}
    } catch {
      error=error||(controller.signal.aborted?'cancelled':'Network request failed');
    } finally {
      clearTimeout(timer);
      this.active.delete(command.id);
    }
    if (!this.disposed) this.complete(command.id,status,body,error,HOST_VERSION,headers);
  }
  dispose() {
    this.disposed=true;
    for (const controller of this.active.values()) controller.abort();
  }
}

export const SERVICE_VERSION=1;
const SERVICE_BYTES=1_048_576, CHUNK_BYTES=65_536;
const serviceKinds=new Set(['timer','cancel','clipboard_read','clipboard_write','file_open','file_download','storage_get','storage_set','storage_remove','watch_location','navigate','history','stream_open','stream_read','socket_open','socket_send','socket_close','print']);
function boundedString(value,max=SERVICE_BYTES) { if(typeof value!=='string'||new TextEncoder().encode(value).length>max) throw Object.assign(new Error('Value exceeds service limit'),{serviceKind:'too_large'}); return value; }
function boundedBytes(value,max=SERVICE_BYTES) { if(!Array.isArray(value)||value.length>max||value.some(v=>!Number.isInteger(v)||v<0||v>255)) throw Object.assign(new Error('Invalid byte buffer'),{serviceKind:'too_large'}); return new Uint8Array(value); }
function serviceError(error) {
  const kind=error?.serviceKind||(error?.name==='NotAllowedError'||error?.name==='SecurityError'?'denied':error?.name==='QuotaExceededError'?'quota':error?.name==='AbortError'?'cancelled':error?.name==='NotSupportedError'?'unavailable':'failed');
  return kind==='failed'?{kind,message:'Host service failed'}:{kind};
}
/** Platform-only adapters. No layout, application state or renderer code belongs here. */
export class HostServices {
  constructor(complete, env=globalThis) {
    this.complete=complete; this.env=env; this.resources=new Map(); this.disposed=false;
    this.locationChanged=()=>{for(const [id,r] of this.resources) if(r.kind==='location') this.result(id,{type:'location',url:this.env.location.href});};
    env.addEventListener?.('popstate',this.locationChanged); env.addEventListener?.('hashchange',this.locationChanged);
  }
  capabilities() {
    const e=this.env; const supported=v=>v?'supported':'unavailable';
    return {version:SERVICE_VERSION,clipboard:supported(e.navigator?.clipboard),files:supported(e.document&&e.Blob),storage:supported(e.indexedDB),navigation:supported(e.history),websocket:supported(e.WebSocket),stream:supported(e.fetch&&e.ReadableStream),print:supported(e.open),scheduling:supported(e.setTimeout),max_bytes:SERVICE_BYTES,max_resources:32};
  }
  validate(c) {
    if(!c||c.version!==SERVICE_VERSION||!Number.isSafeInteger(c.id)||c.id<0||!serviceKinds.has(c.kind)) throw new Error('Invalid service command');
    if(c.id===0&&!['cancel','stream_read','socket_send','socket_close'].includes(c.kind)) throw new Error('Missing service request ID');
    if(['cancel','stream_read','socket_send','socket_close'].includes(c.kind)&&(!Number.isSafeInteger(c.resource)||c.resource<1)) throw new Error('Invalid service resource');
  }
  accept(commands) {
    if(!Array.isArray(commands)||commands.length>64) throw new Error('Service batch exceeds limit');
    commands.forEach(c=>this.validate(c));
    for(const c of commands) { if(this.disposed) break; void this.execute(c); }
  }
  result(id,value) { if(!this.disposed) this.complete(id,JSON.stringify({value}),SERVICE_VERSION); }
  error(id,error) { if(!this.disposed) this.complete(id,JSON.stringify({error:serviceError(error)}),SERVICE_VERSION); }
  unavailable() { throw Object.assign(new Error('Unavailable'),{serviceKind:'unavailable'}); }
  admit(id,resource) {
    if(this.resources.has(id)) throw Object.assign(new Error('Duplicate resource'),{serviceKind:'invalid'});
    if(this.resources.size>=32) throw Object.assign(new Error('Busy'),{serviceKind:'busy'});
    this.resources.set(id,resource); return resource;
  }
  async database() {
    if(!this.env.indexedDB) this.unavailable();
    return new Promise((resolve,reject)=>{
      const req=this.env.indexedDB.open('rustx-state',1);
      req.onupgradeneeded=()=>req.result.createObjectStore('records');
      req.onsuccess=()=>resolve(req.result); req.onerror=()=>reject(req.error);
      req.onblocked=()=>reject(Object.assign(new Error('Storage blocked'),{serviceKind:'failed'}));
    });
  }
  async storage(c) {
    boundedString(c.key,1024); if(!c.key.length) throw Object.assign(new Error('Empty key'),{serviceKind:'invalid'});
    if(c.kind==='storage_set') { boundedString(c.value); if(!Number.isInteger(c.schema_version)||c.schema_version<1) throw Object.assign(new Error('Invalid schema'),{serviceKind:'invalid'}); }
    const db=await this.database();
    try { return await new Promise((resolve,reject)=>{
      const tx=db.transaction('records',c.kind==='storage_get'?'readonly':'readwrite'),store=tx.objectStore('records'); let record;
      const req=c.kind==='storage_get'?store.get(c.key):c.kind==='storage_set'?store.put({version:c.schema_version,value:c.value},c.key):store.delete(c.key);
      req.onsuccess=()=>record=req.result; tx.oncomplete=()=>resolve(c.kind==='storage_get'?(record?{type:'storage',...record}:{type:'missing'}):{type:'unit'}); tx.onerror=()=>reject(tx.error); tx.onabort=()=>reject(tx.error);
    }); } finally {db.close();}
  }
  async execute(c) {
    const e=this.env; let target=c.id||c.resource;
    try {
      switch(c.kind) {
        case 'timer': {
          if(!Number.isInteger(c.delay_ms)||c.delay_ms<1||c.delay_ms>60_000)throw {serviceKind:'invalid'};
          const r=this.admit(c.id,{kind:'timer'});r.timer=e.setTimeout(()=>{if(this.resources.get(c.id)===r){this.resources.delete(c.id);this.result(c.id,{type:'unit'});}},c.delay_ms);return;
        }
        case 'cancel': this.cancel(c.resource); return;
        case 'clipboard_read': {
          if(!e.navigator?.clipboard?.readText) this.unavailable();
          const text=await e.navigator.clipboard.readText(); boundedString(text); this.result(c.id,{type:'text',text}); return;
        }
        case 'clipboard_write': {
          boundedString(c.text); if(!e.navigator?.clipboard?.writeText) this.unavailable();
          await e.navigator.clipboard.writeText(c.text); this.result(c.id,{type:'unit'}); return;
        }
        case 'storage_get': case 'storage_set': case 'storage_remove': this.result(c.id,await this.storage(c)); return;
        case 'watch_location': {
          if(!e.location) this.unavailable(); this.admit(c.id,{kind:'location'}); this.result(c.id,{type:'location',url:e.location.href}); return;
        }
        case 'navigate': {
          boundedString(c.url,8192); if(!e.history||!e.location) this.unavailable();
          const url=new URL(c.url,e.location.href); if(url.origin!==e.location.origin) throw Object.assign(new Error('External navigation denied'),{serviceKind:'denied'});
          if(c.hash) {const route=c.url.startsWith('#')?c.url.slice(1):c.url;const base=new URL(e.location.href);base.hash=route;e.history[c.replace?'replaceState':'pushState'](null,'',base);}
          else e.history[c.replace?'replaceState':'pushState'](null,'',url);
          this.locationChanged(); this.result(c.id,{type:'location',url:e.location.href}); return;
        }
        case 'history': if(!Number.isInteger(c.delta)||Math.abs(c.delta)>100) throw Object.assign(new Error('Invalid history step'),{serviceKind:'invalid'}); if(!e.history) this.unavailable(); e.history.go(c.delta); this.result(c.id,{type:'unit'}); return;
        case 'file_open': {
          if(!e.document) this.unavailable(); boundedString(c.accept,1024);
          if(!Number.isInteger(c.max_bytes)||c.max_bytes<1||c.max_bytes>SERVICE_BYTES) throw Object.assign(new Error('Invalid file limit'),{serviceKind:'invalid'});
          const resource=this.admit(c.id,{kind:'file'});
          const input=e.document.createElement('input');resource.cancel=()=>input.remove(); input.type='file'; input.accept=c.accept;
          input.style.position='fixed';input.style.opacity='0';input.style.width='1px';input.style.height='1px';input.setAttribute('aria-label','Select a file');e.document.body.append(input);
          input.oncancel=()=>{this.resources.delete(c.id);input.remove();this.error(c.id,{serviceKind:'cancelled'});};
          input.onchange=async()=>{
            try { const file=input.files?.[0]; if(!file) throw {serviceKind:'cancelled'};if(file.size>c.max_bytes) throw {serviceKind:'too_large'};
              const bytes=Array.from(new Uint8Array(await file.arrayBuffer()));if(this.resources.get(c.id)===resource)this.result(c.id,{type:'file',name:file.name,mime:file.type||'application/octet-stream',bytes});
            } catch(error) {if(this.resources.get(c.id)===resource)this.error(c.id,error);} finally {input.remove();this.resources.delete(c.id);}
          }; input.click(); return;
        }
        case 'file_download': {
          if(!e.document||!e.URL?.createObjectURL) this.unavailable();boundedString(c.name,256);boundedString(c.mime,256);
          if(!c.name||/[\r\n/\\]/.test(c.name)) throw {serviceKind:'invalid'};
          const bytes=boundedBytes(c.bytes),blob=new e.Blob([bytes],{type:c.mime}),url=e.URL.createObjectURL(blob),a=e.document.createElement('a');a.href=url;a.download=c.name;e.document.body.append(a);a.click();a.remove();e.setTimeout(()=>e.URL.revokeObjectURL(url),0);this.result(c.id,{type:'unit'}); return;
        }
        case 'stream_open': {
          boundedString(c.url,8192); if(!e.fetch) this.unavailable();
          if(!Number.isInteger(c.max_bytes)||c.max_bytes<1||c.max_bytes>SERVICE_BYTES||!Number.isInteger(c.timeout_ms)||c.timeout_ms<1||c.timeout_ms>10_000) throw {serviceKind:'invalid'};
          const controller=new e.AbortController(),r=this.admit(c.id,{kind:'stream',controller,received:0,max:c.max_bytes,reader:null,tail:null,reading:false});
          r.timer=e.setTimeout(()=>{if(this.resources.get(c.id)===r){this.cancel(c.id);this.error(c.id,{serviceKind:'failed'});}},c.timeout_ms);
          const response=await e.fetch(new URL(c.url,e.location?.href),{credentials:'same-origin',signal:controller.signal});
          if(this.resources.get(c.id)!==r)return;r.reader=response.body?.getReader();r.total=Number(response.headers.get('content-length'))||null;
          this.result(c.id,{type:'stream',status:response.status,headers:Array.from(response.headers.entries())}); return;
        }
        case 'stream_read': {
          const r=this.resources.get(c.resource);if(!r||r.kind!=='stream'||r.reading) throw {serviceKind:'invalid'};r.reading=true;
          const chunk=r.tail?{value:r.tail,done:false}:r.reader?await r.reader.read():{done:true};r.tail=null;
          if(this.resources.get(c.resource)!==r)return;
          if(chunk.done){e.clearTimeout(r.timer);r.reader?.releaseLock();this.resources.delete(c.resource);this.result(c.resource,{type:'chunk',bytes:[],done:true,received:r.received,total:r.total});return;}
          if(r.received+chunk.value.byteLength>r.max){this.cancel(c.resource);throw {serviceKind:'too_large'};}
          const bytes=chunk.value.subarray(0,CHUNK_BYTES);if(chunk.value.length>CHUNK_BYTES)r.tail=chunk.value.subarray(CHUNK_BYTES);
          r.received+=bytes.length;r.reading=false;this.result(c.resource,{type:'chunk',bytes:Array.from(bytes),done:false,received:r.received,total:r.total});return;
        }
        case 'socket_open': {
          if(!e.WebSocket)this.unavailable();boundedString(c.url,8192);if(!Array.isArray(c.protocols)||c.protocols.length>8||c.protocols.some(v=>typeof v!=='string'||v.length>128))throw {serviceKind:'invalid'};
          const url=new URL(c.url,e.location?.href);if(c.url.startsWith('/'))url.protocol=url.protocol==='https:'?'wss:':'ws:';if(!['ws:','wss:'].includes(url.protocol))throw {serviceKind:'invalid'};
          const r=this.admit(c.id,{kind:'socket'}),socket=new e.WebSocket(url,c.protocols);r.socket=socket;socket.binaryType='arraybuffer';
          socket.onopen=()=>this.result(c.id,{type:'socket_open'});
          socket.onmessage=event=>{const text=typeof event.data==='string',bytes=text?new TextEncoder().encode(event.data):new Uint8Array(event.data);if(bytes.length>CHUNK_BYTES){this.cancel(c.id);this.error(c.id,{serviceKind:'too_large'});return;}this.result(c.id,{type:'socket_message',text,bytes:Array.from(bytes)});};
          socket.onerror=()=>{this.cancel(c.id);this.error(c.id,{serviceKind:'failed'});};
          socket.onclose=event=>{if(this.resources.get(c.id)===r){this.resources.delete(c.id);this.result(c.id,{type:'socket_closed',code:event.code,reason:event.reason,clean:event.wasClean});}};return;
        }
        case 'socket_send': {
          const r=this.resources.get(c.resource);if(!r||r.kind!=='socket'||r.socket.readyState!==1)throw {serviceKind:'invalid'};
          const bytes=boundedBytes(c.bytes,CHUNK_BYTES);if(r.socket.bufferedAmount+bytes.length>262_144)throw {serviceKind:'busy'};
          r.socket.send(c.text?new TextDecoder('utf-8',{fatal:true}).decode(bytes):bytes);return;
        }
        case 'socket_close': {const r=this.resources.get(c.resource);if(!r||r.kind!=='socket')throw {serviceKind:'invalid'};boundedString(c.reason,123);if(c.code!==1000&&!(c.code>=3000&&c.code<=4999))throw {serviceKind:'invalid'};r.socket.close(c.code,c.reason);return;}
        case 'print': {
          boundedString(c.title,256);boundedString(c.text);if(!e.open)this.unavailable();const popup=e.open('','_blank','popup');if(!popup)throw {serviceKind:'denied'};
          popup.document.title=c.title;const h=popup.document.createElement('h1'),pre=popup.document.createElement('pre');h.textContent=c.title;pre.textContent=c.text;pre.style.whiteSpace='pre-wrap';pre.style.font='16px sans-serif';popup.document.body.replaceChildren(h,pre);popup.print();this.result(c.id,{type:'unit'});return;
        }
      }
    } catch(error) {this.cancel(target);this.error(target,error);}
  }
  cancel(id) {
    const r=this.resources.get(id);if(!r)return;this.resources.delete(id);
    if(r.timer)this.env.clearTimeout(r.timer);r.controller?.abort();r.reader?.cancel().catch(()=>{});r.cancel?.();
    if(r.socket){r.socket.onopen=r.socket.onmessage=r.socket.onerror=r.socket.onclose=null;try{r.socket.close(1000,'Scope disposed');}catch{/* Already closed or failed during construction. */}}
  }
  dispose() {this.disposed=true;for(const id of [...this.resources.keys()])this.cancel(id);this.env.removeEventListener?.('popstate',this.locationChanged);this.env.removeEventListener?.('hashchange',this.locationChanged);}
}

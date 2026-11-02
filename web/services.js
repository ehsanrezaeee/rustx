// Version 1 host boundary: bounded GET/cancel only. No app state or layout here.
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
      http_cancel:typeof AbortController === 'function'?'supported':'unavailable',...HOST_LIMITS};
  }
  validate(command) {
    if (!command || command.version !== HOST_VERSION) throw new Error('Host protocol version mismatch');
    if (!Number.isSafeInteger(command.id) || command.id < 1) throw new Error('Invalid host request ID');
    if (!['Get','Cancel'].includes(command.kind)) throw new Error('Unsupported host command');
    if (command.kind === 'Get' && (typeof command.url !== 'string' || !command.url.length || new TextEncoder().encode(command.url).length > 8192 ||
      !Number.isInteger(command.max_bytes) || command.max_bytes < 1 || command.max_bytes > HOST_LIMITS.max_response_bytes ||
      !Number.isInteger(command.timeout_ms) || command.timeout_ms < 1 || command.timeout_ms > HOST_LIMITS.timeout_ms)) {
      throw new Error('Invalid host request limits');
    }
  }
  accept(commands) {
    if (!Array.isArray(commands) || commands.length > 32) throw new Error('Host command batch exceeds limit');
    for (const command of commands) this.validate(command);
    for (const command of commands) {
      if (command.kind === 'Get') void this.request(command);
      else this.active.get(command.id)?.abort();
    }
  }
  async request(command) {
    this.validate({...command,kind:'Get'});
    if (this.disposed) return;
    if (this.active.has(command.id)) throw new Error('Duplicate active host request ID');
    if (this.active.size >= HOST_LIMITS.max_requests) {
      this.complete(command.id,0,new Uint8Array(),'busy',HOST_VERSION);
      return;
    }
    const controller = new AbortController();
    this.active.set(command.id,controller);
    let error='',status=0,body=new Uint8Array();
    const timer=setTimeout(()=>{error='timeout';controller.abort();},command.timeout_ms);
    try {
      const url=this.base?new URL(command.url,this.base):command.url;
      const response=await this.fetcher.call(globalThis,url,{signal:controller.signal,credentials:'same-origin'});
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
    } catch {
      error=error||(controller.signal.aborted?'cancelled':'Network request failed');
    } finally {
      clearTimeout(timer);
      this.active.delete(command.id);
    }
    if (!this.disposed) this.complete(command.id,status,body,error,HOST_VERSION);
  }
  dispose() {
    this.disposed=true;
    for (const controller of this.active.values()) controller.abort();
  }
}

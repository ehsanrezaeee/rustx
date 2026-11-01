// Browser capability adapter. App code sees typed Rust requests/results only.
export class HostNetwork {
  constructor(complete, fetcher = globalThis.fetch, base = globalThis.location?.href) {
    this.complete = complete; this.fetcher = fetcher; this.base = base; this.active = new Map();
  }
  accept(commands) {
    for (const command of commands) {
      if (command.kind === 'Get') this.request(command);
      else if (command.kind === 'Cancel') this.active.get(command.id)?.abort();
    }
  }
  async request(command) {
    if(this.active.size >= 16){this.complete(command.id, 0, new Uint8Array(), 'Too many pending requests');return;}
    const controller = new AbortController(); this.active.set(command.id, controller);
    let error = ''; const timer = setTimeout(() => { error='timeout';controller.abort(); }, command.timeout_ms);
    try {
      const url = this.base ? new URL(command.url, this.base) : command.url;
      const response = await this.fetcher(url, {signal:controller.signal,credentials:'same-origin'});
      const reader = response.body?.getReader(); const chunks=[];let size=0;
      if(reader){for(;;){const {done,value}=await reader.read();if(done)break;size+=value.byteLength;
        if(size>command.max_bytes){error='too_large';await reader.cancel();throw new Error(error);}chunks.push(value);}}
      const body=new Uint8Array(size);let offset=0;for(const chunk of chunks){body.set(chunk,offset);offset+=chunk.length;}
      this.complete(command.id,response.status,body,'');
    } catch(e) { this.complete(command.id,0,new Uint8Array(),error||(controller.signal.aborted?'cancelled':'Network request failed')); }
    finally { clearTimeout(timer);this.active.delete(command.id); }
  }
  dispose(){for(const controller of this.active.values())controller.abort();}
}

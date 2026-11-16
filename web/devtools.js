// Explicit development UI. This inspects Rust-owned geometry; it never lays out the application.
export async function install(app, api) {
  const lifetime = new AbortController();
  let disposed = false,
    timer;
  const owned = new Set();
  const own = (node) => {
    node.dataset.rustxDom = 'tooling';
    owned.add(node);
    return node;
  };
  const samples = [],
    events = [];
  api.profile = () => ({
    wasm_capacity: api.memoryBytes(),
    frames: samples.slice(),
    events: events.slice(),
    rust: app.profile ? JSON.parse(app.profile()) : null,
    allocations: app.allocations ? JSON.parse(app.allocations()) : null,
  });
  api.recordFrame = (milliseconds) => {
    samples.push({ at: performance.now(), milliseconds });
    if (samples.length > 120) samples.shift();
  };
  api.recordEvent = (event) => {
    events.push({
      at: performance.now(),
      kind: event.type,
      key: event.key?.length === 1 ? '[text]' : event.key,
      target: event.target?.dataset?.testid || null,
    });
    if (events.length > 256) events.shift();
  };
  for (const kind of ['pointerdown', 'pointerup', 'keydown', 'input', 'wheel'])
    document.addEventListener(kind, api.recordEvent, {
      capture: true,
      passive: true,
      signal: lifetime.signal,
    });
  const mark = own(document.createElement('div'));
  Object.assign(mark.style, {
    position: 'fixed',
    border: '2px solid #b5f36b',
    pointerEvents: 'none',
    zIndex: 10000,
    display: 'none',
  });
  document.body.append(mark);
  api.highlight = (id) => {
    if (disposed) return;
    const node = api.inspect().nodes.find((n) => n.node === id);
    if (!node) return;
    const [x, y, width, height] = node.rect;
    Object.assign(mark.style, {
      display: 'block',
      left: x + 'px',
      top: y + 'px',
      width: width + 'px',
      height: height + 'px',
    });
  };
  api.clearHighlight = () => (mark.style.display = 'none');
  api.openInspector = () =>
    window.open('/inspector.html', 'rustx-inspector', 'width=1100,height=800');
  app.set_diagnostics?.(true);
  // A floating tool must never intercept an application's pointer target by default.
  // Developers can open it from the console or explicitly request the launcher.
  if (new URLSearchParams(location.search).get('inspector') === '1') {
    const button = own(document.createElement('button'));
    button.id = 'rustx-devtools';
    button.textContent = 'Inspect RustX';
    button.type = 'button';
    Object.assign(button.style, { position: 'fixed', bottom: '10px', left: '16px', zIndex: 10001 });
    button.onclick = api.openInspector;
    document.body.append(button);
  }
  let revision;
  async function poll() {
    try {
      if (disposed || api.hostState().suspended) return false;
      const r = await fetch('/dev.json', { cache: 'no-store', signal: lifetime.signal });
      if (!r.ok) return false;
      const state = await r.json();
      if (disposed) return false;
      let panel = document.querySelector('#rustx-build-error');
      if (state.error) {
        if (!panel) {
          panel = own(document.createElement('pre'));
          panel.id = 'rustx-build-error';
          Object.assign(panel.style, {
            position: 'fixed',
            inset: '15px',
            overflow: 'auto',
            background: '#341b20',
            color: 'white',
            padding: '20px',
            zIndex: 10002,
          });
          document.body.append(panel);
        }
        panel.textContent = 'Build failed. Fix the source to reload.\n' + state.error;
      } else {
        if (panel) {
          panel.remove();
          owned.delete(panel);
        }
        if (revision !== undefined && state.revision !== revision) location.reload();
        revision = state.revision;
      }
      return true;
    } catch {
      return false;
    }
  }
  // Do not block host initialization on the optional development endpoint.
  poll().then((available) => {
    if (available && !disposed) timer = setInterval(poll, 1000);
  });
  return () => {
    if (disposed) return;
    disposed = true;
    lifetime.abort();
    clearInterval(timer);
    for (const node of owned) {
      node.onclick = null;
      node.remove();
    }
    owned.clear();
    samples.length = events.length = 0;
  };
}

// Opt-in, main-document accounting. No application text or node references are retained.
export function createDomAudit(doc) {
  const categories = [
    'shell',
    'semantic',
    'emergency',
    'service',
    'preview',
    'tooling',
    'resource',
    'unregistered',
  ];
  const blank = () => Object.fromEntries(categories.map((name) => [name, 0]));
  const mutations = Object.fromEntries(
    categories.map((name) => [name, { added: 0, removed: 0, attributes: 0, text: 0, children: 0 }]),
  );
  function category(node, formerParent) {
    const el = node.nodeType === 1 ? node : node.parentElement;
    if (!el) return 'shell';
    if (el.matches('[data-node]') || (el.closest('#semantic') && el.id !== 'semantic'))
      return 'semantic';
    if (el.closest('#fatal') && el.id !== 'fatal') return 'emergency';
    if (el.id === 'rustx-compiled-output') return 'preview';
    const declared = el.closest('[data-rustx-dom]')?.getAttribute('data-rustx-dom');
    if (['service', 'tooling'].includes(declared)) return declared;
    if (
      ['HTML', 'HEAD', 'BODY', 'NOSCRIPT'].includes(el.tagName) ||
      ['surface', 'semantic', 'fatal'].includes(el.id)
    )
      return 'shell';
    if (el.closest('head') || el.tagName === 'SCRIPT') return 'resource';
    if (formerParent?.id === 'fatal') return 'emergency';
    if (formerParent?.closest?.('head')) return 'resource';
    return 'unregistered';
  }
  function elements(node) {
    return node.nodeType === 1 ? [node, ...node.querySelectorAll('*')] : [];
  }
  function consume(records) {
    for (const record of records) {
      const count = mutations[category(record.target)];
      if (record.type === 'attributes') count.attributes++;
      else if (record.type === 'characterData') count.text++;
      else {
        count.children++;
        for (const node of record.addedNodes)
          for (const el of elements(node)) mutations[category(el, record.target)].added++;
        for (const node of record.removedNodes)
          for (const el of elements(node)) mutations[category(el, record.target)].removed++;
      }
    }
  }
  const observer = new MutationObserver(consume);
  observer.observe(doc.documentElement, {
    subtree: true,
    childList: true,
    attributes: true,
    characterData: true,
  });
  return {
    snapshot() {
      consume(observer.takeRecords());
      const counts = blank();
      let visibleSemantic = 0;
      for (const el of doc.querySelectorAll('*')) {
        const name = category(el);
        counts[name]++;
        if (name === 'semantic' && getComputedStyle(el).opacity !== '0') visibleSemantic++;
      }
      return {
        counts,
        mutations: structuredClone(mutations),
        visibleSemantic,
        scope: 'main document only',
      };
    },
    dispose() {
      consume(observer.takeRecords());
      observer.disconnect();
    },
  };
}

// Explicit development UI. This inspects Rust-owned geometry; it never lays out the application.
export async function install(app, api) {
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
    document.addEventListener(kind, api.recordEvent, { capture: true, passive: true });
  const mark = document.createElement('div');
  Object.assign(mark.style, {
    position: 'fixed',
    border: '2px solid #b5f36b',
    pointerEvents: 'none',
    zIndex: 10000,
    display: 'none',
  });
  document.body.append(mark);
  api.highlight = (id) => {
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
    const button = document.createElement('button');
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
      const r = await fetch('/dev.json', { cache: 'no-store' });
      if (!r.ok) return false;
      const state = await r.json();
      let panel = document.querySelector('#rustx-build-error');
      if (state.error) {
        if (!panel) {
          panel = document.createElement('pre');
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
        panel?.remove();
        if (revision !== undefined && state.revision !== revision) location.reload();
        revision = state.revision;
      }
      return true;
    } catch {
      return false;
    }
  }
  if (await poll()) {
    const timer = setInterval(poll, 1000);
    window.addEventListener('pagehide', () => clearInterval(timer), { once: true });
  }
}

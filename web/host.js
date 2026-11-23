import init, * as wasm from './pkg/rustx_lab.js';
import { HostNetwork, HostServices } from './services.js';
import { SemanticProjection } from './semantics.js';
const config = await fetch('/runtime-config.json', { cache: 'no-store' })
  .then((r) => (r.ok ? r.json() : {}))
  .catch(() => ({}));
let instance;
let tools, audit;
let currentHost, restarting;
function restart() {
  if (!restarting)
    restarting = (async () => {
      await currentHost?.dispose();
      currentHost = await mountHost();
    })().finally(() => {
      restarting = null;
    });
  return restarting;
}
async function mountHost() {
  let disposed = false,
    suspended = false,
    animation = 0,
    disposeTools,
    pendingGPU,
    disposal;
  const listeners = new AbortController();
  const app = wasm;
  function listen(target, type, callback, options = {}) {
    target.addEventListener(type, callback, { ...options, signal: listeners.signal });
  }
  let canvas = document.querySelector('#surface');
  const root = document.querySelector('#semantic');
  const status = config.devtools ? document.createElement('div') : null;
  if (status) {
    status.id = 'status';
    status.dataset.rustxDom = 'tooling';
    document.body.append(status);
  }
  function setStatus(text) {
    if (status) status.textContent = text;
  }
  let preview = null;
  let activeDevice = null,
    queued = false,
    renderer = 'cpu',
    ctx,
    ready = false,
    draws = 0;
  const network = new HostNetwork((id, status, body, error, version, headers = []) => {
    if (disposed) return;
    const accepted = app.response_meta(id, status, body, error, version, JSON.stringify(headers));
    if (accepted && error && error !== 'cancelled')
      resourceError(
        'http',
        ['timeout', 'too_large', 'busy'].includes(error) ? error : 'transport',
        null,
      );
    pump();
    schedule();
  });
  const services = new HostServices((id, json, version) => {
    if (disposed) return;
    const accepted = app.service_response(version, id, json),
      error = JSON.parse(json).error;
    if (accepted && error && error.kind !== 'cancelled') resourceError('service', error.kind);
    pump();
    schedule();
  });
  function fatal(e) {
    if (disposed) return;
    dispose();
    const panel = document.querySelector('#fatal');
    const message = document.createElement('p'),
      reload = document.createElement('button');
    message.textContent = 'RustX could not continue. Reload to restart the application.';
    reload.textContent = 'Reload application';
    reload.addEventListener('click', () => location.reload());
    panel.replaceChildren(message, reload);
    reload.focus();
    console.error(e);
  }
  function schedule() {
    if (!queued && ready && !disposed && !suspended && !document.hidden) {
      queued = true;
      animation = requestAnimationFrame(draw);
    }
  }
  const projection = new SemanticProjection(root, app, schedule, pump);
  function resourceError(source, code, status = null) {
    if (ready) projection.hostEvent('resource_error', { resource_error: { source, code, status } });
  }
  function resized() {
    if (ready)
      projection.hostEvent('resize', {
        width: innerWidth,
        height: innerHeight,
        scale: devicePixelRatio || 1,
      });
    schedule();
  }
  function sync() {
    const inspection = JSON.parse(app.inspect());
    projection.sync(JSON.parse(app.semantics()), inspection);
    preview?.sync(inspection);
  }
  listen(document, 'keydown', (e) => {
    if (ready) projection.tab(e);
  });
  function pump() {
    if (disposed) return;
    network.accept(JSON.parse(app.commands()));
    if (app.service_commands) services.accept(JSON.parse(app.service_commands()));
  }
  function draw() {
    queued = false;
    if (disposed || suspended || !ready) return;
    const started = config.devtools ? performance.now() : 0;
    try {
      const w = innerWidth,
        h = innerHeight;
      if (w <= 0 || h <= 0) return;
      const requested = Math.min(devicePixelRatio || 1, 2),
        limit = Math.min(8192 / w, 8192 / h, Math.sqrt(16777216 / (w * h))),
        dpr = Math.min(requested, limit < requested ? limit * 0.999 : requested),
        pw = Math.round(w * dpr),
        ph = Math.round(h * dpr);
      if (canvas.width !== pw) canvas.width = pw;
      if (canvas.height !== ph) canvas.height = ph;
      if (app.frame(w, h, dpr)) {
        draws++;
        if (renderer === 'cpu')
          ctx.putImageData(new ImageData(new Uint8ClampedArray(app.pixels()), pw, ph), 0, 0);
        sync();
        if (config.devtools) window.rustx?.recordFrame?.(performance.now() - started);
      }
      pump();
      if (app.needs_update()) schedule();
    } catch (e) {
      if (renderer === 'gpu') {
        recoverCPU(String(e));
      } else fatal(e);
    }
  }
  function recoverCPU(reason) {
    resourceError('renderer', 'gpu_recovery');
    const device = activeDevice;
    activeDevice = null;
    app.drop_gpu();
    device?.destroy();
    const replacement = canvas.cloneNode();
    canvas.replaceWith(replacement);
    canvas = replacement;
    ctx = canvas.getContext('2d');
    if (!ctx) {
      fatal('CPU canvas unavailable');
      return;
    }
    renderer = 'cpu';
    app.invalidate();
    setStatus('RustX · CPU recovery · ' + reason);
    schedule();
  }
  function initGPU() {
    const work = app.init_gpu(canvas);
    pendingGPU = work;
    return work.finally(() => {
      if (pendingGPU === work) pendingGPU = null;
    });
  }
  async function retryGPU() {
    if (disposed) throw new Error('Host disposed');
    if (pendingGPU) return pendingGPU;
    if (!navigator.gpu) throw new Error('WebGPU unavailable');
    const replacement = canvas.cloneNode();
    canvas.replaceWith(replacement);
    canvas = replacement;
    try {
      await initGPU();
      if (disposed) {
        app.drop_gpu();
        return;
      }
      renderer = 'gpu';
      app.invalidate();
      setStatus('RustX · WebGPU · Stage 4');
      schedule();
    } catch (error) {
      if (!disposed) recoverCPU('GPU initialization failed');
      throw error;
    }
  }
  let restoreGPU = () => {};
  // Observe actual browser device loss without owning geometry, state or layout.
  if (navigator.gpu) {
    const original = navigator.gpu.requestAdapter;
    const request = original.bind(navigator.gpu);
    const wrapped = async (options) => {
      const adapter = await request(options);
      if (adapter) {
        const create = adapter.requestDevice.bind(adapter);
        adapter.requestDevice = async (descriptor) => {
          const device = await create(descriptor);
          if (disposed) {
            device.destroy();
            return device;
          }
          activeDevice = device;
          device.lost.then((info) => {
            if (activeDevice === device && ready && renderer === 'gpu')
              recoverCPU('GPU device lost: ' + info.reason);
          });
          listen(device, 'uncapturederror', (event) => {
            event.preventDefault();
            if (activeDevice === device && ready && renderer === 'gpu') recoverCPU('GPU error');
          });
          return device;
        };
      }
      return adapter;
    };
    navigator.gpu.requestAdapter = wrapped;
    restoreGPU = () => {
      if (navigator.gpu.requestAdapter === wrapped) navigator.gpu.requestAdapter = original;
    };
  }
  let dprQuery;
  function watchDPR() {
    dprQuery?.removeEventListener('change', changedDPR);
    dprQuery = matchMedia(`(resolution: ${devicePixelRatio}dppx)`);
    dprQuery.addEventListener('change', changedDPR, { once: true });
  }
  function changedDPR() {
    watchDPR();
    resized();
  }
  watchDPR();
  listen(document, 'visibilitychange', () => {
    if (!document.hidden) schedule();
  });
  listen(window, 'resize', resized);
  listen(window, 'dragover', (e) => {
    if (ready) e.preventDefault();
  });
  listen(window, 'drop', (e) => {
    if (ready) projection.drop(null, e);
  });
  function wheel(node, dy, dx = 0, x = 0, y = 0, event) {
    if (node && projection.event(node, 'wheel', { delta_y: dy, delta_x: dx, x, y }, event)) return;
    if (!node?.dataset.scrollNode || !app.scroll_node(Number(node.dataset.scrollNode), dy))
      app.scroll(dy);
    schedule();
  }
  listen(
    window,
    'wheel',
    (e) => {
      if (!ready) return;
      const node = e.target.closest?.('[data-node]');
      const unit = e.deltaMode;
      const dx = e.deltaX * (unit === 1 ? 40 : unit === 2 ? innerWidth : 1);
      const dy = e.deltaY * (unit === 1 ? 40 : unit === 2 ? innerHeight : 1);
      if (e.ctrlKey || e.metaKey) {
        if (node?.getAttribute('role') === 'img') {
          e.preventDefault();
          app.canvas_zoom(Number(node.dataset.node), e.clientX, e.clientY, Math.exp(-dy * 0.01));
          schedule();
        }
        return;
      }
      wheel(node, dy, dx, e.clientX, e.clientY, e);
      e.preventDefault();
    },
    { passive: false },
  );
  let touch;
  listen(
    window,
    'touchstart',
    (e) => {
      touch = { y: e.touches[0]?.clientY, node: e.target.closest?.('[data-node]') };
    },
    { passive: true },
  );
  listen(
    window,
    'touchmove',
    (e) => {
      const y = e.touches[0]?.clientY;
      if (ready && y !== undefined && touch?.y !== undefined) {
        wheel(touch.node, touch.y - y, 0, e.touches[0].clientX, y, e);
        touch.y = y;
        e.preventDefault();
      }
    },
    { passive: false },
  );
  function dispose() {
    if (disposed) return disposal;
    disposed = true;
    ready = false;
    cancelAnimationFrame(animation);
    queued = false;
    listeners.abort();
    dprQuery?.removeEventListener('change', changedDPR);
    network.dispose();
    services.dispose();
    projection.dispose();
    preview?.dispose();
    disposeTools?.();
    status?.remove();
    touch = null;
    restoreGPU();
    const device = activeDevice;
    activeDevice = null;
    device?.destroy();
    if (instance) app.dispose_app?.();
    disposal = pendingGPU
      ? pendingGPU.catch(() => {}).then(() => app.drop_gpu())
      : Promise.resolve();
    const replacement = canvas.cloneNode();
    canvas.replaceWith(replacement);
    canvas = replacement;
    return disposal;
  }
  listen(window, 'pagehide', (event) => {
    if (!event.persisted) {
      dispose();
      return;
    }
    suspended = true;
    cancelAnimationFrame(animation);
    queued = false;
  });
  listen(window, 'pageshow', (event) => {
    if (!event.persisted || disposed) return;
    suspended = false;
    if (!ready) return;
    app.invalidate();
    resized();
  });
  try {
    instance ||= await init();
    if (config.devtools && !tools) {
      tools = await import('./devtools.js');
      if (new URLSearchParams(location.search).get('dom-audit') === '1')
        audit = tools.createDomAudit(document);
    }
    if (disposed) return { dispose };
    app.init_app(JSON.stringify(network.capabilities()));
    app.service_capabilities(JSON.stringify(services.capabilities()));
    const requested = new URLSearchParams(location.search).get('renderer') || 'auto';
    let reason = '';
    if (requested !== 'cpu' && navigator.gpu) {
      try {
        await initGPU();
        if (disposed) {
          app.drop_gpu();
          return { dispose };
        }
        renderer = 'gpu';
      } catch (e) {
        reason = 'GPU initialization failed; software mode active.';
        app.drop_gpu();
        activeDevice?.destroy();
        activeDevice = null;
        const replacement = canvas.cloneNode();
        canvas.replaceWith(replacement);
        canvas = replacement;
      }
    }
    if (renderer === 'cpu') {
      ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('Canvas context unavailable');
    }
    app.configure_playground?.(config.compiler || '');
    if (config.compiler)
      preview = await import('./playground.js').then((m) =>
        disposed ? null : m.install(app, config, schedule),
      );
    if (disposed) return { dispose };
    // Initial location/capability work belongs before the first Rust layout/paint.
    // This prevents a deep link from briefly painting the default home route.
    pump();
    ready = true;
    setStatus(
      renderer === 'gpu'
        ? 'RustX · WebGPU · Stage 4'
        : `RustX · Rust CPU renderer · Stage 4${reason ? ' · ' + reason : ''}`,
    );
    if (config.benchmark)
      window.rustxBenchmark = {
        inspect: () => JSON.parse(app.inspect()),
        allocations: () => (app.allocations ? JSON.parse(app.allocations()) : null),
        capacity: () => instance.memory.buffer.byteLength,
        draws: () => draws,
        pan: (id, dx, dy) => {
          app.canvas_pan(id, dx, dy);
          schedule();
        },
      };
    if (config.devtools) {
      window.rustx = {
        sourceRoot: config.source_root,
        dispose,
        restart,
        dom: audit ? () => audit.snapshot() : undefined,
        hostState: () => ({
          disposed,
          suspended,
          queued,
          semantic: projection.elements.size,
          requests: network.active?.size,
          services: services.resources.size,
        }),
        memoryBytes: () => instance.memory.buffer.byteLength,
        inspect: () => JSON.parse(app.inspect()),
        semantics: () => JSON.parse(app.semantics()),
        renderer: () => renderer,
        draws: () => draws,
        retryGPU,
        destroyGPU: () => app.destroy_gpu(),
        setTheme: (light) => {
          app.set_theme(light);
          schedule();
        },
      };
      disposeTools = await tools.install(app, window.rustx);
      if (disposed) {
        disposeTools();
        return { dispose };
      }
      for (const [key, fn] of Object.entries(window.rustx)) {
        if (typeof fn === 'function' && !['dispose', 'restart', 'dom', 'hostState'].includes(key))
          window.rustx[key] = (...args) => (disposed ? undefined : fn(...args));
      }
    }
    schedule();
    if (!config.devtools && 'serviceWorker' in navigator && location.protocol !== 'file:')
      navigator.serviceWorker
        .register('/service-worker.js')
        .catch((e) => console.info('Offline shell unavailable', e.name));
    resized();
    if (reason) resourceError('renderer', 'initialization_failed');
  } catch (e) {
    fatal(e);
  }

  return { dispose };
}
await restart();

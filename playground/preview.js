import { SemanticProjection } from './semantics.js';
const script = document.querySelector('script[data-job]'),
  origin = script.dataset.origin,
  job = script.dataset.job;
const canvas = document.querySelector('#surface'),
  context = canvas.getContext('2d'),
  root = document.querySelector('#semantic'),
  error = document.querySelector('#error');
const workerSource = `import(${JSON.stringify(origin + '/worker.js?origin=' + encodeURIComponent(origin) + '&job=' + encodeURIComponent(job))}).catch(error=>postMessage({type:'error',message:String(error)}));`;
// A classic blob bootstrap can dynamically import the module graph from an opaque
// frame in Chrome/WebKit; their module-worker blob loader rejects opaque origins.
const blob = URL.createObjectURL(new Blob([workerSource], { type: 'text/javascript' }));
const worker = new Worker(blob);
let state,
  seq = 0,
  waiting = 0,
  timer;
function fail(message) {
  clearTimeout(timer);
  worker.terminate();
  error.textContent = message;
  parent.postMessage(
    { type: 'rustx-preview-error', job, message: String(message).slice(0, 1000) },
    '*',
  );
}
function watchdog() {
  clearTimeout(timer);
  timer = setTimeout(() => fail('Compiled application exceeded the runtime response limit'), 2500);
}
function call(method, ...args) {
  if (waiting > 32) {
    fail('Compiled application input queue limit');
    return;
  }
  waiting++;
  worker.postMessage({ type: 'call', seq: ++seq, method, args });
  watchdog();
}
const app = {
  dispatch: (id, data) => {
    call('dispatch', id, data);
    return '{}';
  },
  activate: (id) => call('activate', id),
  focus: (id) => call('focus', id),
  blur: (id) => call('blur', id),
  edit_sync: (...args) => call('edit_sync', ...args),
  edit_value: (id) => state?.inputs[id] || '',
  scroll: (delta) => call('scroll', delta),
  canvas_pan: (...a) => call('canvas_pan', ...a),
  canvas_pick: (...a) => call('canvas_pick', ...a),
  canvas_zoom: (...a) => call('canvas_zoom', ...a),
};
const projection = new SemanticProjection(
  root,
  app,
  () => {},
  () => {},
  () => {},
);
worker.onmessage = (event) => {
  const message = event.data;
  if (message.type === 'error') {
    fail('Compiled application failed: ' + message.message);
    return;
  }
  if (message.type === 'ready') {
    resize();
    return;
  }
  if (message.type !== 'frame') return;
  if (!(message.pixels instanceof ArrayBuffer) || message.pixels.byteLength > 1280 * 720 * 4) {
    fail('Invalid preview output');
    return;
  }
  clearTimeout(timer);
  waiting = 0;
  if (message.seq < seq) return;
  state = message;
  canvas.width = message.width;
  canvas.height = message.height;
  context.putImageData(
    new ImageData(new Uint8ClampedArray(message.pixels), message.width, message.height),
    0,
    0,
  );
  projection.sync(message.semantics, message.inspection);
  parent.postMessage({ type: 'rustx-preview-ready', job }, '*');
};
worker.onerror = (event) =>
  fail('Compiled application worker could not load: ' + (event.message || event.type));
function resize() {
  worker.postMessage({ type: 'resize', width: innerWidth, height: innerHeight });
  watchdog();
}
addEventListener('resize', resize);
addEventListener('keydown', (e) => projection.tab(e));
addEventListener(
  'wheel',
  (e) => {
    e.preventDefault();
    call('scroll', e.deltaY);
  },
  { passive: false },
);
addEventListener(
  'pagehide',
  () => {
    clearTimeout(timer);
    worker.terminate();
    URL.revokeObjectURL(blob);
    projection.dispose();
  },
  { once: true },
);
watchdog();

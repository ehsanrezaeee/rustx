// Every visitor instruction executes in this worker. Termination is owned by the frame.
const params = new URL(import.meta.url).searchParams,
  origin = params.get('origin'),
  job = params.get('job');
const { HostNetwork } = await import(origin + '/services.js');
const app = await import(origin + '/jobs/' + job + '/pkg/rustx_preview.js');
await app.default();
const fetchFixture = (input, options = {}) => {
  const url = new URL(input, origin);
  if (
    url.origin !== origin ||
    url.pathname !== '/api/users' ||
    url.search ||
    (options.method && options.method !== 'GET')
  )
    return Promise.reject(new Error('Preview capability denied'));
  return fetch(url, { ...options, credentials: 'omit', referrerPolicy: 'no-referrer' });
};
let queued = false,
  width = 600,
  height = 420,
  seq = 0;
const host = new HostNetwork(
  (id, status, body, error, version, headers) => {
    app.response_meta(id, status, body, error, version, JSON.stringify(headers));
    pump();
    schedule();
  },
  fetchFixture,
  origin + '/',
);
app.init_app(JSON.stringify({ ...host.capabilities(), http_methods: false }));
function pump() {
  host.accept(JSON.parse(app.commands()));
}
function schedule() {
  if (!queued) {
    queued = true;
    setTimeout(draw, 0);
  }
}
function draw() {
  queued = false;
  try {
    if (app.frame(width, height, 1)) {
      const pixels = new Uint8ClampedArray(app.pixels());
      if (pixels.length > 1280 * 720 * 4) throw Error('Preview surface limit');
      const inspection = JSON.parse(app.inspect()),
        semantics = JSON.parse(app.semantics());
      const inputs = {};
      for (const node of inspection.nodes) {
        if (node.editing) inputs[node.node] = app.edit_value(node.node);
      }
      postMessage(
        { type: 'frame', seq, width, height, pixels: pixels.buffer, inspection, semantics, inputs },
        [pixels.buffer],
      );
    }
    pump();
    if (app.needs_update()) schedule();
  } catch (error) {
    postMessage({ type: 'error', message: String(error) });
    close();
  }
}
const methods = new Set([
  'activate',
  'focus',
  'blur',
  'dispatch',
  'edit_sync',
  'scroll',
  'scroll_node',
  'canvas_pan',
  'canvas_pick',
  'canvas_zoom',
]);
onmessage = (event) => {
  const message = event.data;
  if (message.type === 'resize') {
    width = Math.max(1, Math.min(1280, Math.round(message.width)));
    height = Math.max(1, Math.min(720, Math.round(message.height)));
  } else if (
    message.type === 'call' &&
    methods.has(message.method) &&
    Array.isArray(message.args) &&
    message.args.length <= 8
  ) {
    seq = message.seq;
    app[message.method](...message.args);
  } else return;
  pump();
  schedule();
};
postMessage({ type: 'ready' });
schedule();

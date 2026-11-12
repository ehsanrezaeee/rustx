// The containing rectangle is supplied by Rust. Visitor code runs in an opaque frame's worker.
export function install(app, config, schedule) {
  let iframe,
    current = '',
    nodeId;
  function sync(inspection) {
    const node = inspection.nodes.find((n) => n.id === 'playground-surface');
    const value = node?.text || '';
    let valid = false;
    try {
      const u = new URL(value);
      valid =
        u.origin === config.compiler &&
        /^\/jobs\/[0-9a-f]{32}\/preview\.html$/.test(u.pathname) &&
        !u.search &&
        !u.hash;
    } catch {}
    if (!valid) {
      iframe?.remove();
      iframe = null;
      current = '';
      return;
    }
    nodeId = node.node;
    if (current !== value) {
      iframe?.remove();
      iframe = document.createElement('iframe');
      iframe.id = 'rustx-compiled-output';
      iframe.title = 'Isolated compiled RustX application';
      iframe.setAttribute('sandbox', 'allow-scripts');
      iframe.referrerPolicy = 'no-referrer';
      iframe.src = value;
      Object.assign(iframe.style, { position: 'fixed', border: '0', zIndex: 1 });
      document.body.append(iframe);
      current = value;
      app.focus(node.node);
      schedule();
    }
    const [x, y, w, h] = node.rect,
      [vx, vy, vw, vh] = node.visible_rect;
    Object.assign(iframe.style, {
      left: x + 'px',
      top: y + 'px',
      width: w + 'px',
      height: h + 'px',
      clipPath:
        vw > 0 && vh > 0
          ? `inset(${Math.max(0, vy - y)}px ${Math.max(0, x + w - vx - vw)}px ${Math.max(0, y + h - vy - vh)}px ${Math.max(0, vx - x)}px)`
          : 'inset(100%)',
    });
  }
  addEventListener('message', (event) => {
    if (
      !iframe ||
      event.source !== iframe.contentWindow ||
      event.origin !== 'null' ||
      event.data?.job !== new URL(current).pathname.split('/')[2]
    )
      return;
    if (event.data.type === 'rustx-preview-ready') {
      iframe.dataset.ready = 'true';
    } else if (
      event.data.type === 'rustx-preview-error' &&
      typeof event.data.message === 'string'
    ) {
      app.input(nodeId, event.data.message.slice(0, 1000), false);
      schedule();
    }
  });
  document.addEventListener('click', (event) => {
    if (event.target?.dataset?.testid === 'playground-go-error') {
      const node = JSON.parse(app.inspect()).nodes.find((n) => n.id === 'playground-source');
      if (node) {
        app.focus(node.node);
        schedule();
      }
    }
  });
  addEventListener('pagehide', () => iframe?.remove(), { once: true });
  return { sync };
}

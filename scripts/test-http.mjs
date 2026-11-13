// Actual Rust fixture server -> production JS Fetch adapter -> compiled Wasm.
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import { HostNetwork, HOST_VERSION } from '../web/services.js';
const root = new URL('../', import.meta.url);
const port = Number(process.env.RUSTX_TEST_PORT || 3000);
const server = spawn(
  fileURLToPath(
    new URL(
      `../target/debug/rustx-cli${process.platform === 'win32' ? '.exe' : ''}`,
      import.meta.url,
    ),
  ),
  ['serve', 'dist', String(port)],
  { cwd: root, stdio: ['ignore', 'pipe', 'pipe'] },
);
try {
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Fixture server startup timeout')), 10000);
    const fail = (e) => {
      clearTimeout(timer);
      reject(e);
    };
    server.once('error', fail);
    server.once('exit', (code) => fail(new Error(`Server exited ${code}`)));
    server.stdout.on('data', (chunk) => {
      if (String(chunk).includes('RustX serving')) {
        clearTimeout(timer);
        resolve();
      }
    });
  });
  const app = await import('../dist/pkg/rustx_lab.js');
  await app.default({
    module_or_path: await fs.readFile(new URL('../dist/pkg/rustx_lab_bg.wasm', import.meta.url)),
  });
  const render = () => {
    for (let i = 0; i < 4; i++) {
      app.frame(1000, 800, 1);
      if (!app.needs_update()) break;
    }
  };
  const find = (id) => JSON.parse(app.inspect()).nodes.find((n) => n.id === id);
  const click = (id) => {
    app.activate(find(id).node);
    render();
  };
  const host = new HostNetwork(
    (id, status, body, error, version) => {
      app.response(id, status, body, error, version);
      render();
    },
    globalThis.fetch,
    `http://127.0.0.1:${port}/`,
  );
  const run = async () => {
    for (const c of JSON.parse(app.commands())) {
      if (c.kind === 'Get') await host.request(c);
      else host.accept([c]);
    }
  };
  app.init_app(JSON.stringify(new HostNetwork(() => {}).capabilities()));
  render();
  click('nav-people');
  click('load-fixture');
  await run();
  assert.match(find('api-state').text, /6 people/);
  click('load-error');
  await run();
  assert.match(find('api-state').text, /HTTP 503/);
  click('load-slow');
  const cmd = JSON.parse(app.commands()).find((c) => c.kind === 'Get');
  const pending = host.request(cmd);
  click('cancel');
  host.accept(JSON.parse(app.commands()));
  await pending;
  assert.match(find('api-state').text, /Cancelled/);
  assert.equal(host.active.size, 0);
  console.log(
    'PASS: real Rust HTTP fixture server -> production JS Fetch -> compiled Wasm (success, HTTP 503, cancellation). No browser/CORS claim.',
  );
} finally {
  server.kill();
}

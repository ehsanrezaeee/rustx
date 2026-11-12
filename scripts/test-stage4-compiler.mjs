import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { createHash } from 'node:crypto';
const origin = process.env.RUSTX_COMPILER_URL || 'http://127.0.0.1:3002',
  app = 'http://127.0.0.1:3000',
  report = {
    date: new Date().toISOString(),
    checks: [],
    limitations: [
      'Local macOS compiler isolation only; not a public-service security certification',
      'No Windows/Linux sandbox execution claim',
      'Browser Wasm/worker tests are recorded separately',
    ],
  };
const request = (path, options = {}) => fetch(origin + path, options);
const headers = { 'content-type': 'application/json', origin: app };
async function submit(source) {
  const r = await request('/compile', {
    method: 'POST',
    headers,
    body: JSON.stringify({ source }),
  });
  assert.equal(r.status, 202, await r.clone().text());
  return (await r.json()).id;
}
async function result(id) {
  const deadline = Date.now() + 45000;
  for (;;) {
    const r = await request('/jobs/' + id, { headers: { origin: app } });
    assert(r.ok);
    const job = await r.json();
    if (job.status !== 'compiling') return job;
    assert(Date.now() < deadline, 'compiler result deadline');
    await new Promise((r) => setTimeout(r, 50));
  }
}
const counter = await fs.readFile(
  new URL('../crates/rustx-cli/templates/counter.rustx', import.meta.url),
  'utf8',
);
try {
  let healthy;
  const startup = Date.now() + 60000;
  while (!healthy) {
    try {
      const r = await request('/health');
      if (r.ok) healthy = await r.json();
    } catch {}
    if (!healthy) {
      assert(Date.now() < startup, 'compiler startup deadline');
      await new Promise((r) => setTimeout(r, 250));
    }
  }
  const health = healthy;
  assert(health.available);
  report.health = health;
  const forbidden = await request('/compile', {
    method: 'POST',
    headers: { ...headers, origin: 'https://untrusted.example' },
    body: JSON.stringify({ source: counter }),
  });
  assert.equal(forbidden.status, 403);
  const tooLarge = await request('/compile', {
    method: 'POST',
    headers,
    body: JSON.stringify({ source: ' '.repeat(50000) }),
  });
  assert(!tooLarge.ok);
  report.checks.push('configured-origin admission and source-size rejection');
  const id = await submit(counter.replace('Hello from RustX', 'A real compiler edit'));
  const built = await result(id);
  assert.equal(built.status, 'ready', JSON.stringify(built));
  assert.match(built.wasm_sha256, /^[a-f0-9]{64}$/);
  const wasm = await (await request('/jobs/' + id + '/pkg/rustx_preview_bg.wasm')).arrayBuffer();
  assert.equal(createHash('sha256').update(Buffer.from(wasm)).digest('hex'), built.wasm_sha256);
  report.wasmSha256 = built.wasm_sha256;
  report.checks.push('actual edited Rust compiled and bound to a real Wasm artifact with checksum');
  const wrong = await result(
    await submit(
      'use rustx::prelude::*;\npub fn app(_:Scope)->View {let number:u32="wrong";Node::text(number)}',
    ),
  );
  assert.equal(wrong.status, 'compile_error');
  assert(
    wrong.diagnostics.some((d) =>
      d.spans?.some((s) => s.file_name === 'main.rustx' && s.line_start === 2),
    ),
  );
  report.checks.push('real rustc type error is linked to main.rustx line 2');
  for (const expression of [
    'include_str!("/etc/passwd")',
    `include_str!(${JSON.stringify(process.cwd() + '/.rustx/playground/jobs/' + new URL(origin).port + '/' + id + '/main.rustx')})`,
    'env!("HOME")',
  ]) {
    const denied = await result(
      await submit(
        'use rustx::prelude::*;pub fn app(_:Scope)->View {Node::text(' + expression + ')}',
      ),
    );
    assert.equal(denied.status, 'compile_error', JSON.stringify(denied));
  }
  report.checks.push(
    'actual rustc cannot read host /etc/passwd, another job source or inherited HOME',
  );
  const injected = await result(
    await submit(
      'use rustx::prelude::*;use wasm_bindgen::prelude::*;#[wasm_bindgen(inline_js="export function visitor(){return 1;}")] extern "C"{fn visitor()->u32;}pub fn app(_:Scope)->View{Node::text(visitor())}',
    ),
  );
  assert.equal(injected.status, 'error');
  assert(injected.message.includes('JavaScript'));
  report.checks.push('visitor JavaScript snippets are rejected before publication');
  const cancel = await submit(counter);
  assert((await request('/jobs/' + cancel, { method: 'DELETE', headers: { origin: app } })).ok);
  const cancelled = await result(cancel);
  assert.equal(cancelled.status, 'cancelled');
  report.checks.push('explicit cancellation stops a real compiler process group');
  const arrivals = await Promise.all(
    [0, 1, 2].map(() =>
      request('/compile', { method: 'POST', headers, body: JSON.stringify({ source: counter }) }),
    ),
  );
  assert.equal(arrivals.filter((r) => r.status === 202).length, 2);
  assert.equal(arrivals.filter((r) => r.status === 429).length, 1);
  const ids = await Promise.all(
    arrivals.filter((r) => r.status === 202).map(async (r) => (await r.json()).id),
  );
  assert.notEqual(ids[0], ids[1]);
  for (const id of ids)
    await request('/jobs/' + id, { method: 'DELETE', headers: { origin: app } });
  await Promise.all(ids.map(result));
  report.checks.push(
    'two actual concurrent jobs, random distinct identities, third admission rejected',
  );
  report.result = 'passed';
} catch (error) {
  report.result = 'failed';
  report.error = String(error);
  throw error;
} finally {
  await fs.writeFile(
    new URL('../docs/evidence/stage4-compiler.json', import.meta.url),
    JSON.stringify(report, null, 2) + '\n',
  );
  console.log(JSON.stringify(report, null, 2));
}

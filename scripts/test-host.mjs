import assert from 'node:assert/strict';
import { HostNetwork, HOST_VERSION, HOST_LIMITS } from '../web/services.js';
const command = (id, other = {}) => ({
  kind: 'Get',
  version: HOST_VERSION,
  id,
  url: '/users',
  max_bytes: 1024,
  timeout_ms: 1000,
  ...other,
});
let calls = [];
const host = new HostNetwork(
  (...args) => calls.push(args),
  async () => new Response('ok'),
  'http://localhost/',
);
assert.equal(host.capabilities().version, HOST_VERSION);
for (const c of [
  command(1, { version: 99 }),
  command(1.5),
  command(1, { max_bytes: HOST_LIMITS.max_response_bytes + 1 }),
  command(1, { kind: 'Post' }),
]) {
  assert.throws(() => host.accept([c]));
}
assert.throws(() => host.accept(Array.from({ length: 33 }, (_, i) => command(i + 1))));
assert.equal(calls.length, 0);
assert.equal(host.active.size, 0);
await host.request(command(1));
assert.equal(calls.length, 1);
assert.equal(calls[0][4], HOST_VERSION);
assert.equal(host.active.size, 0);
const pendingFetch = (_url, { signal }) =>
  new Promise((_, reject) =>
    signal.addEventListener('abort', () => reject(new Error('Aborted')), { once: true }),
  );
calls = [];
const pending = new HostNetwork((...args) => calls.push(args), pendingFetch, 'http://localhost/');
const jobs = Array.from({ length: 16 }, (_, i) => pending.request(command(i + 1)));
await pending.request(command(17));
assert.equal(calls[0][3], 'busy');
assert.equal(pending.active.size, 16);
await assert.rejects(pending.request(command(1)), /Duplicate/);
pending.accept(
  Array.from({ length: 16 }, (_, i) => ({ kind: 'Cancel', version: HOST_VERSION, id: i + 1 })),
);
await Promise.all(jobs);
assert.equal(calls.filter((c) => c[3] === 'cancelled').length, 16);
assert.equal(pending.active.size, 0);
calls = [];
const disposing = new HostNetwork((...args) => calls.push(args), pendingFetch, 'http://localhost/');
const job = disposing.request(command(1));
disposing.dispose();
await job;
await disposing.request(command(2));
assert.equal(calls.length, 0);
assert.equal(disposing.active.size, 0);
console.log(
  'PASS: JS protocol/capabilities, malformed IDs/versions/limits/batches, concurrency bound, duplicate rejection, cancellation and disposal. Fake Fetch contract tests only.',
);
await import('./test-host-events.mjs');

// Actual sockets: default loopback is preserved and --host works in either position.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import path from 'node:path';
const cli = path.resolve('target/debug', process.platform === 'win32' ? 'rustx.exe' : 'rustx');
for (const args of [
  ['serve', 'dist', '0'],
  ['serve', 'dist', '0', '--host', '127.0.0.1'],
  ['serve', '--host', '127.0.0.1', 'dist', '0'],
]) {
  const child = spawn(cli, args, { stdio: ['ignore', 'pipe', 'pipe'] });
  try {
    const address = await new Promise((resolve, reject) => {
      let output = '';
      const timer = setTimeout(() => reject(Error('Server startup timeout')), 10000);
      child.on('error', (error) => {
        clearTimeout(timer);
        reject(error);
      });
      child.on('exit', (code) => {
        clearTimeout(timer);
        reject(Error(`Early exit ${code}: ${output}`));
      });
      child.stdout.on('data', (data) => {
        output += data;
        const match = output.match(/http:\/\/(127\.0\.0\.1:\d+)/);
        if (match) {
          clearTimeout(timer);
          resolve(match[1]);
        }
      });
      child.stderr.on('data', (data) => {
        output += data;
      });
    });
    const response = await fetch(`http://${address}/runtime-config.json`);
    assert.equal(response.status, 200);
    assert.equal((await response.json()).version, 1);
  } finally {
    child.kill();
  }
}
for (const args of [
  ['serve', '--host'],
  ['serve', 'dist', '0', '--host', 'not-an-ip'],
  ['serve', 'dist', '0', '--host', '127.0.0.1', '--host', '0.0.0.0'],
]) {
  const result = spawnSync(cli, args, { encoding: 'utf8', timeout: 10000 });
  assert.equal(result.status, 1, result.stderr);
}
console.log(
  'PASS: default loopback, --host positional ordering, invalid and duplicate bind options',
);

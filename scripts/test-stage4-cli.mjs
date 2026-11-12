import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import { execFileSync, spawn } from 'node:child_process';
import { chromium } from 'playwright';
import { compressedShellBytes, measureColdStartup } from './measure-startup.mjs';
const root = process.cwd(),
  cli = path.join(root, 'target/debug', process.platform === 'win32' ? 'rustx.exe' : 'rustx'),
  base = await fs.mkdtemp(path.join(os.tmpdir(), 'rustx-stage4-cli-')),
  project = path.join(base, 'sample');
const report = {
  date: new Date().toISOString(),
  checks: [],
  limitations: [
    "Uses this runner's dependency/toolchain cache; not an independent first-ever-download installation",
  ],
};
const env = { ...process.env, CARGO_TARGET_DIR: path.join(root, 'target') };
let child, browser;
const run = (args, cwd = project) => execFileSync(cli, args, { cwd, env, stdio: 'inherit' });
const waitForServer = async () => {
  const deadline = Date.now() + 120000;
  for (;;) {
    try {
      if ((await fetch('http://127.0.0.1:3010')).ok) return;
    } catch {}
    assert(Date.now() < deadline, 'server startup');
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
};
try {
  run(['new', project], base);
  assert((await fs.stat(project + '/vendor/crates/rustx-core/src/lib.rs')).isFile());
  assert(!(await fs.readdir(project + '/vendor/crates')).includes('rustx-cli'));
  report.checks.push('embedded SDK creates independent vendored project from unrelated directory');
  run(['doctor', '--json']);
  run(['fmt', '--check']);
  run(['check']);
  run(['test']);
  run(['build', '--target', 'web', '--release']);
  report.counterCompressedBytes = await compressedShellBytes(path.join(project, 'dist'));
  report.counterCompression = 'br';
  report.counterGzipBytes = await compressedShellBytes(path.join(project, 'dist'), 'gzip');
  report.counterTransferGate = report.counterCompressedBytes <= 3 * 1024 * 1024;
  report.project = project;
  report.checks.push('new, doctor, formatter, locked check/test and standalone web build');
  browser = await chromium.launch({
    headless: process.env.RUSTX_HEADED !== '1',
    executablePath:
      process.env.CHROME_PATH ||
      (process.platform === 'darwin'
        ? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
        : undefined),
  });
  child = spawn(cli, ['serve', 'dist', '3010'], { cwd: project, env, stdio: 'inherit' });
  await waitForServer();
  report.counterColdStartup = await measureColdStartup(
    browser,
    'http://127.0.0.1:3010/?renderer=cpu',
    'increase',
  );
  const stopped = new Promise((resolve) => child.once('exit', resolve));
  child.kill('SIGTERM');
  await stopped;
  child = spawn(cli, ['dev', '--target', 'web', '--port', '3010'], {
    cwd: project,
    env,
    stdio: 'inherit',
  });
  await waitForServer();
  const page = await browser.newPage();
  await page.goto('http://127.0.0.1:3010/?renderer=cpu');
  await page.getByTestId('increase').waitFor();
  const source = project + '/src/app.rustx',
    initial = await fs.readFile(source, 'utf8');
  await fs.writeFile(source, initial.replace('Hello from RustX', 'Reloaded edited Rust source'));
  await page.waitForFunction(
    () => window.rustx?.inspect().nodes.some((n) => n.text === 'Reloaded edited Rust source'),
    null,
    { timeout: 120000 },
  );
  report.checks.push('real source edit rebuilds Wasm and reloads browser with matching output');
  await fs.writeFile(source, initial.replace('cx.signal(0)', 'cx.signal(0); let _:u32="bad"'));
  await page.waitForSelector('#rustx-build-error', { timeout: 120000 });
  assert((await page.locator('#rustx-build-error').textContent()).includes('mismatched types'));
  await fs.writeFile(source, initial);
  await page.waitForFunction(
    () =>
      window.rustx?.inspect().nodes.some((n) => n.text === 'Hello from RustX') &&
      !document.querySelector('#rustx-build-error'),
    null,
    { timeout: 120000 },
  );
  report.checks.push('real Cargo error overlay and recovery after source correction');
  report.result = 'passed';
} catch (error) {
  report.result = 'failed';
  report.error = String(error);
  throw error;
} finally {
  child?.kill('SIGTERM');
  await browser?.close();
  await fs.writeFile(
    path.join(root, 'docs/evidence/stage4-cli.json'),
    JSON.stringify(report, null, 2) + '\n',
  );
  console.log(JSON.stringify(report, null, 2));
}

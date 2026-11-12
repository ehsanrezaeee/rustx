// Real Chromium presentation, API instrumentation, and initial-adapter-failure path.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { chromium } from 'playwright';
const browser = await chromium.launch({
  headless: process.env.RUSTX_HEADED !== '1',
  ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}),
  args: ['--enable-unsafe-webgpu'],
});
const evidence = new URL('../docs/evidence/', import.meta.url),
  errors = [];
const report = {
  date: '2026-11-01',
  browser: browser.version(),
  platform: process.platform,
  arch: process.arch,
  viewport: [1200, 1000],
  dpr: 1,
  checks: [],
};
try {
  const page = await browser.newPage({
    viewport: { width: 1200, height: 1000 },
    deviceScaleFactor: 1,
  });
  page.on('pageerror', (e) => errors.push(e.message));
  await page.addInitScript(() => {
    const s = (window.presentation = {
      textureCopiesToCPU: 0,
      mapReads: 0,
      submits: 0,
      presents: 0,
      cpuPixelTransfers: 0,
      adapters: [],
    });
    function watch(proto, name, callback) {
      if (!proto?.[name]) return;
      const fn = proto[name];
      proto[name] = function (...args) {
        callback(args);
        return fn.apply(this, args);
      };
    }
    watch(
      globalThis.GPUCommandEncoder?.prototype,
      'copyTextureToBuffer',
      () => s.textureCopiesToCPU++,
    );
    watch(globalThis.GPUBuffer?.prototype, 'mapAsync', (args) => {
      if (args[0] & 1) s.mapReads++;
    });
    watch(globalThis.GPUQueue?.prototype, 'submit', () => s.submits++);
    watch(globalThis.GPUCanvasContext?.prototype, 'getCurrentTexture', () => s.presents++);
    watch(
      globalThis.CanvasRenderingContext2D?.prototype,
      'putImageData',
      () => s.cpuPixelTransfers++,
    );
    if (navigator.gpu) {
      const fn = navigator.gpu.requestAdapter.bind(navigator.gpu);
      navigator.gpu.requestAdapter = async (...args) => {
        const adapter = await fn(...args);
        if (adapter) {
          const info = adapter.info || (await adapter.requestAdapterInfo?.());
          s.adapters.push(
            info
              ? {
                  vendor: info.vendor,
                  architecture: info.architecture,
                  device: info.device,
                  description: info.description,
                  isFallbackAdapter: adapter.isFallbackAdapter,
                }
              : {},
          );
        }
        return adapter;
      };
    }
  });
  await page.goto((process.env.RUSTX_URL || 'http://127.0.0.1:3000') + '/');
  await page.waitForFunction(() => window.rustx?.draws() > 0, null, { timeout: 60000 });
  assert.equal(
    await page.evaluate(() => window.rustx.renderer()),
    'gpu',
    'WebGPU presentation unavailable',
  );
  await page.getByTestId('nav-lab').click();
  await page.getByTestId('inc-ALPHA').click();
  await page.waitForFunction(
    () => document.querySelector('[data-testid="count-ALPHA"]')?.textContent === 'Count: 1',
  );
  await page.screenshot({ path: new URL('browser-gpu.png', evidence).pathname });
  await page.setViewportSize({ width: 900, height: 700 });
  await page.waitForFunction(() => document.querySelector('canvas').width === 900);
  const stats = await page.evaluate(() => window.presentation);
  assert(stats.submits > 0 && stats.presents > 0);
  assert.equal(stats.textureCopiesToCPU, 0);
  assert.equal(stats.cpuPixelTransfers, 0);
  const draws = await page.evaluate(() => window.rustx.draws());
  await page.waitForTimeout(300);
  assert.equal(await page.evaluate(() => window.rustx.draws()), draws);
  report.presentation = stats;
  report.checks.push(
    'real WebGPU canvas presentation, interaction, resize and idle',
    'no copyTextureToBuffer or Canvas2D putImageData during instrumented normal GPU path',
  );
  const fallback = await browser.newPage({ viewport: { width: 1200, height: 1000 } });
  await fallback.addInitScript(() => {
    if (navigator.gpu) navigator.gpu.requestAdapter = async () => null;
  });
  await fallback.goto((process.env.RUSTX_URL || 'http://127.0.0.1:3000') + '/');
  await fallback.waitForFunction(() => window.rustx?.draws() > 0);
  assert.equal(await fallback.evaluate(() => window.rustx.renderer()), 'cpu');
  assert.match(await fallback.locator('#status').textContent(), /GPU initialization failed/);
  await fallback.getByTestId('nav-lab').click();
  await fallback.getByTestId('inc-ALPHA').click();
  await fallback.waitForFunction(
    () => document.querySelector('[data-testid="count-ALPHA"]')?.textContent === 'Count: 1',
  );
  report.checks.push(
    'injected adapter absence triggers actual CPU renderer and usable counter; no device-loss claim',
  );
  assert.deepEqual(errors, []);
  report.result = 'passed';
  report.limitations = [
    'No Windows, Safari, mobile, IME or screen-reader task',
    'API counters are presentation evidence, not performance certification or full device-loss recovery',
  ];
} catch (error) {
  report.result = 'failed';
  report.error = String(error);
  throw error;
} finally {
  await fs.writeFile(new URL('browser-gpu.json', evidence), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report, null, 2));
  await browser.close();
}

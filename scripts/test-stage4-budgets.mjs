// Frozen desktop lab profile. Mobile/Windows and equivalent React/canvas baselines remain open.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { chromium } from 'playwright';
const base = process.env.RUSTX_URL || 'http://127.0.0.1:3012',
  backend = process.env.RUSTX_BACKEND || 'gpu',
  file = new URL(`../docs/evidence/stage4-budgets-${backend}.json`, import.meta.url);
let browser;
const report = {
  date: new Date().toISOString(),
  profile: {
    device: 'Apple M3 MacBook Air, 16 GiB',
    os: 'macOS 27.0.1 arm64',
    viewport: [1280, 720],
    dpr: 1,
    objects: 10000,
    visibleLimit: 1000,
    backend,
    samples: 5,
    framesPerSample: 180,
    refreshTargetHz: 60,
  },
  budgets: { frameP95Ms: 16.7, inputP95Ms: 100, lifecycleGrowthPercent: 10, cycles: 100 },
  limitations: [
    'Desktop local lab profile frozen for this candidate; original blueprint hardware freeze was incomplete',
    'Frame intervals are requestAnimationFrame observations; event response uses next animation frame after Rust result, not physical scanout',
    'Counting allocator includes framework/app allocations; Wasm capacity is recorded separately',
    'No mobile/iOS/Windows certification or equivalent React/canvas baseline; no universal speed claim',
  ],
  checks: [],
};
const percentile = (a, p) => [...a].sort((a, b) => a - b)[Math.max(0, Math.ceil(a.length * p) - 1)];
try {
  browser = await chromium.launch({
    headless: false,
    executablePath:
      process.env.CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    args: ['--enable-unsafe-webgpu'],
  });
  report.browser = browser.version();
  const context = await browser.newContext({
      viewport: { width: 1280, height: 720 },
      deviceScaleFactor: 1,
    }),
    page = await context.newPage();
  await page.addInitScript(() => {
    window.budgetGpu = { submits: 0, adapters: [] };
    const original = globalThis.GPUQueue?.prototype.submit;
    if (original)
      GPUQueue.prototype.submit = function (...args) {
        window.budgetGpu.submits++;
        return original.apply(this, args);
      };
    if (navigator.gpu) {
      const original = navigator.gpu.requestAdapter.bind(navigator.gpu);
      navigator.gpu.requestAdapter = async (...args) => {
        const a = await original(...args);
        if (a)
          window.budgetGpu.adapters.push(
            a.info
              ? {
                  vendor: a.info.vendor,
                  architecture: a.info.architecture,
                  description: a.info.description,
                }
              : {},
          );
        return a;
      };
    }
  });
  await page.goto(base + '/' + (backend === 'cpu' ? '?renderer=cpu' : '') + '#/engine');
  await page.waitForFunction(() => window.rustxBenchmark?.draws() > 0, null, { timeout: 60000 });
  assert.equal(await page.evaluate(() => !!window.rustx), false);
  report.wasmSha256 = createHash('sha256')
    .update(await (await page.request.get(base + '/pkg/rustx_lab_bg.wasm')).body())
    .digest('hex');
  // Focusing the canvas asks the Rust host to reveal its actual drawing rectangle.
  // An offscreen board can still submit a frame, which is not a drawing benchmark.
  await page.getByTestId('drawing-board').focus();
  await page.waitForFunction(() => window.rustxBenchmark.inspect().metrics.shapes_visible > 0);
  const board = await page.evaluate(() => window.rustxBenchmark.inspect());
  assert.equal(board.nodes.find((n) => n.id === 'drawing-board').drawing_count, 10000);
  assert(board.metrics.shapes_visible > 0 && board.metrics.shapes_visible <= 1000);
  report.visible = board.metrics.shapes_visible;
  report.frameSamples = [];
  for (let run = 0; run < 5; run++) {
    const sample = await page.evaluate(async () => {
      const api = window.rustxBenchmark,
        id = api.inspect().nodes.find((n) => n.id === 'drawing-board').node,
        raw = [];
      let last = await new Promise(requestAnimationFrame);
      for (let n = 0; n < 180; n++) {
        api.pan(id, n % 120 < 60 ? 1 : -1, 0.25);
        const time = await new Promise(requestAnimationFrame);
        raw.push(time - last);
        last = time;
      }
      return { raw, p95: [...raw].sort((a, b) => a - b)[170] };
    });
    report.frameSamples.push(sample);
  }
  report.frameP95Ms = percentile(
    report.frameSamples.flatMap((s) => s.raw),
    0.95,
  );
  report.gpu = await page.evaluate(() => window.budgetGpu);
  if (backend === 'gpu') assert(report.gpu.submits > 0, 'actual GPU submissions required');
  report.inputRawMs = await page.evaluate(async () => {
    const raw = [],
      button = document.querySelector('[data-testid=board-right]');
    for (let n = 0; n < 30; n++) {
      const start = performance.now();
      button.click();
      const before = window.rustxBenchmark.draws();
      while (window.rustxBenchmark.draws() === before) await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
      raw.push(performance.now() - start);
    }
    return raw;
  });
  report.inputP95Ms = percentile(report.inputRawMs, 0.95);
  await page.evaluate(() => (location.hash = '/'));
  await page.waitForSelector('[data-testid=nav-home]');
  const settle = () => page.evaluate(() => new Promise((r) => setTimeout(r, 70)));
  const route = async (path) => {
    await page.evaluate((path) => (location.hash = path), path);
    await settle();
  };
  for (let n = 0; n < 5; n++) {
    await route('/engine');
    await route('/people');
    await route('/applications');
    await route('/');
  }
  report.memoryBaseline = await page.evaluate(() => ({
    counts: window.rustxBenchmark.inspect().counts,
    allocations: window.rustxBenchmark.allocations(),
    capacity: window.rustxBenchmark.capacity(),
    metrics: window.rustxBenchmark.inspect().metrics,
  }));
  report.lifecycle = [];
  for (let n = 0; n < 100; n++) {
    await route('/engine');
    await route('/people');
    await page.getByTestId('load-slow').evaluate((e) => e.click());
    await route('/applications');
    await route('/');
    if (n % 10 === 9)
      report.lifecycle.push({
        cycle: n + 1,
        ...(await page.evaluate(() => ({
          counts: window.rustxBenchmark.inspect().counts,
          allocations: window.rustxBenchmark.allocations(),
          capacity: window.rustxBenchmark.capacity(),
        }))),
      });
  }
  for (const row of [report.memoryBaseline, ...report.lifecycle]) {
    const raw = row.allocations;
    assert(Array.isArray(raw) && raw[0] > 0, 'actual allocator bytes required');
    row.allocations = { live_bytes: raw[0], peak_bytes: raw[1], live_blocks: raw[2] };
  }
  report.memoryFinal = report.lifecycle.at(-1);
  report.liveGrowthPercent =
    (report.memoryFinal.allocations.live_bytes / report.memoryBaseline.allocations.live_bytes - 1) *
    100;
  report.countsReturned =
    JSON.stringify(report.memoryBaseline.counts) === JSON.stringify(report.memoryFinal.counts);
  await settle();
  const idle = await page.evaluate(() => ({
    draws: window.rustxBenchmark.draws(),
    submits: window.budgetGpu.submits,
  }));
  await page.waitForTimeout(700);
  const after = await page.evaluate(() => ({
    draws: window.rustxBenchmark.draws(),
    submits: window.budgetGpu.submits,
  }));
  report.idle = { before: idle, after, passed: JSON.stringify(idle) === JSON.stringify(after) };
  report.gates = {
    desktopGpuFrame: backend === 'gpu' ? report.frameP95Ms <= 16.7 : null,
    inputEstimate: report.inputP95Ms <= 100,
    lifecycleCounts: report.countsReturned,
    lifecycleAllocation: report.liveGrowthPercent <= 10,
    idle: report.idle.passed,
  };
  report.result = Object.values(report.gates).every((v) => v !== false)
    ? 'passed-local-measured-gates'
    : 'failed-local-gates';
} catch (error) {
  report.result = 'failed';
  report.error = String(error);
  throw error;
} finally {
  await fs.writeFile(file, JSON.stringify(report, null, 2) + '\n');
  console.log(
    JSON.stringify(
      { ...report, frameSamples: report.frameSamples?.map((s) => ({ p95: s.p95 })) },
      null,
      2,
    ),
  );
  await browser?.close();
}
if (report.result !== 'passed-local-measured-gates') process.exitCode = 1;

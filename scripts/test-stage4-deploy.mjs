import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { chromium, webkit } from 'playwright';
import { compressedShellBytes, measureColdStartup } from './measure-startup.mjs';
const base = process.env.RUSTX_URL || 'http://127.0.0.1:3011',
  engine = process.env.RUSTX_BROWSER || 'chromium',
  root = process.env.RUSTX_DIST || 'dist-release',
  file = `docs/evidence/stage4-deploy-${engine}.json`;
let browser;
const report = {
  date: new Date().toISOString(),
  engine,
  checks: [],
  limitations: [
    'Loopback delivery; no public HTTPS deployment',
    'Offline application shell preserves no implicit API cache',
    'Network throttling estimates navigation to usable frame; no physical scanout measurement',
  ],
};
try {
  const manifest = JSON.parse(await fs.readFile(root + '/assets.json'));
  report.build = manifest.build;
  for (const [name, digest] of Object.entries(manifest.files)) {
    assert.equal(
      createHash('sha256')
        .update(await fs.readFile(root + '/' + name))
        .digest('hex'),
      digest,
    );
  }
  const wasm = Object.keys(manifest.files).find((p) => /\.[a-f0-9]{12}\.wasm$/.test(p)),
    host = Object.keys(manifest.files).find((p) => /^host\.[a-f0-9]{12}\.js$/.test(p));
  assert(wasm && host);
  for (const [accept, encoding] of [
    ['br, gzip', 'br'],
    ['br;q=0, gzip', 'gzip'],
    ['br;q=0.2, gzip;q=1', 'gzip'],
    ['br;q=0, gzip;q=0', null],
  ]) {
    const response = await fetch(base + '/' + wasm, { headers: { 'accept-encoding': accept } });
    assert.equal(response.headers.get('content-encoding'), encoding);
    assert.equal(
      createHash('sha256')
        .update(Buffer.from(await response.arrayBuffer()))
        .digest('hex'),
      manifest.files[wasm],
    );
  }
  const res = await fetch(base + '/' + wasm, { headers: { 'accept-encoding': 'gzip' } });
  assert.equal(res.headers.get('content-type'), 'application/wasm');
  assert.equal(res.headers.get('content-encoding'), 'gzip');
  assert.match(res.headers.get('cache-control'), /immutable/);
  assert.match(res.headers.get('content-security-policy'), /object-src 'none'/);
  assert.equal(res.headers.get('x-content-type-options'), 'nosniff');
  const etag = res.headers.get('etag');
  assert.equal(
    (await fetch(base + '/' + wasm, { headers: { 'if-none-match': etag } })).status,
    304,
  );
  assert.match((await fetch(base + '/')).headers.get('cache-control'), /no-cache/);
  assert.equal((await fetch(base + '/not-here.wasm')).status, 404);
  report.checks.push(
    'manifest SHA256, hash names, Wasm MIME, gzip, immutable assets, revalidated shell/ETag304, CSP/nosniff and missing-asset404',
  );
  browser = await { chromium, webkit }[engine].launch({
    headless: process.env.RUSTX_HEADED !== '1',
    ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}),
  });
  report.version = browser.version();
  if (engine === 'chromium') {
    report.coldStartup = await measureColdStartup(
      browser,
      base + '/?renderer=cpu#/people',
      'load-fixture',
    );
    report.coldStartupMs = report.coldStartup.p95Ms;
    report.coldDesktopGate = report.coldStartup.passed;
  }
  const context = await browser.newContext({
      viewport: { width: 1280, height: 720 },
      serviceWorkers: 'allow',
    }),
    page = await context.newPage();
  let cdp;
  if (engine === 'chromium') {
    cdp = await context.newCDPSession(page);
    await cdp.send('Network.enable');
    await cdp.send('Network.setCacheDisabled', { cacheDisabled: true });
  }
  const started = Date.now();
  await page.goto(base + '/?renderer=cpu#/people');
  await page.waitForSelector('[data-testid=load-fixture]', { timeout: 60000 });
  if (engine !== 'chromium') {
    report.unthrottledObservationMs = Date.now() - started;
    report.coldDesktopGate = null;
  }
  assert.equal(await page.locator(`link[rel=preload][href="/${wasm}"]`).count(), 1);
  assert.equal(await page.evaluate(() => window.rustx), undefined);
  assert.equal(await page.locator('#rustx-devtools').count(), 0);
  assert.equal(await page.request.get(base + '/inspector.html').then((r) => r.status()), 404);
  report.checks.push(
    'production disables devtools/API; cold navigation to usable semantic control measured',
  );
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
    if (!navigator.serviceWorker.controller)
      await new Promise((resolve) =>
        navigator.serviceWorker.addEventListener('controllerchange', resolve, { once: true }),
      );
  });
  if (cdp)
    await cdp.send('Network.emulateNetworkConditions', {
      offline: true,
      latency: 0,
      downloadThroughput: -1,
      uploadThroughput: -1,
    });
  await context.setOffline(true);
  await page.goto(base + '/people/deep/42?renderer=cpu#/people');
  await page.getByTestId('load-fixture').waitFor();
  await page.getByTestId('load-fixture').focus();
  await page.evaluate(
    () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))),
  );
  await page.getByTestId('load-fixture').click();
  await page.waitForFunction(
    () =>
      [
        document.querySelector('[data-testid=api-state]')?.textContent,
        document.querySelector('[data-testid=api-state]')?.value,
        document.querySelector('[data-testid=api-state]')?.getAttribute('aria-label'),
      ].some((v) => v?.includes('Network unavailable')),
    null,
    { timeout: 15000 },
  );
  report.offlineApi = await page.getByTestId('api-state').evaluate((e) => ({
    text: e.textContent,
    value: e.value,
    label: e.getAttribute('aria-label'),
  }));
  if (cdp)
    await cdp.send('Network.emulateNetworkConditions', {
      offline: false,
      latency: 0,
      downloadThroughput: -1,
      uploadThroughput: -1,
    });
  await context.setOffline(false);
  await page.reload();
  await page.getByTestId('load-fixture').waitFor();
  report.checks.push(
    'service-worker cached application launches on offline nested route; API honestly fails; online deep-link reload recovers',
  );
  const transfer = await compressedShellBytes(root);
  report.compressedInitialBytes = transfer;
  report.compression = 'br';
  report.gzipInitialBytes = await compressedShellBytes(root, 'gzip');
  report.transferScope =
    'Complete reference dashboard; standalone counter is measured by test:stage4:cli';
  report.transferGates = {
    dashboard5MiB: transfer <= 5 * 1024 * 1024,
  };
  report.result = 'passed-functional';
} catch (error) {
  report.result = 'failed';
  report.error = String(error);
  throw error;
} finally {
  await fs.writeFile(file, JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report, null, 2));
  await browser?.close();
}

// Cold CPU startup: independent contexts, no service worker/cache, real CDP throttling.
// Endpoint is a usable semantic control, not physical display scanout.
import assert from 'node:assert/strict';

export async function measureColdStartup(browser, url, control, samples = 5) {
  const raw = [];
  const encodings = [];
  for (let index = 0; index < samples; index++) {
    const context = await browser.newContext({
      viewport: { width: 1280, height: 720 },
      deviceScaleFactor: 1,
      serviceWorkers: 'block',
    });
    try {
      const page = await context.newPage();
      const cdp = await context.newCDPSession(page);
      await cdp.send('Network.enable');
      await cdp.send('Network.setCacheDisabled', { cacheDisabled: true });
      await cdp.send('Network.emulateNetworkConditions', {
        offline: false,
        latency: 100,
        downloadThroughput: 10_000_000 / 8,
        uploadThroughput: 2_000_000 / 8,
      });
      const start = performance.now();
      const module = page.waitForResponse((response) =>
        new URL(response.url()).pathname.endsWith('.wasm'),
      );
      await page.goto(url);
      await page.getByTestId(control).waitFor({ timeout: 60000 });
      assert(await page.getByTestId(control).isEnabled());
      raw.push(performance.now() - start);
      encodings.push((await module).headers()['content-encoding'] || 'identity');
      // A preload must be consumed by bindgen, rather than downloading a second module.
      const wasm = await page.evaluate(() =>
        performance
          .getEntriesByType('resource')
          .filter((entry) => new URL(entry.name).pathname.endsWith('.wasm'))
          .map((entry) => entry.name),
      );
      assert.equal(wasm.length, 1, 'One initial Wasm transfer, shared with preload');
    } finally {
      await context.close();
    }
  }
  const sorted = [...raw].sort((a, b) => a - b);
  return {
    rawMs: raw,
    encodings,
    medianMs: sorted[Math.floor(sorted.length / 2)],
    p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
    budgetMs: 3000,
    passed: sorted[Math.ceil(sorted.length * 0.95) - 1] <= 3000,
    method:
      'Fresh Chromium contexts, CPU renderer, no cache/SW, 10 Mbps down / 100 ms RTT; first usable semantic control',
    limitations:
      'Automation timing; hardware/power/device matrix and physical presentation remain separate acceptance',
  };
}

export async function compressedShellBytes(root, encoding = 'br') {
  const fs = await import('node:fs/promises');
  const path = await import('node:path');
  const worker = await fs.readFile(path.join(root, 'service-worker.js'), 'utf8');
  const files = JSON.parse(worker.match(/const FILES\s*=\s*(\[.*?\]);/)[1]);
  let total = 0;
  for (const file of files)
    total += (
      await fs.stat(path.join(root, file.slice(1) + '.' + (encoding === 'br' ? 'br' : 'gz')))
    ).size;
  return total;
}

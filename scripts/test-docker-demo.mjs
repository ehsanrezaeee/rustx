// Requires a running Compose demo; exercises the real container-served production bundle.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { chromium } from 'playwright';
const base = process.env.RUSTX_URL || 'http://127.0.0.1:3000';
const output = new URL('../docs/evidence/docker-demo/', import.meta.url);
const report = { date: new Date().toISOString(), base, checks: [] };
await fs.mkdir(output, { recursive: true });
const config = await (await fetch(base + '/runtime-config.json')).json();
assert.equal(config.devtools, false);
assert.equal(config.compiler, null);
const browser = await chromium.launch({ headless: true });
try {
  report.browser = browser.version();
  const page = await browser.newPage({
    viewport: { width: 1280, height: 900 },
    serviceWorkers: 'block',
  });
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  const visit = async (route, id) => {
    await page.goto(`${base}/?renderer=cpu#${route}`);
    await page.getByTestId(id).waitFor();
    assert.equal(await page.evaluate(() => typeof window.rustx), 'undefined');
    assert.equal(await page.locator('#status').count(), 0);
  };
  const text = (value) =>
    page.waitForFunction(
      (value) => document.querySelector('#semantic').textContent.includes(value),
      value,
    );
  await visit('/capabilities', 'capabilities-title');
  assert(
    await page.locator('#surface').evaluate((canvas) => canvas.width > 100 && canvas.height > 100),
  );
  await visit('/people', 'load-fixture');
  await page.getByTestId('load-fixture').click();
  await text('6 people');
  await page.getByTestId('load-error').click();
  await text('HTTP 503');
  await page.getByTestId('load-slow').click();
  await page.getByTestId('cancel').click();
  await text('Cancelled');
  report.checks.push('production canvas deep links and container fixture API success/error/cancel');
  await visit('/applications', 'profile-name');
  await page.getByTestId('profile-name').fill('Docker demo');
  assert.equal(await page.getByTestId('profile-name').inputValue(), 'Docker demo');
  await visit('/engine', 'drawing-board');
  report.checks.push('Applications editing and Visual engine route load');
  await visit('/store', 'store-publish');
  await page.getByTestId('store-publish').click();
  await text('revision 1: 1 / 2');
  await page.getByTestId('store-toggle').click();
  await page.getByTestId('store-value').waitFor({ state: 'detached' });
  await page.getByTestId('store-toggle').click();
  await text('revision 1: 1 / 2');
  report.checks.push('external-store publication and reader unmount/remount');
  await page.screenshot({ path: new URL('demo-chromium.png', output).pathname });
  assert.deepEqual(errors, []);
  report.result = 'pass';
} catch (error) {
  report.result = 'fail';
  report.error = String(error.stack || error);
  throw error;
} finally {
  await fs.writeFile(new URL('browser.json', output), JSON.stringify(report, null, 2) + '\n');
  await browser.close();
}
console.log(JSON.stringify(report, null, 2));

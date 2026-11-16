import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { chromium, webkit } from 'playwright';
const engine = process.env.RUSTX_BROWSER || 'chromium';
const base = process.env.RUSTX_URL || 'http://127.0.0.1:3039';
const output = new URL('../docs/evidence/react-parity/RP-004/', import.meta.url);
const report = { engine, date: new Date().toISOString(), checks: [] };
const browser = await { chromium, webkit }[engine].launch({
  headless: true,
  ...(engine === 'chromium' ? { ignoreDefaultArgs: ['--disable-back-forward-cache'] } : {}),
});
try {
  report.version = browser.version();
  const context = await browser.newContext({ serviceWorkers: 'block' });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.addInitScript(() => {
    window.documentToken = Math.random();
    addEventListener('pageshow', (event) => {
      window.restoredFromCache = event.persisted;
    });
  });
  await page.goto(`${base}/?renderer=cpu&dom-audit=1#/store`);
  await page.getByTestId('store-publish').waitFor();
  assert.equal(await page.locator('#status').count(), 0);
  assert.equal(await page.evaluate(() => typeof window.rustx), 'undefined');
  assert.equal(await page.locator('[data-rustx-dom="tooling"]').count(), 0);
  const check = (value) =>
    page.waitForFunction(
      (value) => document.querySelector('#semantic').textContent.includes(value),
      value,
    );
  await page.getByTestId('store-publish').click();
  await check('revision 1: 1 / 2');
  report.checks.push(
    'actual production bundle renders canvas with working semantic input; no status/tooling/audit API even when requested',
  );
  const token = await page.evaluate(() => window.documentToken);
  await page.goto(base + '/docs/');
  await page.goBack();
  await page.getByTestId('store-publish').waitFor();
  report.bfcache = await page.evaluate(
    (token) => ({
      persisted: window.restoredFromCache,
      sameDocument: window.documentToken === token,
    }),
    token,
  );
  if (report.bfcache.persisted) {
    assert(report.bfcache.sameDocument);
    await check('revision 1: 1 / 2');
    await page.getByTestId('store-publish').click();
    await check('revision 2: 2 / 4');
    report.checks.push(
      'actual back-navigation BFCache restore preserves Rust state and semantic input',
    );
  } else
    report.checks.push(
      'browser performed fresh back navigation; no actual BFCache success claimed for this engine',
    );
  assert.deepEqual(errors, []);
  const failed = await context.newPage();
  await failed.route('**/*.wasm', (route) => route.abort());
  await failed.goto(base + '/?renderer=cpu');
  await failed.getByRole('button', { name: 'Reload application' }).waitFor();
  assert.equal(await failed.locator('#semantic > *').count(), 0);
  assert.equal(await failed.locator('#status').count(), 0);
  report.checks.push(
    'blocked Wasm startup retains accessible emergency reload UI without ordinary DOM widgets',
  );
  report.result = 'pass';
} catch (e) {
  report.result = 'fail';
  report.error = String(e.stack || e);
  throw e;
} finally {
  await fs.writeFile(
    new URL(`production-${engine}.json`, output),
    JSON.stringify(report, null, 2) + '\n',
  );
  await browser.close();
}
console.log(JSON.stringify(report, null, 2));

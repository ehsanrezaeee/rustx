import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { chromium, webkit } from 'playwright';
const engine = process.env.RUSTX_BROWSER || 'chromium';
const base = process.env.RUSTX_URL || 'http://127.0.0.1:3000';
const output = new URL('../docs/evidence/react-parity/RP-004/', import.meta.url);
const report = { engine, date: new Date().toISOString(), checks: [] };
const browser = await { chromium, webkit }[engine].launch({ headless: true });
try {
  report.version = browser.version();
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto(`${base}/?renderer=cpu&dom-audit=1#/people`);
  await page.waitForFunction(() => window.rustx?.draws() > 0);
  await page.getByTestId('load-slow').evaluate((el) => el.click());
  await page.waitForFunction(() => window.rustx.hostState().requests > 0);
  const state = await page.evaluate(() => {
    const api = window.rustx;
    dispatchEvent(new PageTransitionEvent('pagehide', { persisted: false }));
    return api.hostState();
  });
  assert.equal(state.disposed, true);
  assert.equal(state.requests, 0);
  await page.evaluate(() => window.rustx.restart());
  await page.waitForFunction(() => window.rustx?.draws() > 0);
  await page.waitForTimeout(1600);
  assert.deepEqual(errors, []);
  report.checks.push(
    'real pending HTTP request is cancelled on non-persisted pagehide; restart stays live after old response deadline',
  );
  const audit = await page.evaluate(() => {
    const before = window.rustx.dom();
    const unknown = document.createElement('button');
    document.body.append(unknown);
    const during = window.rustx.dom();
    unknown.remove();
    const emergency = document.createElement('p');
    document.querySelector('#fatal').append(emergency);
    emergency.remove(); // both records delivered after the node has been detached
    const after = window.rustx.dom();
    return { before, during, after };
  });
  assert.equal(audit.during.counts.unregistered, audit.before.counts.unregistered + 1);
  assert.deepEqual(audit.after.counts, audit.before.counts);
  assert.equal(audit.after.mutations.emergency.added - audit.before.mutations.emergency.added, 1);
  assert.equal(
    audit.after.mutations.emergency.removed - audit.before.mutations.emergency.removed,
    1,
  );
  report.checks.push(
    'accounting detects unregistered controls and classifies transient detached emergency nodes',
  );
  // Actual DOM/event adapter test with a stub receiver, not a compiled playground application.
  await page.route('**/rp004-preview.js', async (route) =>
    route.fulfill({
      contentType: 'text/javascript',
      body: await fs.readFile(new URL('../web/playground.js', import.meta.url), 'utf8'),
    }),
  );
  await page.route('**/jobs/*/preview.html', (route) =>
    route.fulfill({
      contentType: 'text/html',
      body: '<!doctype html><title>Adapter fixture</title>',
    }),
  );
  const result = await page.evaluate(async (base) => {
    const { install } = await import('/rp004-preview.js');
    let callbacks = 0;
    const app = {
      focus() {},
      input() {
        callbacks++;
      },
      inspect() {
        callbacks++;
        return JSON.stringify({ nodes: [] });
      },
    };
    const job = '0'.repeat(32);
    const trigger = document.createElement('button');
    trigger.dataset.testid = 'playground-go-error';
    document.body.append(trigger);
    for (let n = 0; n < 100; n++) {
      const preview = install(app, { compiler: base }, () => {});
      preview.sync({
        nodes: [
          {
            id: 'playground-surface',
            node: 1,
            text: `${base}/jobs/${job}/preview.html`,
            rect: [0, 0, 100, 100],
            visible_rect: [0, 0, 100, 100],
          },
        ],
      });
      const frame = document.querySelector('#rustx-compiled-output');
      const event = new MessageEvent('message', {
        source: frame.contentWindow,
        origin: 'null',
        data: { type: 'rustx-preview-error', job, message: 'fixture' },
      });
      dispatchEvent(event);
      preview.dispose();
      preview.dispose();
      dispatchEvent(event);
      trigger.click();
      preview.sync({ nodes: [] });
      if (document.querySelector('#rustx-compiled-output')) throw Error('retained preview iframe');
    }
    trigger.remove();
    return callbacks;
  }, base);
  assert.equal(result, 100);
  report.checks.push(
    '100 preview adapter mount/dispose cycles: one live callback, no stale message/click callbacks or iframe; stub Rust receiver only',
  );
  assert.deepEqual(errors, []);
  report.result = 'pass';
} catch (e) {
  report.result = 'fail';
  report.error = String(e.stack || e);
  throw e;
} finally {
  await fs.writeFile(
    new URL(`ownership-${engine}.json`, output),
    JSON.stringify(report, null, 2) + '\n',
  );
  await browser.close();
}
console.log(JSON.stringify(report, null, 2));

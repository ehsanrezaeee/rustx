import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { chromium, firefox, webkit } from 'playwright';
const name = process.env.RUSTX_BROWSER || 'chromium';
const browser = await { chromium, firefox, webkit }[name].launch({
  headless: process.env.RUSTX_HEADED !== '1',
  ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}),
});
const evidence = new URL('../docs/evidence/', import.meta.url),
  errors = [];
let page,
  checks = [];
try {
  page = await browser.newPage({ viewport: { width: 1200, height: 1000 } });
  page.setDefaultTimeout(10000);
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto((process.env.RUSTX_URL || 'http://127.0.0.1:3000') + '/?renderer=cpu');
  await page.waitForFunction(() => window.rustx?.draws() > 0);
  const count = async (n) =>
    page.waitForFunction(
      (n) => document.querySelector('[data-testid="count-ALPHA"]')?.textContent === `Count: ${n}`,
      n,
    );
  const text = async (id, part) =>
    page.waitForFunction(
      ({ id, part }) => document.querySelector(`[data-testid="${id}"]`)?.textContent.includes(part),
      { id, part },
    );
  const top = async () => {
    await page.mouse.wheel(0, -100000);
    await page.waitForFunction(
      () => window.rustx.inspect().nodes.find((n) => n.id === 'nav-home').rect[1] >= 0,
    );
  };
  const frame = async () =>
    page.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    );
  await page.getByTestId('nav-lab').click();
  await count(0);
  await page.getByTestId('inc-ALPHA').click();
  await count(1);
  await page.getByTestId('inc-ALPHA').focus();
  await page.keyboard.press('Enter');
  await count(2);
  await page.keyboard.press('Space');
  await count(3);
  await frame();
  await count(3);
  checks.push('pointer, Enter and Space each activate once');
  await page.getByTestId('edit-ALPHA').fill('سلام keyed task');
  await frame();
  const before = await page.getByTestId('edit-ALPHA').getAttribute('data-node');
  await page.getByTestId('edit-ALPHA').evaluate((el) => {
    el.focus();
    el.setSelectionRange(2, 5);
  });
  // HTMLElement.click exercises activation without moving focus from the editor.
  await page.getByTestId('reverse').evaluate((el) => el.click());
  await frame();
  assert.equal(await page.getByTestId('edit-ALPHA').getAttribute('data-node'), before);
  assert.equal(await page.getByTestId('edit-ALPHA').inputValue(), 'سلام keyed task');
  assert.deepEqual(
    await page
      .getByTestId('edit-ALPHA')
      .evaluate((el) => [el === document.activeElement, el.selectionStart, el.selectionEnd]),
    [true, 2, 5],
  );
  await count(3);
  checks.push('keyed text/counter state, active input and selection survive reorder');
  await page.getByTestId('toggle').focus();
  await page.keyboard.press('Tab');
  assert.equal(
    await page.getByTestId('inc-BETA').evaluate((el) => el === document.activeElement),
    true,
  );
  checks.push('Tab follows reordered committed tree');
  await page.getByTestId('toggle').click();
  await page.waitForFunction(() => !document.querySelector('[data-testid="edit-ALPHA"]'));
  await page.getByTestId('toggle').click();
  await count(0);
  assert.equal(await page.getByTestId('edit-ALPHA').inputValue(), 'Task ALPHA');
  checks.push('unmount disposes keyed text and counter');
  await page.getByTestId('name-input').focus();
  await frame();
  await page.getByTestId('name-input').fill('سلام RustX');
  await text('echo', 'سلام RustX');
  assert.equal(
    await page.getByTestId('name-input').evaluate((el) => el === document.activeElement),
    true,
  );
  const rect = await page.getByTestId('name-input').boundingBox();
  assert(rect.y >= 0 && rect.y + rect.height <= 1001);
  assert.equal(await page.locator('#semantic').evaluate((el) => el.scrollTop), 0);
  checks.push('browser text editing, retained focus and Rust scroll-to-focus (no OS IME claim)');
  await top();
  await page.getByTestId('nav-people').click();
  await page.getByTestId('load-fixture').click();
  await text('api-state', '6 people');
  await page.getByTestId('load-error').click();
  await text('api-state', 'HTTP 503');
  await page.getByTestId('load-slow').click();
  await page.getByTestId('cancel').click();
  await text('api-state', 'Cancelled');
  await page.waitForTimeout(1400);
  await text('api-state', 'Cancelled');
  await page.getByTestId('load-slow').click();
  await page.getByTestId('load-fixture').click();
  await text('api-state', '6 people');
  await page.waitForTimeout(1400);
  await text('api-state', '6 people');
  await page.getByTestId('load-slow').click();
  await page.getByTestId('nav-home').click();
  await page.waitForTimeout(1400);
  assert.equal(await page.getByTestId('api-state').count(), 0);
  checks.push('real fixture Fetch success, HTTP failure, cancellation, supersede and disposal');
  const a = await page.evaluate(() => window.rustx.draws());
  await page.waitForTimeout(300);
  assert.equal(await page.evaluate(() => window.rustx.draws()), a);
  checks.push('idle has no repeated canvas frames');
  await page.setViewportSize({ width: 390, height: 844 });
  await frame();
  assert.equal(
    await page.evaluate(() =>
      window.rustx
        .inspect()
        .nodes.filter((n) => n.id?.startsWith('nav-'))
        .every((n) => n.rect[0] + n.rect[2] <= 391),
    ),
    true,
  );
  await page.screenshot({ path: new URL(`browser-${name}-390.png`, evidence).pathname });
  const scaled = await browser.newContext({
    viewport: { width: 390, height: 844 },
    deviceScaleFactor: 2,
  });
  const p2 = await scaled.newPage();
  await p2.goto((process.env.RUSTX_URL || 'http://127.0.0.1:3000') + '/?renderer=cpu');
  await p2.waitForFunction(() => window.rustx?.draws() > 0);
  assert.deepEqual(await p2.locator('canvas').evaluate((el) => [el.width, el.height]), [780, 1688]);
  await scaled.close();
  await page.getByTestId('nav-lab').click();
  await page.getByTestId('name-input').focus();
  await frame();
  const mobileRect = await page.getByTestId('name-input').boundingBox();
  assert(mobileRect.y >= 0 && mobileRect.y + mobileRect.height <= 845);
  await top();
  assert.equal(
    await page.getByTestId('name-input').evaluate((el) => el === document.activeElement),
    true,
  );
  checks.push(
    '390px viewport, DPR 2 sizing and scrolling away from a focused editor; no mobile-device claim',
  );
  assert.deepEqual(errors, []);
  const wasm = await page.request.get(
    (process.env.RUSTX_URL || 'http://127.0.0.1:3000') + '/pkg/rustx_lab_bg.wasm',
  );
  assert(wasm.ok());
  const report = {
    result: 'passed',
    date: new Date().toISOString(),
    engine: name,
    version: browser.version(),
    platform: process.platform,
    arch: process.arch,
    backend: 'Rust CPU',
    wasmSha256: createHash('sha256')
      .update(await wasm.body())
      .digest('hex'),
    checks,
    limitations: ['No real OS IME, screen reader, Windows or mobile-device tasks'],
  };
  await fs.writeFile(
    new URL(`browser-${name}.json`, evidence),
    JSON.stringify(report, null, 2) + '\n',
  );
  console.log(JSON.stringify(report, null, 2));
} catch (error) {
  if (page) {
    await page
      .screenshot({ path: new URL(`browser-${name}-failure.png`, evidence).pathname })
      .catch(() => {});
    await fs.writeFile(
      new URL(`browser-${name}-failure.json`, evidence),
      JSON.stringify(
        {
          error: String(error),
          errors,
          checks,
          inspection: await page.evaluate(() => window.rustx?.inspect()).catch(() => null),
        },
        null,
        2,
      ),
    );
  }
  throw error;
} finally {
  await browser.close();
}

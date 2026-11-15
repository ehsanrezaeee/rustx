// Real CPU canvas/Wasm interaction. Semantic DOM is only the input/accessibility bridge.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { chromium, webkit } from 'playwright';

const engine = process.env.RUSTX_BROWSER || 'chromium';
const base = process.env.RUSTX_URL || 'http://127.0.0.1:3000';
const output = new URL('../docs/evidence/react-parity/RP-006/', import.meta.url);
const report = { date: new Date().toISOString(), engine, checks: [] };
const browser = await { chromium, webkit }[engine].launch({
  headless: true,
  ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}),
});
try {
  report.version = browser.version();
  const page = await browser.newPage({ viewport: { width: 1200, height: 1000 } });
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(`${base}/?renderer=cpu#/store`);
  await page.waitForFunction(() => window.rustx?.draws() > 0);
  report.wasmSha256 = createHash('sha256')
    .update(await (await page.request.get(base + '/pkg/rustx_lab_bg.wasm')).body())
    .digest('hex');
  const expectValue = async (text) => {
    await page.waitForFunction(
      (text) => window.rustx.inspect().nodes.some((n) => n.text === text),
      text,
    );
  };
  const click = async (id) => {
    const rect = await page.evaluate(
      (id) => window.rustx.inspect().nodes.find((n) => n.id === id).rect,
      id,
    );
    await page.mouse.click(rect[0] + rect[2] / 2, rect[1] + rect[3] / 2);
  };
  await expectValue('revision 0: 0 / 0');
  await click('store-publish');
  await expectValue('revision 1: 1 / 2');
  await click('store-switch');
  await expectValue('revision 0: 100 / 200');
  await click('store-publish');
  await click('store-switch');
  await expectValue('revision 2: 2 / 4');
  report.checks.push('canvas pointer publication, source replacement and latest source snapshot');
  await click('store-toggle');
  await page.waitForFunction(
    () => !window.rustx.inspect().nodes.some((n) => n.id === 'store-value'),
  );
  await click('store-publish');
  await click('store-toggle');
  await expectValue('revision 3: 3 / 6');
  report.checks.push('unmount, publish without subscriber, remount with latest coherent pair');
  await page.screenshot({ path: new URL(`store-${engine}.png`, output).pathname });
  assert.deepEqual(errors, []);
  report.result = 'pass';
} catch (error) {
  report.result = 'fail';
  report.error = String(error.stack || error);
  throw error;
} finally {
  await fs.mkdir(output, { recursive: true });
  await fs.writeFile(
    new URL(`browser-${engine}.json`, output),
    JSON.stringify(report, null, 2) + '\n',
  );
  await browser.close();
}
console.log(JSON.stringify(report, null, 2));

// Real browser navigation/layout checks; no physical mobile or assistive-technology claim.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { chromium, webkit } from 'playwright';

const engine = process.env.RUSTX_BROWSER || 'chromium';
const base = process.env.RUSTX_URL || 'http://127.0.0.1:3000';
const output = new URL('../docs/evidence/completion-2026-11-13/', import.meta.url);
const report = {
  date: new Date().toISOString(),
  engine,
  checks: [],
  limitations: [
    'Desktop responsive emulation; no physical mobile, OS IME or human screen-reader acceptance',
  ],
};
const browser = await { chromium, webkit }[engine].launch({
  headless: process.env.RUSTX_HEADED !== '1',
  ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}),
});
let page;
try {
  report.version = browser.version();
  page = await browser.newPage({ viewport: { width: 1200, height: 1000 } });
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.addInitScript(() => {
    const original = CanvasRenderingContext2D.prototype.putImageData;
    CanvasRenderingContext2D.prototype.putImageData = function (...args) {
      if (!window.firstRustFrameIds && window.rustx) {
        window.firstRustFrameIds = window.rustx.inspect().nodes.map((node) => node.id);
      }
      return original.apply(this, args);
    };
  });
  let visit = 0;
  const open = async () => {
    await page.goto(`${base}/?renderer=cpu&capabilities-visit=${++visit}#/capabilities`);
    await page.waitForFunction(() => window.rustx?.draws() > 0);
    await page.getByTestId('capabilities-title').waitFor();
  };
  await open();
  report.wasmSha256 = createHash('sha256')
    .update(await (await page.request.get(base + '/pkg/rustx_lab_bg.wasm')).body())
    .digest('hex');
  assert((await page.evaluate(() => window.firstRustFrameIds)).includes('capabilities-title'));
  report.checks.push(
    'direct capabilities link selects its content before the first Rust pixel upload',
  );
  for (const width of [1200, 390]) {
    await page.setViewportSize({ width, height: width === 390 ? 844 : 1000 });
    await open();
    const state = await page.evaluate(() => window.rustx.inspect());
    const overflow = state.nodes.filter(
      (node) => node.rect[2] > 0 && node.rect[0] + node.rect[2] > width + 1,
    );
    assert.deepEqual(
      overflow.map((node) => ({ id: node.id, rect: node.rect })),
      [],
    );
    await page.screenshot({
      path: new URL(`capabilities-${engine}-${width}.png`, output).pathname,
    });
    await page.getByTestId('capabilities-playground').focus();
    await page.keyboard.press('Tab');
    assert(
      await page.getByTestId('capabilities-docs').evaluate((el) => el === document.activeElement),
    );
    await page.mouse.wheel(0, 100000);
    await page.waitForFunction(
      () =>
        window.rustx.inspect().nodes.find((node) => node.id === 'capabilities-limits').rect[1] <
        innerHeight,
    );
    await page.screenshot({
      path: new URL(`capabilities-${engine}-${width}-limits.png`, output).pathname,
    });
    report.checks.push(
      `${width}px: all Rust layout boxes fit the viewport; keyboard links and scroll-to-limits work`,
    );
  }
  await page.setViewportSize({ width: 1200, height: 1000 });
  for (const [label, path, target] of [
    ['Explore the Applications demo →', '/applications', 'profile-name'],
    ['Open People Explorer →', '/people', 'load-fixture'],
    ['Try the Visual engine →', '/engine', 'drawing-board'],
    ['Try the Runtime lab →', '/lab', 'inc-ALPHA'],
  ]) {
    await open();
    const link = page.getByRole('link', { name: label, exact: true });
    await link.focus();
    await page.keyboard.press('Enter');
    await page.waitForFunction((path) => location.hash === '#' + path, path);
    await page.getByTestId(target).waitFor();
    report.checks.push(`keyboard activation opens working ${path} example`);
  }
  await page.getByTestId('nav-capabilities').focus();
  await page.keyboard.press('Enter');
  await page.getByTestId('capabilities-title').waitFor();
  await page.getByTestId('capabilities-playground').focus();
  await page.keyboard.press('Enter');
  await page.getByTestId('playground-source').waitFor();
  report.checks.push('persistent navigation and playground CTA open the real Rust pages');
  assert.deepEqual(errors, []);
  report.result = 'passed';
} catch (error) {
  report.result = 'failed';
  report.error = String(error);
  if (page)
    await page
      .screenshot({ path: new URL(`capabilities-${engine}-failure.png`, output).pathname })
      .catch(() => {});
  throw error;
} finally {
  await fs.writeFile(
    new URL(`capabilities-${engine}.json`, output),
    JSON.stringify(report, null, 2) + '\n',
  );
  console.log(JSON.stringify(report, null, 2));
  await browser.close();
}

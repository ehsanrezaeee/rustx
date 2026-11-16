// Real Wasm host lifecycle and main-document DOM budgets. No AT/IME certification.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { chromium, webkit } from 'playwright';
const engine = process.env.RUSTX_BROWSER || 'chromium';
const base = process.env.RUSTX_URL || 'http://127.0.0.1:3000';
const output = new URL('../docs/evidence/react-parity/RP-004/', import.meta.url);
const budgetFile = new URL(`../fixtures/dom-budget-${engine}.json`, import.meta.url);
const report = { engine, date: new Date().toISOString(), checks: [], routes: {} };
const browser = await { chromium, webkit }[engine].launch({ headless: true });
await fs.mkdir(output, { recursive: true });
try {
  report.version = browser.version();
  const page = await browser.newPage({
    viewport: { width: 1280, height: 720 },
    deviceScaleFactor: 1,
  });
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  const settle = () =>
    page.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    );
  const open = async (route) => {
    await page.goto(`${base}/?renderer=cpu&dom-audit=1#${route}`);
    await page.waitForFunction(() => window.rustx?.draws() > 0);
    await settle();
  };
  const snapshot = () => page.evaluate(() => window.rustx.dom());
  const clean = (value) => {
    assert.equal(value.counts.unregistered, 0);
    assert.equal(value.visibleSemantic, 0);
    assert.equal(value.counts.emergency, 0);
  };
  for (const route of [
    '/',
    '/people',
    '/lab',
    '/engine',
    '/applications',
    '/playground',
    '/capabilities',
    '/store',
  ]) {
    await open(route);
    const value = await snapshot();
    clean(value);
    report.routes[route] = value.counts;
  }
  report.wasmSha256 = createHash('sha256')
    .update(await (await page.request.get(base + '/pkg/rustx_lab_bg.wasm')).body())
    .digest('hex');
  const before = await snapshot();
  await page.getByTestId('store-publish').click();
  await page.waitForFunction(() =>
    window.rustx.inspect().nodes.some((n) => n.text === 'revision 1: 1 / 2'),
  );
  await settle();
  const after = await snapshot();
  report.updateMutations = Object.fromEntries(
    Object.keys(after.mutations).map((category) => [
      category,
      Object.fromEntries(
        Object.keys(after.mutations[category]).map((key) => [
          key,
          after.mutations[category][key] - before.mutations[category][key],
        ]),
      ),
    ]),
  );
  assert.deepEqual(after.counts, before.counts);
  report.checks.push(
    'all eight route counts; coherent semantic update without added/removed bridge elements',
  );
  // Exercise real retained Rust route changes; there is no mocked app receiver.
  for (let cycle = 0; cycle < 100; cycle++) {
    for (const [route, id] of [
      ['/lab', 'inc-ALPHA'],
      ['/store', 'store-publish'],
    ]) {
      await page.evaluate((route) => {
        location.hash = '#' + route;
      }, route);
      await page.getByTestId(id).waitFor();
      await settle();
      const state = await snapshot();
      clean(state);
      assert.deepEqual(state.counts, report.routes[route], `route cycle ${cycle} ${route}`);
    }
  }
  report.checks.push('100 route cycles return to exact live category baselines');
  let disposedBaseline;
  for (let cycle = 0; cycle < 100; cycle++) {
    const state = await page.evaluate(async () => {
      const api = window.rustx;
      const button = document.querySelector('[data-testid="store-publish"]');
      api.dispose();
      api.dispose();
      button.click(); // retained detached listener must not call the dropped Rust APP
      dispatchEvent(new Event('resize'));
      api.setTheme(true); // retained development API cannot affect the next root
      const state = { dom: api.dom(), host: api.hostState() };
      await Promise.all([api.restart(), api.restart()]);
      button.click();
      return state;
    });
    assert.equal(state.host.semantic, 0);
    assert.equal(state.host.requests, 0);
    assert.equal(state.host.services, 0);
    assert.equal(state.host.queued, false);
    assert.equal(state.dom.counts.tooling, 0);
    assert.equal(state.dom.counts.semantic, 0);
    disposedBaseline ||= state.dom.counts;
    assert.deepEqual(state.dom.counts, disposedBaseline);
    await page.waitForFunction(() => window.rustx?.draws() > 0);
    await settle();
    assert.deepEqual((await snapshot()).counts, report.routes['/store']);
    assert(
      await page.evaluate(() =>
        window.rustx.inspect().nodes.some((n) => n.text === 'revision 0: 0 / 0'),
      ),
    );
  }
  report.disposedCounts = disposedBaseline;
  report.checks.push(
    '100 root dispose/restart cycles; double dispose, concurrent restart and detached old controls are inert',
  );
  await page.getByTestId('store-publish').click();
  await page.waitForFunction(() =>
    window.rustx.inspect().nodes.some((n) => n.text === 'revision 1: 1 / 2'),
  );
  await page.evaluate(() =>
    dispatchEvent(new PageTransitionEvent('pagehide', { persisted: true })),
  );
  assert.equal((await page.evaluate(() => window.rustx.hostState())).suspended, true);
  await page.evaluate(() =>
    dispatchEvent(new PageTransitionEvent('pageshow', { persisted: true })),
  );
  await settle();
  await page.getByTestId('store-publish').click();
  await page.waitForFunction(() =>
    window.rustx.inspect().nodes.some((n) => n.text === 'revision 2: 2 / 4'),
  );
  report.checks.push(
    'synthetic persisted lifecycle preserves Rust state and resumes input; not actual BFCache evidence',
  );
  await page.screenshot({ path: new URL(`dom-${engine}.png`, output).pathname });
  assert.deepEqual(errors, []);
  const budget = {
    viewport: '1280x720 DPR1 CPU',
    routes: report.routes,
    disposedCounts: report.disposedCounts,
    updateCeilings: Object.fromEntries(
      Object.entries(report.updateMutations).map(([category, values]) => [
        category,
        Object.fromEntries(
          Object.entries(values).map(([key, value]) => [
            key,
            ['added', 'removed'].includes(key) ? value : Math.ceil(value * 1.1) + (value ? 10 : 0),
          ]),
        ),
      ]),
    ),
  };
  if (process.env.RUSTX_RECORD_DOM === '1')
    await fs.writeFile(budgetFile, JSON.stringify(budget, null, 2) + '\n');
  else {
    const frozen = JSON.parse(await fs.readFile(budgetFile, 'utf8'));
    assert.deepEqual(report.routes, frozen.routes);
    assert.deepEqual(report.disposedCounts, frozen.disposedCounts);
    for (const [category, values] of Object.entries(report.updateMutations))
      for (const [key, value] of Object.entries(values))
        assert(
          value <= frozen.updateCeilings[category][key],
          `${category}.${key} exceeds frozen mutation budget`,
        );
  }
  report.result = 'pass';
} catch (e) {
  report.result = 'fail';
  report.error = String(e.stack || e);
  throw e;
} finally {
  await fs.writeFile(
    new URL(`dom-${engine}${process.env.RUSTX_RECORD_DOM === '1' ? '-baseline' : ''}.json`, output),
    JSON.stringify(report, null, 2) + '\n',
  );
  await browser.close();
}
console.log(JSON.stringify(report, null, 2));

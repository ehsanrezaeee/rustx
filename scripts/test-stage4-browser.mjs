// Actual browser, Rust compiler and opaque worker execution. No IME/AT certification.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { chromium, webkit, firefox } from 'playwright';
const engine = process.env.RUSTX_BROWSER || 'chromium',
  base = process.env.RUSTX_URL || 'http://127.0.0.1:3000',
  compiler = 'http://127.0.0.1:3002',
  file = new URL(`../docs/evidence/stage4-browser-${engine}.json`, import.meta.url);
const report = {
  date: new Date().toISOString(),
  engine,
  checks: [],
  limitations: [
    'Automated browser keyboard only; no human screen-reader or OS IME task',
    'WebKit is not actual Safari/iOS; no physical mobile or Windows evidence',
    'Loopback compiler isolation and runtime limits are not a public security certification',
  ],
};
let browser;
const errors = [];
try {
  for (let n = 0; ; n++) {
    try {
      if ((await fetch(compiler + '/health', { signal: AbortSignal.timeout(1000) })).ok) break;
    } catch {}
    assert(n < 120, 'compiler startup');
    await new Promise((r) => setTimeout(r, 500));
  }
  browser = await { chromium, webkit, firefox }[engine].launch({
    headless: process.env.RUSTX_HEADED !== '1',
    timeout: 60000,
    ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}),
  });
  report.version = browser.version();
  const context = await browser.newContext({ viewport: { width: 1200, height: 1000 } }),
    page = await context.newPage();
  page.setDefaultTimeout(45000);
  page.on('pageerror', (e) => errors.push(e.message));
  const frames = () =>
    page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
  const click = async (id) => {
    await page.getByTestId(id).focus();
    await frames();
    await page.getByTestId(id).click();
    await frames();
  };
  const text = (id, value) =>
    page.waitForFunction(
      ({ id, value }) =>
        window.rustx
          ?.inspect()
          .nodes.find((n) => n.id === id)
          ?.text.includes(value),
      { id, value },
    );
  await page.goto(base + '/?renderer=cpu#/playground');
  await page.waitForFunction(() => window.rustx?.draws() > 0);
  report.wasmSha256 = createHash('sha256')
    .update(await (await page.request.get(base + '/pkg/rustx_lab_bg.wasm')).body())
    .digest('hex');
  const original = await page.getByTestId('playground-source').inputValue();
  await page
    .getByTestId('playground-source')
    .fill(
      original
        .replace('Hello from RustX', 'Edited real Rust')
        .replace('cx.signal(0)', 'cx.signal(7)'),
    );
  await click('playground-run');
  await page.waitForSelector('#rustx-compiled-output[data-ready=true]');
  const frame = page.frameLocator('#rustx-compiled-output');
  assert.equal(
    await page.locator('#rustx-compiled-output').getAttribute('sandbox'),
    'allow-scripts',
  );
  await frame.getByTestId('count').waitFor();
  assert((await frame.getByTestId('count').textContent()).includes('7'));
  await frame.getByTestId('increase').click();
  await frame.getByTestId('count').filter({ hasText: '8' }).waitFor();
  assert.equal(
    await frame.locator('body').evaluate(() => {
      try {
        return !!parent.document;
      } catch {
        return false;
      }
    }),
    false,
  );
  assert.equal(
    await frame.locator('body').evaluate(() => {
      try {
        localStorage.getItem('visitor');
        return true;
      } catch {
        return false;
      }
    }),
    false,
  );
  await page.screenshot({
    path: new URL(`../docs/evidence/stage4-playground-${engine}-1200.png`, import.meta.url)
      .pathname,
  });
  report.checks.push(
    'edited source is compiled to real interactive Wasm; counter changes 7 to 8 inside opaque script-only iframe; parent DOM and storage denied',
  );
  await click('template-api');
  await click('playground-run');
  await page.waitForSelector('#rustx-compiled-output[data-ready=true]');
  await frame.getByTestId('load').click();
  await frame.getByTestId('result').filter({ hasText: 'Loaded 6 people' }).waitFor();
  report.checks.push(
    'independently compiled API template fetches real loopback fixture through restricted worker host',
  );
  await page.getByTestId('playground-source').fill(`use rustx::prelude::*;
pub fn app(cx: Scope) -> View {
    let count = cx.signal(0);
    let shown = count.clone();
    let front = count.clone();
    rustx! {
        <Column style={Style::column().width(300.).height(180.)}>
            <Text id="result">{move || format!("{}", shown.get())}</Text>
            <Button id="front" style={Style::row().absolute().left(Length::Px(20.)).top(Length::Px(60.)).width(160.).height(44.).z_index(5)} on_click={move |_| { let _ = front.update(|n| *n += 1); }}>"Front"</Button>
            <Button id="back" style={Style::row().absolute().left(Length::Px(20.)).top(Length::Px(60.)).width(160.).height(44.).z_index(1)} on_click={move |_| { let _ = count.update(|n| *n += 100); }}>"Back"</Button>
        </Column>
    }
}`);
  await click('playground-run');
  await page.waitForSelector('#rustx-compiled-output[data-ready=true]');
  await frame.getByTestId('front').click();
  await frame.getByTestId('result').filter({ hasText: /^1$/ }).waitFor();
  await frame.getByTestId('front').focus();
  await frame.getByTestId('front').press('Tab');
  assert(await frame.getByTestId('back').evaluate((element) => element === document.activeElement));
  await frame.getByTestId('back').press('Enter');
  await frame.getByTestId('result').filter({ hasText: /^101$/ }).waitFor();
  await frame.getByTestId('back').press('Space');
  await frame.getByTestId('result').filter({ hasText: /^201$/ }).waitFor();
  report.checks.push(
    'compiled absolute/z-index controls share Rust paint/pointer order while retaining logical Tab order',
  );
  await page
    .getByTestId('playground-source')
    .fill('use rustx::prelude::*;\npub fn app(_:Scope)->View {let n:u32="wrong";Node::text(n)}');
  await click('playground-run');
  await text('compiler-feedback', 'main.rustx:2:');
  await click('playground-go-error');
  await text('compiler-feedback', 'Selected main.rustx:2:');
  assert.equal(
    await page.getByTestId('playground-source').evaluate((e) => e === document.activeElement),
    true,
  );
  report.checks.push(
    'actual rustc type error and Go to error select the original main.rustx source',
  );
  await click('playground-reset');
  assert.equal(await page.getByTestId('playground-source').inputValue(), original);
  await click('playground-run');
  await click('playground-cancel');
  await text('compiler-feedback', 'Cancelled');
  await page.waitForTimeout(600);
  assert.equal(await page.locator('#rustx-compiled-output').count(), 0);
  report.checks.push('reset restores source; cancelling actual compilation rejects late preview');
  await page
    .getByTestId('playground-source')
    .fill('use rustx::prelude::*;pub fn app(_:Scope)->View {loop{std::hint::spin_loop();}}');
  await click('playground-run');
  await text('compiler-feedback', 'runtime response limit');
  await click('playground-reset');
  await text('compiler-feedback', 'Source reset');
  report.checks.push(
    'infinite visitor Rust loop is stopped by frame watchdog; main editor stays interactive',
  );
  await context.setOffline(true);
  await click('playground-run');
  await text('compiler-feedback', 'Compiler unavailable');
  await context.setOffline(false);
  await click('playground-reset');
  const second = await context.newPage();
  await second.goto(base + '/?renderer=cpu#/playground');
  await second.waitForSelector('[data-testid=playground-source]');
  await second
    .getByTestId('playground-source')
    .fill(original.replace('cx.signal(0)', 'cx.signal(23)'));
  await second.getByTestId('playground-run').focus();
  await second.getByTestId('playground-run').click();
  await second.waitForSelector('#rustx-compiled-output[data-ready=true]');
  assert(
    (
      await second.frameLocator('#rustx-compiled-output').getByTestId('count').textContent()
    ).includes('23'),
  );
  assert.equal(await page.locator('#rustx-compiled-output').count(), 0);
  await second.close();
  report.checks.push(
    'network-unavailable compiler is explicit and another visitor receives its own compiled state/job',
  );
  await page.evaluate(() => (location.hash = '/people'));
  await page.getByTestId('load-fixture').waitFor();
  await click('load-slow');
  await text('api-state', 'Loading');
  await click('cancel');
  await text('api-state', 'Cancelled');
  await click('load-fixture');
  await text('api-state', '6 people');
  await page.getByTestId('people-search').fill('Ada');
  await text('api-state', '1 people');
  await page.getByTestId('person-1').focus();
  await page.keyboard.press('Enter');
  await text('person-details-text', 'Ada');
  await page.getByTestId('people-search').fill('');
  await click('load-empty');
  await text('people-empty', 'No people');
  await click('load-malformed');
  await text('api-state', 'Request failed');
  await click('load-error');
  await text('api-state', 'HTTP 503');
  await click('load-timeout');
  await text('api-state', 'Timeout');
  await context.setOffline(true);
  await click('load-fixture');
  await text('api-state', 'Network unavailable');
  await context.setOffline(false);
  await click('retry-people');
  await text('api-state', '6 people');
  report.checks.push(
    'People actual fixture loading/success/search/keyboard details/empty/malformed/HTTP503/timeout/cancel/offline/retry',
  );
  await click('load-live');
  try {
    await text('api-state', '10 people');
    report.liveApi = {
      result: 'passed',
      url: 'https://jsonplaceholder.typicode.com/users',
      state: await page.evaluate(
        () => window.rustx.inspect().nodes.find((n) => n.id === 'api-state').text,
      ),
    };
    report.checks.push('actual public JSONPlaceholder browser fetch returns 10 typed people');
  } catch (error) {
    report.liveApi = {
      result: 'failed',
      state: await page.evaluate(
        () => window.rustx.inspect().nodes.find((n) => n.id === 'api-state')?.text,
      ),
    };
    throw error;
  }
  await page.screenshot({
    path: new URL(`../docs/evidence/stage4-people-${engine}-1200.png`, import.meta.url).pathname,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await frames();
  const nodes = await page.evaluate(() => window.rustx.inspect().nodes);
  assert(nodes.filter((n) => n.id?.startsWith('nav-')).every((n) => n.rect[0] + n.rect[2] <= 391));
  await click('load-fixture');
  await text('api-state', '6 people');
  await page.screenshot({
    path: new URL(`../docs/evidence/stage4-people-${engine}-390.png`, import.meta.url).pathname,
  });
  await page.evaluate(() => (location.hash = '/playground'));
  await page.getByTestId('playground-source').focus();
  await frames();
  await page.screenshot({
    path: new URL(`../docs/evidence/stage4-playground-${engine}-390.png`, import.meta.url).pathname,
  });
  await page
    .getByTestId('playground-source')
    .fill(original + '\n' + '// Scrollable Rust source\n'.repeat(60));
  await page.getByTestId('playground-source').press('Control+End');
  await frames();
  assert(
    (await page.evaluate(
      () => window.rustx.inspect().nodes.find((n) => n.id === 'playground-source').editing.scroll_y,
    )) > 0,
  );
  report.checks.push(
    '390px navigation/People/playground reflow and Rust-owned multiline source scrolling/caret reveal',
  );
  await page.setViewportSize({ width: 1200, height: 1000 });
  await page.evaluate(() => (location.hash = '/engine'));
  await page.getByTestId('drawing-board').waitFor();
  await click('board-right');
  const profile = await page.evaluate(() => window.rustx.profile());
  assert(profile.rust.enabled);
  assert(profile.rust.frames.some((f) => f.invalidation.length));
  assert(profile.rust.frames.some((f) => f.text_measurements.length));
  assert(profile.rust.events.some((e) => e.path.length));
  report.profiler = {
    frames: profile.rust.frames.length,
    hotspots: profile.rust.frames.flatMap((f) => f.text_measurements).slice(-5),
  };
  const popup = page.waitForEvent('popup');
  await page.evaluate(() => window.rustx.openInspector());
  const inspector = await popup;
  await inspector.waitForSelector('tbody tr');
  assert((await inspector.locator('a[href^="vscode://file/"]').count()) > 0);
  await inspector.locator('tbody tr').first().click();
  assert((await inspector.locator('#details').textContent()).includes('rect'));
  await inspector.close();
  report.checks.push(
    'actual inspector source links/ownership/rect highlight, event paths, invalidation and per-node text measurement timings',
  );
  assert.deepEqual(errors, []);
  report.result = 'passed';
} catch (error) {
  report.result = 'failed';
  report.error = String(error);
  throw error;
} finally {
  try {
    const old = JSON.parse(await fs.readFile(file, 'utf8'));
    if (old.result !== 'passed')
      await fs.writeFile(
        new URL(
          `../docs/evidence/history/stage4/browser-${engine}-${Date.now()}.json`,
          import.meta.url,
        ),
        JSON.stringify(old, null, 2) + '\n',
      );
  } catch {}
  await fs.writeFile(file, JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report, null, 2));
  await browser?.close();
}

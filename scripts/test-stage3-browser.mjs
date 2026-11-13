// Real browser input and services. Synthetic composition is never OS IME evidence.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { chromium, webkit } from 'playwright';
const engine = process.env.RUSTX_BROWSER || 'chromium',
  base = process.env.RUSTX_URL || 'http://127.0.0.1:3000';
const report = {
  date: new Date().toISOString(),
  engine,
  checks: [],
  limitations: ['No Windows, human screen-reader, real OS IME, actual Safari or mobile acceptance'],
};
const browser = await { chromium, webkit }[engine].launch({
  headless: process.env.RUSTX_HEADED !== '1',
  ...(process.env.CHROME_PATH ? { executablePath: process.env.CHROME_PATH } : {}),
});
let page;
const errors = [];
try {
  const context = await browser.newContext({
    viewport: { width: 1200, height: 1050 },
    acceptDownloads: true,
  });
  page = await context.newPage();
  page.setDefaultTimeout(20000);
  page.on('pageerror', (e) => errors.push(e.message));
  await page.addInitScript(() => {
    const original = CanvasRenderingContext2D.prototype.putImageData;
    CanvasRenderingContext2D.prototype.putImageData = function (...args) {
      if (!window.firstRustFrameIds && window.rustx) {
        window.firstRustFrameIds = window.rustx.inspect().nodes.map((node) => node.id);
      }
      return original.apply(this, args);
    };
  });
  report.version = browser.version();
  await page.goto(`${base}/?renderer=cpu#/applications`);
  await page.waitForFunction(() => window.rustx?.draws() > 0);
  const text = (id, value) =>
    page.waitForFunction(
      ({ id, value }) =>
        window.rustx
          .inspect()
          .nodes.find((n) => n.id === id)
          ?.text.includes(value),
      { id, value },
    );
  const frames = () =>
    page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
  const click = async (id) => {
    await page.getByTestId(id).focus();
    await frames();
    await page.getByTestId(id).click();
    await frames();
  };
  await page.waitForSelector('[data-testid="profile-name"]');
  assert((await page.evaluate(() => window.firstRustFrameIds)).includes('profile-name'));
  report.checks.push('initial deep link is selected before the first actual Rust pixel frame');
  report.checks.push('hash deep link mounts the public Rust application');
  const dropPrevented = await page.getByTestId('profile-name').evaluate((el) => {
    const transfer = new DataTransfer();
    transfer.items.add(new File(['metadata only'], 'dropped.json', { type: 'application/json' }));
    const event = new DragEvent('drop', {
      bubbles: true,
      cancelable: true,
      dataTransfer: transfer,
      clientX: 10,
      clientY: 10,
    });
    el.dispatchEvent(event);
    return event.defaultPrevented;
  });
  assert.equal(dropPrevented, true);
  report.checks.push(
    'synthetic File metadata crosses the browser drop bridge synchronously with navigation prevented; no OS file-drop claim',
  );
  await page.screenshot({
    path: new URL(`../docs/evidence/stage3-${engine}-1200.png`, import.meta.url).pathname,
  });
  const deep = await context.newPage();
  await deep.goto(`${base}/applications/profile/42?renderer=cpu#/applications`);
  await deep.waitForSelector('[data-testid="profile-name"]');
  await deep.reload();
  await deep.waitForSelector('[data-testid="profile-name"]');
  await deep.close();
  report.checks.push('nested extensionless shell/deep-link reload');
  await click('submit-profile');
  await text('name-error', 'Enter your name');
  assert.equal(await page.getByTestId('profile-name').getAttribute('aria-invalid'), 'true');
  await page.getByTestId('profile-name').fill('Ada سلام');
  await page.getByTestId('profile-email').fill('ada@example.test');
  await text('form-state', 'dirty=true');
  await page.getByTestId('profile-password').fill('secret-stage3-🔒');
  await frames();
  const diagnostic = await page.evaluate(() =>
    JSON.stringify({ inspect: window.rustx.inspect(), semantics: window.rustx.semantics() }),
  );
  assert(!diagnostic.includes('secret-stage3'));
  assert.equal(await page.getByTestId('profile-password').getAttribute('type'), 'password');
  const bio = page.getByTestId('profile-bio');
  await bio.fill('A👩‍👩‍👧‍👦é سلام');
  await bio.press('End');
  await bio.press('Backspace');
  await frames();
  assert(!(await bio.inputValue()).endsWith('م'));
  await bio.press(process.platform === 'darwin' ? 'Meta+z' : 'Control+z');
  await frames();
  assert((await bio.inputValue()).endsWith('سلام'));
  const rapid = await bio.evaluate((el) => {
    el.setSelectionRange(0, 0);
    el.dispatchEvent(new Event('select'));
    for (const key of ['End', 'Backspace']) {
      el.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
      el.dispatchEvent(new Event('select'));
    }
    return { value: el.value, selection: [el.selectionStart, el.selectionEnd] };
  });
  assert.equal(rapid.value, 'A👩‍👩‍👧‍👦é سلا');
  assert.deepEqual(rapid.selection, [18, 18]);
  await frames();
  assert.equal(await bio.inputValue(), rapid.value);
  await bio.press(process.platform === 'darwin' ? 'Meta+z' : 'Control+z');
  await frames();
  assert((await bio.inputValue()).endsWith('سلام'));
  report.checks.push(
    'queued selection events cannot overwrite consecutive Rust keyboard edits before a visual frame (synthetic event ordering regression)',
  );
  await bio.fill('abcd\nx\nabcd\n\nسلام é');
  await frames();
  await bio.evaluate((el) => {
    el.setSelectionRange(3, 3);
    el.dispatchEvent(new Event('select'));
  });
  await bio.press('ArrowDown');
  assert.deepEqual(await bio.evaluate((el) => [el.selectionStart, el.selectionEnd]), [6, 6]);
  await bio.press('ArrowDown');
  assert.deepEqual(await bio.evaluate((el) => [el.selectionStart, el.selectionEnd]), [10, 10]);
  await bio.press('Control+End');
  await bio.press('Control+Shift+ArrowLeft');
  assert.equal(await bio.evaluate((el) => el.value.slice(el.selectionStart, el.selectionEnd)), 'é');
  await frames();
  report.checks.push('shared Unicode-word and shaped vertical-line keyboard selection');
  report.checks.push(
    'controlled fields, dirty/errors, UTF16/grapheme editing, Rust undo and password diagnostic redaction',
  );
  await page.getByTestId('profile-consent').focus();
  await page.keyboard.press('Space');
  await frames();
  assert.equal(await page.getByTestId('profile-consent').getAttribute('aria-checked'), 'true');
  await page.keyboard.press('Space');
  await frames();
  assert.equal(await page.getByTestId('profile-consent').getAttribute('aria-checked'), 'false');
  await page.getByTestId('profile-level').focus();
  await page.keyboard.press('ArrowRight');
  await text('preferences-state', 'level 50');
  await click('profile-region');
  assert.equal(await page.getByRole('listbox').count(), 1);
  await page.getByRole('option', { name: 'Tokyo', exact: true }).click();
  await text('preferences-state', 'tokyo');
  report.checks.push(
    'checkbox Space activates exactly once; slider keys and Rust-painted select popup work',
  );
  await click('submit-profile');
  assert.equal(await page.getByRole('dialog').count(), 1);
  await frames();
  assert.equal(
    await page.getByTestId('confirm-profile').evaluate((e) => e === document.activeElement),
    true,
  );
  await page.keyboard.press('Shift+Tab');
  assert.equal(
    await page.getByTestId('close-profile').evaluate((e) => e === document.activeElement),
    true,
  );
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => !document.querySelector('[role=dialog]'));
  assert.equal(
    await page.getByTestId('submit-profile').evaluate((e) => e === document.activeElement),
    true,
  );
  await click('submit-profile');
  await click('confirm-profile');
  await text('profile-feedback', 'submitted successfully');
  report.checks.push('modal focus trap, Escape restoration and validated submission');
  await click('save-profile');
  await text('profile-feedback', 'saved locally');
  await page.getByTestId('profile-name').fill('Changed');
  await click('restore-profile');
  await text('profile-feedback', 'restored');
  assert.equal(await page.getByTestId('profile-name').inputValue(), 'Ada سلام');
  await page.getByTestId('profile-name').fill('Old name');
  await click('validate-name');
  await page.getByTestId('profile-name').fill('New name');
  await page.waitForTimeout(1300);
  assert.equal(await page.getByTestId('profile-name').inputValue(), 'New name');
  report.checks.push(
    'actual IndexedDB draft persistence and stale asynchronous name-validation rejection',
  );
  await text('query-state', 'Cached people: 6');
  await click('query-refresh');
  await text('query-state', 'Cached people: 6');
  await click('simulate-view-error');
  await page.waitForFunction(
    () => !window.rustx.inspect().nodes.some((n) => n.id === 'recovered-content'),
  );
  await page.getByRole('button', { name: 'Try again', exact: true }).focus();
  await page.getByRole('button', { name: 'Try again', exact: true }).click();
  await text('recovered-content', 'ready');
  report.checks.push('query cache refresh and recoverable subtree retry');
  await click('service-post');
  await text('service-output', 'Hello from Rust');
  await click('service-upload');
  await text('service-output', 'HTTP 200');
  await click('service-stream');
  await text('service-output', 'complete');
  await click('service-socket');
  await text('service-output', 'Hello socket');
  report.checks.push(
    'real fixture HTTP JSON POST, multipart upload, pull streaming and WebSocket echo through compiled Rust',
  );
  const downloadPromise = page.waitForEvent('download');
  await click('export-profile');
  const download = await downloadPromise;
  assert.equal(download.suggestedFilename(), 'rustx-profile.json');
  const tmp = new URL('../docs/evidence/stage3-profile-export.json', import.meta.url);
  await download.saveAs(tmp.pathname);
  const exported = JSON.parse(await fs.readFile(tmp, 'utf8'));
  assert.equal(exported.name, 'New name');
  assert(!('password' in exported));
  await page.getByTestId('import-profile').focus();
  const chooserPromise = page.waitForEvent('filechooser');
  await page.getByTestId('import-profile').click();
  const chooser = await chooserPromise;
  await chooser.setFiles({
    name: 'import.json',
    mimeType: 'application/json',
    buffer: Buffer.from('{"name":"Imported","email":"import@example.test"}'),
  });
  await text('profile-feedback', 'imported');
  assert.equal(await page.getByTestId('profile-name').inputValue(), 'Imported');
  report.checks.push('actual file chooser import and named download without password export');
  await page.getByTestId('nav-people').evaluate((e) => e.click());
  await frames();
  await page.getByTestId('load-fixture').evaluate((e) => e.click());
  await text('api-state', '6 people');
  await page.getByTestId('people-search').fill('Ada');
  await text('api-state', '1 people');
  await click('person-1');
  await text('person-details', 'Ada');
  if (engine === 'chromium') {
    await context.grantPermissions(['clipboard-read', 'clipboard-write'], {
      origin: new URL(base).origin,
    });
    await click('copy-person');
    assert((await page.evaluate(() => navigator.clipboard.readText())).includes('Ada Morgan'));
    report.checks.push('actual Chromium clipboard write and readback contains selected person');
  }
  const details = page.getByTestId('person-details-text');
  await details.focus();
  await frames();
  assert.equal(await details.getAttribute('readonly'), '');
  await details.press(process.platform === 'darwin' ? 'Meta+a' : 'Control+a');
  await frames();
  const selectedDetails = await details.evaluate((el) =>
    el.value.slice(el.selectionStart, el.selectionEnd),
  );
  assert(selectedDetails.includes('Ada Morgan'));
  assert.equal(selectedDetails, await details.inputValue());
  if (engine === 'chromium') {
    await details.press(process.platform === 'darwin' ? 'Meta+c' : 'Control+c');
    assert.equal(await page.evaluate(() => navigator.clipboard.readText()), selectedDetails);
    report.checks.push(
      'actual Chromium native selected-text clipboard copy from readonly ordinary Text',
    );
  }
  const detailsRect = await page.evaluate(
    () => window.rustx.inspect().nodes.find((n) => n.id === 'person-details-text').rect,
  );
  await page.mouse.move(detailsRect[0] + 1, detailsRect[1] + 10);
  await page.mouse.down();
  await page.mouse.move(detailsRect[0] + 85, detailsRect[1] + 10, { steps: 4 });
  await page.mouse.up();
  await frames();
  const pointerDetails = await details.evaluate((el) =>
    el.value.slice(el.selectionStart, el.selectionEnd),
  );
  assert(pointerDetails.includes('Ada'));
  assert(pointerDetails.length < selectedDetails.length);
  report.checks.push(
    'ordinary Rust Text supports readonly keyboard selection and shaped pointer drag',
  );
  const popupPromise = page.waitForEvent('popup');
  await click('print-person');
  const popup = await popupPromise;
  await popup.waitForLoadState();
  assert((await popup.locator('body').innerText()).includes('Ada Morgan'));
  await popup.close();
  report.checks.push(
    'actual printable semantic export contains selected person; no physical printer claim',
  );
  await page.goBack();
  await page.waitForSelector('[data-testid="profile-name"]');
  await page.reload();
  await page.waitForSelector('[data-testid="profile-name"]');
  report.checks.push('Rust search/details, history back and reload from a deep link');
  await page.getByTestId('nav-engine').evaluate((e) => e.click());
  await frames();
  const board = page.getByTestId('drawing-board');
  await board.focus();
  await frames();
  await board.click({ position: { x: 20, y: 15 } });
  await text('board-picked', 'Selected object: 1');
  await click('board-edit');
  await board.focus();
  await page.keyboard.press('ArrowRight');
  await frames();
  const drawingDownload = page.waitForEvent('download');
  await click('board-export');
  const drawing = await drawingDownload;
  const path = new URL('../docs/evidence/stage3-drawing-export.csv', import.meta.url);
  await drawing.saveAs(path.pathname);
  const csv = await fs.readFile(path, 'utf8');
  assert.equal(csv.trim().split('\n').length, 10001);
  assert(csv.split('\n')[1].startsWith('1,10,'));
  report.checks.push('drawing object moves with keyboard and exports all 10,000 rectangles');
  await page.getByTestId('nav-applications').evaluate((e) => e.click());
  await frames();
  await page.setViewportSize({ width: 390, height: 844 });
  await frames();
  const state = await page.evaluate(() => window.rustx.inspect());
  assert(
    state.nodes.filter((n) => n.id?.startsWith('nav-')).every((n) => n.rect[0] + n.rect[2] <= 391),
  );
  await page.screenshot({
    path: new URL(`../docs/evidence/stage3-${engine}-390.png`, import.meta.url).pathname,
  });
  const wasm = await page.request.get(`${base}/pkg/rustx_lab_bg.wasm`);
  report.wasmSha256 = createHash('sha256')
    .update(await wasm.body())
    .digest('hex');
  report.checks.push('390px responsive application');
  await frames();
  const before = await page.evaluate(() => window.rustx.draws());
  await page.waitForTimeout(300);
  assert.equal(await page.evaluate(() => window.rustx.draws()), before);
  assert.deepEqual(errors, []);
  report.checks.push('settled application stops drawing; no browser page errors');
  report.result = 'passed';
} catch (error) {
  report.result = 'failed';
  report.error = String(error);
  report.pageErrors = errors;
  if (page)
    await page
      .screenshot({
        path: new URL(`../docs/evidence/stage3-${engine}-failure.png`, import.meta.url).pathname,
      })
      .catch(() => {});
  throw error;
} finally {
  await fs.writeFile(
    new URL(`../docs/evidence/stage3-browser-${engine}.json`, import.meta.url),
    JSON.stringify(report, null, 2) + '\n',
  );
  await browser.close();
  console.log(JSON.stringify(report, null, 2));
}

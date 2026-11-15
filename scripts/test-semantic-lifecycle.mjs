// Real browser DOM lifecycle, with a stub Rust callback receiver. Not an app/AT/IME test.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { chromium, webkit, firefox } from 'playwright';
const engine = process.env.RUSTX_BROWSER || 'chromium';
const browser = await { chromium, webkit, firefox }[engine].launch({ headless: true });
try {
  const page = await browser.newPage();
  await page.setContent('<main id="root"><span id="foreign">Preserve me</span></main>');
  const source = await fs.readFile(new URL('../web/semantics.js', import.meta.url), 'utf8');
  const result = await page.evaluate(async (source) => {
    const url = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
    const { SemanticProjection } = await import(url);
    URL.revokeObjectURL(url);
    const root = document.querySelector('#root');
    let calls = 0;
    for (let i = 0; i < 100; i++) {
      const app = { activate: () => calls++, focus() {}, blur() {} };
      const p = new SemanticProjection(
        root,
        app,
        () => {},
        () => {},
        () => {},
      );
      const button = p.create({ id: 1, role: 'button', focusable: true });
      button.click();
      const editor = p.create({ id: 2, role: 'textbox' });
      p.composing.add(editor);
      p.sync({ id: 0, children: [] }, { nodes: [] });
      button.click();
      if (p.composing.size || root.childElementCount !== 1)
        return { error: 'removed nodes retained', i };
      const mounted = p.create({ id: 3, role: 'button', focusable: true });
      p.lastModal = button;
      p.restoreFocus = editor;
      p.modal = button;
      p.dispose();
      p.dispose();
      // Retained element references must not call the old runtime after unmount.
      mounted.click();
      try {
        p.create({ id: 4 });
        return { error: 'disposed projection accepted create', i };
      } catch (error) {
        if (!error.message.includes('disposed')) throw error;
      }
      p.sync([], { nodes: [] });
      if (root.childElementCount !== 1) return { error: 'owned DOM survived disposal', i };
      if (
        p.elements.size ||
        p.composing.size ||
        p.modal ||
        p.lastModal ||
        p.restoreFocus ||
        p.flat.length
      )
        return { error: 'projection retained state', i };
    }
    return {
      calls,
      children: root.childElementCount,
      foreign: !!document.querySelector('#foreign'),
    };
  }, source);
  assert.deepEqual(result, { calls: 100, children: 1, foreign: true });
  console.log(
    JSON.stringify(
      {
        result: 'passed',
        engine,
        version: browser.version(),
        cycles: 100,
        checks: [
          'owned nodes removed',
          'retained nodes inert',
          'repeat disposal',
          'foreign child preserved',
          'composition/focus references cleared',
        ],
        limit: 'Real DOM; stub Rust receiver. No application, GPU, IME or screen-reader claim.',
      },
      null,
      2,
    ),
  );
} finally {
  await browser.close();
}

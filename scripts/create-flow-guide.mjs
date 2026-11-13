// Documentation only. Uses the project's existing Playwright dependency.
import fs from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import sharp from 'sharp';
const out = new URL('../docs/visual-guide/', import.meta.url);
await fs.mkdir(out, { recursive: true });
const esc = (s) => s.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
function box(x, y, w, title, lines = [], tone = 'rust', h = 100) {
  return `<g class="${tone}"><rect x="${x}" y="${y}" width="${w}" height="${h}" rx="12"/><text x="${x + 18}" y="${y + 29}" class="bt">${esc(title)}</text>${lines.map((l, i) => `<text x="${x + 18}" y="${y + 53 + i * 19}" class="bl">${esc(l)}</text>`).join('')}</g>`;
}
function arrow(x, y, x2, y2, label = '') {
  return `<path d="M${x} ${y} L${x2} ${y2}" class="arrow" marker-end="url(#tip)"/>${label ? `<text x="${(x + x2) / 2}" y="${(y + y2) / 2 - 10}" class="edge">${esc(label)}</text>` : ''}`;
}
function svg(content) {
  return `<svg viewBox="0 0 1120 450" xmlns="http://www.w3.org/2000/svg"><defs><marker id="tip" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10z" fill="#617589"/></marker></defs>${content}</svg>`;
}
const note = (title, text) => `<div class="note"><strong>${title}</strong><p>${text}</p></div>`;
function page(n, title, sub, diagram, notes, source) {
  return `<section><div class="eyebrow">RUSTX / VISUAL SYSTEM GUIDE <span>10 NOVEMBER 2026</span></div><h1>${title}</h1><p class="subtitle">${sub}</p>${svg(diagram)}<div class="notes">${notes}</div><footer><div>${source}</div><b>${String(n).padStart(2, '0')} / 06</b></footer></section>`;
}
let pages = [];
pages.push(
  page(
    1,
    'One app, owned by Rust',
    'Read the arrows from authoring to pixels. The browser supplies platform capabilities around the Rust UI engine.',
    box(10, 30, 235, 'Write the app', [
      'Rust + rustx! components',
      'Typed styles, signals, callbacks',
    ]) +
      arrow(245, 80, 290, 80) +
      box(290, 30, 235, 'Retained runtime', [
        'Mount component setup once',
        'Track bindings and node identity',
      ]) +
      arrow(525, 80, 570, 80) +
      box(570, 30, 235, 'Layout + text', [
        'Taffy computes geometry',
        'Rust shapes and measures text',
      ]) +
      arrow(805, 80, 850, 80) +
      box(
        850,
        30,
        260,
        'Draw the UI',
        ['Choose GPU or CPU backend', 'Present pixels on a canvas'],
        'gpu',
      ) +
      box(
        290,
        225,
        235,
        'User interaction',
        ['Pointer, keyboard, editing', 'Hit tests + event dispatch'],
        'host',
      ) +
      arrow(407, 225, 407, 135, 'state update') +
      box(
        570,
        225,
        235,
        'Platform bridges',
        ['Fetch, storage, clipboard, files', 'Semantics + input adapter'],
        'host',
      ) +
      arrow(570, 275, 525, 275) +
      box(
        850,
        225,
        260,
        'What the user sees',
        ['Overview, People, Runtime lab', 'Visual engine, Applications'],
        'neutral',
      ) +
      arrow(980, 130, 980, 225) +
      arrow(850, 275, 805, 275) +
      '<text x="12" y="409" class="caption">COLOR KEY</text><circle cx="150" cy="403" r="6" fill="#207368"/><text x="164" y="409" class="bl">Rust ownership</text><circle cx="360" cy="403" r="6" fill="#3764b0"/><text x="374" y="409" class="bl">Browser / platform</text><circle cx="610" cy="403" r="6" fill="#8a58b4"/><text x="624" y="409" class="bl">GPU rendering</text>',
    note(
      'Visible UI',
      'Rust owns component state, geometry, shaped text, editing and drawing. Canvas displays the result.',
    ) +
      note(
        'Browser support',
        'A semantic DOM projection and input/textarea bridge provide accessibility and platform text input. They do not compute visible component layout.',
      ),
    'Sources: docs/BLUEPRINT.md §4–5 · examples/lab/src/app.rustx · crates/rustx-engine/src/lib.rs',
  ),
);
pages.push(
  page(
    2,
    'From source code to the first frame',
    'Build time creates the runnable browser bundle. Startup then mounts the application and selects a renderer.',
    box(10, 20, 245, '1 / Rust source', [
      '.rs / .rustx + rustx! macro',
      'Macro expands to Rust builders',
    ]) +
      arrow(255, 70, 290, 70) +
      box(290, 20, 245, '2 / Release build', [
        'Cargo → WebAssembly',
        'wasm-bindgen → JS bindings',
      ]) +
      arrow(535, 70, 570, 70) +
      box(
        570,
        20,
        245,
        '3 / dist/ bundle',
        ['HTML, host JS, bindings, Wasm', 'Serve with the RustX CLI'],
        'host',
      ) +
      arrow(815, 70, 850, 70) +
      box(
        850,
        20,
        260,
        '4 / Browser boot',
        ['Load Wasm and host adapters', 'Report host capabilities'],
        'host',
      ) +
      arrow(980, 120, 980, 210) +
      box(850, 210, 260, '5 / Mount app', [
        'Create Engine and root scope',
        'Register signals and callbacks',
      ]) +
      arrow(850, 260, 815, 260) +
      box(
        570,
        210,
        245,
        '6 / Select renderer',
        ['Try WebGPU unless forced CPU', 'Unavailable / failure → CPU'],
        'gpu',
      ) +
      arrow(570, 260, 535, 260) +
      box(
        290,
        210,
        245,
        '7 / Schedule frame',
        ['requestAnimationFrame', 'Measure viewport + pixel ratio'],
        'host',
      ) +
      arrow(290, 260, 255, 260) +
      box(10, 210, 245, '8 / Present UI', [
        'Engine prepares current frame',
        'Backend paints; semantics sync',
      ]) +
      '<text x="12" y="407" class="caption">NATIVE PATH</text><text x="155" y="407" class="bl">The same app + engine compile to a native binary; winit and native adapters replace the browser host.</text>',
    note(
      'Mount once',
      'Component setup runs when its instance mounts. Signal changes update reactive bindings. A route or keyed remount may create a new instance.',
    ) +
      note(
        'Renderer recovery',
        'Browser device loss or GPU drawing failure switches to CPU while keeping the application engine. An explicit retry can initialize GPU again.',
      ),
    'Sources: scripts/build-web.sh · web/host.js · examples/lab/src/lib.rs · crates/rustx-engine/src/host.rs',
  ),
);
pages.push(
  page(
    3,
    'What happens when you click?',
    'Example: increasing a counter. The same loop also handles scrolling, editing and asynchronous responses.',
    box(
      10,
      25,
      245,
      '1 / Receive input',
      ['Browser semantic/input adapter', 'Forwards event to Rust'],
      'host',
    ) +
      arrow(255, 75, 290, 75) +
      box(290, 25, 245, '2 / Dispatch event', [
        'Resolve target / hit test',
        'Capture → target → bubble',
      ]) +
      arrow(535, 75, 570, 75) +
      box(570, 25, 245, '3 / Change state', [
        'Callback changes a signal',
        'Dependent bindings invalidate',
      ]) +
      arrow(815, 75, 850, 75) +
      box(
        850,
        25,
        260,
        '4 / Schedule update',
        ['Coalesce requests into a frame', 'requestAnimationFrame'],
        'host',
      ) +
      arrow(980, 125, 980, 220) +
      box(850, 220, 260, '5 / Flush runtime', [
        'Apply pending reactive updates',
        'Take current tree snapshot',
      ]) +
      arrow(850, 270, 815, 270) +
      box(570, 220, 245, '6 / Need layout?', [
        'Size / model / font changes?',
        'Compute boxes when required',
      ]) +
      arrow(570, 270, 535, 270) +
      box(290, 220, 245, '7 / Need painting?', [
        'Dirty / layout / resources / DPR?',
        'Prepare scene and draw backend',
      ]) +
      arrow(290, 270, 255, 270) +
      box(
        10,
        220,
        245,
        '8 / Commit to host',
        ['Present pixels; sync semantics', 'Pump pending service commands'],
        'host',
      ) +
      '<path d="M132 320 L132 380 L980 380 L980 325" class="arrow" marker-end="url(#tip)"/><text x="556" y="370" class="edge">Schedule again only when needs_update() requests it</text>',
    note(
      'When idle',
      'If nothing requires repainting, Engine::frame returns false. The browser skips presentation and does not continuously redraw a settled app.',
    ) +
      note(
        'Important distinction',
        'Retained layout and cached scene fragments reduce preparation work. In the CPU path, a changed frame still clears and repaints the pixel buffer.',
      ),
    'Sources: web/host.js draw()/schedule() · crates/rustx-engine/src/lib.rs Engine::frame · rustx-core runtime',
  ),
);
pages.push(
  page(
    4,
    'How Rust turns UI into pixels',
    'Both paths share Rust layout and shaped text. They perform the final painting differently.',
    box(365, 0, 390, 'Shared preparation', [
      'Runtime snapshot → layout boxes + text',
      'Engine also builds a scaled Vello scene',
    ]) +
      arrow(435, 100, 275, 150) +
      arrow(685, 100, 845, 150) +
      box(
        25,
        150,
        490,
        'CPU / Software painting',
        [
          'Walk retained layout boxes with tiny-skia',
          'Paint paths, clips, images, glyphs and shadows',
          'Add focus / editing decorations to RGBA pixmap',
        ],
        'rust',
        125,
      ) +
      box(
        605,
        150,
        490,
        'GPU / Vello painting',
        [
          'Consume the engine’s prepared Vello scene',
          'Vello + wgpu render into a GPU texture',
          'Blit to the acquired surface texture',
        ],
        'gpu',
        125,
      ) +
      arrow(270, 275, 270, 325) +
      arrow(850, 275, 850, 325) +
      box(
        25,
        325,
        490,
        'Copy pixels to the browser',
        ['Rust Vec → JS bytes → Uint8ClampedArray', 'ImageData → canvas 2D putImageData'],
        'host',
      ) +
      box(
        605,
        325,
        490,
        'Present the GPU surface',
        [
          'Submit GPU commands and present the frame',
          'Normal path avoids a CPU framebuffer readback',
        ],
        'gpu',
      ),
    note(
      'CPU is a separate painter',
      'It paints from layout boxes, not by replaying Vello’s GPU commands. Shared state and text positions do not imply identical rasterization.',
    ) +
      note(
        'What is preserved',
        'Both routes keep layout and text in Rust. Native presentation uses native surfaces instead of browser ImageData / canvas APIs.',
      ),
    'Sources: rustx-web/src/lib.rs WebApp::frame / WebGpu::draw · rustx-engine/src/cpu.rs · examples/lab/src/lib.rs pixels()',
  ),
);
pages.push(
  page(
    5,
    'Why CPU mode feels slower',
    'These are implementation costs visible in source. Their relative impact needs measurement on the actual device.',
    box(
      10,
      25,
      340,
      'A / More pixel work',
      [
        'Changed frame clears full framebuffer',
        'Paths and blending execute in software',
        'DPR 2 = 4× the pixels of DPR 1',
      ],
      'rust',
      125,
    ) +
      box(
        390,
        25,
        340,
        'B / Repeated temporary buffers',
        [
          'Full-canvas clip masks for clipped nodes',
          'A colored pixmap per painted glyph',
          'Extra layers for group opacity',
        ],
        'rust',
        125,
      ) +
      box(
        770,
        25,
        340,
        'C / Software effects + copies',
        [
          'Two-pass shadow blur with nested loops',
          'Pixel Vec copied across Wasm / JS',
          'New typed array + putImageData',
        ],
        'rust',
        125,
      ) +
      arrow(180, 150, 180, 230) +
      arrow(560, 150, 560, 230) +
      arrow(940, 150, 940, 230) +
      box(
        10,
        230,
        340,
        'CPU work occupies the UI thread',
        ['Browser input waits for synchronous work', 'Long frames feel like scroll / typing lag'],
        'host',
      ) +
      box(
        390,
        230,
        340,
        'GPU performs parallel raster work',
        ['Vello submits work to the GPU', 'Hardware draws and blends many pixels'],
        'gpu',
      ) +
      box(
        770,
        230,
        340,
        'Potential improvements',
        ['Partial repaint; smaller / reused masks', 'Cached text bitmaps; fewer frame copies'],
        'neutral',
      ) +
      '<text x="12" y="404" class="caption">ILLUSTRATION</text><text x="165" y="404" class="bl">1200 × 800 CSS pixels → DPR 1: 0.96M pixels / 3.84 MB RGBA; DPR 2: 3.84M / 15.36 MB per buffer.</text>',
    note(
      'Existing optimizations',
      'Demand-driven scheduling, text/glyph caches, retained layout, scene-fragment reuse and drawing culling are already present.',
    ) +
      note(
        'No invented benchmark',
        'This diagram explains possible costs. It does not establish an FPS, a CPU/GPU speed ratio, or which operation dominates your device.',
      ),
    'Sources: rustx-engine/src/cpu.rs draw / paint_node / paint_shadow / paint_text · web/host.js · examples/lab/src/lib.rs',
  ),
);
pages.push(
  page(
    6,
    'Navigation, data and platform services',
    'The demo’s pages share the same engine. External work returns through adapters and re-enters the update loop.',
    box(10, 10, 245, 'Navigate the demo', [
      'Hash router → page signal',
      'Mount selected page branch',
    ]) +
      arrow(255, 60, 290, 60) +
      box(
        290,
        10,
        820,
        'Pages in the current app',
        [
          '/ Overview     /people People explorer     /lab Runtime lab',
          '/engine Visual engine     /applications Applications dashboard',
        ],
        'neutral',
      ) +
      box(10, 180, 245, 'Rust application', [
        'HTTP / service / file request',
        'Scoped task and request identity',
      ]) +
      arrow(255, 230, 290, 230) +
      box(290, 180, 245, 'Command queue', [
        'Typed bridge commands',
        'Host pumps pending requests',
      ]) +
      arrow(535, 230, 570, 230) +
      box(
        570,
        180,
        245,
        'Platform adapter',
        ['Browser Fetch / WebSocket', 'Storage, clipboard, files, etc.'],
        'host',
      ) +
      arrow(815, 230, 850, 230) +
      box(850, 180, 260, 'Completion', [
        'Return result + request identity',
        'Validate version / scope / state',
      ]) +
      '<path d="M980 280 L980 365 L132 365 L132 285" class="arrow" marker-end="url(#tip)"/><text x="556" y="355" class="edge">Accepted result → update Rust state → schedule frame → render current UI</text>' +
      '<text x="12" y="423" class="bl">Semantic tree → browser accessibility DOM / native AccessKit. Input bridge → Rust text model → painted caret and selection.</text>',
    note(
      'Read the implementation',
      'Start with app.rustx → examples/lab/src/lib.rs → web/host.js → Engine::frame → scene.rs / cpu.rs → rustx-web GPU module. Local paths are relative to the project root.',
    ) +
      note(
        'Scope of this guide',
        'Source-based explanation of the current Stage 3 implementation. It adds no Windows, real OS IME, screen-reader or performance acceptance claims; see docs/ACCEPTANCE.md.',
      ),
    'Sources: examples/lab/src/app.rustx · web/services.js · web/semantics.js · docs/STAGE3_SERVICES.md · docs/ACCEPTANCE.md',
  ),
);
const html = `<!doctype html><html lang="en"><meta charset="utf-8"><title>RustX — App flow and UI rendering</title><style>
@page{size:A4 landscape;margin:0}*{box-sizing:border-box}body{margin:0;background:#e5eaf0;font-family:Arial,Helvetica,sans-serif;color:#182d40}section{width:297mm;height:210mm;background:#f8fafb;padding:12mm 14mm 10mm;break-after:page;position:relative;overflow:hidden}section:last-child{break-after:auto}.eyebrow{font-size:10px;letter-spacing:2px;font-weight:700;color:#207368;display:flex;justify-content:space-between}.eyebrow span{color:#607386;letter-spacing:1px}h1{font-size:34px;letter-spacing:-1px;margin:17px 0 8px}.subtitle{font-size:14px;color:#516578;margin:0 0 15px;line-height:1.45}svg{width:100%;height:405px;display:block}.rust rect{fill:#e4f2ee;stroke:#9bc7bd}.host rect{fill:#e9effa;stroke:#a3b9dd}.gpu rect{fill:#efe8f7;stroke:#bfa7d8}.neutral rect{fill:#edf0f3;stroke:#bac6d0}.bt{font-size:17px;font-weight:bold;fill:#182d40}.bl{font-size:14px;fill:#3d5367}.arrow{fill:none;stroke:#617589;stroke-width:2}.edge{font-size:12px;fill:#516578;text-anchor:middle;paint-order:stroke;stroke:#f8fafb;stroke-width:5;stroke-linejoin:round}.caption{font-size:11px;font-weight:bold;letter-spacing:1px;fill:#207368}.notes{display:grid;grid-template-columns:1fr 1fr;gap:24px;margin-top:8px}.note{border-top:3px solid #207368;padding-top:10px;font-size:12px;line-height:1.5}.note strong{font-size:13px}.note p{margin:5px 0 0;color:#516578}footer{position:absolute;bottom:9mm;left:14mm;right:14mm;display:flex;align-items:center;gap:20px;justify-content:space-between;color:#6b7c8d;font-size:9px;border-top:1px solid #d6dfe6;padding-top:10px}footer b{white-space:nowrap;font-size:11px}@media screen{section{margin:20px auto;box-shadow:0 4px 30px #182d4020}}
</style>${pages.join('')}</html>`;
await fs.writeFile(new URL('rustx-app-flow.html', out), html);
const executablePath =
  process.env.CHROME_PATH || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const browser = await chromium.launch({ executablePath, headless: true });
try {
  const p = await browser.newPage({ viewport: { width: 1123, height: 794 }, deviceScaleFactor: 1 });
  await p.goto(new URL('rustx-app-flow.html', out).href);
  await p.emulateMedia({ media: 'print' });
  const layout = await p.locator('section').evaluateAll((nodes) =>
    nodes.map((n, i) => ({
      page: i + 1,
      overflow: n.scrollHeight > n.clientHeight,
      notesBottom:
        n.querySelector('.notes').getBoundingClientRect().bottom - n.getBoundingClientRect().top,
      footerTop:
        n.querySelector('footer').getBoundingClientRect().top - n.getBoundingClientRect().top,
    })),
  );
  if (layout.some((n) => n.overflow || n.notesBottom > n.footerTop - 8))
    throw new Error(JSON.stringify(layout));
  await p.pdf({
    path: fileURLToPath(new URL('rustx-app-flow.pdf', out)),
    printBackground: true,
    preferCSSPageSize: true,
    tagged: true,
    outline: true,
  });
  const thumbnails = [];
  for (let i = 0; i < pages.length; i++) {
    const shot = await p.locator('section').nth(i).screenshot();
    if (i === 3) await fs.writeFile(new URL('rendering-preview.png', out), shot);
    thumbnails.push({
      input: await sharp(shot).resize({ width: 674 }).toBuffer(),
      left: (i % 2) * 674,
      top: Math.floor(i / 2) * 477,
    });
  }
  await sharp({ create: { width: 1348, height: 1431, channels: 3, background: '#d6dfe6' } })
    .composite(thumbnails)
    .png()
    .toFile(fileURLToPath(new URL('all-pages-preview.png', out)));
  console.log(
    JSON.stringify(
      { pages: pages.length, layout, pdf: 'docs/visual-guide/rustx-app-flow.pdf' },
      null,
      2,
    ),
  );
} finally {
  await browser.close();
}

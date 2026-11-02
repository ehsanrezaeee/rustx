# Remaining Stage 1 acceptance tasks

Stage 1 cannot be accepted until actual Windows execution, real screen-reader/button use and real OS IME input are recorded. Browser automation and text injection do not substitute for these tasks. Do not advance to Stage 2 based only on the automated results.

Use the same source revision and web bundle. Record Git revision (`git rev-parse HEAD`), OS/browser/tool versions, GPU/driver/backend, viewport/DPR, tester, task results, logs and screenshots. Store results under `docs/evidence/`; update ACCEPTANCE.md after the tasks have actually run. Record failures as well as passes.

## Windows x64

On an actual Windows machine with Rust 1.90.0/MSVC and C++ build tools:

```powershell
cargo test --workspace --locked
./scripts/build-web.ps1
./scripts/build-windows.ps1
cargo run -p rustx-cli --locked -- serve dist 3000
# Another terminal:
./dist/windows/rustx-lab.exe
# Repeat with the CPU renderer:
$env:RUSTX_RENDERER='cpu'
./dist/windows/rustx-lab.exe
```

Verify both native backends and the browser counter. In Runtime lab, edit both keyed tasks, increase their counters, reverse order and unmount/remount. Verify identity, cleanup and reset. Resize the window, move between monitors with different DPI, Tab/Shift+Tab through controls, enter mixed Persian/English text and use local HTTP success/error/cancel. Capture the OS, adapter/driver, backend and actual behavior. A successful build is insufficient.

Native selection/caret/navigation and preedit remain incomplete; record the actual limitations. Native UI Automation is a Stage 3 gap and must not be reported as working.

## Browser input and assistive technology

With the rebuilt fixture server running, open `http://127.0.0.1:3000/?renderer=cpu` in actual Safari with VoiceOver, then a Windows browser with NVDA. Repeat the basic feasibility tasks with the GPU path where available.

1. Navigate by screen reader to “Runtime lab”, “Increase +”, and the labeled “Task for ALPHA” editor. Confirm intelligible names, current text, order and a single activation when pressing Enter/Space.
2. Edit the task, reverse the list while editing, and check text/selection/focus stay with the same key. Unmount/remount and check old controls disappear and state resets.
3. Use a real installed OS IME/input source. Record its exact name/version. Start composition, update preedit, cancel it, then start again and commit Persian/English mixed text. Check the Rust canvas result and semantic editing value agree without duplicate text or focus loss. DOM input/composition event injection does not count.
4. Check Tab/Shift+Tab, scroll-to-focus and narrow viewports. The semantic root must not independently scroll (`#semantic.scrollTop` stays zero). Verify only one editing representation is announced.
5. Note the known absence of Rust-painted selection/caret/preedit and full editing commands. Fix any feasibility failure before accepting RX-09. Do not certify full RX-22/RX-24 from this small spike.

No physical Windows machine, human screen-reader session, or real OS IME task was available to this implementation run. Actual mobile keyboard/touch/AT tasks are also unverified. Browser WebKit automation is distinct from Safari.

## Reproducible automated browser checks

```sh
npm ci
npx playwright install chromium firefox webkit
# With the fixture server still running:
npm run test:browser
RUSTX_BROWSER=firefox npm run test:browser
RUSTX_BROWSER=webkit npm run test:browser
# Physical GPU presentation on this Mac used a visible installed Chrome:
RUSTX_HEADED=1 CHROME_PATH='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' npm run test:gpu
```

`test:gpu` records adapter information and WebGPU API counters; it also injects initial adapter absence and confirms real CPU rendering. The initial-failure test does not certify device-loss recovery. Headless GPU availability depends on the host. These are feasibility checks, not performance certification.

Freeze the benchmark profiles only when actual desktop and Android devices are selected. This run used an Apple M3 MacBook Air (8-core GPU, 16 GiB RAM), macOS 27.0.1 build 26A434. It does not establish the missing Android benchmark device or Windows support.

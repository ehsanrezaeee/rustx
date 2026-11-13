# Remaining platform acceptance tasks

Stages 1–4 cannot receive full acceptance until the required Windows interactions, real screen-reader tasks and real OS IME input are recorded. Windows CI CPU startup is recorded separately; it does not complete this manual matrix. Browser automation and text injection do not substitute for these tasks.

Use the same source revision and web bundle. Record Git revision (`git rev-parse HEAD`), OS/browser/tool versions, GPU/driver/backend, viewport/DPR, tester, task results, logs and screenshots. Store results under `docs/evidence/`; update ACCEPTANCE.md after the tasks have actually run. Record failures as well as passes.

## Windows x64

On an actual Windows machine with Rust 1.90.0/MSVC and C++ build tools:

```powershell
cargo test --workspace --locked
./scripts/build-web.ps1
./scripts/build-windows.ps1
cargo run -p rustx-cli --locked -- serve dist 3000
# Another terminal:
./releases/rustx-0.1.0-windows-x64/rustx-lab.exe
# Repeat with the CPU renderer:
$env:RUSTX_RENDERER='cpu'
./releases/rustx-0.1.0-windows-x64/rustx-lab.exe
```

Verify both native backends and the browser counter. In Runtime lab, edit both keyed tasks, increase their counters, reverse order and unmount/remount. Verify identity, cleanup and reset. Resize the window, move between monitors with different DPI, Tab/Shift+Tab through controls, enter mixed Persian/English text and use local HTTP success/error/cancel. Capture the OS, adapter/driver, backend and actual behavior. A successful build is insufficient.

Rust-painted selection, caret, navigation and preedit and the AccessKit projection now have implementation and deterministic coverage. Their actual native Windows/IME/UI Automation behavior still requires these tasks and must not be inferred from the implementation.

## Browser input and assistive technology

With the rebuilt fixture server running, open `http://127.0.0.1:3000/?renderer=cpu` in actual Safari with VoiceOver, then a Windows browser with NVDA. Repeat the basic feasibility tasks with the GPU path where available.

1. Navigate by screen reader to “Runtime lab”, “Increase +”, and the labeled “Task for ALPHA” editor. Confirm intelligible names, current text, order and a single activation when pressing Enter/Space.
2. Edit the task, reverse the list while editing, and check text/selection/focus stay with the same key. Unmount/remount and check old controls disappear and state resets.
3. Use a real installed OS IME/input source. Record its exact name/version. Start composition, update preedit, cancel it, then start again and commit Persian/English mixed text. Check the Rust canvas result and semantic editing value agree without duplicate text or focus loss. DOM input/composition event injection does not count.
4. Check Tab/Shift+Tab, scroll-to-focus and narrow viewports. The semantic root must not independently scroll (`#semantic.scrollTop` stays zero). Verify only one editing representation is announced.
5. Check the Rust-painted selection/caret/preedit and editing commands against the actual input and announcements. Record any mismatch. Fix feasibility failures before accepting RX-09; full RX-22/RX-24 additionally require the complete form/editing matrix.

Actual Windows CI CPU/package startup is recorded in ACCEPTANCE.md. No human screen-reader session, real OS IME task or clean external Windows machine completed these manual tasks. Physical mobile keyboard/touch/AT tasks are also unverified. Browser WebKit automation is distinct from Safari.

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

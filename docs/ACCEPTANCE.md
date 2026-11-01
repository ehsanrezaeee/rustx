# Acceptance ledger — 1 November 2026

Authoritative register: BLUEPRINT.md RX-01–RX-46. Exact snapshot: SOURCE-REVISION.txt (SHA-256 of SOURCE_MANIFEST.sha256). **Stage 1 is still open.** Five of 46 requirements are verified at their core contract level: checklist coverage, not effort completed or platform certification.

## Executed checks

| Evidence | Command | Actual result and limit |
| --- | --- | --- |
| E1 | `cargo test --workspace --locked` | PASS: 14 runtime tests, 1 numeric host-ID test, compiler harness with 4 negative fixtures; Linux x64 Rust 1.90.0. See evidence/workspace-tests.log. |
| E2 | `bash scripts/build-web.sh` | PASS: release wasm32 and matching wasm-bindgen 0.2.100 output. See evidence/web-build.log. |
| E3 | `npm ci`, `npm run test:wasm` | PASS: actual Wasm in Node 24.22.1, CPU raster images, state/identity/Persian model/memo/idle/HTTP/cancel/stale IDs, 30 route cycles, 390px layout; fake-Fetch success/size/timeout/abort tests. See evidence/wasm-tests.json and .log. |
| E4 | `npm run test:http` | PASS: real Rust fixture server → production JS Fetch → actual Wasm; six-person success, HTTP 503 and cancellation. See evidence/http-tests.log. This is Node networking, not browser/CORS or public API evidence. |
| E5 | `cargo run -p rustx-lab --example render_probe --locked -- docs/evidence --gpu` | CPU and Vello offscreen scene rendered and visually inspected; Mesa llvmpipe LLVM 20.1.2 software GL, Mesa 25.2.8. Image filtering differs. Not hardware/performance/parity certification. See evidence/renderer-probe.log and probe PNGs. |
| E6 | `cargo fmt --all -- --check` | PASS (empty output); this does not certify .rustx macro-body/editor formatting. See evidence/format-check.log. |

Demo PNGs are actual Rust engine output, not browser screenshots. Node tests do not replace real IME/assistive-technology tasks. Expected compiler diagnostics were generated once, then the normal workspace test command passed without overwrite mode.

## Current independent replay and Stage 1 fixes

Current target: Apple M3 MacBook Air, 8-core integrated GPU, 16 GiB RAM; macOS 27.0.1 (26A434), aarch64; Rust 1.90.0; Node 24.18.1. The supplied source was imported as Git baseline `657bcb8` on `main`; the changes are on `stage-1-completion`. The final source identity is in SOURCE-REVISION.txt; the full archive additionally records its exact Git commit and artifact checksums. E1–E6 above describe the supplied Linux snapshot, not this Mac replay.

| Evidence | Command / actual result | Limit |
| --- | --- | --- |
| E7 | `cargo test --workspace --locked` PASS: 15 runtime tests, 5 host-contract tests, 4 expected compiler failures, numeric-ID test and CPU group/border regression. `current-workspace-tests.log`. | macOS/core contracts only. |
| E8 | `bash scripts/build-web.sh`, `cargo fmt --all -- --check`, `npm ci`, `npm run test:host`, `npm run test:wasm` PASS. See `current-*-tests.log`, `current-web-build.log`, `current-format-check.log`, `current-npm-ci.log`. | Actual Wasm in Node; fake Fetch adapter contracts are explicitly separate from browser/network proof. |
| E9 | `RUSTX_TEST_PORT=3001 npm run test:http` PASS: real fixture server → JS Fetch → compiled Wasm success/503/cancel. `current-http-tests.log`. | Loopback, not public API/CORS support certification. |
| E10 | `CHROME_PATH='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' npm run test:browser` PASS on Chrome 156.0.8078.74; `RUSTX_BROWSER=webkit npm run test:browser` PASS on Playwright WebKit 27.2. Reports `browser-chromium.json`, `browser-webkit.json`; screenshots and `current-browser-*.log`. | Actual browser processes: single pointer/Enter/Space activation, keyed editable text/counter identity, focus/selection/reorder, Tab order, unmount reset, Rust scroll-to-focus, fixture success/failure/cancel/supersede/disposal, idle, 390px/DPR 2. Text injection is not OS IME; WebKit is not actual Safari. |
| E11 | Visible installed Chrome: `RUSTX_HEADED=1 CHROME_PATH='…/Google Chrome' npm run test:gpu` PASS. `browser-gpu.json`, `browser-gpu.png`, `current-browser-gpu-headed.log`. Adapter reports Apple Metal 3. | Real WebGPU presentation, counter, resize and idle; instrumented run observed GPU submissions/presentation and zero texture-to-buffer copies, MAP_READ calls or Canvas2D pixel uploads. Injected adapter absence exercised actual CPU fallback. No device-loss/performance/Windows claim. |
| E12 | `cargo run -p rustx-lab --example render_probe --locked -- docs/evidence/current-probe --gpu`, `npm run test:probe` PASS. Actual Apple M3 Metal plus CPU scene. Mean RGB error 0.569/255, 0.530% pixels above channel error 32 (limits 2/255 and 2%). `current-renderer-comparison.json`, `current-renderer-probe.log`. | Fixed 900×450 DPR 1 native offscreen scene; differences in text rasterizers allowed. Does not certify full Stage 2 conformance or Windows/browser parity. |
| E13 | `RUSTX_SMOKE_FRAMES=2 cargo run -p rustx-lab --bin rustx-lab --locked` and forced `RUSTX_RENDERER=cpu`: both actual native Mac windows started and exited successfully. `current-native-*-smoke.log`. | Two-frame startup only, no Windows or full native interaction claim. |
| E14 | Updated exact test pins to Playwright 1.64.1 and sharp 0.35.5; clean `npm ci` and `npm audit --json` PASS, zero reported npm vulnerabilities. `current-npm-audit.json`. | npm tooling only; a full Cargo/transitive license/unsafe/security audit is still open. |

Corrections found by the actual browser runs: empty semantic Text labels; unbound browser Fetch receiver; native Tab policy skipping buttons on macOS; semantic-root scrolling and active-input detachment during reorder. CPU scene corrections: bilinear image sampling, group compositing and inward borders (the regression test exposed expanded borders in both backends). Host tests cover handshake/version mismatch, unsupported capability states, stale/oversized completions, reentrant callbacks and 10,000 start/cancel cycles without queue growth.

## Missing platform evidence

- No actual Windows machine/session was exercised, and no Windows binary is included. Native Mac window startup does not close RX-07.
- No human screen-reader or real OS IME task completed. Rust-painted caret/selection/preedit and full editing remain incomplete. Follow PLATFORM_CHECKS.md before accepting RX-09.
- Firefox 135/157 failed to start with profile/sandbox errors on this host, in headless and attempted visible runs. Retain `current-browser-firefox.log` and `history/` logs; no Firefox pass is claimed. Playwright WebKit does not certify Safari, iOS or Android.
- The pinned Chromium download returned a regional CDN 403 (`current-browser-install.log`); the final Chromium suite used the already installed Chrome. This is a download limitation, not a passing bundled-Chromium result.
- JSONPlaceholder live API success, full visual conformance/device recovery, Windows replay and the exact Android benchmark device remain open.
- Earlier failures are retained in `history/` for diagnosis; final successful results are named `current-*` and `browser-*.json`.

## Requirement states

| ID | Status | Evidence / remaining gate |
| --- | --- | --- |
| RX-01 | implemented_unverified | Pinned Rust/Cargo/npm locks and scripts; independent fresh-machine/Windows replay pending. |
| RX-02 | verified | E1: typed components/default props/generics; four expected compiler-error fixtures. |
| RX-03 | verified | E1: imported component/Props, initial and reactive props, children/fragments/named View slot; E2/E3 demo crate. |
| RX-04 | verified | E1/E3: mount once, memo/effect cleanup, dynamic dependencies, batching, scoped handles/refs, bounded cycles. |
| RX-05 | verified | E1/E3: reorder keeps keyed state/identity; removal disposes and resets; duplicate-key diagnostic. |
| RX-06 | verified | E7–E10: one-time typed version/capability report, bounded/coalesced queues, safe IDs, stale/mismatched completions, scoped disposal and actual GET/cancel host path. Stage 3 service API remains separate. |
| RX-07 | blocked | E10/E11 browser counter runs; E13 Mac native startup. Actual Windows execution remains blocked. |
| RX-08 | in_progress | E11/E12 physical browser WebGPU and native Metal/CPU feasibility; small scene comparison passes. Windows/shared target matrix remains open. |
| RX-09 | blocked | E10 real browser keyboard, semantic text, editing/focus pass; human screen-reader and OS IME tasks remain open. |
| RX-10 | in_progress | Typed styles, grid responsiveness and a small font breakpoint; full theme/query contract missing. |
| RX-11 | in_progress | Taffy integrated; dirty propagation and complete layout fixture suite pending. |
| RX-12 | in_progress | Persian/English samples shape; font fallback/emoji and full bidi fixtures pending. |
| RX-13 | in_progress | E12 fixed scene comparison passes after CPU group/filter and shared border fixes; complete Stage 2 backend conformance remains open. |
| RX-14 | in_progress | E11 real Chrome/Metal presentation API counters show no frame readback in the instrumented run; full native/target trace matrix pending. |
| RX-15 | in_progress | CPU and initial GPU fallback; device loss/recovery incomplete. |
| RX-16 | not_started | No acceptance implementation/evidence yet. |
| RX-17 | not_started | No acceptance implementation/evidence yet. |
| RX-18 | in_progress | E10/E11 browser idle, resize and DPR 2 checks; background/native DPI/support matrix pending. |
| RX-19 | in_progress | Click/input dispatch; full capture/bubble/default-action contract missing. |
| RX-20 | in_progress | Basic focus/scroll/native keyboard; nested/modal/touch matrix missing. |
| RX-21 | in_progress | Primitive components only; full required library missing. |
| RX-22 | in_progress | Whole-string model/composition forwarding; visible caret/selection/undo/grapheme editing incomplete. |
| RX-23 | not_started | No acceptance implementation/evidence yet. |
| RX-24 | in_progress | Semantic projection source; real screen-reader tasks missing. |
| RX-25 | not_started | No acceptance implementation/evidence yet. |
| RX-26 | in_progress | E4 real fixture HTTP success/503/cancel; full browser/native/race matrix and API pending. |
| RX-27 | not_started | No acceptance implementation/evidence yet. |
| RX-28 | not_started | No acceptance implementation/evidence yet. |
| RX-29 | not_started | No acceptance implementation/evidence yet. |
| RX-30 | not_started | No acceptance implementation/evidence yet. |
| RX-31 | not_started | No acceptance implementation/evidence yet. |
| RX-32 | in_progress | Demo error states and fatal host message; complete recovery flows pending. |
| RX-33 | in_progress | .rustx module inclusion works; editor/formatter parity missing. |
| RX-34 | in_progress | serve command supplied; project generation/doctor/reload commands missing. |
| RX-35 | in_progress | JSON node/count/semantic inspector; profiler/event tracing missing. |
| RX-36 | in_progress | Prebuilt web bundle and MIME fixture server; offline/cache/deep-links pending. |
| RX-37 | in_progress | Windows build script; no exercised native distribution. |
| RX-38 | not_started | No acceptance implementation/evidence yet. |
| RX-39 | not_started | No acceptance implementation/evidence yet. |
| RX-40 | in_progress | 100 request cleanup checks and 30 route cycles; full memory/GPU suite pending. |
| RX-41 | in_progress | E14 test dependency advisories fixed; npm audit reports zero. Full Cargo/transitive license/unsafe/security/CSP audit pending. |
| RX-42 | in_progress | README/current handoff; complete public tutorial/migration guide pending. |
| RX-43 | in_progress | Shared-source demo; cross-target behavior matrix pending. |
| RX-44 | in_progress | Current source/bundle/checksums/evidence; final release package pending. |
| RX-45 | in_progress | People Explorer fixture path verified; actual public API/browser/native/search/details/all-state gates pending. |
| RX-46 | in_progress | Homepage/preset interactions verified; editable Rust compile/run playground pending. |

## Stage 1 manual gates

Record source hash, OS/browser/driver versions, backend, viewport/DPR, commands/tasks and actual results. Retain failures.

- [x] Browser CPU keyboard/Tab/focus/reorder/unmount tasks pass without duplicate activation on declared Chrome/WebKit versions (E10).
- [ ] Labeled button and editable field are usable with a real screen reader.
- [ ] Real Persian/English IME composition/cancel/commit and focus-retention tasks pass.
- [x] Direct WebGPU presentation, forced CPU and injected initial adapter absence are exercised in Chrome (E10/E11).
- [ ] Shared GPU/CPU scene parity is resolved across browser/native paths.
- [ ] Same app runs on real Windows in both backends with resize/DPI/HTTP/input checks.
- [x] Host capability/version contract and mismatch/queue bounds are completed (E7–E10).
- [ ] Independent fresh-machine replay and exact benchmark hardware profiles are recorded.

Do not close Stage 1 until these gates pass. Final release additionally requires RX-45/RX-46's two successful examples as defined in blueprint Section 13.2.1.

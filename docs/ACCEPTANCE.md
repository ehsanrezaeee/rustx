# Acceptance ledger — 10 November 2026

Authoritative register: BLUEPRINT.md RX-01–RX-46. Exact snapshot: SOURCE-REVISION.txt (SHA-256 of SOURCE_MANIFEST.sha256). **Stages 1–3 platform/manual acceptance remains open.** Five of 46 requirements are verified at their core contract level: checklist coverage, not effort completed or platform certification.

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
- No human screen-reader or real OS IME task completed. Stage 3 implements Rust-painted caret/selection/preedit and grapheme/undo/composition editing, but their actual IME/assistive-technology target acceptance remains open. Follow PLATFORM_CHECKS.md before accepting RX-09.
- Firefox 135/157 failed to start with profile/sandbox errors on this host, in headless and attempted visible runs. Retain `current-browser-firefox.log` and `history/` logs; no Firefox pass is claimed. Playwright WebKit does not certify Safari, iOS or Android.
- The pinned Chromium download returned a regional CDN 403 (`current-browser-install.log`); the final Chromium suite used the already installed Chrome. This is a download limitation, not a passing bundled-Chromium result.
- JSONPlaceholder live API success, full visual conformance/device recovery, Windows replay and the exact Android benchmark device remain open.
- Earlier failures are retained in `history/` for diagnosis; final successful results are named `current-*` and `browser-*.json`.

## Stage 2 execution — 2 November 2026

The user authorized Stage 2 on the same branch without accepting earlier missing platform gates. Exact final integration Wasm SHA-256: `d9ae281bd9f46414a83c3b0de323f07d677fbe90c6c435524760778f94a59243`; all four integration reports agree. Environment remains macOS 27.0.1 arm64 / Apple M3 MacBook Air, 16 GiB RAM, Rust 1.90.0, Node 24.18.1; installed Chrome updated to 156.0.8078.80. The source content manifest and Git revision identify the delivered code; no new archive was requested.

| Evidence | Executed command / result | Limit |
| --- | --- | --- |
| S2-1 | `cargo test --workspace --locked` PASS, including **27 new regressions** (10 core, 17 engine), existing runtime/host/compiler fixtures and CPU/ID tests. `stage2-workspace-tests.log`. `cargo fmt --all -- --check` PASS. | Deterministic core/engine contracts on this Mac; does not certify unavailable platforms. |
| S2-2 | `bash scripts/build-web.sh`, `npm run test:stage2` PASS. `stage2-web-build.log`, `stage2-wasm.json`, `stage2-wasm.log`. Actual Wasm CPU: max **52** materialized rows over 60 large scrolls, retained focus/key identity, theme pixel round-trip, mixed text, camera/picking, cache budgets, DPR/idle, 10 demo mount/unmount cycles returning to baseline `[80,5,4]`. | Node Wasm execution; not a browser/device substitute. |
| S2-3 | `CHROME_PATH='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' npm run test:stage2:browser`; `RUSTX_BROWSER=webkit npm run test:stage2:browser` PASS. `stage2-browser-chromium-cpu.json`, `stage2-browser-webkit-cpu.json`, matching logs/screenshots. | Actual Chrome 156.0.8078.80 / Playwright WebKit 27.2 CPU. Real pointer pick/drag, keyboard camera pan, nested list wheel/focus/ownership, responsive theme/dashboard, 390px/DPR2 and idle. No actual Safari/mobile/IME/AT claim. |
| S2-4 | `RUSTX_TEST_GPU=1 RUSTX_HEADED=1 CHROME_PATH='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' npm run test:stage2:browser` PASS. `stage2-browser-chromium-gpu.json` plus log/screenshots. Normal GPU phase: **64 submissions / 32 canvas textures, zero texture-to-buffer copies, MAP_READ or Canvas2D uploads**. Actual active `GPUDevice.destroy` → real CPU recovery with selected record retained → newly created GPU renders retained state. | Physical Apple Metal 3 browser adapter. Controlled actual device destruction, not a spontaneous driver reset; no native Windows recovery or performance certification. |
| S2-5 | `cargo run -p rustx-lab --example stage2_probe --locked -- docs/evidence/stage2-probe --gpu`; `npm run test:stage2:probe` PASS. Matched 900×720 DPR1 CPU/Apple M3 Metal fixture: RGB mean **0.387/255**, **0.362%** pixels above channel error 32; frozen limits remain 2/255 and 2%. `stage2-renderer-comparison.json`, probe log/PNGs. | Shared text/emoji, shadow, clip/transform, group/image alpha and custom path/text scene. Different text rasterizers allowed. Full target/feature matrix remains open. |
| S2-6 | `RUSTX_DEMO=engine RUSTX_SMOKE_FRAMES=2 cargo run -p rustx-lab --bin rustx-lab --locked` and forced CPU both PASS with actual native Mac windows. `stage2-native-*-smoke.log`. `npm run test:http`, `npm run test:host` and inherited Wasm suite also PASS (`stage2-http-tests.log`, `stage2-host-tests.log`, `stage2-legacy-wasm-tests.log`). | Native two-frame startup only; fixture HTTP, not external API/Windows acceptance. |
| S2-7 | `stage2-wasm.json` retains 121 raw local CPU samples in `stage2-wasm.json`: median **17.416 ms**, p95 **19.354 ms** in the recorded run. | Mixed integration frames/viewport changes, not a frozen benchmark workload, event-to-present estimate, React comparison or release-budget certification. |

Tests exposed and fixed: alpha-premultiplication/image-alpha mismatch, missed image-epoch repaint, nonfinite text coordinates, offscreen shadow culling, wrong path/rounded hits and stroke overhang, nested row scrolling, virtual focus/projection ownership and insufficient control contrast. Font-collection admission rolls back excess faces; browser surface admission reduces DPR before oversized canvas allocation. Stroke joins/caps use the shared round-style contract. See STAGE2.md for the finite API, cache accounting and remaining layout/text/editing limits.

Stage 2 is implemented and exercised on these available paths; acceptance remains unverified for Windows/native cross-target parity/recovery, actual Safari/mobile, background/native DPI matrix, broader text/effect conformance and fixed-hardware performance. Original OS IME/screen-reader gates remain open. Earlier diagnostic failures are retained under `history/stage2`; reports marked passed are final runs.

## Stage 3 delivery — 10 November 2026

Stage 3 implementation is delivered on `stage-3`, preserving the mount-once runtime and Rust-owned layout, text, editing and scene. The Applications dashboard uses the public forms/widgets/services; People exposes search/details/copy/print and the drawing board supports editing/export. Environment: macOS 27.0.1 (26A434), arm64 Apple M3 MacBook Air, Rust 1.90.0, Node 24.18.1, installed Chrome 157.0.8103.40 and Playwright WebKit 27.2. Source content is identified by SOURCE-REVISION.txt; BUILD-INFO.json identifies the rebuilt browser artifacts. The delivered archive adds its exact commit and payload checksums. All five final Stage 3/engine browser reports identify Wasm SHA-256 `ab207951fc412b9ec041b319a9aae7acbf3143ede855fc27be7093a8344fd86b`; prior runs remain under `history/stage3/pre-2026-11-10-replay/`.

| Evidence | Executed command / result | Limit |
| --- | --- | --- |
| S3-1 | `cargo test --workspace --locked` PASS: **120 tests**, zero failures, one ignored native networking fixture; four expected compiler-error cases also pass. `cargo fmt --all -- --check` PASS. `stage3-final-workspace-tests.log`, `stage3-final-format-check.log`. | Deterministic Mac core/engine contracts, including committed event paths/cancellation, capture disposal, widget state, grapheme/UTF-16/composition/undo, forms/validation, query races, routing, assets and native semantic projection. Does not certify Windows, OS IME or AT tasks. |
| S3-2 | `bash scripts/build-web.sh`, `npm ci`, `npm run test:host`, `npm run test:wasm`, `npm run test:stage2` PASS. `stage3-final-web-build.log`, `stage3-final-npm-ci.log`, `stage3-final-host-tests.log`, `stage3-final-inherited-wasm.log`, `stage3-final-stage2-wasm.log` and `stage2-wasm.json`. | Actual rebuilt Wasm/CPU in Node; injected host contracts remain distinct from real browser/network evidence. Local frame samples are not frozen performance acceptance. |
| S3-3 | `RUSTX_TEST_PORT=3001 npm run test:http`, `npm run test:stage3:services` PASS. `stage3-final-http-tests.log`, `stage3-final-services.log`. | Actual loopback Rust server → production Fetch/WebSocket adapters; methods/headers/body, streaming pull, echo/close/cleanup, timers/cancel. Clipboard denial is an explicitly injected contract test. No external API/CORS/native Windows claim. |
| S3-4 | `CHROME_PATH='…/Google Chrome' npm run test:stage3:browser`, `RUSTX_BROWSER=webkit npm run test:stage3:browser` PASS. `stage3-browser-{chromium,webkit}.json` and `stage3-final-browser-*.log`/1200px/390px screenshots. | Actual browser forms, dirty/errors/validation/stale results, password redaction, UTF-16/grapheme/undo and queued-selection ordering, modal focus/Escape, IndexedDB drafts, JSON/multipart/stream/socket, file chooser/download, recoverable subtree retry, search/details/printable export, history/reload, drawing edit/10,000-object CSV export, narrow layout and idle. Chromium additionally verifies real clipboard write/readback. Text injection/ordering regressions are not OS IME; printable text is not a physical printer task; WebKit is not Safari. |
| S3-5 | `npm run test:stage2:browser` PASS in Chrome and WebKit CPU; visible Chrome `RUSTX_TEST_GPU=1 RUSTX_HEADED=1` PASS. `stage3-final-stage2-browser-*.log`, `stage2-browser-{chromium-cpu,webkit-cpu,chromium-gpu}.json`. | Actual pointer/keyboard camera, bounded/focus-retaining virtual collection, nested wheel/resize/DPR/idle. Physical Apple Metal 3 direct browser presentation observes 80 submissions / 40 canvas textures and zero texture-to-buffer copies, MAP_READ or CPU uploads in the normal GPU phase. Actual `GPUDevice.destroy` recovers to CPU with retained state, then a fresh GPU device renders. No Windows/driver-reset/full conformance/performance claim. |
| S3-6 | `cargo test -p rustx-engine --locked actual_native_pull_stream_and_socket_echo -- --ignored --nocapture` PASS. Applications `RUSTX_SMOKE_FRAMES=2` native runs PASS with the default renderer and forced CPU. `stage3-final-native-services.log`, `stage3-final-native-gpu-smoke.log`, `stage3-final-native-cpu-smoke.log`. | Actual native Mac loopback stream/socket and two-frame native windows, on final source. Startup is not full native interaction, UIA, IME or Windows evidence. |

Delivery review fixed a missing direct PNG lock dependency and unexported asset modules, joined-grapheme caret/undo/read-only edge cases, controlled combobox synchronization/disabled-option navigation, terminal pointer-capture release after disabling/hiding/modal changes, native history-mode navigation, and stale browser selection events overwriting newer Rust keyboard edits before the next frame. Regression failures and sandbox limitations are retained under `evidence/history/stage3/`. Earlier Stage 2 reports are preserved in `history/stage3/pre-delivery/` before replay against the new Wasm. The final 10 November review adds regressions for composition joining combining marks, regional indicators and ZWJ emoji; successful async validation preserving field/form-level synchronous errors; and logical asset kinds rejecting changes without orphaning renderer resources. `stage3-final-editing-tests.log` and `stage3-delivery-assets.log` record focused passes; before-fix failures are retained under `history/stage3/`. Repeated packaging clears its generated directory/archive so old payloads and manifests cannot invalidate checksums. Source/build metadata is refreshed for Stage 3. No unavailable platform was substituted with a mock or compile-only pass.

Stage 3 implementation delivery does not close its full exit gate: real Windows browser/native forms and services, NVDA/UI Automation, VoiceOver with actual Safari, real Persian/English OS IME, physical mobile keyboards/touch/AT, full native permissions/printer behavior and cross-target renderer/DPI/recovery still require actual target evidence. Stage 4 tooling/release and RX-45/RX-46 final successful examples remain separate.

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
| RX-10 | implemented_unverified | S2-1–S2-4: typed flex/grid alignment/sizing, theme tokens, bounded viewport overrides and responsive dashboard; full target matrix pending. |
| RX-11 | implemented_unverified | S2-1/S2-2: retained Taffy IDs, text/font measurement epochs, reflow and bounded invalidation fixtures; cross-target matrix pending. |
| RX-12 | in_progress | S2-1–S2-5: shared Persian/English/emoji fallback, direction/alignment, clusters/UTF-16 and font readiness; full font/language/ellipsis and editing/conformance contract remains open. |
| RX-13 | in_progress | S2-1/S2-5: shared path/stroke/image/text, affine camera geometry, clips, opacity/shadow/alpha and matched CPU/Metal fixture; complete backend conformance matrix pending. |
| RX-14 | implemented_unverified | S2-4: actual direct Chrome/Metal presentation with zero observed normal-path frame readback; native/target trace matrix pending. |
| RX-15 | implemented_unverified | S2-3/S2-4: actual CPU, initial fallback and real device destruction/recovery/retry with retained state; native Windows/driver-recovery matrix pending. |
| RX-16 | implemented_unverified | S2-1–S2-4: bounded LRU/payload policies, drawing culling, scoped 100k list, focused-row retention, metadata and lifecycle checks; full resource/device matrix pending. |
| RX-17 | implemented_unverified | S2-1–S2-4: public retained CanvasScene, paths/transforms, hit tests, camera-relative large coordinates, pointer/keyboard pan/pick and zoom; native interaction matrix pending. |
| RX-18 | in_progress | S2-2–S2-4: demand-driven idle, resize, viewport/DPR2 and guarded surfaces; hidden/resume/native DPI and platform matrix pending. |
| RX-19 | implemented_unverified | S3-1: committed capture/target/bubble/default actions, pointer ownership/release and cancellation regressions; complete browser/native target replay pending. |
| RX-20 | implemented_unverified | S3-1/S3-4/S3-5: modal/keyboard/slider/nested-scroll/capture behavior; physical touch/mobile and Windows task matrix pending. |
| RX-21 | implemented_unverified | S3-1: finite widget vocabulary, composed controls, semantic/interaction fixtures; full real-target per-widget tasks pending. |
| RX-22 | implemented_unverified | S3-1: grapheme/UTF-16/bidi selection, Rust caret/preedit, undo/redo/read-only; real OS IME, autofill/mobile and full target matrix pending. |
| RX-23 | implemented_unverified | S3-1/S3-4: dirty/touched/reset/errors/controlledness, owner/generation-safe async validation/submission; Windows/AT/mobile target form tasks pending. |
| RX-24 | blocked | Browser semantic/input projection implemented with relationships, modal/focus and password redaction; human screen-reader tasks remain unexecuted. |
| RX-25 | blocked | S3-1 native AccessKit projection/action source and fixtures; actual Windows UIA/NVDA form completion unavailable. |
| RX-26 | implemented_unverified | S3-1–S3-3: methods/headers/body/futures, timeout/status/decode/size/cancel/stale disposal; full browser/native target and public API matrix pending. |
| RX-27 | implemented_unverified | S3-1/S3-3/S3-6: multipart, actual pull streams/WebSocket, size/backpressure/close/disposal; full target/adverse network matrix pending. |
| RX-28 | implemented_unverified | S3-1: bounded query deduplication, observers/invalidation/refetch, mutations and stale/disposal tests; full real-target tasks pending. |
| RX-29 | implemented_unverified | S3-1/S3-4: typed Unicode params/query/fragment, guard/history/hash and native push/replace/back/forward; real browser deep-link/reload/Back. Complete target URL task matrix pending. |
| RX-30 | implemented_unverified | S3-1/S3-3: typed storage/migrations/files/clipboard, explicit denial/quota/cleanup, logical revisioned assets/PNG decode; actual native permission/dialog matrix pending. |
| RX-31 | implemented_unverified | S3-4: Rust search/selection, actual Chromium clipboard readback, actual browser printable text export; native/screen-reader/physical printer matrix pending. |
| RX-32 | implemented_unverified | S3-4: actual recoverable subtree retry; fatal host reload UI implemented. Complete native/browser failure matrix pending. |
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

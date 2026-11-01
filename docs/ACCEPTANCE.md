# Acceptance ledger — 1 November 2026

Authoritative register: BLUEPRINT.md RX-01–RX-46. Exact snapshot: SOURCE-REVISION.txt (SHA-256 of SOURCE_MANIFEST.sha256). **Stage 1 is still open.** Four of 46 requirements are verified at their core contract level: checklist coverage, not effort completed or platform certification.

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

## Missing platform evidence

- Browser process/window socket creation is blocked in this environment: Unix socket EPERM recorded in evidence/platform-probe.log. Browser suite supplied, not run successfully. Retry on a normal workstation/CI.
- No actual Windows machine/session was exercised. No Windows binary is included. Native Linux compilation/offscreen output does not count as Windows.
- No physical GPU/browser presentation trace, real Safari/mobile device, screen-reader or OS IME task completed. Software GL is only API-path evidence.
- JSONPlaceholder action is implemented; current success proof uses deterministic local fixtures. Live public API and browser/CORS checks remain open.

## Requirement states

| ID | Status | Evidence / remaining gate |
| --- | --- | --- |
| RX-01 | implemented_unverified | Pinned Rust/Cargo/npm locks and scripts; independent fresh-machine/Windows replay pending. |
| RX-02 | verified | E1: typed components/default props/generics; four expected compiler-error fixtures. |
| RX-03 | verified | E1: imported component/Props, initial and reactive props, children/fragments/named View slot; E2/E3 demo crate. |
| RX-04 | verified | E1/E3: mount once, memo/effect cleanup, dynamic dependencies, batching, scoped handles/refs, bounded cycles. |
| RX-05 | verified | E1/E3: reorder keeps keyed state/identity; removal disposes and resets; duplicate-key diagnostic. |
| RX-06 | in_progress | E1/E3/E4: typed GET/cancel, safe numeric IDs, stale completions and limits; version/capability contract missing. |
| RX-07 | blocked | Wasm/native source builds; real browser and Windows execution pending. |
| RX-08 | in_progress | E5 CPU and software-GL scene rendered; filtering differs; physical/browser matrix pending. |
| RX-09 | blocked | Semantic/input bridge source exists; real keyboard/AT/IME tasks missing. |
| RX-10 | in_progress | Typed styles, grid responsiveness and a small font breakpoint; full theme/query contract missing. |
| RX-11 | in_progress | Taffy integrated; dirty propagation and complete layout fixture suite pending. |
| RX-12 | in_progress | Persian/English samples shape; font fallback/emoji and full bidi fixtures pending. |
| RX-13 | in_progress | E5 shared scene renders; backend conformance not passed. |
| RX-14 | implemented_unverified | Direct browser surface source compiles; presentation trace pending. |
| RX-15 | in_progress | CPU and initial GPU fallback; device loss/recovery incomplete. |
| RX-16 | not_started | No acceptance implementation/evidence yet. |
| RX-17 | not_started | No acceptance implementation/evidence yet. |
| RX-18 | in_progress | E3 idle and viewport checks; actual DPI/background/platform suite pending. |
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
| RX-41 | in_progress | Locks, MIT/font notices; transitive license/unsafe/security/CSP audit pending. |
| RX-42 | in_progress | README/current handoff; complete public tutorial/migration guide pending. |
| RX-43 | in_progress | Shared-source demo; cross-target behavior matrix pending. |
| RX-44 | in_progress | Current source/bundle/checksums/evidence; final release package pending. |
| RX-45 | in_progress | People Explorer fixture path verified; actual public API/browser/native/search/details/all-state gates pending. |
| RX-46 | in_progress | Homepage/preset interactions verified; editable Rust compile/run playground pending. |

## Stage 1 manual gates

Record source hash, OS/browser/driver versions, backend, viewport/DPR, commands/tasks and actual results. Retain failures.

- [ ] Browser CPU keyboard/Tab/focus/reorder/unmount tasks pass without duplicate activation.
- [ ] Labeled button and editable field are usable with a real screen reader.
- [ ] Real Persian/English IME composition/cancel/commit and focus-retention tasks pass.
- [ ] Direct WebGPU presentation, forced CPU and initial GPU failure are exercised.
- [ ] Shared GPU/CPU scene parity is resolved across browser/native paths.
- [ ] Same app runs on real Windows in both backends with resize/DPI/HTTP/input checks.
- [ ] Host capability/version contract and mismatch/queue bounds are completed.
- [ ] Independent fresh-machine replay and exact benchmark hardware profiles are recorded.

Do not close Stage 1 until these gates pass. Final release additionally requires RX-45/RX-46's two successful examples as defined in blueprint Section 13.2.1.

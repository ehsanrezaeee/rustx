# RustX roadmap

RustX has delivered four implementation stages. The current milestone is **RP-008 native interaction**. The Windows CPU form has passed forward Tab and dialog restoration checks. Reverse Tab on Windows and the macOS interaction replay are still open.

RustX uses TSX-like markup for authoring and its own runtime and renderer. The performance goals are to match or beat optimized plain JavaScript canvas rendering on the web and to deliver a substantial native speed and responsiveness advantage on equivalent desktop workloads. The [vision implementation guide](VISION_IMPLEMENTATION.md) defines the workloads and how to verify those goals.

## Current stage review

| Stage | Delivered | Stability fixes and remaining work |
| --- | --- | --- |
| 1 · Authoring and lifecycle | Typed markup, mount-once scopes, signals, keyed children, browser and native entry points | Signal callbacks and destructors can access live state without a conflicting runtime borrow. Memos initialize once, and a broad reactive update can settle in one commit. Real browser back/forward cache behavior and full cross-target lifecycle replay remain open. |
| 2 · Shared rendering | Rust layout, shaping, canvas, virtual lists, and CPU/GPU backends | Browser line- and page-mode wheel input is converted to logical pixels. Native wheel events expose both axes to Rust handlers. Native GPU recovery, driver coverage, mixed DPI, and workload-specific frame budgets remain open. |
| 3 · Interaction and services | Forms, text editing, routing, semantics, networking, and platform services | Stacked dialogs restore the correct opener, hidden descendants stay out of the tab order, and the browser updates a projected field when its control type changes. History routing preserves queries at the root URL. Clipboard, file dialogs, pointer capture across the window boundary, physical touch, OS IME, and assistive-technology checks remain open. |
| 4 · Tooling and delivery | Scaffold, CLI, dev reload, editor, web serving, offline shell, and local playground | Windows builds use distinct library and executable names for debug symbols. Web dev reserves its port before building, and web builds protect unrelated output directories. CI builds the current demo from source. Independent onboarding, license/advisory review, and hosted compiler review remain open. |

## Project roadmap

| Milestone | State and next work |
| --- | --- |
| RP-001 | Define the reference application workflows, user-facing requirements, and acceptance checks. Begin with scene editing, a large data workspace, and live telemetry. |
| RP-005 | Benchmark RustX Wasm against tuned plain JavaScript canvas implementations. Compare native RustX with the equivalent browser workloads on the same hardware. Record per-workload measurements and comparison limits. |
| RP-002, RP-003, RP-004, and RP-006 | Browser semantics cleanup, context isolation, browser resource accounting, and external-store snapshots have shipped. Real BFCache restoration and broader device checks remain open. |
| RP-007 | Windows and macOS developer-package startup was verified. The current delivery builds from source and produces no app ZIP. Clean-machine and signed distribution checks remain open if binary distribution resumes. |
| **RP-008** | Windows CPU checks cover posted Tab, Enter, Escape, Space, and resize, with UI Automation readback. The form skipped disabled and hidden controls, confined focus to the top dialog, and restored both openers. Next verify native Shift+Tab, replay on macOS, then cover pointer capture, scrolling, clipboard, files, mixed DPI, and multi-window state. |
| RP-009 | Verify native text, IME composition, multilingual editing, and human screen-reader interaction. |
| RP-010 | Verify native rendering and device-loss recovery. Measure frame work, latency, and sustainable throughput against the browser results and the native targets in the vision guide. |
| RP-011 | Verify browser input and accessibility on desktop and physical Android/iOS, including Safari and Firefox coverage. |

## Evidence

The pre-cleanup Windows and macOS package run and Windows counter interaction are recorded in Actions. The cleanup retains the historical records in Git at `7c6edc1`. Reports and standalone replay files live outside the tracked source tree. [Previous native run](https://github.com/ehsanrezaeee/rustx/actions/runs/44459070847) · [Historical records](https://github.com/ehsanrezaeee/rustx/tree/7c6edc14d2066b4801184c03637d64a9c776b3c6/docs)

The cleanup replay passed 21 Rust unit checks, a separate native loopback stream and socket check, and formatting. The Windows source-built Applications demo presented two CPU frames. A fresh scaffold built for web and native, and Edge reloaded the running scaffold after a RustX edit. The production browser demo drew the CPU canvas at a narrow viewport, fetched updated configuration through a controlling service worker, reloaded offline, fetched the local sample over HTTP, and reached Applications by keyboard. Physical touch and native GPU, IME, and screen-reader tasks remain open.

The stability pass ran 28 workspace unit checks, 46 local regression checks, and the native loopback stream and socket check. The Windows release CLI built the current Applications demo, which exited after its two-frame CPU smoke run. A separate CPU form replay sent Tab, Enter, and Escape to a Windows window and verified focus through UI Automation. In Edge 157 on Windows, the source-built form passed text editing, forward and reverse keyboard navigation, application-controlled Tab, nested dialog focus restoration, and input-to-textarea updates. The current source-built demo also passed the narrow viewport, service worker, offline, fixture API, keyboard navigation, and line-mode wheel checks. The current CLI reloaded the running scaffold after a RustX edit, preserved the existing bundle on a busy dev port, and refused an unrelated output directory.

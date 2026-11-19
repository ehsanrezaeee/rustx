# RustX roadmap

RustX has delivered four implementation stages. The current milestone is **RP-008 native interaction**. The next delivery is the multi-control form and dialog keyboard check on Windows, followed by macOS interaction replay where available.

## Current stage review

| Stage | Delivered | Cleanup fixes and remaining work |
| --- | --- | --- |
| 1 · Authoring and lifecycle | Typed markup, mount-once scopes, signals, keyed children, browser and native entry points | Browser resume now waits for initialization before invalidating the app. Real browser back/forward cache behavior and full cross-target lifecycle replay remain open. |
| 2 · Shared rendering | Rust layout, shaping, canvas, virtual lists, and CPU/GPU backends | CPU recovery invalidates the Rust frame after replacing the browser canvas. Native pixel wheel deltas now use logical coordinates. Native GPU recovery, driver coverage, mixed DPI, and the 16.7 ms GPU frame budget remain open. |
| 3 · Interaction and services | Forms, text editing, routing, semantics, networking, and platform services | Native pointer exit preserves an active capture and leaves composition alone. Window focus loss clears transient input state and redraws. Clipboard, file dialogs, pointer capture across the window boundary, physical touch, OS IME, and assistive-technology checks remain open. |
| 4 · Tooling and delivery | Scaffold, CLI, dev reload, editor, web serving, offline shell, and local playground | Builds resolve Windows Cargo paths consistently and generate distinct library and executable names for Windows debug symbols. Web dev reports bind failures, preserves the previous bundle on a failed replacement, and fetches fresh mutable assets when online. CI builds the current demo from source. Independent onboarding, license/advisory review, and hosted compiler review remain open. |

## Project roadmap

| Milestone | State and next work |
| --- | --- |
| RP-001 and RP-005 | Complete the per-API parity inventory and build equivalent workloads for comparative benchmarks. |
| RP-002, RP-003, RP-004, and RP-006 | Browser semantics cleanup, context isolation, browser resource accounting, and external-store snapshots have shipped. Real BFCache restoration and broader device checks remain open. |
| RP-007 | Windows and macOS developer-package startup was verified. The current delivery builds from source and produces no app ZIP. Clean-machine and signed distribution checks remain open if binary distribution resumes. |
| **RP-008** | The Windows CPU counter passed posted Tab, Enter, resize, and Space with UI Automation readback. Next verify Tab and Shift+Tab order, disabled controls, dialog focus confinement, and Escape restoration. Then cover pointer capture, scrolling, clipboard, files, mixed DPI, and multi-window state. |
| RP-009 | Verify native text, IME composition, multilingual editing, and human screen-reader interaction. |
| RP-010 | Verify native rendering and device-loss recovery. Measure idle work, latency, and frame times against the existing budgets. |
| RP-011 | Verify browser input and accessibility on desktop and physical Android/iOS, including Safari and Firefox coverage. |

## Evidence

The pre-cleanup Windows and macOS package run and Windows counter interaction are recorded in Actions. The cleanup retains the historical records in Git at `7c6edc1`. Reports and standalone replay files live outside the tracked source tree. [Previous native run](https://github.com/ehsanrezaeee/rustx/actions/runs/44459070847) · [Historical records](https://github.com/ehsanrezaeee/rustx/tree/7c6edc14d2066b4801184c03637d64a9c776b3c6/docs)

The cleanup replay passed 21 Rust unit checks, a separate native loopback stream and socket check, and formatting. The Windows source-built Applications demo presented two CPU frames. A fresh scaffold built for web and native, and Edge reloaded the running scaffold after a RustX edit. The production browser demo drew the CPU canvas at a narrow viewport, fetched updated configuration through a controlling service worker, reloaded offline, fetched the local sample over HTTP, and reached Applications by keyboard. Physical touch and native GPU, IME, and screen-reader tasks remain open.

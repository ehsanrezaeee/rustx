# Native and browser parity execution ledger — revision 2

17 November 2026. Native delivery is the current execution priority. RP-007 is authorized for implementation, commit and push. Its Windows CI repair and replay are in progress; see [repair evidence](evidence/react-parity/RP-007/windows-fix-2026-11-17/README.md). RX release gates remain binding; finite verification is not full platform acceptance.

## Complete task accounting (review before each increment)

| Task | Actual status | Outstanding work / next action |
| --- | --- | --- |
| RP-000 | Never assigned | M0 is a roadmap milestone, not a completed RP-000 task; numbering began at RP-001 |
| RP-001 | **Implementing; incomplete** | Finish per-API/HTML/SVG/property/event inventory and target evidence; native packaging now contributes a concrete native row; matched apps remain RP-005 |
| RP-002 | **Verified finite semantic-adapter cleanup** | Does not certify whole-host/BFCache/AT behavior; host follow-up is RP-004, browser conformance RP-011 |
| RP-003 | **Verified finite scoped-context contract** | Full reactive provider semantics are still missing in the parity matrix; native evidence was startup only |
| RP-004 | **Verified measured CPU host/DOM slice; platform follow-ups open** | Actual BFCache restore, GPU lifecycle, multi-root ownership and physical browser input/AT remain unverified; do not mark full browser lifecycle complete |
| RP-005 | **Proposed; not started** | Implement four equivalent RustX/React apps, fixtures/locks and real target baselines; missing content/media features block full workload D; no benchmark winner yet |
| RP-006 | **Verified finite UI-thread push-store contract** | Generic pull/subscribe adapters, hydration and multithreaded semantics remain outside the implemented API; native interaction evidence still limited |
| RP-007 | **Implemented; macOS package/startup verified, Windows and release gates open** | Delivered scaffold → package → extract → launch workflow; actual macOS CPU startup passed; Windows workflow results remain required; signing, installers and clean-machine checks remain explicit until verified |
| RP-008 | **Proposed: native interaction conformance** | Keyboard/focus/pointer/scroll/clipboard/files/dialogs/DPI on actual Windows and macOS |
| RP-009 | **Proposed: native text and accessibility** | Real OS IME, multilingual editing, platform accessibility trees and human screen-reader tasks |
| RP-010 | **Proposed: native rendering and performance** | CPU/GPU lifecycle/recovery, multi-window/resource cleanup and measured native workloads; RP-005 required for comparative claims |
| RP-011 | **Proposed: browser input and accessibility conformance** | Carries the earlier browser-input proposal without taking the native RP-007 slot; full browser scope remains required |

Unfinished tasks are retained with their original IDs. Next means execution priority, not all lower IDs completed. Every handoff must update this table, record new evidence/limits, and name pending acceptance rather than hiding it in a higher-numbered milestone.

## RP-001 — Capability audit and comparative baseline contract

Milestone M0; owner implementing agent; state **implementing**. No prerequisites. Outcome: auditable source gaps and fair reference workloads, not new parity claims. Source/test anchors are in REACT_PARITY_MATRIX.md. No API change.

Delivered: named React 19.3 reference API groups, all roadmap outcome areas, runtime/owner/task findings, DOM exception register, workload contract below and prioritized backlog. Remaining: individual HTML/SVG tags, attributes/input types/events and per-API behavior tests; executable matched React applications with exact dependency lock; actual target/device provisioning. RP-004 has since measured eight RustX DOM routes; that does not supply the missing matched React baselines. Therefore RP-001 and M0 are not complete.

Acceptance: every required surface has a row with source, status, task and target evidence; matched apps implement equivalent behavior and frozen workload checks; no benchmark winner without raw comparable runs. Documentation checks only in this increment. RP-003 has since delivered its finite slice; continue this inventory in bounded batches alongside RP-007. Evidence: evidence/react-parity/RP-002/README.md describes this increment and its limits.

## RP-002 — Dispose the browser semantic projection completely

Milestone M1/M2 lifecycle slice; owner implementing agent; state **verified** for the finite adapter lifecycle contract only; evidence: [RP-002](evidence/react-parity/RP-002/README.md). Depends on the initial RP-001 source inspection. Outcome: unmount releases only projection-owned DOM nodes/listeners and composition/focus references. Excludes complete host teardown, native lifecycle, BFCache, AT and IME certification.

Source: web/semantics.js; existing tests scripts/test-host-events.mjs and scripts/test-browser.mjs. Public API unchanged: dispose is idempotent, sync after disposal is a no-op, create after disposal rejects. Each node's listeners have a scoped AbortController, aborted on removal/disposal. WeakMap avoids retaining removed nodes. Foreign root children remain intact.

Acceptance fixed before change: 100 create/dispose cycles leave the root's one foreign child intact; each live button activates exactly once; retained detached buttons never activate the disposed receiver; all owned state clears; double disposal is safe. Real Chromium/WebKit DOM execution with stub Rust callback receiver must be labeled adapter-only. Rebuild and run inherited Wasm/application tests for host regression confidence. Before-fix failure retained.

RP-003 was authorized in the following turn.

## RP-003 — Context and owner isolation contract

Milestone M1; owner implementing agent; state **verified** for the finite core contract; [evidence](evidence/react-parity/RP-003/README.md) records ten focused regressions, 144 passing workspace tests and rebuilt integration replays. Depends on RP-001 initial audit. Source: Scope::provide/context, owner/dispose in crates/rustx-core/src/lib.rs; context_children_fragments in crates/rustx/tests/runtime.rs.

Outcome: library authors can rely on nearest provider and owner lifetime. Excludes new external-store/scheduler APIs. First write a short ADR documenting plain-value lookup versus provided Signal reactivity, replacement behavior, runtime isolation and disposed scope access. Add regressions for nested shadowing, siblings, same TypeId in independent roots, provider disposal/remount, retained signal cleanup, dynamic signal reads and no unwanted subscription on plain lookup. Fix only reproduced violations. No migration unless an existing contract is unsafe; document any deliberate change.

Acceptance: exact expected values/identity and cleanup counts across all cases; compile and execute tests, not API-name assertions. Run workspace/format and rebuilt Wasm when runtime code changes; native/browser context demo required before any platform usability claim. Evidence goes to evidence/react-parity/RP-003/. The historical next increment RP-006 has since delivered its finite push-store contract; current native priority is RP-007.

## RP-004 — DOM accounting and host lifecycle

Milestone M2; owner current browser agent; state **verified for the measured CPU host/DOM contract**; depends on RP-002. Delivered ADR-015, opt-in main-document category/mutation counters, frozen eight-route budgets, dev-only status, singleton host disposal/restart, scoped preview/tools, Rust runtime disposal exports and synchronous HTTP registration/timeout cleanup. Chromium/WebKit execute 100 route and 100 real Rust root cycles plus production/recovery checks. [Evidence](evidence/react-parity/RP-004/README.md). Actual BFCache restoration, GPU, physical IME/AT/mobile and simultaneous roots remain open; this is not full browser lifecycle certification. Sources: DOM_EXCEPTIONS.md, web/host.js, web/semantics.js, scripts/test-stage4-budgets.mjs. Outcome: measured bridge resource bounds and reusable host teardown. No input pooling before actual AT/IME checks.

Implement opt-in category/count/mutation instrumentation, then 100 route/root cycles with real Rust app. Specify pagehide/pageshow/BFCache semantics and multi-root ownership before changing listeners. Move ordinary status rendering to Rust or confine to devtools. Preserve fatal-host recovery exception. Acceptance: owned live counts return to baseline, no stale runtime callbacks, no ordinary DOM visual widgets and no semantic loss. Freeze workload DOM budgets before optimization. Evidence needs real browsers; mocks only test accounting helpers. Next native priority: RP-007; browser input/AT is retained as RP-011 and actual BFCache remains an RP-004 follow-up.

## RP-005 — Matched benchmark applications and lockfiles

Milestone M0/M5; owner next benchmark agent; state **proposed**; depends on RP-001 workload contract, not full RP-003. Sources: examples/lab, scripts/test-stage4-budgets.mjs, docs/stage4/PERFORMANCE.md. Outcome: four executable equivalents and reproducible runners. API changes not required. No performance optimization in the baseline change.

Pin React/react-dom 19.3.0 and compatible build/compiler/library versions in an isolated benchmark lockfile; validate registry availability before installation. Record exact RustX revision and build flags. Select available real hardware/OS/browser versions before runs; physical Android/iOS/clean Windows availability remains open. Acceptance: task assertions/data/result hashes match, full accessible builds and raw traces retained, no universal performance claims. Next: measured profiling to choose one optimization.

## Frozen workload contract v1 (before comparative measurement)

| Workload | Dataset and required behavior | Functional oracle |
| --- | --- | --- |
| A: admin/forms | 1,000 synthetic records, 20 editable fields; search/details, nested navigation, sync/async validation, cancel/retry, optimistic update with stale response, dialog and draft restore | Same fixture responses, validation results, final saved record and keyboard tasks |
| B: virtual grid | 100,000 deterministic records, 20 visible fixed-height rows, 2 viewports overscan per side; filter/sort/select/edit, focused-row retention | Same sorted IDs/results, at most 120 materialized row scopes plus documented bounded focus retention |
| C: diagram | 10,000 deterministic objects, at most 1,000 visible; pan/zoom/pick/drag, labels, undo/redo and export | Same selected IDs, transforms and exported geometry; compare mature React canvas renderer as well as relevant DOM tasks |
| D: content/media | 10 deep-linkable content routes, 3 nested loading boundaries, multilingual article, image/audio/video/captions, form and metadata; print, offline reload, failed activation | Same accessible content, route metadata, submitted values and documented media/browser tasks |

Fixture generator seed: 1; commit generator and its output hash before first comparison. Viewports: desktop 1280×720 DPR 1; mobile 390×844 with physical device DPR recorded separately. Same fonts/media/data/network faults across implementations. Existing demos are starting material, not completed equivalents. Missing RustX features in workload D block the full comparison; do not delete those tasks.

Retain absolute RX budgets (including 16.7 ms desktop GPU and 3-second throttled cold startup) and roadmap sample counts. Candidate advantages: 25% lower memory or 20% lower p95 input latency on two workloads. Every other critical metric may regress by at most 10% relative to baseline and must still satisfy absolute budgets; if measurement uncertainty exceeds the threshold, report inconclusive. Record CPU fallback separately. Require real input-to-present method for a presentation claim; rAF/synthetic proxies are labeled. No hardware profile is certified by this contract; freeze exact available machines before taking measurements.

## RP-006 — External-store snapshot contract

Milestone M1/M3; owner current implementing agent; state **verified for the finite push-store contract**; depends on RP-003. Delivered ExternalStore/StoreSnapshot/StoreSubscription, ADR-014, ten regressions and a native/browser canvas example at `#/store`. Evidence and explicit compatibility limits: [RP-006](evidence/react-parity/RP-006/README.md). Define subscription ownership, coherent snapshot reads, notification during commit, deduplication, replacement, cross-runtime boundaries and disposal before adding a public API. Inspect Signal, TaskQueue and QueryCache; do not equate their presence with an external-store contract. Deliver a small ADR, a public store adapter and race/disposal regressions with a runnable example. Acceptance: no torn snapshots or callbacks after disposal, bounded repeated notifications and independent-root behavior; actual browser/native examples are required for platform claims. Preserve mount-once setup and keep browser DOM out of core APIs. Evidence belongs in evidence/react-parity/RP-006/. This is a single-UI-thread push adapter, not full useSyncExternalStore compatibility. Native evidence is startup only; Chromium/WebKit exercise canvas pointer interactions. RP-004 subsequently delivered its measured slice; native delivery now continues in RP-007.

## RP-007 — Native build and packaging

Milestone M2/M5; owner current implementing agent; state **implemented_unverified for full delivery; macOS finite package/startup slice verified**. Depends on the existing native host/scaffold, not on claiming RP-001 or RP-005 complete. Outcome: developers can generate, package and hand over a standalone native app without a recipient Rust/browser toolchain. Windows/macOS are required; platform results stay separate.

Deliver `rustx package --target native|macos|windows`, host-specific binary selection, macOS .app/ZIP and Windows portable ZIP, dependency inspection, static Windows CRT, package identity/checksums, framework/font/dependency license inventory, and a generated-project native quick start. Preserve the existing raw native build command. Refuse wrong-host packaging, unsafe output paths and replacement of existing package directories. No new browser/DOM UI is part of this task.

Acceptance: scaffold an independent app, package it, extract and verify all hashes, hide its source, remove toolchains from launch PATH, and execute the actual native CPU window from an unrelated directory. Record OS/architecture/dependencies and the limits of two-frame smoke separately from interaction. Add actual-platform CI for Windows and macOS and retain artifacts. Clean machines without installed SDKs, signed distribution/notarization, installers/update/uninstall and Linux packaging are open delivery gates; a compiled package or runner with a restricted PATH does not close them. Do not mark RP-007 fully verified until required target evidence exists.

Sources: crates/rustx-cli/src/tooling/native.rs, scaffold templates, scripts/test-native-package.mjs. Evidence: evidence/react-parity/RP-007/. Next: fill missing RP-007 target checks, then RP-008; keep RP-001/RP-005 visible.

## RP-008 — Native interaction conformance

Milestone M2/M4; proposed. Use an extracted RP-007 package on actual Windows/macOS. Freeze tasks for keyboard/tab order/focus restoration, pointer capture, wheel/scroll, clipboard, file dialogs, window resizing, scale/DPI and multi-window isolation. Reproduce/fix concrete host gaps and record CPU and GPU separately. Mock service tests are not native OS evidence. Browser counterparts remain RP-011. Next: RP-009 text/AT.

## RP-009 — Native text and accessibility

Milestone M2/M4; proposed. Exercise real OS composition/IME, UTF-16/grapheme boundaries, RTL/multilingual selection and editing, accessible names/roles/actions and focus announcements. Required platforms include Windows UIA/NVDA and macOS accessibility/VoiceOver. Human/physical tasks remain pending when unavailable; synthetic composition cannot close them. Depends on RP-008 baseline tasks. Next: RP-010 reliability/performance.

## RP-010 — Native rendering, lifecycle and performance

Milestone M2/M6; proposed. Verify native CPU/GPU drawing, resize/DPI, device loss/recovery, multi-window isolation, idle behavior and bounded lifetime/resource use. Freeze native workloads/hardware/methodology before optimization. Use RP-005 matched workloads before any comparative React/Electron claim; keep absolute RX budgets unchanged. Native mobile/Linux expansion remains separately scoped, not inferred from desktop compilation.

## RP-011 — Browser input and accessibility conformance

Milestone M2; proposed. Preserve the full browser track: keyboard/focus, pointer/touch, text composition, platform editing/selection, semantic actions and accessibility. Run real browser tasks; real IME/AT/mobile checks remain distinct. This retains the earlier suggested browser-input milestone under a new ID; it must not displace the native RP-007–010 priority. Coordinate unresolved BFCache tasks under RP-004.

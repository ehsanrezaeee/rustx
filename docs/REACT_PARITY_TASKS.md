# React parity execution ledger — revision 1

16 November 2026. This ledger adds RP tasks without changing RX verification. The roadmap is REACT_PARITY_PLAN.md; source findings are REACT_PARITY_MATRIX.md. Current milestone: RP-003 context/owner isolation, authorized for commit and push after verification. Pause before the next increment.

## RP-001 — Capability audit and comparative baseline contract

Milestone M0; owner implementing agent; state **implementing**. No prerequisites. Outcome: auditable source gaps and fair reference workloads, not new parity claims. Source/test anchors are in REACT_PARITY_MATRIX.md. No API change.

Delivered: named React 19.3 reference API groups, all roadmap outcome areas, runtime/owner/task findings, DOM exception register, workload contract below and prioritized backlog. Remaining: individual HTML/SVG tags, attributes/input types/events and per-API behavior tests; executable matched React applications with exact dependency lock; actual target/device provisioning and measured DOM baseline. Therefore RP-001 and M0 are not complete.

Acceptance: every required surface has a row with source, status, task and target evidence; matched apps implement equivalent behavior and frozen workload checks; no benchmark winner without raw comparable runs. Documentation checks only in this increment. Next smallest task: RP-003, while expanding RP-001's inventory in bounded batches. Evidence: evidence/react-parity/RP-002/README.md describes this increment and its limits.

## RP-002 — Dispose the browser semantic projection completely

Milestone M1/M2 lifecycle slice; owner implementing agent; state **verified** for the finite adapter lifecycle contract only; evidence: [RP-002](evidence/react-parity/RP-002/README.md). Depends on the initial RP-001 source inspection. Outcome: unmount releases only projection-owned DOM nodes/listeners and composition/focus references. Excludes complete host teardown, native lifecycle, BFCache, AT and IME certification.

Source: web/semantics.js; existing tests scripts/test-host-events.mjs and scripts/test-browser.mjs. Public API unchanged: dispose is idempotent, sync after disposal is a no-op, create after disposal rejects. Each node's listeners have a scoped AbortController, aborted on removal/disposal. WeakMap avoids retaining removed nodes. Foreign root children remain intact.

Acceptance fixed before change: 100 create/dispose cycles leave the root's one foreign child intact; each live button activates exactly once; retained detached buttons never activate the disposed receiver; all owned state clears; double disposal is safe. Real Chromium/WebKit DOM execution with stub Rust callback receiver must be labeled adapter-only. Rebuild and run inherited Wasm/application tests for host regression confidence. Before-fix failure retained.

RP-003 was authorized in the following turn.

## RP-003 — Context and owner isolation contract

Milestone M1; owner implementing agent; state **verified** for the finite core contract; [evidence](evidence/react-parity/RP-003/README.md) records ten focused regressions, 144 passing workspace tests and rebuilt integration replays. Depends on RP-001 initial audit. Source: Scope::provide/context, owner/dispose in crates/rustx-core/src/lib.rs; context_children_fragments in crates/rustx/tests/runtime.rs.

Outcome: library authors can rely on nearest provider and owner lifetime. Excludes new external-store/scheduler APIs. First write a short ADR documenting plain-value lookup versus provided Signal reactivity, replacement behavior, runtime isolation and disposed scope access. Add regressions for nested shadowing, siblings, same TypeId in independent roots, provider disposal/remount, retained signal cleanup, dynamic signal reads and no unwanted subscription on plain lookup. Fix only reproduced violations. No migration unless an existing contract is unsafe; document any deliberate change.

Acceptance: exact expected values/identity and cleanup counts across all cases; compile and execute tests, not API-name assertions. Run workspace/format and rebuilt Wasm when runtime code changes; native/browser context demo required before any platform usability claim. Evidence goes to evidence/react-parity/RP-003/. Next after this: RP-006 external-store snapshot design, after owner confirmation.

## RP-004 — DOM accounting and host lifecycle

Milestone M2; owner next browser agent; state **proposed**; depends on RP-002. Sources: DOM_EXCEPTIONS.md, web/host.js, web/semantics.js, scripts/test-stage4-budgets.mjs. Outcome: measured bridge resource bounds and reusable host teardown. No input pooling before actual AT/IME checks.

Implement opt-in category/count/mutation instrumentation, then 100 route/root cycles with real Rust app. Specify pagehide/pageshow/BFCache semantics and multi-root ownership before changing listeners. Move ordinary status rendering to Rust or confine to devtools. Preserve fatal-host recovery exception. Acceptance: owned live counts return to baseline, no stale runtime callbacks, no ordinary DOM visual widgets and no semantic loss. Freeze workload DOM budgets before optimization. Evidence needs real browsers; mocks only test accounting helpers. Next: input/AT platform conformance.

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

Milestone M1/M3; owner current implementing agent; state **verified for the finite push-store contract**; depends on RP-003. Delivered ExternalStore/StoreSnapshot/StoreSubscription, ADR-014, ten regressions and a native/browser canvas example at `#/store`. Evidence and explicit compatibility limits: [RP-006](evidence/react-parity/RP-006/README.md). Define subscription ownership, coherent snapshot reads, notification during commit, deduplication, replacement, cross-runtime boundaries and disposal before adding a public API. Inspect Signal, TaskQueue and QueryCache; do not equate their presence with an external-store contract. Deliver a small ADR, a public store adapter and race/disposal regressions with a runnable example. Acceptance: no torn snapshots or callbacks after disposal, bounded repeated notifications and independent-root behavior; actual browser/native examples are required for platform claims. Preserve mount-once setup and keep browser DOM out of core APIs. Evidence belongs in evidence/react-parity/RP-006/. This is a single-UI-thread push adapter, not full useSyncExternalStore compatibility. Native evidence is startup only; Chromium/WebKit exercise canvas pointer interactions. Pause for owner confirmation before RP-004 DOM exception accounting.

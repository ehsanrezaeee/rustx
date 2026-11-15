# RustX: full browser React coverage on canvas and a native-first core

Planning date: 16 November 2026. Status: proposed implementation roadmap, not a completion claim. Owner: ehsanrezaeee.

## 1. Mission and precedence

Make RustX a practical choice over React + TypeScript/TSX for interactive applications, with a shared Rust core that runs natively without a browser engine or DOM. Full React-equivalent browser application coverage is a required delivery track, alongside native applications. Native applications retain architectural priority, but browser capability coverage cannot be reduced to a Wasm demo or deferred behind native completion. Both targets use the same Rust-owned canvas/scene model. This updates the older blueprint's browser-first priority for future work without retroactively accepting any unfinished RX requirement.

Preserve mount-once component setup, scoped reactive lifetimes, Rust-owned layout/text/editing/scene, and real CPU/GPU paths. Extend the existing implementation instead of starting a new renderer or rewriting the runtime without a measured need. Browser accessibility and input bridges remain necessary while targeting ordinary browsers. Removing them before an equivalent platform capability exists would regress usability. A future standalone native runtime can be DOM-free; a canvas browser application still needs a browser shell and platform adapters.

Parity means equivalent developer and user outcomes, not implementing React's internal rendering algorithm or accepting arbitrary TSX unchanged. Rust ownership and types can require different APIs. Do not advertise source compatibility with JavaScript packages, HTML/CSS, or React hooks unless an actual compatibility layer passes a published contract.

Two explicit release levels, both required by this roadmap:

1. **Interactive application parity:** accessible forms, data-heavy tools, routing, asynchronous state, composition, reliable development tools, and native/browser deployment. This is an intermediate adoption milestone, not the final browser scope.
2. **Broad React ecosystem alternative:** content delivery, discoverability, server workflows, mature libraries and migration tooling as well as interactive apps. Do not describe level 1 as a universal React replacement.

## 2. Starting point and evidence discipline

Read AGENTS.md, BLUEPRINT.md, STATUS.md, ACCEPTANCE.md, NEXT_AGENT.md, REVIEW.md and stage4/{SUPPORT,PERFORMANCE,SECURITY}.md before coding. Use the current source and ledger to distinguish absent features from implemented but unverified contracts.

Existing foundations include signals/memos/effects, keyed views, scoped cleanup, typed markup, retained layout and text, CPU/GPU scenes, widgets, editing/forms, routing/services/resources, diagnostics, scaffolding and web packaging. The 16 November checkout replay records 134 Rust tests and actual Chromium CPU tasks. It does not establish native platform parity. Existing blockers include manual accessibility/IME, clean native installation, cross-target conformance, the strict GPU frame budget and dependency findings.

Keep the RX-01–RX-46 acceptance register intact. Give new work IDs `RP-001`, `RP-002`, etc. Every task must record:

```text
ID / milestone / owner / state
User-visible outcome and excluded scope
Dependencies and prerequisite IDs
Current source paths and existing tests inspected
Proposed public API and invariant being protected
Implementation steps and migration impact
Deterministic checks and actual target/manual tasks
Acceptance threshold fixed before measurement
Evidence paths, exact source/tool/artifact versions, failures and limitations
Next smallest runnable task
```

Allowed states: proposed, ready, implementing, implemented_unverified, verified, blocked. A source implementation is not verified by naming an API or compiling it. Keep unavailable platform tasks open; proceed with independent useful work. Never delete failures, lower thresholds after measurement, or substitute mocks for browser/native execution.

## 3. Outcome coverage matrix

This is an audit checklist, not an assertion that these features are missing. For each row, find current APIs/tests, identify gaps and create bounded RP tasks. React library features, browser features and ecosystem capabilities are deliberately distinguished.

| Area | Required RustX outcome | Hard acceptance cases |
| --- | --- | --- |
| Composition and TSX ergonomics | Typed props/defaults, generic components, children, fragments, named slots, conditional/keyed views, reusable custom reactive abstractions | Diagnostics at user source spans; generic props; nested composition; duplicate keys; state preservation and deliberate reset |
| State and derived values | Signals, reducer-style state transitions, memo/selectors, transactions, predictable equality, effect timing | Glitch-free derived reads; nested batching; reentrancy; dynamic dependencies; no lost updates or leaked subscriptions |
| Context and external state | Typed scoped providers, nearest-provider lookup, independent roots, external-store snapshot/subscription API | Provider replacement/disposal; tearing prevention; subscription change during commit; stale callback rejection |
| Refs and lifecycle | Typed node/imperative handles and explicit post-layout effects | No access to disposed nodes; focus after commit; cleanup order; temporary hiding distinct from disposal |
| Async composition | Resources, nested loading/error boundaries, retries, lazy feature loading, stale-while-refresh policy | Out-of-order completion, cancellation, nested failures, retained content during refresh, no duplicate work or infinite retry |
| Responsive scheduling | Urgent input, deferred derived work, interruptible/background jobs and coherent commits | Typing during large filter/layout; cancellation; starvation bounds; no partially committed visuals or semantics |
| Errors | Recoverable subtree errors, diagnostics and explicit fatal-host recovery | Errors during setup/effect/task/render; scoped retry does not duplicate subscriptions; secrets redacted; panic/abort limits documented |
| Forms and mutations | Controlled/uncontrolled state, async validation, submission status, optimistic updates and rollback | Concurrent edits/submissions; stale responses; rollback preserves newer edits; password privacy; file validation |
| Routing and data | Nested layouts/routes, typed params, loaders, redirects, guards, history, cache invalidation and mutation policies | Deep links, back/forward, cancellation, offline/reconnect, auth expiry, scroll/focus restoration, per-user cache isolation |
| Overlays and events | Portals/overlay roots, logical event ancestry, capture/bubble/default actions, focus scopes | Nested modal/menu/popover, stacking/clipping, capture cancellation, focus restore, multi-window ownership |
| Styles and motion | Typed flex/grid/constraints, themes, responsive styles, transforms, transitions and animation | Layout/paint/hit-test agreement; DPI; reduced motion; interrupted animation; frame pacing; bidirectional layout |
| Text and internationalization | Shaping, bidi, grapheme editing/selection, font fallback, locale formatting, translated messages | Persian/English and CJK IME; emoji clusters; native caret candidate position; copy/paste; undo/redo; large text |
| Accessibility | One semantic model with browser and native adapters | Human keyboard-only and screen-reader completion; live regions; roles/states/relationships; virtual collection navigation; high contrast |
| Browser expectations | Links, selection, find, print, zoom, autofill, clipboard, drag/drop, files, history, metadata | Browser shortcuts and password managers; system permission denial; selectable text fidelity; printable output; no hidden inaccessible canvas controls |
| Native expectations | Windows/macOS/Linux adapters, menus/shortcuts, multiple windows, dialogs, clipboard, drag/drop, storage and app lifecycle | Clean install, suspend/resume, window close, DPI/monitor changes, crash recovery, platform permissions and package updates |
| Server and content workflows | Typed server data/actions, auth boundary, caching, metadata and indexable content strategy | Request isolation, authorization on every action, CSRF handling where relevant, streaming/cancel, progressive loading and search-visible content |
| Ecosystem and interoperability | Public widget/library extension contracts, charts/tables/editor integration, platform capabilities, incremental migration | Independent library built without private APIs; dependency/version conflicts; safe scoped interop; documented limits on DOM-only libraries |
| Tooling | Fast edit/build/reload, debugger/profiler, component inspection, formatting, completions, test utilities, source maps | Clean checkout; real editor errors; actionable macro/Wasm stack locations; state-preserving update with explicit reset fallback |

Do not add React hook names merely to check boxes. Define equivalent Rust contracts. Context and external-store semantics are particularly important for composition; loading boundaries and scheduling require an explicit runtime design, not a spinner wrapped around a fetch call.

## 3A. Full React-in-the-browser coverage on canvas

### Coverage inventory: no silent omissions

M0 must enumerate the entire pinned stable React and React DOM public reference, including hooks, components, client/server/static APIs, resource hints, form actions, metadata, DOM component categories and events. Record experimental/canary and legacy/deprecated APIs separately; they do not silently expand the stable compatibility target. Audit the inventory again before release. This is behavioral coverage in Rust, not binary/API compatibility with React.

For every entry record: React behavior and source URL; RustX equivalent; core/renderer/host responsibility; canvas implementation; unavoidable browser bridge; example and regression test; real browser results; limitations. Allowed coverage classifications are verified equivalent, implemented unverified, missing, blocked, or legacy migration path. An API whose mechanism is DOM-specific still needs an outcome mapping. Marking it “not applicable because canvas” is insufficient. Arbitrary DOM mutation and third-party DOM-only components require a migration or isolated interoperability contract, not a false claim of drop-in compatibility.

Required inventory groups include:

| React/browser area | RustX canvas implementation and acceptance |
| --- | --- |
| State, reducers, memo/callback, effects, context, refs, IDs and external stores | Typed Rust equivalents with stable identity, cleanup/effect phases, hydration-safe IDs, imperative handles and coherent snapshots; include custom abstractions and diagnostic labels |
| Transitions, deferred values, Suspense, lazy loading, resource reads and optimistic/action state | Nested boundaries, pending UI, ordered mutations, rollback, cancellation, error recovery and actual deferred/code-loading behavior; input remains responsive |
| Component lifecycle, fragments, keys, error boundaries, profiling and development checks | Mount-once equivalents, explicit reset/hide/resume contracts, deterministic diagnostics and profiling; audit all stable built-in components, including newer activity/transition APIs at the pinned version |
| Roots and portals | Multiple isolated canvas roots, updates/unmount, overlay scene layers, logical event propagation and focus ownership; no visible DOM portal widgets |
| Client takeover and server/static rendering | Server-provided data/content/state, streaming and deterministic client activation; mismatch diagnostics, preserved pre-activation input where promised, bounded replay and recovery; do not claim React HTML hydration from painting a canvas |
| Forms and every control category | Canvas input/textarea/select/options/check/radio/range/date/time/color/file and other stable control outcomes; constraints, labels, reset/submit, autofill and form action/status behavior; native browser pickers via declared bridges |
| Document/content elements | Headings, links, lists, tables, disclosure, dialogs, progress, inline text, images/vector graphics and rich content; equivalent semantics, selection, wrapping and interaction in the Rust scene |
| Events and styling | Full event-family audit: pointer/mouse/touch/keyboard/focus/composition/input/clipboard/drag/drop/scroll/media/load/error; CSS-derived layout/typography/transform/animation outcomes through typed Rust styles, with explicit property gaps |
| Media and embedded content | Audio/video controls, captions, playback, fullscreen/PiP and user-activation rules; draw supported video frames to canvas, retain necessary media service elements; external iframe/custom-element/DRM content requires an explicit capability exception and real tests |
| Metadata and resource delivery | Title/meta/link, fonts/images, preload/preconnect/module loading, streaming, caching and failures; bounded head/resource adapters, no DOM stylesheet dependency for normal visible UI |
| Server workflows | Server data/components-equivalent composition and actions with serialization/auth/cache/isolation contracts; account for browser loading/error/retry and progressive delivery, without promising React wire compatibility |

The initial mapping is guided by the official [hooks inventory](https://react.dev/reference/react/hooks), [client APIs](https://react.dev/reference/react-dom/client), [portals](https://react.dev/reference/react-dom/createPortal), [transitions](https://react.dev/reference/react/useTransition) and [action state](https://react.dev/reference/react/useActionState), plus the references in section 7. M0 must expand this into an exhaustive versioned inventory rather than treating this table as exhaustive.

### DOM minimization contract

The live RustX UI is canvas-rendered. Rust owns visible widgets, layout, typography, selection/caret, hit testing, overlays, scrolling and animation. Do not introduce a parallel HTML/CSS widget renderer to satisfy browser parity. Keep the DOM boundary in host adapters, out of application/component APIs.

Maintain a DOM exception register with element/API, capability, owner/lifetime, cardinality, updates, alternatives investigated and browser/assistive-technology evidence. The permitted categories are:

1. Minimal bootstrap document and canvas surface(s).
2. Accessibility semantics and platform focus/input/composition/autofill bridges. They expose the Rust model and follow Rust-computed geometry; they do not compute visible widget layout. Do not hide semantic content from assistive technology merely to reduce node counts.
3. Browser-managed capabilities such as media decoding, file pickers, fullscreen, clipboard/drag operations and required metadata/resources. Some need DOM objects; other Web APIs do not. Inventory actual dependencies separately from JavaScript calls.
4. Generated indexable/printable/progressive-fallback content derived from the same content model. It is a capability projection, not a second source of application state or an alternative live widget engine.
5. Explicitly isolated interoperability for external embeds that cannot be rendered into canvas because of origin, security or browser constraints. Mark these exceptions visibly in the coverage ledger; they cannot be counted as DOM-free canvas implementation.

Default target: zero DOM elements used to render ordinary visible application controls. Minimize bridge nodes and mutations subject to complete functionality. Set workload-specific budgets in M0; semantic tree size may scale with accessible content and focus requirements, so a universal constant-node promise is inappropriate. Prefer a small reusable active editing bridge where it works, but prove autofill, password-manager, IME and AT behavior before pooling controls.

Instrument total DOM nodes by category, active listeners, mutations per update/frame, bridge allocations, disposal and synchronization time. Check that route churn and root removal release them, and that decorative scene changes do not rebuild semantic/input trees. Browser CSS may place the canvas/bridge; it must not determine visible component layout. No per-widget DOM measurement as a hidden layout engine. Benchmark the complete accessible build, not a stripped version with bridges disabled.

### Browser capability and conformance track

Add a fourth reference application: content-rich routes with media, editable forms, server-loaded data, metadata, printable output and offline navigation. Exercise deep links, reload, history, multi-tab storage events, cookies/auth/CORS/credentials, cache isolation, downloads/uploads, file permissions, WebSocket reconnect, service-worker update/rollback, CSP and secure-context restrictions. Include IndexedDB migrations/quota failures and privacy/permission denial.

Test user-visible browser contracts: Ctrl/Cmd+F, native text selection/copy, link context menus/open-in-new-tab, zoom, pinch/visual viewport, printing, autofill/password managers, spellcheck, browser translation where applicable, reduced motion/high contrast, touch/pen, mobile keyboard resize, IME candidate placement, focus restoration and accessibility navigation. An application search field does not by itself satisfy native find-in-page; a text export does not establish browser printing parity. Where browser-owned behavior cannot be reproduced on canvas, retain the smallest necessary projection/bridge and keep any remaining limitation blocking the relevant parity row.

Require real Chrome/Edge, Firefox and branded Safari task evidence, plus physical Android Chrome and iOS Safari for full browser coverage. Freeze supported versions in M0 and refresh the matrix before release. Chromium/WebKit automation covers regression tests, not every browser or physical-device task. Require CPU fallback with GPU unavailable, actual GPU recovery, hidden-tab/resume, context loss, memory pressure and slow/failed Wasm/font/module startup. Record capabilities per target and usable fallback behavior; browser restrictions are not permission to fake support.

No JavaScript/no Wasm delivery is a separately documented progressive fallback, not an executable canvas application. Require meaningful server-visible content and forms/navigation where promised, with activation failure recovery. Server content, semantics and live canvas must agree; prevent duplicate screen-reader announcements and conflicting focus during takeover.

### Full browser release gate

Every stable inventory entry must have a verified outcome or an explicit legacy migration path with its scope stated. Missing/blocked required browser outcomes prevent the full-coverage label. All reference apps must run as canvas applications with the permitted bridges, pass user and accessibility tasks on the declared browser matrix, satisfy the DOM budget and survive deployment/lifecycle failures. Native success, Node Wasm tests and benchmark wins cannot substitute for this gate. Browser compatibility exceptions must remain prominent even after a scoped release.

## 4. Milestones and coding order

### M0 — Freeze scope, contracts and comparative baselines

Deliver `docs/REACT_PARITY_MATRIX.md` and a versioned RP task ledger after auditing the matrix above. Every row needs source links, target-specific status and an executable example or a precise missing contract. Define Windows as the first required native acceptance platform; retain macOS as an early development target and add Linux explicitly. Native mobile is a separately gated expansion; physical mobile browsers are required by the full browser coverage gate.

Create equivalent React + TSX and RustX reference applications before performance optimization: a form-driven admin app, a virtual data grid, and a drawing/diagram editor. Pin React, build tooling, libraries, datasets and algorithms. Specify matched functionality and accessibility. Record cold build/startup, edit-to-visible time, transfer/package size, memory, frame/input latency, idle work and manual user tasks. No superiority claim yet.

Exit: auditable gap register, target/hardware profiles, reproducible baseline commands and prioritized first tasks. Dependencies: none.

### M1 — Reinforce ownership and reactive correctness

Primary paths: `crates/rustx-core/src`, `crates/rustx-macros`, `crates/rustx/tests`, engine commit boundaries.

1. Specify owner/node/generation identity, disposal, dependency tracking, transactions and effect phases in an ADR.
2. Add adversarial/property tests for mount/update/reorder/remove/retry sequences, dynamic dependencies, cleanup, stale handles and async completion races. Use a small deterministic reference model where useful.
3. Audit context, external stores, reducers/selectors and refs; fill actual gaps with typed APIs and compile-fail tests.
4. Document single-thread versus cross-thread access. Keep UI mutation owned by its executor; bridge background work through bounded typed messages.
5. Measure allocations and invalidation. Optimize only after correctness tests expose the invariant to preserve.

Exit: deterministic commits and cleanup under randomized sequences; no observable partial transaction; bounded work/resource policies; existing RX lifecycle regressions pass. Depends on M0.

### M2 — Establish native and browser platform contracts together

Primary paths: `crates/rustx-engine/src/host`, accessibility/editing modules, browser adapters and native packaging scripts.

Define a shared capability interface for windows/surfaces, input, semantic actions, clipboard/files, timers/networking and lifecycle. Implement real adapters without making core widgets depend on DOM objects. Specify platform-specific differences rather than pretending every capability exists everywhere.

Prioritize Windows keyboard/focus, text input and AccessKit integration alongside macOS. Run an editable multilingual form with a modal and API data on both platforms. Exercise real IME composition/cancel/commit, screen-reader form completion, DPI change, sleep/resume, window recreation and CPU fallback. Add multi-window state isolation and resource cleanup before declaring desktop readiness. Test packages on machines without Rust or a browser runtime. Track Linux separately.

Run the browser contract track in section 3A concurrently: real Chromium, Firefox and Safari, browser input/semantics, lifecycle and DOM accounting. Native progress cannot substitute for this track.

Exit: actual native and browser end-to-end application and accessibility evidence, clean native install/uninstall and lifecycle tasks, plus browser capability and DOM inventories. M1 owns shared invariants; adapter work can start during M1. Native failures must influence public API design before stabilization.

### M3 — Async boundaries and scheduling

Primary paths: core resources/services/router/errors and host task queues. Depends on M1; validate contracts against M2 adapters.

Specify loading/error boundary ownership, refresh retention, retry/reset, cancellation and resource sharing. Add lazy module/asset loading with honest Wasm linkage constraints: route-level data laziness alone is not executable code splitting. Introduce urgent and deferred work with bounded queues, starvation protection and commit atomicity. Do not yield halfway through externally observable mutation. Add optimistic mutation transactions with rollback scoped to operation identity.

Exit: nested async route demo remains editable during expensive filtering; races/disposal/retry/cancel are reproducible on native and browser; measured latency and resource bounds pass the frozen workload criteria.

### M4 — Complete practical application primitives

Depends on M1–M3. Work in small vertical slices spanning core, engine, native/browser adapters, tests and public example.

Prioritize nested routing/loaders and forms, then overlays/menus, table/tree virtualization, drag/drop, animation, richer text and localization. Keep logical navigation order independent of paint order. Define extension contracts for custom widgets, layout measurement, semantics and renderer assets. Build a themeable accessible component library using public APIs; internal demos must consume the same library users can consume.

Exit: the admin app and data grid complete the same scripted and human tasks as their React references, including failures, keyboard-only navigation and screen readers. Independent developers can implement a widget without modifying framework internals.

### M5 — Developer adoption and ecosystem

Depends on M1 public API stability; can proceed alongside M3–M4 with explicit provisional APIs.

Improve diagnostics at `.rustx` source locations, completions/generic props, go-to-definition, debugging and profiler explanations. Measure incremental edit-to-visible latency and implement safe state-preserving updates where feasible, with visible reset reasons otherwise. Provide component/service test harnesses and deterministic time/network fixtures; do not confuse those with target evidence.

Ship a React-to-RustX guide mapping state, effects, context, refs, boundaries, forms and routing to real runnable examples. Implement one incremental migration example: an existing React app mounting a RustX region with a typed message boundary and cleanup; document CSS/DOM and accessibility integration limits. Add versioned package/extension APIs, compatibility policy, migration notes and independent library examples. Generated apps must build outside this repository on clean machines.

Exit: developers unfamiliar with RustX complete installation, port a bounded React feature and debug an intentional error using published docs. Record assistance needed, time and blockers. Do not describe all npm libraries as portable.

### M6 — Prove a useful advantage and ship application parity

Depends on M2–M5 and the existing acceptance gates relevant to supported targets. Optimize the matched reference workloads against the protocol below. Resolve dependency/notice findings, fuzz exposed parsers/protocols, review unsafe boundaries, and test update/rollback/distribution. Preserve a complete runnable source checkout and reproducible artifacts.

Exit: all declared application-parity rows verified on supported platforms; at least one preregistered comparative advantage; no hidden essential functionality/accessibility omissions; compatibility and maintenance policy published. A narrow benchmark victory alone does not constitute parity.

### M7 — Complete full browser parity and DOM-free platform expansion

Browser/content implementation begins in M2–M4; M7 integrates and closes the remaining full-coverage gates, rather than postponing web delivery research until native completion. Server rendering, indexable content, document semantics and progressive delivery are important to broad React replacement even if native apps are the first market. Write an ADR choosing a browser delivery strategy: semantic/static HTML projection or a content companion generated from the same application content. An app-only scope cannot close the full browser parity gate. A semantic HTML export may support web discovery while interactive/native layout remains Rust-owned. Verify actual crawler-visible output, navigation, metadata, sharing and failure behavior; a canvas screenshot is insufficient.

Typed server actions need explicit authorization, serialization, request isolation, caching and deployment contracts. Do not promise React Server Component wire compatibility. Native mobile requires actual Android/iOS lifecycle, keyboards, accessibility, permissions and store/package validation. A standalone DOM-free host must replace platform capabilities deliberately; retain the browser bridge for browser users. Full browser parity remains open until every stable React/React DOM inventory row and required browser/content/ecosystem outcome has a verified equivalent. Unsupported or blocked behavior stays visible and prevents that claim; it cannot be silently marked out of scope to finish the roadmap.

## 5. How RustX can beat React fairly

The first hypothesis is a shared native/browser implementation that handles dense, highly interactive UI with lower resource use and predictable updates. Native deployment without an embedded browser is an architectural capability; it does not by itself prove lower memory or better UX.

Freeze the benchmark protocol before tuning. Proposed comparative gates below must be ratified in M0 and kept separate from existing absolute RX budgets:

- **Native product advantage:** the same application behavior and layout/text code produces a standalone desktop package without a browser engine. Compare to an optimized React desktop wrapper for identical tasks; measure installed size, cold startup and total process-tree memory. Also compare a suitable React Native/native solution where available before making broad native claims.
- **Resource advantage candidate:** at least 25% lower steady-state total process-tree memory on two agreed reference applications, with equivalent data, functionality and accessibility.
- **Responsiveness advantage candidate:** at least 20% lower p95 input-to-present latency on two agreed dense-interaction workloads, meeting the existing absolute frame/input limits.
- **Developer advantage candidate:** measure install-to-first-app, edit-to-visible and port/debug tasks. Count compiler diagnostics and safety only through reproducible defect examples; Rust's type system alone is not proof of better productivity.

For the final browser replacement claim, at least one comparative advantage must be demonstrated browser-to-browser; a native packaging win alone does not qualify.

Ship a performance claim only when at least one comparative candidate passes, the full functional scope passes, and material regressions are reported. For other critical dimensions set regression tolerances in M0; do not trade away input, accessibility or severe cold-start regressions to win memory. It is acceptable to discover that a hypothesis fails and choose another product advantage through a new, disclosed experiment.

Use production builds on the same hardware, OS, renderer class, viewport/DPR, datasets and network conditions. Permit normal React memoization/compiler optimizations, virtualization and suitable mature chart/canvas libraries. A 10,000-node unvirtualized React DOM example is not a fair baseline for a culled RustX canvas. Browser-vs-browser and native-vs-native-wrapper measurements must be separately labeled.

Use at least 20 independent startup runs and 5 steady-state sessions with raw samples, randomized run order, warmup and recorded thermal/power conditions. Report medians, p95, spread/confidence intervals, total memory accounting, CPU/GPU usage and energy where measurable. Require the claimed improvement to exceed observed uncertainty. Measure physical presentation with an appropriate trace/device method; rAF intervals and synthetic input are proxies and must be labeled. Keep the 16.7 ms GPU gate intact. Include low-end hardware and real native targets before generalized claims.

## 6. Agent execution protocol

1. Read the current handoff and inspect Git status. Preserve unrelated changes. Select one ready RP task on the dependency path; do not implement the entire roadmap in a single change.
2. Reproduce current behavior and identify the smallest missing contract. Draft an ADR when changing lifecycle, scheduling, threading, serialization or public compatibility.
3. Deliver a runnable vertical slice with a public example and focused regression checks. Separate correctness fixes from benchmark optimization.
4. Run formatting and appropriate workspace/target tests. Rebuild CLI when embedded SDK/docs change; rebuild Wasm after source/host changes. Follow NEXT_AGENT.md for commands, but scale full platform replay to the actual change and claims.
5. Record exact commands, source/artifact hashes, hardware/tool versions, raw results and remaining gaps under `docs/evidence/react-parity/<task-id>/`. Update the RP ledger and relevant RX rows only to the strength of actual evidence.
6. Update STATUS.md and NEXT_AGENT.md with the completed contract and the next concrete task. Explain public API changes and migration impact. Return the full runnable project; never rely on ignored artifacts copied from a developer's machine.
7. Commit/push/package according to the user's active authorization. This roadmap does not authorize public hosting, new spending or replacing historical evidence. Do not infer implementation success from this document.

### First work package for the next coding agent

Create `RP-001: native-first capability audit and baseline contract` in M0. Inspect existing APIs and tests for every coverage row, then author REACT_PARITY_MATRIX.md with explicit source links and unverified target gaps. Inventory the owner/runtime/task boundaries and propose the first M1 correctness task. Include an exhaustive stable React/React DOM API inventory, browser capability inventory, DOM exception register and a fourth content/media/server-delivery reference app. Specify the four matched React reference workloads and lock their acceptance criteria before adding dependencies or measuring winners. Keep existing applications runnable. This package produces the actionable backlog; it does not certify parity or start a speculative runtime rewrite.

## 7. Primary references

Checked 16 November 2026. Recheck the official stable API inventory and pin the benchmark React/tool versions when M0 starts. The following establish comparison areas, not a requirement to duplicate implementation details:

- [React built-in components](https://react.dev/reference/react/components): composition boundaries and coordination features.
- [Context](https://react.dev/reference/react/createContext): scoped shared values.
- [External stores](https://react.dev/reference/react/useSyncExternalStore): subscriptions and coherent snapshots.
- [Suspense](https://react.dev/reference/react/Suspense): nested loading/reveal behavior.
- [Resource reads](https://react.dev/reference/react/use): async resources and context.
- [React DOM components](https://react.dev/reference/react-dom/components) and [common element behavior](https://react.dev/reference/react-dom/components/common): browser behavior that canvas applications must account for.
- [Server React DOM APIs](https://react.dev/reference/react-dom/server): HTML/server delivery responsibilities for broad web replacement.

Routing, query caches, component libraries, native packaging and most developer workflows require ecosystem comparisons in addition to React core. Audit those independently in M0; do not attribute every application feature to React itself.

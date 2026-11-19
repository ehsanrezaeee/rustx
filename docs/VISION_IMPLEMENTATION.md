# RustX vision implementation guide

[Current baseline](#1-current-baseline) · [Target and architecture](#2-target-and-architecture) · [Implementation stages](#3-implementation-stages) · [Agent execution and recovery](#4-agent-execution-and-recovery)

## 1. Current baseline

This inventory describes the source at `7cbb847`, the stability commit preceding this guide. **Implemented** means an API or host path exists in the checkout. **Verified** refers to a recorded run on the stated target. The [roadmap](ROADMAP.md) carries the current evidence and open requirements.

| Area | Implemented now | Verified behavior and remaining scope |
| --- | --- | --- |
| Authoring and identity | `rustx!` markup in `.rs` and `.rustx` files, `#[component]`, typed props and defaults, expressions, children, runtime fragments, keyed lists and conditional views | The runtime lab exercises reorder, unmount, and remount. Extend authoring where the reference applications expose a concrete need in V00 and V02. |
| Reactive runtime | Mount-once setup, signals, memos, effects and cleanup, scoped context, node refs, external-store snapshots, and explicit recoverable error boundaries | The stability pass covered signal reentrancy, memo initialization, broad subscriber updates, and teardown. Async boundaries, action state, and scheduling priorities still need specification and delivery. |
| Rendering | Shared Rust layout and scene, typed flex and grid, responsive styles and themes, text shaping, canvas drawings, images, transforms, hit testing, virtual lists, and CPU and GPU renderer paths | Windows and Edge CPU smoke runs are recorded. GPU recovery, mixed DPI, text fidelity across hosts, and comparable performance measurements are open. |
| UI and editing | Buttons, inputs, lists, tables, select and combobox controls, overlays, dialogs, forms and validation, keyboard, pointer, wheel and touch event paths, selection, and editing | Windows CPU form passed posted forward Tab and stacked dialog restoration with UI Automation. Edge verified forward and reverse Tab, editing, field kind changes, and stacked dialogs. Physical touch, native IME, and human screen-reader results are still required. |
| Host and accessibility | The browser boots Wasm, draws to a canvas, and projects Rust semantics and editing bridges into the DOM. Native uses winit, shared Rust rendering, and an AccessKit projection. | Windows has current source-built CPU evidence. Earlier macOS startup is recorded. Linux has an X11 dependency path and some platform service code. Linux has no native job in the current workflow. |
| Application services | Router with history and hash modes, typed HTTP and cancellation, streams, WebSockets, timers, scoped tasks, asset loading, clipboard, storage, file dialogs, and printing where supported | Edge verified the sample HTTP route. A Windows native loopback check verified pull streaming and socket echo. Native service availability varies. The current task queue and demo server are foundations for the application-level Rust concurrency and HTTP server work. |
| Tooling and delivery | CLI scaffold, check, format, build, dev reload and serve, editor support, local compiler playground, service worker, inspector, and source CI | The demo builds from the checkout. Current source-check workflow defines Windows and macOS native CPU jobs and a browser build and HTTP smoke job. Edge verified live reload and offline reload. Independent onboarding, expanded platform CI, and release acceptance remain open. |

Use `examples/lab/src/app.rustx` and the Applications, People, Engine, Runtime Lab, and Store routes as starting points. [Features and demo routes](FEATURES.md)

## 2. Target and architecture

RustX is a Rust UI framework with TSX-like tag and expression syntax for writing typed components. RustX defines its own component lifetime, runtime, styling, rendering, and features. App authors express UI and business logic in Rust and RustX markup. Choose features by building applications that take advantage of the canvas and native desktop, such as an interactive scene editor, a large data workspace, and a low-latency telemetry dashboard.

**Web goal.** Match or beat well-optimized, framework-free JavaScript rendering the same UI to a browser canvas. Rust and Wasm own style resolution, layout, text, editing, hit testing, and the scene. A generated host shell loads Wasm and connects it to platform facilities, including JavaScript networking through Fetch, streams, and WebSockets. The shell also carries browser input, accessibility, IME, and lifecycle events. Application code uses typed Rust interfaces. Keep browser origin and permission behavior visible through typed capabilities and errors.

**Native goal.** Build exceptionally responsive desktop applications in Rust for Windows, macOS, and Linux, with a substantial performance lead over equivalent browser implementations. A RustX executable opens an OS window, renders through native GPU or CPU backends, and uses OS input, accessibility, and presentation APIs. Applications can use Rust networking, threads, async runtimes, persistence, and optional HTTP servers. The native UI runs without a webview, browser runtime, HTML, CSS, or JavaScript.

Share app source, component identity, semantics, and the observable layout and editing contracts. Give each host room to optimize scene updates, data movement, input scheduling, and presentation independently. Guide API design with working applications and guide optimizations with profiles and end-to-end measurements. V00 defines the workloads and V08 defines what counts as achieving the speed targets. The current baseline contains functional results. The speed targets still need comparative measurements.

| Layer | Ownership |
| --- | --- |
| Public API and authoring | `crates/rustx` and `crates/rustx-macros` own the public vocabulary, component expansion, and diagnostics. |
| Shared runtime | `crates/rustx-core` owns scope lifetime, signals, state, events, editing models, services, and typed layout inputs. Keep UI-owned values on the UI thread. |
| Rendering and native host | `crates/rustx-engine` owns layout execution, shaping, scene generation, native window and input handling, accessibility, and native presentation. |
| Browser host | `crates/rustx-web` and `web/` own Wasm exports and thin browser adapters. Keep the lab and scaffold on the same supported host contract. |
| Native application services | Extend the typed core service contract with native implementations behind native-only dependencies. Share domain and protocol types with a separate Rust server process or an opt-in embedded server. |
| Delivery and reference app | `crates/rustx-cli`, `examples/lab`, and `.github/workflows/source-checks.yml` own source builds, tooling, demos, and verification. |

**Invariants**

- Component setup runs once per mount. Reactive bindings and effects update independently. Stable keys preserve state. Unmount releases scopes, listeners, timers, workers, requests, and host resources owned by the subtree.
- Commits publish coherent snapshots. Off-thread producers deliver owned messages to the UI thread. Browser and native hosts share layout and semantic contracts and can choose different rendering and presentation strategies.
- Host failures produce explicit outcomes such as cancelled, denied, unavailable, timeout, or failed. Preserve valid app state when recovery is possible. A fatal Wasm trap requires a new host session.
- Keep the current demo runnable from source throughout the work. Deliver feature slices through the existing demo or independent local scaffolds. Keep standalone validation assets, outputs, and app archives outside Git. Python remains outside the build and runtime.
- Feature claims require direct evidence for the named platform, hardware or browser, renderer, and interaction method. A unit check and a frame smoke check are valuable evidence with different scopes.

## 3. Implementation stages

The `Vxx` IDs belong to this guide. Keep the existing `RP-xxx` milestones in the roadmap. Each stage can ship in small commits. `V00` establishes the inventory, `V01` is the first engineering slice, and `V10` closes release acceptance. Where a stage has a prerequisite, it can use a verified subset of that prerequisite and carry the rest as an explicit dependency.

| Work | Stage and dependency |
| --- | --- |
| Native Windows, macOS, and Linux baseline | V01 starts with RP-008. |
| Component and visual API | V02 precedes the reusable controls in V03. |
| Text, accessibility, and host lifecycle | V04 starts from V01's replay and uses the controls as they land. |
| State, async UI, and I/O | V05 can start with fake hosts. V06 verifies real host transport. V07 builds application services on the typed transport and state contracts. |
| Performance and startup | V00 defines workloads. V08 measures and optimizes them on web and native. V09 uses the same evidence to improve web startup and large-scene delivery. |
| Source delivery and supported targets | V10 closes the required gates from all stages. |

### V00 · Define the reference workloads and performance contract

**Deliver.** Turn RP-001 into a list of RustX application scenarios and RP-005 into a reproducible benchmark plan. For each workload define what users can do, expected visual and accessibility behavior, dataset and event sequence, target hosts, required features, and acceptance evidence. Name an owner stage for each missing feature. Choose primary performance metrics and thresholds before collecting comparison results. Use the Engine, Applications, and People demos as starting points.

**Technical view.** Keep reference data deterministic. Specify the scene size, viewport, visible content, update pattern, and renderer for every run. Track rendering-only workloads separately from interactive ones that include layout, text, input, and semantics. Compare RustX Wasm to plain JavaScript drawing the same output into a canvas. Replay the same workloads with the native executable and the browser on the same desktop machine. Keep standalone benchmark implementations and raw results in ignored validation output.

**Example.** Start with these three workloads and one short startup scenario.

| Workload | Behavior to preserve | What to measure |
| --- | --- | --- |
| Scene editing | Pan and zoom a scene with 2,000 shapes, drag a selection, and change a small subset of shapes | CPU frame work, presentation cadence, and pointer-to-visible-update latency |
| Data workspace | Filter and scroll 10,000 rows, edit a visible row, and preserve focus when results reorder | Key-to-visible-update latency, missed frame deadlines, and memory after repeated use |
| Live telemetry | Update several charts at a fixed rate while the user pans, opens controls, and changes a query | Sustainable update throughput, frame-time tails, input latency under load, and idle work after the feed stops |
| Open the workspace | Start at the same usable view with local fixed data, then repeat with a warm session | Cold and warm time to the first usable frame, peak memory, and web transfer size |

**Exit.** The roadmap lists the reference workloads, current demo entry points, and the first missing feature or measurement for each. An agent can reproduce a workload using its fixed data, interaction sequence, quality checks, and performance metrics.

### V01 · Stabilize the existing interaction slice and bring up Linux native

**Deliver.** Continue RP-008 from the Windows CPU form evidence in the roadmap. Replay the multi-control form and off-screen focus reveal on macOS. Extend the same scenario to a Linux CPU build. Bring up a declared Linux distribution and X11 session first using the existing configuration. Validate Wayland separately before advertising it and document any dependency or service gap. Add a Linux source-build and virtual-display CPU smoke job after a local or runner repro. Keep a desktop interaction run separate from the CI smoke. Make progress on X11 and shared code when macOS hardware is unavailable.

**Technical view.** Start in `examples/lab/src/applications.rustx`, `crates/rustx-engine/src/host.rs`, and `crates/rustx-engine/src/accessibility.rs`. Route host events through the shared engine. Keep keyboard modifiers, logical units, focus, and redraw behavior consistent. Record platform services that currently rely on OS helper programs and verify capability reporting when helpers are missing.

**Example.** Build with `cargo run -p rustx-cli --locked -- build --target native --out dist-native` on each desktop. In the form, move forward and backward between an editor, a disabled control, a hidden subtree, and a dialog opener. Verify that an off-screen focused field scrolls into the viewport. Open two dialogs and close them in reverse order. Verify that every Escape returns focus to its own opener. Record executable revision, OS and display server, CPU renderer, input method, and focus readback.

**Exit.** Windows, macOS, and at least one declared Linux desktop produce runnable native builds with repeatable interaction evidence. CI has build and CPU smoke coverage for each declared target. The roadmap retains any outstanding compositor, mixed-DPI, multi-window, pointer capture, clipboard, file, or hardware check as an individual item.

### V02 · Complete typed component authoring and identity

**Deliver.** Cover typed required and optional props, supported generic component forms, reusable children and named slots, fragments, conditional expressions, keyed collections, component composition, typed events, stable IDs, and scoped refs. Specify prop updates for mounted components, including the supported signal or store mechanism for live props. Add precise compile errors for invalid markup, duplicate keys or props where determinable, invalid handlers, and unsupported forms. Provide stable focus and measurement handles that fail safely after disposal.

**Technical view.** Extend `rustx-macros` and the `rustx` facade using Rust types, `IntoView`, scope ownership, and reconciliation. Keep tag and expression syntax familiar to writers of TSX, with Rust expression and ownership rules. Separate immutable setup inputs from live signal reads. Define overlay logical ownership independently of scene placement, semantic parentage, and hit order. Use macro expansion and runtime regressions to keep the markup easy to reason about.

**Example.** A `TaskCard` accepts a required task ID, a reactive task title, and footer content. Render three cards keyed by task ID. Edit the middle card, reorder the list, and change its title from the parent. Its text selection and draft stay with its key, the live title updates, and the setup counter stays at one. Delete it and verify its cleanup runs once.

This is the current RustX pattern for the reactive title inside that example.

```rust
use rustx::prelude::*;

#[component]
fn TaskTitle(_cx: Scope, title: Signal<String>) -> View {
    rustx! {
        <Text>{ move || title.get() }</Text>
    }
}
```

**Exit.** Document supported markup and prop semantics with compilable examples. Positive examples compile on native and Wasm. Invalid examples report the relevant RustX source location. Runtime replay proves independent nested state, reorder, parent updates, and scope cleanup.

### V03 · Make layout, controls, and styling complete enough for real applications

**Deliver.** Extend typed style primitives for constraints, responsiveness, themes, state variants, focus indicators, reduced motion, right-to-left layout, overflow, scrolling, and animation. Build controls for the reference workloads around the existing forms, lists, menus, tables, overlays, and split panes. Support accessible names, stable label relationships, selection, and error recovery. Add first-class primitives for canvas workspaces, including viewport transforms, editable drawing objects, and ordinary controls embedded alongside an interactive scene.

**Technical view.** Reuse `style.rs`, `widgets.rs`, `forms.rs`, and the engine layout, text, scene, and control modules. Compare expected Rust layout boxes and semantic relationships alongside screenshots. Keep coordinate conversion and viewport input at the host boundary. Let a transformed drawing surface update independently of surrounding controls while preserving coherent hit testing and selection.

**Example.** Extend the Applications demo into an inventory editor with a responsive sidebar, searchable virtual table, multiline detail form, and confirmation dialog. A narrow canvas stacks the panes. A wide desktop shows them side by side. High contrast and reduced motion change the rendered presentation while preserving the same route, controls, and editor state.

**Exit.** The same RustX view behaves consistently at narrow, wide, and high-DPI sizes. Acceptance covers resizing, nested scrolling, keyboard usage, form reset, disabled controls, and selection in large lists.

### V04 · Deliver native and browser accessibility, text, and host lifecycle

**Deliver.** Complete RP-009 and RP-011 for platform input. Verify IME composition and cancellation, candidate placement, undo and redo, selection, grapheme movement, bidirectional text, shortcuts, clipboard, pointer capture, touch scrolling, focus, and live announcements. Add native window lifecycle and multi-window isolation where specified. Test browser resize, suspend and resume, and actual back/forward cache behavior. Record the browser/device matrix, including Chromium, Firefox, Safari, physical Android, and physical iOS. Include Windows UI Automation and a human screen-reader session, macOS VoiceOver, and Linux AT-SPI with a human screen-reader session on the declared desktop.

**Technical view.** Keep Rust editor data authoritative. Normalize platform ranges at the boundary and preserve grapheme, byte, and UTF-16 conversions. Project roles, names, relationships, actions, and focus through AccessKit or generated browser semantics. Treat browser editing bridges as host adapters. Scope listeners and composition to the active window or host instance. Test mount, hide, suspend, resume, and unmount separately.

**Example.** Enter Japanese text, Arabic text, a combining accent, and an emoji in a multiline form. Move the caret, cancel a composition, undo a committed edit, then open and close a dialog. A screen-reader user can identify the field and error message, operate the dialog, and resume at the prior control. Verify this with the relevant platform input method and assistive technology.

**Exit.** The roadmap reports each input and accessibility result per target and labels every remaining hardware check. A native input session, an accessibility tree inspection, and a human screen-reader run are tracked independently. Unsupported browser capabilities present a meaningful usable fallback.

### V05 · Build scoped data, responsive scheduling, and recoverable actions

**Deliver.** Give applications a clear way to manage state, data loading, live feeds, edits, and background work. Define batched updates, reusable Rust state and domain logic, scope-owned resources, and clear pending, ready, empty, and error states. Keep typing and dragging responsive when a large view update or data task is pending. Support cancellation, change validation, save and retry, and rollback where the workflow benefits from immediate edits. Make resource lifetime and unmount behavior explicit.

**Technical view.** Build on `Runtime`, `Scope`, `TaskQueue`, `ExternalStore`, and the error boundary. Start by measuring how signals, tasks, and commits currently interact. Explore prioritized work, bounded incremental updates, and coalesced producer messages where profiles show contention. Keep committed layout, hit testing, and semantics coherent. Use request generations and cancellation to preserve the latest accepted result. Keep inactive workspaces lightweight while preserving edit state where the product flow requires it.

**Example.** In a live telemetry workspace, drag a chart selection while updates arrive. The pointer and selection remain responsive. The chart incorporates the freshest available samples with a documented history policy. Changing the query replaces the prior subscription and stale samples cannot alter the new chart. A transient failure preserves the current view and offers retry.

**Exit.** Use deterministic fake hosts to replay competing updates, cancellation, repeated submits, failed edits, and interrupted work. Replay one real browser and native workflow. Measure input under load on both paths and document scheduling, data loss or coalescing policy, and resource cleanup behavior.

### V06 · Unify networking and typed platform services

**Deliver.** Publish one Rust-facing transport contract for requests, response status and headers, streaming, uploads, sockets, timeouts, cancellation, and backpressure. Keep browser transport on its JS bridge and native transport on Rust. Publish the same style of capability and errors for storage, files, clipboard, printing, timers, and navigation where supported. Support typed configuration for an API base URL, authentication state, and local fixtures. Define platform differences for origin rules, cookies, exposed headers, permission prompts, and streaming support. [WebAssembly host interoperation](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Concepts)

**Technical view.** Inventory `HttpClient` and `ServiceClient` overlaps before changing the public surface. Keep existing callers working during migration. Put portable request policies, cancellation generations, limits, and redacted errors in Rust. Keep platform permissions and browser Fetch, streams, and WebSocket primitives at the boundary. Make native results wake the event loop through a bounded queue. Test typed errors and ensure late results cannot update disposed state.

**Example.** People Explorer loads paged JSON, cancels a query after navigation, reads a deliberately slow stream on demand, and reconnects a socket after a normal disconnect. The browser uses JS host services against the fixture endpoint. The native app calls the same service contract with the Rust adapter. Simulate a timeout and a denied browser permission and show actionable, typed UI errors.

**Exit.** Run the same contract scenarios against a fake transport and real local endpoints from the browser and all declared native platforms. Route both native and web hosts through the supported API while preserving the demo. Record genuine platform differences in the feature guide.

### V07 · Add native Rust concurrency and application HTTP servers

**Deliver.** Ship an optional application service path that supports an in-process native server or independent Rust server binary. Cover worker jobs, cancellation and progress, typed APIs and shared domain types, persistence, health checks, lifecycle management, and graceful shutdown. A normal desktop UI build starts without a listener. Embedded server mode uses an OS-assigned loopback port by default and publishes its endpoint only after a successful bind. External service deployment uses explicit configuration.

**Technical view.** Keep desktop startup and UI state on the event loop. Use native Rust threads or an async runtime selected through a documented dependency decision. Bridge `Send` data back to the UI through a bounded queue and a wakeup, with a defined overflow policy. Supervise background work and stop it on window close or scope cancellation as appropriate. Build application routing on maintained Rust server libraries and shared typed contracts. Give request and body limits, errors, logging, and data migration defined behavior. Keep server-only dependencies outside the Wasm build. The CLI demo server remains a source-demo and fixture tool.

**Example.** A desktop inventory app imports a CSV in the background while the user edits a different item. A Rust service commits valid records and exposes `GET /items` and `GET /health`. Run it first as a native-owned loopback service, then as a separate Rust binary for a browser session using the same Rust request and response types. Cancel an unfinished import and close the desktop window. Verify the listener, tasks, file handles, and database transaction settle as specified.

**Exit.** Three native hosts pass startup, cancellation, concurrent work, shutdown, and restart tests. The browser can consume the independent service through V06 under the configured origin policy. The Wasm dependency graph excludes server-side networking and storage dependencies.

### V08 · Meet the browser and native performance targets

**Deliver.** Complete RP-005 and RP-010 with benchmarkable versions of the V00 workloads, production diagnostics, and renderer recovery. Profile before choosing optimizations. Investigate small-region or retained scene updates, reuse of layout and shaped text, GPU batching, predictable allocation, and fewer Wasm boundary copies or calls. Give native its own opportunities for parallel preparation, compact scene data, direct presentation, and better frame pacing. Preserve text quality, interaction, accessibility, and resource limits on both targets.

**Technical view.** Start with `diagnostics.rs`, `allocation.rs`, the engine scene and cache modules, GPU backends, and `web/host.js`. Run the first benchmark before major rendering changes and repeat it after each substantial change.

- **Browser comparison.** Use a tuned plain JavaScript canvas implementation with normal optimizations such as caching, batching, and culling where the workload allows them. Compare equivalent drawing and quality paths, using Canvas 2D or WebGPU as appropriate. Report CPU and GPU configurations separately. Include app-level rendering and interaction when both implementations provide the same observable behavior. Keep draw-only tests separate.
- **Native comparison.** Run the native Rust executable, the RustX Wasm build, and the comparable JavaScript canvas reference on the same physical desktop. Use the faster browser result for each headline metric. Hold data, scripted input, visible work, logical and physical viewport size, refresh rate, and output quality constant. Measure each supported OS separately. Record renderer, CPU, GPU, driver, browser build, device pixel ratio, power mode, and cache state.
- **Measurement.** Use optimized builds, warm-up runs, repeated trials, and a predeclared tolerance for environmental noise. Record medians and tail percentiles for CPU frame work and input-to-present latency, along with missed frames, first usable frame, idle activity, memory, and sustained throughput where relevant. Include semantics work in full-app runs. Distinguish CPU submission time, GPU execution, and actual display presentation. Use a presentation trace, platform timing, or an external latency measurement for visible-frame claims, and label proxies when direct timing is unavailable.
- **Web acceptance.** Match or beat the JavaScript baseline on primary rendering metrics for the declared workloads within the predeclared measurement tolerance. An inconclusive trial stays open. Report each renderer and workload, including regressions in latency, visual quality, or behavior.
- **Native acceptance.** Use half the faster browser result’s p95 CPU frame work as a proposed starting target for the workloads with substantial rendering work. Also aim for materially lower input-to-present latency or higher sustainable workload throughput at the same responsiveness and visual quality. Set the exact latency or throughput thresholds in V00. At 60 Hz a frame interval is about 16.7 ms. If both apps meet the display limit, measure additional scene or update capacity at the same latency and frame cadence.

**Example.** Replay the 2,000-shape editor scenario in the JavaScript canvas baseline, RustX Wasm, and native RustX. After verifying equivalent visible output and interactions, measure the cost of changing a small selection while panning. If full-scene regeneration dominates, retain stable draw data and update the changed region or batch. Then repeat the same trials. In a separate recovery run, force renderer loss on a supported GPU path and check that the CPU renderer resumes with the selection intact.

**Exit.** Publish the benchmark protocol, hardware and renderer details, per-workload results, and open gaps in the roadmap. Carry missed targets forward as measurable work. Advertise performance leads only for the configurations and workloads that demonstrate them. Record device loss and recovery separately from throughput and latency.

### V09 · Improve web startup and stream large workspaces

**Deliver.** Make deep links and large workspaces useful quickly. Measure the startup path and prioritize the initial view, essential fonts, and input readiness. Investigate staged assets, route prefetch, or versioned Rust-prepared startup data where they improve the measured result. Support progressive loading, offline recovery, and interrupted transfers with predictable UI state.

**Technical view.** Trace the time spent downloading, compiling Wasm, initializing the host, decoding assets, and presenting the first usable frame. Reuse shared Rust domain types across the server and client. If preparation on the server helps, send a bounded stream of versioned public data with stable identifiers and a cold-start fallback. Keep application data and canvas presentation under Rust ownership. Preserve focus and editing state as tiles, figures, or data arrive. Put a measured limit on work and memory spent on prefetch and speculative decoding.

**Example.** Open a large telemetry workspace via a saved link on a cold browser session. Show the controls and latest samples as soon as they are usable. The history stream fills in visible chart ranges first and yields to user input. Interrupt and reconnect the transfer. The latest committed view stays usable while progress resumes from its recorded point.

**Exit.** Verify cold and warm startup, deep links, interrupted streams, version mismatches, slow connections, and offline recovery. Demonstrate the first-usable-frame or large-scene improvement against the recorded baseline with rendering quality and input behavior preserved.

### V10 · Deliver the SDK, continuously updated demo, and release evidence

**Deliver.** Publish a coherent API surface, source install and upgrade path, generated scaffold, editor diagnostics, API examples, and migration notes. Make the same reference app demonstrate typed components, forms, routing, virtual lists, persistence, async actions, native workers and server mode, and browser Wasm transport. Build the latest demo from the selected revision and identify it using the source commit and bundle ID. Keep delivery free of app ZIP archives. Document exact support tiers for native OS versions and Linux display servers, browsers and devices, renderers, services, and accessibility.

**Technical view.** Keep the scaffold and lab aligned on the same public entry points. Use the pinned toolchain and lockfile with representative Windows, macOS, Linux, and browser source checks. Verify local and hosted delivery separately. Review dependencies, licenses, advisories, deployment configuration, documentation, and default resource limits. Use platform tests, user scenarios, and performance results as the release evidence.

**Example.** A new developer clones the repo, runs the current browser demo, creates an independent app with `rustx new`, edits a RustX component, and runs it on Windows, macOS, Linux, and a phone browser. The same inventory form saves through a Rust service, can be navigated by keyboard, and recovers from a service restart with a usable status. The delivery report links each verified workflow to its revision and target.

**Exit.** A fresh-checkout replay succeeds on each supported target, with claim-specific platform evidence. Each required workload has a functional result and a performance result against its published target. The README, features, and roadmap match the demo, support matrix, and measured results.

## 4. Agent execution and recovery

### Start and complete a slice

1. Read `AGENTS.md`, the [roadmap](ROADMAP.md), the [feature guide](FEATURES.md), and the [agent handoff](AGENT_GUIDE.md). Inspect `git status --short --branch`, `git log -5 --oneline`, and relevant diffs. Keep any user changes intact. Identify the exact stage, RP item, current behavior, and acceptance criterion.
2. Establish a baseline from the real entry point. Reproduce the smallest affected use case and record the command, environment, expected behavior, and actual result. Classify host-specific failures before changing shared code.
3. Deliver one complete vertical slice. Reuse existing types, widgets, and adapters. Add an inline regression for a specific failure when existing checks miss it. Put standalone replays, benchmark data, and logs under ignored `.rustx/validation/` or outside the checkout. Update the existing demo when it is the natural entry point.
4. Run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, and `npm run format:check` after installing contributor dependencies. Rebuild the CLI after changing embedded browser assets. Build with `cargo run -p rustx-cli --locked -- build --target web --release` or `cargo run -p rustx-cli --locked -- build --target native --out dist-native` as applicable. Replay the affected behavior through the relevant host using the current build.
5. For each result record the commit, environment, exact command, scenario, pass/fail/blocked state, and evidence location. Put durable summaries and unresolved items in `ROADMAP.md`. Update `FEATURES.md` when public behavior changes. Record the next concrete slice in the agent handoff if the active milestone changed.
6. Inspect `git diff --check` and the staged paths. Commit coherent, verified work when requested. Report platform gaps and any authorized publishing status. Mark the stage complete only when its exit conditions are met.

### Resume after interruption or failure

| Situation | Recovery action |
| --- | --- |
| Agent interrupted mid-edit | Read the last status entry and inspect Git status, staged and unstaged diffs, recent commits, and any still-running build or dev process. Verify which changes are present. Re-run the last named failing or pending command. Resume at the first incomplete acceptance check. |
| Baseline build or CI already fails | Capture the failing invocation, first relevant diagnostic, host/tool versions, and last known good revision. Reproduce on the same platform where possible. Inspect the job's actual build path. Isolate the baseline failure from new work and fix the smallest prerequisite first. Use a separate checkout if a clean baseline is needed and authorized. |
| Feature passes a unit check and fails in the app | Trace the actual app path from input or request through the host, Wasm or native adapter, Rust model, and frame or semantic output. Add a focused regression at the failed boundary. Rebuild the full app and repeat the original interaction. |
| Performance target fails or results vary | Recheck workload equivalence, build flags, device settings, and output quality. Repeat the comparison with the original baseline and record variance. Profile the slow phase, change one likely cause, and repeat the same run. Keep inconclusive results and unresolved regressions open in the roadmap. |
| Dev build or live reload fails | Preserve the last working output. Check the first build diagnostic, bound port, CLI version, served bundle ID, runtime configuration, and service-worker state. Fix the cause, rebuild the current sources, and confirm from the browser that the intended revision loaded. |
| Optional hardware, OS, runner, or permission is unavailable | Mark the exact acceptance check blocked, including the missing prerequisite, last completed checkpoint, and reproduction steps. Continue independent groundwork. Leave the affected claim open until direct evidence is collected. |
| Worker, request, native listener, or browser host fails during replay | Reproduce teardown and restart. Check for stale completions, held ports, open handles, leaked listeners, and partial writes. Stop only test processes owned by that run. Verify the next startup and that visible state follows the documented recovery behavior. |
| An approach breaks compatibility or architecture | Identify the affected callers and behavior. Adapt or revert only the failing slice while preserving user changes and useful evidence. Record the decision if it changes the public contract. Resume from the last good acceptance checkpoint. |

Keep a short checkpoint in the handoff or active roadmap entry before yielding if work is incomplete.

```text
Stage and milestone
Base revision and relevant changes
Last verified command, target, and result
First failing or blocked acceptance check
Evidence location and exact reproduction
Next actionable change or command
```

**First action.** Define the first scene editing and data workspace recordings in V00 and identify the demo routes that exercise them. For platform implementation, continue V01 by replaying the form on macOS and creating the first Linux X11 CPU baseline. Collect the first comparable frame-time samples as soon as the web and native workloads run.

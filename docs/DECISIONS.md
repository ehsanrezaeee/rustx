# Architecture decisions

## ADR-001 — Rust and mount-once components

Rust is the core/application language for integration with the selected layout/text/graphics ecosystem. This is an architectural choice, not a Go/React speed benchmark. Use normal Rust modules with rustx!, not a compiler fork. Plain props are initial configuration; explicit signals carry changes. Key changes remount. Reuse rendering modules from the supplied MIT RustUI project; replace its runtime spine, not its verified rendering work without cause.

## ADR-002 — Explicit host boundary

Rust owns state/layout/text/scene and scoped IDs. JS loads Wasm and connects browser networking/input/semantics. IDs are monotonic and bounded to exact JavaScript integers; malformed numeric IDs are rejected, stale results require a live owner. GET/cancel is the current slice. Version/capability negotiation and broader services are future gates. Browser AbortController cancels transport; native cancellation currently rejects completion while blocking I/O times out.

## ADR-003 — Vello plus CPU adapter

Pin the compatible Taffy/cosmic-text/Vello/wgpu set inherited from supplied rendering work, and isolate tiny-skia/fontdue CPU replay behind an adapter. Direct browser GPU presentation is separate from diagnostic offscreen readback. CPU parity is unfinished; upgrade/replace its adapter without rewriting authoring or scoped state. Software graphics output is not hardware certification.

## ADR-004 — Examples are release gates

Build the homepage/API app early through public RustX APIs. Clearly label current precompiled presets. The final two examples require real API success and real editable Rust compilation. Select a sandboxed compilation architecture in a new ADR before implementing visitor code execution; never add unrestricted native execution to the fixture server.

## ADR-005 — Versioned bounded Stage 1 host contract

Context: the original GET/cancel implementation admitted requests without a handshake and could grow its outbound queue indefinitely. Browser adapter tests also missed the native Fetch receiver requirement.

Options: retain an implicit version, add a broad speculative services ABI, or version the existing finite slice. Choose a one-time typed capability report before mounting, versioned GET/cancel/completion envelopes, 16 live requests, 32 coalesced commands, bounded body/URL/timeout values and a 16-result native channel. Keep wasm-bindgen 0.2.100 as the binding layer. This closes RX-06's Stage 1 contract without claiming Stage 3 networking.

Consequences: hosts must report capabilities and completion versions; old generated bundles must be rebuilt together with adapters. Wrong versions fail before dispatch or leave the valid pending request untouched. Browser Fetch is explicitly called with the global receiver. Contract and rationale are in HOST_PROTOCOL.md. Migration: replace old `init_app()` with the host report argument; pass the protocol version through completion. Rollback requires reverting both adapters and Rust together, never mixing bundles.

## ADR-006 — Preserve browser editing sessions during keyed reorder

Context: detach/reinsert of an active input can discard native focus, selection or composition. Browser-native scrolling of an invisible semantic tree can diverge from Rust canvas geometry. Literal/reactive Text wrappers also projected empty semantic labels.

Choose sibling reordering around the active element, Rust-owned scroll-to-focus and `overflow:clip` for the semantic root. Normalize Tab across the declared control order on macOS browsers whose native preferences skip buttons; let Tab leave at app boundaries. Derive Text labels from the same complete text content used for Rust shaping. Keep one transparent native input per field as its semantic/editing representation.

Evidence: actual Chromium and WebKit behavior suites, a semantic-label regression test and actual browser fixture networking. These checks do not establish OS IME or screen-reader behavior. Full caret/selection/preedit rendering remains an explicit gap. Revert the projection policy independently of mount-once components if platform tasks expose a better option; never move visible layout into DOM measurement.

## ADR-007 — Repair the shared feasibility scene

Context: the original CPU renderer used nearest-neighbor images and per-primitive opacity, while Vello used bilinear images and group compositing. A regression test also exposed positive kurbo Insets expanding borders in both backends.

Choose bilinear CPU images, offscreen group opacity and inward centered borders in both renderers. Group scratch pixmaps currently span the frame; Stage 2 must bound and optimize them. The probe is a small feasibility scene with shaped text, gradient, clip, image and translation, not general renderer certification. CPU color regression tests and CPU/Metal image comparisons retain the limits of their exact scene/viewport. Future cache/resource changes must preserve these semantics.

## ADR-008 — Stage 2 retained visual data and bounded work

Context: Stage 1 established mount-once components and a small shared scene, but lacked reusable viewport styles, list virtualization, custom retained drawing data and bounded cache policies. The user explicitly authorized Stage 2 implementation on the existing branch while outstanding Stage 1 platform/manual gates remain recorded.

Options: expand visual DOM layout, replace the runtime, or extend the retained Rust engine behind its existing boundaries. Choose typed palette tokens and bounded viewport overrides resolved before Taffy; stable Taffy node identity with text/font epochs; fixed-height keyed row windows with limited focus retention; and a portable immutable `CanvasScene` plus a separate reactive camera. Rust remains responsible for visible layout, text, culling and scene geometry. JavaScript remains the capability and semantic/input adapter.

Drawing geometry is rebased in `f64` before backend conversion. Spatial candidates are bounded by scene admission limits and viewport queries, and both GPU/software paths consume the same portable primitive data. Add entry/byte budgets to shaped-text and software glyph caches, and explicit registration budgets/epochs for images and fonts. Use visible subtree bounds for CPU opacity-group scratch storage. Retained full-frame composition remains the correctness baseline; damage-region redraw and complete CSS/SVG behavior are not inferred from these changes.

Consequences: `CanvasScene::new` and image/font registration can reject invalid or oversized resources. Fixed-height virtualization requires a row-height/viewport contract; it is not a general variable-height list algorithm. Cache accounting does not by itself bound every driver/parser/temporary allocation. The finite API, limits and outstanding conformance work are documented in STAGE2.md and the authoritative acceptance ledger. Rollback can replace individual layout/renderer/cache adapters without changing the mount-once ownership model or handing visible geometry to the DOM.

Dependency basis: pinned Taffy 0.7.7, cosmic-text 0.14.2, Vello 0.5.0/wgpu 24.0.5 and tiny-skia 0.11.4/fontdue 0.9.3. No Rust compiler fork, new application language or Python build/runtime dependency was added.

## ADR-009 — Shared fallback font and explicit GPU recovery

Context: a glyph ID from a fallback font cannot be rasterized through the primary font and remain the same glyph. Likewise, initial adapter fallback is not equivalent to surviving loss of an already active GPU device.

Choose a pinned bundled monochrome Noto Emoji font alongside DejaVu Sans. Carry each shaped run's actual font into both renderers, key software glyphs by font identity as well as glyph/size, and use a font epoch to invalidate measurement/scene results when font data becomes ready. This supplies a reproducible fallback path while leaving color-font completeness and full editing/IME acceptance explicit.

For browser recovery, observe the real browser device lifecycle in the host, release GPU state, replace the presentation canvas when changing context type, and keep the existing Rust engine, scopes and host requests. GPU retry creates new rendering resources against the same retained application. Native device failures wake the window and enter the CPU recovery path; surface lost/outdated conditions retry surface configuration. Keep normal GPU presentation direct and reserve frame readback for explicit diagnostics/exports.

Consequences: recovery tests must destroy/lose the actual active device and verify preserved state, not merely throw from a mock initializer. A passing browser path does not certify native Windows, driver reset, Safari/mobile or complete resource/performance behavior. Actual evidence is recorded only after execution. Rollback can alter loss observation or the backend recreation policy independently of application/state ownership.

Font provenance: Google Fonts repository revision `6c1dbe27dcf7d817d4b62cec033ba77fa8e2a5ec`, `ofl/notoemoji/NotoEmoji[wght].ttf`, distributed with its SIL Open Font License 1.1 file. DEPENDENCIES.md records exact file hashes and source links.

## ADR-010 — Stage 3 owned application capabilities

Context: applications need editing, forms, routing and asynchronous services while retaining the mount-once model and shared browser/native engine. A broad unbounded host escape hatch would lose cancellation, resource accounting and portable errors. Browser visual controls would also split layout ownership.

Choose committed synchronous capture/target/bubble events; controlled widget state; a shared grapheme-aware Editor with bounded history; and field/form generation tickets for asynchronous validation/submission. Browser/native hosts project the same semantics and use platform input only as a bridge. Password values are explicitly redacted from diagnostic/semantic projections. Rust paints control state and editing decorations in both renderers.

Extend the existing HTTP API compatibly with builders/futures and versioned capability/operation envelopes. Scope cleanup cancels requests, tasks, streams, sockets and timers; generations reject stale completions. Query deduplication and cache observers have explicit bounds; mutation replay and reconnect remain application policies. Use explicit native adapters and reported OS utility availability instead of an embedded browser or Python. Storage migrations remain Rust functions. Assets use validated logical IDs, revisioned URLs and bounded shared PNG/font decoding.

Consequences: hosts and Wasm must be rebuilt and shipped together. Capabilities can fail or be unavailable; permissions are determined by actual operations. Native cancellation can suppress a result before a blocking OS/I/O operation exits. All finite limits are documented in STAGE3.md and STAGE3_SERVICES.md. Dependencies are pinned in Cargo.lock, including unicode-segmentation 1.13.3, AccessKit 0.25.1/accesskit_winit 0.34.1, tungstenite 0.28.0 and png 0.17.16. Local source/contract tests and actual browser/native runs are separate evidence; real IME/AT/Windows acceptance remains explicit. Rollback requires reverting each Rust protocol and its host adapter together; the retained runtime and renderer boundaries remain reusable.

## ADR-011 — Synchronize handled editing keys before browser selection events

Context: a browser can queue selection events between a handled Rust key action and the next animation frame. Reflecting only a committed scene snapshot lets the input bridge send its previous value/caret back into Rust and undo a newer key action. A deterministic End → queued selection → Backspace regression reproduced this without any OS IME claim.

Options: wait a frame after each key, ignore all browser selection events, or synchronize the live Rust editor immediately. Choose a narrow `edit_selection` host export alongside `edit_value`; after a handled key, update the existing input's value and UTF-16 selection synchronously. The next browser selection event then observes the current model. Keep composition handling and ordinary platform selection events active; no DOM measurement or visible layout is introduced.

Consequences: the input bridge and Wasm export change together. Rust still owns text/selection and the next frame paints that state. Chrome and WebKit regression tasks exercise ordering separately from real OS IME acceptance. Rollback must replace the immediate synchronization with an equally ordered bridge; adding test sleeps would conceal the race.

Ordinary Text/Paragraph nodes can explicitly opt into a retained read-only selection model with `.selectable(true)`. Keep their Rust text kind and geometry; expose one read-only multiline platform bridge, and flatten text children only in that projection to prevent duplicate accessible content. Default text does not enter the Tab order. Native pointer selection uses the same shaped clusters; clipboard uses the selected Rust string and readonly text does not enable IME. This supplies selectable ordinary content without making every label an editable field or moving visible text layout to the browser.

## ADR-013 — Scoped context lookup and owner lifetime (RP-003)

Decision, 16 November 2026: keep `Scope::provide<T>` and `Scope::context<T>` as typed, nearest-owner context. Same-type provision replaces the value in that owner only; siblings/independent runtimes cannot see it. Use distinct wrapper types for independent contexts with the same payload type. Lookup returns a clone of the value captured at lookup time. Replacement neither remounts children nor notifies readers. A provided `Signal<T>` supplies reactivity when its `get()` is called inside a tracked computation; consumers holding a prior signal keep that signal until their own code looks up another one. Scope/Signal handles do not keep the owning runtime alive.

A disposed scope returns None from context lookup and rejects provide with the existing `disposed scope` panic contract. Retained signals reject reads/writes once their owner is disposed. Child owners dispose before parents; callbacks within one owner have no promised order. Context resources are released after removing that owner's runtime records and before its registered cleanup callbacks. User-defined Clone/Drop code on context values must run outside the runtime RefCell borrow so it can inspect live scopes or signals without an internal borrow panic. A lookup's captured value may remain alive until its clone completes, even if reentrant code replaces the provider.

Implementation choice: retain context entries internally with Rc, clone the Rc under the borrow, and clone T after releasing the borrow. Remove/replace entries under a short borrow and drop retired contexts outside it. This does not turn context lookup into a subscription. A custom Clone that explicitly reads a signal has the normal signal tracking behavior; such user code is not a plain lookup. Arbitrary panic recovery or recursive provider mutation is not promised.

No public signature or application migration is needed. New regressions cover shadowing, siblings, roots, replacement, disposal/remount, dynamic signal dependencies, and reentrant Clone/Drop. Full reactive provider replacement, external store snapshots, layout effects and platform UI usability remain separate tasks. Rollback must retain the regressions and an equivalent borrow-safe context lifetime design.

## ADR-014 — Published external snapshots (RP-006)

Choose a single-UI-thread `ExternalStore<T>` with immutable, versioned `StoreSnapshot<T>` values and `StoreSubscription<T>` handles owned by Scope. External host/data adapters publish complete owned values; core contains no DOM or browser API. This first adapter accepts push publication rather than executing arbitrary external getSnapshot/subscribe callbacks. It does not claim React API compatibility, concurrent threads, server hydration or a general executor.

`publish` compares against the current value and deduplicates equal values. It updates every live subscription before invoking any runtime waker, once per runtime, so independent roots receive the same revision. Equal publication returns Ok(false), including during a commit. Changed publication while any subscribed runtime is mounting/flushing, or any reentrant publication while this store is already publishing, returns `ExternalStoreError::Busy` without changing the snapshot. Callers must retry via their host event queue after commit; writes during render/effect are never silently dropped or deferred by core. All data reachable through T must be immutable; interior mutation bypassing publish violates the contract. PartialEq must be pure. One returned snapshot contains both revision and Rc value; read it once for related fields.

Subscriptions read a scope-owned signal, enabling existing memo/view tracking. Dropping a handle does not unsubscribe; owner disposal or explicit disconnect does. Replacing a source preserves that signal and retires old membership before notifying the runtime. Failed replacement leaves the original subscription intact. A store supports at most 64 live subscriptions; revisions/IDs use checked counters. Explicit disconnect removes store membership and cleanup registration, and invalidates reads; its signal storage remains scope-owned until disposal (as for ordinary Scope::signal). Repeated replacements do not allocate additional signals or cleanup registrations.

Use weak store membership and a strong scope cleanup registration, avoiding a runtime/store reference cycle. Runtime mount/flush uses an unwind-safe commit flag shared with its scopes. Rejecting publication during commit is deliberate backpressure, not asynchronous scheduling. Existing host wakers are scheduling callbacks, not arbitrary unbounded mutation loops; same-store reentrant publication returns Busy. Panic recovery from user callbacks and atomicity across different stores are excluded. Validate snapshots, equality, race/replacement/disposal, bounds, independent roots and actual demos before making a finite completion claim.

## ADR-015 — Browser host ownership and DOM accounting (RP-004)

One page owns one live browser host and one Wasm APP slot. This is an explicit singleton contract, not multi-root support. Development restart disposes the previous host before creating the next; overlapping restart requests share one promise. Independent documents remain independent. The shell canvas/semantic/fatal containers belong to the document; the host owns listeners, animation requests, network/services, projection nodes, preview, development tools and the active Rust runtime. Disposal is idempotent, aborts host listeners, cancels animation, disposes adapters before dropping Rust, removes owned tool/preview nodes, and prevents asynchronous completions from reviving the old host. Foreign children are not removed.

A persisted pagehide suspends painting without destroying Rust state or the semantic projection. Persisted pageshow resumes/invalidate/resizes the same runtime. Non-persisted pagehide disposes it. This follows the pagehide/pageshow distinction described in [Chrome's BFCache guidance](https://web.dev/articles/bfcache). It does not guarantee eligibility with active sockets, other browser services or embedded tools; actual cache restoration and synthetic lifecycle-event tests must be reported separately. No unload listener is introduced.

DOM accounting is opt-in (`devtools` build plus `?dom-audit=1`). It retains only aggregate category/mutation counters, not mutation records or removed nodes. Categories are shell, semantic bridge, emergency fallback, browser service, isolated preview, developer tooling, resources and unregistered elements. Counts cover the main document; popup/iframe interiors are separate documents, not silently counted as zero. Production has no ordinary status badge or developer accounting API. Semantic opacity/layout contracts remain; no pooling or accessibility deletion is authorized. Freeze reference-route counts and mutation ceilings with actual browsers before any later optimization. Bounds are regression checks for these workloads, not a universal cap on accessible content.

## ADR-016 — Optional source-built demo container

Docker Compose is an optional onboarding path for the browser examples, not a new native runtime or a dependency of normal Rust development. A Linux multi-stage image installs Rust 1.90.0 and wasm-bindgen 0.2.100, builds the locked production Wasm bundle and Rust CLI from the checkout, then copies only the executable, bundle and notices into a non-root Debian runtime. A source allowlist excludes host caches, generated bundles, Git metadata and local credentials. BuildKit caches compilation; no host Rust/Node/compiler installation or source bind mount is required.

Use the existing Rust HTTP server so fixture success/error/cancel and WebSocket routes retain their behavior. `serve --host IP` explicitly changes the bind address; absent that option it remains 127.0.0.1. The container binds 0.0.0.0 internally and Compose publishes only 127.0.0.1 on the host. The image does not start the isolated compiler: Linux sandbox operation remains unverified and must not be silently replaced with unrestricted compilation. Browser GPU/input capabilities belong to the user's browser, not a container display server. Docker Desktop on Windows must use Linux containers. Container build/browser evidence on one host does not certify native Windows or Windows Desktop onboarding.

## ADR-017 — Native-first portable packaging (RP-007)

The next execution priority is native delivery, not a browser task. `rustx package --target native|macos|windows` builds only the chosen Cargo binary in release mode for the actual rustc host triple and creates a relocatable developer package. This is distinct from the existing raw `build --target native` output. macOS uses an `.app` with Info.plist, executable and resource notices, archived with system ditto; Windows uses an extracted portable directory/ZIP with static CRT linkage and requires an actual MSVC developer shell for dumpbin inspection. No cross-build or startup-only test is promoted to full OS acceptance. Linux packaging, signing/notarization, installers, external asset configuration and clean-machine verification remain explicit RP-007 follow-ups, not hidden success claims.

Dependencies are inspected before publishing output. Only recognized system link dependencies are admitted; unbundled custom/CRT libraries fail closed. This does not certify dynamically loaded GPU drivers. Payload hashes, dependency output, target/version/toolchain metadata and licenses travel with the archive. Build/archive work stages in a private sibling directory; failures remove staging and never replace an existing package output. Recipients run the extracted application without Rust/Node/browser tooling. Developers still need Rust and native build tools; Docker remains a separate browser-demo convenience.

macOS structure follows [Apple's bundle layout](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/BundleTypes/BundleTypes.html). Windows static CRT uses the [Rust linkage contract](https://doc.rust-lang.org/reference/linkage.html). These references guide implementation; actual artifact startup and platform reports remain required. No signing credentials, app-store submission or public release is assumed.

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

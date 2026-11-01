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

# Architecture decisions

## ADR-001 — Rust and mount-once components

Rust is the core/application language for integration with the selected layout/text/graphics ecosystem. This is an architectural choice, not a Go/React speed benchmark. Use normal Rust modules with rustx!, not a compiler fork. Plain props are initial configuration; explicit signals carry changes. Key changes remount. Reuse rendering modules from the supplied MIT RustUI project; replace its runtime spine, not its verified rendering work without cause.

## ADR-002 — Explicit host boundary

Rust owns state/layout/text/scene and scoped IDs. JS loads Wasm and connects browser networking/input/semantics. IDs are monotonic and bounded to exact JavaScript integers; malformed numeric IDs are rejected, stale results require a live owner. GET/cancel is the current slice. Version/capability negotiation and broader services are future gates. Browser AbortController cancels transport; native cancellation currently rejects completion while blocking I/O times out.

## ADR-003 — Vello plus CPU adapter

Pin the compatible Taffy/cosmic-text/Vello/wgpu set inherited from supplied rendering work, and isolate tiny-skia/fontdue CPU replay behind an adapter. Direct browser GPU presentation is separate from diagnostic offscreen readback. CPU parity is unfinished; upgrade/replace its adapter without rewriting authoring or scoped state. Software graphics output is not hardware certification.

## ADR-004 — Examples are release gates

Build the homepage/API app early through public RustX APIs. Clearly label current precompiled presets. The final two examples require real API success and real editable Rust compilation. Select a sandboxed compilation architecture in a new ADR before implementing visitor code execution; never add unrestricted native execution to the fixture server.

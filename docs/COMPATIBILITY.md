# Compatibility and limitations

Compilation and mocked services do not certify real targets. The acceptance ledger separates the supplied snapshot from current execution evidence.

- Rust owns visible state/layout/shaping/scene. Generated HTML/JS and transparent semantic/input controls remain necessary. This is canvas-rendered, not literally zero DOM/JS.
- Current actual browser behavior evidence: Chrome 156.0.8078.74 CPU and visible WebGPU on macOS 27.0.1 / Apple M3; Playwright WebKit 27.2 CPU. Keyed editing/focus/selection, single keyboard activation, Tab order, fixture Fetch/cancellation, viewport/DPR and idle checks pass. WebKit is not Safari certification. No Windows, human screen-reader, real OS IME, Android/iOS or mobile keyboard task has passed.
- Firefox 135/157 failed to start with profile/sandbox errors on this host. The updated Chromium CDN download returned a regional 403; installed Chrome was used. Exact logs are retained. These conditions are recorded as blocked verification, not framework/browser success.
- Native CPU and Metal windows started on this Mac and exited after two frames. The small CPU/Metal scene passes its declared comparison tolerance. No Windows executable is included or certified. Full backend conformance, device-loss recovery and native interaction/DPI checks remain open.
- CPU uses tiny-skia/fontdue with one embedded DejaVu font. Bilinear images, group opacity and inward borders now agree in the feasibility scene. Font fallback/emoji and general effects remain incomplete. Group opacity currently uses full-frame scratch pixmaps. The responsive font override has one 600px breakpoint. No speed/resource-budget claim.
- Browser editing uses one transparent native input per field. Native selection survives keyed reorder and labels reflect Rust text. Value/composition forwarding exists, but Rust-painted caret/selection/preedit, full grapheme/bidi editing, undo/redo, clipboard/autofill and native editing are unfinished. Native supports basic appended text/backspace/IME commit; Windows UI Automation is absent.
- Keyed matching is not optimized for huge collections. Layout/rendering still process the changed tree broadly. Culling, virtualization, bounded resource lifecycle and comparative performance remain Stage 2+ work.
- Host protocol version 1 supports bounded GET/cancel, typed capability reports, mismatch rejection and stale completion protection. Native blocking I/O may continue until timeout after cancellation. No streaming/upload/WebSocket/query cache/general service API. See HOST_PROTOCOL.md for exact limits. Fake Fetch contract tests are separate from actual browser fixture networking.
- Navigation is component state, not history/deep links. The loopback fixture server is for development. No public deployment was performed; live JSONPlaceholder acceptance remains open.
- Homepage presets are precompiled. No arbitrary Rust compiler, first-class .rustx editor, project generator, published crates or accepted Windows distribution yet.
- npm test tooling is pinned to Playwright 1.64.1 and sharp 0.35.5; the current npm audit reports zero vulnerabilities. This does not substitute for the full transitive Cargo/license/unsafe/security audit.

Do not describe this prototype as production-ready, universally faster than React, a complete React/HTML/CSS replacement or a new browser. Stage 1 is still open.

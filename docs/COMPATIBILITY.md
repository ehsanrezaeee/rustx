# Compatibility and limitations

Compilation, offscreen images and mock networking do not certify browser/Windows usability. Consult the evidence ledger for current executed commands.

- Browser visual state/layout/shaping belong to Rust. Generated HTML, JS capability adapters and transparent semantic DOM controls remain necessary. This is canvas-rendered, not literally zero DOM/JS.
- Real Chromium/Firefox/Safari, Android/iOS, Windows, IME and screen-reader tasks are unverified. Browser/window socket creation was blocked with EPERM in this execution environment. A 390px image is not mobile-device evidence.
- CPU uses tiny-skia/fontdue and one embedded DejaVu font. Persian/English shaping works for the sample. Font fallback/emoji, blur/shadow, group opacity and general effect parity are incomplete. CPU images use different filtering than Vello. The responsive font override has one 600px breakpoint.
- Browser input forwards whole strings/composition events. Visible caret/selection/preedit, undo/redo, grapheme/bidi editing, clipboard/autofill and full native input are unfinished. Native currently supports basic appended text/backspace/IME commit; Windows UI Automation is absent.
- Keyed matching is not optimized for huge collections. Layout/rendering still process the changed tree broadly. Culling, virtualization, cache/lifecycle budgets and performance comparisons remain unmeasured.
- HTTP supports bounded GET/cancel. Native cancellation discards stale completion while blocking I/O can run until timeout. No streaming/upload/WebSocket/query cache/header API or capability/version handshake yet. JS adapter tests use fake Fetch implementations for deterministic failures.
- Navigation is component state, not history/deep links. The loopback fixture server is a simple dev server, not hardened production hosting. No public deployment was performed.
- Homepage playground components are precompiled. No arbitrary Rust compilation, first-class .rustx editor, project generator, published crates or final Windows distribution yet.

Do not describe this prototype as production-ready, universally faster than React, a complete React/HTML/CSS replacement or a new browser.

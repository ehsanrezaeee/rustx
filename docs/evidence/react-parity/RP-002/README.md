# First React parity increment — 16 November 2026

Baseline source: `43c8b53`. Changes: initial roadmap/audit/backlog and semantic projection teardown. RP-001 remains implementing; RP-002's finite adapter lifecycle contract is verified. Full host/root lifecycle, BFCache, platform support and React parity remain open.

Before the change, `node scripts/test-semantic-lifecycle.mjs` failed in real Chromium on the first disposal: owned DOM remained attached (`before.log`). The fixed projection uses per-node AbortController registrations, aborts listeners before removing nodes, clears composition state when a node disappears, and releases semantic/focus/composition references during idempotent disposal. Late sync does nothing; create after disposal rejects. Foreign root children survive. No Rust ownership/layout/text/rendering code changed.

| Check | Result / evidence |
| --- | --- |
| `npm run test:semantics:lifecycle` | Chromium 156.0.8078.4 passes 100 cycles; chromium.log |
| `RUSTX_BROWSER=webkit npm run test:semantics:lifecycle` | WebKit 27.2 passes 100 cycles; webkit.log |
| `npm run test:host` | Pass, adapter contracts with fake services; host.log |
| `bash scripts/build-web.sh` | Fresh host/CLI/Wasm bundle; build.log |
| `npm run format:check`, `cargo fmt --all -- --check` | Pass; format.log (rustfmt emits no output) |
| `cargo test --workspace --locked` | 134 passed, 0 failed, 1 ignored native networking fixture; workspace.log |
| `npm run test:wasm` | Pass; real Wasm in Node, CPU rendering; wasm.log |
| Inherited `scripts/test-browser.mjs`, Chromium and WebKit | Pass against rebuilt app on loopback port 3038; app-chromium.log, app-webkit.log, JSON and screenshots |

The new lifecycle suite imports the production semantic module into a real browser DOM, using a stub Rust callback receiver. It verifies detach/listener/state behavior, not full app correctness or accessibility. The separate inherited browser suites exercise actual Rust Wasm/CPU input, navigation, fixtures and responsive behavior. They were run from a temporary script copy so prior reports were not overwritten. Neither establishes OS IME, human screen reader, branded Safari, Windows, GPU, physical mobile or performance superiority.

Host source and served `dist/semantics.js` SHA-256: `861f3a2abe0796f821377d87a1ff027df2289e9bc7ac26d29dfd326519bc1d50`.

Wasm SHA-256: `59e8c6b339a4c44634bf93828717b48505920152d2221d2daa39dae73d2a84da`. Unchanged application source can produce the same Wasm hash as historical builds; the changed host hash identifies this fix. Node: 24.18.1, Rust: 1.90.0, macOS arm64. JSON dates are UTC; user-local date is 16 November (Asia/Tehran). CI now includes the lifecycle suite in its existing Chromium job; that CI change has not executed remotely in this increment.

Audit checks: source links in REACT_PARITY_MATRIX.md resolve; whitespace/format checks pass. React 19.3 reference inventories were consulted, but full element/property/event enumeration and executable comparative baselines remain unfinished. No existing RX status is upgraded by this audit. Next proposed increment is RP-003, context/owner isolation contracts, after owner confirmation.

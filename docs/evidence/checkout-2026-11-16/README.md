# Source checkout verification — 16 November 2026

The previous README began with `serve dist` and described artifacts supplied only in a delivery archive. Git ignores those generated bundles, so that first command could not launch a fresh GitHub checkout. The revised README installs prerequisites, builds the source, then serves the result. `.gitignore` now documents how ignored outputs are regenerated. No required source or font asset was found missing from Git.

Verification used `git archive HEAD` of baseline `4971e24` extracted into `/tmp/rustx-clean-checkout.vdxdrW`, with the updated README, LICENSE, NOTICE.md, .gitignore and docs/DEPENDENCIES.md overlaid. Initially this copy had no `target/`, `dist/`, `.rustx/`, `node_modules/`, `releases/` or CLI SDK archive. Its build used its own new target directory, with the machine's existing Rust/tool installations and Cargo registry cache. This is a tracked-source replay on the existing Mac, not independent first-download or clean-machine certification. Later evidence/status-only additions do not change application code. Historical build manifests remain attributed to their original artifacts.

Actual commands and outcomes:

| Command | Result / evidence |
| --- | --- |
| `bash scripts/build-web.sh --compiler-url http://127.0.0.1:3002` | Pass; built CLI directly from tracked SDK source, compiled Wasm and generated `dist/`; `build-web.log` |
| `npm ci` | Initial sandbox DNS/cache-write failure retained in `npm-ci.log`; network-enabled replay with writable temporary npm cache passes, zero audit findings; `npm-ci-network.log` |
| `npm run format:check` | Pass; `format-check.log` |
| `cargo fmt --all -- --check` | Pass; `rustfmt.log` (empty success output) |
| `cargo test --workspace --locked` | 134 passed, zero failed, one ignored native loopback fixture; `workspace-tests.log` |
| `npm run test:host` | Pass; JavaScript adapter contracts, including fake Fetch; `host.log` |
| `npm run test:wasm` | Pass; actual rebuilt Wasm under Node, Rust CPU renderer; `wasm.log`, `wasm-tests.json` |
| `npm run test:stage2` | Pass; actual Wasm/CPU virtualization, text/layout and drawing contracts; `stage2-wasm.log`, `stage2-wasm.json` |
| `npm run test:http` | Sandbox fixture startup failure retained in `http.log`; loopback-enabled replay passes real Rust server → Fetch → Wasm success/error/cancel; `http-loopback.log` |
| `npm run test:stage3:services` | Sandbox fixture startup failure retained in `services.log`; loopback-enabled replay passes actual HTTP/pull-stream/WebSocket tasks; `services-loopback.log` |
| `cargo run -p rustx-cli --locked -- serve dist 3038` | Pass; freshly generated bundle served over loopback; `server.log` |
| `RUSTX_URL=http://127.0.0.1:3038 npm run test:browser` | Pass in actual Chromium 156.0.8078.4, Rust CPU backend; `browser-chromium.log`, JSON and 390px screenshot |

Wasm SHA-256: `fbe810f55b01925bdcab038935c00315a3c64c7ea27323c291d8a5339bc47365`.

The server used port 3038 to avoid disturbing local services. JSON timestamps use UTC; this session's user-local date is 16 November 2026 (Asia/Tehran). No Windows, WebKit, GPU, OS IME, human screen-reader, physical mobile or complete v1 certification is inferred. Compiler/editor/production/native distribution suites were not replayed for this documentation and attribution change. All earlier release gates remain open.

# RP-003 — Context and owner isolation

16 November 2026. Source baseline `a2db1a7`; this increment changes scoped context internals and adds ten public-API regressions, ADR-013 and a tutorial example. RP-003 is verified for its finite core contract. It does not complete reactive provider replacement, external stores, platform usability or React parity.

## Reproduction and correction

The initial eight regression tests produced five passes and three failures on the original implementation (`before.log`). Retrieving a context whose custom Clone reads a signal, replacing a context whose Drop reads the runtime, and disposing a context whose Drop inspects its scope all triggered internal RefCell borrow panics. The earlier test-authoring compiler error is retained separately in `test-authoring-error.log`; it is not a framework defect.

Context entries now use internal Rc ownership. Lookup captures the entry under a short borrow and clones the application value after releasing that borrow. Replacement drops the retired entry outside the mutable borrow. Disposal removes the owner and its runtime records, releases the borrow, then drops context resources before registered cleanup callbacks. Public signatures are unchanged; disposed provide now uses the explicit `disposed scope` diagnostic consistently.

The ten tests cover nearest-provider shadowing and sibling isolation, independent runtimes, plain replacement without rerender/remount, dynamic dependencies through provided signals, provider removal/remount and invalidated retained handles, reentrant Clone/Drop, signal replacement retaining existing consumer bindings, and child-before-parent/resource-before-callback cleanup. Same-owner callback ordering remains unspecified. Context lookup itself does not subscribe; explicitly reading a signal in custom Clone code still has normal tracking semantics. Rc may extend a captured entry's lifetime until an in-progress lookup finishes; arbitrary recursive user mutation or panic recovery is not promised.

## Executed checks

| Command/task | Actual result |
| --- | --- |
| `cargo test -p rustx --test context --locked` | 10 pass; context.log |
| `cargo test --workspace --locked` | 144 pass, zero fail, one ignored native networking fixture; workspace.log |
| `npm run format:check`; `cargo fmt --all -- --check` | Pass; format.log; rustfmt emits no output |
| `bash scripts/build-web.sh` | CLI/host/Wasm rebuilt; build.log |
| `scripts/test-wasm.mjs`, `scripts/test-stage2.mjs` | Pass, real Wasm CPU in Node; wasm.log, stage2-wasm.log and JSON |
| `npm run test:host` | Pass; host.log; fake service/adapter contract checks |
| `RUSTX_TEST_PORT=3001 npm run test:http`, `npm run test:stage3:services` | Actual loopback HTTP/Fetch/Wasm and stream/WebSocket checks; http.log, services.log |
| `scripts/test-browser.mjs`, `scripts/test-stage2-browser.mjs` | Pass on Chromium 156.0.8078.4 and WebKit 27.2, CPU backend; app/engine logs, JSON and PNGs |
| `RUSTX_DEMO=applications RUSTX_RENDERER=cpu RUSTX_SMOKE_FRAMES=2 cargo run -p rustx-lab --bin rustx-lab --locked` | Exit 0, actual native Mac two-frame startup; native-cpu.log |

The inherited browser demos exercise existing context consumers, including services and theme state. They are integration regression evidence, not a new complete context UI usability study. Native startup is not full interaction/IME/AT evidence. No GPU, Windows, physical mobile, branded Safari or human screen-reader claim is made. Browser and Node scripts were copied unchanged into `/tmp/rustx-rp003.0PAO50`, with local web/dist/fixture/dependency links, to retain output separately from historical reports. Server URL was `http://127.0.0.1:3038`; it was stopped after verification.

Core source SHA-256: `320d45768806769b5b605786b171b3476727036ae3a49dfa85c5556019864311`.
Wasm SHA-256: `036dba4b983c78ae035f525292e2074e8c093de7170bc26c976257b8c355be30`.
Environment: Rust 1.90.0, Node 24.18.1, macOS arm64. JSON timestamps use UTC; session date is 16 November in Asia/Tehran.

An initial automatic-approval usage-limit error prevented browser/native verification commands from being executed. After the owner resumed the task, approval succeeded and the actual runs above completed. This interruption was neither a failed application test nor an unsafe-action determination.

Next proposed increment: RP-006 external-store snapshot contract, after owner confirmation. RP-001 exhaustive browser inventory and matched comparative apps remain unfinished. Existing RX acceptance gates retain their prior states.

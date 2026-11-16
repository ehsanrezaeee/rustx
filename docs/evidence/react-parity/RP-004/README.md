# RP-004 — DOM budgets and browser host ownership

16 November 2026. Source baseline `f268132`. Verified scope: measured singleton CPU host ownership, development-only main-document accounting, frozen reference-route budgets and production status removal. This does not certify full browser lifecycle, BFCache restoration, simultaneous roots, native platforms, GPU, physical IME or human assistive technology.

## Changes and reproduced failure

ADR-015 defines one live host per document/Wasm APP slot. Disposal aborts host listeners, cancels animation, disposes network/services/semantic projection and preview/tools, removes developer DOM, releases the Rust APP, and resets the canvas. Restart waits for disposal (including pending GPU initialization) and coalesces overlapping requests. Captured old development methods are inert except explicit lifecycle/accounting methods. The lab and shared scaffold export `dispose_app`. The document shell, opt-in page-owned audit and unrelated root children have separate ownership.

Persisted pagehide suspends painting while retaining Rust and semantic state; persisted pageshow invalidates/resizes/resumes. Non-persisted pagehide disposes. No unload handler is installed. Developer tools now own their event listeners, polling fetch/timer and nodes; preview owns its iframe/message/click handlers. Production never creates the normal status badge. Fatal startup retains the reload button and explanatory message.

The new actual-browser pending-request regression first failed: immediately after disposal the HTTP admission map still contained one aborted request. `ownership-before-fix.json` and `.log` retain this result. HostNetwork now clears admission entries synchronously and clears timeout registrations on abort. Promise completion remains suppressed after disposal. The focused fake-Fetch test also asserts synchronous release; the actual-browser regression now passes in both engines.

An attempted production output under `/tmp` was rejected by the CLI's existing real-child-output rule (`production-output-path-error.log`). The final production build uses `.rustx/rp004-production` inside the project. This is a test invocation correction, not a relaxed output restriction. Earlier bundles and baseline collection logs are retained separately.

## Frozen counts and enforcement

See [DOM_EXCEPTIONS.md](../../../DOM_EXCEPTIONS.md) and `fixtures/dom-budget-{chromium,webkit}.json`. Both engines measured semantic counts 36/21/26/99/67/18/50/13 for `/`, `/people`, `/lab`, `/engine`, `/applications`, `/playground`, `/capabilities`, `/store`. Settled dev shell counts are shell 7, resource 6, tooling 2. Disposal leaves only shell 7/resource 6. Optional service, preview, emergency and unregistered categories are zero in those settled route workloads; that does not waive their independent capability tests.

The baseline was collected before any semantic optimization; none was performed in this increment. A store publication adds/removes zero elements. Nonzero attribute/text/child-list ceilings are the recorded baseline plus 10% rounded up and ten records; zero and element additions/removals get no slack. Subsequent default runs enforce the frozen files rather than rewriting them. Main-document counters exclude popup/iframe contents and browser heap/GPU allocations. Mutation records count writes, not necessarily changed values, and retain no application text. A negative test detects an injected unregistered control and verifies same-task insertion/removal classification for transient emergency elements.

## Executed checks

| Command / task | Actual result |
| --- | --- |
| `cargo test --workspace --locked` | 154 pass, zero fail, one ignored native networking test; workspace.log |
| `npm run format:check`; `cargo fmt --all -- --check` | Pass; format.log; rustfmt silent |
| `npm run test:host` | Pass including immediate disposal assertion; host.log. Mock Fetch/service contract evidence only |
| `bash scripts/build-web.sh` | CLI/host/Wasm rebuilt; build-final.log |
| `target/debug/rustx build --target web --release --out .rustx/rp004-production` | Actual production build; production-build-final.log |
| `scripts/test-wasm.mjs`, `scripts/test-stage2.mjs` | Actual rebuilt Wasm/CPU passes in Node; wasm/stage2-wasm JSON/logs |
| `scripts/test-browser.mjs`, `scripts/test-stage2-browser.mjs`, `scripts/test-stage3-browser.mjs` | Actual Chromium 156.0.8078.4 and WebKit 27.2 CPU application, engine, input and services tasks pass; app/engine/services logs and inherited JSON |
| `RUSTX_URL=http://127.0.0.1:3038 npm run test:dom:budget` and WebKit override | Each passes eight route counts, per-update mutation ceilings, 100 round-trip route cycles, 100 actual Rust root dispose/restart cycles, stale detached controls, duplicate dispose/concurrent restart, and synthetic persisted lifecycle state/input preservation; dom JSON/logs/PNGs |
| `RUSTX_URL=http://127.0.0.1:3038 npm run test:host:ownership` and WebKit override | Actual pending HTTP cancellation, no old completion after restart, negative accounting probes, and 100 preview adapter mount/dispose/message/click cycles pass; ownership JSON/logs |
| `RUSTX_URL=http://127.0.0.1:3039 npm run test:dom:production` and WebKit override | Actual production canvas/input, no status/tooling/audit API even with query opt-in, blocked-Wasm emergency reload UI pass; production JSON/logs |
| Standalone CLI replay `scripts/test-stage4-cli.mjs` | Embedded SDK/scaffold builds and executes in Chromium; source edit reload, real Cargo error overlay and recovery pass; cli-browser.log and stage4-cli.json |

The final ownership, frozen-budget and production runs use the final host/audit/services source including the synchronous HTTP correction. Workspace, Wasm and broad inherited browser tasks cover the unchanged final Rust exports and earlier host version; targeted final runs cover subsequent host-only fixes. The standalone CLI replay used the shared Rust export and tool teardown changes before the final HTTP/map correction; its result certifies those scaffold/refresh paths, not a separate fresh-machine installation of the final commit. Final CLI embedding is refreshed after documentation updates.

Inherited scripts were copied into `/tmp/rustx-rp004.KLiSec` with linked web/dist/fixtures/node_modules to preserve historical reports. The CLI runner's output destination alone was changed to this evidence directory; assertions were unchanged. CLI uses the local dependency/toolchain cache. Source revision/hash values printed by historical inherited tools retain their original meanings. The temporary local servers on 3038/3039/3040 were stopped after verification.

## Limits and next work

Both actual production back-navigation runs reported `persisted: false` and a new document. They are **not** successful BFCache restoration evidence. Synthetic persisted events validate host branching and state retention only. Actual BFCache eligibility/restore, especially with sockets, embedded previews and developer tools, remains open. Preview cleanup tests execute real browser DOM/events with a stub Rust callback receiver; they do not certify newly compiled previews or inner-frame accessibility. Host GPU lifecycle cleanup is source-reviewed but not GPU-tested in this increment.

No mobile device, branded Safari, Windows, native startup, OS IME, human screen-reader, benchmark advantage, or full React parity success is inferred from these checks. Existing semantic opacity/counts and scripted keyboard/input behavior are not complete accessibility acceptance. Linux Chromium CI is wired to the new budget/ownership/production checks; that workflow must run remotely before its result is claimed.

Final SHA-256:

- `web/host.js`: `958237013f70c37717b98935ce3acee9d8b96ec6d5cbcd54fd5d5050edd849e8`
- `web/devtools.js`: `1f2c67692aaec61a285e634d69c610b91018ec9cd9a26e5a1a3c54e4713bcaec`
- `web/services.js`: `54fd87447bf3f2b75a2a82b031d198563676d8e8f55805a3cf9bfd417afa4651`
- Development and production Wasm: `903e0a77dfb797c63bc8863a04f0f3413d41a3190c3c776e066b7ed0dff5d02b`

Environment: macOS arm64, Rust 1.90.0, Node 24.18.1. JSON timestamps are UTC; user-facing date uses Asia/Tehran. Generated bundles remain reproducible from tracked source. Pause for owner confirmation before the next browser input/AT conformance increment; RP-001 exhaustive inventory and matched React benchmark work remain unfinished.

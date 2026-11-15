# RP-006 — External-store snapshots

16 November 2026; source baseline `b72c0f9`. This increment delivers the finite UI-thread push-store contract in ADR-014, ten public-API regressions, a canvas example and runnable browser checks. It does not complete useSyncExternalStore compatibility, arbitrary third-party getSnapshot/subscribe adapters, hydration/server snapshots or multithreaded publication.

`ExternalStore` owns immutable versioned values outside Runtime. Scoped subscriptions use the existing signal tracker. Changed publication installs all subscription snapshots before waking each live runtime once; a synchronous earlier wake can dispose another root without a stale callback. Mount/flush publication returns Busy and the caller must retry later. Equal publication deduplicates; reentrant publication returns Busy. Membership is bounded to 64 subscriptions, source replacement preserves signal/cleanup allocations, and disposal/disconnect invalidates retained readers. Snapshot values must remain logically immutable and PartialEq pure. Cross-store transactions and arbitrary callback panic recovery are excluded; the commit flag itself resets on unwind.

## Executed checks

| Command | Result and retained evidence |
| --- | --- |
| `cargo test -p rustx --test external_store --locked` | 10 pass; core.log |
| `cargo test --workspace --locked` | 154 pass, zero fail, one ignored native network fixture; workspace.log |
| `npm run format:check`; `cargo fmt --all -- --check` | Pass; format.log (rustfmt silent) |
| `bash scripts/build-web.sh` | CLI/host/Wasm rebuilt; build.log |
| `node scripts/test-wasm.mjs`; `node scripts/test-stage2.mjs` | Real Wasm/CPU checks pass in Node; wasm.log, stage2-wasm.log and JSON |
| `npm run test:host` | Pass; host.log; mock host/service contract evidence only |
| `RUSTX_URL=http://127.0.0.1:3038 node scripts/test-external-store-browser.mjs` | Actual Chromium 156.0.8078.4 CPU canvas pointer tasks pass; browser-chromium JSON/log and PNG |
| Same command with `RUSTX_BROWSER=webkit` | Actual WebKit 27.2 CPU canvas pointer tasks pass; browser-webkit JSON/log and PNG |
| `RUSTX_DEMO=store RUSTX_RENDERER=cpu RUSTX_SMOKE_FRAMES=2 cargo run -p rustx-lab --bin rustx-lab --locked` | Exit 0; actual native Mac two-frame startup only; native-cpu.log |
| `node scripts/prepare-cli.mjs`; `cargo build -p rustx-cli --locked` | Embedded SDK refreshed and CLI built; cli.log |

Core tests cover deduplication, 100 publications across independent roots, staging before notification, Busy on mount/effect/view publication, unchanged values during rejected writes, 100 source replacements without owner resource growth, capacity failure preserving the old source and retry after disposal, invalid retained handles, unwind reset, synchronous root disposal, and mounting a new reader during a wake.

Browser tasks use pointer coordinates from Rust layout, inspect committed Rust values, and capture canvas pixels. They publish a coherent pair, switch to another source, publish while reading the alternate source, switch back, unmount, publish without a reader, and remount at the latest revision. This adds no visible DOM layout or browser dependencies to core. Existing semantic/input bridges remain. Native startup is not native interaction evidence. No Windows, GPU, branded Safari, physical mobile, OS IME, human screen-reader, performance advantage or full React parity claim is made.

Inherited Node scripts were copied unchanged into a temporary `/tmp/rustx-rp006.*` tree with linked dist/web/fixtures/node_modules, keeping historical evidence untouched. Their JSON reports are retained here. The temporary browser server on port 3038 was stopped after verification. Build outputs remain ignored and reproducible from tracked source; see TUTORIAL.md for browser/native run commands.

During test authoring, an initial missing helper closing brace prevented compilation; it was corrected. The initial capacity regression expected Capacity from replacement inside mount, but correctly received Busy (seven other tests passed). That assertion now runs after mount and tests capacity admission plus successful retry after slots are freed. These were test-authoring corrections, not observed framework failures; early logs were overwritten before the final evidence collection.

SHA-256:

- `crates/rustx-core/src/external_store.rs`: `29e0c9efd008835a0b0a6e73346cbbb7b7f1995b239ca5575bb27e7120855206`
- `crates/rustx-core/src/lib.rs`: `1623126dfbcf4201b1b54968ac4352e1131025b771478d84cbc186003b673f38`
- `dist/pkg/rustx_lab_bg.wasm`: `e5d25d4eaa9c1606d5fe5926ddbf79e9b77dff9859b1cd539eb9a79ea14ec532`

Environment: Rust 1.90.0, Node 24.18.1, macOS arm64. Reports use UTC; session date is 16 November in Asia/Tehran. One capacity-test retry assertion was added after the workspace run and passed in the final focused run. Runtime/example source is identical across workspace, fresh Wasm and browser/native checks.

Pause for owner confirmation before RP-004 DOM exception accounting. Existing RX acceptance gates and incomplete RP-001 comparative/exhaustive browser coverage remain open.

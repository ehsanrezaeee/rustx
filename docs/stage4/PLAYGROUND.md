# Real Rust compilation playground

## Run locally

Build/install the CLI as in TUTORIAL.md. In one terminal at the framework root:

    rustx playground --port 3002 --app-origin http://127.0.0.1:3000

Wait for the `isolated compiler serving` message or GET `/health`. Initial trusted dependency-cache preparation can take time. In a second terminal:

    rustx build --target web --release --compiler-url http://127.0.0.1:3002
    rustx serve dist 3000

Use **127.0.0.1**, not an interchangeable localhost spelling: CORS admission is exact-origin. Open `http://127.0.0.1:3000/#/playground`. The main editor/feedback/templates/layout are authored in RustX. Change the counter initializer, compile, and observe matching interactive output. Select the API template and load the actual bundled public fixture. Type an invalid Rust type to see main.rustx line/column diagnostics; Go to error selects that source position. Reset/templates invalidate older work. Explicit cancel deletes the job asynchronously and rejects late results. Unmount drops scoped polling/requests; the bounded server job may finish before TTL cleanup.

## Isolation and resources

ADR-012 was written before accepting visitor source. Only a source string up to 32 KiB is accepted; visitors cannot provide manifests/dependencies/build scripts. Locked trusted dependencies are prepared separately. Real rustc builds a Wasm cdylib with a trusted CPU-only host wrapper; wasm-bindgen 0.2.100 produces browser bindings. Visitor native executables are never invoked. JavaScript/module snippet outputs are rejected before publication.

macOS uses deny-default sandbox-exec (Seatbelt); Linux has a bubblewrap provider that is source-only/unexecuted in this delivery. Missing/failed isolation disables the service. Windows compiler isolation is unavailable and fails closed. Toolchain/cache are read-only, each job has a private writable directory, environment is cleared and compiler network access is denied. Actual rustc denial of host files, another job's source and inherited HOME is tested. That is specific evidence, not an exhaustive sandbox-escape audit.

Limits: two concurrent jobs; eight retained; cryptographically random 128-bit job IDs; five-minute TTL; 30-second total compile/bind wall time; 60 CPU seconds per subprocess; 1.5 GiB process-tree memory threshold; 32 MiB output; 64 KiB diagnostics; 128 file descriptors. macOS checks RSS every 20 ms because RLIMIT_AS/DATA were unavailable: this is sampled termination, not a hard instantaneous memory ceiling. Linux's address-space limit has not been exercised here. Cancel/timeout stops the process group. A one-second janitor expires jobs; restart removes only this service port's private store. Do not run several compiler-load suites against the same two-job service.

The compiled app runs in a dedicated worker inside an opaque-origin `sandbox="allow-scripts"` iframe, with no same-origin permission. A classic blob bootstrap dynamically imports the module graph because Chrome/WebKit opaque module-worker loading differs. The blob remains alive until teardown. The worker has a 256 MiB Wasm memory ceiling, 1280×720 output bound, bounded event queue and 2.5-second response watchdog. Infinite setup loops are terminated without blocking the main editor. Browser/JS/process memory is not bounded solely by the Wasm ceiling. This is a CPU preview with a finite asynchronous input bridge, not a GPU/IME/AT parity certification for arbitrary apps.

Only the dedicated origin's public GET `/api/users` is offered by the trusted preview HTTP adapter. Clipboard, files, storage, navigation, sockets, stream, print and timers are unavailable. Main app credentials and DOM are inaccessible. CSP bounds network/script access to the dedicated preview origin, which must contain **no authenticated app, cookies or secrets**. Wasm code can call available Web APIs; the origin/CSP boundary is therefore essential in addition to the typed capability report. Job status/compile/cancel accept only the configured application origin; public generated artifacts are deliberately readable without credentials. Random IDs are not authentication for a confidential hosted product.

## Deployment and cost

This delivery runs only on loopback, with no public hosting or spending. The service requires the pinned Rust toolchain, matching bindgen executable, trusted framework source/cache and a tested OS sandbox provider. It uses local CPU, memory and disk within the stated limits; there is no third-party compile API bill. A hosted offering needs separate authorization and operational review: dedicated origin/host/container/VM isolation, HTTPS, authentication/admission/rate controls, tenant separation, abuse/DoS mitigation, log/source retention/privacy policy, monitoring, updates and cost ownership. The local memory monitor and tests do not certify public multi-tenant security.

The canvas homepage links this playground and the separate optional documentation mirror. Existing homepage presets remain labelled precompiled. They are not used as compiler success fallbacks. Compiler/network/build/runtime failures are reported explicitly. Exact real browser/editor/compiler results and missing Windows/mobile/human tasks are in ACCEPTANCE.md.

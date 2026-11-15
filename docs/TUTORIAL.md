# Build a RustX application

RustX draws application controls in a Rust-owned canvas scene. Browser HTML/JavaScript are the bootstrap and platform bridge. Transparent semantic/input controls supply browser accessibility and IME integration; they do not determine visible layout. Windows uses the same source with a native host. See ACCEPTANCE.md for the actual tested support matrix and open platform gates.

## Install and create

Install Rust 1.90.0, then from the supplied source:

    cargo install --path crates/rustx-cli --bin rustx --locked
    rustup target add wasm32-unknown-unknown
    cargo install wasm-bindgen-cli --version 0.2.100 --locked
    rustx doctor
    rustx new my-app
    cd my-app
    rustx dev --target web

Open http://127.0.0.1:3000. The generated project vendors complete framework source and lockfiles. Edit src/app.rustx, save, and the development server rebuilds and reloads. Build errors remain visible without replacing the last working bundle. Full page reload resets application state; state-preserving hot reload is not provided. Node is optional test/export tooling. Python and Docker are not normal build/runtime dependencies.

## Authoring and state

.rustx files contain ordinary valid Rust modules. Markup is inside rustx!; uppercase tags resolve to typed framework widgets or imported Rust components. Setup executes once per mount. Signals and explicit closures update retained nodes. Key changes remount a component; stable keys preserve identity.

    use rustx::prelude::*;
    pub fn app(cx: Scope) -> View {
        let count = cx.signal(0);
        let shown = count.clone();
        rustx! {
            <Column>
                <Text>{move || format!("Count: {}", shown.get())}</Text>
                <Button on_click={move |_| { let _ = count.update(|n| *n += 1); }}>"Increase"</Button>
            </Column>
        }
    }

Never perform a network request as a rendering side effect. Use the scoped HttpClient/TaskQueue services from context and start work from setup or an event. Requests and resource observers belong to scopes; canceled/stale results cannot overwrite a newer generation. HTTP status is a response until error_for_status is requested. See STAGE3_SERVICES.md for methods, streaming, sockets, query cache, routes, storage and limits.

## Edit, check and inspect

Install the supplied RustX VSIX and rust-analyzer in VS Code. Set rustx.path if the CLI is not on PATH. .rustx uses the Rust language ID. RustX adds markup formatting, component/prop completion and source-linked compiler checks; rust-analyzer supplies normal Rust semantic assistance. Run RustX: Check project after editing. The actual editor smoke evidence is separate from a provider mock.

    rustx fmt
    rustx fmt --check
    rustx check
    rustx test

Use rustx dev or build with --devtools to enable the bounded inspector. Select a node to inspect its owner/source, Rust rectangle, semantic state and resource metrics. Profile captures show Rust reactive/layout/scene CPU work and event paths, plus host frame timings. These are not GPU execution or physical presentation times. Passwords and request contents are redacted. Production builds leave diagnostics disabled.

## Build and host

    rustx build --target web --release
    rustx serve dist 3000

The bundle includes a content-hash manifest, hashed executable assets, gzip copies and an offline application shell. Serve Wasm as application/wasm. Hashed files are immutable; HTML/config/manifests must revalidate. Use HTTPS outside localhost. History routes require extensionless SPA fallback; hash routes work without rewrites. Root hosting is the current path contract. The offline shell never fabricates or caches network API success. Keep CSP connect-src/frame-src aligned with explicitly configured API/compiler origins. No cross-origin isolation is required for the default single-thread runtime.

On a real Windows/MSVC runner:

    rustx build --target windows --release

Run and verify the resulting executable on clean Windows without Rust or a browser. Compilation and CI configuration are not native interaction, DPI, IME or UI Automation evidence.

## Playground

The main framework demo includes a Rust-authored source editor. Start the dedicated compiler service in another terminal:

    rustx playground --port 3002 --app-origin http://127.0.0.1:3000
    rustx build --target web --release --compiler-url http://127.0.0.1:3002
    rustx serve dist 3000

Open #/playground. Counter and API templates are editable source, compiled by real rustc and executed as Wasm in an isolated worker/frame. Compile and runtime errors are real. The preview receives only a public fixture GET capability. Missing compiler isolation fails closed. See stage4/PLAYGROUND-ADR.md and stage4/PLAYGROUND.md for limits, setup and security boundaries. No public compiler service has been deployed.

## Migrating from React/HTML

Replace JSX functions with Rust components and typed props; replace hooks with scoped signals/memo/effects. Move visible CSS geometry into typed Style values. Replace DOM events with synchronous typed Rust events and native/browser services. HTML/SVG/React libraries do not import unchanged. Use explicit Link, form, semantic labels and error relationships. The canvas does not reproduce browser Find, translation, document selection or SEO behavior. This optional indexed documentation mirror is separate from the RustX-authored visual application. No performance advantage is claimed without matched workloads and actual measurements.

## Scoped context and reactive providers

Use `cx.provide(value)` to make a typed value available to descendants. `cx.context::<T>()` clones the nearest value of that exact type; an inner provider shadows its ancestors without affecting siblings. Wrapper types let you provide different values with the same underlying type. Context does not cross independent Runtime roots.

Provide a signal when descendants need reactive updates:

```rust
use rustx::prelude::*;

let mut app = Runtime::new(|cx| {
    let count = cx.signal(0u32);
    cx.provide(count);
    Node::component("readout", |cx| {
        let count = cx.context::<Signal<u32>>().expect("count provider");
        dynamic(move || count.get().to_string())
    })
});
app.flush();
assert_eq!(app.snapshot().text_content(), "0");
```

Replacing a plain provided value does not schedule an update or rerun component setup. Replacing a provided signal also does not redirect a signal already held by a consumer; mutate the original signal for shared updates. A later lookup observes the replacement. Only tracked signal reads subscribe; looking up or cloning a Signal handle does not subscribe by itself.

On owner removal, retained Scope handles report `is_alive() == false` and context lookup returns None. Retained signals return `Err(Disposed)` from `try_get`/`set`; `get` panics on a disposed signal. Providing into a disposed scope is a programming error and panics with `disposed scope`. Cleanup occurs child-before-parent; callback order within one owner is unspecified. See ADR-013 in DECISIONS.md for context resource destruction and reentrant Clone/Drop behavior. These contracts do not imply React-style reactive provider replacement or a cross-runtime external-store subscription API.

## External state snapshots (RP-006)

Use `ExternalStore<T>` when a host/data adapter owns state outside component lifetimes. It is single-threaded and independent of Runtime; clone it into multiple roots. Publish whole logically immutable values (`PartialEq` must be pure; do not mutate snapshot values through interior mutability). Equal values do not notify or advance revision. Component setup subscribes once; reactive views read the subscription:

```rust
let store = ExternalStore::new((0u32, 0u32));
let source = store.clone();
let mut runtime = Runtime::new(move |cx| {
    let subscription = source.subscribe(&cx).expect("subscription capacity");
    dynamic(move || {
        let snapshot = subscription.get();
        format!("{} / {}", snapshot.value.0, snapshot.value.1)
    })
});
store.publish((1, 2)).unwrap(); // host event, outside a commit
runtime.flush();
assert_eq!(runtime.snapshot().text_content(), "1 / 2");
```

`snapshot()` is an untracked read. `StoreSubscription::get()` tracks the ordinary RustX signal dependency. All subscribers receive the same revision before any runtime is woken. Each runtime wakes at most once per publication; pending notifications coalesce through its existing dirty set. This is per-store coherence, not a transaction across separate stores.

Scope disposal unsubscribes even if the application retains the handle. Dropping a handle alone does not unsubscribe: the scope owns it. `disconnect()` is idempotent; later `try_get()` fails. Signal storage remains until scope disposal. `replace_source(&other)` reuses the signal and cleanup registration and immediately exposes the new snapshot. Failure preserves the old source. A store admits at most `MAX_STORE_SUBSCRIPTIONS` (64); admission returns `Capacity` rather than growing indefinitely.

Changed publication during a subscribed runtime's mount/flush returns `ExternalStoreError::Busy` without changing any snapshot. Reentrant publication also returns Busy. Replacement during commit/publication is rejected. Handle Busy by retrying from a later host event/task after the commit; never spin inside an effect. There is no automatic retry queue. Equal publication outside reentrant publication returns `Ok(false)`, including during a commit. `Disposed`, `Capacity` and checked identifier/revision `Exhausted` are explicit errors. This push adapter does not yet implement arbitrary third-party getSnapshot/subscribe callbacks, server snapshots/hydration, multithreaded publication or React hook compatibility.

Run the public-API canvas example after building the checkout:

```sh
bash scripts/build-web.sh
target/debug/rustx serve dist 3000
# Open http://127.0.0.1:3000/?renderer=cpu#/store
# In another terminal, exercise actual browser pointer interaction:
node scripts/test-external-store-browser.mjs
# Native Mac window (close normally when finished):
RUSTX_DEMO=store RUSTX_RENDERER=cpu cargo run -p rustx-lab --bin rustx-lab --locked
```

Publish, switch sources, unmount the reader, publish again and remount. The displayed pair remains coherent and remount reads the latest primary snapshot. Rust owns the layout/text and canvas pixels; this example adds no DOM UI or core browser dependency. See ADR-014 and the RP-006 evidence ledger for verification limits.

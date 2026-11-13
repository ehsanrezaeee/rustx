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

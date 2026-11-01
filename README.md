# RustX — Stage 1 foundation

Rust-authored components, WebAssembly and a Rust-owned canvas engine. This is an experimental foundation, not an accepted Stage 1 or production release. See `docs/BLUEPRINT.md` for the complete contract and `docs/NEXT_AGENT.md` to continue.

## Run the prebuilt demo

Install Rust through rustup. Extract this entire folder, open a terminal inside it, then:

```sh
cargo run -p rustx-cli --locked -- serve dist 3000
```

Open **http://127.0.0.1:3000/?renderer=cpu**. The Wasm bundle is included; previewing it needs no Node or Wasm rebuild. First-time Cargo/toolchain setup needs internet. Do not open index.html as a file URL. The fixture server binds loopback and is for development, not production hosting.

Explore **Overview**, **People explorer** and **Runtime lab**. Load the local sample first; JSONPlaceholder makes a real external request and can fail with network/service availability. The homepage playground runs precompiled Rust components. An arbitrary-source editor/compiler is a final-release requirement, not implemented yet.

Remove `?renderer=cpu` to try WebGPU with initial CPU fallback. Read the compatibility matrix: compilation does not certify a real browser/GPU/Windows session.

## Edit and rebuild

App source: `examples/lab/src/app.rustx`.

Bash (Linux/macOS/Git Bash):

```sh
bash scripts/build-web.sh
cargo run -p rustx-cli --locked -- serve dist 3000
```

Windows PowerShell:

```powershell
./scripts/build-web.ps1
cargo run -p rustx-cli --locked -- serve dist 3000
```

Rust 1.90.0 is pinned. Scripts install the Wasm target and wasm-bindgen CLI 0.2.100 when needed. Refresh after rebuilding; hot reload is not implemented. Python is not a project build/runtime dependency. The only current CLI command is `serve`; blueprint commands such as `rustx new` are future work.

## Native Windows

On Windows with Rust/MSVC and Visual Studio C++ build tools:

```powershell
./scripts/build-windows.ps1
./dist/windows/rustx-lab.exe
```

Set `$env:RUSTX_RENDERER='cpu'` to force CPU mode. For local API fixtures keep the server running on port 3000 in another terminal. No Windows executable is included/certified; run and verify it on actual Windows. Linux native compilation is useful but not Windows evidence.

## Authoring

```rust
use rustx::prelude::*;
#[component]
fn Counter(cx: Scope, #[prop(default = 0)] initial: i32) -> View {
    let count = cx.signal(initial);
    let shown = count.clone();
    rustx! {
        <Column style={Style::column().gap(12.)}>
            <Text>{move || format!("Count: {}", shown.get())}</Text>
            <Button on_click={move |_| { let _ = count.update(|n| *n += 1); }}>
                {"Increase"}
            </Button>
        </Column>
    }
}
```

Setup runs once per mount. Plain props are initial immutable configuration; pass `Signal<T>` props/read them in closures for updates, or change a key to remount. Clone signals into separate closures. Import both `Name` and generated `NameProps` for a custom component. `children: Vec<View>` accepts nested markup; a `View` prop is a named slot. Fragments use `<>...</>`.

`.rustx` is ordinary Rust included with `#[path = "app.rustx"] mod app;`, with markup inside `rustx!`. A compiler fork, bare JSX and first-class editor/formatter integration are not implemented. Builtins: Div, Row, Column, Text, Button, Input, Image, Canvas. HTML/React/CSS/npm component compatibility is not automatic.

HTTP code stays in Rust: `HttpClient::get(&cx, url, callback)`. The browser JS host uses Fetch/AbortController and returns the result to the live owning scope. Current limits: GET only, 10 seconds, 1 MiB body, 16 host requests. Streaming/upload/WebSocket/cache and general service capabilities come later.

## Tests

```sh
cargo test --workspace --locked
cargo fmt --all -- --check
npm ci
npm run test:wasm
npm run test:http
```

Rebuild Wasm before testing changed Rust. `test:wasm` executes actual compiled Wasm in Node and writes CPU PNG evidence; it is not a browser test. `test:http` also checks the real Rust fixture server → production JS Fetch adapter → compiled Wasm path. With the fixture server running in another terminal:

```sh
npx playwright install chromium
npm run test:browser
```

Real IME/screen-reader tasks remain separate. Shared scene probes:

```sh
cargo run -p rustx-lab --example render_probe --locked -- docs/evidence
cargo run -p rustx-lab --example render_probe --locked -- docs/evidence --gpu
```

The second command requires an adapter. Software graphics adapters exercise GPU APIs but do not establish hardware performance or browser presentation.

## Layout

| Path | Responsibility |
| --- | --- |
| crates/rustx-core | Scoped runtime, styles, tree, semantics and GET contract |
| crates/rustx-macros, crates/rustx | Component/markup compiler and facade |
| crates/rustx-engine | Layout, shaping, scene, CPU/native adapters |
| crates/rustx-web, web | Wasm/browser rendering, semantics, input and Fetch |
| crates/rustx-cli | Fixture/static dev server |
| examples/lab | Homepage, API example and runtime lab |
| docs | Blueprint, acceptance ledger, decisions and continuation |

Adapted rendering modules from the supplied RustUI Step 2 project; rewrote the runtime/authoring spine. Previous completion claims are not inherited. LICENSE and font notices are included. `SOURCE_MANIFEST.sha256` and `SOURCE-REVISION.txt` identify this snapshot. Build/dependency caches are excluded.

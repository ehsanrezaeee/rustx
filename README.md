# RustX — Stage 2 shared visual engine

Rust-authored components, WebAssembly and a Rust-owned canvas engine. Visual engine adds responsive themes, Unicode/emoji fallback, a 10,000-object drawing board and a 100,000-record virtual explorer. Stage 1 platform/manual and Stage 2 full target acceptance remain open. See `docs/BLUEPRINT.md`, `docs/ACCEPTANCE.md` and `docs/STAGE2.md`.

## Git history and review

The local repository has the original import on `main` and this work on `stage-1-completion`. No remote has been created. Review the changes with:

```sh
git log --oneline --decorate
git diff main...stage-1-completion
git status
```

The archives under `releases/` are historical Stage 1 snapshots. This Stage 2 continuation uses the current source and rebuilt `dist/`; no new archive was requested. To inspect its Git history in a separate directory, run `git clone rustx-history.bundle ../rustx-history`. Regenerate an archive from a clean committed tree with `npm run package`; Node is only packaging/test tooling. Source content identity is recorded by `npm run snapshot` and the source manifest.

## Run the prebuilt demo

Install Rust through rustup. Extract this entire folder, open a terminal inside it, then:

```sh
cargo run -p rustx-cli --locked -- serve dist 3000
```

Open **http://127.0.0.1:3000/?renderer=cpu**. The Wasm bundle is included; previewing it needs no Node or Wasm rebuild. First-time Cargo/toolchain setup needs internet. Do not open index.html as a file URL. The fixture server binds loopback and is for development, not production hosting.

Explore **Overview**, **People explorer**, **Runtime lab** and **Visual engine**. The new page supports theme switching, pointer/keyboard drawing gestures, camera controls and virtual-record navigation. Runtime lab now has editable tasks whose text, counter state and identity survive keyed reorder and reset on unmount. Load the local sample first; JSONPlaceholder makes a real external request and can fail with network/service availability. The homepage playground runs precompiled Rust components. An arbitrary-source editor/compiler is a final-release requirement, not implemented yet.

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

The host negotiates version 1 and its GET/cancel capability/limits before mounting the lab. Custom hosts connect `HttpClient` with a `HostCapabilities` report before mounting request-producing components; see `docs/HOST_PROTOCOL.md`. HTTP code stays in Rust: `HttpClient::get(&cx, url, callback)`. The browser JS host uses Fetch/AbortController and returns the result to the live owning scope. Current limits: GET only, 10 seconds, 1 MiB body, 16 host requests, 32 coalesced queued commands, and 8 KiB URLs. Streaming/upload/WebSocket/cache and general service capabilities come later.

## Tests

```sh
cargo test --workspace --locked
cargo fmt --all -- --check
npm ci
npm run test:host
npm run test:wasm
npm run test:http
npm run test:stage2
```

Rebuild Wasm before testing changed Rust. `test:wasm` executes actual compiled Wasm in Node and writes CPU PNG evidence; it is not a browser test. `test:http` also checks the real Rust fixture server → production JS Fetch adapter → compiled Wasm path. With the fixture server running in another terminal:

```sh
npx playwright install chromium firefox webkit
npm run test:browser
RUSTX_BROWSER=firefox npm run test:browser
RUSTX_BROWSER=webkit npm run test:browser
```

The final Mac CPU suite runs in installed Chrome with `CHROME_PATH='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' npm run test:browser`. The Chromium download CDN returned a regional 403; using an installed browser avoids that download. Real Firefox/WebKit availability is recorded in the ledger, and WebKit does not certify Safari. Physical browser GPU checks use `RUSTX_HEADED=1 CHROME_PATH='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' npm run test:gpu`. Real IME/screen-reader tasks remain separate; follow `docs/PLATFORM_CHECKS.md`. Shared scene probes:

```sh
cargo run -p rustx-lab --example render_probe --locked -- docs/evidence/current-probe
cargo run -p rustx-lab --example render_probe --locked -- docs/evidence/current-probe --gpu
```

Compare the two outputs with `npm run test:probe` after rendering to `docs/evidence/current-probe`. The second command requires an adapter. Software graphics adapters exercise GPU APIs but do not establish hardware performance or browser presentation.

## Stage 2 checks

With the fixture server running and Wasm rebuilt:

```sh
CHROME_PATH='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' npm run test:stage2:browser
RUSTX_BROWSER=webkit npm run test:stage2:browser
RUSTX_TEST_GPU=1 RUSTX_HEADED=1 CHROME_PATH='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' npm run test:stage2:browser
cargo run -p rustx-lab --example stage2_probe --locked -- docs/evidence/stage2-probe --gpu
npm run test:stage2:probe
```

The GPU suite destroys the active device, checks CPU recovery with retained state, then retries WebGPU. The native probe compares matched CPU/Metal output; limits are in the ledger. Open the native visual demo directly on this Mac with `RUSTX_DEMO=engine cargo run -p rustx-lab --bin rustx-lab --locked`; add `RUSTX_RENDERER=cpu` for software rendering. Windows uses the same source with documented prerequisites and still needs actual verification.

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

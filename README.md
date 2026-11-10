# RustX — Stage 3 applications

Rust-authored components, WebAssembly and a Rust-owned canvas engine. Stage 3 adds events, widgets, Unicode editing, forms, semantics, HTTP/resources, routing and platform services to the shared visual engine. Work is on `stage-3`.

The full source workspace, lockfiles, scripts, fixtures and rebuilt `dist/` browser application are included. Stage acceptance remains open for actual Windows, real OS IME, human screen-reader and the complete target matrix. [ACCEPTANCE.md](docs/ACCEPTANCE.md) records what was actually tested; [STAGE3.md](docs/STAGE3.md) describes the APIs and limits.

## Run the browser application

From this directory, with Rust installed:

```sh
cargo run -p rustx-cli --locked -- serve dist 3000
```

Open **http://127.0.0.1:3000/?renderer=cpu#/applications**. Previewing the rebuilt demo needs no Node or Wasm rebuild. Remove `?renderer=cpu` to request WebGPU with CPU fallback. The fixture server binds loopback and is for development.

Applications contains a Persian/English profile form, async validation, password/multiline editing, preferences, modal confirmation, local drafts, file import/export and connected service operations. People Explorer provides fixture/live HTTP, search, details, copy and printable export. Visual Engine provides themes, 100,000 virtual records and a 10,000-object drawing board: pick an object, enable editing, then drag/use arrows; CSV export includes all board rectangles. Overview and Runtime Lab retain the original counter/keyed lifecycle examples.

Routes use `#/`, `#/people`, `#/lab`, `#/engine` and `#/applications`, including back/forward and reload. The homepage playground remains precompiled; arbitrary-source compilation is Stage 4 work.

## Edit and rebuild

```sh
bash scripts/build-web.sh
```

Windows PowerShell:

```powershell
./scripts/build-web.ps1
```

Rust 1.90.0, wasm-bindgen 0.2.100 and dependency lockfiles are pinned. Scripts install missing target/binding tooling. App source is in `examples/lab/src/*.rustx`; markup remains inside `rustx!`. Setup runs once per mount, while closures and signals update retained views. Python is not a project build/runtime dependency.

## Native application

```sh
RUSTX_DEMO=applications cargo run -p rustx-lab --bin rustx-lab --locked
```

Set `RUSTX_RENDERER=cpu` to force software rendering. Keep the fixture server on port 3000 for native relative API URLs. Native clipboard/file/print utilities and their errors are described in [STAGE3_SERVICES.md](docs/STAGE3_SERVICES.md).

On Windows with Rust/MSVC and Visual Studio C++ build tools:

```powershell
./scripts/build-windows.ps1
$env:RUSTX_DEMO='applications'
./dist/windows/rustx-lab.exe
```

AccessKit supplies the native semantic adapter. Actual Windows launch, UI Automation and NVDA tasks remain unverified; compilation or Mac execution does not certify them. Follow [PLATFORM_CHECKS.md](docs/PLATFORM_CHECKS.md).

## Verify

```sh
cargo test --workspace --locked
cargo fmt --all -- --check
npm ci
npm run test:host
npm run test:wasm
npm run test:http
npm run test:stage2
npm run test:stage3:services
```

With the fixture server running, and Wasm rebuilt after Rust edits:

```sh
CHROME_PATH='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' npm run test:stage3:browser
RUSTX_BROWSER=webkit npm run test:stage3:browser
npm run test:stage2:browser
```

Omit `CHROME_PATH` when using a Playwright-installed Chromium. `RUSTX_URL` overrides the server origin. WebKit automation does not establish actual Safari support. Physical browser GPU recovery checks use `RUSTX_TEST_GPU=1 RUSTX_HEADED=1` with `test:stage2:browser`. Native fixture streaming/WebSocket tests are explicitly runnable:

```sh
cargo test -p rustx-engine --locked actual_native_pull_stream_and_socket_echo -- --ignored --nocapture
```

Manual IME/screen-reader tasks stay separate from injected text and semantic metadata. No universal performance or production-readiness claim is made.

## Source map

| Path | Responsibility |
| --- | --- |
| `crates/rustx-core` | Scoped runtime, events, widgets, editing/forms, services, resources/routes |
| `crates/rustx-macros`, `crates/rustx` | Component/markup compiler and public facade |
| `crates/rustx-engine` | Layout, shaping, GPU/CPU, native services and AccessKit |
| `crates/rustx-web`, `web` | Wasm/browser rendering, input, semantic projection and adapters |
| `crates/rustx-cli`, `fixtures` | Loopback static/HTTP/stream/WebSocket fixture server |
| `examples/lab` | Shared-source reference applications |
| `docs` | Blueprint, API guides, evidence, limitations and handoff |

`SOURCE_MANIFEST.sha256` and `SOURCE-REVISION.txt` identify source content. `BUILD-INFO.json` identifies the rebuilt Wasm. `npm run snapshot` regenerates source identity; `npm run package` packages a clean committed workspace. Dependency/build caches are excluded. No remote deployment or publication was performed. See [NEXT_AGENT.md](docs/NEXT_AGENT.md) to continue.

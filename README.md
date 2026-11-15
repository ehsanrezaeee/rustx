# RustX

RustX is an experimental declarative UI framework for writing interactive applications in Rust. Compose typed components with `rustx!` markup, manage state in Rust, and compile to WebAssembly for the browser or a shared native application. The browser is the primary target; native Windows is the second target.

Rust owns component identity, reactive state, layout, text shaping, editing, hit testing and the rendered scene. A canvas displays the UI through WebGPU or a CPU renderer. A small browser host connects platform features such as input, accessibility semantics, networking, storage and clipboard. Component setup runs once per mount; reactive updates change retained state and views.

## Goals and current scope

- Let developers build applications with typed Rust components and styles, without authoring HTML, CSS or JavaScript for ordinary application UI.
- Share application logic and the layout/text/rendering engine between browser and native targets.
- Support practical applications: dashboards, API clients, forms, virtual collections and interactive drawing tools.
- Provide a complete development workflow: scaffolding, formatting, diagnostics, source watching, inspection, production web builds and an optional isolated Rust playground.
- Measure rendering, startup and resource use against explicit acceptance criteria.

This is a Stage 4 implementation candidate, **not an accepted v1 release**. Full Windows interaction and clean-machine installation, real OS IME, human screen-reader tasks, physical mobile/Safari coverage, the strict GPU frame budget and dependency findings remain open. See the [acceptance ledger](docs/ACCEPTANCE.md) and [support matrix](docs/stage4/SUPPORT.md) for evidence and limits.

## Run from a GitHub checkout

Install Git and Rust through rustup, including the native linker/build tools for your OS. macOS needs Xcode Command Line Tools; Windows needs the MSVC Rust toolchain and Visual Studio C++ Build Tools. Linux builds of the native workspace need its X11 development libraries. Initial setup needs network access to download the pinned toolchain and locked dependencies. Node is optional for application builds; contributor tests and packaging use Node 24 and npm.

```sh
git clone https://github.com/ehsanrezaeee/rustx.git
cd rustx
rustup toolchain install 1.90.0 --profile minimal --component rustfmt --target wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.100 --locked
```

The repository pins Rust 1.90.0 in `rust-toolchain.toml`. The wasm-bindgen CLI must match the locked Rust crate version, 0.2.100.

Build the demo on macOS/Linux or Git Bash:

```sh
bash scripts/build-web.sh --compiler-url http://127.0.0.1:3002
cargo run -p rustx-cli --locked -- serve dist 3000
```

In Windows PowerShell, use the equivalent build script:

```powershell
./scripts/build-web.ps1 -BuildArguments @('--compiler-url', 'http://127.0.0.1:3002')
cargo run -p rustx-cli --locked -- serve dist 3000
```

Wait for the build and server startup, then open **http://127.0.0.1:3000/?renderer=cpu**. Remove `?renderer=cpu` to request WebGPU with CPU fallback. The server binds loopback. First builds can take several minutes. The application works without the optional compiler service; live playground compilation requires the service described below.

**A clone contains source, not prebuilt bundles.** `dist/`, `dist-release/`, `dist-bench/`, `dist-native/`, `target/`, `node_modules/`, `.rustx/` and `releases/` are generated and ignored by Git. Running `serve dist` before building cannot launch the application. The scripts above recreate `dist/` from tracked source, fonts, fixtures and lockfiles. The generated `crates/rustx-cli/assets/framework.bin` is needed only when packaging the standalone CLI; checkout builds embed the tracked SDK source directly. No ignored delivery files need to be copied from another machine.

## Explore the demo

The homepage links these RustX-authored applications:

| Route | What it demonstrates |
| --- | --- |
| `#/capabilities` | Application ideas, working examples and current limits |
| `#/people` | API data explorer, search, details, loading/error/cancel states, copy and printable export |
| `#/applications` | Persian/English forms, validation, password/multiline editing, dialogs, drafts and files |
| `#/engine` | Themes, 100,000 virtual records and a 10,000-object drawing board with pan/zoom/export |
| `#/playground` | Editable Rust source, real Wasm compilation and source diagnostics with the optional compiler |

People Explorer can use deterministic fixtures or the public JSONPlaceholder API; the live API needs internet access.

## Develop and create applications

After installing the prerequisites above, rebuild and reload this demo as source changes:

```sh
cargo run -p rustx-cli --locked -- dev --target web --compiler-url http://127.0.0.1:3002
```

Stop any server already using port 3000 first, or select another port with `--port`. `dev` builds the initial bundle, watches source and reports compilation errors while retaining the last working bundle.

To install the CLI and create a separate application:

```sh
cargo install --path crates/rustx-cli --bin rustx --locked
rustx doctor
rustx new my-app
cd my-app
rustx dev --target web
```

Edit `src/app.rustx` in the generated project. The CLI carries the source SDK and vendors framework source, fonts, notices and locks into the application, so it does not depend on the original checkout's path. `doctor` reports tool availability; install any missing tools before building.

```sh
rustx fmt
rustx check
rustx test
rustx build --target web --release
rustx serve dist 3000
```

Development builds enable the inspector and bounded event/layout profiler. Production builds disable developer APIs by default. To create a production bundle of this repository's demo, run from the repository root:

```sh
cargo run -p rustx-cli --locked -- build --target web --release --out dist-release
cargo run -p rustx-cli --locked -- serve dist-release 3011
```

See the [developer tutorial](docs/TUTORIAL.md), [tooling guide](docs/STAGE4.md) and [deployment guide](docs/stage4/DEPLOYMENT.md). The offline shell does not cache API data or provide offline compilation.

## Optional local Rust playground

Keep the demo server running on port 3000. In a second terminal at the repository root:

```sh
cargo run -p rustx-cli --locked -- playground --port 3002 --app-origin http://127.0.0.1:3000
```

Wait for `isolated compiler serving`, then open `http://127.0.0.1:3000/?renderer=cpu#/playground`. Use that exact application origin; if you change it, change `--app-origin` too. Initial trusted compiler preparation can take time.

Compilation requires the pinned toolchain, dependencies and a functioning OS sandbox. macOS Seatbelt is the locally exercised provider; the Linux bubblewrap provider remains unverified, and Windows compiler isolation is unavailable. Missing isolation fails closed. Compiled output runs in an opaque iframe and bounded worker. See the [playground guide](docs/stage4/PLAYGROUND.md) for resource bounds and isolation limits.

## Native application

From the repository root on macOS/Linux:

```sh
RUSTX_DEMO=applications cargo run -p rustx-lab --bin rustx-lab --locked
```

In PowerShell:

```powershell
$env:RUSTX_DEMO = 'applications'
cargo run -p rustx-lab --bin rustx-lab --locked
```

Set `RUSTX_RENDERER=cpu` to force software rendering. Keep the demo fixture server on port 3000 for relative native API URLs. Windows x64/MSVC packaging uses `scripts/build-windows.ps1`. Native startup evidence and remaining GPU/DPI/services/UIA/NVDA/IME tasks are documented in [PLATFORM_CHECKS.md](docs/PLATFORM_CHECKS.md).

## Contributor checks and editor tools

Install Node 24, then run from the repository root after building the demo:

```sh
npm ci
npm run format:check
cargo fmt --all -- --check
cargo test --workspace --locked
npm run test:host
npm run test:wasm
npm run test:stage2
npm run test:http
npm run test:stage3:services
```

Stop the demo server before `test:http` (it starts its own server on port 3000), or set `RUSTX_TEST_PORT` to a free port. Browser tests require a real browser installation; install the test engines with `npx playwright install`. Detailed browser, compiler, deployment and benchmark commands are in [NEXT_AGENT.md](docs/NEXT_AGENT.md). Automated checks do not certify OS IME or human assistive-technology use. Python and Docker are not project build/runtime dependencies.

Build the optional VS Code extension with `npm run package:editor`, then install the generated `releases/rustx-editor-0.1.0.vsix` and rust-analyzer. See [editor instructions](editors/vscode/README.md). To package the standalone CLI crate, first generate its SDK archive with `node scripts/prepare-cli.mjs`, then run `cargo package -p rustx-cli --locked`. Delivery archives may contain prebuilt bundles and tools; those are separate from a source checkout. Existing build manifests and evidence identify those historical deliveries, not newly rebuilt artifacts.

## Source map and license

| Path | Purpose |
| --- | --- |
| `crates/rustx-core` | Reactive runtime, typed components, styles, events and services |
| `crates/rustx-macros` | Rust component and `rustx!` markup macros |
| `crates/rustx-engine` | Retained layout, text, scenes, CPU/GPU rendering and native host |
| `crates/rustx-web` | Rust browser bindings |
| `crates/rustx-cli` | Scaffolding, formatting, builds, dev server and isolated compiler |
| `examples/lab` | Shared demo application source |
| `web`, `playground` | Browser platform bridges and compilation preview host |
| `docs`, `scripts` | Architecture, acceptance evidence and reproducible tooling |

Read the [blueprint](docs/BLUEPRINT.md) for the design contract and [status](docs/STATUS.md) for implementation progress.

MIT © 2026 ehsanrezaeee. See [LICENSE](LICENSE); bundled fonts and dependencies retain their separate notices in [NOTICE.md](NOTICE.md).

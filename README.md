# RustX — Stage 4 delivery candidate

Rust-authored components, WebAssembly, and a Rust-owned canvas engine. This delivery adds the project CLI, `.rustx` formatting and VS Code tools, inspector/profiler, hashed web bundles and offline shell, and an editable Rust compilation playground. Source code uses consistent indentation and spacing; `npm run format:check` checks Rust/RustX and the browser/tooling sources.

The complete source, locks, fixtures, tests, evidence, prebuilt browser bundles, local CLI package, editor extension and available dependency notices are included. [The four-stage review](docs/REVIEW.md) records the fixes, 134 passing Rust tests and successful Linux/macOS/Windows CI. [PR #2](https://github.com/ehsanrezaeee/rustx/pull/2) delivers this branch to main. **Stage 4 and v1 acceptance remain open:** human screen-reader/OS IME tasks, physical mobile/Safari support, Windows clean-machine/full interaction tasks, the GPU frame budget and dependency findings remain in [the acceptance ledger](docs/ACCEPTANCE.md). No public deployment was performed.

## Run the supplied browser application

From this directory:

```sh
cargo run -p rustx-cli --locked -- serve dist 3000
```

The delivery archive also includes a prebuilt Apple Silicon Mac CLI:

```sh
./releases/rustx-macos-arm64 serve dist 3000
```

Open **http://127.0.0.1:3000/?renderer=cpu**. Remove `?renderer=cpu` to request WebGPU with CPU fallback. The server binds loopback. Previewing the supplied bundle needs no Node or Wasm rebuild.

- [People Explorer](http://127.0.0.1:3000/?renderer=cpu#/people): live JSONPlaceholder or deterministic fixtures, search, person details, retry/error/empty/timeout/cancel/offline states, copy and printable export.
- [What can I build?](http://127.0.0.1:3000/?renderer=cpu#/capabilities): a RustX-rendered guide to dashboards, API clients, drawing tools and utilities, with supported building blocks, working examples and current limits.
- [Rust playground](http://127.0.0.1:3000/?renderer=cpu#/playground): edit counter/API source, compile to actual Wasm, run, inspect source-linked errors and reset. Start the separate compiler below.
- [Applications](http://127.0.0.1:3000/?renderer=cpu#/applications): Persian/English forms, validation, password/multiline editing, dialog, drafts, files and services.
- [Visual engine](http://127.0.0.1:3000/?renderer=cpu#/engine): themes, 100,000 virtual records and a 10,000-object drawing board with pan, zoom, editing and export.

The homepage explains the framework and links each example. Its older presets are labelled precompiled. Rust owns visible layout, text, editing and scene; transparent browser input/semantic controls remain the platform bridge. Setup runs once per mount.

## Run the real Rust playground

Keep the application server above running. In a second terminal:

```sh
cargo run -p rustx-cli --locked -- playground --port 3002 --app-origin http://127.0.0.1:3000
```

Wait for `isolated compiler serving`, then open the playground. The prebuilt bundles configure `http://127.0.0.1:3002`; use the exact application origin above. Initial trusted compiler preparation may take time. Compilation requires the pinned toolchain, dependencies and a functioning OS sandbox. The locally exercised provider is macOS Seatbelt; the Linux bubblewrap provider remains unverified and Windows compiler isolation is unavailable. Missing isolation fails closed. The native Windows application is a separate target.

Compiled output runs in an opaque iframe and a bounded worker with a public fixture GET capability. [The playground guide](docs/stage4/PLAYGROUND.md) explains setup, resource bounds, cancellation and isolation limits. No public compiler service or hosting bill is involved.

## Create, edit and build an application

```sh
cargo install --path crates/rustx-cli --bin rustx --locked
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.100 --locked
rustx doctor
rustx new my-app
cd my-app
rustx dev --target web
```

The CLI carries a source SDK. Generated projects vendor their framework source and do not depend on this checkout's absolute path. `.rs` and `.rustx` use valid Rust modules; markup lives inside `rustx!`. Rust 1.90.0 and lockfiles are pinned. Python and Docker are not project build/runtime dependencies; Node is optional formatting/test/package tooling.

Install the supplied `releases/rustx-editor-0.1.0.vsix` and rust-analyzer in VS Code. RustX supplies markup formatting, component/prop completions, navigation and a Cargo diagnostic command. See [the tutorial](docs/TUTORIAL.md) and [Stage 4 guide](docs/STAGE4.md).

```sh
rustx fmt
rustx check
rustx test
rustx build --target web --release
rustx serve dist 3000
```

Use `rustx dev` or `build --devtools` for the bounded inspector and event/layout profiler. Default production builds disable developer APIs. The archive's `dist/` enables inspection for the integration suites; `dist-release/` is the production bundle and `dist-bench/` is the separate instrumented lab. To rebuild this demo:

```sh
bash scripts/build-web.sh --compiler-url http://127.0.0.1:3002
```

[Deployment instructions](docs/stage4/DEPLOYMENT.md) cover hashes, MIME, Brotli/gzip, CSP, root hosting, deep links, offline caching and rollback. The offline shell does not cache API data or provide offline compilation.

## Native application

```sh
RUSTX_DEMO=applications cargo run -p rustx-lab --bin rustx-lab --locked
```

`RUSTX_RENDERER=cpu` forces software rendering. Keep the fixture server on port 3000 for relative native API URLs. On Windows x64/MSVC, run `scripts/build-windows.ps1`. The PR workflow also supplies `rustx-0.1.0-windows-x64.zip` in its `contracts-windows-2025` artifact, with native app/CLI, font notices, DLL report and checksums. An actual Windows runner passes CPU two-frame startup of both the build and packaged app. Review the recorded DLL report and remaining notice-copy gaps before redistribution. Clean Windows without development tools, GPU/DPI/services/UIA/NVDA and OS IME tasks remain in [PLATFORM_CHECKS.md](docs/PLATFORM_CHECKS.md).

## Verify and identify the delivery

```sh
npm ci
npm run format:check
cargo test --workspace --locked
npm run test:host
npm run test:wasm
npm run test:stage2
npm run test:http
npm run test:stage3:services
```

Browser, installed editor, compiler, deployment and benchmark replay commands are in [NEXT_AGENT.md](docs/NEXT_AGENT.md). Automated text and semantic assertions do not certify OS IME or human assistive-technology tasks. [The support matrix](docs/stage4/SUPPORT.md) names tested paths and missing devices. [The security report](docs/stage4/SECURITY.md) retains dependency advisories and missing license-copy findings.

`SOURCE-REVISION.txt` identifies source content; `BUILD-INFO.json` identifies each delivered bundle and tool artifact. The ZIP adds `GIT-REVISION.txt`, portable Git history and `ARTIFACT_MANIFEST.sha256`. After extraction, verify all payloads with `shasum -a 256 -c ARTIFACT_MANIFEST.sha256`. Build/dependency caches and credentials are excluded.

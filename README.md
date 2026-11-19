# RustX

RustX is an experimental UI framework for building browser canvas and native desktop apps from the same Rust source. It uses TSX-like `rustx!` markup for typed components and has its own runtime and renderer. Rust owns state, layout, text shaping, editing, and the scene. In the browser, WebAssembly draws the UI and a small JavaScript host connects it to browser APIs.

## Where RustX is today

The source-built demo includes mount-once components, reactive state, typed layout, editable forms, dialogs, virtual lists, canvas drawing, routing, and network requests. The CLI can scaffold an app, build the web or native target, and reload the web demo after Rust source edits.

| Target | Verified so far |
| --- | --- |
| Browser Wasm | Edge on Windows passed CPU canvas rendering, keyboard and text editing checks, live reload, and offline reload. Safari, Firefox, and physical mobile interaction are still open. |
| Windows native | The source-built release demo passed a CPU smoke run. Tab and Shift+Tab, off-screen focus reveal, and nested dialog restoration passed with UI Automation readback. |
| macOS native | An earlier native startup check passed. Multi-control keyboard and dialog interaction still need a native replay. |
| Linux native | Debian 12 amd64 passed the X11 CPU startup and keyboard form replays under Xvfb. Physical desktop and Wayland checks remain open. |

The active milestone is **RP-008 native interaction**. Detailed results and open platform checks are in the [roadmap](docs/ROADMAP.md). The [feature guide](docs/FEATURES.md) lists what you can try today.

## Where RustX is headed

- **Web.** Match or beat optimized vanilla JavaScript drawing the same interface to canvas. Compare rendering time and responsiveness with equivalent output and interactions.
- **Native.** Build pure-Rust apps for Windows, macOS, and Linux that run substantially faster and respond more smoothly than equivalent browser apps on the same hardware. Use Rust for native rendering, networking, background work, and optional application HTTP servers.

These are performance targets awaiting comparative measurements. The next platform steps are the macOS form replay and a physical Linux desktop replay. CI checks native CPU startup on Windows, macOS, and Linux X11. The benchmark plan starts with a scene editor, a large data workspace, and a live telemetry dashboard. [Vision and implementation stages](docs/VISION_IMPLEMENTATION.md)

## Try the demo

Clone this repository and run Docker Compose on a Windows, macOS, or Linux computer.

```sh
git clone https://github.com/ehsanrezaeee/rustx.git
cd rustx
docker compose up --build -d --wait
```

Open `http://127.0.0.1:3000/#/capabilities`. The guide links to Applications, People Explorer, the rendering engine, and the runtime lab. Docker builds the current checkout with its pinned Rust and Wasm tools. On Windows use Docker Desktop's Linux container mode.

To refresh the demo after the branch changes, run `git pull --ff-only` and `docker compose up --build -d --wait`, then reload the page. The bundle ID at `/assets.json` identifies the build currently served. Stop the demo with `docker compose down`.

### Open it on a phone or tablet

Connect the device and host computer to the same trusted local network. Replace `192.168.1.20` with the host computer's LAN address.

**Windows PowerShell**

```powershell
$env:RUSTX_BIND = '192.168.1.20'
docker compose up --build -d --wait
```

**macOS or Linux**

```sh
RUSTX_BIND=192.168.1.20 docker compose up --build -d --wait
```

Allow incoming TCP port 3000 on the host's private network if required. On the other device open `http://192.168.1.20:3000/?renderer=cpu#/capabilities`. Plain HTTP on a LAN address may limit browser features such as WebGPU, offline caching, and clipboard access. Use the browser's CPU mode for this setup. Physical mobile support remains under verification.

## Develop from source

Install Rust through rustup. Use the version pinned in `rust-toolchain.toml`. Windows needs the MSVC toolchain and Visual Studio C++ Build Tools. macOS needs Xcode Command Line Tools. The web build scripts install the Wasm target and the matching wasm-bindgen CLI when needed.

**Windows PowerShell**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-web.ps1
cargo run -p rustx-cli --locked -- serve dist 3000
```

**macOS or Linux**

```sh
bash scripts/build-web.sh
cargo run -p rustx-cli --locked -- serve dist 3000
```

For Rust source watching and browser reload use `cargo run -p rustx-cli --locked -- dev --target web`. Restart this command after editing the browser host embedded in the CLI. For a browser build with developer features disabled, use `cargo run -p rustx-cli --locked -- build --target web --release`. Serve that build with `serve dist 3000 --host <host-LAN-IP>` to reach it from another device.

### Run the native demo

```sh
cargo run -p rustx-lab --bin rustx-lab --locked
```

On Linux, use an X11 session. The Debian 12 validation used `libx11-6`, `libx11-xcb1`, `libxcursor1`, `libxi6`, and `libxkbcommon-x11-0`. Headless startup checks also use `xvfb` and `xauth`.

Build a local executable with `cargo run -p rustx-cli --locked -- build --target native --out dist-native`. On Windows and macOS the optional `rustx package --target native` command produces a local application directory with a dependency and license inventory. On Windows, run that command in a Visual Studio developer shell with `dumpbin` available. The app delivery uses the checkout and the current demo build.

### Start your own project

```sh
cargo install --path crates/rustx-cli --bin rustx --locked
rustx new my-app
cd my-app
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.100 --locked
rustx dev --target web
```

Use `rustx dev --target native` on a supported desktop host. The built-in demo runs without Node or Python. Node is used for contributor formatting, dependency audits, and optional editor packaging.

The [agent guide](docs/AGENT_GUIDE.md) points contributors to the active milestone, relevant code, and validation requirements.

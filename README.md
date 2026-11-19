# RustX

RustX is an experimental declarative UI framework for Rust. Build with typed components and `rustx!` markup. The same retained application can draw to a browser canvas through WebAssembly or run in a native desktop window. Rust owns component identity, state, layout, text shaping, editing, and the scene.

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

### Run natively on Windows or macOS

```sh
cargo run -p rustx-lab --bin rustx-lab --locked
```

Build a local executable with `cargo run -p rustx-cli --locked -- build --target native --out dist-native`. The optional `rustx package --target native` command produces a local application directory with a dependency and license inventory. On Windows, run that command in a Visual Studio developer shell with `dumpbin` available. The app delivery uses the checkout and the current demo build.

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

See [Features](docs/FEATURES.md), [Roadmap](docs/ROADMAP.md), and the [agent guide](docs/AGENT_GUIDE.md).

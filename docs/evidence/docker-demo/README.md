# Docker demo onboarding

16 November 2026; source baseline `082f9c4`. A source-built Linux image provides the browser demonstrations without host Rust, rustup, native build tools, wasm-bindgen or Node. Docker Desktop/Engine and Compose are still prerequisites. The actual UI and platform input/rendering run in the host browser.

## Delivered

- Multi-stage `Dockerfile`: Rust 1.90.0-bookworm, wasm-bindgen CLI 0.2.100, locked CLI/Wasm production build, Debian bookworm-slim runtime with the existing Rust fixture/static server.
- Source-allowlist `.dockerignore`: no Git metadata, local credentials, target/node_modules, prebuilt bundles or historical evidence enters the build context. The initial transferred context was 5.46 MB. Fonts, fixtures, locks and required source/docs are included.
- Compose publishes `127.0.0.1:${RUSTX_PORT:-3000}:3000`, runs a non-root, read-only container, and waits for its HTTP health check. No host source bind mounts or Windows shell script execution is needed.
- `rustx serve ... --host IP` explicitly supports container binding, preserving loopback by default. Option placement, invalid/duplicate values and real loopback serving are checked by `scripts/test-server-bind.mjs`.
- README explains Docker/PowerShell setup, source builds, new store/runtime routes and RP progress since Stage 4, including native-first architecture and the full browser parity track.
- A Linux Docker CI job builds from the checkout and exercises the served bundle in Chromium. No remote CI result is inferred from this local run.

## Actual verification

Environment: Docker Desktop on macOS arm64, Docker CLI 28.4.0, Compose v2.39.2-desktop.1; Linux ARM64 container. Browser: Chromium 156.0.8078.4. Node 24.18.1 and the local Rust installation were used for contributor validation, not to build the Docker image or run its server.

| Command/task | Result |
| --- | --- |
| `docker compose config --quiet` | Compose configuration accepted |
| `docker compose --progress plain build` | First source-only build succeeded; build.log records downloaded base-image digests, pinned tools and compilation |
| `RUSTX_PORT=3041 docker compose up -d --wait` | Healthy; start.log and compose.json |
| `RUSTX_URL=http://127.0.0.1:3041 node scripts/test-docker-demo.mjs` | Actual production bundle passes canvas deep links, fixture success/error/cancel, form editing, drawing route load, store publication and unmount/remount; browser.json/log and screenshot |
| Container runtime inspection | UID/GID 10001; no rustc/cargo/node or `/app/target`; runtime.log; image.log records architecture, size and image ID |
| `node scripts/test-server-bind.mjs` | Actual temporary socket tests pass default loopback, option ordering and rejection cases; server-bind.log |
| `cargo test --workspace --locked` | 154 passed, zero failed, one ignored native networking fixture; workspace.log |
| `npm run format:check`; `cargo fmt --all -- --check` | Passed; format.log; Rust formatting is silent |

The first Docker version probe could not contact the stopped daemon. Starting the already installed Docker Desktop resolved this; it was not an application/build failure. The initial Compose build command placed `--progress` after `build`; Compose emitted a usage-position notice and still completed. Subsequent commands use the global option before `build`.

The final documentation refresh is rebuilt through the same Docker cache and checked again; build-final.log and the final runtime/browser reports identify that image. The temporary test container is stopped with `docker compose down` after verification. Local Docker image/build caches remain available for later runs. No image registry publishing or public application/compiler deployment is performed.

## Limits

This proves a Linux ARM64 container and real Chromium application behavior on this Mac. It does not prove Docker Desktop installation or operation on an actual Windows computer, Linux AMD64 CI success, a native Windows UI, GPU rendering, physical IME or assistive technology. The Compose configuration uses Linux images with no host-specific path mounts; Windows users must enable Docker Desktop's Linux containers mode. Follow the official Docker setup links in README if WSL 2/virtualization is not ready.

Live playground compilation is deliberately absent from the container configuration: the Linux isolation provider remains unverified. The playground page is available but requires the separately documented compiler service for compilation. No unsafe compiler fallback was introduced. Live public API requests still need internet access; local fixture demos do not. The first image build needs internet access and downloads/compiler time; this is a convenience path, not a prebuilt download or performance claim.

Reference configuration is the checked-in Dockerfile/compose.yaml. Reproduce from the repository root with `docker compose up --build -d --wait`, then open `http://127.0.0.1:3000/?renderer=cpu#/capabilities`. Use `docker compose down` to stop. Current roadmap/release acceptance remains unchanged.

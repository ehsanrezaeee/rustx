# Web and native deployment

## Build and local preview

From the full source: `cargo build -p rustx-cli --locked`, then `target/debug/rustx build --target web --release --out dist-release`. Serve with `target/debug/rustx serve dist-release 3000`. The CLI embeds host assets and the source SDK. Rebuild the CLI after changing those inputs. `scripts/build-web.sh`/`.ps1` explicitly enable devtools for the inherited integration suite; release deployment uses the default CLI command above.

Web output contains the Rust Wasm module, generated bindings, hashed host/services/semantics, HTML/config, assets.json, Brotli/gzip representations and a versioned service worker. A matching Wasm fetch preload starts the large transfer while the host modules load; bindgen consumes that transfer once. Stable binding/host aliases are retained for compatibility tests; the browser entry point uses hashes. assets.json gives SHA-256 for its input payload and a build ID. It excludes itself, its generated service worker and compressed copies to avoid recursive identity; BUILD-INFO and the release payload manifest cover the full delivered files.

The static root is `/`. Subpath hosting is not implemented. An optional explicitly separate `/docs/index.html` mirrors the authoring guide; it is not the application rendering surface. Search engines cannot reliably index Rust-painted canvas text. No default SharedArrayBuffer/cross-origin isolation is required.

## Hosting contract

Use HTTPS outside loopback. Serve `.wasm` as `application/wasm`, `.js` as JavaScript and `.css` as CSS. Negotiate Brotli or gzip with the matching `Content-Encoding` and `Vary: Accept-Encoding`; do not send compressed bytes without the encoding header. The local server respects encoding quality/zero preferences, preferring Brotli at equal quality. Hashed, existing assets use `Cache-Control: public, max-age=31536000, immutable`; HTML, runtime configuration, service worker and stable aliases revalidate. ETag/304 is supported by the local server. Missing files with extensions return 404; extensionless history URLs rewrite to index.html. Hash routes need no special rewrite.

Default CSP permits same-origin script with `wasm-unsafe-eval`, same-origin resources, the explicitly used JSONPlaceholder API and an optional configured compiler origin. It denies objects, base injection and framing of the main app. nosniff and no-referrer headers are sent. Inline visible HTML styling is restricted to the bootstrap/explicit documentation/developer surfaces; application geometry is Rust-owned. Third-party APIs need explicit connect-src/CORS configuration. Do not put credentials in runtime-config.json. The local fixture server is development tooling, not a hardened public ingress.

Upload a complete versioned bundle atomically. Keep previous hashed assets until older clients/service workers expire. Deploy shell/config/worker together, revalidate the worker, and retain the prior bundle for rollback. Reusing an old shell with a new stable Wasm alias can break the ABI; do not partially copy `pkg/`. Verify the build and release payload checksums before publishing. No public deployment was requested or performed.

## Offline

After the first successful installation, the versioned service worker caches the application shell, runtime config and hashed dependencies. Navigation uses network first with cached index fallback. Asset requests use matching cached hashes. It never caches `/api/` responses or fabricates data. An offline People request shows a network-unavailable error. A compilation service also remains a network dependency; offline compilation is unavailable. Reopening at a nested extensionless route is exercised separately from a fixture success. Development bundles do not register a service worker, preventing stale code during rebuild/reload.

## Windows

Use an actual Windows x64 runner, Rust 1.90.0 and MSVC C++ tools. Run `scripts/build-windows.ps1` in an MSVC developer PowerShell. Fonts are embedded; available notices are copied. The PR workflow builds the native app/CLI and launches the Applications CPU app for two frames, including a second launch from the package. The recorded run is linked in REVIEW.md. Review `DLL-DEPENDENCIES.txt` for non-system DLLs, resolve remaining notice-copy records, verify all payload hashes, and run on a clean machine without Rust or any browser runtime. Signing and installer UI are optional. A Windows CI runner has development tools installed; its successful startup does not close the clean-machine, GPU/DPI/services/UIA/IME tasks in PLATFORM_CHECKS.md.

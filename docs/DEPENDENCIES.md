# Dependency choices

Rust is pinned to 1.90.0; Cargo.lock and package-lock.json are delivered. Use Cargo --locked and npm ci. These are prototype-compatible choices, not claims of latest versions.

| Dependency | Version | Purpose |
| --- | --- | --- |
| wasm-bindgen / CLI | 0.2.100 | Matched Wasm bindings |
| wasm-bindgen-futures / web-sys | 0.4.50 / 0.3.77 | Browser adapters |
| Taffy | 0.7.7 | Layout |
| cosmic-text | 0.14.2 | Text shaping/layout |
| Vello / wgpu | 0.5.0 / 24.0.5 | GPU pipeline |
| tiny-skia / fontdue | 0.11.4 / 0.9.3 | CPU rendering |
| winit / softbuffer | 0.30.12 / 0.4.6 | Native window/presentation |
| ureq | 2.12.1 | Native HTTP |
| tungstenite | 0.28.0 | Bounded native ws/wss adapter and local WebSocket fixture server; rustls TLS for clients |
| AccessKit / accesskit_winit | 0.25.1 / 0.34.1 | Native semantic projection and Windows UI Automation adapter source |
| unicode-segmentation | 1.13.3 | Grapheme-aware portable editing |
| png | 0.17.16 | Bounded shared PNG asset decoding; expands palette/grayscale/16-bit input to RGBA8 |
| typed-builder / trybuild | 0.20.1 / 1.0.115 | Props/compiler tests |
| sharp / Playwright | 0.35.5 / 1.64.1 | Test PNGs/browser automation |
| DejaVu Sans | Embedded asset | See assets/FONT-LICENSE.txt |
| Noto Emoji | Google Fonts revision `6c1dbe27dcf7d817d4b62cec033ba77fa8e2a5ec` | Bundled monochrome fallback; SIL OFL 1.1 |

Other exact/transitive versions are in the locks. Node is test tooling, not required to preview the included bundle through the Rust server. Windows needs Rust/MSVC and C++ build tools. Initial setup/build needs network. No Python project dependency.

LICENSE retains the original MIT notice. Font notices are included. A complete transitive license/security/unsafe-code audit is still required for RX-41; this document is not that audit. Do not ship unpinned Git main dependencies.

Current replay: Node 24.18.1, macOS 27.0.1 arm64. The original Playwright/sharp pins had concrete high-severity npm advisories; both were updated and locked. `docs/evidence/history/current-npm-audit.json` records the inherited findings, and `docs/evidence/current-npm-audit.json` reports zero after clean installation. See [sharp 0.35.5](https://github.com/lovell/sharp/releases/tag/v0.35.5) and the advisory URLs in the audit report. No complete Cargo vulnerability/license audit is claimed.

## Stage 2 bundled emoji fallback

The font is copied from the official [Google Fonts Noto Emoji directory at the pinned revision](https://github.com/google/fonts/tree/6c1dbe27dcf7d817d4b62cec033ba77fa8e2a5ec/ofl/notoemoji). The original filename is `NotoEmoji[wght].ttf`; the project stores the unmodified bytes as `crates/rustx-engine/assets/NotoEmoji.ttf`. The matching `OFL.txt` is stored as `crates/rustx-engine/assets/NOTO-EMOJI-LICENSE.txt`. Both files are bundled; builds do not download the font.

| File | SHA-256 |
| --- | --- |
| NotoEmoji.ttf | `de6c18832938afc99caf132b39d6a30a19bac7f2e812e28db2535b4608d27551` |
| NOTO-EMOJI-LICENSE.txt | `500bb1ccf43df7bbb522112f9133a52b16e1c35e809632f5d8609b179152de5b` |

This is monochrome outline fallback, not a claim of complete color emoji or every emoji sequence. Shared shaping chooses each run's font; CPU rasterization uses the same font identity rather than treating fallback glyph IDs as DejaVu glyph IDs. The SIL license and copyright notice travel with the source/assets. Stage 2 does not introduce a new Cargo dependency for this font.

Stage 2 promotes the already locked kurbo 0.11.3 geometry library to a direct core dependency for path winding/stroke hit tests, reusing established geometry calculations.

Stage 3 adds typed Serde/serde_json service envelopes, versioned storage values and JSON response decoding. Exact direct/transitive dependency resolutions remain in Cargo.lock. AccessKit integration source and successful compilation do not establish Windows UI Automation or screen-reader task success. Native clipboard/file/print adapters use explicitly detected OS utilities rather than adding a Python or browser runtime dependency.

## Stage 4 candidate — 12 November 2026

The CLI adds pinned flate2 1.1.10, brotli 8.0.2, sha2 0.10.9, getrandom 0.3.4 and libc 0.2.190 for compressed distribution, content identity, unguessable local jobs and OS isolation/resource accounting. Brotli generation adds locked alloc-no-stdlib 2.0.4, alloc-stdlib 0.2.4 and brotli-decompressor 5.0.3. syn/proc-macro2/quote reuse the compiler's parser ecosystem. The compatible rustix lock update is 1.1.6. Cargo.lock remains authoritative. The refreshed inventory contains 392 Cargo packages and queries all 385 registry resolutions; the same four maintenance findings and seven missing notice-copy records remain. Upstream notice recovery uses fresh locked Cargo metadata and rejects a stale inventory.

Contributor formatting uses exact Prettier 3.6.2 alongside Rust 1.90.0 rustfmt and the RustX macro formatter. Prettier, Playwright and sharp are optional Node tooling; they are not RustX application build/runtime dependencies. Python and Docker are not project build/runtime dependencies.

See stage4/SECURITY.md and evidence/stage4-{cargo-audit,npm-audit,licenses}.json for actual query identities, advisories, SPDX inventory, copied/pinned-upstream notices and missing-copy records. Remaining maintenance advisories and missing license copies are open release gates; no full transitive safety/license certification is claimed.

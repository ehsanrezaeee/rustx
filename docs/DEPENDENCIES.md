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

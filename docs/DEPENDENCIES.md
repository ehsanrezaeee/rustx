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

Other exact/transitive versions are in the locks. Node is test tooling, not required to preview the included bundle through the Rust server. Windows needs Rust/MSVC and C++ build tools. Initial setup/build needs network. No Python project dependency.

LICENSE retains the original MIT notice. Font notices are included. A complete transitive license/security/unsafe-code audit is still required for RX-41; this document is not that audit. Do not ship unpinned Git main dependencies.

Current replay: Node 24.18.1, macOS 27.0.1 arm64. The original Playwright/sharp pins had concrete high-severity npm advisories; both were updated and locked. `docs/evidence/history/current-npm-audit.json` records the inherited findings, and `docs/evidence/current-npm-audit.json` reports zero after clean installation. See [sharp 0.35.5](https://github.com/lovell/sharp/releases/tag/v0.35.5) and the advisory URLs in the audit report. No complete Cargo vulnerability/license audit is claimed.

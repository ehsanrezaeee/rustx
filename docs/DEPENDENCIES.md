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
| sharp / Playwright | 0.34.5 / 1.51.1 | Test PNGs/browser automation |
| DejaVu Sans | Embedded asset | See assets/FONT-LICENSE.txt |

Other exact/transitive versions are in the locks. Node is test tooling, not required to preview the included bundle through the Rust server. Windows needs Rust/MSVC and C++ build tools. Initial setup/build needs network. No Python project dependency.

LICENSE retains the original MIT notice. Font notices are included. A complete transitive license/security/unsafe-code audit is still required for RX-41; this document is not that audit. Do not ship unpinned Git main dependencies.

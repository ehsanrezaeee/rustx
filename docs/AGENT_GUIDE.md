# RustX agent guide

Read the [roadmap](ROADMAP.md), [features](FEATURES.md), and [README](../README.md). Use the [vision implementation guide](VISION_IMPLEMENTATION.md) to connect the active milestone to the reference applications and performance targets. Start with the next unchecked task in the roadmap. Keep the demos and CLI usable from a fresh checkout.

## Where to work

| Area | Source |
| --- | --- |
| Markup expansion and the public API | `crates/rustx-macros`, `crates/rustx` |
| Mount-once scopes, state, events, and typed services | `crates/rustx-core` |
| Layout, shaped text, scene, native event loop, and AccessKit | `crates/rustx-engine` |
| Browser bindings and platform adapters | `crates/rustx-web`, `web` |
| Scaffolding, builds, development server, and local compiler | `crates/rustx-cli`, `playground` |
| Demo pages, sample people, and editor | `examples/lab`, `fixtures/users.json`, `editors/vscode` |

Keep visible UI layout, text, and editing in Rust. Keep platform APIs at the host boundaries. Preserve keyed identity and release scoped work when its owner is unmounted. Treat bundled fonts and demo data as runtime assets.

Use application workflows to decide what to build. For web performance, compare RustX with optimized plain JavaScript drawing the same UI to canvas. For native performance, replay the same workloads against the browser builds on the same desktop. Record measurements using the protocol in the [vision implementation guide](VISION_IMPLEMENTATION.md).

## Continue RP-008

Use the Applications demo or an ignored local scaffold for the multi-control form. Continue with Windows Shift+Tab, then replay the full form on macOS, including disabled controls, dialog focus confinement, and Escape restoration. Use an actual native window and record the OS, renderer, input method, and expected versus actual results. Windows UI Automation can read back focus. For manual hardware or screen-reader coverage, record the device and assistive technology used.

## Validate and hand off

- Run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, and `npm run format:check` after installing the contributor Node dependencies with `npm ci`.
- Rebuild the CLI after editing the embedded web host, templates, or feature guide. Build a fresh web bundle and replay the changed demo in a browser. Check native behavior in a native window.
- Put standalone test harnesses and logs in `.rustx/validation/`. The prior local suites were moved to `.rustx/validation/legacy/` on the cleanup machine. A fresh checkout can recover the old files from Git history when needed. Inline Rust unit checks stay with their implementation.
- Record verified behavior and pending checks in the roadmap. Keep rebuilt bundles, archives, local reports, and screenshots out of Git. CI builds the demo from the checked-out revision.

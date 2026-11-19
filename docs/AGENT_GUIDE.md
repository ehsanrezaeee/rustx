# RustX agent guide

Read the [roadmap](ROADMAP.md), [features](FEATURES.md), and [README](../README.md). Start with the next unchecked task in the current roadmap milestone. Keep the demos and CLI usable from a fresh checkout.

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

## Continue RP-008

Extend the existing Applications demo or use an ignored local scaffold for the multi-control form. Check forward and reverse Tab order, disabled controls, dialog focus confinement, and Escape restoring the opener. Use an actual native window and record the OS, renderer, input method, and expected versus actual results. Windows UI Automation can read back focus. For manual hardware or screen-reader coverage, record the device and assistive technology used.

## Validate and hand off

- Run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, and `npm run format:check` after installing the contributor Node dependencies with `npm ci`.
- Rebuild the CLI after editing the embedded web host, templates, or feature guide. Build a fresh web bundle and replay the changed demo in a browser. Check native behavior in a native window.
- Put standalone test harnesses and logs in `.rustx/validation/`. The prior local suites were moved to `.rustx/validation/legacy/` on the cleanup machine. A fresh checkout can recover the old files from Git history when needed. Inline Rust unit checks stay with their implementation.
- Record verified behavior and pending checks in the roadmap. Keep rebuilt bundles, archives, local reports, and screenshots out of Git. CI builds the demo from the checked-out revision.

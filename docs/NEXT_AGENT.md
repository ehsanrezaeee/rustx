# Continue here

1. Read AGENTS.md, BLUEPRINT.md, STATUS.md, ACCEPTANCE.md and COMPATIBILITY.md. Record source manifest hash and environment. Do not restart the framework or require an older ZIP.
2. Run workspace tests, formatter check, web build and `npm ci && npm run test:wasm`. Never test stale Wasm after Rust edits.
3. Start the fixture server and run the supplied Playwright CPU suite on a normal workstation/CI. Add keyboard exactly-once activation, Tab order after keyed reorder, focus retention, scroll-to-focus, 390px/DPR changes and server failures. Fix real browser findings.
4. Run Chromium WebGPU, forced CPU and initial GPU failure. Record adapter/version and trace direct presentation. Test Firefox/Safari fallback. Device-loss recovery and full CPU/GPU parity are missing; the probe exposes image interpolation differences. Test physical hardware, not only software adapters.
5. Build and launch on real Windows x64. Exercise GPU/CPU, counter identity, input, HTTP success/cancel, resize/DPI. Retain screenshots/logs and actual OS/driver versions. Compilation alone cannot close RX-07.
6. Complete the Stage 1 screen-reader/button/IME/input feasibility tasks. Whole-string injection is not IME evidence. Visible caret/selection, full native editing and Windows UI Automation are unfinished.
7. Finish host capability/version negotiation, bounded message queues and mismatch tests. Add a keyed editable-list demo (currently keyed counters plus a separate input). Freeze exact benchmark hardware profiles when devices are available.
8. Update the ledger/blueprint with actual evidence and only then close Stage 1. Stage 2 adds engine conformance/fonts/caches/virtualization; Stage 3 full widgets/events/services; Stage 4 tooling/release.
9. Complete RX-45 People Explorer with search/details, all data states and real live API evidence. Complete RX-46 homepage with installation/docs and actual editable Rust compile/run playground. Write the compilation/isolation ADR first; do not relabel presets as a compiler.

Use shared public Rust APIs for demos. Preserve mount-once setup: plain props initialize, explicit signals update. Keep JS as the browser capability adapter. Return the complete project, built artifacts, updated blueprint/evidence and exact next unmet gate. No Python project build/runtime requirement.

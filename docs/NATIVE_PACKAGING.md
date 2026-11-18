# Native build and packaging — RP-007

RustX native applications use the shared Rust layout/text/scene engine and an OS window. They do not embed a browser or DOM. `rustx package` adds a handoff package to the existing native development/build commands. Docker remains a browser-demo option, not a native application generator.

## Developer workflow

Developers need the pinned Rust 1.90.0 toolchain and the platform's native compiler/linker. macOS needs Xcode Command Line Tools. Windows needs the MSVC Rust toolchain and Visual Studio C++ Build Tools; run packaging from a **Developer PowerShell for Visual Studio** so `dumpbin` is available. Neither Node nor wasm-bindgen is required for native packaging. The first build may download locked dependencies.

Install the current CLI from a checkout, then create a separate project:

```sh
cargo install --path crates/rustx-cli --bin rustx --locked
rustx new my-native-app
cd my-native-app
rustx dev --target native
```

Edit `src/app.rustx`; component setup remains mount-once. Close the native development window to stop that run. Build an executable with `rustx build --target native --release`, or create a distributable developer package:

```sh
rustx package --target native
```

Use `--target macos` or `--target windows` to require that actual host OS. These options do not cross-compile. The command selects Cargo's `default-run`, a binary matching the package name, or the sole binary; ambiguous projects must set `package.default-run`. It builds only that application's release binary for the rustc host triple. Windows packaging applies static CRT linkage; inspection rejects non-system direct link dependencies rather than producing a silently incomplete package.

Output is `dist-native/` by default. Existing directories are never overwritten, including previous packages. For another build:

```sh
rustx package --target native --out releases/native-v2
```

This is also supported from the RustX repository root, where the selected application is `rustx-lab`. The existing `scripts/build-windows.ps1` Stage 4 candidate workflow remains available; RP-007 adds a generic command usable from independently scaffolded projects.

## What recipients receive

The output contains a platform/architecture/version-named directory, its ZIP and a SHA-256 archive checksum. Extract the ZIP fully before launching:

- **macOS:** open the included `.app`; its executable is under `Contents/MacOS` and its metadata in `Contents/Info.plist`. Resources include framework, font and dependency notices.
- **Windows:** run the included `.exe` from the extracted directory. Generated Windows applications use the GUI subsystem, so they do not need a companion console window.

The recipient needs the matching OS/architecture and its system libraries, but not Rust, Cargo, Node or a browser runtime. This first package contract handles embedded assets (including RustX fonts); arbitrary app-specific external files and services are not automatically discovered or bundled. CPU mode can be selected with `RUSTX_RENDERER=cpu` using the same shell conventions as the main README.

Each package includes:

- `package.json`: identity, target, selected executable, toolchain, direct dependencies and payload hashes.
- `SHA256SUMS.txt`: hashes for payload files and package manifest; an outer checksum covers the ZIP.
- `DEPENDENCIES.txt`: actual `otool -L` or `dumpbin /DEPENDENTS` output.
- License copies and `licenses/DEPENDENCIES.json` (inside `.app/Contents/Resources` on macOS): Cargo dependency license expressions, available upstream license files and explicit review-needed entries. This includes build/non-runtime graph packages and is not a legal compliance certification.
- `RUN.md`: launch instructions and verification limits.

These are portable **developer packages**, not signed installers or store releases. Code signing/notarization, SmartScreen reputation, icons/custom bundle identity, installer/update/uninstall behavior, Linux packaging, complete dependency-license review and clean-machine acceptance remain open RP-007 delivery work. Do not describe a restricted-PATH test on a developer machine as a clean-machine test. GPU drivers and dynamically loaded OS facilities need separate platform testing beyond the direct dependency audit.

## Verification and continuation

`node scripts/test-native-package.mjs` creates a fresh scaffold, rejects unsafe/existing output, packages/extracts it, checks hashes, hides the source directory and starts the native CPU app from an unrelated directory with Rust/Node absent from `PATH`. It requires Node only as contributor test tooling. Two rendered frames and exit success prove startup, not keyboard/IME/accessibility/GPU correctness.

Windows packaging uses the system copy of `tar.exe` (libarchive) to write the ZIP from inside the staging directory. Recipients can extract it with Explorer or PowerShell. The native-package CI workflow runs on actual macOS and Windows runners and uploads ZIPs and evidence. Consult [RP-007 evidence](evidence/react-parity/RP-007/README.md) for actual results and clean-machine limits.

The 17 November Windows repair probes dumpbin with the CLI's real PE headers; help-command status is not a tool-availability check. Cargo package paths are canonicalized before application selection. CI runs the CLI regressions on both hosts and retains reports in a directory named for the run. Set `RUSTX_NATIVE_EVIDENCE_DIR` for a separate local replay. [Repair results](evidence/react-parity/RP-007/windows-fix-2026-11-17/README.md) distinguish pending and executed platform checks.

The [task ledger](REACT_PARITY_TASKS.md) explicitly preserves unfinished RP-001/RP-005 and remaining platform gates. Next native milestones are RP-008 interaction, RP-009 text/accessibility and RP-010 rendering/performance. Browser conformance remains RP-011; it no longer occupies RP-007.

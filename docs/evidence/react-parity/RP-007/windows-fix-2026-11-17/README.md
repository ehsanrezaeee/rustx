# RP-007 Windows CI repair — 17 November 2026

Continues source `ac7870e3fdfe8ab6a283a2d7220298427761c3a2`. The [original native-package run](https://github.com/ehsanrezaeee/rustx/actions/runs/43916508107) passes macOS and fails Windows before compiling the scaffolded application. `windows-ci-before.log` retains the failing Windows step: Visual Studio's developer shell starts, scaffolding succeeds, then the CLI incorrectly rejects the nonzero status of `dumpbin /?` as an unavailable tool.

The CLI now validates dumpbin by reading its own actual PE headers with `/HEADERS`. The application's `/DEPENDENTS` audit still requires success and rejects unsupported libraries. Cargo application selection canonicalizes the metadata directory before comparing it with the CLI's canonical working directory, including Windows extended paths. No renderer, layout, text, input or component-lifetime behavior changes.

The native workflow runs CLI regressions on both hosts before building. Each CI run writes evidence into its own directory, preserving previous platform reports. Reports identify the source, CLI hash and OS version. Local replays can set `RUSTX_NATIVE_EVIDENCE_DIR` to retain separate results.

The first repair replay, [run 44167929706](https://github.com/ehsanrezaeee/rustx/actions/runs/44167929706) at `d0759be`, passes both Windows CLI test entrypoints and builds the scaffold's release executable, then exposes a second bug: the system-library list omits `combase.dll`. `windows-ci-dependency-before.log` retains the complete failed Windows job. The locked Microsoft `windows-core` bindings import this library; the previous actual Windows CI import report already contains it. The system list now includes this exact library, and dependency-audit errors retain the full inspection output. Custom and CRT libraries still fail. The regression fixture is the actual 19-library Windows report from `d80ef49`/run 43387127048; it is dependency-parser evidence, not a new OS execution claim. `dependency-before.log` reproduces rejection before the correction.

## Executed checks

- `cargo test --workspace --locked`: **160 passed**, zero failed, one ignored native networking fixture; `workspace.log`.
- New canonical-directory regression with the old comparison: fails to select the independent app; `path-before.log`. With the correction: both CLI entrypoints pass all seven tests each; `path-after.log`.
- After the system-library correction, `cargo test --workspace --locked`: **162 passed**, zero failed, one ignored native networking fixture; `workspace-final.log`. The recorded-import regression accepts all 19 system imports and still rejects an appended dynamic CRT.
- Rustfmt and RustX/Prettier formatting pass; final logs are retained with this repair.
- Actual macOS arm64 `node scripts/test-native-package.mjs`: pass, including ZIP/payload hashes, unrelated launch directory, hidden source and toolchain-free PATH; `native-macos.log`, `native-darwin.json`, `package-darwin.json`, `dependencies-darwin.txt`, `startup-darwin.log`. The sandboxed attempt passed packaging/hash checks but aborted at window startup; `*-sandbox-failure.*` retain that failure. Desktop-enabled replay reaches two CPU frames and exits successfully.

The corrected actual Windows CI run is pending at this checkpoint. Only its completed result may establish Windows success. A restricted launch PATH on an SDK-equipped runner is not a clean-machine test. Native interaction, GPU/DPI, IME, human screen-reader, signing/notarization and installer gates remain open. RP-001/RP-005 remain unfinished; RP-008 follows the finite RP-007 platform check.

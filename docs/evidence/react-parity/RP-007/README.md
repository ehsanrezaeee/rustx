# RP-007 — Native package workflow

16 November 2026; source baseline `4e65327`. Implemented native packaging and corrected the execution roadmap to explicit native priorities. **Overall RP-007 remains implemented with open target/release gates**, not fully verified. The finite macOS scaffold/package/extract/startup contract has actual evidence below. Windows workflow results are tracked separately from source implementation.

## Deliverable

`rustx package --target native|macos|windows [--out dist-native]` builds a selected host-native release binary from a framework checkout or independently scaffolded application. It creates a macOS `.app`/ZIP or Windows portable ZIP, direct system-dependency report, target/version/toolchain manifest, payload/archive checksums, framework/font notices and a Cargo dependency-license inventory with available upstream license copies. Windows uses static CRT and requires dumpbin in an MSVC developer shell. Custom link dependencies are rejected rather than silently omitted. Only embedded app assets are covered; signing, installers and arbitrary external asset bundling are not claimed.

Output is staged privately and published only after build/audit/archive succeed. Existing outputs and unsafe paths are rejected. Generated project README and CLI handoff now lead with native development/package commands; Windows generated mains use the GUI subsystem. Raw native/web build commands remain present. No renderer/core/layout/text behavior was changed.

The roadmap/ledger now explicitly accounts for RP-000 (never assigned; numbering started at 001), unfinished RP-001 and unstarted RP-005, finite RP-002/003/004/006 slices and their limits, native RP-007–010, and retained browser-input RP-011. Both blueprint copies carry the native-first precedence update without changing historical RX acceptance. A higher task number is not completion of its predecessors.

## Executed checks

| Command/task | Actual result |
| --- | --- |
| `cargo test --workspace --locked` | 158 passed, zero failed, one ignored native networking fixture; workspace.log |
| `cargo test -p rustx-cli --locked` | Native dependency-filter/output-path regressions and existing formatter tests pass in both CLI entrypoints; cli-tests-final.log |
| `npm run format:check`; `cargo fmt --all -- --check` | Pass; format.log; Rust formatter silent |
| `cargo build -p rustx-cli --locked` | Embedded CLI/source workflow built; cli-build-final.log |
| `node scripts/test-native-package.mjs` on actual macOS arm64 | Fresh scaffold builds/packages, ZIP extracts, payload/archive hashes verify, dependency license index is present, wrong-host/traversal/existing-output checks pass, actual extracted native CPU window reaches two frames and exits with source hidden, unrelated cwd and toolchain-free PATH; native-darwin.json/native-macos.log/startup-darwin.log |

The initial native run before adding the full dependency-license index is retained in native-macos-initial.log. The final run includes that inventory and review-needed entries; missing license copies are not fabricated or described as legal clearance. package-darwin.json records the exact payload hashes; dependencies-darwin.txt records actual system framework/dylib imports. Native test output ZIP and checksum are kept under ignored `releases/RP-007/`, and the JSON report identifies their path/hash. Recipients can extract and launch this package without build tools. Source and tooling remain tracked and runnable from a checkout.

The native-package workflow is configured for actual macOS/Windows runners and artifact upload. Merely adding the workflow is not Windows execution evidence. No local Windows machine is available; pending CI results and independent clean-machine acceptance remain open until recorded. A runner with SDKs installed and a restricted PATH is not a clean machine. No native input interaction, IME, screen-reader, DPI/multi-window, GPU recovery, public-signing/notarization or installer/update/uninstall success is claimed. Those remain explicit RP-007 release gaps and RP-008–010 tasks.

## Reproduction

Install the CLI from this checkout, then from a new project: `rustx new native-demo`, `cd native-demo`, `rustx package --target native`. On Windows use Developer PowerShell with MSVC and dumpbin. No wasm32 target, wasm-bindgen, Node, Docker or browser is required for native package creation or app execution (Node is only the contributor test runner). Existing package outputs are never overwritten; choose a new `--out` for subsequent builds.

See [native packaging guide](../../../NATIVE_PACKAGING.md), ADR-017, and the complete task ledger. Available-platform verification can advance independently, but RP-007 cannot be marked fully complete while its required Windows/clean-machine/distribution checks remain open. Keep RP-001/RP-005 and browser follow-ups visible when selecting RP-008 work.

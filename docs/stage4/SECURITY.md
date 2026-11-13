# Security, license and unsafe review — candidate

The release is a locally tested candidate. The remaining findings below are not waived by successful compilation or a clean npm audit. No production multi-tenant compiler deployment has been authorized.

## Dependency evidence

Cargo.lock and package-lock.json pin exact resolutions. `npm audit --json` queries the current test-tool graph. `scripts/audit-dependencies.mjs` inventories all Cargo metadata packages and queries every locked registry package/version against the live OSV API, then retains full advisory records. It includes target-specific dependencies that are not reachable on this Mac. The report's Cargo.lock SHA-256 is the query identity. This detects known published advisories; it is not a full source audit.

The initial query found `RUSTSEC-2026-0486` in rustix 1.1.5 (zero-length xattr initialization). Cargo.lock now selects the compatible fixed 1.1.6. Remaining OSV records mark paste, rustybuzz and ttf-parser unmaintained. These inherited text/rendering dependencies require an upstream/migration plan and review; no silent registry fork or broad renderer upgrade was introduced solely to hide warnings. Exact versions and full details are in `evidence/stage4-cargo-audit.json`; RX-41 remains open for that review.

`evidence/stage4-licenses.json` records SPDX metadata, repositories and copied notices. `releases/licenses` contains the actual supplied license texts. Crates that omit notices are supplemented only when an official upstream text exists at the archive's recorded VCS commit; each recovered file records its exact origin. Several old/target-only crates have no packaged/pinned upstream license text (listed in `missingLicenseCopies`). Their declared SPDX metadata is retained, but missing notices are not replaced with invented copyright statements. Completing that inventory/review remains a release gate. Bundled DejaVu/Noto font notices are included with the exact font bytes. See DEPENDENCIES.md.

## Unsafe boundaries

The pre-existing native engine uses OS/graphics/font/network dependencies and explicit platform FFI. This is not an audit of every dependency's unsafe implementation. Stage 4 adds these reviewable boundaries:

- `rustx-core/src/allocation.rs`: optional global allocator wrapper delegates unchanged pointers/layouts to an underlying GlobalAlloc; successful allocation/reallocation adjusts atomics without allocating. Failed realloc leaves counters unchanged; deallocation uses the caller's original layout. Tests/benchmarks exercise actual allocation and lifecycle behavior. This feature is disabled in default builds.
- `rustx-cli/src/playground.rs`: Unix CommandExt pre_exec sets resource limits in the child; every setrlimit failure rejects the child. A new process group belongs to each compiler. Cancellation/limit termination sends SIGKILL only to that child's negative group ID, then waits. No visitor native program is run. macOS libproc writes into a correctly sized MaybeUninit rusage record; only success permits reading it. PID arrays are bounded. Memory-accounting failure terminates compilation. Signal handlers only store an atomic flag. Linux/Windows behavior is not inferred from Mac execution.
- Existing GPU/native API unsafe boundaries remain adapter-owned. Real GPU-loss/recovery and CPU tests are evidence for those tasks, not a proof that all unsafe code is sound.

The new unsafe blocks include safety comments. Do not treat the word `unsafe` count as an audit result; repeat Miri/sanitizer/target review where supported before final v1 acceptance.

## Host and compiler defaults

Production diagnostics are off. Passwords and typed key characters are redacted in development event traces. Ordinary app text remains inspectable only with explicit devtools. Source diagnostics are sanitized to visitor/framework logical files; compiler private paths and rendered expansion chains are removed. No keys/tokens/cookies are part of runtime config or package payload.

The local server has strict path containment, missing-asset errors, bounded request parsing and explicit CSP/nosniff/cache behavior. It is a development fixture server and is not sold as a public ingress. Separate compiler origin, opaque frame, worker watchdog, source/output/memory/time/concurrency/retention bounds and actual denied-read tests are documented in PLAYGROUND.md and ADR-012. OS isolation has a deny-default startup probe and no unsandboxed fallback. Its local tests are not an exhaustive adversarial sandbox review. Main app credentials must never be present on the preview origin.

The VS Code extension only runs project tools in trusted workspaces. Running a local project's Cargo checks follows VS Code's existing trust model; it can execute trusted project build scripts. This is distinct from visitor compilation, which never accepts build scripts/manifests. Do not enable the extension's project check on an untrusted checkout.

Before public release: resolve/document dependency maintenance and missing-license gates, review a real hosted compiler threat model if one is proposed, execute platform/manual tasks and retain fresh audit/identity evidence. No secrets or external deployment are needed for the supplied local demo.

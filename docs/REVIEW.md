# Sequential review of Stages 1–4 — 12 November 2026

Requested follow-up: test each stage, fix missing/failing implementation, open a PR from stage-4 to main, and merge after checks. This authorizes repository push/PR/merge and CI for this review. It does not authorize a public application/compiler deployment or waive missing manual/device acceptance.

The starting revision is 0336ca108c3c686257e5c6df1e19c26308443f06. Evidence for this review is in evidence/review-2026-11-12. ACCEPTANCE.md remains the authoritative RX register. Available target evidence is distinguished from unavailable Windows UIA, real OS IME, human screen-reader and physical-mobile tasks.

Stage 1: fresh workspace/formatter/build, host, actual Wasm, loopback HTTP, real Chrome/WebKit keyboard/keyed/lifecycle/input and physical Metal presentation/fallback checks pass. No missing core identity/ownership contract was reproduced. Actual OS IME and human AT tasks remain unverified.

Stage 2: inherited Wasm, Chrome/WebKit, physical GPU destruction/retry and native CPU/Metal scene comparison pass. Review found missing public style/text contracts: percentage/auto sizing, min/max height, physical/logical edge spacing, absolute offsets, stacking, font-family/weight/line-height and visual ellipsis. These now share Rust layout, shaping, paint and hit testing. Added regressions cover retained geometry, pixel/hit order, grapheme-safe truncation/source offsets and full editable text with asymmetric caret padding. Per-language glyph-variant shaping, complete affine style transforms, and real target/manual conformance remain open.

Stage 3: replay follows the updated Stage 2 build. Events/widgets/forms/editing, browser services, native loopback services and Applications startup are checked separately from real device/permission/AT tasks.

Stage 4: replay covers standalone CLI, actual editor, isolated compiler, real browser source compilation/live API, deployment/offline, dependency findings and measured budgets. Cross-platform CI is added; CI results will be recorded before merge. A runner's Windows CPU startup is not a clean-machine, GPU, UIA, IME or screen-reader result.

The desktop 16.7 ms GPU threshold is unchanged. Single cold-load/full-demo measurements do not certify a standalone counter budget or the five-sample/device contract. Failures and unavailable verification remain visible; no required threshold is relaxed.

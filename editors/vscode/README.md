# RustX for VS Code

Install rust-analyzer, then install the local VSIX built with `node scripts/package-editor.mjs`. Build/install `rustx` using Cargo and set `rustx.path` if it is not on PATH. Open a trusted Cargo project. `.rustx` files use the Rust language ID so rust-analyzer remains responsible for Rust semantics; this extension adds markup-aware formatting, imported component/prop completion, and source-linked `rustx check` diagnostics.

Choose RustX as the default formatter for Rust when needed. “RustX: Check project” invokes real Cargo/rustc; it does not simulate type errors. The CLI discovers component signatures through Syn. Full Rust semantic navigation remains rust-analyzer's responsibility. No tools execute in untrusted workspaces. A local extension package and provider tests do not establish a real editor smoke pass; executed evidence is recorded separately.

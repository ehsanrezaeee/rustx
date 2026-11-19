#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown
if ! command -v wasm-bindgen >/dev/null || [[ "$(wasm-bindgen --version)" != 'wasm-bindgen 0.2.100' ]]; then
  cargo install wasm-bindgen-cli --version 0.2.100 --locked
fi
cargo build -p rustx-cli --locked
# Local build with the inspector enabled.
exec target/debug/rustx build --target web --release --devtools "$@"

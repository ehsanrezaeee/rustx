#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown
if ! command -v wasm-bindgen >/dev/null || [[ "$(wasm-bindgen --version)" != 'wasm-bindgen 0.2.100' ]];then
 cargo install wasm-bindgen-cli --version 0.2.100 --locked
fi
cargo build -p rustx-lab --lib --target wasm32-unknown-unknown --release --locked
mkdir -p dist/pkg
wasm-bindgen target/wasm32-unknown-unknown/release/rustx_lab.wasm --target web --out-dir dist/pkg
cp web/index.html web/host.js web/services.js web/semantics.js dist/
echo 'Built dist/. Run: cargo run -p rustx-cli -- serve dist 3000'

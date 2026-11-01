$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
rustup target add wasm32-unknown-unknown
if ($LASTEXITCODE -ne 0) { throw 'Wasm target installation failed' }
$binding = Get-Command wasm-bindgen -ErrorAction SilentlyContinue
if (-not $binding -or ((wasm-bindgen --version) -ne 'wasm-bindgen 0.2.100')) {
 cargo install wasm-bindgen-cli --version 0.2.100 --locked
 if ($LASTEXITCODE -ne 0) { throw 'wasm-bindgen installation failed' }
}
cargo build -p rustx-lab --lib --target wasm32-unknown-unknown --release --locked
if ($LASTEXITCODE -ne 0) { throw 'Wasm build failed' }
New-Item -ItemType Directory -Force dist/pkg | Out-Null
wasm-bindgen target/wasm32-unknown-unknown/release/rustx_lab.wasm --target web --out-dir dist/pkg
if ($LASTEXITCODE -ne 0) { throw 'Bindings generation failed' }
Copy-Item web/index.html,web/host.js,web/services.js dist/
Write-Host 'Built dist/. Run: cargo run -p rustx-cli --locked -- serve dist 3000'

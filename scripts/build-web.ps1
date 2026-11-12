param([string[]]$BuildArguments = @())
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
rustup target add wasm32-unknown-unknown
if ($LASTEXITCODE -ne 0) { throw 'Wasm target installation failed' }
$binding = Get-Command wasm-bindgen -ErrorAction SilentlyContinue
if (-not $binding -or ((wasm-bindgen --version) -ne 'wasm-bindgen 0.2.100')) {
    cargo install wasm-bindgen-cli --version 0.2.100 --locked
    if ($LASTEXITCODE -ne 0) { throw 'wasm-bindgen installation failed' }
}
cargo build -p rustx-cli --locked
if ($LASTEXITCODE -ne 0) { throw 'CLI build failed' }
& target/debug/rustx.exe build --target web --release --devtools @BuildArguments
if ($LASTEXITCODE -ne 0) { throw 'Web build failed' }

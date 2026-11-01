$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
cargo build -p rustx-lab --bin rustx-lab --release --locked
if ($LASTEXITCODE -ne 0) { throw 'Native build failed; install Rust and MSVC build tools.' }
New-Item -ItemType Directory -Force dist/windows | Out-Null
Copy-Item target/release/rustx-lab.exe dist/windows/
Write-Host 'Built dist/windows/rustx-lab.exe. Run on Windows to verify.'

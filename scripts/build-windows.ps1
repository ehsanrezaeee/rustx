# This script must execute on an actual Windows x64 MSVC runner.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
if (-not $IsWindows -and $env:OS -ne 'Windows_NT') { throw 'Actual Windows required' }
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo test --workspace --locked --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw 'Windows tests failed' }
cargo build -p rustx-cli -p rustx-lab --bins --release --locked --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw 'Native build failed; install pinned Rust and MSVC C++ tools.' }
$bundle = 'releases/rustx-0.1.0-windows-x64'
if (Test-Path $bundle) { Remove-Item -Recurse -Force $bundle }
New-Item -ItemType Directory -Force $bundle | Out-Null
Copy-Item target/x86_64-pc-windows-msvc/release/rustx-lab.exe,target/x86_64-pc-windows-msvc/release/rustx.exe $bundle
Copy-Item LICENSE,NOTICE.md $bundle
Copy-Item -Recurse crates/rustx-engine/assets (Join-Path $bundle 'fonts-and-licenses')
if (Test-Path releases/licenses) { Copy-Item -Recurse releases/licenses (Join-Path $bundle 'licenses') }
$dumpbin = Get-Command dumpbin -ErrorAction SilentlyContinue
if (-not $dumpbin) { throw 'Run from an MSVC developer shell so runtime DLL dependencies can be audited with dumpbin.' }
& dumpbin /DEPENDENTS (Join-Path $bundle 'rustx-lab.exe') | Out-File (Join-Path $bundle 'DLL-DEPENDENCIES.txt')
if ($LASTEXITCODE -ne 0) { throw 'DLL dependency inspection failed' }
@'
Run rustx-lab.exe. No Rust/browser runtime is used by the native executable.
Fonts are embedded in the executable; license copies are included.
DLL-DEPENDENCIES.txt must be reviewed for non-system libraries before release.
This build output is not proof of clean-Windows execution, UIA/NVDA, IME,
DPI/resize, CPU/GPU rendering or native clipboard/files/permissions tasks.
Record those real tasks using docs/PLATFORM_CHECKS.md before RX-37 acceptance.
'@ | Set-Content (Join-Path $bundle 'RUN.txt')
git rev-parse HEAD | Set-Content (Join-Path $bundle 'GIT-REVISION.txt')
# Freeze the payload before opening the manifest, so it never hashes itself.
$payloadFiles = @(Get-ChildItem $bundle -Recurse -File | Sort-Object FullName)
$payloadFiles | ForEach-Object { $relative = $_.FullName.Substring((Resolve-Path $bundle).Path.Length + 1); "$(($_ | Get-FileHash -Algorithm SHA256).Hash.ToLower())  $relative" } | Set-Content (Join-Path $bundle 'SHA256SUMS.txt')
# Validate every emitted row before archiving the candidate.
Get-Content (Join-Path $bundle 'SHA256SUMS.txt') | ForEach-Object {
    $digest, $relative = $_ -split '  ', 2
    if ($relative -eq 'SHA256SUMS.txt') { throw 'Checksum manifest includes itself' }
    $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $bundle $relative)).Hash.ToLower()
    if ($actual -ne $digest) { throw "Payload checksum mismatch: $relative" }
}
Compress-Archive -Path $bundle -DestinationPath "$bundle.zip" -Force
Get-FileHash "$bundle.zip" -Algorithm SHA256 | Format-List | Out-File "$bundle.zip.sha256"
Write-Host 'Built Windows candidate. Clean-machine/manual acceptance still required.'

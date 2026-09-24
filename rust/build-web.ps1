# Builds the WebAssembly version into dist/web (serve it from any static host).
#   powershell -File build-web.ps1
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
cargo build --release --lib --target wasm32-unknown-unknown
if ($LASTEXITCODE) { exit $LASTEXITCODE }
wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/release/void_harvest.wasm
if ($LASTEXITCODE) { exit $LASTEXITCODE }
Remove-Item dist/web -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force dist/web | Out-Null
Copy-Item web/index.html dist/web/
Copy-Item web/pkg dist/web/pkg -Recurse
$kb = [math]::Round((Get-ChildItem dist/web -Recurse | Measure-Object Length -Sum).Sum / 1KB)
Write-Host "wrote dist/web ($kb KB). Serve it over http(s); file:// cannot load wasm modules."

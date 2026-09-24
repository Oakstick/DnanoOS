# Builds the Steam depot for Windows into dist/windows.
#   powershell -File build-windows.ps1          # plain standalone build
#   powershell -File build-windows.ps1 -Steam   # with Steamworks (achievements)
param([switch]$Steam)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$features = @()
if ($Steam) { $features = @('--features', 'steam') }
cargo build --release @features
if ($LASTEXITCODE) { exit $LASTEXITCODE }
$out = 'dist/windows'
Remove-Item $out -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item target/release/VoidHarvest.exe $out
if ($Steam) {
    # The Steamworks runtime has to sit next to the executable.
    $dll = Get-ChildItem target/release -Recurse -Filter steam_api64.dll | Select-Object -First 1
    if (-not $dll) { throw 'steam_api64.dll not found under target/release' }
    Copy-Item $dll.FullName $out
}
Get-ChildItem $out | Format-Table Name, @{ n = 'KB'; e = { [math]::Round($_.Length / 1KB) } }
Write-Host "Depot ready in $out. Upload with steamcmd: see steam/README.md"

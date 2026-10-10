# Every check the code must pass before it is committed: formatting, lints on the desktop and web
# targets (warnings are errors), and the tests.
#   powershell -File check.ps1
# For a refactor, also compare the capture suite before and after (see tools/capture_suite.py).
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

function Step($name, [scriptblock]$cmd) {
    Write-Host "== $name" -ForegroundColor Cyan
    & $cmd
    if ($LASTEXITCODE) { Write-Host "FAILED: $name" -ForegroundColor Red; exit $LASTEXITCODE }
}

Step 'format' { cargo fmt --check }
Step 'lint (desktop, tests included)' { cargo clippy --release --all-targets -- -D warnings }
Step 'lint (web)' { cargo clippy --release --lib --target wasm32-unknown-unknown -- -D warnings }
Step 'tests' { cargo test --release }
Write-Host 'all checks passed' -ForegroundColor Green

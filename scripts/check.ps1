# Runs everything CI runs: format check, clippy (warnings are errors), tests.
# Usage: powershell -File scripts/check.ps1
$ErrorActionPreference = 'Continue'
$env:Path += ";$env:USERPROFILE\.cargo\bin"
Set-Location (Split-Path $PSScriptRoot -Parent)
cargo fmt --all
if ($LASTEXITCODE -ne 0) { Write-Host 'FAIL: fmt'; exit 1 }
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { Write-Host 'FAIL: clippy'; exit 1 }
cargo test --workspace
if ($LASTEXITCODE -ne 0) { Write-Host 'FAIL: tests'; exit 1 }
Write-Host 'ALL CHECKS PASSED'

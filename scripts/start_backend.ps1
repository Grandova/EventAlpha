# PolyQuant 5M - Launch Backend Standalone
$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location (Join-Path $RootDir "backend")

Write-Host "Starting PolyQuant Rust Backend on port 8080..." -ForegroundColor Cyan
cargo run

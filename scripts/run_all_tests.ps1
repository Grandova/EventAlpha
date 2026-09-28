# PolyQuant 5M - Comprehensive Test Suite Verification
$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   POLYQUANT 5M - FULL AUTOMATED VERIFICATION SUITE       " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. Run Cargo Test Suite
Write-Host "`n[1/2] Running 15 Rust Integration Test Suites + Unit Tests..." -ForegroundColor Yellow
Set-Location (Join-Path $RootDir "backend")
cargo test -- --nocapture

if ($LASTEXITCODE -ne 0) {
    Write-Host "`n[FAILED] Backend tests encountered errors." -ForegroundColor Red
    exit 1
}

# 2. Run Frontend Build Check
Write-Host "`n[2/2] Running Frontend TypeScript Typecheck & Asset Bundle..." -ForegroundColor Yellow
Set-Location (Join-Path $RootDir "frontend")
npm run build

if ($LASTEXITCODE -ne 0) {
    Write-Host "`n[FAILED] Frontend build encountered errors." -ForegroundColor Red
    exit 1
}

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host "   ALL 15 TEST SUITES + FRONTEND BUILD PASSED (100% OK)   " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
Set-Location $RootDir

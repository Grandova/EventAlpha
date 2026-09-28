# PolyQuant 5M - One-Click Launch Script
# Starts both the Rust high-performance quant backend and frontend Web console

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   POLYQUANT 5M - QUANTITATIVE PREDICTION & SIMULATION    " -ForegroundColor Cyan
Write-Host "   Safety Lock: ENGAGED | Real Trading: STRICTLY DISABLED " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Cyan

$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

# 1. Build frontend dist if not built yet
$DistPath = Join-Path $RootDir "frontend\dist"
if (-not (Test-Path $DistPath)) {
    Write-Host "`n[1/3] Building frontend static web assets..." -ForegroundColor Yellow
    Set-Location (Join-Path $RootDir "frontend")
    npm run build
    Set-Location $RootDir
} else {
    Write-Host "`n[1/3] Frontend assets verified at $DistPath." -ForegroundColor Green
}

# 2. Check and start backend
Write-Host "`n[2/3] Launching PolyQuant Backend on http://127.0.0.1:8080..." -ForegroundColor Cyan
$BackendJob = Start-Process -FilePath "cargo" -ArgumentList "run --manifest-path backend/Cargo.toml" -PassThru -NoNewWindow

Start-Sleep -Seconds 3

# 3. Open browser
Write-Host "`n[3/3] Opening Web Dashboard at http://127.0.0.1:8080..." -ForegroundColor Cyan
Start-Process "http://127.0.0.1:8080"

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host "   System is RUNNING! Press Ctrl+C or kill process to exit " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green

Wait-Process -Id $BackendJob.Id

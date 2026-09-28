# PolyQuant 5M - Launch Frontend Vite Dev Server
$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location (Join-Path $RootDir "frontend")

Write-Host "Starting Vite Development Server on http://localhost:3000..." -ForegroundColor Cyan
npm run dev

# SIDM - Smart Internet Download Manager Launcher for Windows (PowerShell)
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$BinRelease = Join-Path $ScriptDir "sidm-core\target\release\sidm-core.exe"
$BinDebug = Join-Path $ScriptDir "sidm-core\target\debug\sidm-core.exe"

$Bin = $BinRelease
if (-not (Test-Path $Bin)) {
    $Bin = $BinDebug
}

if (-not (Test-Path $Bin)) {
    Write-Host "==================================================" -ForegroundColor Cyan
    Write-Host "   ⚡ Building SIDM Core Engine for Windows...   " -ForegroundColor Cyan
    Write-Host "==================================================" -ForegroundColor Cyan
    
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "[ERROR] Rust and Cargo are required. Install from https://rustup.rs/" -ForegroundColor Red
        Read-Host "Press Enter to exit"
        exit 1
    }

    Set-Location (Join-Path $ScriptDir "sidm-core")
    cargo build --release
    Set-Location $ScriptDir
    $Bin = $BinRelease
}

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "    ⚡ SIDM - Smart Internet Download Manager     " -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "🌐 Web Dashboard: http://127.0.0.1:45678" -ForegroundColor Yellow
Write-Host "🔌 Browser Extension: Load unpacked '$ScriptDir\sidm-extension'" -ForegroundColor Yellow
Write-Host "--------------------------------------------------" -ForegroundColor Gray

# Open browser dashboard
Start-Process "http://127.0.0.1:45678"

# Start SIDM Daemon
& $Bin server

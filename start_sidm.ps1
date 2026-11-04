# SIDM - Smart Internet Download Manager Launcher for Windows (PowerShell)
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$BinRelease = Join-Path $ScriptDir "sidm-core\target\release\sidm-core.exe"
$BinDebug = Join-Path $ScriptDir "sidm-core\target\debug\sidm-core.exe"
$BinRoot = Join-Path $ScriptDir "sidm-core.exe"

$Bin = $BinRelease
if (-not (Test-Path $Bin)) {
    if (Test-Path $BinRoot) {
        $Bin = $BinRoot
    } elseif (Test-Path $BinDebug) {
        $Bin = $BinDebug
    }
}

$BinDir = Join-Path $ScriptDir "bin"
if (-not (Test-Path $BinDir)) {
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
}

$YtdlpExe = Join-Path $BinDir "yt-dlp.exe"
if (-not (Test-Path $YtdlpExe)) {
    Write-Host "[SIDM] Downloading high-speed video extractor (yt-dlp)..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe" -OutFile $YtdlpExe
}

$FfmpegExe = Join-Path $BinDir "ffmpeg.exe"
if (-not (Test-Path $FfmpegExe)) {
    Write-Host "[SIDM] Downloading FFmpeg video/audio merger engine..." -ForegroundColor Cyan
    $ZipPath = Join-Path $BinDir "ffmpeg.zip"
    $TmpPath = Join-Path $BinDir "ffmpeg-tmp"
    Invoke-WebRequest -Uri "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip" -OutFile $ZipPath
    Expand-Archive -Path $ZipPath -DestinationPath $TmpPath -Force
    Get-ChildItem -Path $TmpPath -Recurse -Filter "ffmpeg.exe" | ForEach-Object { Copy-Item $_.FullName $FfmpegExe -Force }
    Get-ChildItem -Path $TmpPath -Recurse -Filter "ffprobe.exe" | ForEach-Object { Copy-Item $_.FullName (Join-Path $BinDir "ffprobe.exe") -Force }
    Remove-Item -Recurse -Force $TmpPath -ErrorAction SilentlyContinue
    Remove-Item -Force $ZipPath -ErrorAction SilentlyContinue
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

# Terminate any previously running instances
Stop-Process -Name "sidm-core" -Force -ErrorAction SilentlyContinue

# Open browser dashboard
Start-Process "http://127.0.0.1:45678"

# Start SIDM Daemon silently in background (No persistent console window)
Start-Process -FilePath $Bin -ArgumentList "server" -WindowStyle Hidden
Write-Host "✅ SIDM Core is now running silently in the background!" -ForegroundColor Green
Start-Sleep -Seconds 1

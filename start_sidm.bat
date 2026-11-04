@echo off
setlocal enabledelayedexpansion
title SIDM - Smart Internet Download Manager

set "DIR=%~dp0"
set "BIN=%DIR%sidm-core\target\release\sidm-core.exe"

if not exist "%BIN%" (
    if exist "%DIR%sidm-core.exe" (
        set "BIN=%DIR%sidm-core.exe"
    ) else if exist "%DIR%target\release\sidm-core.exe" (
        set "BIN=%DIR%target\release\sidm-core.exe"
    ) else if exist "%DIR%sidm-core\target\debug\sidm-core.exe" (
        set "BIN=%DIR%sidm-core\target\debug\sidm-core.exe"
    )
)

:: Ensure bin directory exists
if not exist "%DIR%bin" mkdir "%DIR%bin"

:: Ensure yt-dlp.exe exists
if not exist "%DIR%bin\yt-dlp.exe" (
    echo [SIDM] Downloading high-speed video extractor (yt-dlp)...
    powershell -NoProfile -Command "Invoke-WebRequest -Uri 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe' -OutFile '%DIR%bin\yt-dlp.exe'" >nul 2>&1
)

:: Ensure ffmpeg.exe exists for audio/video muxing
if not exist "%DIR%bin\ffmpeg.exe" (
    echo [SIDM] Downloading FFmpeg video/audio merger engine...
    powershell -NoProfile -Command "Invoke-WebRequest -Uri 'https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip' -OutFile '%DIR%bin\ffmpeg.zip'; Expand-Archive -Path '%DIR%bin\ffmpeg.zip' -DestinationPath '%DIR%bin\ffmpeg-tmp' -Force; Get-ChildItem -Path '%DIR%bin\ffmpeg-tmp' -Recurse -Filter 'ffmpeg.exe' | ForEach-Object { Copy-Item $_.FullName '%DIR%bin\ffmpeg.exe' -Force }; Get-ChildItem -Path '%DIR%bin\ffmpeg-tmp' -Recurse -Filter 'ffprobe.exe' | ForEach-Object { Copy-Item $_.FullName '%DIR%bin\ffprobe.exe' -Force }; Remove-Item -Recurse -Force '%DIR%bin\ffmpeg-tmp'; Remove-Item -Force '%DIR%bin\ffmpeg.zip'" >nul 2>&1
)

if not exist "%BIN%" (
    echo ==================================================
    echo    ⚡ Building SIDM Core Engine for Windows...
    echo ==================================================
    where cargo >nul 2>&1
    if %errorlevel% neq 0 (
        echo [ERROR] Rust and Cargo are required to compile SIDM on Windows.
        echo Please install Rust from https://rustup.rs/
        echo (Download and run rustup-init.exe)
        pause
        exit /b 1
    )
    cd /d "%DIR%sidm-core"
    cargo build --release
    cd /d "%DIR%"
    set "BIN=%DIR%sidm-core\target\release\sidm-core.exe"
)

echo ==================================================
echo     ⚡ SIDM - Smart Internet Download Manager     
echo ==================================================
echo 🌐 Web Dashboard: http://127.0.0.1:45678
echo 🔌 Browser Extension: Load unpacked '%DIR%sidm-extension'
echo --------------------------------------------------

:: Auto-integrate extension with Chrome, Edge, and Brave
if exist "%DIR%installer\install_browsers.bat" (
    call "%DIR%installer\install_browsers.bat" >nul 2>&1
)

:: Auto-open dashboard in default browser
start "" "http://127.0.0.1:45678"

:: Ensure any previous instance of SIDM is closed before launch
taskkill /F /IM sidm-core.exe >nul 2>&1

:: Start SIDM Daemon silently in background (No persistent CMD window)
start "" /B "%BIN%" server
echo  ✅ SIDM Core is now running silently in the background!
echo  🌐 Dashboard: http://127.0.0.1:45678
timeout /t 2 >nul
exit

@echo off
setlocal enabledelayedexpansion
title SIDM - Smart Internet Download Manager

set "DIR=%~dp0"
set "BIN=%DIR%sidm-core\target\release\sidm-core.exe"

if not exist "%BIN%" (
    set "BIN=%DIR%sidm-core\target\debug\sidm-core.exe"
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

:: Auto-open dashboard in default browser
start "" "http://127.0.0.1:45678"

:: Start SIDM Daemon
"%BIN%" server
pause

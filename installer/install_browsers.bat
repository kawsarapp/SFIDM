@echo off
setlocal enabledelayedexpansion
title SIDM - Browser Extension Auto-Installer

set "DIR=%~dp0.."
pushd "%DIR%"
set "ROOT=%CD%"
popd

set "EXT_DIR=%ROOT%\sidm-extension"
set "BIN_EXE=%ROOT%\sidm-core\target\release\sidm-core.exe"

set "EXT_ID=gadhilaplipaklonafepmbegnhoafice"

echo ==========================================================
echo    ⚡ SIDM Universal Browser Extension Auto-Installer
echo ==========================================================
echo Extension ID: %EXT_ID%
echo Extension Path: %EXT_DIR%
echo ----------------------------------------------------------

:: 1. Google Chrome (User Registry)
echo [*] Integrating with Google Chrome...
reg add "HKCU\Software\Google\Chrome\Extensions\%EXT_ID%" /v path /t REG_SZ /d "%EXT_DIR%" /f >nul 2>&1
reg add "HKCU\Software\Google\Chrome\Extensions\%EXT_ID%" /v version /t REG_SZ /d "1.0.0" /f >nul 2>&1

:: 2. Microsoft Edge (User Registry)
echo [*] Integrating with Microsoft Edge...
reg add "HKCU\Software\Microsoft\Edge\Extensions\%EXT_ID%" /v path /t REG_SZ /d "%EXT_DIR%" /f >nul 2>&1
reg add "HKCU\Software\Microsoft\Edge\Extensions\%EXT_ID%" /v version /t REG_SZ /d "1.0.0" /f >nul 2>&1

:: 3. Brave Browser (User Registry)
echo [*] Integrating with Brave Browser...
reg add "HKCU\Software\BraveSoftware\Brave-Browser\Extensions\%EXT_ID%" /v path /t REG_SZ /d "%EXT_DIR%" /f >nul 2>&1
reg add "HKCU\Software\BraveSoftware\Brave-Browser\Extensions\%EXT_ID%" /v version /t REG_SZ /d "1.0.0" /f >nul 2>&1

:: 4. Windows Startup Autostart (starts SIDM on Windows boot)
if exist "%BIN_EXE%" (
    echo [*] Setting up Windows boot autostart...
    reg add "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v "SIDM" /t REG_SZ /d "\"%BIN_EXE%\" server" /f >nul 2>&1
)

echo.
echo ==========================================================
echo  ✅ Integration Complete!
echo  Next time you open Chrome or Edge, click "Enable extension"
echo  when prompted at the top right of your browser!
echo ==========================================================
echo.
pause

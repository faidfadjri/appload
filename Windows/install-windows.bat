@echo off
setlocal
cd /d "%~dp0"
title AppLoad Installer

powershell.exe -NoProfile -ExecutionPolicy Bypass -Command ^
    "$ScriptDir = '%~dp0'; " ^
    "$candidates = @('AppLoad-Setup.exe', 'setup.exe', 'appload-windows-x86_64.exe', 'appload.exe', 'Shiply-Setup.exe', 'shiply.exe'); " ^
    "$found = $candidates | ForEach-Object { Join-Path $ScriptDir $_ } | Where-Object { Test-Path $_ } | Select-Object -First 1; " ^
    "if ($found) { " ^
    "    Unblock-File $found -ErrorAction SilentlyContinue; " ^
    "    Start-Process -FilePath $found -ArgumentList '--setup'; " ^
    "} else { " ^
    "    Write-Host 'Downloading AppLoad Installer...' -ForegroundColor Cyan; " ^
    "    $tmp = Join-Path $env:TEMP 'AppLoad-Setup.exe'; " ^
    "    Invoke-WebRequest -Uri 'https://github.com/faidfadjri/appload/releases/latest/download/AppLoad-Setup.exe' -OutFile $tmp -UseBasicParsing; " ^
    "    Unblock-File $tmp -ErrorAction SilentlyContinue; " ^
    "    Start-Process -FilePath $tmp -ArgumentList '--setup'; " ^
    "}"

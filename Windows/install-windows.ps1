# AppLoad Windows Desktop Installer Script
# Installs AppLoad to %LOCALAPPDATA%\Programs\AppLoad\appload.exe and creates
# Desktop and Start Menu shortcuts with logo.ico.

$ErrorActionPreference = 'Stop'
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Split-Path -Parent $ScriptDir
$GithubRepo = "faidfadjri/appload"

$AppLoadSetupExe = Join-Path $ScriptDir "AppLoad-Setup.exe"
$SetupExe = Join-Path $ScriptDir "setup.exe"
$AppLoadExe = Join-Path $ScriptDir "appload.exe"
$LegacyShiplySetupExe = Join-Path $ScriptDir "Shiply-Setup.exe"
$LegacyShiplyExe = Join-Path $ScriptDir "shiply.exe"

if (Test-Path $AppLoadSetupExe) {
    Unblock-File $AppLoadSetupExe -ErrorAction SilentlyContinue
    Write-Host "Running local AppLoad Windows Installer ($AppLoadSetupExe --install)..." -ForegroundColor Cyan
    & $AppLoadSetupExe --install
} elseif (Test-Path $SetupExe) {
    Unblock-File $SetupExe -ErrorAction SilentlyContinue
    Write-Host "Running local AppLoad Windows Installer ($SetupExe --install)..." -ForegroundColor Cyan
    & $SetupExe --install
} elseif (Test-Path $AppLoadExe) {
    Unblock-File $AppLoadExe -ErrorAction SilentlyContinue
    Write-Host "Running local AppLoad Windows Installer ($AppLoadExe --install)..." -ForegroundColor Cyan
    & $AppLoadExe --install
} elseif (Test-Path $LegacyShiplySetupExe) {
    Unblock-File $LegacyShiplySetupExe -ErrorAction SilentlyContinue
    Write-Host "Running legacy Shiply Windows Installer ($LegacyShiplySetupExe --install)..." -ForegroundColor Cyan
    & $LegacyShiplySetupExe --install
} elseif (Test-Path $LegacyShiplyExe) {
    Unblock-File $LegacyShiplyExe -ErrorAction SilentlyContinue
    Write-Host "Running legacy Shiply Windows Installer ($LegacyShiplyExe --install)..." -ForegroundColor Cyan
    & $LegacyShiplyExe --install
} elseif ((Get-Command "cargo" -ErrorAction SilentlyContinue) -and (Test-Path (Join-Path $RepoRoot "Cargo.toml"))) {
    Write-Host "Building and installing AppLoad from source with Cargo..." -ForegroundColor Cyan
    Push-Location $RepoRoot
    try {
        & cargo build --release
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo build failed with exit code $LASTEXITCODE."
        }
        $BuiltExe = Join-Path $RepoRoot "target\release\appload.exe"
        Unblock-File $BuiltExe -ErrorAction SilentlyContinue
        Write-Host "Running built installer ($BuiltExe --install)..." -ForegroundColor Cyan
        & $BuiltExe --install
    } finally {
        Pop-Location
    }
} else {
    Write-Host "Downloading latest AppLoad Installer from GitHub Releases..." -ForegroundColor Cyan
    $DownloadUrl = "https://github.com/$GithubRepo/releases/latest/download/AppLoad-Setup.exe"
    $TempInstaller = Join-Path $env:TEMP "AppLoad-Setup-$([System.Guid]::NewGuid().ToString('N')).exe"
    try {
        Write-Host "Downloading $DownloadUrl..." -ForegroundColor Gray
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempInstaller -UseBasicParsing
        Unblock-File $TempInstaller -ErrorAction SilentlyContinue
        Write-Host "Running downloaded installer ($TempInstaller --install)..." -ForegroundColor Cyan
        & $TempInstaller --install
    } finally {
        if (Test-Path $TempInstaller) {
            Remove-Item $TempInstaller -Force -ErrorAction SilentlyContinue
        }
    }
}

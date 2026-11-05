# AppLoad Universal Windows Installer Script
# Builds AppLoad from source (or downloads the latest release from GitHub if Cargo is absent)
# and installs/updates AppLoad in %LOCALAPPDATA%\Programs\AppLoad\appload.exe with Desktop/Start Menu shortcuts.

$ErrorActionPreference = 'Stop'
$GithubRepo = "faidfadjri/appload"

if ($PSScriptRoot) {
    Set-Location -LiteralPath $PSScriptRoot
}

if ((Get-Command "cargo" -ErrorAction SilentlyContinue) -and (Test-Path "Cargo.toml")) {
    Write-Host "[1/2] Building AppLoad release binary (cargo build --release)..." -ForegroundColor Cyan
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo release build failed (exit code $LASTEXITCODE)."
    }

    $InstallerExe = "./target/release/appload.exe"
    Write-Host "[2/2] Running self-installer ($InstallerExe --install)..." -ForegroundColor Cyan
    & $InstallerExe --install
    if ($LASTEXITCODE -ne 0) {
        throw "Installation failed (exit code $LASTEXITCODE)."
    }
} else {
    Write-Host "Cargo not found; downloading latest AppLoad binary from GitHub Releases..." -ForegroundColor Cyan
    $DownloadUrl = "https://github.com/$GithubRepo/releases/latest/download/appload-windows-x86_64.exe"
    $TempInstaller = Join-Path $env:TEMP "appload-installer-$([System.Guid]::NewGuid().ToString('N')).exe"
    try {
        Write-Host "Downloading $DownloadUrl..." -ForegroundColor Gray
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempInstaller -UseBasicParsing
        Write-Host "Running downloaded installer ($TempInstaller --install)..." -ForegroundColor Cyan
        & $TempInstaller --install
    } finally {
        if (Test-Path $TempInstaller) {
            Remove-Item $TempInstaller -Force -ErrorAction SilentlyContinue
        }
    }
}

Write-Host "Successfully installed/updated AppLoad!" -ForegroundColor Green
Write-Host "  Installed path: $env:LOCALAPPDATA\Programs\AppLoad\appload.exe" -ForegroundColor Gray

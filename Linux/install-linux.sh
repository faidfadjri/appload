#!/usr/bin/env bash
# AppLoad Linux Desktop Installer
# Installs the AppLoad binary to ~/.local/bin/appload, installs the app icon,
# and creates a desktop entry at ~/.local/share/applications/appload.desktop.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

INSTALL_BIN_DIR="$HOME/.local/bin"
INSTALL_SHARE_DIR="$HOME/.local/share/appload"
APPLICATIONS_DIR="$HOME/.local/share/applications"

mkdir -p "$INSTALL_BIN_DIR" "$INSTALL_SHARE_DIR" "$APPLICATIONS_DIR"

GITHUB_REPO="faidfadjri/appload"

if [[ -x "$SCRIPT_DIR/appload" ]]; then
    echo "[1/3] Installing prebuilt AppLoad binary from $SCRIPT_DIR/appload..."
    cp -f "$SCRIPT_DIR/appload" "$INSTALL_BIN_DIR/appload"
elif [[ -x "$SCRIPT_DIR/shiply" ]]; then
    echo "[1/3] Installing legacy prebuilt binary from $SCRIPT_DIR/shiply as appload..."
    cp -f "$SCRIPT_DIR/shiply" "$INSTALL_BIN_DIR/appload"
elif command -v cargo >/dev/null 2>&1 && [[ -f "$REPO_ROOT/Cargo.toml" ]]; then
    echo "[1/3] Building AppLoad release binary with Cargo..."
    (cd "$REPO_ROOT" && cargo build --release)
    cp -f "$REPO_ROOT/target/release/appload" "$INSTALL_BIN_DIR/appload"
else
    echo "[1/3] Downloading latest AppLoad binary from GitHub Releases..."
    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/latest/download/appload-linux-x86_64"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$INSTALL_BIN_DIR/appload" "$DOWNLOAD_URL"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$INSTALL_BIN_DIR/appload" "$DOWNLOAD_URL"
    else
        echo "ERROR: Neither cargo, curl, nor wget is installed." >&2
        exit 1
    fi
fi

chmod +x "$INSTALL_BIN_DIR/appload"

echo "[2/3] Installing AppLoad logo icon..."
if [[ -f "$REPO_ROOT/logo.png" ]]; then
    cp -f "$REPO_ROOT/logo.png" "$INSTALL_SHARE_DIR/logo.png"
    ICON_PATH="$INSTALL_SHARE_DIR/logo.png"
elif [[ -f "$SCRIPT_DIR/logo.ico" ]]; then
    cp -f "$SCRIPT_DIR/logo.ico" "$INSTALL_SHARE_DIR/logo.ico"
    ICON_PATH="$INSTALL_SHARE_DIR/logo.ico"
elif [[ -f "$REPO_ROOT/logo.ico" ]]; then
    cp -f "$REPO_ROOT/logo.ico" "$INSTALL_SHARE_DIR/logo.ico"
    ICON_PATH="$INSTALL_SHARE_DIR/logo.ico"
else
    ICON_PATH="utilities-terminal"
fi

echo "[3/3] Creating Linux desktop launcher ($APPLICATIONS_DIR/appload.desktop)..."
cat > "$APPLICATIONS_DIR/appload.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=AppLoad
Comment=Universal Desktop Deployment & Release Automation Platform
Exec=$INSTALL_BIN_DIR/appload
Path=$INSTALL_SHARE_DIR
Icon=$ICON_PATH
Terminal=false
Categories=Development;Utility;
StartupWMClass=AppLoad
EOF

chmod +x "$APPLICATIONS_DIR/appload.desktop"

# Clean up legacy desktop entry if present
rm -f "$APPLICATIONS_DIR/shiply.desktop" 2>/dev/null || true

echo "AppLoad installed successfully!"
echo "  Binary:   $INSTALL_BIN_DIR/appload"
echo "  Launcher: $APPLICATIONS_DIR/appload.desktop"

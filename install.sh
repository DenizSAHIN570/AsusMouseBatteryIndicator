#!/usr/bin/env bash
set -euo pipefail

REPO="DenizSAHIN570/AsusMouseBatteryIndicator"
BINARY_NAME="mouse-battery"
EXTENSION_UUID="asus-mouse-battery-icon@gnome"
EXTENSION_SRC="$(cd "$(dirname "$0")/gnome-extension" && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()    { echo -e "${GREEN}[+]${NC} $*"; }
warn()    { echo -e "${YELLOW}[!]${NC} $*"; }
error()   { echo -e "${RED}[x]${NC} $*" >&2; exit 1; }

# ── 1. Install the daemon binary ────────────────────────────────────────────

install_binary_from_release() {
    local url
    url="https://github.com/$REPO/releases/latest/download/$BINARY_NAME"
    info "Downloading $BINARY_NAME from GitHub Releases…"
    curl -fsSL "$url" -o /tmp/mouse-battery || return 1
    install -Dm755 /tmp/mouse-battery ~/.local/bin/mouse-battery
    rm -f /tmp/mouse-battery
}

install_binary_from_source() {
    info "Building $BINARY_NAME from source…"
    local cargo
    cargo="$(command -v cargo 2>/dev/null \
        || find "$HOME/.rustup/toolchains" -name cargo -type f 2>/dev/null | head -1 \
        || true)"
    [[ -z "$cargo" ]] && error "cargo not found. Install Rust from https://rustup.rs then re-run."
    local manifest
    manifest="$(cd "$(dirname "$0")/daemon" && pwd)/Cargo.toml"
    RUSTC="$(dirname "$cargo")/rustc" \
        RUSTUP_HOME="$HOME/.rustup" \
        CARGO_HOME="$HOME/.cargo" \
        "$cargo" build --release --manifest-path "$manifest"
    install -Dm755 "$(dirname "$manifest")/target/release/mouse-battery" \
        ~/.local/bin/mouse-battery
}

mkdir -p ~/.local/bin
if ! install_binary_from_release 2>/dev/null; then
    warn "No pre-built release found — building from source."
    install_binary_from_source
fi
info "Daemon installed to ~/.local/bin/mouse-battery"

# ── 2. udev rule + group (grants hidraw access without root at runtime) ───────

info "Installing udev rule (requires sudo)…"

# Ensure the plugdev group exists
if ! getent group plugdev &>/dev/null; then
    sudo groupadd plugdev
fi

# Add current user to plugdev if not already a member
GROUP_ADDED=false
if ! id -nG "$USER" | grep -qw plugdev; then
    sudo usermod -aG plugdev "$USER"
    GROUP_ADDED=true
fi

sudo install -Dm644 "$(dirname "$0")/udev/99-mouse-battery.rules" \
    /etc/udev/rules.d/99-mouse-battery.rules
sudo udevadm control --reload
sudo udevadm trigger
info "udev rule installed."

if $GROUP_ADDED; then
    warn "Added '$USER' to the 'plugdev' group."
    warn "You must log out and back in for device access to take effect."
fi

# ── 3. systemd user service ──────────────────────────────────────────────────

info "Installing systemd user service…"
install -Dm644 "$(dirname "$0")/systemd/mouse-battery.service" \
    ~/.config/systemd/user/mouse-battery.service
systemctl --user daemon-reload
systemctl --user enable --now mouse-battery
info "Daemon service enabled and started."

# ── 4. GNOME Shell extension ─────────────────────────────────────────────────

EXTENSION_DEST="$HOME/.local/share/gnome-shell/extensions/$EXTENSION_UUID"
info "Installing GNOME extension…"
mkdir -p "$EXTENSION_DEST"
cp "$EXTENSION_SRC/metadata.json" \
   "$EXTENSION_SRC/extension.js" \
   "$EXTENSION_SRC/stylesheet.css" \
   "$EXTENSION_DEST/"
if command -v gnome-extensions &>/dev/null; then
    gnome-extensions enable "$EXTENSION_UUID" 2>/dev/null || true
fi
info "Extension installed to $EXTENSION_DEST"

# ── Done ─────────────────────────────────────────────────────────────────────

echo
echo -e "${GREEN}Installation complete.${NC}"
echo
echo "  • The daemon is running as a systemd user service."
echo "    Check status: systemctl --user status mouse-battery"
echo "    View logs:    journalctl --user -u mouse-battery -f"
echo
echo "  • Restart GNOME Shell to activate the indicator:"
echo "    X11:    Alt+F2 → r → Enter"
echo "    Wayland: log out and log back in"
echo
echo "  • Then enable the extension if not already active:"
echo "    gnome-extensions enable $EXTENSION_UUID"

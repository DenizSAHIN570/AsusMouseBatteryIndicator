#!/usr/bin/env bash
set -euo pipefail

REPO="DenizSAHIN570/AsusMouseBatteryIndicator"
BINARY_NAME="mouse-battery"
BINARY_PATH="$HOME/.local/bin/$BINARY_NAME"
SERVICE_FILE="$HOME/.config/systemd/user/$BINARY_NAME.service"
UDEV_RULE="/etc/udev/rules.d/99-mouse-battery.rules"
EXTENSION_DEST="$HOME/.local/share/gnome-shell/extensions/asus-mouse-battery-icon@gnome"
EXTENSION_FILES=("metadata.json" "extension.js" "stylesheet.css")

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[+]${NC} $*"; }
warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
error() { echo -e "${RED}[x]${NC} $*" >&2; }
fail()  { error "$*"; exit 1; }

# ── Preflight checks ────────────────────────────────────────────────────────

PREFLIGHT_OK=true

check() {
    local label="$1" ok="$2" hint="$3"
    if [[ "$ok" == "true" ]]; then
        echo -e "  ${GREEN}✓${NC} $label"
    else
        echo -e "  ${RED}✗${NC} $label — $hint"
        PREFLIGHT_OK=false
    fi
}

echo "Checking installation…"

check "Binary installed" \
    "$([[ -x "$BINARY_PATH" ]] && echo true || echo false)" \
    "run install.sh first"

check "systemd service installed" \
    "$([[ -f "$SERVICE_FILE" ]] && echo true || echo false)" \
    "run install.sh first"

check "systemd service enabled" \
    "$(systemctl --user is-enabled "$BINARY_NAME" &>/dev/null && echo true || echo false)" \
    "run: systemctl --user enable $BINARY_NAME"

check "udev rule present" \
    "$([[ -f "$UDEV_RULE" ]] && echo true || echo false)" \
    "run install.sh (requires sudo)"

check "user in plugdev group" \
    "$(id -nG "$USER" | grep -qw plugdev && echo true || echo false)" \
    "run: sudo usermod -aG plugdev \$USER, then log out/in"

EXT_OK=true
for f in "${EXTENSION_FILES[@]}"; do
    [[ -f "$EXTENSION_DEST/$f" ]] || { EXT_OK=false; break; }
done
check "GNOME extension installed" "$EXT_OK" "run install.sh first"

if [[ "$PREFLIGHT_OK" != "true" ]]; then
    echo
    fail "Fix the issues above before updating. Run install.sh for a full installation."
fi

echo

# ── Fetch latest release version ───────────────────────────────────────────

info "Fetching latest release info…"
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])")
[[ -z "$LATEST" ]] && fail "Could not determine latest release version."
info "Latest release: $LATEST"

# ── Download and install binary ─────────────────────────────────────────────

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST/$BINARY_NAME"
info "Downloading $BINARY_NAME $LATEST…"
curl -fsSL "$DOWNLOAD_URL" -o /tmp/mouse-battery
install -Dm755 /tmp/mouse-battery "$BINARY_PATH"
rm -f /tmp/mouse-battery
info "Binary updated at $BINARY_PATH"

# ── Update GNOME extension files ────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
EXT_SRC="$SCRIPT_DIR/gnome-extension"

if [[ -d "$EXT_SRC" ]]; then
    for f in "${EXTENSION_FILES[@]}"; do
        src="$EXT_SRC/$f"
        dst="$EXTENSION_DEST/$f"
        # Skip if source and destination are the same file (symlinked install)
        if [[ "$(realpath "$src" 2>/dev/null)" != "$(realpath "$dst" 2>/dev/null)" ]]; then
            cp "$src" "$dst"
        fi
    done
    info "GNOME extension updated"
fi

# ── Restart service ──────────────────────────────────────────────────────────

info "Restarting service…"
systemctl --user restart "$BINARY_NAME"
systemctl --user status "$BINARY_NAME" --no-pager

echo
echo -e "${GREEN}Update complete.${NC} Running $LATEST"
echo
echo "  If the GNOME indicator looks stale, restart the shell:"
echo "    X11:     Alt+F2 → r → Enter"
echo "    Wayland: log out and back in"

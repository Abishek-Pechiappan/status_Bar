#!/usr/bin/env bash
set -euo pipefail

REPO_URL="https://github.com/hyrostrix/bar"
INSTALL_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/bar"
BIN_DIR="$HOME/.local/bin"
CFG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/bar"

# ── Colors ────────────────────────────────────────────────────────────────────
ok()   { printf '\033[32m✓\033[0m %s\n' "$*"; }
info() { printf '\033[34m→\033[0m %s\n' "$*"; }
err()  { printf '\033[31m✗\033[0m %s\n' "$*" >&2; exit 1; }

# ── Checks ────────────────────────────────────────────────────────────────────
command -v git   >/dev/null 2>&1 || err "git is required but not installed"
command -v cargo >/dev/null 2>&1 || err "Rust/cargo not found. Install via: curl https://sh.rustup.rs | sh"

# ── Clone or update the repo ──────────────────────────────────────────────────
if [ -d "$INSTALL_DIR/.git" ]; then
    info "Updating source in $INSTALL_DIR..."
    git -C "$INSTALL_DIR" pull --ff-only
else
    info "Cloning into $INSTALL_DIR..."
    git clone "$REPO_URL" "$INSTALL_DIR"
fi

# ── Build ─────────────────────────────────────────────────────────────────────
info "Building (this takes a minute the first time)..."
cargo build --release --manifest-path "$INSTALL_DIR/Cargo.toml"

# ── Install binaries ──────────────────────────────────────────────────────────
mkdir -p "$BIN_DIR"
install -m755 "$INSTALL_DIR/target/release/bar"        "$BIN_DIR/bar"
install -m755 "$INSTALL_DIR/target/release/bar-editor" "$BIN_DIR/bar-editor"
ok "Installed bar and bar-editor to $BIN_DIR"

# ── Install bar-update helper ─────────────────────────────────────────────────
cat > "$BIN_DIR/bar-update" <<EOF
#!/usr/bin/env bash
set -euo pipefail
INSTALL_DIR="${INSTALL_DIR}"
BIN_DIR="${BIN_DIR}"
info() { printf '\033[34m→\033[0m %s\n' "\$*"; }
ok()   { printf '\033[32m✓\033[0m %s\n' "\$*"; }
info "Pulling latest changes..."
git -C "\$INSTALL_DIR" pull --ff-only
info "Building..."
cargo build --release --manifest-path "\$INSTALL_DIR/Cargo.toml"
install -m755 "\$INSTALL_DIR/target/release/bar"        "\$BIN_DIR/bar"
install -m755 "\$INSTALL_DIR/target/release/bar-editor" "\$BIN_DIR/bar-editor"
ok "Updated."
pkill -x bar 2>/dev/null || true
sleep 0.4
nohup bar >/dev/null 2>&1 &
ok "Bar restarted."
EOF
chmod +x "$BIN_DIR/bar-update"
ok "Installed bar-update to $BIN_DIR"

# ── Example config ────────────────────────────────────────────────────────────
if [ ! -f "$CFG_DIR/bar.toml" ]; then
    mkdir -p "$CFG_DIR"
    cp "$INSTALL_DIR/bar.toml" "$CFG_DIR/bar.toml"
    ok "Config installed to $CFG_DIR/bar.toml"
else
    info "Config already exists at $CFG_DIR/bar.toml — skipping"
fi

# ── PATH hint ─────────────────────────────────────────────────────────────────
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo ""
    printf '\033[33m!\033[0m Add %s to your PATH:\n' "$BIN_DIR"
    echo "    echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc && source ~/.bashrc"
fi

# ── Done ──────────────────────────────────────────────────────────────────────
echo ""
ok "Done! Next steps:"
echo "   bar             — start the bar"
echo "   bar-editor      — open the GUI editor"
echo "   bar-update      — update to the latest version (run from anywhere)"
echo ""
echo "   Add to Hyprland: exec-once = bar"

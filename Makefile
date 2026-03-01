# Default: install system-wide so `exec-once = bar` works in Hyprland without
# needing to set PATH.  Override with `make PREFIX=/home/user/.local` for a
# per-user install (you must then ensure ~/.local/bin is in your login PATH).
PREFIX ?= /usr/local

.PHONY: build install install-user update upgrade clean check uninstall

build:
	cargo build --release --workspace

install: build
	sudo install -Dm755 target/release/bar        $(PREFIX)/bin/bar
	sudo install -Dm755 target/release/bar-editor $(PREFIX)/bin/bar-editor
	@echo "Installed to $(PREFIX)/bin/"
	@echo "Add to hyprland.conf:  exec-once = bar"
	@echo "Copy example config if needed:"
	@echo "  mkdir -p ~/.config/bar && cp bar.toml ~/.config/bar/bar.toml"

# Per-user install (requires ~/.local/bin in PATH — won't work with exec-once by default)
install-user: build
	install -Dm755 target/release/bar        $(HOME)/.local/bin/bar
	install -Dm755 target/release/bar-editor $(HOME)/.local/bin/bar-editor
	@echo "Installed to ~/.local/bin/"

uninstall:
	sudo rm -f $(PREFIX)/bin/bar $(PREFIX)/bin/bar-editor

# Rebuild, reinstall, and live-restart the running bar instance.
update: install
	@echo "Restarting bar..."
	@pkill -x bar 2>/dev/null || true
	@sleep 0.4
	@bar &
	@echo "Done."

# Pull latest git changes, then rebuild and restart.
upgrade:
	git pull
	$(MAKE) update

clean:
	cargo clean

check:
	cargo check --workspace

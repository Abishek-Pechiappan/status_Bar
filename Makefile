PREFIX ?= $(HOME)/.local

.PHONY: build install update clean check

build:
	cargo build --release -p bar-dashboard

install: build
	install -Dm755 target/release/bar-dashboard $(PREFIX)/bin/bar-dashboard
	@echo "Installed to $(PREFIX)/bin/bar-dashboard"
	@echo "Add to hyprland.conf:  bind = SUPER, D, exec, bar-dashboard"
	@echo "Copy example config if needed:"
	@echo "  mkdir -p ~/.config/bar && cp bar.toml ~/.config/bar/bar.toml"

# Rebuild, reinstall, and that's it — bar-dashboard is launched on demand via keybind.
update: install
	@echo "bar-dashboard updated."

clean:
	cargo clean

check:
	cargo check -p bar-dashboard

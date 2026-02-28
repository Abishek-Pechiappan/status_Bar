PREFIX ?= $(HOME)/.local

.PHONY: build install update upgrade clean check

build:
	cargo build --release

install: build
	install -Dm755 target/release/bar        $(PREFIX)/bin/bar
	install -Dm755 target/release/bar-editor $(PREFIX)/bin/bar-editor
	@echo "Installed to $(PREFIX)/bin/"
	@echo "Copy the example config if you haven't yet:"
	@echo "  mkdir -p ~/.config/bar && cp bar.toml ~/.config/bar/bar.toml"

update: install
	@echo "Restarting bar..."
	@pkill -x bar 2>/dev/null || true
	@sleep 0.4
	@bar &
	@echo "Done."

# Pull latest changes from git, then rebuild and restart.
upgrade:
	git pull
	$(MAKE) update

clean:
	cargo clean

check:
	cargo check --workspace

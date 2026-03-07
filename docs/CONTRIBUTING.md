# Contributing

Thank you for your interest in contributing to **bar-dashboard**!

---

## Development setup

### Prerequisites

- Rust 1.80+ via [rustup](https://rustup.rs/)
- A running Hyprland session (for testing the overlay)
- Wayland session with `wlr-layer-shell` support

### Clone and build

```bash
git clone https://github.com/hyrostrix/bar
cd bar

# Build the dashboard
cargo build -p bar-dashboard

# Run the dashboard overlay (requires Hyprland)
cargo run -p bar-dashboard

# Run all tests
cargo test --workspace
```

### Useful dev commands

```bash
# Fast type-check without linking
cargo check -p bar-dashboard

# Debug logging
RUST_LOG=debug cargo run -p bar-dashboard

# Release build
cargo build --release -p bar-dashboard
```

---

## Crate overview

| Crate | Path | Purpose |
|---|---|---|
| `bar-config` | `crates/config` | `DashConfig` TOML schema, `load()`, `ConfigWatcher` |
| `bar-theme` | `crates/theme` | `Color`, `Theme` (parsed from `ThemeConfig`) |
| `bar-dashboard` | `crates/dashboard` | Full-screen bento overlay binary |

---

## Adding a new dashboard card

1. **Add a new arm** to `make_card()` in `crates/dashboard/src/main.rs`:

```rust
"mycard" => {
    let col = Color::from_rgba(0.8, 0.6, 0.9, opacity);
    let content: Element<'_, Message> = column![
        text("MY").size(fsize + 10.0).color(col),
        text("My Card").size(fsize - 2.0).color(label_col),
        text("value").size(fsize + 4.0).font(bold_font).color(val_col),
    ].spacing(4.0).align_x(Alignment::Center).into();
    (content, col)
}
```

2. **Add a span** to `card_span()` if the card should be wider than 1 column:

```rust
"mycard" => 2,
```

3. **Add it to `bar.toml`** under `items`:

```toml
items = [
    "clock", "cpu", "memory",
    "mycard",
]
```

4. **Document it** in [CONFIGURATION.md](CONFIGURATION.md).

---

## Code style

- **No `unwrap()` in production paths** — use `?` or log and fall back gracefully
- Prefer `tracing::warn!` / `tracing::error!` over `eprintln!`
- Format with `cargo fmt` before committing
- Lint with `cargo clippy -p bar-dashboard -- -D warnings`

---

## Pull request checklist

- [ ] `cargo check -p bar-dashboard` passes
- [ ] `cargo clippy -p bar-dashboard` produces no new warnings
- [ ] `cargo fmt --check` passes
- [ ] New card (if any) is documented in [CONFIGURATION.md](CONFIGURATION.md)
- [ ] Commit message is concise and describes *why*, not just *what*

---

## Commit message convention

```
feat: add weather card with wttr.in integration
fix: prevent sparkline panic when history buffer is empty
chore: bump sysinfo to 0.39
docs: document card theming options
```

Prefixes: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`, `perf`

---

## Reporting issues

Please include:
- Hyprland version (`hyprctl version`)
- Rust version (`rustc --version`)
- Your `bar.toml`
- `RUST_LOG=debug bar-dashboard 2>&1` output

File issues at: https://github.com/hyrostrix/bar/issues

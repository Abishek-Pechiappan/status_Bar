# Contributing

Thank you for your interest in contributing to **bar**!

---

## Development setup

### Prerequisites

- Rust 1.80+ via [rustup](https://rustup.rs/)
- A running Hyprland session (for testing the bar itself)
- Wayland session with `wlr-layer-shell` support

### Clone and build

```bash
git clone https://github.com/hyrostrix/bar
cd bar

# Build everything
cargo build --workspace

# Run the bar (requires Hyprland)
cargo run

# Run the editor (works on any Wayland/X11 desktop)
cargo run --bin bar-editor

# Run all tests
cargo test --workspace
```

### Useful dev commands

```bash
# Fast type-check without linking
cargo check --workspace

# Debug logging
RUST_LOG=debug cargo run

# Check a single crate
cargo check -p bar-core

# Build only the editor
cargo build -p bar-editor
```

---

## Crate overview

| Crate | Path | Purpose |
|---|---|---|
| `bar-core` | `crates/core` | `AppState`, `Message` enum, `BarWidget` trait, `BarError` |
| `bar-config` | `crates/config` | `BarConfig` TOML schema, `load()`, `ConfigWatcher` |
| `bar-theme` | `crates/theme` | `Color`, `Theme`, `BarStyle` |
| `bar-system` | `crates/system` | CPU/RAM/disk/network/battery async monitor |
| `bar-ipc` | `crates/hyprland-ipc` | Hyprland IPC client, event parser |
| `bar-widgets` | `crates/widgets` | All 7 built-in widgets |
| `bar-renderer` | `crates/renderer` | Layout engine (Phase 2 stub) |
| `bar-wayland` | `crates/wayland` | `iced-layershell` application loop |
| `bar-editor` | `crates/editor` | GUI config editor (standard iced window) |

---

## Adding a new widget

1. **Create the widget file** in `crates/widgets/src/`:

```rust
// crates/widgets/src/mywidget.rs
use bar_core::state::AppState;
use iced::Element;

pub struct MyWidget;

impl MyWidget {
    pub fn view<'a, Message: Clone + 'static>(state: &'a AppState) -> Element<'a, Message> {
        iced::widget::text(format!("hello {}", state.active_workspace)).into()
    }
}
```

2. **Export it** in `crates/widgets/src/lib.rs`:

```rust
pub mod mywidget;
pub use mywidget::MyWidget;
```

3. **Wire it** in `crates/wayland/src/lib.rs` inside the `view()` function's widget match:

```rust
"mywidget" => MyWidget::view(&self.state).into(),
```

4. **Register it** in the editor's widget picker in `crates/editor/src/main.rs`:

```rust
const ALL_WIDGETS: &[&str] = &[
    "workspaces", "title", "clock", "cpu", "memory", "network", "battery",
    "mywidget",  // add here
];
```

---

## Code style

- **No `unwrap()` in production paths** — use `?` or log and fall back gracefully
- **No panics** in library crates (`bar-core`, `bar-config`, etc.)
- Prefer `tracing::warn!` / `tracing::error!` over `eprintln!`
- Format with `cargo fmt` before committing
- Lint with `cargo clippy --workspace -- -D warnings`

---

## Pull request checklist

- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` produces no new warnings
- [ ] `cargo fmt --check` passes
- [ ] New widget (if any) is documented in [CONFIGURATION.md](CONFIGURATION.md)
- [ ] Commit message is concise and describes *why*, not just *what*

---

## Commit message convention

```
feat: add MPRIS widget for media playback control
fix: prevent workspace widget panic when IPC socket is missing
chore: bump sysinfo to 0.39
docs: document per-monitor config override
```

Prefixes: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`, `perf`

---

## Reporting issues

Please include:
- Hyprland version (`hyprctl version`)
- Rust version (`rustc --version`)
- GPU/driver info (Vulkan/Mesa version if relevant)
- The `bar.toml` that reproduces the issue
- `RUST_LOG=debug bar 2>&1` output

File issues at: https://github.com/hyrostrix/bar/issues

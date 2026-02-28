# bar

> Vibe-coded a status bar because I wanted one I could actually customize without touching a config file every time.

A Wayland-native status bar for Hyprland. Built in Rust, themed with Catppuccin out of the box, and ships with a **GUI editor** so you can drag widgets around, pick colors, and tweak spacing live — no TOML required.

```
[1] [2] [3]   active_workspace   [window title]   [clock]   [↓ 1.2k ↑ 0.4k] [CPU 4%] [RAM 6.1/16G] [▓ 87%]
```

## Features

- **Deep Hyprland IPC integration** — workspace switching, active window title, fullscreen detection; all event-driven with no polling
- **Live config reload** — edit `bar.toml` and the bar updates instantly via inotify
- **GUI editor** — `bar-editor` lets you rearrange widgets, change theme colors, and adjust spacing without touching TOML
- **Wayland-native** — built on `iced-layershell` / wlr-layer-shell; exclusive zone keeps windows from overlapping
- **Low resource usage** — async Tokio runtime, batched system updates, no unnecessary redraws
- **Catppuccin Mocha** default theme; fully customizable

### Widgets

| Widget | Position | Description |
|---|---|---|
| `workspaces` | left | Clickable workspace buttons with active highlight |
| `title` | left | Active window title (truncated at 60 chars) |
| `clock` | center | Time and date (configurable strftime format) |
| `cpu` | right | Average CPU usage percentage |
| `memory` | right | RAM used / total |
| `network` | right | Download / upload rates (bytes/sec) |
| `battery` | right | Battery level with icon; auto-hidden on desktops |
| `disk` | right | Root filesystem used / total; auto-hidden if unavailable |
| `temperature` | right | CPU package temperature in °C; auto-hidden if no sensor |
| `volume` | right | Audio sink volume with mute indicator (requires `wpctl`) |
| `brightness` | right | Screen brightness %; auto-hidden if no backlight |

---

## Requirements

- Hyprland (any recent version)
- Wayland compositor with `wlr-layer-shell-unstable-v1` support
- A [Nerd Font](https://www.nerdfonts.com/) or **JetBrains Mono** for the battery icon
- Rust 1.80+ (`rustup` recommended)

---

## Installation

> **Requires:** `git` and Rust. Install Rust with: `curl https://sh.rustup.rs | sh`

### Install

```bash
git clone https://github.com/Abishek-Pechiappan/status_Bar
cd status_Bar
bash install.sh
```

The script builds both binaries, installs them to `~/.local/bin`, copies the example config to `~/.config/bar/bar.toml`, and sets up the `bar-update` command.

### Update

```bash
bar-update
```

Pulls the latest changes, rebuilds, and restarts the bar automatically.

### Arch Linux (AUR)

```bash
# Via yay / paru (once published)
yay -S bar-hyprland
```

---

## Quick Start

1. Add `bar` to your Hyprland config:

```
# ~/.config/hypr/hyprland.conf
exec-once = bar
```

2. Launch the GUI editor to customize layout and theme:

```bash
bar-editor
```

3. Edit `~/.config/bar/bar.toml` directly for advanced settings — changes are applied live.

---

## Configuration

The config file lives at `$XDG_CONFIG_HOME/bar/bar.toml` (default: `~/.config/bar/bar.toml`).

### Minimal example

```toml
[global]
height         = 40
position       = "top"   # "top" | "bottom"
exclusive_zone = true
opacity        = 0.95

[[left]]
kind = "workspaces"

[[left]]
kind = "title"

[[center]]
kind = "clock"

[[right]]
kind = "cpu"

[[right]]
kind = "memory"

[theme]
background    = "#1e1e2e"
foreground    = "#cdd6f4"
accent        = "#cba6f7"
font          = "JetBrains Mono"
font_size     = 13.0
border_radius = 6.0
padding       = 8
gap           = 4
```

See [CONFIGURATION.md](docs/CONFIGURATION.md) for the full reference.

---

## Project Structure

```
bar/
├── src/main.rs           — entry point, init logging, launch wayland loop
├── bar.toml              — example config (Catppuccin Mocha)
├── crates/
│   ├── core/             — AppState, Message enum, BarWidget trait, BarError
│   ├── config/           — BarConfig schema, load(), ConfigWatcher (inotify)
│   ├── theme/            — Color, Theme, BarStyle
│   ├── system/           — CPU/RAM/disk/network/battery async monitor
│   ├── hyprland-ipc/     — IPC client, event parser, WorkspaceInfo
│   ├── widgets/          — all 7 built-in widgets
│   ├── renderer/         — layout engine (Phase 2)
│   ├── wayland/          — iced-layershell app loop
│   └── editor/           — bar-editor GUI (iced desktop window)
└── docs/
    ├── ARCHITECTURE.md
    ├── CONFIGURATION.md
    └── CONTRIBUTING.md
```

---

## GUI Editor (`bar-editor`)

```bash
bar-editor
```

The editor opens as a regular desktop window with three tabs:

- **Global** — bar height, top/bottom position, opacity, exclusive zone toggle
- **Layout** — drag widgets between Left / Center / Right columns with ↑ ↓ × buttons and an Add picker
- **Theme** — hex color swatches for background / foreground / accent, font, and size/radius/spacing sliders

Pressing **Save Changes** writes `~/.config/bar/bar.toml`; the running bar reloads automatically.

---

## Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run the editor
cargo run --bin bar-editor

# Run all tests
cargo test --workspace

# Check only (fast)
cargo check --workspace
```

### Adding a widget

1. Create `crates/widgets/src/mywidget.rs` implementing the `view(state: &AppState)` pattern used by other widgets
2. Add a `pub mod mywidget;` line to `crates/widgets/src/lib.rs`
3. Wire it into `crates/wayland/src/lib.rs` — add an arm to the `match widget.kind.as_str()` block in `view()`
4. Add `"mywidget"` to the `ALL_WIDGETS` slice in `crates/editor/src/main.rs`

---

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for crate dependency diagram, message flow, and subscription lifecycle.

---

## License

MIT — see [LICENSE](LICENSE).

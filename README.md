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
- **Notification panel** — acts as an `org.freedesktop.Notifications` D-Bus daemon; shows a count badge and expands a drop-down panel on click. Falls back to polling `dunstctl history` automatically when dunst/mako is already running
- **Floating bar mode** — set `margin` (horizontal) and `margin_top` (vertical) in `[global]` to float the bar away from screen edges with full compositor-level transparency underneath
- **19 built-in widgets** — from workspaces and media controls to battery, brightness, custom shell commands, and notifications

### Widgets

| Widget | Description |
|---|---|
| `workspaces` | Clickable workspace buttons with active highlight |
| `title` | Active window title (truncated at 60 chars) |
| `clock` | Time and date (configurable strftime format) |
| `cpu` | Average CPU usage percentage |
| `memory` | RAM used / total |
| `network` | Download/upload speed; optionally shows interface name and WiFi signal strength (configurable) |
| `battery` | Battery level with icon; auto-hidden on desktops |
| `disk` | Root filesystem used / total; auto-hidden if unavailable |
| `temperature` | CPU package temperature in °C; auto-hidden if no sensor |
| `volume` | Audio sink volume with mute indicator (requires `wpctl`) |
| `brightness` | Screen brightness %; auto-hidden if no backlight |
| `swap` | Swap space used / total; auto-hidden if no swap |
| `uptime` | System uptime (e.g. `3d 14h 22m`) |
| `load` | 1 / 5 / 15-minute load averages |
| `keyboard` | Active keyboard layout name (from Hyprland `activelayout` event) |
| `media` | Current media track title + artist; requires `playerctl` |
| `custom` | Output of any shell command (set `custom_command` in `[global]`) |
| `separator` | Visual spacer / divider between widgets |
| `notify` | Notification count badge; click to expand drop-down panel |

Place any widget in `[[left]]`, `[[center]]`, or `[[right]]` — there are no restrictions.

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

### Floating / ricing options

```toml
[global]
margin     = 8    # horizontal gap from screen edges (requires restart)
margin_top = 6    # vertical gap from screen edge (requires restart)

[theme]
widget_bg    = "#313244"       # widget pill background (empty = transparent)
border_color = "#cba6f7"       # bar border color (empty = no border)
border_width = 1               # bar border width in pixels
network_show = "speed,signal"  # comma list: "speed", "name", "signal"
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
│   ├── widgets/          — 19 built-in widgets (workspaces → notifications)
│   ├── renderer/         — layout engine (Phase 2)
│   ├── wayland/          — iced-layershell app loop, notification D-Bus daemon
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

- **Global** — bar height, top/bottom position, opacity, exclusive zone, horizontal and vertical margin (floating mode; ⟲ requires restart to apply)
- **Layout** — add/remove/reorder widgets in Left, Center, Right columns; 19 widget types available
- **Theme** — full color picker (background, text, accent, widget background, border); font family & size; border radius, padding, gap, widget padding; workspace display style (numbers vs dots); network display options (speed / name / signal); clock & date format; 14 built-in color presets + pywal import

Theme changes apply **live** via inotify — the bar reloads instantly without restarting. Geometry changes (height, position, margins) prompt a save-and-restart from within the editor.

---

## Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run the editor
cargo run -p bar-editor

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

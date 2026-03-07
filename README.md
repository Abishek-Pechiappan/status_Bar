# bar

> A production-grade, Wayland-native status bar for Hyprland — built in Rust, themed with Catppuccin Mocha out of the box.

[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![iced](https://img.shields.io/badge/iced-0.14-blueviolet)](https://github.com/iced-rs/iced)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![AUR](https://img.shields.io/badge/AUR-bar--hyprland-1793d1?logo=arch-linux)](https://aur.archlinux.org/)

```
[1] [2] [3]   My Terminal   |   12:34   |   ↓ 1.2k  ↑ 0.4k   CPU 4%   RAM 6.1/16G   ▓ 87%
```

---

## Screenshot / Preview

> Screenshots coming soon.

The **bento dashboard overlay** (`bar-dashboard`) displays a full-screen grid of system info cards — CPU history sparklines, GPU stats, network speeds, Bluetooth device, weather, media controls, and more — launched from a single keybind and dismissed with Escape.

---

## Features

### Status Bar

- **Wayland-native** — built on `iced-layershell` / `wlr-layer-shell-unstable-v1`; no XWayland, no wrappers
- **Deep Hyprland IPC integration** — workspace switching, active window title, submap changes; fully event-driven with zero polling
- **Live config reload** — edit `bar.toml` and the bar updates instantly via inotify (`notify` crate)
- **GUI editor** (`bar-editor`) — rearrange widgets, change colors, pick fonts, adjust spacing live without touching TOML
- **Notification daemon** — registers as `org.freedesktop.Notifications` over D-Bus; shows count badge and expands a drop-down panel on click; falls back to polling `dunstctl history` automatically
- **Power menu** — three display styles (`dropdown`, `inline`, `overlay`); animated with `lilt` (slide / fade / scale)
- **Floating bar** — set `margin` and `margin_top` to float the bar away from screen edges with compositor-level transparency underneath
- **27 built-in widgets** — workspaces, media, volume, brightness, GPU, submap, screencast indicator, DND toggle, and more
- **Fully customizable theme** — Catppuccin Mocha default; per-widget backgrounds, borders, pill/rounded/sharp shape presets, custom font

### Bento Dashboard Overlay

- **Full-screen bento grid** — configurable column count (2–4), card order, and display theme (`minimal`, `cards`, `full`, `vivid`)
- **Sparkline graphs** — animated CPU usage history and network throughput history drawn with iced Canvas
- **Weather card** — live conditions from `wttr.in`; configure `weather_location` in `[global]`
- **Bluetooth card** — adapter status and connected device name via `bluetoothctl`
- **GPU card** — utilization %, temperature, VRAM used/total; auto-hidden when no GPU detected
- **Media controls** — track title, artist, play/pause/skip via `playerctl`
- **Keyboard-dismissible** — press Escape or click the dim overlay to close

---

## Requirements

### Runtime

| Dependency | Purpose | Notes |
|---|---|---|
| Hyprland | Wayland compositor | Any recent version |
| Nerd Font | Widget icons (battery, volume, etc.) | JetBrains Mono Nerd Font recommended |
| `wpctl` (PipeWire) | `volume` widget | Part of `pipewire-audio` |
| `brightnessctl` | `brightness` widget | Optional; widget auto-hides |
| `playerctl` | `media` widget | Optional |
| `bluetoothctl` | Dashboard Bluetooth card | Optional |
| `curl` | Dashboard weather card | Optional; card hidden if `weather_location` is empty |
| `dunst` / `mako` / `swaync` | Notification fallback | Optional; bar acts as its own daemon |

### Build

- Rust 1.80+ — install via `curl https://sh.rustup.rs | sh`
- `git`
- Standard C linker (provided by `base-devel` on Arch)

---

## Installation

### One-line install (recommended)

Clones the repo, builds both binaries in release mode, installs them to `~/.local/bin`, and copies the example config:

```bash
git clone https://github.com/Abishek-Pechiappan/status_Bar
cd status_Bar
bash install.sh
```

### Update

```bash
bar-update
```

Pulls the latest changes, rebuilds, and restarts the bar automatically.

### Build from source manually

```bash
git clone https://github.com/Abishek-Pechiappan/status_Bar
cd status_Bar
cargo build --release --workspace

# Install binaries
install -m755 target/release/bar        ~/.local/bin/bar
install -m755 target/release/bar-editor ~/.local/bin/bar-editor
install -m755 target/release/bar-dashboard ~/.local/bin/bar-dashboard

# Copy example config (first time only)
mkdir -p ~/.config/bar
cp bar.toml ~/.config/bar/bar.toml
```

### Arch Linux (AUR)

```bash
# Via yay or paru (once published)
yay -S bar-hyprland
```

### Hyprland autostart

Add to `~/.config/hypr/hyprland.conf`:

```
exec-once = bar

# Optional: open the dashboard with Super+D
bind = SUPER, D, exec, bar-dashboard
```

---

## Configuration

The config file lives at `$XDG_CONFIG_HOME/bar/bar.toml` (default: `~/.config/bar/bar.toml`). Changes to most settings are applied live — geometry changes (height, position, margins) require a restart, which `bar-editor` handles automatically.

### Full config reference

```toml
[global]
height         = 40        # bar height in logical pixels
position       = "top"     # "top" or "bottom"
exclusive_zone = true      # reserve space so windows do not overlap the bar
opacity        = 0.95
margin         = 0         # horizontal gap from screen edges (floating bar; requires restart)
margin_top     = 0         # vertical gap from screen edge   (floating bar; requires restart)

# Power menu
lock_command      = "loginctl lock-session"
power_menu_style  = "overlay"   # "overlay" | "dropdown" | "inline"
power_anim_style  = "slide"     # "slide" | "fade" | "scale" | "none"
power_actions     = ["lock", "sleep", "hibernate", "logout", "reboot", "shutdown"]

# Dashboard weather card — leave empty to hide the card
weather_location  = ""          # e.g. "London" or "48.8566,2.3522"

# Widget layout — three sections: left · center · right
[[left]]
kind = "workspaces"

[[left]]
kind = "title"

[[center]]
kind = "clock"

[[right]]
kind = "network"

[[right]]
kind = "cpu"

[[right]]
kind = "memory"

[[right]]
kind = "battery"    # auto-hidden on desktops

[theme]
background    = "#1e1e2e"   # Catppuccin Mocha — base
foreground    = "#cdd6f4"   # Catppuccin Mocha — text
accent        = "#cba6f7"   # Catppuccin Mocha — mauve
font          = "JetBrains Mono"
font_size     = 13.0
border_radius = 6.0
padding       = 8
gap           = 4

# Widget pill background — comment out or set to "" for transparent widgets
widget_bg    = "#313244"   # Catppuccin Mocha — surface0
border_color = ""          # bar border hex color; empty = no border
border_width = 0           # bar border width in pixels

clock_format       = "%H:%M"        # strftime format for the time portion
date_format        = "%a %d %b"     # strftime format for the date portion
clock_show_seconds = false          # append :SS to clock
battery_warn_percent = 20           # low battery threshold (highlights in accent)
power_button_style = "icon_label"   # "icon_label" | "icon_only" | "pill"

[dashboard]
enabled = true
theme   = "cards"   # "minimal" | "cards" | "full" | "vivid"
columns = 3         # 2–4 columns in the bento grid

items = [
    "clock", "network", "battery",
    "cpu", "memory", "disk",
    "volume", "brightness", "uptime",
    "temperature", "updates", "swap",
    "load", "gpu", "bluetooth",
    "media", "power",
    # "weather",   # uncomment and set weather_location in [global] to enable
]
```

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for the full reference including per-widget style overrides.

---

## Widgets

All widgets can be placed in `[[left]]`, `[[center]]`, or `[[right]]` — there are no restrictions on position.

| Widget | `kind` key | Description | Interactions |
|---|---|---|---|
| Workspaces | `workspaces` | Clickable Hyprland workspace buttons with active highlight | Click to switch workspace |
| Title | `title` | Active window title, truncated at 60 chars | — |
| Clock | `clock` | Time and date (configurable strftime format); optional seconds | Click to open calendar popup |
| CPU | `cpu` | Average CPU usage % | — |
| Memory | `memory` | RAM used / total | — |
| Network | `network` | Real-time download / upload speed in bytes/sec | — |
| Battery | `battery` | Battery level with icon; auto-hidden on desktops | — |
| Disk | `disk` | Root filesystem used / total; auto-hidden if unavailable | — |
| Temperature | `temperature` | CPU package temperature in °C; auto-hidden if no sensor | — |
| Volume | `volume` | Audio sink volume % with mute indicator (requires `wpctl`) | Scroll to adjust; click to mute/unmute |
| Brightness | `brightness` | Screen brightness %; auto-hidden if no backlight | Scroll to adjust |
| Swap | `swap` | Swap space used / total; auto-hidden if no swap configured | — |
| Uptime | `uptime` | System uptime (e.g. `3d 14h 22m`) | — |
| Load | `load` | 1 / 5 / 15-minute load averages | — |
| GPU | `gpu` | GPU utilization % and temperature; auto-hidden when no GPU detected | — |
| Keyboard | `keyboard` | Active keyboard layout (from Hyprland `activelayout` event) | — |
| Media | `media` | Current track title and artist; requires `playerctl` | — |
| Custom | `custom` | Output of any shell command (set `custom_command` in `[global]`) | — |
| Separator | `separator` | Visual spacer / divider between widget groups | — |
| Notify | `notify` | Notification count badge; click to expand or collapse the drop-down panel | Click to toggle panel |
| Submap | `submap` | Shows the active Hyprland submap in an accent-colored pill; hidden in default map | — |
| Screencast | `screencast` | Pulsing red dot indicator when a screen share is active; auto-hides when inactive | — |
| DND Toggle | `dnd` | Bell icon that crosses out when Do-Not-Disturb is active; click to toggle | Click to toggle DND |
| Updates | `updates` | Package icon with pending update count badge | — |

---

## Dashboard Cards

The bento dashboard overlay (`bar-dashboard`) is a separate binary launched via keybind. Cards are listed under `items` in `[dashboard]` and laid out in the configured column grid.

| Card | `items` key | Description |
|---|---|---|
| Clock | `clock` | Large time and date display |
| Network | `network` | Download / upload speeds |
| Battery | `battery` | Battery level and charge status |
| CPU | `cpu` | CPU usage % with animated sparkline history graph |
| Memory | `memory` | RAM used / total with progress bar |
| Disk | `disk` | Root filesystem usage |
| Volume | `volume` | Current audio volume level |
| Brightness | `brightness` | Screen brightness % |
| Uptime | `uptime` | System uptime |
| Temperature | `temperature` | CPU package temperature |
| Updates | `updates` | Pending package update count |
| Swap | `swap` | Swap used / total with mini progress bar |
| Load | `load` | 1 / 5 / 15-minute load averages; displayed in a 2-wide card |
| GPU | `gpu` | GPU utilization %, temperature, and VRAM used/total; auto-hidden when no GPU detected |
| Bluetooth | `bluetooth` | Adapter on/off status and connected device name |
| Media | `media` | Current track title, artist, and playback controls via `playerctl` |
| Power | `power` | Shortcut buttons for lock, sleep, reboot, and shutdown |
| Weather | `weather` | Live weather conditions from `wttr.in` (set `weather_location` in `[global]` to enable) |

**Sparkline graphs** — CPU usage history and network throughput history are drawn as animated line graphs using the iced Canvas API and appear behind their respective metric cards.

---

## Theming and Ricing

### Default theme: Catppuccin Mocha

The default config ships Catppuccin Mocha colors. Override any value in `[theme]` to customize.

### Shape presets

Set `border_radius` in `[theme]` to control widget corner rounding:

| Preset | `border_radius` | Effect |
|---|---|---|
| Sharp | `0.0` | Square corners |
| Rounded | `8.0` | Subtly rounded |
| Pill | `height / 2` | Fully pill-shaped |

The `bar-editor` GUI exposes these as one-click presets.

### Widget backgrounds

Set `widget_bg` to give each widget a distinct background pill — without it, `border_radius` has no visible effect because there is nothing to round.

```toml
[theme]
widget_bg    = "#313244"   # surface0 from Catppuccin Mocha
border_color = "#cba6f7"   # accent-colored bar border
border_width = 1
```

### Floating bar

```toml
[global]
margin     = 8   # horizontal gap from screen edges
margin_top = 6   # vertical gap from screen edge
```

These are structural changes that require a restart. The GUI editor handles this automatically.

### Font

Set `font` to any font family installed on your system. The editor includes a font picker populated from `fc-list` and a one-click Nerd Font installer (downloads JetBrains Mono Nerd Font to `~/.local/share/fonts/NerdFonts`).

---

## Power Menu

Three display styles are available, set via `power_menu_style` in `[global]`:

| Style | Description |
|---|---|
| `overlay` | Full-screen bento-style popup with large action buttons (default) |
| `dropdown` | Animated panel that drops below the bar |
| `inline` | Power buttons replace the bar content row in-place; a cancel button dismisses |

### Animation

Set `power_anim_style` to control the entrance and exit animation:

| Value | Effect |
|---|---|
| `slide` | Panel slides in from the bar edge |
| `fade` | Opacity crossfade |
| `scale` | Scale-up from center |
| `none` | Instant, no animation |

Animations are driven by `lilt` 0.8.1 with configurable easing. Button hover states animate independently.

### Customizing actions

```toml
[global]
power_actions = ["lock", "sleep", "reboot", "shutdown"]   # omit hibernate and logout
```

---

## Notifications

`bar` registers itself as an `org.freedesktop.Notifications` D-Bus daemon (`zbus 5`). When no other notification daemon is running, it receives notifications directly, stores up to 50 entries in memory, and displays a count badge on the `notify` widget. Clicking the widget toggles a drop-down panel listing all notifications with individual dismiss and clear-all controls.

**Fallback mode:** If another daemon (dunst, mako, swaync) is already registered on D-Bus, `bar` automatically falls back to polling `dunstctl history` every 2 seconds and populating the panel from that output.

---

## GUI Editor (`bar-editor`)

```bash
bar-editor
```

The editor opens as a standard desktop window with three tabs:

- **Global** — bar height, position (top/bottom), opacity, exclusive zone, horizontal and vertical floating margin, system poll interval, lock command
- **Layout** — add, remove, and reorder widgets in the Left, Center, and Right columns; all widget kinds are available from a picker
- **Theme** — full color pickers for background, foreground, accent, widget background, and border; font family and size with live `fc-list` picker and Nerd Font installer; border radius, padding, gap; clock and date format; power button style; battery warning threshold; shape presets (pill / rounded / sharp)

Theme changes apply **live** — the bar reloads via inotify without restarting. Geometry changes (height, position, margins) prompt a save-and-restart from within the editor.

---

## Architecture

```
status_Bar/
├── src/main.rs              — entry point: init tracing, call bar_wayland::run()
├── bar.toml                 — example config (Catppuccin Mocha)
├── crates/
│   ├── core/                — AppState, Message enum, BarWidget trait, BarError
│   ├── config/              — BarConfig TOML schema, load(), ConfigWatcher (inotify)
│   ├── theme/               — Color, Theme, BarStyle, WidgetStyle
│   ├── system/              — async CPU / RAM / disk / network / battery / GPU monitor
│   ├── hyprland-ipc/        — IPC socket client, event parser, WorkspaceInfo
│   ├── widgets/             — all bar widget implementations
│   ├── renderer/            — BarLayout engine (Phase 2)
│   ├── wayland/             — iced-layershell app loop, D-Bus notification daemon
│   ├── editor/              — bar-editor GUI (iced desktop window)
│   └── dashboard/           — bar-dashboard bento overlay (iced-layershell, Layer::Overlay)
└── docs/
    ├── ARCHITECTURE.md      — crate dependency diagram and message flow
    ├── CONFIGURATION.md     — full config field reference
    └── CONTRIBUTING.md      — contribution guide
```

### Key dependencies

| Crate | Version | Role |
|---|---|---|
| `iced` | 0.14 | UI rendering |
| `iced-layershell` | 0.15 | Wayland layer-shell integration |
| `lilt` | 0.8.1 | Animation engine |
| `zbus` | 5 | D-Bus notification daemon |
| `sysinfo` | 0.38 | CPU, RAM, disk, battery stats |
| `notify` | 8 | Config hot-reload via inotify |
| `tokio` | 1 | Async runtime |
| `chrono` | 0.4 | Clock and date formatting |

---

## Roadmap

- [x] Step 1: Workspace scaffold — all crates, root binary
- [x] Step 2: Subscriptions wired (IPC, system monitor, config watcher)
- [x] Step 3: Core widgets (workspaces, title, clock, CPU, RAM, network, battery)
- [x] Step 4: GUI editor, ricing features, floating bar, notification daemon, power menu
- [x] Step 5: Extended widgets (volume, brightness, submap, screencast, DND, GPU, updates)
- [x] Step 6: Bento dashboard overlay with sparklines, weather, Bluetooth, swap, load, GPU cards
- [ ] Step 7: Arch Linux PKGBUILD / AUR package
- [ ] Step 8: Phase 2 — per-monitor layouts, multi-output support, advanced animations

---

## Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run the editor
cargo run -p bar-editor

# Run the dashboard overlay
cargo run -p bar-dashboard

# Check all crates without building
cargo check --workspace

# Run all tests
cargo test --workspace
```

### Adding a widget

1. Create `crates/widgets/src/mywidget.rs` implementing the `view(state: &AppState)` pattern
2. Add `pub mod mywidget;` to `crates/widgets/src/lib.rs`
3. Wire it into `crates/wayland/src/lib.rs` — add an arm to the `match widget.kind.as_str()` block in `view()`
4. Add `"mywidget"` to the `ALL_WIDGETS` slice in `crates/editor/src/main.rs`

---

## Contributing

Contributions are welcome. Please read [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) before opening a pull request.

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes and run `cargo check --workspace` and `cargo test --workspace`
4. Submit a pull request with a clear description of what changed and why

For bugs and feature requests, open an issue on GitHub.

---

## License

MIT — see [LICENSE](LICENSE).

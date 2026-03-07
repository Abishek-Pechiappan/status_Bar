# bar-dashboard

> A Wayland-native bento-grid system dashboard overlay for Hyprland — built in Rust, themed with Catppuccin Mocha out of the box.

[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![iced](https://img.shields.io/badge/iced-0.14-blueviolet)](https://github.com/iced-rs/iced)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

---

## Screenshot / Preview

> Screenshots coming soon.

**bar-dashboard** is a full-screen bento-grid overlay launched from a keybind and dismissed with Escape. It displays live system info cards — CPU history sparklines, GPU stats, network speeds, Bluetooth device, weather, media controls, and more.

---

## Features

- **Full-screen bento grid** — configurable column count (2–4), card order, and display theme (`minimal`, `cards`, `full`, `vivid`)
- **Staggered entrance animation** — cards fade and slide in with EaseOutCubic easing
- **Glassmorphism card style** — frosted semi-transparent cards with white highlight borders
- **Alert glow borders** — cards glow in their semantic color when values are critical (CPU >80%, RAM >85%, temp >75°C, battery low)
- **Sparkline graphs** — animated CPU usage history and network throughput drawn with iced Canvas
- **Weather card** — live conditions from `wttr.in`; set `weather_location` in config to enable
- **Bluetooth card** — adapter status and connected device name via `bluetoothctl`
- **GPU card** — utilization %, temperature, VRAM used/total; auto-hidden when no GPU detected
- **Media controls** — track title, artist, play/pause/skip via `playerctl`
- **Power actions** — lock, sleep, hibernate, logout, reboot, shutdown from within the overlay
- **Volume & brightness sliders** — interactive controls with `wpctl` and `brightnessctl`
- **Keyboard-dismissible** — press Escape to close
- **Catppuccin Mocha** default theme; fully configurable via `bar.toml`

---

## Requirements

### Runtime

| Dependency | Purpose | Notes |
|---|---|---|
| Hyprland | Wayland compositor | Any recent version |
| Nerd Font | Card icons | JetBrains Mono Nerd Font recommended |
| `wpctl` (PipeWire) | Volume card | Part of `pipewire-audio` |
| `brightnessctl` | Brightness card | Optional; card auto-hides |
| `playerctl` | Media card | Optional |
| `bluetoothctl` | Bluetooth card | Optional |
| `curl` | Weather card | Optional; card hidden if `weather_location` is empty |

### Build

- Rust 1.80+ — install via `curl https://sh.rustup.rs | sh`
- `git`
- Standard C linker (provided by `base-devel` on Arch)

---

## Installation

### Build from source

```bash
git clone https://github.com/Abishek-Pechiappan/status_Bar
cd status_Bar
make install
```

This builds `bar-dashboard` in release mode and installs it to `~/.local/bin/bar-dashboard`.

### Update

```bash
make update
```

Rebuilds and reinstalls `bar-dashboard`.

### Manual install

```bash
cargo build --release -p bar-dashboard
install -m755 target/release/bar-dashboard ~/.local/bin/bar-dashboard

# Copy example config (first time only)
mkdir -p ~/.config/bar
cp bar.toml ~/.config/bar/bar.toml
```

### Hyprland keybind

Add to `~/.config/hypr/hyprland.conf`:

```
bind = SUPER, D, exec, bar-dashboard
```

---

## Configuration

The config file lives at `$XDG_CONFIG_HOME/bar/bar.toml` (default: `~/.config/bar/bar.toml`).

### Full config reference

```toml
# Command run when "Lock" is chosen in the power card.
lock_command = "loginctl lock-session"

# City name for the weather card.  Leave empty to hide the card.
weather_location = ""   # e.g. "London" or "48.8566,2.3522"

[theme]
background    = "#1e1e2e"   # Catppuccin Mocha — base
foreground    = "#cdd6f4"   # Catppuccin Mocha — text
accent        = "#cba6f7"   # Catppuccin Mocha — mauve
font          = "JetBrains Mono"
font_size     = 13.0
border_radius = 6.0
padding       = 8
gap           = 4

# Card background color.  Empty string = transparent.
widget_bg    = "#313244"   # Catppuccin Mocha — surface0

clock_format        = "%H:%M"       # strftime format for time
date_format         = "%a %d %b"    # strftime format for date
clock_show_seconds  = false
battery_warn_percent = 20           # low battery glow threshold
power_button_style  = "icon_label"  # "icon_label" | "icon_only" | "pill"

[dashboard]
enabled = true
theme   = "cards"   # "minimal" | "cards" | "full" | "vivid"
columns = 3         # 2–4 columns in the bento grid

# Cards to display — order determines layout position.
items = [
    "clock", "network", "battery",
    "cpu", "memory", "disk",
    "volume", "brightness", "uptime",
    "temperature", "updates", "swap",
    "load", "gpu", "bluetooth",
    "media", "power",
    # "weather",   # uncomment and set weather_location above to enable
]
```

---

## Dashboard Cards

| Card | `items` key | Description |
|---|---|---|
| Clock | `clock` | Large time and date display |
| Network | `network` | Download / upload speeds with sparkline |
| Battery | `battery` | Battery level and charge status |
| CPU | `cpu` | CPU usage % with animated sparkline history graph |
| Memory | `memory` | RAM used / total with progress bar |
| Disk | `disk` | Root filesystem usage |
| Volume | `volume` | Audio volume level with interactive slider |
| Brightness | `brightness` | Screen brightness % with interactive slider |
| Uptime | `uptime` | System uptime |
| Temperature | `temperature` | CPU package temperature |
| Updates | `updates` | Pending package update count |
| Swap | `swap` | Swap used / total with mini progress bar |
| Load | `load` | 1 / 5 / 15-minute load averages (2-wide card) |
| GPU | `gpu` | GPU utilization %, temperature, and VRAM; auto-hidden when no GPU |
| Bluetooth | `bluetooth` | Adapter status and connected device name |
| Media | `media` | Track title, artist, and playback controls via `playerctl` |
| Power | `power` | Lock, sleep, reboot, and shutdown buttons |
| Weather | `weather` | Live weather from `wttr.in` (requires `weather_location`) |

---

## Theming

### Default theme: Catppuccin Mocha

Override any value in `[theme]` to customize. All changes take effect on next launch.

### Card themes

Set `theme` in `[dashboard]`:

| Value | Style |
|---|---|
| `minimal` | Text only, no borders, compact |
| `cards` | Rounded cards with subtle borders (default) |
| `full` | Rich cards with progress bars and semantic colors |
| `vivid` | Bold semantic colors, accent top strips, strong contrast |

### Shape presets

Set `border_radius` in `[theme]`:

| Preset | `border_radius` | Effect |
|---|---|---|
| Sharp | `0.0` | Square corners |
| Rounded | `8.0` | Subtly rounded |
| Pill | `height / 2` | Fully pill-shaped |

### Font

Set `font` to any family installed on your system. Set `icon_style = "ascii"` if Nerd Font icons show as question marks.

---

## Architecture

```
status_Bar/
├── bar.toml                 — example config (Catppuccin Mocha)
├── crates/
│   ├── config/              — DashConfig TOML schema, load(), ConfigWatcher
│   ├── theme/               — Color, Theme (parsed from ThemeConfig)
│   └── dashboard/           — bar-dashboard binary (iced-layershell overlay)
└── docs/
    ├── ARCHITECTURE.md
    ├── CONFIGURATION.md
    └── CONTRIBUTING.md
```

### Key dependencies

| Crate | Version | Role |
|---|---|---|
| `iced` | 0.14 | UI rendering and canvas |
| `iced-layershell` | 0.15 | Wayland layer-shell integration |
| `lilt` | 0.8 | Animation engine |
| `sysinfo` | 0.38 | CPU, RAM, disk, battery stats |
| `tokio` | 1 | Async runtime |
| `chrono` | 0.4 | Clock and date formatting |

---

## Development

```bash
# Debug run
cargo run -p bar-dashboard

# Release build
cargo build --release -p bar-dashboard

# Type-check only
cargo check -p bar-dashboard
```

---

## Contributing

Contributions are welcome. Please read [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) before opening a pull request.

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes and run `cargo check -p bar-dashboard`
4. Submit a pull request with a clear description of what changed and why

---

## License

MIT — see [LICENSE](LICENSE).

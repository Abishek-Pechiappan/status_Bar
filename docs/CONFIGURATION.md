# Configuration Reference

All settings live in `$XDG_CONFIG_HOME/bar/bar.toml` (default: `~/.config/bar/bar.toml`).

The file is watched with inotify — save it and the bar reloads instantly. If the file is missing, built-in defaults are used.

---

## `[global]`

Global settings that apply to every monitor.

| Key | Type | Default | Description |
|---|---|---|---|
| `height` | integer | `40` | Bar height in logical pixels |
| `position` | string | `"top"` | `"top"` or `"bottom"` |
| `exclusive_zone` | bool | `true` | Reserve space so windows don't overlap the bar |
| `opacity` | float | `0.95` | Overall background opacity (`0.0` – `1.0`) |

```toml
[global]
height         = 40
position       = "top"
exclusive_zone = true
opacity        = 0.95
```

---

## Widget Layout

Widgets are declared as arrays of TOML tables under `[[left]]`, `[[center]]`, and `[[right]]`.
They render in the order they appear in the file.

```toml
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
kind = "battery"
```

### Widget entry fields

| Key | Type | Required | Description |
|---|---|---|---|
| `kind` | string | yes | Widget type identifier (see table below) |
| `label` | string | no | Optional display label override |

### Available widget kinds

| Kind | Description |
|---|---|
| `workspaces` | Hyprland workspace list with active highlight |
| `title` | Active window title (max 60 chars, then `…`) |
| `clock` | Current time (`HH:MM`) and date (`Weekday DD Mon`) |
| `cpu` | Average CPU usage across all cores |
| `memory` | RAM used / total and usage percentage |
| `network` | Download (`↓`) and upload (`↑`) rates in human-readable form |
| `battery` | Battery level percentage with charge indicator; hidden automatically when no battery is present |

---

## `[theme]`

Visual styling for the entire bar.

| Key | Type | Default | Description |
|---|---|---|---|
| `background` | hex string | `"#1e1e2e"` | Bar background color |
| `foreground` | hex string | `"#cdd6f4"` | Primary text color |
| `accent` | hex string | `"#cba6f7"` | Highlight / active element color |
| `font` | string | `"JetBrains Mono"` | Font family name |
| `font_size` | float | `13.0` | Font size in points |
| `border_radius` | float | `6.0` | Widget container corner radius in pixels |
| `padding` | integer | `8` | Inner padding for each widget in pixels |
| `gap` | integer | `4` | Gap between widgets in pixels |

```toml
[theme]
background    = "#1e1e2e"   # Catppuccin Mocha — base
foreground    = "#cdd6f4"   # Catppuccin Mocha — text
accent        = "#cba6f7"   # Catppuccin Mocha — mauve
font          = "JetBrains Mono"
font_size     = 13.0
border_radius = 6.0
padding       = 8
gap           = 4
```

---

## `[monitors.<name>]`

Per-monitor overrides. The key is the Wayland output name (e.g. `DP-1`, `HDMI-A-1`).
Unset fields fall back to `[global]`.

```toml
[monitors."DP-2"]
height   = 36
position = "bottom"
```

---

## Popular themes

### Catppuccin Mocha (default)

```toml
[theme]
background = "#1e1e2e"
foreground = "#cdd6f4"
accent     = "#cba6f7"
```

### Catppuccin Latte

```toml
[theme]
background = "#eff1f5"
foreground = "#4c4f69"
accent     = "#8839ef"
```

### Gruvbox Dark

```toml
[theme]
background = "#282828"
foreground = "#ebdbb2"
accent     = "#fabd2f"
```

### Tokyo Night

```toml
[theme]
background = "#1a1b26"
foreground = "#c0caf5"
accent     = "#7aa2f7"
```

### Nord

```toml
[theme]
background = "#2e3440"
foreground = "#eceff4"
accent     = "#88c0d0"
```

---

## Full example

```toml
[global]
height         = 40
position       = "top"
exclusive_zone = true
opacity        = 0.95

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
kind = "battery"

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

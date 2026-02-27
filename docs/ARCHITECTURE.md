# Architecture

## Overview

`bar` uses an **event-driven, workspace-based** Rust architecture.

```
┌──────────────────────────────────────────────────────┐
│                     bar (binary)                     │
│  initialises logging → calls bar_wayland::run()      │
└────────────────────┬─────────────────────────────────┘
                     │
          ┌──────────▼──────────┐
          │     bar-wayland     │  ← iced-layershell app loop
          │  Message enum       │  ← #[to_layer_message]
          │  Bar struct (state) │
          └──┬──────┬──────┬───┘
             │      │      │
    ┌────────▼─┐ ┌──▼───┐ ┌▼────────────┐
    │bar-widgets│ │bar-  │ │bar-system   │
    │ clock     │ │theme │ │ spawn_monitor│
    │ workspaces│ │      │ │ CpuHistory  │
    │ cpu       │ └──────┘ └─────────────┘
    └──────────┘
         │
    ┌────▼──────────┐   ┌──────────────┐   ┌────────────┐
    │  bar-core     │   │  bar-config  │   │  bar-ipc   │
    │  AppState     │   │  BarConfig   │   │ HyprlandIpc│
    │  Message enum │   │  load()      │   │ parse_event│
    │  BarWidget    │   │  ConfigWatch │   │            │
    └───────────────┘   └──────────────┘   └────────────┘
```

## Crates

| Crate | Responsibility |
|---|---|
| `bar-core` | Central types: `AppState`, `Message`, `BarWidget` trait, errors |
| `bar-config` | TOML parsing, schema, live-reload watcher |
| `bar-theme` | Color parsing, compiled `Theme` struct, style helpers |
| `bar-system` | Async system monitor (CPU/RAM/disk via `sysinfo`) |
| `bar-ipc` | Hyprland IPC client (event socket + command socket) |
| `bar-widgets` | Clock, Workspaces, CPU/RAM widgets (pure Iced views) |
| `bar-renderer` | Layout engine (Phase 1: stub; Phase 2: full layout) |
| `bar-wayland` | iced-layershell surface, app loop, subscriptions |

## Message Flow

```
[timer tick]    ──► Message::Tick  ──► Bar::update ──► state.time = now
[IPC socket]    ──► HyprlandEvent  ──► Message::App(WorkspaceChanged) ──► Bar::update
[system monitor]──► SystemSnapshot ──► Message::App(SystemSnapshot)   ──► Bar::update
[config watcher]──►                ──► Message::App(ConfigReloaded)   ──► Bar::update
```

## Adding a Widget

1. Create `crates/widgets/src/my_widget.rs`
2. Implement `BarWidget` trait (for `bar-core` integration)
3. Add a `view()` method returning `Element<'_, Message>`
4. Export from `crates/widgets/src/lib.rs`
5. Add to `Bar::view()` in `crates/wayland/src/lib.rs`
6. Register in `bar.toml` with `kind = "my_widget"`

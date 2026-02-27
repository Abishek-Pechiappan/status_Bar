
---

## 🎯 PROJECT OVERVIEW

Create a **production-grade, modular, Wayland-native status bar for Hyprland** that provides:

- Advanced ricing & visual customization
    
- Deep Hyprland IPC integration
    
- Power-user system monitoring
    
- GUI-based layout editor (later phase)
    
- Plugin/widget system (extensible)
    
- High performance and low resource usage
    

The application must be:

- Written with clean architecture
    
- Fully documented
    
- Testable
    
- Packaged for Arch Linux
    
- GitHub portfolio quality
    

---

## 🏗️ CORE TECH STACK

### Language

Rust (stable)

### Rendering / UI

Iced (primary UI framework)

### Wayland Integration

smithay-client-toolkit  
wayland-client  
wlr-layer-shell protocol

### Async Runtime

Tokio

### Serialization

Serde (TOML for config)

### System Information

sysinfo

### Hyprland IPC

Unix socket communication (JSON events)

### Logging

tracing + tracing-subscriber

### Error Handling

thiserror + anyhow

### Configuration

TOML with live reload

### Styling Engine

Custom theming layer (not CSS)

---

## 📦 PROJECT STRUCTURE

Use a workspace-based structure:

```
hyprbar/
 ├─ crates/
 │   ├─ core/            → app state, event system
 │   ├─ wayland/         → layer-shell surface
 │   ├─ hyprland-ipc/    → IPC client
 │   ├─ system/          → system stats
 │   ├─ widgets/         → built-in widgets
 │   ├─ renderer/        → layout & drawing
 │   ├─ config/          → config parsing & live reload
 │   └─ theme/           → styling engine
 │
 ├─ assets/
 ├─ examples/
 ├─ docs/
 ├─ hyprbar.toml
 └─ Cargo.toml
```

---

## 🧱 ARCHITECTURE REQUIREMENTS

Use an **event-driven architecture**.

### Core concepts

- Central AppState
    
- Message/Event bus
    
- Widget trait system
    
- Reactive updates (no polling when possible)
    

---

## 🧩 WIDGET SYSTEM DESIGN

Each widget must:

- Implement a common trait
    
- Receive context
    
- Emit update messages
    
- Support styling
    
- Support interactivity
    

Example responsibilities:

- Workspace widget (Hyprland IPC driven)
    
- Clock widget (time updates)
    
- CPU widget (system module driven)
    
- Media widget (MPRIS in later phase)
    

---

## 🎨 RICING / VISUAL SYSTEM

Support:

- Per-widget styling
    
- Animations (state transitions)
    
- Dynamic colors
    
- Rounded containers
    
- Layout spacing engine
    

Theme must be:

- Declarative
    
- Hot reloadable
    

---

## ⚙️ CONFIG SYSTEM

TOML format.

Must support:

- Multiple monitors
    
- Widget layout
    
- Theme selection
    
- Conditional rendering rules
    

Hot reload using filesystem watcher.

---

## 🔌 HYPRLAND INTEGRATION

Implement:

- Active workspace tracking
    
- Workspace list
    
- Active window title
    
- Fullscreen state
    
- Monitor info
    

Use event-driven IPC — no polling.

---

## 📊 SYSTEM MODULES

Provide:

- CPU usage (per core)
    
- RAM usage
    
- Disk usage
    
- Network stats
    
- Temperature sensors (extensible)
    

All updates must be async and efficient.

---

## 🖥️ WAYLAND LAYER-SHELL

Create:

- Top bar surface
    
- Per-monitor instance
    
- Exclusive zone handling
    
- DPI awareness
    

---

## 🚀 PERFORMANCE REQUIREMENTS

The bar must:

- Use < 150MB RAM
    
- Use minimal CPU while idle
    
- Avoid unnecessary redraws
    
- Use batched updates
    

---

## 🧪 TESTING

Include:

- Unit tests for config parsing
    
- Widget state tests
    
- IPC message parsing tests
    

---

## 📚 DOCUMENTATION

Provide:

- README.md
    
- ARCHITECTURE.md
    
- CONTRIBUTING.md
    
- CONFIGURATION.md
    

---

## 📦 ARCH LINUX PACKAGING

Prepare:

- PKGBUILD
    
- Install paths
    
- Example config in `/etc/xdg/`
    

---

## 🎯 MVP FEATURES (PHASE 1)

Implement first:

- Wayland bar window
    
- Workspace widget
    
- Clock widget
    
- CPU widget
    
- Basic theme
    
- Config loader
    

No GUI editor yet.

---

## 🌟 PHASE 2 FEATURES

- Animation system
    
- Interactive widgets
    
- Per-monitor layouts
    
- Advanced theming
    

---

## 🧠 PHASE 3 FEATURES

- GUI layout editor
    
- Plugin system
    
- Widget marketplace
    

---

## 🧾 CODE QUALITY RULES

- Follow Rust idioms
    
- No unwrap() in production code
    
- Proper error propagation
    
- Modular design
    
- Full type safety
    

---

## 🧰 DEV EXPERIENCE

Provide:

- `cargo run --example minimal`
    
- Sample config
    
- Hot reload in dev mode
    
- Debug logging mode
    

---

## 📍 OUTPUT REQUIREMENTS FOR THE LLM

Generate:

1. Step-by-step implementation plan
    
2. File-by-file code
    
3. Build instructions
    
4. Run instructions
    
5. Example config
    
6. Screenshots mock description (for README)
    

---

# 🔥 HOW YOU WILL USE THIS

In future you will say:

> “Step 1: Initialize the workspace”

or

> “Generate the core crate”

and the LLM will output **real production code**, not tutorials.

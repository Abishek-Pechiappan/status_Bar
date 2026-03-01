use bar_config::{default_path, load as load_config, BarConfig, Position, WidgetConfig};
use iced::{
    widget::{button, checkbox, column, container, mouse_area, pick_list, row, rule, scrollable, slider, text, text_input},
    Alignment, Color, Element, Length, Size, Subscription, Task,
};
use std::path::PathBuf;

const ALL_WIDGETS: &[&str] = &[
    "workspaces", "title", "clock",
    "cpu", "memory", "network", "battery",
    "disk", "temperature", "volume", "brightness",
    "swap", "uptime", "load", "keyboard", "media", "custom",
    "separator", "notify",
];

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    iced::application(Editor::new, Editor::update, Editor::view)
        .title("Bar Editor")
        .subscription(Editor::subscription)
        .window_size(Size::new(860.0, 700.0))
        .run()
}

// ── Color field identifiers (for the colour picker) ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorField {
    Background,
    Foreground,
    Accent,
    WidgetBg,
    BorderColor,
    WidgetBorderColor,
}

// ── Theme presets ─────────────────────────────────────────────────────────────

struct ThemePreset {
    name:       &'static str,
    background: &'static str,
    foreground: &'static str,
    accent:     &'static str,
}

const THEME_PRESETS: &[ThemePreset] = &[
    ThemePreset { name: "Catppuccin Mocha",  background: "#1e1e2e", foreground: "#cdd6f4", accent: "#cba6f7" },
    ThemePreset { name: "Catppuccin Latte",  background: "#eff1f5", foreground: "#4c4f69", accent: "#8839ef" },
    ThemePreset { name: "Catppuccin Frappe", background: "#303446", foreground: "#c6d0f5", accent: "#ca9ee6" },
    ThemePreset { name: "Gruvbox Dark",      background: "#282828", foreground: "#ebdbb2", accent: "#fabd2f" },
    ThemePreset { name: "Gruvbox Light",     background: "#fbf1c7", foreground: "#3c3836", accent: "#d79921" },
    ThemePreset { name: "Tokyo Night",       background: "#1a1b26", foreground: "#c0caf5", accent: "#7aa2f7" },
    ThemePreset { name: "Nord",              background: "#2e3440", foreground: "#eceff4", accent: "#88c0d0" },
    ThemePreset { name: "Dracula",           background: "#282a36", foreground: "#f8f8f2", accent: "#bd93f9" },
    ThemePreset { name: "Rose Pine",         background: "#191724", foreground: "#e0def4", accent: "#c4a7e7" },
    ThemePreset { name: "Rose Pine Dawn",    background: "#faf4ed", foreground: "#575279", accent: "#907aa9" },
    ThemePreset { name: "One Dark",          background: "#282c34", foreground: "#abb2bf", accent: "#61afef" },
    ThemePreset { name: "Solarized Dark",    background: "#002b36", foreground: "#839496", accent: "#268bd2" },
    ThemePreset { name: "Everforest Dark",   background: "#2d353b", foreground: "#d3c6aa", accent: "#a7c080" },
    ThemePreset { name: "Kanagawa",          background: "#1f1f28", foreground: "#dcd7ba", accent: "#7e9cd8" },
];

// ── Save status ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
enum SaveStatus {
    #[default]
    Idle,
    Saved,
    Restarting,
    Error(String),
}

// ── Sections ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Global,
    Layout,
    Theme,
}

// ── Side of the bar ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Center,
    Right,
}

impl Side {
    fn index(self) -> usize {
        match self {
            Side::Left   => 0,
            Side::Center => 1,
            Side::Right  => 2,
        }
    }

    fn widgets_mut(self, cfg: &mut BarConfig) -> &mut Vec<WidgetConfig> {
        match self {
            Side::Left   => &mut cfg.left,
            Side::Center => &mut cfg.center,
            Side::Right  => &mut cfg.right,
        }
    }

    fn widgets(self, cfg: &BarConfig) -> &[WidgetConfig] {
        match self {
            Side::Left   => &cfg.left,
            Side::Center => &cfg.center,
            Side::Right  => &cfg.right,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

struct Editor {
    config:              BarConfig,
    config_path:         PathBuf,
    section:             Section,
    save_status:         SaveStatus,
    /// Height/position/margins at the time the bar was last launched — used to
    /// detect structural changes that require a full process restart.
    launched_height:     u32,
    launched_position:   Position,
    launched_margin:     u32,
    launched_margin_top: u32,
    // Per-column "kind to add" selection
    new_kind:            [&'static str; 3],
    // Buffered inputs so invalid hex doesn't clobber config mid-type
    bg_buf:              String,
    fg_buf:              String,
    accent_buf:          String,
    font_buf:            String,
    widget_bg_buf:       String,
    border_color_buf:          String,
    widget_border_color_buf:   String,
    clock_format_buf:          String,
    date_format_buf:     String,
    // Colour picker state
    active_picker: Option<ColorField>,
    /// HSV of the last colour cell clicked in the grid.
    picker_h:     f32,
    picker_s:     f32,
    picker_v:     f32,
    /// Saturation scale (0 = grey, 1 = full grid saturation).
    picker_sat:   f32,
    /// Alpha / opacity (0 = transparent, 1 = opaque).
    picker_alpha: f32,
    /// `true` when a non-structural change is waiting to be auto-saved.
    pending_autosave: bool,
}

// ── Messages ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    Tab(Section),

    // Global settings
    HeightChanged(f32),
    PositionChanged(Position),
    OpacityChanged(f32),
    ExclusiveZoneToggled(bool),
    MarginChanged(f32),
    MarginTopChanged(f32),

    // Layout
    MoveUp   { side: Side, i: usize },
    MoveDown { side: Side, i: usize },
    Remove   { side: Side, i: usize },
    NewKind  { side: Side, kind: &'static str },
    Add(Side),

    // Theme
    BgChanged(String),
    FgChanged(String),
    AccentChanged(String),
    FontChanged(String),
    FontSizeChanged(f32),
    RadiusChanged(f32),
    PaddingChanged(f32),
    GapChanged(f32),
    WidgetBgChanged(String),
    BorderColorChanged(String),
    BorderWidthChanged(f32),
    ClockFormatChanged(String),
    DateFormatChanged(String),
    UseNerdIcons(bool),
    WidgetPadXChanged(f32),
    WidgetPadYChanged(f32),
    WorkspaceStyle(bool),   // true = dots, false = numbers
    WorkspaceShowAll(bool), // true = all, false = active only
    NetworkShowSpeed(bool),
    NetworkShowName(bool),
    NetworkShowSignal(bool),
    WidgetBorderColorChanged(String),
    WidgetBorderWidthChanged(f32),
    // Colour picker
    TogglePicker(ColorField),
    ColorGridPicked(f32, f32, f32),  // h, s, v from the grid cell
    PickerSat(f32),
    PickerAlpha(f32),
    ApplyThemePreset(usize),
    ImportWal,
    ResetDefaults,

    // Actions
    Save,
    AutoSaveTick,
    KeyEvent(iced::keyboard::Event),
}

// ── Init ──────────────────────────────────────────────────────────────────────

impl Editor {
    fn new() -> (Self, Task<Message>) {
        let config_path         = default_path();
        let config              = load_config(&config_path).unwrap_or_default();
        let bg_buf              = config.theme.background.clone();
        let fg_buf              = config.theme.foreground.clone();
        let accent_buf          = config.theme.accent.clone();
        let font_buf            = config.theme.font.clone();
        let widget_bg_buf       = config.theme.widget_bg.clone();
        let border_color_buf          = config.theme.border_color.clone();
        let widget_border_color_buf   = config.theme.widget_border_color.clone();
        let clock_format_buf          = config.theme.clock_format.clone();
        let date_format_buf     = config.theme.date_format.clone();
        let launched_height     = config.global.height;
        let launched_position   = config.global.position;
        let launched_margin     = config.global.margin;
        let launched_margin_top = config.global.margin_top;

        (
            Self {
                config,
                config_path,
                section:             Section::Global,
                save_status:         SaveStatus::Idle,
                launched_height,
                launched_position,
                launched_margin,
                launched_margin_top,
                new_kind:            ["workspaces", "clock", "cpu"],
                bg_buf,
                fg_buf,
                accent_buf,
                font_buf,
                widget_bg_buf,
                border_color_buf,
                widget_border_color_buf,
                clock_format_buf,
                date_format_buf,
                active_picker:    None,
                picker_h:         220.0,
                picker_s:         1.0,
                picker_v:         0.8,
                picker_sat:       1.0,
                picker_alpha:     1.0,
                pending_autosave: false,
            },
            Task::none(),
        )
    }
}

// ── Save logic ────────────────────────────────────────────────────────────────

impl Editor {
    fn do_save(&mut self) {
        let structural_change = self.config.global.height     != self.launched_height
                             || self.config.global.position   != self.launched_position
                             || self.config.global.margin     != self.launched_margin
                             || self.config.global.margin_top != self.launched_margin_top;

        match save_config(&self.config, &self.config_path) {
            Err(e) => self.save_status = SaveStatus::Error(e),
            Ok(()) => {
                if structural_change {
                    self.launched_height     = self.config.global.height;
                    self.launched_position   = self.config.global.position;
                    self.launched_margin     = self.config.global.margin;
                    self.launched_margin_top = self.config.global.margin_top;
                    self.save_status = SaveStatus::Restarting;
                    std::thread::spawn(|| {
                        let _ = std::process::Command::new("pkill")
                            .args(["-x", "bar"])
                            .status();
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        let _ = std::process::Command::new("bar").spawn();
                    });
                } else {
                    self.save_status = SaveStatus::Saved;
                }
            }
        }
    }

    fn sync_bufs(&mut self) {
        self.bg_buf           = self.config.theme.background.clone();
        self.fg_buf           = self.config.theme.foreground.clone();
        self.accent_buf       = self.config.theme.accent.clone();
        self.font_buf         = self.config.theme.font.clone();
        self.widget_bg_buf    = self.config.theme.widget_bg.clone();
        self.border_color_buf        = self.config.theme.border_color.clone();
        self.widget_border_color_buf = self.config.theme.widget_border_color.clone();
        self.clock_format_buf        = self.config.theme.clock_format.clone();
        self.date_format_buf  = self.config.theme.date_format.clone();
        self.active_picker = None; // close picker when presets/reset are applied
        self.picker_sat    = 1.0;
        self.picker_alpha  = 1.0;
    }

    /// Recompute the colour from stored HSV + saturation scale + alpha and
    /// write it back to whichever colour field the picker is open for.
    fn apply_grid_color(&mut self) {
        if self.active_picker.is_none() { return; }
        let s = (self.picker_s * self.picker_sat).clamp(0.0, 1.0);
        let (r, g, b) = hsv_to_rgb(self.picker_h, s, self.picker_v);
        let hex = if self.picker_alpha < 0.995 {
            let a = (self.picker_alpha * 255.0).round() as u8;
            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}")
        };
        match self.active_picker {
            Some(ColorField::Background) => {
                self.bg_buf = hex.clone();
                self.config.theme.background = hex;
            }
            Some(ColorField::Foreground) => {
                self.fg_buf = hex.clone();
                self.config.theme.foreground = hex;
            }
            Some(ColorField::Accent) => {
                self.accent_buf = hex.clone();
                self.config.theme.accent = hex;
            }
            Some(ColorField::WidgetBg) => {
                self.widget_bg_buf = hex.clone();
                self.config.theme.widget_bg = hex;
            }
            Some(ColorField::BorderColor) => {
                self.border_color_buf = hex.clone();
                self.config.theme.border_color = hex;
            }
            Some(ColorField::WidgetBorderColor) => {
                self.widget_border_color_buf = hex.clone();
                self.config.theme.widget_border_color = hex;
            }
            None => {}
        }
    }
}

// ── Subscription ─────────────────────────────────────────────────────────────

impl Editor {
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::keyboard::listen().map(Message::KeyEvent),
            iced::time::every(std::time::Duration::from_millis(400))
                .map(|_| Message::AutoSaveTick),
        ])
    }
}

// ── Update ────────────────────────────────────────────────────────────────────

impl Editor {
    fn update(&mut self, msg: Message) -> Task<Message> {
        // Clear save status on any interaction except save-related or timer messages.
        if !matches!(
            msg,
            Message::Save | Message::Tab(_) | Message::TogglePicker(_) | Message::AutoSaveTick
        ) {
            self.save_status = SaveStatus::Idle;
        }

        // Mark a pending auto-save for any non-structural, non-UI message.
        // Structural changes (height/position/margins) need a manual Save+restart
        // so we deliberately exclude them from auto-save triggering.
        if !matches!(
            msg,
            Message::HeightChanged(_)
                | Message::PositionChanged(_)
                | Message::MarginChanged(_)
                | Message::MarginTopChanged(_)
                | Message::Tab(_)
                | Message::Save
                | Message::AutoSaveTick
                | Message::KeyEvent(_)
                | Message::TogglePicker(_)
        ) {
            self.pending_autosave = true;
        }

        match msg {
            Message::Tab(s) => self.section = s,

            // ── Global ──────────────────────────────────────────────────────
            Message::HeightChanged(v)        => self.config.global.height         = v as u32,
            Message::PositionChanged(p)      => self.config.global.position       = p,
            Message::OpacityChanged(v)       => self.config.global.opacity        = v,
            Message::ExclusiveZoneToggled(b) => self.config.global.exclusive_zone = b,
            Message::MarginChanged(v)        => self.config.global.margin         = v as u32,
            Message::MarginTopChanged(v)     => self.config.global.margin_top     = v as u32,

            // ── Layout ──────────────────────────────────────────────────────
            Message::MoveUp { side, i } => {
                let list = side.widgets_mut(&mut self.config);
                if i > 0 { list.swap(i - 1, i); }
            }
            Message::MoveDown { side, i } => {
                let list = side.widgets_mut(&mut self.config);
                if i + 1 < list.len() { list.swap(i, i + 1); }
            }
            Message::Remove { side, i } => {
                let list = side.widgets_mut(&mut self.config);
                if i < list.len() { list.remove(i); }
            }
            Message::NewKind { side, kind } => self.new_kind[side.index()] = kind,
            Message::Add(side) => {
                let kind = self.new_kind[side.index()].to_string();
                side.widgets_mut(&mut self.config).push(WidgetConfig::new(kind));
            }

            // ── Theme ────────────────────────────────────────────────────────
            Message::BgChanged(s) => {
                self.bg_buf = s.clone();
                if s.is_empty() || is_valid_hex(&s) { self.config.theme.background = s; }
            }
            Message::FgChanged(s) => {
                self.fg_buf = s.clone();
                if is_valid_hex(&s) { self.config.theme.foreground = s; }
            }
            Message::AccentChanged(s) => {
                self.accent_buf = s.clone();
                if is_valid_hex(&s) { self.config.theme.accent = s; }
            }
            Message::FontChanged(s) => {
                self.font_buf = s.clone();
                self.config.theme.font = s;
            }
            Message::FontSizeChanged(v)     => self.config.theme.font_size     = v,
            Message::RadiusChanged(v)       => self.config.theme.border_radius = v,
            Message::PaddingChanged(v)      => self.config.theme.padding       = v as u16,
            Message::GapChanged(v)          => self.config.theme.gap           = v as u16,

            Message::WidgetBgChanged(s) => {
                self.widget_bg_buf = s.clone();
                self.config.theme.widget_bg = s;
            }
            Message::BorderColorChanged(s) => {
                self.border_color_buf = s.clone();
                if s.is_empty() || is_valid_hex(&s) {
                    self.config.theme.border_color = s;
                }
            }
            Message::BorderWidthChanged(v) => self.config.theme.border_width = v as u32,
            Message::WidgetBorderColorChanged(s) => {
                self.widget_border_color_buf = s.clone();
                if s.is_empty() || is_valid_hex(&s) {
                    self.config.theme.widget_border_color = s;
                }
            }
            Message::WidgetBorderWidthChanged(v) => self.config.theme.widget_border_width = v as u32,

            Message::ClockFormatChanged(s) => {
                self.clock_format_buf = s.clone();
                self.config.theme.clock_format = s;
            }
            Message::DateFormatChanged(s) => {
                self.date_format_buf = s.clone();
                self.config.theme.date_format = s;
            }

            Message::UseNerdIcons(b) => {
                self.config.theme.icon_style = if b { "nerd".to_string() } else { "ascii".to_string() };
            }
            Message::WidgetPadXChanged(v) => self.config.theme.widget_padding_x = v as u16,
            Message::WidgetPadYChanged(v) => self.config.theme.widget_padding_y = v as u16,
            Message::WorkspaceStyle(dots) => {
                self.config.theme.workspace_style =
                    if dots { "dots".to_string() } else { "numbers".to_string() };
            }
            Message::WorkspaceShowAll(all) => self.config.theme.workspace_show_all = all,
            Message::NetworkShowSpeed(v) => toggle_network_show(&mut self.config.theme.network_show, "speed",  v),
            Message::NetworkShowName(v)  => toggle_network_show(&mut self.config.theme.network_show, "name",   v),
            Message::NetworkShowSignal(v)=> toggle_network_show(&mut self.config.theme.network_show, "signal", v),

            // ── Colour picker ────────────────────────────────────────────────
            Message::TogglePicker(field) => {
                if self.active_picker == Some(field) {
                    self.active_picker = None;
                } else {
                    // Restore alpha from the current value if it's an 8-char hex.
                    let hex = match field {
                        ColorField::Background        => &self.config.theme.background,
                        ColorField::Foreground        => &self.config.theme.foreground,
                        ColorField::Accent            => &self.config.theme.accent,
                        ColorField::WidgetBg          => &self.config.theme.widget_bg,
                        ColorField::BorderColor       => &self.config.theme.border_color,
                        ColorField::WidgetBorderColor => &self.config.theme.widget_border_color,
                    };
                    let trimmed = hex.trim_start_matches('#');
                    self.picker_alpha = if trimmed.len() == 8 {
                        u8::from_str_radix(&trimmed[6..8], 16)
                            .map(|a| a as f32 / 255.0)
                            .unwrap_or(1.0)
                    } else {
                        1.0
                    };
                    self.picker_sat = 1.0;
                    self.active_picker = Some(field);
                }
            }
            Message::ColorGridPicked(h, s, v) => {
                self.picker_h = h;
                self.picker_s = s;
                self.picker_v = v;
                self.apply_grid_color();
            }
            Message::PickerSat(v)   => { self.picker_sat   = v; self.apply_grid_color(); }
            Message::PickerAlpha(v) => { self.picker_alpha = v; self.apply_grid_color(); }

            Message::ApplyThemePreset(idx) => {
                if let Some(p) = THEME_PRESETS.get(idx) {
                    self.config.theme.background = p.background.to_string();
                    self.config.theme.foreground = p.foreground.to_string();
                    self.config.theme.accent     = p.accent.to_string();
                    self.sync_bufs();
                }
            }

            Message::ImportWal => {
                if let Some((bg, fg, ac)) = load_wal_colors() {
                    self.config.theme.background = bg;
                    self.config.theme.foreground = fg;
                    self.config.theme.accent     = ac;
                    self.sync_bufs();
                } else {
                    self.save_status = SaveStatus::Error(
                        "~/.cache/wal/colors.json not found or invalid".to_string()
                    );
                }
            }

            Message::ResetDefaults => {
                let defaults = BarConfig::default();
                self.config = defaults;
                self.sync_bufs();
                self.save_status = SaveStatus::Idle;
            }

            // ── Auto-save (fires every 400 ms) ───────────────────────────────
            Message::AutoSaveTick => {
                if self.pending_autosave {
                    let has_structural =
                        self.config.global.height     != self.launched_height
                        || self.config.global.position   != self.launched_position
                        || self.config.global.margin     != self.launched_margin
                        || self.config.global.margin_top != self.launched_margin_top;
                    if !has_structural {
                        if let Err(e) = save_config(&self.config, &self.config_path) {
                            self.save_status = SaveStatus::Error(e);
                        }
                        self.pending_autosave = false;
                    }
                    // If structural changes are pending, leave pending_autosave = true
                    // so it fires once after the user clicks Save+restart.
                }
            }

            // ── Save ─────────────────────────────────────────────────────────
            Message::Save => {
                self.do_save();
                self.pending_autosave = false;
            }

            // ── Keyboard shortcuts ───────────────────────────────────────────
            Message::KeyEvent(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                if modifiers.command() {
                    if let iced::keyboard::Key::Character(c) = &key {
                        if c.as_str() == "s" {
                            self.do_save();
                        }
                    }
                }
            }
            Message::KeyEvent(_) => {}
        }

        Task::none()
    }
}

// ── View ──────────────────────────────────────────────────────────────────────

impl Editor {
    fn view(&self) -> Element<'_, Message> {
        let preview = self.view_preview();

        let tabs = row![
            tab_btn("Global", Section::Global, self.section),
            tab_btn("Layout", Section::Layout, self.section),
            tab_btn("Theme",  Section::Theme,  self.section),
        ]
        .spacing(4);

        let body: Element<'_, Message> = match self.section {
            Section::Global => self.view_global(),
            Section::Layout => self.view_layout(),
            Section::Theme  => self.view_theme(),
        };

        let has_structural =
            self.config.global.height     != self.launched_height
            || self.config.global.position   != self.launched_position
            || self.config.global.margin     != self.launched_margin
            || self.config.global.margin_top != self.launched_margin_top;

        let status: Element<'_, Message> = if has_structural {
            text("⟲ Save required — geometry changes need a bar restart")
                .size(12.0)
                .color(Color::from_rgb8(0xf9, 0xe2, 0xaf))
                .into()
        } else {
            match &self.save_status {
                SaveStatus::Idle        => text("● Theme changes apply live automatically")
                    .size(12.0)
                    .color(Color::from_rgb8(0x6c, 0x70, 0x86))
                    .into(),
                SaveStatus::Saved       => text("✓ Saved")
                    .color(Color::from_rgb8(0xa6, 0xe3, 0xa1))
                    .into(),
                SaveStatus::Restarting  => text("✓ Saved — restarting bar…")
                    .color(Color::from_rgb8(0x89, 0xb4, 0xfa))
                    .into(),
                SaveStatus::Error(e)    => text(format!("✗ {e}"))
                    .color(Color::from_rgb8(0xf3, 0x8b, 0xa8))
                    .into(),
            }
        };

        let footer = row![
            button(text("Save")).on_press(Message::Save),
            button(text("Reset Defaults"))
                .on_press(Message::ResetDefaults)
                .style(iced::widget::button::danger),
            text(format!("  {}", self.config_path.display()))
                .size(10.0)
                .color(Color::from_rgb8(0x6c, 0x70, 0x86)),
            status,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        column![
            preview,
            tabs,
            rule::horizontal(1.0f32),
            scrollable(
                container(body).padding(12)
            )
            .height(Length::Fill),
            rule::horizontal(1.0f32),
            container(footer).padding([8, 0]),
        ]
        .padding(12)
        .spacing(8)
        .into()
    }

    // ── Global section ────────────────────────────────────────────────────────

    fn view_global(&self) -> Element<'_, Message> {
        let g = &self.config.global;

        column![
            section_header("⟲  Requires bar restart on save"),
            labeled_row(
                "Height",
                row![
                    slider(20.0f32..=100.0, g.height as f32, Message::HeightChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", g.height)).width(60),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Position",
                row![
                    pos_btn("Top",    Position::Top,    g.position),
                    pos_btn("Bottom", Position::Bottom, g.position),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "H. Margin",
                row![
                    slider(0.0f32..=100.0, g.margin as f32, Message::MarginChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", g.margin)).width(60),
                    text("floating bar").size(11.0).color(Color::from_rgb8(0x6c, 0x70, 0x86)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "V. Margin",
                row![
                    slider(0.0f32..=40.0, g.margin_top as f32, Message::MarginTopChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", g.margin_top)).width(60),
                    text("floating bar").size(11.0).color(Color::from_rgb8(0x6c, 0x70, 0x86)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            section_header("Live — applies immediately"),
            labeled_row(
                "Opacity",
                row![
                    slider(0.0f32..=1.0, g.opacity, Message::OpacityChanged)
                        .step(0.01f32)
                        .width(200),
                    text(format!("{:.0}%", g.opacity * 100.0)).width(50),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Exclusive Zone",
                checkbox(g.exclusive_zone)
                    .label("Reserve space so windows don't overlap the bar")
                    .on_toggle(Message::ExclusiveZoneToggled),
            ),
        ]
        .spacing(20)
        .into()
    }

    // ── Layout section ────────────────────────────────────────────────────────

    fn view_layout(&self) -> Element<'_, Message> {
        row![
            self.widget_column(Side::Left,   "Left"),
            self.widget_column(Side::Center, "Center"),
            self.widget_column(Side::Right,  "Right"),
        ]
        .spacing(16)
        .into()
    }

    fn widget_column(&self, side: Side, title: &'static str) -> Element<'_, Message> {
        let widgets = side.widgets(&self.config);
        let len     = widgets.len();

        let mut col = column![
            text(title).size(15),
            rule::horizontal(1.0f32),
        ]
        .spacing(6)
        .width(Length::Fill);

        for (i, w) in widgets.iter().enumerate() {
            let row_el: Element<'_, Message> = row![
                text(&w.kind).width(Length::Fill),
                button(text("↑")).on_press_maybe(
                    (i > 0).then(|| Message::MoveUp { side, i })
                ),
                button(text("↓")).on_press_maybe(
                    (i + 1 < len).then(|| Message::MoveDown { side, i })
                ),
                button(text("×")).on_press(Message::Remove { side, i }),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
            .into();

            col = col.push(row_el);
        }

        let idx = side.index();
        let add_row: Element<'_, Message> = row![
            pick_list(
                ALL_WIDGETS,
                Some(self.new_kind[idx]),
                move |k: &'static str| Message::NewKind { side, kind: k },
            )
            .width(Length::Fill),
            button(text("Add")).on_press(Message::Add(side)),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into();

        col.push(add_row).into()
    }

    // ── Theme preview strip ───────────────────────────────────────────────────

    fn view_preview(&self) -> Element<'_, Message> {
        let t  = &self.config.theme;
        let bg = parse_hex(&t.background).unwrap_or(Color::BLACK);
        let fg = parse_hex(&t.foreground).unwrap_or(Color::WHITE);
        let ac = parse_hex(&t.accent).unwrap_or(Color::from_rgb8(0xcb, 0xa6, 0xf7));

        let pill = |label: &'static str, col: Color| -> Element<'_, Message> {
            container(text(label).color(col).size(12.0))
                .padding([3, 8])
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(Color {
                        a: 0.15,
                        ..col
                    })),
                    border: iced::Border { radius: 10.0.into(), ..Default::default() },
                    ..Default::default()
                })
                .into()
        };

        let inner = row![
            pill("workspaces", ac),
            text("  Window Title").color(fg).size(13.0),
            iced::widget::Space::new().width(Length::Fill),
            text("12:34  Sat 01 Mar").color(fg).size(13.0),
            iced::widget::Space::new().width(Length::Fill),
            text("↓ 1.2k  CPU 4%  RAM 6G").color(fg).size(13.0),
        ]
        .align_y(Alignment::Center)
        .spacing(8)
        .padding([0, 12]);

        container(inner)
            .width(Length::Fill)
            .height(Length::Fixed(38.0))
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::Border { radius: 6.0.into(), ..Default::default() },
                ..Default::default()
            })
            .into()
    }

    // ── Theme section ─────────────────────────────────────────────────────────

    fn view_theme(&self) -> Element<'_, Message> {
        let t = &self.config.theme;

        let nerd_active = t.icon_style.to_lowercase() != "ascii";
        let icon_btn = |label: &'static str, use_nerd: bool| -> Element<'_, Message> {
            let active = nerd_active == use_nerd;
            let btn = button(text(label).size(13.0)).on_press(Message::UseNerdIcons(use_nerd));
            if active { btn.style(iced::widget::button::primary).into() } else { btn.into() }
        };

        let ws_dots   = t.workspace_style.to_lowercase() == "dots";
        let ws_all    = t.workspace_show_all;
        let ws_style_btn = |label: &'static str, dots: bool| -> Element<'_, Message> {
            let active = ws_dots == dots;
            let btn = button(text(label).size(13.0)).on_press(Message::WorkspaceStyle(dots));
            if active { btn.style(iced::widget::button::primary).into() } else { btn.into() }
        };
        let ws_show_btn = |label: &'static str, all: bool| -> Element<'_, Message> {
            let active = ws_all == all;
            let btn = button(text(label).size(13.0)).on_press(Message::WorkspaceShowAll(all));
            if active { btn.style(iced::widget::button::primary).into() } else { btn.into() }
        };

        let net_tokens: Vec<&str> = t.network_show.split(',').map(str::trim).collect();
        let net_speed  = net_tokens.contains(&"speed");
        let net_name   = net_tokens.contains(&"name");
        let net_signal = net_tokens.contains(&"signal");
        let net_btn = |label: &'static str, active: bool, msg: Message| -> Element<'_, Message> {
            let btn = button(text(label).size(13.0)).on_press(msg);
            if active { btn.style(iced::widget::button::primary).into() } else { btn.into() }
        };

        let ps = self.picker_sat;
        let pa = self.picker_alpha;
        let picker_for = |field: ColorField| -> Option<(f32, f32)> {
            if self.active_picker == Some(field) { Some((ps, pa)) } else { None }
        };

        // Build theme preset buttons
        let preset_btns: Vec<Element<'_, Message>> = THEME_PRESETS
            .iter()
            .enumerate()
            .map(|(i, p)| {
                button(text(p.name).size(12.0))
                    .on_press(Message::ApplyThemePreset(i))
                    .into()
            })
            .collect();

        column![
            // ── Widget Behaviour ──────────────────────────────────────────────
            section_header("Widget Behaviour"),
            labeled_row(
                "Workspace Style",
                row![
                    ws_style_btn("Numbers", false),
                    ws_style_btn("Dots", true),
                ]
                .spacing(4),
            ),
            labeled_row(
                "Workspace Visible",
                row![
                    ws_show_btn("All", true),
                    ws_show_btn("Active Only", false),
                ]
                .spacing(4),
            ),
            labeled_row(
                "Network Display",
                row![
                    net_btn("Speed",  net_speed,  Message::NetworkShowSpeed(!net_speed)),
                    net_btn("Name",   net_name,   Message::NetworkShowName(!net_name)),
                    net_btn("Signal", net_signal, Message::NetworkShowSignal(!net_signal)),
                    text("select any combination").size(11.0)
                        .color(Color::from_rgb8(0x6c, 0x70, 0x86)),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Icon Style",
                row![
                    icon_btn("Nerd Font", true),
                    icon_btn("ASCII", false),
                    text("use ASCII if icons show as \"?\"").size(11.0)
                        .color(Color::from_rgb8(0x6c, 0x70, 0x86)),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Clock Format",
                row![
                    text_input("%H:%M", &self.clock_format_buf)
                        .on_input(Message::ClockFormatChanged)
                        .width(150),
                    text("strftime format").size(11.0).color(Color::from_rgb8(0x6c, 0x70, 0x86)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Date Format",
                row![
                    text_input("%a %d %b", &self.date_format_buf)
                        .on_input(Message::DateFormatChanged)
                        .width(150),
                    text("strftime format").size(11.0).color(Color::from_rgb8(0x6c, 0x70, 0x86)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            // ── Colors ────────────────────────────────────────────────────────
            section_header("Colors"),
            color_input_optional("Background",        &self.bg_buf,           &t.background,   Message::BgChanged,
                ColorField::Background, picker_for(ColorField::Background)),
            color_input("Text Color",         &self.fg_buf,           &t.foreground,   Message::FgChanged,
                ColorField::Foreground, picker_for(ColorField::Foreground)),
            color_input("Accent",             &self.accent_buf,       &t.accent,       Message::AccentChanged,
                ColorField::Accent, picker_for(ColorField::Accent)),
            color_input_optional("Widget Background", &self.widget_bg_buf,    &t.widget_bg,    Message::WidgetBgChanged,
                ColorField::WidgetBg, picker_for(ColorField::WidgetBg)),
            color_input_optional("Border Color",      &self.border_color_buf, &t.border_color, Message::BorderColorChanged,
                ColorField::BorderColor, picker_for(ColorField::BorderColor)),
            color_input_optional("Widget Border",     &self.widget_border_color_buf, &t.widget_border_color, Message::WidgetBorderColorChanged,
                ColorField::WidgetBorderColor, picker_for(ColorField::WidgetBorderColor)),
            // ── Shape & Spacing ───────────────────────────────────────────────
            section_header("Shape & Spacing"),
            labeled_row(
                "Border Width",
                row![
                    slider(0.0f32..=8.0, t.border_width as f32, Message::BorderWidthChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", t.border_width)).width(60),
                    text("bar outer border").size(11.0).color(Color::from_rgb8(0x6c, 0x70, 0x86)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Widget Border Width",
                row![
                    slider(0.0f32..=8.0, t.widget_border_width as f32, Message::WidgetBorderWidthChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", t.widget_border_width)).width(60),
                    text("per-widget pill border").size(11.0).color(Color::from_rgb8(0x6c, 0x70, 0x86)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Border Radius",
                row![
                    slider(0.0f32..=40.0, t.border_radius, Message::RadiusChanged)
                        .step(0.5f32)
                        .width(200),
                    text(format!("{:.1} px", t.border_radius)).width(60),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Padding",
                row![
                    slider(0.0f32..=32.0, t.padding as f32, Message::PaddingChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", t.padding)).width(60),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Gap",
                row![
                    slider(0.0f32..=24.0, t.gap as f32, Message::GapChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", t.gap)).width(60),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Widget Pad X",
                row![
                    slider(0.0f32..=32.0, t.widget_padding_x as f32, Message::WidgetPadXChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", t.widget_padding_x)).width(60),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Widget Pad Y",
                row![
                    slider(0.0f32..=20.0, t.widget_padding_y as f32, Message::WidgetPadYChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", t.widget_padding_y)).width(60),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            // ── Font ──────────────────────────────────────────────────────────
            section_header("Font"),
            labeled_row(
                "Font Family",
                text_input("Font name", &self.font_buf)
                    .on_input(Message::FontChanged)
                    .width(200),
            ),
            labeled_row(
                "Font Size",
                row![
                    slider(8.0f32..=32.0, t.font_size, Message::FontSizeChanged)
                        .step(0.5f32)
                        .width(200),
                    text(format!("{:.1} pt", t.font_size)).width(60),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            // ── Theme Presets ─────────────────────────────────────────────────
            section_header("Theme Presets"),
            labeled_row(
                "Presets",
                column![
                    iced::widget::Row::from_vec(preset_btns).spacing(4).wrap(),
                    row![
                        button(text("⬇ Import pywal").size(12.0))
                            .on_press(Message::ImportWal),
                        text("Imports ~/.cache/wal/colors.json").size(11.0)
                            .color(Color::from_rgb8(0x6c, 0x70, 0x86)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ]
                .spacing(6),
            ),
        ]
        .spacing(20)
        .into()
    }
}

// ── Widget helpers ────────────────────────────────────────────────────────────

fn labeled_row<'a>(
    label: &'a str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![
        text(label).width(160),
        content.into(),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn section_header(title: &'static str) -> Element<'static, Message> {
    column![
        rule::horizontal(1.0f32),
        text(title).size(11.0).color(Color::from_rgb8(0x6c, 0x70, 0x86)),
    ]
    .spacing(4)
    .into()
}

fn tab_btn(label: &str, target: Section, current: Section) -> Element<'_, Message> {
    let btn = button(text(label).size(14.0)).on_press(Message::Tab(target));
    if target == current {
        btn.style(iced::widget::button::primary).into()
    } else {
        btn.into()
    }
}

fn pos_btn(label: &str, target: Position, current: Position) -> Element<'_, Message> {
    let active = target == current;
    button(text(if active {
        format!("[{label}]")
    } else {
        label.to_string()
    }))
    .on_press(Message::PositionChanged(target))
    .into()
}

fn color_input<'a>(
    label: &'a str,
    buf: &'a str,
    config_val: &'a str,
    on_change: fn(String) -> Message,
    field: ColorField,
    picker_state: Option<(f32, f32)>,  // Some((sat_scale, alpha)) when open
) -> Element<'a, Message> {
    let swatch_color = parse_hex(config_val).unwrap_or(Color::BLACK);

    let swatch = container(text(""))
        .width(Length::Fixed(24.0))
        .height(Length::Fixed(24.0))
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(swatch_color)),
            border: iced::Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        });

    let valid = is_valid_hex(buf);
    let input = text_input("#rrggbb", buf).on_input(on_change).width(110);

    let pick_icon = if picker_state.is_some() { "▲" } else { "▼" };
    let pick_btn = button(text(pick_icon).size(11.0))
        .on_press(Message::TogglePicker(field));

    let main_row = labeled_row(
        label,
        row![swatch, input, text(if valid { "" } else { "invalid" }), pick_btn]
            .spacing(8)
            .align_y(Alignment::Center),
    );

    if let Some((sat, alpha)) = picker_state {
        let picker_content = column![
            color_grid(),
            row![
                text("S").width(20).size(12.0),
                slider(0.0f32..=1.0, sat, Message::PickerSat).step(0.01).width(180),
                text(format!("{:.0}%", sat * 100.0)).width(40).size(12.0),
            ].spacing(4).align_y(Alignment::Center),
            row![
                text("A").width(20).size(12.0),
                slider(0.0f32..=1.0, alpha, Message::PickerAlpha).step(0.01).width(180),
                text(format!("{:.0}%", alpha * 100.0)).width(40).size(12.0),
            ].spacing(4).align_y(Alignment::Center),
            container(text(""))
                .width(Length::Fixed(244.0))
                .height(Length::Fixed(14.0))
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(swatch_color)),
                    border: iced::Border { radius: 3.0.into(), ..Default::default() },
                    ..Default::default()
                }),
        ].spacing(6);
        let picker_row = labeled_row("", picker_content);
        let mut col = iced::widget::Column::new().spacing(4);
        col = col.push(main_row);
        col = col.push(picker_row);
        col.into()
    } else {
        main_row
    }
}

/// Like `color_input` but allows an empty string (meaning "disabled / none").
fn color_input_optional<'a>(
    label: &'a str,
    buf: &'a str,
    config_val: &'a str,
    on_change: fn(String) -> Message,
    field: ColorField,
    picker_state: Option<(f32, f32)>,  // Some((sat_scale, alpha)) when open
) -> Element<'a, Message> {
    let swatch_color = parse_hex(config_val).unwrap_or(Color::from_rgba8(0, 0, 0, 0.0));

    let swatch = container(text(""))
        .width(Length::Fixed(24.0))
        .height(Length::Fixed(24.0))
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(swatch_color)),
            border: iced::Border {
                radius: 4.0.into(),
                color: Color::from_rgb8(0x45, 0x47, 0x5a),
                width: 1.0,
            },
            ..Default::default()
        });

    let hint = if buf.is_empty() { "none" } else if is_valid_hex(buf) { "" } else { "invalid" };
    let input = text_input("#rrggbb or empty", buf).on_input(on_change).width(110);

    let pick_icon = if picker_state.is_some() { "▲" } else { "▼" };
    let pick_btn = button(text(pick_icon).size(11.0))
        .on_press(Message::TogglePicker(field));

    let main_row = labeled_row(
        label,
        row![swatch, input, text(hint), pick_btn]
            .spacing(8)
            .align_y(Alignment::Center),
    );

    if let Some((sat, alpha)) = picker_state {
        let picker_content = column![
            color_grid(),
            row![
                text("S").width(20).size(12.0),
                slider(0.0f32..=1.0, sat, Message::PickerSat).step(0.01).width(180),
                text(format!("{:.0}%", sat * 100.0)).width(40).size(12.0),
            ].spacing(4).align_y(Alignment::Center),
            row![
                text("A").width(20).size(12.0),
                slider(0.0f32..=1.0, alpha, Message::PickerAlpha).step(0.01).width(180),
                text(format!("{:.0}%", alpha * 100.0)).width(40).size(12.0),
            ].spacing(4).align_y(Alignment::Center),
            container(text(""))
                .width(Length::Fixed(244.0))
                .height(Length::Fixed(14.0))
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(swatch_color)),
                    border: iced::Border { radius: 3.0.into(), ..Default::default() },
                    ..Default::default()
                }),
        ].spacing(6);
        let picker_row = labeled_row("", picker_content);
        let mut col = iced::widget::Column::new().spacing(4);
        col = col.push(main_row);
        col = col.push(picker_row);
        col.into()
    } else {
        main_row
    }
}

// ── Colour grid ───────────────────────────────────────────────────────────────

/// 2-D HSV colour grid: 24 hue columns × 8 rows (7 colour rows + 1 grey row).
/// Clicking a cell emits `Message::ColorGridPicked(hex)`.
fn color_grid<'a>() -> Element<'a, Message> {
    const HUES: usize = 24;
    const HUE_STEP: f32 = 360.0 / HUES as f32;
    const CELL: f32 = 14.0;
    const GAP:  f32 = 2.0;

    // (saturation, value) for each colour row
    const SV_ROWS: &[(f32, f32)] = &[
        (1.00, 1.00), // vivid, bright
        (0.80, 0.95), // slightly softer
        (1.00, 0.75), // darker vivid
        (1.00, 0.55), // darker
        (1.00, 0.35), // very dark
        (0.40, 0.95), // pastel
        (0.20, 0.70), // muted
    ];

    let make_cell = |h: f32, s: f32, v: f32| -> Element<'a, Message> {
        let (r, g, b) = hsv_to_rgb(h, s, v);
        let color = Color::from_rgb8(r, g, b);
        mouse_area(
            container(text(""))
                .width(Length::Fixed(CELL))
                .height(Length::Fixed(CELL))
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(color)),
                    border: iced::Border { radius: 2.0.into(), ..Default::default() },
                    ..Default::default()
                }),
        )
        .on_press(Message::ColorGridPicked(h, s, v))
        .into()
    };

    let mut rows: Vec<Element<'a, Message>> = Vec::new();

    // Colour rows
    for &(s, v) in SV_ROWS {
        let cells: Vec<Element<'a, Message>> = (0..HUES)
            .map(|i| make_cell(i as f32 * HUE_STEP, s, v))
            .collect();
        rows.push(
            iced::widget::Row::from_vec(cells).spacing(GAP).into()
        );
    }

    // Grey row (white → black)
    let grey_cells: Vec<Element<'a, Message>> = (0..HUES)
        .map(|i| {
            let v = 1.0 - (i as f32 / (HUES - 1) as f32) * 0.95;
            make_cell(0.0, 0.0, v)
        })
        .collect();
    rows.push(
        iced::widget::Row::from_vec(grey_cells).spacing(GAP).into()
    );

    iced::widget::Column::from_vec(rows).spacing(GAP).into()
}

// ── Pure helpers ──────────────────────────────────────────────────────────────

fn parse_hex(s: &str) -> Option<Color> {
    let s = s.trim_start_matches('#');
    let byte = |chunk: &str| u8::from_str_radix(chunk, 16).ok();
    match s.len() {
        6 => Some(Color::from_rgb8(byte(&s[0..2])?, byte(&s[2..4])?, byte(&s[4..6])?)),
        8 => Some(Color::from_rgba8(
            byte(&s[0..2])?,
            byte(&s[2..4])?,
            byte(&s[4..6])?,
            byte(&s[6..8])? as f32 / 255.0,
        )),
        _ => None,
    }
}

fn is_valid_hex(s: &str) -> bool {
    parse_hex(s).is_some()
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    if s < 1e-6 {
        let c = (v * 255.0).round() as u8;
        return (c, c, c);
    }
    let h6 = (h / 60.0).rem_euclid(6.0);
    let i  = h6 as i32;
    let f  = h6 - i as f32;
    let p  = v * (1.0 - s);
    let q  = v * (1.0 - f * s);
    let t  = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    ((r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8)
}


/// Read `~/.cache/wal/colors.json` and return `(background, foreground, accent)` hex strings.
fn load_wal_colors() -> Option<(String, String, String)> {
    let home  = std::env::var("HOME").ok()?;
    let path  = std::path::Path::new(&home).join(".cache/wal/colors.json");
    let text  = std::fs::read_to_string(path).ok()?;
    // Minimal parse — just extract the values we care about without pulling in serde_json.
    // Expected keys: "special": { "background": "#...", "foreground": "#..." }
    //                "colors":  { "color1": "#..." }
    let bg = extract_json_string(&text, "background")?;
    let fg = extract_json_string(&text, "foreground")?;
    // Use color1 as accent (first non-background colour in a pywal palette is usually the accent).
    let ac = extract_json_string(&text, "color1")?;
    Some((bg, fg, ac))
}

/// Naive key lookup in a JSON string — finds the first `"key": "#value"` pair.
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start  = json.find(&needle)? + needle.len();
    let after  = json[start..].trim_start();
    let after  = after.strip_prefix(':')?.trim_start();
    let after  = after.strip_prefix('"')?;
    let end    = after.find('"')?;
    Some(after[..end].to_string())
}

/// Add or remove a token (e.g. "speed", "name", "signal") from the comma-separated
/// `network_show` string without affecting the other tokens.
fn toggle_network_show(field: &mut String, token: &str, enable: bool) {
    let mut parts: Vec<&str> = field.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
    parts.retain(|t| *t != token);
    if enable {
        parts.push(token);
    }
    *field = parts.join(",");
}

fn save_config(config: &BarConfig, path: &std::path::Path) -> Result<(), String> {
    let toml_str = toml::to_string_pretty(config)
        .map_err(|e| format!("Serialize error: {e}"))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create directory: {e}"))?;
    }

    std::fs::write(path, toml_str)
        .map_err(|e| format!("Cannot write file: {e}"))?;

    Ok(())
}

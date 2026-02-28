use bar_config::{default_path, load as load_config, BarConfig, Position, WidgetConfig};
use iced::{
    widget::{button, checkbox, column, container, pick_list, row, rule, scrollable, slider, text, text_input},
    Alignment, Color, Element, Length, Size, Subscription, Task,
};
use std::path::PathBuf;

const ALL_WIDGETS: &[&str] = &[
    "workspaces", "title", "clock",
    "cpu", "memory", "network", "battery",
    "disk", "temperature", "volume", "brightness",
    "swap", "uptime", "load", "keyboard", "media", "custom",
];

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    iced::application(Editor::new, Editor::update, Editor::view)
        .title("Bar Editor")
        .subscription(Editor::subscription)
        .window_size(Size::new(740.0, 640.0))
        .run()
}

// ── Shape presets ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapePreset {
    Pill,
    Rounded,
    Sharp,
}

// ── Color field identifiers (for the inline HSL picker) ───────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorField {
    Background,
    Foreground,
    Accent,
    WidgetBg,
    BorderColor,
}

// ── Theme presets ─────────────────────────────────────────────────────────────

struct ThemePreset {
    name:       &'static str,
    background: &'static str,
    foreground: &'static str,
    accent:     &'static str,
}

const THEME_PRESETS: &[ThemePreset] = &[
    ThemePreset { name: "Catppuccin Mocha", background: "#1e1e2e", foreground: "#cdd6f4", accent: "#cba6f7" },
    ThemePreset { name: "Catppuccin Latte", background: "#eff1f5", foreground: "#4c4f69", accent: "#8839ef" },
    ThemePreset { name: "Gruvbox Dark",     background: "#282828", foreground: "#ebdbb2", accent: "#fabd2f" },
    ThemePreset { name: "Tokyo Night",      background: "#1a1b26", foreground: "#c0caf5", accent: "#7aa2f7" },
    ThemePreset { name: "Nord",             background: "#2e3440", foreground: "#eceff4", accent: "#88c0d0" },
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
    border_color_buf:    String,
    clock_format_buf:    String,
    date_format_buf:     String,
    // Inline HSL color picker state
    active_picker:       Option<ColorField>,
    picker_h:            f32,   // hue 0 – 360
    picker_s:            f32,   // saturation 0 – 1
    picker_l:            f32,   // lightness 0 – 1
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
    ShapePreset(ShapePreset),
    WidgetBgChanged(String),
    BorderColorChanged(String),
    BorderWidthChanged(f32),
    ClockFormatChanged(String),
    DateFormatChanged(String),
    UseNerdIcons(bool),
    WidgetPadXChanged(f32),
    WidgetPadYChanged(f32),
    // Color picker
    TogglePicker(ColorField),
    PickerHue(f32),
    PickerSat(f32),
    PickerLit(f32),
    ApplyThemePreset(usize),
    ResetDefaults,

    // Actions
    Save,
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
        let border_color_buf    = config.theme.border_color.clone();
        let clock_format_buf    = config.theme.clock_format.clone();
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
                clock_format_buf,
                date_format_buf,
                active_picker: None,
                picker_h:      0.0,
                picker_s:      0.5,
                picker_l:      0.5,
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
        self.border_color_buf = self.config.theme.border_color.clone();
        self.clock_format_buf = self.config.theme.clock_format.clone();
        self.date_format_buf  = self.config.theme.date_format.clone();
        self.active_picker    = None; // close picker when presets/reset are applied
    }

    fn apply_picker_color(&mut self) {
        let hex = hsl_to_hex(self.picker_h, self.picker_s, self.picker_l);
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
            None => {}
        }
    }
}

// ── Subscription ─────────────────────────────────────────────────────────────

impl Editor {
    fn subscription(&self) -> Subscription<Message> {
        iced::keyboard::listen().map(Message::KeyEvent)
    }
}

// ── Update ────────────────────────────────────────────────────────────────────

impl Editor {
    fn update(&mut self, msg: Message) -> Task<Message> {
        if !matches!(msg, Message::Save | Message::Tab(_) | Message::TogglePicker(_)) {
            self.save_status = SaveStatus::Idle;
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
                if is_valid_hex(&s) { self.config.theme.background = s; }
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

            Message::ShapePreset(p) => {
                self.config.theme.border_radius = match p {
                    ShapePreset::Pill    => self.config.global.height as f32 / 2.0,
                    ShapePreset::Rounded => 8.0,
                    ShapePreset::Sharp   => 0.0,
                };
            }
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

            // ── Color picker ─────────────────────────────────────────────────
            Message::TogglePicker(field) => {
                if self.active_picker == Some(field) {
                    self.active_picker = None;
                } else {
                    let hex = match field {
                        ColorField::Background  => self.config.theme.background.clone(),
                        ColorField::Foreground  => self.config.theme.foreground.clone(),
                        ColorField::Accent      => self.config.theme.accent.clone(),
                        ColorField::WidgetBg    => self.config.theme.widget_bg.clone(),
                        ColorField::BorderColor => self.config.theme.border_color.clone(),
                    };
                    if let Some((h, s, l)) = hex_to_hsl(&hex) {
                        self.picker_h = h;
                        self.picker_s = s;
                        self.picker_l = l;
                    } else {
                        self.picker_h = 220.0;
                        self.picker_s = 0.7;
                        self.picker_l = 0.5;
                    }
                    self.active_picker = Some(field);
                }
            }
            Message::PickerHue(v) => { self.picker_h = v; self.apply_picker_color(); }
            Message::PickerSat(v) => { self.picker_s = v; self.apply_picker_color(); }
            Message::PickerLit(v) => { self.picker_l = v; self.apply_picker_color(); }

            Message::ApplyThemePreset(idx) => {
                if let Some(p) = THEME_PRESETS.get(idx) {
                    self.config.theme.background = p.background.to_string();
                    self.config.theme.foreground = p.foreground.to_string();
                    self.config.theme.accent     = p.accent.to_string();
                    self.sync_bufs();
                }
            }

            Message::ResetDefaults => {
                let defaults = BarConfig::default();
                self.config = defaults;
                self.sync_bufs();
                self.save_status = SaveStatus::Idle;
            }

            // ── Save ─────────────────────────────────────────────────────────
            Message::Save => {
                self.do_save();
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

        let status: Element<'_, Message> = match &self.save_status {
            SaveStatus::Idle        => text("").into(),
            SaveStatus::Saved       => text("✓ Saved — bar will reload automatically")
                .color(Color::from_rgb8(0xa6, 0xe3, 0xa1))
                .into(),
            SaveStatus::Restarting  => text("✓ Saved — restarting bar…")
                .color(Color::from_rgb8(0x89, 0xb4, 0xfa))
                .into(),
            SaveStatus::Error(e)    => text(format!("✗ {e}"))
                .color(Color::from_rgb8(0xf3, 0x8b, 0xa8))
                .into(),
        };

        let footer = row![
            button(text("Save Changes")).on_press(Message::Save),
            button(text("Reset Defaults"))
                .on_press(Message::ResetDefaults)
                .style(iced::widget::button::danger),
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
            labeled_row(
                "Height",
                row![
                    slider(20.0f32..=100.0, g.height as f32, Message::HeightChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", g.height)).width(60),
                    text("⟲ restart").size(11.0).color(Color::from_rgb8(0xf9, 0xe2, 0xaf)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            labeled_row(
                "Position",
                row![
                    pos_btn("Top",    Position::Top,    g.position),
                    pos_btn("Bottom", Position::Bottom, g.position),
                    text("⟲ restart").size(11.0).color(Color::from_rgb8(0xf9, 0xe2, 0xaf)),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            ),
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
            labeled_row(
                "H. Margin",
                row![
                    slider(0.0f32..=100.0, g.margin as f32, Message::MarginChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", g.margin)).width(60),
                    text("⟲ restart").size(11.0).color(Color::from_rgb8(0xf9, 0xe2, 0xaf)),
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
                    text("⟲ restart").size(11.0).color(Color::from_rgb8(0xf9, 0xe2, 0xaf)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
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

        // Detect current shape preset from border_radius value
        let half_height = self.config.global.height as f32 / 2.0;
        let current_shape = if (t.border_radius - half_height).abs() < 0.5 {
            Some(ShapePreset::Pill)
        } else if (t.border_radius - 8.0).abs() < 0.5 {
            Some(ShapePreset::Rounded)
        } else if t.border_radius == 0.0 {
            Some(ShapePreset::Sharp)
        } else {
            None
        };

        let shape_btn = |label: &'static str, preset: ShapePreset| -> Element<'_, Message> {
            let active = current_shape == Some(preset);
            let btn = button(text(label).size(13.0)).on_press(Message::ShapePreset(preset));
            if active { btn.style(iced::widget::button::primary).into() } else { btn.into() }
        };

        let nerd_active = t.icon_style.to_lowercase() != "ascii";
        let icon_btn = |label: &'static str, use_nerd: bool| -> Element<'_, Message> {
            let active = nerd_active == use_nerd;
            let btn = button(text(label).size(13.0)).on_press(Message::UseNerdIcons(use_nerd));
            if active { btn.style(iced::widget::button::primary).into() } else { btn.into() }
        };

        let ph = self.picker_h;
        let ps = self.picker_s;
        let pl = self.picker_l;
        let picker_for = |field: ColorField| -> Option<(f32, f32, f32)> {
            if self.active_picker == Some(field) { Some((ph, ps, pl)) } else { None }
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
            // ── Theme presets ─────────────────────────────────────────────────
            labeled_row(
                "Presets",
                iced::widget::Row::from_vec(preset_btns).spacing(4).wrap(),
            ),
            // ── Shape presets ─────────────────────────────────────────────────
            labeled_row(
                "Widget Shape",
                row![
                    shape_btn("Pill",    ShapePreset::Pill),
                    shape_btn("Rounded", ShapePreset::Rounded),
                    shape_btn("Sharp",   ShapePreset::Sharp),
                ]
                .spacing(4),
            ),
            // ── Icon style ───────────────────────────────────────────────────
            labeled_row(
                "Icons",
                row![
                    icon_btn("Nerd Font", true),
                    icon_btn("ASCII", false),
                    text("Use ASCII if icons show as \"?\"").size(11.0)
                        .color(Color::from_rgb8(0x6c, 0x70, 0x86)),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            ),
            // ── Widget pill padding ──────────────────────────────────────────
            labeled_row(
                "Pill Pad X",
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
                "Pill Pad Y",
                row![
                    slider(0.0f32..=16.0, t.widget_padding_y as f32, Message::WidgetPadYChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", t.widget_padding_y)).width(60),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            // ── Colors ────────────────────────────────────────────────────────
            color_input("Background",  &self.bg_buf,        &t.background,   Message::BgChanged,
                ColorField::Background, picker_for(ColorField::Background)),
            color_input("Foreground",  &self.fg_buf,        &t.foreground,   Message::FgChanged,
                ColorField::Foreground, picker_for(ColorField::Foreground)),
            color_input("Accent",      &self.accent_buf,    &t.accent,       Message::AccentChanged,
                ColorField::Accent, picker_for(ColorField::Accent)),
            color_input_optional("Widget BG",    &self.widget_bg_buf,    &t.widget_bg,    Message::WidgetBgChanged,
                ColorField::WidgetBg, picker_for(ColorField::WidgetBg)),
            color_input_optional("Border Color", &self.border_color_buf, &t.border_color, Message::BorderColorChanged,
                ColorField::BorderColor, picker_for(ColorField::BorderColor)),
            // ── Border width ─────────────────────────────────────────────────
            labeled_row(
                "Border Width",
                row![
                    slider(0.0f32..=8.0, t.border_width as f32, Message::BorderWidthChanged)
                        .step(1.0f32)
                        .width(200),
                    text(format!("{} px", t.border_width)).width(60),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
            // ── Font ──────────────────────────────────────────────────────────
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
            // ── Spacing ──────────────────────────────────────────────────────
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
            // ── Clock formats ─────────────────────────────────────────────────
            labeled_row(
                "Clock Format",
                row![
                    text_input("%H:%M", &self.clock_format_buf)
                        .on_input(Message::ClockFormatChanged)
                        .width(150),
                    text("strftime").size(11.0).color(Color::from_rgb8(0x6c, 0x70, 0x86)),
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
                    text("strftime").size(11.0).color(Color::from_rgb8(0x6c, 0x70, 0x86)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
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
        text(label).width(140),
        content.into(),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
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
    picker: Option<(f32, f32, f32)>,
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

    let pick_icon = if picker.is_some() { "▲" } else { "▼" };
    let pick_btn = button(text(pick_icon).size(11.0))
        .on_press(Message::TogglePicker(field));

    let main_row = labeled_row(
        label,
        row![swatch, input, text(if valid { "" } else { "invalid" }), pick_btn]
            .spacing(8)
            .align_y(Alignment::Center),
    );

    if let Some((h, s, l)) = picker {
        let preview_color = swatch_color;
        let picker_content = column![
            row![
                text("H").width(14).size(12.0),
                slider(0.0f32..=360.0, h, Message::PickerHue).step(1.0).width(185),
                text(format!("{h:.0}°")).width(38).size(12.0),
            ].spacing(4).align_y(Alignment::Center),
            row![
                text("S").width(14).size(12.0),
                slider(0.0f32..=1.0, s, Message::PickerSat).step(0.01).width(185),
                text(format!("{:.0}%", s * 100.0)).width(38).size(12.0),
            ].spacing(4).align_y(Alignment::Center),
            row![
                text("L").width(14).size(12.0),
                slider(0.0f32..=1.0, l, Message::PickerLit).step(0.01).width(185),
                text(format!("{:.0}%", l * 100.0)).width(38).size(12.0),
            ].spacing(4).align_y(Alignment::Center),
            container(text(""))
                .width(Length::Fixed(240.0))
                .height(Length::Fixed(18.0))
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(preview_color)),
                    border: iced::Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }),
        ].spacing(4);
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
    picker: Option<(f32, f32, f32)>,
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

    let pick_icon = if picker.is_some() { "▲" } else { "▼" };
    let pick_btn = button(text(pick_icon).size(11.0))
        .on_press(Message::TogglePicker(field));

    let main_row = labeled_row(
        label,
        row![swatch, input, text(hint), pick_btn]
            .spacing(8)
            .align_y(Alignment::Center),
    );

    if let Some((h, s, l)) = picker {
        let preview_color = swatch_color;
        let picker_content = column![
            row![
                text("H").width(14).size(12.0),
                slider(0.0f32..=360.0, h, Message::PickerHue).step(1.0).width(185),
                text(format!("{h:.0}°")).width(38).size(12.0),
            ].spacing(4).align_y(Alignment::Center),
            row![
                text("S").width(14).size(12.0),
                slider(0.0f32..=1.0, s, Message::PickerSat).step(0.01).width(185),
                text(format!("{:.0}%", s * 100.0)).width(38).size(12.0),
            ].spacing(4).align_y(Alignment::Center),
            row![
                text("L").width(14).size(12.0),
                slider(0.0f32..=1.0, l, Message::PickerLit).step(0.01).width(185),
                text(format!("{:.0}%", l * 100.0)).width(38).size(12.0),
            ].spacing(4).align_y(Alignment::Center),
            container(text(""))
                .width(Length::Fixed(240.0))
                .height(Length::Fixed(18.0))
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(preview_color)),
                    border: iced::Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }),
        ].spacing(4);
        let picker_row = labeled_row("", picker_content);
        let mut col = iced::widget::Column::new().spacing(4);
        col = col.push(main_row);
        col = col.push(picker_row);
        col.into()
    } else {
        main_row
    }
}

// ── Pure helpers ──────────────────────────────────────────────────────────────

fn parse_hex(s: &str) -> Option<Color> {
    let s = s.trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Color::from_rgb8(r, g, b))
    } else {
        None
    }
}

fn is_valid_hex(s: &str) -> bool {
    parse_hex(s).is_some()
}

/// Convert a `#rrggbb` hex string to HSL (H: 0–360, S: 0–1, L: 0–1).
/// Returns `None` when the hex is invalid or empty.
fn hex_to_hsl(hex: &str) -> Option<(f32, f32, f32)> {
    let c = parse_hex(hex)?;
    let (r, g, b) = (c.r, c.g, c.b);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < 1e-6 {
        return Some((0.0, 0.0, l));
    }

    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };

    let h = if (max - r).abs() < 1e-6 {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if (max - g).abs() < 1e-6 {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };

    Some((h * 360.0, s, l))
}

/// Convert HSL (H: 0–360, S: 0–1, L: 0–1) to a `#rrggbb` hex string.
fn hsl_to_hex(h: f32, s: f32, l: f32) -> String {
    if s < 1e-6 {
        let v = (l * 255.0).round() as u8;
        return format!("#{v:02x}{v:02x}{v:02x}");
    }

    fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
        if t < 0.0 { t += 1.0; }
        if t > 1.0 { t -= 1.0; }
        if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
        if t < 0.5 { return q; }
        if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
        p
    }

    let hn = h / 360.0;
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;

    let r = (hue_to_rgb(p, q, hn + 1.0 / 3.0) * 255.0).round() as u8;
    let g = (hue_to_rgb(p, q, hn) * 255.0).round() as u8;
    let b = (hue_to_rgb(p, q, hn - 1.0 / 3.0) * 255.0).round() as u8;

    format!("#{r:02x}{g:02x}{b:02x}")
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

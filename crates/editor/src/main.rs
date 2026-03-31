//! `bar-editor` — bento grid layout editor with live preview.
//!
//! Reads `~/.config/bar/bar.toml`, lets you reorder cards and edit their
//! col/row spans, previews the grid in real-time, and saves back to disk.

use bar_config::{
    default_path, load as load_config,
    schema::{CardConfig, DashConfig},
};
use iced::{
    widget::{
        button, column, container, pick_list, row, scrollable, text,
        rule,
    },
    Alignment, Background, Border, Color, Element, Length, Padding, Size,
    Task,
};

// ── Constants ─────────────────────────────────────────────────────────────────

const ALL_CARD_KINDS: &[&str] = &[
    "clock", "network", "battery", "cpu", "memory", "disk", "volume",
    "brightness", "media", "power", "uptime", "temperature", "updates",
    "swap", "load", "gpu", "bluetooth", "weather",
];

// ── Color helpers ─────────────────────────────────────────────────────────────

fn hex_to_color(s: &str) -> Color {
    let s = s.trim_start_matches('#');
    if s.len() < 6 {
        return Color::BLACK;
    }
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0) as f32 / 255.0;
    Color::from_rgb(r, g, b)
}

/// Slightly lighten a color component-wise.
fn lighten(c: Color, amt: f32) -> Color {
    Color {
        r: (c.r + amt).min(1.0),
        g: (c.g + amt).min(1.0),
        b: (c.b + amt).min(1.0),
        a: c.a,
    }
}

// ── Card accent colors (Catppuccin Mocha palette) ─────────────────────────────

fn card_accent_color(kind: &str) -> Color {
    match kind {
        "cpu" | "temperature" => Color::from_rgb(0.96, 0.54, 0.67),
        "memory" | "swap"     => Color::from_rgb(0.79, 0.65, 0.97),
        "network"             => Color::from_rgb(0.54, 0.71, 0.98),
        "disk"                => Color::from_rgb(0.98, 0.89, 0.68),
        "battery"             => Color::from_rgb(0.67, 0.88, 0.63),
        "volume"              => Color::from_rgb(0.58, 0.89, 0.84),
        "brightness"          => Color::from_rgb(0.98, 0.89, 0.55),
        "gpu"                 => Color::from_rgb(0.54, 0.87, 0.75),
        "load"                => Color::from_rgb(0.98, 0.81, 0.68),
        "bluetooth"           => Color::from_rgb(0.49, 0.72, 0.97),
        "weather"             => Color::from_rgb(0.53, 0.82, 0.96),
        "media"               => Color::from_rgb(0.96, 0.54, 0.84),
        "uptime"              => Color::from_rgb(0.58, 0.89, 0.84),
        "updates"             => Color::from_rgb(0.98, 0.70, 0.53),
        "power"               => Color::from_rgb(0.96, 0.54, 0.67),
        _                     => Color::from_rgb(0.79, 0.73, 0.62), // mauve/fallback
    }
}

// ── Card default span (mirrors dashboard logic) ───────────────────────────────

fn default_col_span(kind: &str) -> u8 {
    match kind {
        "clock" | "media" | "power" | "load" => 2,
        _ => 1,
    }
}

// ── Message ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    /// Config loaded from disk at startup.
    Loaded(Box<DashConfig>),
    ColSpanInc(usize),
    ColSpanDec(usize),
    RowSpanInc(usize),
    RowSpanDec(usize),
    MoveUp(usize),
    MoveDown(usize),
    RemoveCard(usize),
    /// The pick_list selection changed to this card kind.
    AddCardPick(String),
    /// Confirm adding the currently-selected kind.
    AddCard,
    ColumnsInc,
    ColumnsDec,
    Save,
    SaveDone(Result<(), String>),
}

// ── State ─────────────────────────────────────────────────────────────────────

struct Editor {
    /// Full config (theme + dashboard sections).
    config:      DashConfig,
    /// Currently-selected kind in the "Add card" pick_list.
    add_pick:    Option<String>,
    /// Status message shown after save.
    save_status: Option<String>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            config:      DashConfig::default(),
            add_pick:    Some("clock".to_string()),
            save_status: None,
        }
    }
}

// ── Boot ──────────────────────────────────────────────────────────────────────

fn boot() -> (Editor, Task<Message>) {
    let task = Task::perform(
        async {
            let cfg = load_config(default_path()).unwrap_or_default();
            Box::new(cfg)
        },
        Message::Loaded,
    );
    (Editor::default(), task)
}

// ── Update ────────────────────────────────────────────────────────────────────

fn update(editor: &mut Editor, msg: Message) -> Task<Message> {
    match msg {
        Message::Loaded(cfg) => {
            editor.config = *cfg;
            // Seed pick_list to first available kind not already in items
            editor.add_pick = pick_first_unused_kind(&editor.config.dashboard.items);
        }

        Message::ColSpanInc(i) => {
            if let Some(c) = editor.config.dashboard.items.get_mut(i) {
                c.col_span = (c.col_span + 1).min(4);
            }
        }
        Message::ColSpanDec(i) => {
            if let Some(c) = editor.config.dashboard.items.get_mut(i) {
                c.col_span = (c.col_span).saturating_sub(1).max(1);
            }
        }
        Message::RowSpanInc(i) => {
            if let Some(c) = editor.config.dashboard.items.get_mut(i) {
                c.row_span = (c.row_span + 1).min(3);
            }
        }
        Message::RowSpanDec(i) => {
            if let Some(c) = editor.config.dashboard.items.get_mut(i) {
                c.row_span = (c.row_span).saturating_sub(1).max(1);
            }
        }

        Message::MoveUp(i) => {
            if i > 0 {
                editor.config.dashboard.items.swap(i, i - 1);
            }
        }
        Message::MoveDown(i) => {
            let len = editor.config.dashboard.items.len();
            if i + 1 < len {
                editor.config.dashboard.items.swap(i, i + 1);
            }
        }
        Message::RemoveCard(i) => {
            if i < editor.config.dashboard.items.len() {
                editor.config.dashboard.items.remove(i);
            }
        }

        Message::AddCardPick(kind) => {
            editor.add_pick = Some(kind);
        }
        Message::AddCard => {
            if let Some(kind) = &editor.add_pick {
                let col_span = default_col_span(kind);
                editor.config.dashboard.items.push(CardConfig {
                    kind:     kind.clone(),
                    col_span,
                    row_span: 1,
                });
            }
            editor.add_pick = pick_first_unused_kind(&editor.config.dashboard.items);
        }

        Message::ColumnsInc => {
            editor.config.dashboard.columns =
                (editor.config.dashboard.columns + 1).min(4);
        }
        Message::ColumnsDec => {
            editor.config.dashboard.columns =
                (editor.config.dashboard.columns).saturating_sub(1).max(2);
        }

        Message::Save => {
            let path     = default_path();
            let cfg_snap = editor.config.clone();
            return Task::perform(
                async move { save_config(cfg_snap, path).await },
                Message::SaveDone,
            );
        }
        Message::SaveDone(result) => {
            editor.save_status = Some(match result {
                Ok(()) => "Saved.".to_string(),
                Err(e) => format!("Error: {e}"),
            });
        }
    }
    Task::none()
}

/// Pick the first kind from ALL_CARD_KINDS not already in items, falling back
/// to the first kind overall if all are present.
fn pick_first_unused_kind(items: &[CardConfig]) -> Option<String> {
    let used: std::collections::HashSet<&str> =
        items.iter().map(|c| c.kind.as_str()).collect();
    ALL_CARD_KINDS
        .iter()
        .find(|&&k| !used.contains(k))
        .or_else(|| ALL_CARD_KINDS.first())
        .map(|&k| k.to_string())
}

// ── Save logic ────────────────────────────────────────────────────────────────

async fn save_config(cfg: DashConfig, path: std::path::PathBuf) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Cannot create config dir: {e}"))?;
    }

    // Re-read existing raw TOML so we only touch the [dashboard] section.
    // If the file doesn't exist yet, start from a full serialization.
    let mut doc: toml::Value = if path.exists() {
        let raw = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Cannot read config: {e}"))?;
        toml::from_str(&raw).map_err(|e| format!("TOML parse error: {e}"))?
    } else {
        toml::Value::Table(toml::map::Map::new())
    };

    // Serialize the dashboard section
    let dash_value = toml::Value::try_from(&cfg.dashboard)
        .map_err(|e| format!("Serialize error: {e}"))?;

    if let toml::Value::Table(ref mut root) = doc {
        root.insert("dashboard".to_string(), dash_value);
    }

    let serialized = toml::to_string_pretty(&doc)
        .map_err(|e| format!("TOML serialize error: {e}"))?;

    tokio::fs::write(&path, serialized)
        .await
        .map_err(|e| format!("Cannot write config: {e}"))
}

// ── View ──────────────────────────────────────────────────────────────────────

fn view(editor: &Editor) -> Element<'_, Message> {
    let tc = &editor.config.theme;
    let bg      = hex_to_color(&tc.background);
    let fg      = hex_to_color(&tc.foreground);
    let accent  = hex_to_color(&tc.accent);
    let fsize   = tc.font_size;

    // Panel background — slightly lighter than bg
    let panel_bg = lighten(bg, 0.05);
    // Card preview bg
    let preview_card_bg = if tc.widget_bg.is_empty() {
        lighten(bg, 0.08)
    } else {
        hex_to_color(&tc.widget_bg)
    };
    let border_radius = tc.border_radius;

    // Muted colors
    let muted = Color { a: 0.55, ..fg };

    // ── Left panel: card list ─────────────────────────────────────────────────
    let card_list = view_card_list(
        editor, bg, fg, accent, panel_bg, muted, fsize,
    );

    // ── Right panel: preview ──────────────────────────────────────────────────
    let preview = view_preview(
        editor, bg, fg, accent, preview_card_bg, border_radius, muted, fsize,
    );

    // ── Save button / status ──────────────────────────────────────────────────
    let save_label = editor
        .save_status
        .as_deref()
        .unwrap_or("Save");

    let save_btn = button(
        text(save_label).size(fsize).color(fg),
    )
    .padding(Padding { top: 6.0, right: 16.0, bottom: 6.0, left: 16.0 })
    .style(move |_: &iced::Theme, status| {
        let bg_color = match status {
            button::Status::Hovered | button::Status::Pressed => {
                Color { a: 0.25, ..accent }
            }
            _ => Color { a: 0.15, ..accent },
        };
        button::Style {
            background: Some(Background::Color(bg_color)),
            border: Border {
                radius: 6.0.into(),
                color: Color { a: 0.40, ..accent },
                width: 1.0,
            },
            text_color: fg,
            ..Default::default()
        }
    })
    .on_press(Message::Save);

    // ── Top bar ───────────────────────────────────────────────────────────────
    let top_bar = container(
        row![
            text("bar-editor")
                .size(fsize + 2.0)
                .color(Color { a: 0.85, ..fg }),
            iced::widget::Space::new().width(Length::Fill),
            save_btn,
        ]
        .align_y(Alignment::Center)
        .spacing(12.0),
    )
    .padding(Padding { top: 10.0, right: 16.0, bottom: 10.0, left: 16.0 })
    .width(Length::Fill)
    .style(move |_: &iced::Theme| container::Style {
        background: Some(Background::Color(lighten(bg, 0.04))),
        border: Border {
            color: Color { a: 0.12, ..fg },
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    });

    let divider = rule::horizontal(1);

    // ── Main body: left + right panels ───────────────────────────────────────
    let body = row![card_list, preview]
        .spacing(0.0)
        .height(Length::Fill);

    let root = column![top_bar, divider, body]
        .spacing(0.0)
        .width(Length::Fill)
        .height(Length::Fill);

    container(root)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &iced::Theme| container::Style {
            background: Some(Background::Color(bg)),
            ..Default::default()
        })
        .into()
}

// ── Left panel ────────────────────────────────────────────────────────────────

fn view_card_list<'a>(
    editor:    &'a Editor,
    _bg:       Color,
    fg:        Color,
    accent:    Color,
    panel_bg:  Color,
    muted:     Color,
    fsize:     f32,
) -> Element<'a, Message> {
    let items = &editor.config.dashboard.items;

    let mut rows: Vec<Element<'_, Message>> = Vec::new();

    // Header
    rows.push(
        text("CARDS")
            .size(fsize - 1.0)
            .color(muted)
            .into(),
    );
    rows.push(
        rule::horizontal(1).into(),
    );

    for (i, card) in items.iter().enumerate() {
        let is_first = i == 0;
        let is_last  = i + 1 == items.len();

        let kind_accent = card_accent_color(&card.kind);

        // Kind label with colored dot
        let dot = container(iced::widget::Space::new())
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(8.0))
            .style(move |_: &iced::Theme| container::Style {
                background: Some(Background::Color(Color { a: 0.85, ..kind_accent })),
                border: Border { radius: 99.0.into(), ..Default::default() },
                ..Default::default()
            });

        let kind_label = row![
            dot,
            text(card.kind.clone()).size(fsize - 1.0).color(fg),
        ]
        .spacing(6.0)
        .align_y(Alignment::Center);

        // col_span stepper
        let col_span_row = span_stepper(
            "col",
            card.col_span,
            fsize,
            fg, accent,
            Message::ColSpanDec(i),
            Message::ColSpanInc(i),
        );

        // row_span stepper
        let row_span_row = span_stepper(
            "row",
            card.row_span,
            fsize,
            fg, accent,
            Message::RowSpanDec(i),
            Message::RowSpanInc(i),
        );

        // Reorder + remove buttons
        let up_btn = small_btn("↑", fg, accent, fsize,
            if is_first { None } else { Some(Message::MoveUp(i)) });
        let dn_btn = small_btn("↓", fg, accent, fsize,
            if is_last { None } else { Some(Message::MoveDown(i)) });
        let rm_btn = small_btn("×", fg, Color::from_rgb(0.96, 0.54, 0.67), fsize,
            Some(Message::RemoveCard(i)));

        let controls = row![
            col_span_row,
            iced::widget::Space::new().width(Length::Fixed(8.0)),
            row_span_row,
            iced::widget::Space::new().width(Length::Fill),
            up_btn,
            dn_btn,
            iced::widget::Space::new().width(Length::Fixed(4.0)),
            rm_btn,
        ]
        .align_y(Alignment::Center)
        .spacing(4.0);

        let card_row = container(
            column![kind_label, controls].spacing(6.0),
        )
        .padding(Padding { top: 8.0, right: 10.0, bottom: 8.0, left: 10.0 })
        .width(Length::Fill)
        .style(move |_: &iced::Theme| container::Style {
            background: Some(Background::Color(lighten(panel_bg, 0.03))),
            border: Border {
                radius: 8.0.into(),
                color: Color { a: 0.12, ..fg },
                width: 1.0,
            },
            ..Default::default()
        });

        rows.push(card_row.into());
    }

    // ── Add card row ──────────────────────────────────────────────────────────
    rows.push(rule::horizontal(1).into());

    let kind_options: Vec<String> = ALL_CARD_KINDS.iter().map(|&s| s.to_string()).collect();
    let selected_kind = editor.add_pick.clone();

    let pick = pick_list(
        kind_options,
        selected_kind,
        Message::AddCardPick,
    )
    .text_size(fsize - 1.0)
    .padding(Padding { top: 4.0, right: 8.0, bottom: 4.0, left: 8.0 })
    .style(move |_: &iced::Theme, _| iced::widget::pick_list::Style {
        text_color: fg,
        placeholder_color: muted,
        handle_color: accent,
        background: Background::Color(lighten(panel_bg, 0.04)),
        border: Border {
            radius: 6.0.into(),
            color: Color { a: 0.25, ..fg },
            width: 1.0,
        },
    });

    let add_btn = button(
        text("+ Add").size(fsize - 1.0).color(fg),
    )
    .padding(Padding { top: 4.0, right: 12.0, bottom: 4.0, left: 12.0 })
    .style(move |_: &iced::Theme, status| {
        let alpha = match status {
            button::Status::Hovered | button::Status::Pressed => 0.25,
            _ => 0.14,
        };
        button::Style {
            background: Some(Background::Color(Color { a: alpha, ..accent })),
            border: Border {
                radius: 6.0.into(),
                color: Color { a: 0.35, ..accent },
                width: 1.0,
            },
            text_color: fg,
            ..Default::default()
        }
    })
    .on_press(Message::AddCard);

    let add_row = row![pick, add_btn]
        .spacing(8.0)
        .align_y(Alignment::Center);

    rows.push(add_row.into());

    let list_col = iced::widget::Column::from_vec(rows)
        .spacing(8.0)
        .width(Length::Fill);

    let scrolled = scrollable(list_col).width(Length::Fill).height(Length::Fill);

    container(scrolled)
        .width(Length::Fixed(300.0))
        .height(Length::Fill)
        .padding(Padding { top: 14.0, right: 14.0, bottom: 14.0, left: 14.0 })
        .style(move |_: &iced::Theme| container::Style {
            background: Some(Background::Color(panel_bg)),
            border: Border {
                color: Color { a: 0.12, ..fg },
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// ── Right panel: live preview ─────────────────────────────────────────────────

fn view_preview<'a>(
    editor:        &'a Editor,
    bg:            Color,
    fg:            Color,
    accent:        Color,
    card_bg:       Color,
    border_radius: f32,
    muted:         Color,
    fsize:         f32,
) -> Element<'a, Message> {
    let dash    = &editor.config.dashboard;
    let cols    = dash.columns.clamp(2, 4) as usize;
    let gap     = 12.0f32;

    // Card base size in preview
    let base_w  = 140.0f32;
    let base_h  = 90.0f32;

    // Build rows just as the dashboard does
    let mut grid_rows: Vec<Element<'_, Message>> = Vec::new();
    let mut row_items: Vec<Element<'_, Message>> = Vec::new();
    let mut row_span  = 0usize;

    for card in &dash.items {
        let kind  = card.kind.as_str();
        let col_s = if card.col_span > 1 {
            (card.col_span as usize).min(cols)
        } else {
            (default_col_span(kind) as usize).min(cols)
        };
        let row_s = (card.row_span as usize).max(1);

        if row_span + col_s > cols && !row_items.is_empty() {
            grid_rows.push(
                iced::widget::Row::from_vec(std::mem::take(&mut row_items))
                    .spacing(gap)
                    .align_y(Alignment::Start)
                    .into(),
            );
            row_span = 0;
        }

        let card_w = base_w * col_s as f32 + gap * (col_s - 1) as f32;
        let card_h = base_h * row_s as f32 + gap * (row_s - 1) as f32;
        let kind_accent = card_accent_color(kind);
        let kind_owned  = kind.to_string();

        let card_elem = container(
            column![
                // Accent dot header line
                container(iced::widget::Space::new())
                    .width(Length::Fixed(28.0))
                    .height(Length::Fixed(2.0))
                    .style(move |_: &iced::Theme| container::Style {
                        background: Some(Background::Color(Color { a: 0.70, ..kind_accent })),
                        border: Border { radius: 1.0.into(), ..Default::default() },
                        ..Default::default()
                    }),
                text(kind_owned)
                    .size(fsize - 1.0)
                    .color(Color { a: 0.75, ..fg }),
                text(format!("{}×{}", col_s, row_s))
                    .size(fsize - 3.0)
                    .color(muted),
            ]
            .spacing(4.0)
            .align_x(Alignment::Center),
        )
        .width(Length::Fixed(card_w))
        .height(Length::Fixed(card_h))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .padding(Padding { top: 10.0, right: 10.0, bottom: 10.0, left: 10.0 })
        .style(move |_: &iced::Theme| container::Style {
            background: Some(Background::Color(card_bg)),
            border: Border {
                radius: border_radius.into(),
                color: Color { a: 0.30, ..kind_accent },
                width: 1.0,
            },
            ..Default::default()
        });

        row_items.push(card_elem.into());
        row_span += col_s;

        if row_span >= cols {
            grid_rows.push(
                iced::widget::Row::from_vec(std::mem::take(&mut row_items))
                    .spacing(gap)
                    .align_y(Alignment::Start)
                    .into(),
            );
            row_span = 0;
        }
    }
    if !row_items.is_empty() {
        grid_rows.push(
            iced::widget::Row::from_vec(row_items)
                .spacing(gap)
                .align_y(Alignment::Start)
                .into(),
        );
    }

    let grid = iced::widget::Column::from_vec(grid_rows)
        .spacing(gap)
        .align_x(Alignment::Start);

    // ── Columns stepper ───────────────────────────────────────────────────────
    let cols_stepper = row![
        text("Columns:").size(fsize - 1.0).color(muted),
        small_btn("−", fg, accent, fsize,
            if dash.columns > 2 { Some(Message::ColumnsDec) } else { None }),
        container(
            text(format!("{}", dash.columns)).size(fsize - 1.0).color(fg),
        )
        .width(Length::Fixed(24.0))
        .align_x(Alignment::Center),
        small_btn("+", fg, accent, fsize,
            if dash.columns < 4 { Some(Message::ColumnsInc) } else { None }),
    ]
    .spacing(6.0)
    .align_y(Alignment::Center);

    let scrolled_grid = scrollable(
        container(grid)
            .width(Length::Fill)
            .padding(Padding { top: 4.0, right: 4.0, bottom: 4.0, left: 4.0 }),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    let preview_col = column![
        row![
            text("PREVIEW")
                .size(fsize - 1.0)
                .color(muted),
            iced::widget::Space::new().width(Length::Fill),
            cols_stepper,
        ]
        .align_y(Alignment::Center),
        rule::horizontal(1),
        scrolled_grid,
    ]
    .spacing(10.0)
    .width(Length::Fill)
    .height(Length::Fill);

    container(preview_col)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding { top: 14.0, right: 18.0, bottom: 14.0, left: 18.0 })
        .style(move |_: &iced::Theme| container::Style {
            background: Some(Background::Color(bg)),
            ..Default::default()
        })
        .into()
}

// ── Shared widget helpers ─────────────────────────────────────────────────────

/// A small (30px wide) icon button.
fn small_btn<'a>(
    label:   &'static str,
    fg:      Color,
    accent:  Color,
    fsize:   f32,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let btn = button(
        container(text(label).size(fsize - 1.0).color(fg))
            .width(Length::Fixed(26.0))
            .align_x(Alignment::Center),
    )
    .padding(Padding { top: 2.0, right: 2.0, bottom: 2.0, left: 2.0 })
    .style(move |_: &iced::Theme, status| {
        let alpha = match status {
            button::Status::Hovered | button::Status::Pressed => 0.20,
            _ => 0.08,
        };
        button::Style {
            background: Some(Background::Color(Color { a: alpha, ..accent })),
            border: Border {
                radius: 4.0.into(),
                color: Color { a: 0.15, ..fg },
                width: 1.0,
            },
            text_color: fg,
            ..Default::default()
        }
    });

    if let Some(msg) = on_press {
        btn.on_press(msg).into()
    } else {
        // Disabled appearance
        button(
            container(text(label).size(fsize - 1.0).color(Color { a: 0.25, ..fg }))
                .width(Length::Fixed(26.0))
                .align_x(Alignment::Center),
        )
        .padding(Padding { top: 2.0, right: 2.0, bottom: 2.0, left: 2.0 })
        .style(move |_: &iced::Theme, _| button::Style {
            background: Some(Background::Color(Color { a: 0.04, ..fg })),
            border: Border {
                radius: 4.0.into(),
                color: Color { a: 0.06, ..fg },
                width: 1.0,
            },
            text_color: Color { a: 0.25, ..fg },
            ..Default::default()
        })
        .into()
    }
}

/// A `col: [−][N][+]` or `row: [−][N][+]` stepper widget.
fn span_stepper<'a>(
    label:   &'static str,
    value:   u8,
    fsize:   f32,
    fg:      Color,
    accent:  Color,
    dec_msg: Message,
    inc_msg: Message,
) -> Element<'a, Message> {
    let max: u8 = if label == "col" { 4 } else { 3 };
    let dec_enabled = value > 1;
    let inc_enabled = value < max;

    row![
        text(format!("{label}:")).size(fsize - 2.5).color(Color { a: 0.55, ..fg }),
        small_btn("−", fg, accent, fsize, if dec_enabled { Some(dec_msg) } else { None }),
        container(
            text(format!("{value}")).size(fsize - 2.0).color(fg),
        )
        .width(Length::Fixed(18.0))
        .align_x(Alignment::Center),
        small_btn("+", fg, accent, fsize, if inc_enabled { Some(inc_msg) } else { None }),
    ]
    .spacing(3.0)
    .align_y(Alignment::Center)
    .into()
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> iced::Result {
    iced::application(boot, update, view)
        .title("bar-editor")
        .window_size(Size::new(1100.0, 650.0))
        .run()
}

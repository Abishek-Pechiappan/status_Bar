use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{column, container, row, text},
    Alignment, Background, Border, Element, Length,
};

/// Battery widget with a progress-border design:
///
/// - Border color = charge state (green=charging, red=warn, fg=normal)
/// - Inner horizontal fill strip = charge level
/// - Fully charged + plugged in → only a lightning icon (no clutter)
#[derive(Debug, Default)]
pub struct BatteryWidget;

impl BatteryWidget {
    pub fn new() -> Self {
        Self
    }

    /// Returns `None` when there is no battery — callers should skip rendering.
    pub fn view<'a>(
        &'a self,
        state: &'a AppState,
        theme: &'a Theme,
    ) -> Option<Element<'a, Message>> {
        let pct      = state.system.battery_percent?;
        let charging = state.system.battery_charging.unwrap_or(false);
        let frac     = pct as f32 / 100.0;
        let fsize    = theme.font_size;

        let green = iced::Color::from_rgb8(0xa6, 0xe3, 0xa1); // charging green
        let warn  = iced::Color::from_rgb8(0xf3, 0x8b, 0xa8); // low battery red
        let fg    = theme.foreground.to_iced();

        // ── Fully charged + plugged in: just the lightning icon ───────────────
        if charging && pct >= 100 {
            return Some(
                text(if theme.use_nerd_icons { "\u{f0e7}" } else { "⚡" })
                    .size(fsize + 2.0)
                    .color(green)
                    .into(),
            );
        }

        // ── State color ───────────────────────────────────────────────────────
        let fill_col = if charging {
            green
        } else if pct <= theme.battery_warn_percent {
            warn
        } else {
            fg
        };

        let icon = battery_icon(pct, charging, theme.use_nerd_icons);

        // ── Content row: icon  percentage ─────────────────────────────────────
        let content: Element<'a, Message> = row![
            text(icon).size(fsize).color(fill_col),
            text(format!(" {pct}%")).size(fsize).color(fg),
        ]
        .spacing(2.0)
        .align_y(Alignment::Center)
        .into();

        // ── Thin fill-strip (progress bar) below the text ─────────────────────
        let strip_total = 60.0_f32;
        let fill_w      = (frac.clamp(0.0, 1.0) * strip_total).max(2.0);
        let empty_w     = (strip_total - fill_w).max(0.0);
        let strip_h     = 3.0_f32;

        let track_col = iced::Color { a: 0.15, ..fg };
        let fill_solid = iced::Color { a: 0.90, ..fill_col };

        let filled: Element<'a, Message> = container(
            iced::widget::Space::new()
                .width(Length::Fixed(fill_w))
                .height(Length::Fixed(strip_h)),
        )
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(fill_solid)),
            border: Border { radius: 99.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into();

        let empty: Element<'a, Message> = iced::widget::Space::new()
            .width(Length::Fixed(empty_w))
            .height(Length::Fixed(strip_h))
            .into();

        let strip: Element<'a, Message> = container(
            row![filled, empty].height(Length::Fixed(strip_h)),
        )
        .width(Length::Fixed(strip_total))
        .height(Length::Fixed(strip_h))
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(track_col)),
            border: Border { radius: 99.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into();

        // ── Outer container: colored border that acts as the progress frame ────
        let border_col = iced::Color { a: 0.55, ..fill_col };
        let radius     = theme.border_radius;

        Some(
            container(
                column![content, strip]
                    .spacing(5.0)
                    .align_x(Alignment::Center),
            )
            .padding([4.0, 8.0])
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                border: Border {
                    color: border_col,
                    width: 1.5,
                    radius: radius.into(),
                },
                ..Default::default()
            })
            .into(),
        )
    }
}

fn battery_icon(pct: u8, charging: bool, nerd: bool) -> &'static str {
    if nerd {
        if charging { return "\u{f0e7}"; } // ⚡ nerd flash/bolt
        match pct {
            91..=100 => "\u{f079}",  // 󰁹
            81..=90  => "\u{f082}",  // 󰂂
            71..=80  => "\u{f081}",  // 󰂁
            61..=70  => "\u{f080}",  // 󰂀
            51..=60  => "\u{f07f}",  // 󰁿
            41..=50  => "\u{f07e}",  // 󰁾
            31..=40  => "\u{f07d}",  // 󰁽
            21..=30  => "\u{f07c}",  // 󰁼
            11..=20  => "\u{f07b}",  // 󰁻
            1..=10   => "\u{f07a}",  // 󰁺
            _        => "\u{f083}",  // 󰂃 unknown
        }
    } else {
        if charging { return "⚡"; }
        match pct {
            80..=100 => "█",
            60..=79  => "▊",
            40..=59  => "▌",
            20..=39  => "▎",
            _        => "▏",
        }
    }
}

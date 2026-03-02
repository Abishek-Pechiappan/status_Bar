use bar_core::event::Message;
use bar_theme::Theme;
use iced::{
    widget::{column, container, row},
    Background, Border, Element, Length,
};
use std::collections::VecDeque;

/// Thin horizontal fill bar: accent-colored fill over a dimmed track.
/// `total_w` is the total track width in logical pixels.
pub(crate) fn mini_bar<'a>(frac: f32, total_w: f32, theme: &Theme) -> Element<'a, Message> {
    let fill_w  = (frac.clamp(0.0, 1.0) * total_w).max(0.0);
    let track_h = (theme.font_size * 0.28).max(3.0);
    let accent   = theme.accent.to_iced();
    let track_bg = theme.foreground.with_alpha(0.12).to_iced();

    container(
        row![
            container(
                iced::widget::Space::new()
                    .width(Length::Fixed(fill_w))
                    .height(Length::Fixed(track_h)),
            )
            .style(move |_: &iced::Theme| container::Style {
                background: Some(Background::Color(accent)),
                border: Border { radius: 2.0.into(), ..Default::default() },
                ..Default::default()
            }),
            iced::widget::Space::new().width(Length::Fill),
        ]
        .height(Length::Fixed(track_h)),
    )
    .width(Length::Fixed(total_w))
    .height(Length::Fixed(track_h))
    .style(move |_: &iced::Theme| container::Style {
        background: Some(Background::Color(track_bg)),
        border: Border { radius: 2.0.into(), ..Default::default() },
        ..Default::default()
    })
    .into()
}

/// Color by usage fraction: foreground → accent at 70% → warning red at 85%.
pub(crate) fn usage_color(frac: f32, theme: &Theme) -> iced::Color {
    if frac >= 0.85 {
        iced::Color::from_rgb8(0xf3, 0x8b, 0xa8)
    } else if frac >= 0.70 {
        theme.accent.to_iced()
    } else {
        theme.foreground.to_iced()
    }
}

/// Bottom-anchored sparkline bars built from a rolling history of CPU %.
/// Each sample renders as a 3 px wide bar whose height is proportional to the value.
pub(crate) fn mini_sparkline<'a>(history: &VecDeque<f32>, theme: &Theme) -> Element<'a, Message> {
    let max_h: f32 = theme.font_size;
    let bar_w: f32 = 3.0;
    let accent = theme.accent.to_iced();

    let bars: Vec<Element<'a, Message>> = history
        .iter()
        .map(|&val| {
            let h = ((val / 100.0) * max_h).max(1.0);
            column![
                iced::widget::Space::new().height(Length::Fixed(max_h - h)),
                container(
                    iced::widget::Space::new()
                        .width(Length::Fixed(bar_w))
                        .height(Length::Fixed(h)),
                )
                .style(move |_: &iced::Theme| container::Style {
                    background: Some(Background::Color(accent)),
                    ..Default::default()
                }),
            ]
            .width(Length::Fixed(bar_w))
            .height(Length::Fixed(max_h))
            .into()
        })
        .collect();

    iced::widget::Row::from_vec(bars).spacing(1.0).into()
}

use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::container,
    Element, Length,
};

/// A thin vertical line drawn between other widgets.
///
/// Add `{ kind = "separator" }` to any layout column in `bar.toml`.
#[derive(Debug, Default)]
pub struct SeparatorWidget;

impl SeparatorWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, _state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        let color = theme.foreground.with_alpha(0.25).to_iced();

        container(iced::widget::Space::new())
            .width(iced::Length::Fixed(1.0))
            .height(Length::Fill)
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(color)),
                ..Default::default()
            })
            .into()
    }
}

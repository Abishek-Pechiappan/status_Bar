use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{
    widget::{button, row, text},
    Alignment, Background, Border, Color, Element, Length,
};
use iced::widget::container;

/// Displays currently open windows as clickable chips in the bar.
///
/// Each window appears as a small pill showing its application class name
/// (e.g. `kitty`, `firefox`).  Clicking focuses that window.
/// Windows are shown in the order Hyprland reports them.
#[derive(Debug, Default)]
pub struct TrayWidget;

impl TrayWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Element<'a, Message> {
        if state.clients.is_empty() {
            return text("—")
                .size(theme.font_size)
                .color(theme.foreground.with_alpha(0.35).to_iced())
                .into();
        }

        let fg        = theme.foreground.to_iced();
        let accent    = theme.accent.to_iced();
        let font_size = theme.font_size;

        let chips: Vec<Element<'a, Message>> = state
            .clients
            .iter()
            .map(|client| {
                let is_active = state.active_window.as_deref()
                    .map(|title| title == client.title)
                    .unwrap_or(false);

                let label = app_label(&client.class, theme.use_nerd_icons);
                let addr  = client.address.clone();

                let txt_color = if is_active { accent } else { fg };

                let chip_bg = if is_active {
                    Color::from_rgba8(0xcb, 0xa6, 0xf7, 0.18)
                } else {
                    Color::from_rgba8(0x45, 0x47, 0x5a, 0.30)
                };

                let chip: Element<'a, Message> = container(
                    button(
                        text(label)
                            .size(font_size - 1.0)
                            .color(txt_color),
                    )
                    .on_press(Message::WindowFocusRequested(addr))
                    .padding(0)
                    .style(iced::widget::button::text),
                )
                .padding([3.0, 8.0])
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(chip_bg)),
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into();

                chip
            })
            .collect();

        row(chips)
            .spacing(4.0)
            .align_y(Alignment::Center)
            .width(Length::Shrink)
            .into()
    }
}

/// Map an app class name to a display label.
/// Uses Nerd Font icons for common apps when `nerd` is true.
fn app_label(class: &str, nerd: bool) -> String {
    if nerd {
        let icon = match class.to_lowercase().as_str() {
            c if c.contains("firefox")    => "󰈹",
            c if c.contains("chrome")
              || c.contains("chromium")   => "",
            c if c.contains("kitty")      => "",
            c if c.contains("alacritty")  => "",
            c if c.contains("foot")       => "󰽡",
            c if c.contains("wezterm")    => "",
            c if c.contains("code")
              || c.contains("vscodium")   => "󰨞",
            c if c.contains("discord")    => "󰙯",
            c if c.contains("telegram")   => "",
            c if c.contains("spotify")    => "󰓇",
            c if c.contains("mpv")        => "",
            c if c.contains("vlc")        => "󰕼",
            c if c.contains("thunar")
              || c.contains("nautilus")
              || c.contains("dolphin")    => "󰉋",
            c if c.contains("steam")      => "󰓓",
            c if c.contains("obsidian")   => "󱞁",
            c if c.contains("gimp")       => "",
            c if c.contains("inkscape")   => "",
            c if c.contains("blender")    => "󰂫",
            c if c.contains("slack")      => "󰒱",
            c if c.contains("signal")     => "󰍦",
            c if c.contains("zathura")
              || c.contains("evince")
              || c.contains("okular")     => "󰈦",
            _                             => "󰣆",
        };
        icon.to_string()
    } else {
        // Capitalize first letter and cap at 8 chars for compactness
        let s = class.chars().next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default()
            + &class[class.char_indices().nth(1).map(|(i,_)| i).unwrap_or(class.len())..];
        if s.len() > 8 {
            format!("{}…", &s[..7])
        } else {
            s
        }
    }
}

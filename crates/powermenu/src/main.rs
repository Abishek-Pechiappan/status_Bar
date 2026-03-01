//! `bar-powermenu` — full-screen power menu overlay for the bar.
//!
//! Launched as a child process by the bar's power widget.
//! Reads the bar theme so colours match the rest of the desktop.
//! Press Escape or click any action to dismiss.

use bar_config::{default_path, load as load_config};
use bar_theme::Theme;
use iced::{
    widget::{column, container, row, text},
    Alignment, Background, Border, Color, Element, Length, Subscription, Task,
};
use iced::widget::{button, mouse_area};
use iced_layershell::{
    build_pattern::application,
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::{LayerShellSettings, Settings},
    to_layer_message,
};

fn main() -> iced_layershell::Result {
    application(
        PowerMenu::new,
        PowerMenu::namespace,
        PowerMenu::update,
        PowerMenu::view,
    )
    .subscription(PowerMenu::subscription)
    .style(PowerMenu::style)
    .settings(Settings {
        layer_settings: LayerShellSettings {
            // Anchor to all 4 edges → fills the entire screen.
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            layer:  Layer::Overlay,
            // -1 = don't push other surfaces; we're an overlay.
            exclusive_zone: -1,
            // Request keyboard so Escape works.
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

// ── Message ───────────────────────────────────────────────────────────────────

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    /// User clicked one of the four action cards.
    Act(PowerAction),
    /// Background click or Escape — close without doing anything.
    Dismiss,
    /// Raw keyboard event (for Escape handling).
    KeyEvent(iced::keyboard::Event),
}

#[derive(Debug, Clone, Copy)]
enum PowerAction {
    Lock,
    Sleep,
    Reboot,
    Shutdown,
}

// ── State ─────────────────────────────────────────────────────────────────────

struct PowerMenu {
    theme: Theme,
}

impl PowerMenu {
    fn new() -> (Self, Task<Message>) {
        let config = load_config(default_path()).unwrap_or_default();
        let theme  = Theme::from_config(&config.theme);
        (Self { theme }, Task::none())
    }

    fn namespace() -> String {
        "bar-powermenu".to_string()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Act(action) => {
                let cmd: &[&str] = match action {
                    PowerAction::Lock     => &["loginctl", "lock-session"],
                    PowerAction::Sleep    => &["systemctl", "suspend"],
                    PowerAction::Reboot   => &["systemctl", "reboot"],
                    PowerAction::Shutdown => &["systemctl", "poweroff"],
                };
                let _ = std::process::Command::new(cmd[0]).args(&cmd[1..]).spawn();
                std::process::exit(0);
            }
            Message::Dismiss => std::process::exit(0),
            Message::KeyEvent(iced::keyboard::Event::KeyPressed { key, .. }) => {
                if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) {
                    std::process::exit(0);
                }
            }
            _ => {}
        }
        Task::none()
    }

    // ── View ──────────────────────────────────────────────────────────────────

    fn view(&self) -> Element<'_, Message> {
        let t = &self.theme;

        let actions: &[(&str, &str, PowerAction)] = &[
            ("󰌾", "Lock",     PowerAction::Lock),
            ("󰒲", "Sleep",    PowerAction::Sleep),
            ("󰑓", "Reboot",   PowerAction::Reboot),
            ("󰤆", "Shutdown", PowerAction::Shutdown),
        ];

        let accent   = t.accent.to_iced();
        let fg       = t.foreground.to_iced();
        let card_bg  = Color::from_rgba8(0x1e, 0x1e, 0x2e, 0.90);
        let card_hover = Color::from_rgba8(
            (t.accent.r * 255.0) as u8,
            (t.accent.g * 255.0) as u8,
            (t.accent.b * 255.0) as u8,
            0.22,
        );

        let cards: Vec<Element<'_, Message>> = actions
            .iter()
            .map(|(icon, label, action)| {
                let action = *action;
                let icon_txt = text(*icon).size(40.0).color(accent);
                let label_txt = text(*label).size(14.0).color(fg);

                let card_content = column![icon_txt, label_txt]
                    .spacing(10.0)
                    .align_x(Alignment::Center);

                button(
                    container(card_content)
                        .width(Length::Fixed(120.0))
                        .height(Length::Fixed(130.0))
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center),
                )
                .on_press(Message::Act(action))
                .padding(0)
                .style(move |_: &iced::Theme, status| {
                    let bg = if status == iced::widget::button::Status::Hovered {
                        card_hover
                    } else {
                        card_bg
                    };
                    iced::widget::button::Style {
                        background: Some(Background::Color(bg)),
                        border: Border {
                            radius: 16.0.into(),
                            color: if status == iced::widget::button::Status::Hovered {
                                accent
                            } else {
                                Color::from_rgba8(0x45, 0x47, 0x5a, 0.60)
                            },
                            width: 1.5,
                        },
                        text_color: fg,
                        ..Default::default()
                    }
                })
                .into()
            })
            .collect();

        let card_row = row(cards).spacing(20.0).align_y(Alignment::Center);

        let hint = text("Esc to cancel")
            .size(12.0)
            .color(Color::from_rgba8(0x9f, 0xa2, 0xb5, 0.7));

        let center_panel = column![card_row, hint]
            .spacing(20.0)
            .align_x(Alignment::Center);

        // Wrap in a mouse_area so clicking the dark background dismisses.
        mouse_area(
            container(center_panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .on_press(Message::Dismiss)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::keyboard::listen().map(Message::KeyEvent)
    }

    fn style(&self, _theme: &iced::Theme) -> iced::theme::Style {
        iced::theme::Style {
            // Transparent surface — the dark overlay is drawn by the container.
            background_color: Color::TRANSPARENT,
            text_color: self.theme.foreground.to_iced(),
        }
    }
}

//! `bar-powermenu` — full-screen power menu overlay for the bar.
//!
//! Launched as a child process by the bar's power widget (overlay mode).
//! Reads the bar config so colours and button style match the rest of the desktop.
//! Respects `power_actions` to show only configured actions.
//! Press Escape or click the background to dismiss.

use bar_config::{default_path, load as load_config};
use bar_theme::Theme;
use iced::{
    animation::{Animation, Easing},
    widget::{column, container, mouse_area, row, text},
    Alignment, Background, Border, Color, Element, Length, Subscription, Task,
};
use iced::widget::button;
use iced_layershell::{
    build_pattern::application,
    reexport::{Anchor, KeyboardInteractivity, Layer},
    settings::{LayerShellSettings, Settings},
    to_layer_message,
};
use std::time::{Duration, Instant};

fn main() -> iced_layershell::Result {
    let config = load_config(default_path()).unwrap_or_default();
    let font_name: &'static str = Box::leak(config.theme.font.clone().into_boxed_str());
    let default_font = iced::Font {
        family: iced::font::Family::Name(font_name),
        weight: iced::font::Weight::Normal,
        stretch: iced::font::Stretch::Normal,
        style:  iced::font::Style::Normal,
    };

    application(
        PowerMenu::new,
        PowerMenu::namespace,
        PowerMenu::update,
        PowerMenu::view,
    )
    .subscription(PowerMenu::subscription)
    .style(PowerMenu::style)
    .settings(Settings {
        default_font,
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
    /// User clicked one of the action cards.
    Act(String),
    /// Background click or Escape — close without doing anything.
    Dismiss,
    /// Raw keyboard event (for Escape handling).
    KeyEvent(iced::keyboard::Event),
    /// 60 fps animation tick (active only during entrance animation).
    AnimFrame,
}

// ── State ─────────────────────────────────────────────────────────────────────

struct PowerMenu {
    theme:         Theme,
    lock_command:  String,
    /// Ordered list of action keys to display (from config).
    actions:       Vec<String>,
    /// Button visual style: "icon_label" | "icon_only" | "pill".
    button_style:  String,
    /// Entrance fade-in animation.
    enter_anim:    Animation<bool>,
}

impl PowerMenu {
    fn new() -> (Self, Task<Message>) {
        let config = load_config(default_path()).unwrap_or_default();
        let theme  = Theme::from_config(&config.theme);
        let lock_command  = config.global.lock_command.clone();
        let actions       = config.global.power_actions.clone();
        let button_style  = config.theme.power_button_style.clone();
        // Start the entrance animation immediately.
        let mut enter_anim = Animation::new(false)
            .slow()
            .easing(Easing::EaseOutCubic);
        enter_anim.go_mut(true, Instant::now());
        (Self { theme, lock_command, actions, button_style, enter_anim }, Task::none())
    }

    fn namespace() -> String {
        "bar-powermenu".to_string()
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Act(action) => {
                execute_power_action(action, self.lock_command.clone());
                std::process::exit(0);
            }
            Message::Dismiss => std::process::exit(0),
            Message::KeyEvent(iced::keyboard::Event::KeyPressed { key, .. }) => {
                if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) {
                    std::process::exit(0);
                }
            }
            Message::AnimFrame => {}
            _ => {}
        }
        Task::none()
    }

    // ── View ──────────────────────────────────────────────────────────────────

    fn view(&self) -> Element<'_, Message> {
        let now      = Instant::now();
        let t        = &self.theme;
        let fsize    = t.font_size;
        let accent   = t.accent.to_iced();
        let fg       = t.foreground.to_iced();
        let use_nerd = t.use_nerd_icons;
        let btn_style = self.button_style.as_str();

        // Entrance fade progress (0 = transparent → 1 = fully visible).
        let prog = self.enter_anim.interpolate(0.0f32, 1.0f32, now);

        let card_bg = Color::from_rgba(
            t.background.r,
            t.background.g,
            t.background.b,
            0.88 * prog,
        );
        let card_hover = Color {
            a: 0.22 * prog,
            ..accent
        };

        let all_action_info: &[(&str, &str, &str, &str)] = &[
            // (key, nerd_icon, label, ascii_icon)
            ("lock",      "\u{f033e}", "Lock",      "\u{1f512}"),
            ("sleep",     "\u{f0904}", "Sleep",     "\u{1f4a4}"),
            ("hibernate", "\u{f04b2}", "Hibernate", "\u{1f319}"),
            ("logout",    "\u{f05fd}", "Log Out",   "\u{1f6aa}"),
            ("reboot",    "\u{f0453}", "Reboot",    "\u{1f504}"),
            ("shutdown",  "\u{f0425}", "Shutdown",  "\u{23fb}"),
        ];

        let cards: Vec<Element<'_, Message>> = self.actions
            .iter()
            .filter_map(|action_key| {
                let info = all_action_info.iter().find(|(k, ..)| *k == action_key.as_str())?;
                let (key, nerd_icon, label, ascii_icon) = info;
                let icon  = if use_nerd { *nerd_icon } else { *ascii_icon };
                let key   = key.to_string();
                let a_col = Color { a: accent.a * prog, ..accent };
                let f_col = Color { a: fg.a * prog, ..fg };

                let card_content: Element<'_, Message> = match btn_style {
                    "icon_only" => {
                        container(
                            text(icon).size(fsize + 18.0).color(a_col),
                        )
                        .width(Length::Fixed(110.0))
                        .height(Length::Fixed(110.0))
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .into()
                    }
                    "pill" => {
                        container(
                            row![
                                text(icon).size(fsize + 8.0).color(a_col),
                                text(*label).size(fsize).color(f_col),
                            ]
                            .spacing(8.0)
                            .align_y(Alignment::Center),
                        )
                        .width(Length::Fixed(140.0))
                        .height(Length::Fixed(60.0))
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .into()
                    }
                    _ => { // "icon_label" (default)
                        container(
                            column![
                                text(icon).size(fsize + 18.0).color(a_col),
                                text(*label).size(fsize - 1.0).color(f_col),
                            ]
                            .spacing(10.0)
                            .align_x(Alignment::Center),
                        )
                        .width(Length::Fixed(120.0))
                        .height(Length::Fixed(130.0))
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .into()
                    }
                };

                Some(
                    button(card_content)
                        .on_press(Message::Act(key))
                        .padding(0)
                        .style(move |_: &iced::Theme, status| {
                            let bg = if status == iced::widget::button::Status::Hovered
                                || status == iced::widget::button::Status::Pressed
                            {
                                card_hover
                            } else {
                                card_bg
                            };
                            let border_col = if status == iced::widget::button::Status::Hovered
                                || status == iced::widget::button::Status::Pressed
                            {
                                Color { a: accent.a * prog, ..accent }
                            } else {
                                Color::from_rgba(0.27, 0.28, 0.35, 0.6 * prog)
                            };
                            iced::widget::button::Style {
                                background: Some(Background::Color(bg)),
                                border: Border {
                                    radius: 16.0.into(),
                                    color: border_col,
                                    width: 1.5,
                                },
                                text_color: fg,
                                ..Default::default()
                            }
                        })
                        .into(),
                )
            })
            .collect();

        let card_row = iced::widget::Row::from_vec(cards)
            .spacing(20.0)
            .align_y(Alignment::Center);

        let hint_col = Color::from_rgba(0.62, 0.64, 0.71, 0.7 * prog);
        let hint = text("Esc to cancel").size(fsize - 3.0).color(hint_col);

        let center_panel = column![card_row, hint]
            .spacing(20.0)
            .align_x(Alignment::Center);

        // Dim background with fade-in.
        let overlay_bg = Color::from_rgba(0.0, 0.0, 0.0, 0.55 * prog);

        mouse_area(
            container(center_panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(overlay_bg)),
                    ..Default::default()
                }),
        )
        .on_press(Message::Dismiss)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let now = Instant::now();
        let mut subs = vec![iced::keyboard::listen().map(Message::KeyEvent)];
        if self.enter_anim.is_animating(now) {
            subs.push(
                iced::time::every(Duration::from_millis(16)).map(|_| Message::AnimFrame),
            );
        }
        Subscription::batch(subs)
    }

    fn style(&self, _theme: &iced::Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: Color::TRANSPARENT,
            text_color: self.theme.foreground.to_iced(),
        }
    }
}

// ── Power action execution ─────────────────────────────────────────────────────

fn execute_power_action(action: String, lock_cmd: String) {
    let cmd_str: &str = match action.as_str() {
        "lock"      => &lock_cmd,
        "sleep"     => "systemctl suspend",
        "hibernate" => "systemctl hibernate",
        "logout"    => "hyprctl dispatch exit",
        "reboot"    => "systemctl reboot",
        "shutdown"  => "systemctl poweroff",
        _           => return,
    };
    let mut parts = cmd_str.split_whitespace();
    if let Some(prog) = parts.next() {
        let _ = std::process::Command::new(prog).args(parts).spawn();
    }
}

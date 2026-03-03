//! `bar-powermenu` — bento grid power menu overlay for the bar.
//!
//! Launched as a child process by the bar's power widget (overlay mode).
//! Displays a centered modal with a bento grid of large action cards.
//! Reads the bar config so colours match the rest of the desktop.
//! Press Escape or click the dimmed background to dismiss.

use bar_config::{default_path, load as load_config};
use bar_theme::{Color as ThemeColor, Theme};
use iced::{
    animation::{Animation, Easing},
    widget::{column, container, mouse_area, text},
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
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            layer:  Layer::Overlay,
            exclusive_zone: -1,
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
    Act(String),
    Dismiss,
    KeyEvent(iced::keyboard::Event),
    AnimFrame,
}

// ── State ─────────────────────────────────────────────────────────────────────

struct PowerMenu {
    theme:        Theme,
    lock_command: String,
    actions:      Vec<String>,
    button_style: String,
    enter_anim:   Animation<bool>,
}

impl PowerMenu {
    fn new() -> (Self, Task<Message>) {
        let config       = load_config(default_path()).unwrap_or_default();
        let theme        = Theme::from_config(&config.theme);
        let lock_command = config.global.lock_command.clone();
        let actions      = config.global.power_actions.clone();
        let button_style = config.theme.power_button_style.clone();
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

        // Animation progress: 0.0 = just opened, 1.0 = fully visible.
        let prog = self.enter_anim.interpolate(0.0f32, 1.0f32, now);

        // Slide-up: modal starts 40px below final position and rises.
        let slide_offset = (1.0 - prog) * 40.0;

        // Modal background: blend 18% of fg into bg → lifted card look.
        let bg  = t.background;
        let fgc = t.foreground;
        let mix = 0.18_f32;
        let modal_bg_color = ThemeColor {
            r: (bg.r + (fgc.r - bg.r) * mix).clamp(0.0, 1.0),
            g: (bg.g + (fgc.g - bg.g) * mix).clamp(0.0, 1.0),
            b: (bg.b + (fgc.b - bg.b) * mix).clamp(0.0, 1.0),
            a: 0.97 * prog,
        };
        let modal_bg = modal_bg_color.to_iced();

        // Modal border: subtle accent-tinted edge.
        let modal_border_col = Color { a: accent.a * 0.3 * prog, ..accent };

        // Danger color for reboot / shutdown hover tints.
        let danger_col = Color::from_rgba(0.92, 0.28, 0.28, prog);

        // Dark overlay that covers the whole screen.
        let overlay_bg = Color::from_rgba(0.0, 0.0, 0.0, 0.60 * prog);

        // ── Action meta-table ─────────────────────────────────────────────────
        let all_action_info: &[(&str, &str, &str, &str)] = &[
            ("lock",      "\u{f033e}", "Lock",      "\u{1f512}"),
            ("sleep",     "\u{f0904}", "Sleep",     "\u{1f4a4}"),
            ("hibernate", "\u{f04b2}", "Hibernate", "\u{1f319}"),
            ("logout",    "\u{f05fd}", "Log Out",   "\u{1f6aa}"),
            ("reboot",    "\u{f0453}", "Reboot",    "\u{1f504}"),
            ("shutdown",  "\u{f0425}", "Shutdown",  "\u{23fb}"),
        ];

        // ── Card builder ──────────────────────────────────────────────────────
        let card_bg = Color::from_rgba(
            (bg.r + (fgc.r - bg.r) * 0.08).clamp(0.0, 1.0),
            (bg.g + (fgc.g - bg.g) * 0.08).clamp(0.0, 1.0),
            (bg.b + (fgc.b - bg.b) * 0.08).clamp(0.0, 1.0),
            0.90 * prog,
        );
        let dim_border = Color { a: 0.25 * prog, ..fg };

        let make_card = |action: &str| -> Option<Element<'_, Message>> {
            let info  = all_action_info.iter().find(|(k, ..)| *k == action)?;
            let (key, nerd_icon, label, ascii_icon) = info;
            let icon  = if use_nerd { *nerd_icon } else { *ascii_icon };
            let key   = key.to_string();
            let is_danger = matches!(action, "reboot" | "shutdown");

            let icon_col  = Color { a: accent.a  * prog, ..accent };
            let label_col = Color { a: fg.a * prog, ..fg };

            // Content inside the card — respects button_style config.
            let card_content: Element<'_, Message> = match btn_style {
                "icon_only" => {
                    container(
                        text(icon).size(fsize + 22.0).color(icon_col),
                    )
                    .width(Length::Fixed(150.0))
                    .height(Length::Fixed(160.0))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .into()
                }
                "pill" => {
                    container(
                        iced::widget::row![
                            text(icon).size(fsize + 12.0).color(icon_col),
                            text(*label).size(fsize + 1.0).color(label_col),
                        ]
                        .spacing(10.0)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fixed(170.0))
                    .height(Length::Fixed(80.0))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .padding([0.0, 16.0])
                    .into()
                }
                _ => { // "icon_label" — the bento default
                    container(
                        column![
                            text(icon).size(fsize + 22.0).color(icon_col),
                            text(*label).size(fsize + 1.0).color(label_col),
                        ]
                        .spacing(12.0)
                        .align_x(Alignment::Center),
                    )
                    .width(Length::Fixed(150.0))
                    .height(Length::Fixed(160.0))
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
                        let hovered = status == iced::widget::button::Status::Hovered
                            || status == iced::widget::button::Status::Pressed;
                        let bg_col = if hovered {
                            if is_danger {
                                Color { a: 0.14 * prog, r: 0.92, g: 0.28, b: 0.28 }
                            } else {
                                Color { a: 0.14 * prog, ..accent }
                            }
                        } else {
                            card_bg
                        };
                        let border_col = if hovered {
                            if is_danger { danger_col } else { Color { a: prog, ..accent } }
                        } else {
                            dim_border
                        };
                        iced::widget::button::Style {
                            background: Some(Background::Color(bg_col)),
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
        };

        // ── Build 3-column bento grid ─────────────────────────────────────────
        let action_keys: Vec<&str> = self.actions.iter().map(|s| s.as_str()).collect();
        let grid_rows: Vec<Element<'_, Message>> = action_keys
            .chunks(3)
            .filter_map(|chunk| {
                let row_cards: Vec<Element<'_, Message>> = chunk
                    .iter()
                    .filter_map(|&action| make_card(action))
                    .collect();
                if row_cards.is_empty() {
                    None
                } else {
                    Some(
                        iced::widget::Row::from_vec(row_cards)
                            .spacing(16.0)
                            .align_y(Alignment::Center)
                            .into(),
                    )
                }
            })
            .collect();

        let grid = iced::widget::Column::from_vec(grid_rows)
            .spacing(16.0)
            .align_x(Alignment::Center);

        // ── Hint text ─────────────────────────────────────────────────────────
        let hint_col = Color::from_rgba(
            fgc.r, fgc.g, fgc.b,
            0.45 * prog,
        );
        let hint = text("Esc or click outside to close")
            .size(fsize - 1.0)
            .color(hint_col);

        // ── Modal box ─────────────────────────────────────────────────────────
        let modal = container(
            column![grid, hint]
                .spacing(24.0)
                .align_x(Alignment::Center),
        )
        .padding(iced::Padding {
            top:    40.0,
            right:  48.0,
            bottom: 40.0 + slide_offset,
            left:   48.0,
        })
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(modal_bg)),
            border: Border {
                radius: 20.0.into(),
                color: modal_border_col,
                width: 1.0,
            },
            ..Default::default()
        });

        // ── Full-screen dim overlay with centered modal ───────────────────────
        mouse_area(
            container(modal)
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

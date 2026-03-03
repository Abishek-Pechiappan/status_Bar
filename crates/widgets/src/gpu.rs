use crate::helpers::{mini_bar, usage_color};
use bar_core::{event::Message, state::AppState};
use bar_theme::Theme;
use iced::{widget::text, Alignment, Element};

/// Displays GPU utilisation with a mini progress bar and optional temperature.
///
/// Hidden when no GPU is detected.  Supports AMD (via sysfs) and NVIDIA (nvidia-smi).
#[derive(Debug, Default)]
pub struct GpuWidget;

impl GpuWidget {
    pub fn new() -> Self { Self }

    pub fn view<'a>(&'a self, state: &'a AppState, theme: &'a Theme) -> Option<Element<'a, Message>> {
        let pct  = state.system.gpu_percent?;
        let frac = pct / 100.0;
        let icon = if theme.use_nerd_icons { "󰍹" } else { "GPU" };
        let col  = usage_color(frac, theme);
        let fg   = theme.foreground.to_iced();

        let mut elems: Vec<Element<'a, Message>> = vec![
            text(format!("{icon}  ")).size(theme.font_size).color(fg).into(),
            mini_bar(frac, 44.0, theme),
            text(format!("  {pct:.0}%")).size(theme.font_size).color(col).into(),
        ];

        if let Some(temp) = state.system.gpu_temp {
            elems.push(
                text(format!("  {temp:.0}°"))
                    .size(theme.font_size)
                    .color(fg)
                    .into(),
            );
        }

        Some(
            iced::widget::Row::from_vec(elems)
                .align_y(Alignment::Center)
                .into(),
        )
    }
}

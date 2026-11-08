//! Reusable card container component for Alolite.

use crate::gui::theme::ThemePalette;
use eframe::egui::{Frame, Margin, Response, Rounding, Stroke, Ui};

/// Renders a modern elevated card with standard rounded corners and subtle border.
pub(in crate::gui) fn elevated_card<R>(
    ui: &mut Ui,
    pal: &ThemePalette,
    width: Option<f32>,
    padding: (f32, f32),
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Response {
    Frame::none()
        .fill(pal.bg_elevated)
        .rounding(Rounding::same(16.0))
        .stroke(Stroke::new(1.0, pal.border))
        .inner_margin(Margin::symmetric(padding.0, padding.1))
        .show(ui, |ui| {
            if let Some(w) = width {
                ui.set_width(w);
            }
            add_contents(ui)
        })
        .response
}

//! Reusable button components for Alolite (React-like component architecture).

use crate::gui::theme::ThemePalette;
use eframe::egui::{self, Color32, CursorIcon, Response, RichText, Rounding, Stroke, Ui, Vec2};

/// Standard primary action button (Alolite Blue).
pub(in crate::gui) fn primary_button(
    ui: &mut Ui,
    text: &str,
    pal: &ThemePalette,
    width: Option<f32>,
    enabled: bool,
) -> Response {
    let w = width.unwrap_or_else(|| ui.available_width());
    let btn = egui::Button::new(
        RichText::new(text)
            .color(Color32::WHITE)
            .size(13.8)
            .strong(),
    )
    .fill(if enabled {
        pal.primary
    } else {
        pal.primary.linear_multiply(0.4)
    })
    .stroke(Stroke::NONE)
    .rounding(Rounding::same(8.0))
    .min_size(Vec2::new(w, 38.0));

    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.add_enabled(enabled, btn)
            .on_hover_cursor(CursorIcon::PointingHand)
    })
    .inner
}

/// Standard secondary / outline button.
#[allow(dead_code)]
pub(in crate::gui) fn secondary_button(
    ui: &mut Ui,
    text: &str,
    pal: &ThemePalette,
    width: Option<f32>,
) -> Response {
    let w = width.unwrap_or_else(|| ui.available_width());
    let btn = egui::Button::new(RichText::new(text).color(pal.text_primary).size(13.5))
        .fill(pal.bg_input)
        .stroke(Stroke::new(1.0, pal.border))
        .rounding(Rounding::same(8.0))
        .min_size(Vec2::new(w, 36.0));

    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.add(btn).on_hover_cursor(CursorIcon::PointingHand)
    })
    .inner
}

/// Destructive / Danger button (Red).
#[allow(dead_code)]
pub(in crate::gui) fn danger_button(
    ui: &mut Ui,
    text: &str,
    pal: &ThemePalette,
    width: Option<f32>,
) -> Response {
    let w = width.unwrap_or_else(|| ui.available_width());
    let btn = egui::Button::new(
        RichText::new(text)
            .color(Color32::WHITE)
            .size(13.5)
            .strong(),
    )
    .fill(pal.danger)
    .stroke(Stroke::NONE)
    .rounding(Rounding::same(8.0))
    .min_size(Vec2::new(w, 36.0));

    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.add(btn).on_hover_cursor(CursorIcon::PointingHand)
    })
    .inner
}

#[allow(dead_code)]
/// Segmented tab button (used in switchers).
pub(in crate::gui) fn tab_button(
    ui: &mut Ui,
    text: &str,
    active: bool,
    width: f32,
    pal: &ThemePalette,
) -> Response {
    let btn = egui::Button::new(
        RichText::new(text)
            .size(13.0)
            .color(if active {
                Color32::WHITE
            } else {
                pal.text_secondary
            })
            .strong(),
    )
    .fill(if active {
        pal.primary
    } else {
        Color32::TRANSPARENT
    })
    .stroke(Stroke::NONE)
    .rounding(Rounding::same(7.0))
    .min_size(Vec2::new(width, 32.0));

    ui.add(btn).on_hover_cursor(CursorIcon::PointingHand)
}

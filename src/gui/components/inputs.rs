//! Reusable input components for Alolite (React-like component architecture).

use crate::gui::theme::ThemePalette;
use eframe::egui::{self, Color32, FontId, Margin, Response, RichText, Ui};

/// Props for configuring a standard text input field.
#[derive(Clone, Debug)]
pub struct TextInputProps<'a> {
    pub hint: &'a str,
    pub hint_color: Option<Color32>,
    pub is_password: bool,
    pub height: f32,
    pub width: Option<f32>,
    pub font_size: f32,
    pub horizontal_padding: f32,
    pub vertical_padding: f32,
    pub enabled: bool,
}

impl<'a> Default for TextInputProps<'a> {
    fn default() -> Self {
        Self {
            hint: "",
            hint_color: None,
            is_password: false,
            height: 38.0,
            width: None,
            font_size: 14.0,
            horizontal_padding: 12.0,
            vertical_padding: 9.0,
            enabled: true,
        }
    }
}

/// A perfectly vertically centered, styled text input field.
///
/// Fixes the common egui issue where singleline TextEdit content hugs the top edge.
pub(in crate::gui) fn text_input(
    ui: &mut Ui,
    value: &mut String,
    props: TextInputProps<'_>,
) -> Response {
    let w = props.width.unwrap_or_else(|| ui.available_width());
    let hint_widget: egui::WidgetText = if let Some(color) = props.hint_color {
        RichText::new(props.hint)
            .color(color)
            .font(FontId::proportional(props.font_size))
            .into()
    } else {
        props.hint.into()
    };

    let te = egui::TextEdit::singleline(value)
        .hint_text(hint_widget)
        .password(props.is_password)
        .font(FontId::proportional(props.font_size))
        .margin(Margin::symmetric(
            props.horizontal_padding,
            props.vertical_padding,
        ));

    ui.add_sized([w, props.height], te)
}

/// A labeled input group: Title Label + Centered Text Input + Spacing.
pub(in crate::gui) fn labeled_text_input(
    ui: &mut Ui,
    label: &str,
    value: &mut String,
    hint: &str,
    is_password: bool,
    pal: &ThemePalette,
) -> Response {
    ui.label(
        RichText::new(label)
            .size(13.0)
            .color(pal.text_secondary)
            .strong(),
    );
    ui.add_space(5.0);

    let props = TextInputProps {
        hint,
        hint_color: Some(pal.text_muted),
        is_password,
        height: 38.0,
        width: Some(ui.available_width()),
        font_size: 14.0,
        horizontal_padding: 12.0,
        vertical_padding: 9.0,
        enabled: true,
    };

    let resp = text_input(ui, value, props);
    ui.add_space(12.0);
    resp
}

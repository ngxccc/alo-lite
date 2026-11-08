use super::flags::SettingsActionFlags;
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(super) fn render_tab_appearance(
        &mut self,
        ui: &mut egui::Ui,
        pal: &ThemePalette,
        flags: &mut SettingsActionFlags,
        inner_w: f32,
    ) {
        ui.label(
            RichText::new("Cài đặt giao diện")
                .color(pal.text_primary)
                .size(13.5)
                .strong(),
        );
        ui.add_space(8.0);
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(Margin::symmetric(18.0, 16.0))
            .show(ui, |ui| {
                let card_avail_w = (inner_w - 36.0).max(300.0);
                ui.set_min_width(card_avail_w);
                ui.set_max_width(card_avail_w);
                ui.horizontal(|ui| {
                    let col_w = 132.0_f32;
                    let gap = 18.0_f32;
                    let total_w = col_w * 3.0 + gap * 2.0;
                    let left_pad = ((card_avail_w - total_w) * 0.5).max(0.0);
                    if left_pad > 0.0 {
                        ui.add_space(left_pad);
                    }
                    ui.spacing_mut().item_spacing.x = gap;
                    for (mode_key, label, is_dark_val) in [
                        ("light", "Sáng", false),
                        ("dark", "Tối", true),
                        ("system", "Hệ Thống", true),
                    ] {
                        let selected = self.settings.theme_mode == mode_key;
                        ui.allocate_ui_with_layout(
                            Vec2::new(col_w, 114.0),
                            Layout::top_down(Align::Center),
                            |ui| {
                                let (card_r, c_resp) =
                                    ui.allocate_exact_size(Vec2::new(132.0, 84.0), Sense::click());
                                let stroke_col = if selected {
                                    pal.primary_hover
                                } else if c_resp.hovered() {
                                    pal.text_secondary
                                } else {
                                    pal.border
                                };
                                if mode_key == "system" {
                                    let left_half = Rect::from_min_max(
                                        card_r.min,
                                        Pos2::new(card_r.center().x, card_r.max.y),
                                    );
                                    let right_half = Rect::from_min_max(
                                        Pos2::new(card_r.center().x, card_r.min.y),
                                        card_r.max,
                                    );
                                    ui.painter().rect_filled(
                                        left_half,
                                        Rounding {
                                            nw: 8.0,
                                            sw: 8.0,
                                            ne: 0.0,
                                            se: 0.0,
                                        },
                                        Color32::from_rgb(235, 242, 252),
                                    );
                                    ui.painter().rect_filled(
                                        right_half,
                                        Rounding {
                                            nw: 0.0,
                                            sw: 0.0,
                                            ne: 8.0,
                                            se: 8.0,
                                        },
                                        Color32::from_rgb(24, 26, 31),
                                    );
                                } else {
                                    ui.painter().rect_filled(
                                        card_r,
                                        Rounding::same(8.0),
                                        if is_dark_val {
                                            Color32::from_rgb(24, 26, 31)
                                        } else {
                                            Color32::from_rgb(235, 242, 252)
                                        },
                                    );
                                }
                                ui.painter().rect_stroke(
                                    card_r,
                                    Rounding::same(8.0),
                                    Stroke::new(
                                        if selected { 2.0_f32 } else { 1.0_f32 },
                                        stroke_col,
                                    ),
                                );
                                ui.painter().circle_filled(
                                    Pos2::new(card_r.left() + 18.0, card_r.top() + 22.0),
                                    5.5,
                                    Color32::from_rgb(82, 139, 255),
                                );
                                let mock_in = Rect::from_min_size(
                                    Pos2::new(card_r.left() + 30.0, card_r.top() + 15.0),
                                    Vec2::new(62.0, 22.0),
                                );
                                ui.painter().rect_filled(
                                    mock_in,
                                    Rounding::same(4.0),
                                    if is_dark_val && mode_key != "light" {
                                        Color32::from_rgb(76, 82, 94)
                                    } else {
                                        Color32::WHITE
                                    },
                                );
                                let mock_out = Rect::from_min_size(
                                    Pos2::new(card_r.right() - 56.0, card_r.bottom() - 28.0),
                                    Vec2::new(44.0, 18.0),
                                );
                                ui.painter().rect_filled(
                                    mock_out,
                                    Rounding::same(4.0),
                                    Color32::from_rgb(82, 139, 255),
                                );

                                ui.add_space(6.0);
                                let r_clicked = ui.radio(selected, label).clicked();
                                if c_resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
                                    || r_clicked
                                {
                                    self.settings.theme_mode = mode_key.into();
                                    self.dark_mode = is_dark_val;
                                    flags.theme_switched = true;
                                }
                            },
                        );
                    }
                });
            });
        ui.add_space(16.0);

        ui.label(
            RichText::new("Hình nền chat")
                .color(pal.text_primary)
                .size(13.5)
                .strong(),
        );
        ui.add_space(6.0);
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Sử dụng Avatar làm hình nền")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.use_avatar_as_wallpaper, pal);
                    });
                });
            });
        ui.add_space(16.0);
        ui.label(
            RichText::new("Theme & Sticker Pack tùy chỉnh (Plugin JSON)")
                .color(pal.text_primary)
                .size(13.5)
                .strong(),
        );
        ui.add_space(6.0);
        egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                                .show(ui, |ui| {
                                                    ui.label(RichText::new("Tùy biến bảng màu qua file theme.json hoặc nạp bộ nhãn dán riêng qua custom_stickers.json.").color(pal.text_secondary).size(12.0));
                                                    ui.add_space(8.0);
                                                    ui.horizontal_wrapped(|ui| {
                                                        if ui.button("Xuất mẫu theme.json").clicked() {
                                                            flags.export_theme_tpl = true;
                                                        }
                                                        if ui.button("Tải lại theme.json").clicked() {
                                                            flags.reload_theme_json = true;
                                                        }
                                                        if ui.button("Xuất mẫu custom_stickers.json").clicked() {
                                                            flags.export_sticker_tpl = true;
                                                        }
                                                        if ui.button("Nạp lại Sticker Pack").clicked() {
                                                            flags.reload_sticker_json = true;
                                                        }
                                                    });
                                                });
    }
}

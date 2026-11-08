use super::flags::SettingsActionFlags;
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(super) fn render_tab_utilities(
        &mut self,
        ui: &mut egui::Ui,
        pal: &ThemePalette,
        flags: &mut SettingsActionFlags,
        _inner_w: f32,
    ) {
        ui.label(
            RichText::new("Gợi ý Sticker")
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
                        RichText::new("Hiện gợi ý Sticker phù hợp với nội dung tin nhắn đang soạn")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(
                            ui,
                            &mut self.settings.suggest_stickers_while_typing,
                            pal,
                        );
                    });
                });
            });
        ui.add_space(16.0);

        ui.label(
            RichText::new("Gợi ý @")
                .color(pal.text_primary)
                .size(13.5)
                .strong(),
        );
        ui.add_space(6.0);
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(14.0, 12.0))
            .show(ui, |ui| {
                // Row 1: Gợi ý nhắc tên
                ui.horizontal(|ui| {
                    let (b_rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                    ui.painter().circle_filled(
                        b_rect.center(),
                        15.0,
                        Color32::from_rgb(224, 108, 117),
                    );
                    draw_lucide_icon(
                        ui.painter(),
                        b_rect.center(),
                        lucide::USER,
                        15.0,
                        Color32::WHITE,
                    );
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Gợi ý nhắc tên")
                                .color(pal.text_primary)
                                .size(13.2)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Gợi ý nhắc tên theo nội dung đang soạn")
                                .color(pal.text_secondary)
                                .size(11.8),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.suggest_mentions, pal);
                    });
                });
                ui.add_space(10.0);
                // Row 2: Tìm kiếm GIF
                ui.horizontal(|ui| {
                    let (b_rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                    ui.painter().circle_filled(
                        b_rect.center(),
                        15.0,
                        Color32::from_rgb(82, 139, 255),
                    );
                    ui.painter().text(
                        b_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "GIF",
                        FontId::proportional(10.5),
                        Color32::WHITE,
                    );
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Tìm kiếm GIF")
                                .color(pal.text_primary)
                                .size(13.2)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Gõ từ khóa để tìm GIF")
                                .color(pal.text_secondary)
                                .size(11.8),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.enable_gif_search, pal);
                    });
                });
                ui.add_space(10.0);
                // Row 3: Tìm sticker
                ui.horizontal(|ui| {
                    let (b_rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                    ui.painter().circle_filled(
                        b_rect.center(),
                        15.0,
                        Color32::from_rgb(152, 88, 178),
                    );
                    draw_lucide_icon(
                        ui.painter(),
                        b_rect.center(),
                        lucide::SMILE,
                        15.5,
                        Color32::WHITE,
                    );
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Tìm sticker")
                                .color(pal.text_primary)
                                .size(13.2)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Gõ từ khóa để tìm Sticker")
                                .color(pal.text_secondary)
                                .size(11.8),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(
                            ui,
                            &mut self.settings.enable_sticker_keyword_search,
                            pal,
                        );
                    });
                });
            });
        ui.add_space(16.0);
        ui.label(
            RichText::new("Tự động trả lời (Auto-Reply) & REST API / MCP")
                .color(pal.text_primary)
                .size(13.5)
                .strong(),
        );
        ui.add_space(6.0);
        egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::symmetric(14.0, 12.0))
                                                .show(ui, |ui| {
                                                    let rules = crate::mcp::load_autoreply_rules();
                                                    let active_cnt = rules.iter().filter(|r| r.enabled).count();
                                                    ui.label(RichText::new(format!("Quy tắc Auto-Reply đang bật: {active_cnt}/{} (cấu hình trong autoreply_rules.json)", rules.len())).color(pal.text_primary).size(12.6).strong());
                                                    ui.add_space(4.0);
                                                    ui.label(RichText::new("Khởi chạy REST API cục bộ với `--api-port 3789` hoặc chế độ MCP Server chuẩn JSON-RPC 2.0 với `--mcp` để kết nối Claude Desktop / Cursor / AI Agent.").color(pal.text_secondary).size(11.8));
                                                    ui.add_space(8.0);
                                                    if ui.button("Tạo / Xuất mẫu autoreply_rules.json").clicked() {
                                                        flags.export_autoreply_tpl = true;
                                                    }
                                                });
    }
}

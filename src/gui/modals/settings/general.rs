use super::flags::SettingsActionFlags;
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(super) fn render_tab_general(
        &mut self,
        ui: &mut egui::Ui,
        pal: &ThemePalette,
        flags: &mut SettingsActionFlags,
        inner_w: f32,
    ) {
        ui.label(
            RichText::new("Danh bạ")
                .color(pal.text_primary)
                .size(13.5)
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(RichText::new("Hiển thị danh bạ trong Zalo, bạn có thể chọn hiển thị tất cả danh bạ hoặc chỉ những bạn bè đang sử dụng Zalo").color(pal.text_secondary).size(12.0));
        ui.add_space(6.0);
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(inner_w - 28.0);
                    ui.radio_value(
                        &mut self.settings.show_all_contacts,
                        true,
                        "Hiển thị tất cả danh bạ",
                    );
                    ui.add_space(4.0);
                    ui.radio_value(
                        &mut self.settings.show_all_contacts,
                        false,
                        "Chỉ hiển thị bạn bè đang sử dụng Zalo",
                    );
                });
            });

        ui.label(
            RichText::new("Ngôn ngữ")
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
                        RichText::new("Thay đổi ngôn ngữ")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        egui::ComboBox::from_id_salt("setting_lang_combo")
                            .selected_text(&self.settings.language)
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.language,
                                    "Tiếng Việt".into(),
                                    "Tiếng Việt",
                                );
                                ui.selectable_value(
                                    &mut self.settings.language,
                                    "English".into(),
                                    "English",
                                );
                            });
                    });
                });
            });
        ui.add_space(14.0);

        ui.label(
            RichText::new("Khởi động & Lưu File")
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
                        RichText::new("Khởi động Zalo khi mở máy")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.autostart_on_boot, pal);
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Ghi nhớ tài khoản đăng nhập lần trước")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.remember_login, pal);
                    });
                });
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(
                    RichText::new("File được lưu tại:")
                        .color(pal.text_secondary)
                        .size(12.2),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(truncate_chars(&self.settings.download_dir, 36))
                            .size(12.0)
                            .color(pal.text_muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Thay đổi").clicked() {
                            flags.pick_dl_dir = true;
                        }
                        if ui.button("Mở thư mục").clicked() {
                            flags.open_dl_dir = true;
                        }
                    });
                });
            });

        ui.label(
            RichText::new("🌐 Cấu hình Proxy & Đa tài khoản (Multi-Zalo PC)")
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
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            RichText::new(
                                                                "Định tuyến lưu lượng Zalo qua Proxy (http://, https://, socks5://) và mở nhiều cửa sổ Zalo độc lập với thư mục dữ liệu riêng (--profile <tên>).",
                                                            )
                                                            .color(pal.text_secondary)
                                                            .size(12.0),
                                                        );
                                                        ui.add_space(8.0);
                                                        ui.label(RichText::new("Địa chỉ Proxy (để trống nếu kết nối trực tiếp):").color(pal.text_primary).size(12.6));
                                                        ui.add_space(4.0);
                                                        ui.horizontal(|ui| {
                                                            ui.add(
                                                                egui::TextEdit::singleline(&mut self.settings.proxy_url)
                                                                    .hint_text("http://user:pass@ip:port hoặc socks5://127.0.0.1:1080")
                                                                    .desired_width((inner_w - 190.0).max(160.0)),
                                                            );
                                                            if ui.button("Áp dụng Proxy").clicked() {
                                                                flags.apply_proxy_now = true;
                                                            }
                                                            if ui.button("Tắt").clicked() {
                                                                flags.clear_proxy_now = true;
                                                            }
                                                        });
                                                        ui.add_space(10.0);
                                                        ui.separator();
                                                        ui.add_space(6.0);
                                                        ui.label(RichText::new("Chạy thêm tài khoản Zalo mới trên PC (Multi-Profile):").color(pal.text_primary).size(12.6));
                                                        ui.add_space(4.0);
                                                        ui.horizontal(|ui| {
                                                            ui.label(RichText::new("Tên Profile:").color(pal.text_secondary).size(12.2));
                                                            ui.add(
                                                                egui::TextEdit::singleline(&mut self.settings.multi_profile_name)
                                                                    .hint_text("acc2, work, shop1…")
                                                                    .desired_width(120.0),
                                                            );
                                                            if ui
                                                                .button(RichText::new("➕ Mở thêm cửa sổ Zalo mới").strong())
                                                                .clicked()
                                                            {
                                                                flags.spawn_multi_zalo = true;
                                                            }
                                                        });
                                                    });
                                                });
    }
}

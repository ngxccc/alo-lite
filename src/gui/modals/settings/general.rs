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
        ui.add_space(14.0);

        ui.label(
            RichText::new("Thiết bị Âm thanh & Video (Cuộc gọi)")
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
                let (mics, cams) = crate::media::list_media_devices(false);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{} Đầu vào Mic:", lucide::MIC))
                            .color(pal.text_primary)
                            .size(13.0),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let cur_mic_lbl = if self.settings.selected_mic_device.is_empty() {
                            "Mặc định hệ thống"
                        } else {
                            &self.settings.selected_mic_device
                        };
                        egui::ComboBox::from_id_salt("setting_mic_device_combo")
                            .selected_text(truncate_chars(cur_mic_lbl, 30))
                            .width(230.0)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(
                                        self.settings.selected_mic_device.is_empty(),
                                        "Mặc định hệ thống",
                                    )
                                    .clicked()
                                {
                                    self.settings.selected_mic_device.clear();
                                    crate::media::set_selected_mic_device("");
                                    self.settings.save();
                                }
                                for m in &mics {
                                    if ui
                                        .selectable_label(
                                            self.settings.selected_mic_device == *m,
                                            m,
                                        )
                                        .clicked()
                                    {
                                        self.settings.selected_mic_device = m.clone();
                                        crate::media::set_selected_mic_device(m);
                                        self.settings.save();
                                    }
                                }
                            });
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{} Máy ảnh (Camera):", lucide::VIDEO))
                            .color(pal.text_primary)
                            .size(13.0),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let cur_cam_lbl = if self.settings.selected_camera_device.is_empty() {
                            cams.first()
                                .map(String::as_str)
                                .unwrap_or("Mặc định hệ thống")
                        } else {
                            &self.settings.selected_camera_device
                        };
                        egui::ComboBox::from_id_salt("setting_cam_device_combo")
                            .selected_text(truncate_chars(cur_cam_lbl, 30))
                            .width(230.0)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(
                                        self.settings.selected_camera_device.is_empty(),
                                        "Tự động (Camera đầu tiên)",
                                    )
                                    .clicked()
                                {
                                    self.settings.selected_camera_device.clear();
                                    crate::media::set_selected_camera_device("");
                                    self.settings.save();
                                }
                                for c in &cams {
                                    if ui
                                        .selectable_label(
                                            self.settings.selected_camera_device == *c,
                                            c,
                                        )
                                        .clicked()
                                    {
                                        self.settings.selected_camera_device = c.clone();
                                        crate::media::set_selected_camera_device(c);
                                        self.settings.save();
                                    }
                                }
                            });
                    });
                });
                ui.add_space(6.0);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Độ nhạy Micro (Gain):")
                            .color(pal.text_primary)
                            .size(12.8),
                    );
                    let mut gain_val = self.settings.mic_gain_percent;
                    if ui
                        .add(egui::Slider::new(&mut gain_val, 0..=200).suffix("%"))
                        .changed()
                    {
                        self.settings.mic_gain_percent = gain_val;
                        crate::media::set_mic_gain_percent(gain_val);
                        self.settings.save();
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Mức tín hiệu Mic:")
                            .color(pal.text_secondary)
                            .size(12.0),
                    );
                    let lvl = (crate::media::mic_level_permille() as f32 / 1000.0).clamp(0.0, 1.0);
                    let (m_rect, _) = ui.allocate_exact_size(Vec2::new(180.0, 8.0), Sense::hover());
                    ui.painter()
                        .rect_filled(m_rect, Rounding::same(4.0), pal.bg_input);
                    if lvl > 0.005 {
                        let fill_w = (m_rect.width() * lvl).max(4.0);
                        let fill_col = if lvl > 0.85 {
                            pal.danger
                        } else if lvl > 0.6 {
                            pal.warning
                        } else {
                            pal.accent_online
                        };
                        ui.painter().rect_filled(
                            Rect::from_min_size(m_rect.min, Vec2::new(fill_w, m_rect.height())),
                            Rounding::same(4.0),
                            fill_col,
                        );
                    }
                });
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .small_button(format!("{} Quét lại thiết bị", lucide::REFRESH))
                            .clicked()
                        {
                            let _ = crate::media::list_media_devices(true);
                        }
                    });
                });
            });
        ui.add_space(14.0);

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

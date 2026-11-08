use super::flags::SettingsActionFlags;
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(super) fn render_tab_privacy(
        &mut self,
        ui: &mut egui::Ui,
        pal: &ThemePalette,
        _flags: &mut SettingsActionFlags,
        _inner_w: f32,
    ) {
        ui.label(
            RichText::new("Cá nhân")
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
                        RichText::new("Hiện ngày sinh")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        egui::ComboBox::from_id_salt("setting_bday_combo")
                            .selected_text(&self.settings.show_birthday)
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.show_birthday,
                                    "Không hiện".into(),
                                    "Không hiện",
                                );
                                ui.selectable_value(
                                    &mut self.settings.show_birthday,
                                    "Hiện ngày, tháng, năm".into(),
                                    "Hiện ngày, tháng, năm",
                                );
                                ui.selectable_value(
                                    &mut self.settings.show_birthday,
                                    "Hiện ngày, tháng".into(),
                                    "Hiện ngày, tháng",
                                );
                            });
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Hiển thị trạng thái truy cập")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.show_online_status, pal);
                    });
                });
            });
        ui.add_space(14.0);
        ui.label(
            RichText::new(
                "Chủ động Quyền riêng tư (Chặn gửi trạng thái lên máy chủ — Ctrl+Shift+P)",
            )
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
                                                    let mut priv_changed = false;
                                                    ui.horizontal(|ui| {
                                                        ui.vertical(|ui| {
                                                            ui.label(RichText::new("Ẩn trạng thái \"Đang soạn tin\" (Block Typing)").color(pal.text_primary).size(13.2).strong());
                                                            ui.label(RichText::new("Chặn gửi gói tin /api/message/typing & /api/group/typing khi bạn gõ phím (bạn vẫn thấy khi người khác đang gõ).").color(pal.text_secondary).size(11.5));
                                                        });
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            if render_toggle_switch(ui, &mut self.settings.block_typing, pal).changed() {
                                                                priv_changed = true;
                                                            }
                                                        });
                                                    });
                                                    ui.add_space(6.0);
                                                    ui.separator();
                                                    ui.add_space(6.0);
                                                    ui.horizontal(|ui| {
                                                        ui.vertical(|ui| {
                                                            ui.label(RichText::new("Ẩn trạng thái \"Đã nhận\" (Block Delivered)").color(pal.text_primary).size(13.2).strong());
                                                            ui.label(RichText::new("Chặn gửi xác nhận /api/message/deliveredv2 & /api/group/deliveredv2 về máy chủ.").color(pal.text_secondary).size(11.5));
                                                        });
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            if render_toggle_switch(ui, &mut self.settings.block_delivered, pal).changed() {
                                                                priv_changed = true;
                                                            }
                                                        });
                                                    });
                                                    ui.add_space(6.0);
                                                    ui.separator();
                                                    ui.add_space(6.0);
                                                    ui.horizontal(|ui| {
                                                        ui.vertical(|ui| {
                                                            ui.label(RichText::new("Ẩn trạng thái \"Đã xem\" (Block Seen)").color(pal.text_primary).size(13.2).strong());
                                                            ui.label(RichText::new("Đọc tin nhắn thoải mái mà không gửi xác nhận /api/message/seenv2 & /api/group/seenv2.").color(pal.text_secondary).size(11.5));
                                                        });
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            if render_toggle_switch(ui, &mut self.settings.block_seen, pal).changed() {
                                                                priv_changed = true;
                                                            }
                                                        });
                                                    });
                                                    if priv_changed {
                                                        self.settings.show_seen_status = !self.settings.block_seen;
                                                        crate::session::set_privacy_blocker(
                                                            self.settings.block_typing,
                                                            self.settings.block_delivered,
                                                            self.settings.block_seen,
                                                        );
                                                        self.settings.save();
                                                    }
                                                });
        ui.add_space(14.0);

        ui.label(
            RichText::new("Tin nhắn và cuộc gọi")
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
                        RichText::new("Hiện trạng thái \"Đã xem\"")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.show_seen_status, pal);
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Cho phép nhắn tin")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        egui::ComboBox::from_id_salt("setting_msg_stranger_combo")
                            .selected_text(&self.settings.allow_message_from)
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.allow_message_from,
                                    "Tất cả mọi người".into(),
                                    "Tất cả mọi người",
                                );
                                ui.selectable_value(
                                    &mut self.settings.allow_message_from,
                                    "Chỉ bạn bè".into(),
                                    "Chỉ bạn bè",
                                );
                            });
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Cho phép gọi điện")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        egui::ComboBox::from_id_salt("setting_call_stranger_combo")
                            .selected_text(&self.settings.allow_call_from)
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.allow_call_from,
                                    "Bạn bè và người lạ từng liên hệ".into(),
                                    "Bạn bè và người lạ từng liên hệ",
                                );
                                ui.selectable_value(
                                    &mut self.settings.allow_call_from,
                                    "Tất cả mọi người".into(),
                                    "Tất cả mọi người",
                                );
                                ui.selectable_value(
                                    &mut self.settings.allow_call_from,
                                    "Chỉ bạn bè".into(),
                                    "Chỉ bạn bè",
                                );
                            });
                    });
                });
            });
        ui.add_space(14.0);

        ui.label(
            RichText::new("Nguồn tìm kiếm")
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
                        RichText::new("Cho phép người lạ tìm thấy và kết bạn qua số điện thoại")
                            .color(pal.text_primary)
                            .size(13.0),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.allow_search_by_phone, pal);
                    });
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Cho phép người lạ kết bạn")
                        .color(pal.text_secondary)
                        .size(12.2),
                );
                ui.add_space(4.0);
                ui.checkbox(&mut self.settings.friend_src_qr, "Mã QR của tôi");
                ui.checkbox(&mut self.settings.friend_src_group, "Nhóm chung");
                ui.checkbox(&mut self.settings.friend_src_card, "Danh thiếp Zalo");
            });
    }

    pub(super) fn render_tab_sync(
        &mut self,
        ui: &mut egui::Ui,
        pal: &ThemePalette,
        flags: &mut SettingsActionFlags,
        _inner_w: f32,
    ) {
        ui.label(
            RichText::new(format!(
                "{} Đồng bộ tin nhắn từ điện thoại (1-1 & Nhóm)",
                lucide::PHONE
            ))
            .color(pal.text_primary)
            .size(13.5)
            .strong(),
        );
        ui.add_space(6.0);
        egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(10.0))
                                                .inner_margin(Margin::same(14.0))
                                                .show(ui, |ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            RichText::new(
                                                                "Gửi yêu cầu đồng bộ trực tiếp tới ứng dụng Zalo trên điện thoại (pull_mobile_msg) để kéo toàn bộ lịch sử tin nhắn 1-1 và nhóm từ điện thoại về máy tính.",
                                                            )
                                                            .color(pal.text_primary)
                                                            .size(13.0),
                                                        );
                                                        ui.add_space(4.0);
                                                        ui.label(
                                                            RichText::new(
                                                                "Mẹo: Hãy giữ ứng dụng Zalo trên điện thoại mở và kết nối mạng trong lúc đồng bộ (xác nhận trên điện thoại nếu Zalo hỏi).",
                                                            )
                                                            .color(pal.text_secondary)
                                                            .size(12.2),
                                                        );
                                                        ui.add_space(10.0);
                                                        if crate::sync::PHONE_SYNC_RUNNING
                                                            .load(std::sync::atomic::Ordering::Relaxed)
                                                        {
                                                            ui.add(
                                                                egui::ProgressBar::new(
                                                                    self.phone_sync_progress.clamp(0.05, 1.0),
                                                                )
                                                                .text(&self.phone_sync_status)
                                                                .animate(true),
                                                            );
                                                            ui.add_space(6.0);
                                                            if ui
                                                                .button(RichText::new("Dừng đồng bộ điện thoại").size(13.0))
                                                                .clicked()
                                                            {
                                                                crate::sync::PHONE_SYNC_CANCEL.store(
                                                                    true,
                                                                    std::sync::atomic::Ordering::Relaxed,
                                                                );
                                                            }
                                                        } else {
                                                            if ui
                                                                .button(
                                                                    RichText::new(format!(
                                                                        "{} Đồng bộ tin nhắn từ điện thoại ngay",
                                                                        lucide::REFRESH
                                                                    ))
                                                                    .size(13.0)
                                                                    .strong(),
                                                                )
                                                                .clicked()
                                                            {
                                                                flags.trigger_phone_sync = true;
                                                            }
                                                            if !self.phone_sync_status.is_empty() {
                                                                ui.add_space(6.0);
                                                                ui.label(
                                                                    RichText::new(&self.phone_sync_status)
                                                                        .color(pal.text_secondary)
                                                                        .size(12.0),
                                                                );
                                                            }
                                                        }
                                                    });
                                                });
        ui.add_space(16.0);
        ui.label(
            RichText::new(format!(
                "{} Đồng bộ lịch sử Nhóm & Cloud từ máy chủ",
                lucide::CLOUD
            ))
            .color(pal.text_primary)
            .size(13.5)
            .strong(),
        );
        ui.add_space(6.0);
        egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(10.0))
                                                .inner_margin(Margin::same(14.0))
                                                .show(ui, |ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(RichText::new("Tải lịch sử cũ của các nhóm và Cloud của tôi từ máy chủ Zalo về zalo_history.db, mỗi lần tối đa 1.000 tin/hội thoại; bấm lần nữa để lùi xa hơn.").color(pal.text_primary).size(13.0));
                                                        ui.add_space(4.0);
                                                        if ui
                                                            .add(
                                                                egui::Label::new(
                                                                    RichText::new(format!(
                                                                        "Tìm hiểu chi tiết cơ chế đồng bộ trong docs/TINH_NANG.md {}",
                                                                        lucide::EXTERNAL_LINK
                                                                    ))
                                                                    .size(11.8)
                                                                    .color(pal.primary_hover),
                                                                )
                                                                .sense(Sense::click()),
                                                            )
                                                            .on_hover_cursor(CursorIcon::PointingHand)
                                                            .clicked()
                                                        {
                                                            open_external_url("https://github.com/ngxccc/alolite/blob/main/docs/TINH_NANG.md");
                                                        }
                                                        ui.add_space(4.0);
                                                        let total_msgs: usize = self.threads.iter().map(|t| t.messages.len()).sum();
                                                        ui.label(RichText::new(format!("Hiện có {} hội thoại ({total_msgs} tin nhắn) trong bộ nhớ.", self.threads.len())).color(pal.text_secondary).size(12.2));
                                                        ui.add_space(10.0);
                                                        if crate::sync::RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                                                            ui.label(RichText::new(&self.status_msg).color(pal.primary_hover).size(12.5));
                                                            if ui.button(RichText::new("Dừng").size(13.0)).clicked() {
                                                                crate::sync::CANCEL.store(true, std::sync::atomic::Ordering::Relaxed);
                                                            }
                                                        } else if ui.button(RichText::new(format!("{} Đồng bộ Nhóm & Cloud ngay", lucide::REFRESH)).size(13.0).strong()).clicked() {
                                                            flags.trigger_sync = true;
                                                        }
                                                    });
                                                });
        ui.add_space(16.0);
        ui.label(
            RichText::new(format!(
                "{} Nhập lịch sử tin nhắn từ file sao lưu / Zalo PC (Migrate)",
                lucide::FOLDER_OPEN
            ))
            .color(pal.text_primary)
            .size(13.5)
            .strong(),
        );
        ui.add_space(6.0);
        egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, pal.border))
                                                .rounding(Rounding::same(10.0))
                                                .inner_margin(Margin::same(14.0))
                                                .show(ui, |ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(RichText::new("Nhập tin nhắn từ database SQLite (.db / .sqlite) hoặc JSON (threads_cache.json / bản xuất lịch sử) mà không làm mất tin nhắn đang có.").color(pal.text_secondary).size(12.2));
                                                        ui.add_space(8.0);
                                                        ui.horizontal(|ui| {
                                                            let te = egui::TextEdit::singleline(&mut self.migrate_path_input)
                                                                .hint_text("Đường dẫn file .db, .sqlite, .json hoặc thư mục Zalo…")
                                                                .desired_width(ui.available_width() - 170.0);
                                                            ui.add(te);
                                                            if ui.button(RichText::new("Duyệt…").size(12.5)).clicked() {
                                                                flags.pick_migrate_file = true;
                                                            }
                                                            if ui.button(RichText::new("Nhập ngay").size(12.5).strong()).clicked() {
                                                                flags.run_migrate_input = true;
                                                            }
                                                        });
                                                        ui.add_space(8.0);
                                                        if ui.button(RichText::new(format!("{} Tự động quét & nạp dữ liệu Zalo trên máy", lucide::SEARCH)).size(12.2)).clicked() {
                                                            flags.auto_detect_zalo_dir = true;
                                                        }
                                                    });
                                                });
    }
}

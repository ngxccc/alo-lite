use super::flags::SettingsActionFlags;
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(super) fn render_tab_notifications(
        &mut self,
        ui: &mut egui::Ui,
        pal: &ThemePalette,
        _flags: &mut SettingsActionFlags,
        inner_w: f32,
    ) {
        #[cfg(target_os = "macos")]
        {
            ui.label(RichText::new("Quyền thông báo macOS").strong());
            let authorization = &self.notification_authorization;
            ui.label(if authorization.pending.is_some() {
                "Đang kiểm tra quyền…".to_owned()
            } else {
                match &authorization.result {
                    Some(Ok(status)) => status.label().to_owned(),
                    Some(Err(error)) => error.to_string(),
                    None => "Chưa kiểm tra quyền thông báo".to_owned(),
                }
            });
            ui.label("Quyền hệ điều hành tách biệt với cài đặt Bật/Tắt bên dưới. Chưa bật gửi thông báo native.");
            let ready = authorization.pending.is_none();
            ui.horizontal(|ui| {
                let refresh = ui
                    .add_enabled(ready, egui::Button::new("Đọc lại quyền"))
                    .clicked();
                let request = ui
                    .add_enabled(ready, egui::Button::new("Yêu cầu quyền macOS"))
                    .clicked();
                if refresh || request {
                    if let Some(request_id) = self.notification_authorization.begin() {
                        let command = if request {
                            WorkerCmd::RequestNotificationAuthorization { request_id }
                        } else {
                            WorkerCmd::ReadNotificationAuthorization { request_id }
                        };
                        if worker::WORKER_TX.get().map(|tx| tx.send(command).is_ok()) != Some(true)
                        {
                            self.notification_authorization.finish(
                                request_id,
                                Err(crate::native_notifications::NotificationError::Io),
                            );
                        }
                    }
                }
            });
            ui.add_space(16.0);
        }

        ui.label(
            RichText::new("Cài đặt thông báo")
                .color(pal.text_primary)
                .size(13.5)
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new("Nhận được thông báo mỗi khi có tin nhắn mới")
                .color(pal.text_secondary)
                .size(12.2),
        );
        ui.add_space(8.0);
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(Margin::symmetric(24.0, 16.0))
            .show(ui, |ui| {
                let card_avail_w = (inner_w - 48.0).max(280.0);
                ui.set_min_width(card_avail_w);
                ui.set_max_width(card_avail_w);
                ui.horizontal(|ui| {
                    let col_w = 152.0_f32;
                    let gap = 36.0_f32;
                    let total_w = col_w * 2.0 + gap;
                    let left_pad = ((card_avail_w - total_w) * 0.5).max(0.0);
                    if left_pad > 0.0 {
                        ui.add_space(left_pad);
                    }
                    ui.spacing_mut().item_spacing.x = gap;
                    for (en_val, label) in [(true, "Bật"), (false, "Tắt")] {
                        let selected = self.settings.enable_notifications == en_val;
                        ui.allocate_ui_with_layout(
                            Vec2::new(col_w, 122.0),
                            Layout::top_down(Align::Center),
                            |ui| {
                                let (lap_r, l_resp) =
                                    ui.allocate_exact_size(Vec2::new(148.0, 92.0), Sense::click());
                                let stroke_c = if selected {
                                    pal.primary_hover
                                } else {
                                    pal.border
                                };
                                let scr_r = Rect::from_min_max(
                                    Pos2::new(lap_r.left() + 14.0, lap_r.top() + 6.0),
                                    Pos2::new(lap_r.right() - 14.0, lap_r.bottom() - 16.0),
                                );
                                ui.painter().rect_stroke(
                                    scr_r,
                                    Rounding::same(4.0),
                                    Stroke::new(1.8_f32, stroke_c),
                                );
                                let base_r = Rect::from_min_max(
                                    Pos2::new(lap_r.left() + 6.0, lap_r.bottom() - 16.0),
                                    Pos2::new(lap_r.right() - 6.0, lap_r.bottom() - 6.0),
                                );
                                ui.painter().rect_stroke(
                                    base_r,
                                    Rounding::same(4.0),
                                    Stroke::new(1.8_f32, stroke_c),
                                );
                                if en_val {
                                    let toast_r = Rect::from_min_size(
                                        Pos2::new(scr_r.right() - 62.0, scr_r.top() + 8.0),
                                        Vec2::new(54.0, 22.0),
                                    );
                                    ui.painter().rect_filled(
                                        toast_r,
                                        Rounding::same(3.0),
                                        pal.bg_selected,
                                    );
                                }
                                ui.add_space(6.0);
                                let r_click = ui.radio(selected, label).clicked();
                                if l_resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
                                    || r_click
                                {
                                    self.settings.enable_notifications = en_val;
                                }
                            },
                        );
                    }
                });
            });
        ui.add_space(16.0);

        ui.label(
            RichText::new("Âm thanh thông báo")
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
                        RichText::new("Phát âm thanh khi có tin nhắn & thông báo mới")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.play_notification_sound, pal);
                    });
                });
            });
    }

    pub(super) fn render_tab_messages(
        &mut self,
        ui: &mut egui::Ui,
        pal: &ThemePalette,
        _flags: &mut SettingsActionFlags,
        _inner_w: f32,
    ) {
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Tin nhắn nhanh")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.quick_messages_enabled, pal);
                    });
                });
            });
        ui.add_space(14.0);

        ui.label(
            RichText::new("Thiết lập ẩn trò chuyện")
                .color(pal.text_primary)
                .size(13.5)
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(RichText::new("Đặt mã PIN cho các cuộc trò chuyện riêng tư để tránh người khác nhìn thấy trên máy của bạn").color(pal.text_secondary).size(12.0));
        ui.add_space(6.0);
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Mã PIN").color(pal.text_primary).size(13.2));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.hidden_chat_pin_enabled, pal);
                    });
                });
            });
        ui.add_space(14.0);

        ui.label(
            RichText::new("Chia mục Ưu tiên và Khác")
                .color(pal.text_primary)
                .size(13.5)
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new(
                "Tách riêng trò chuyện không ưu tiên và chuyển sang mục Khác để tập trung hơn",
            )
            .color(pal.text_secondary)
            .size(12.0),
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
                        RichText::new("Chia mục Ưu tiên")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.split_priority_inbox, pal);
                    });
                });
            });
        ui.add_space(14.0);

        ui.label(
            RichText::new("Hiển thị nội dung file Excel")
                .color(pal.text_primary)
                .size(13.5)
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new("Định dạng khi sao chép file Excel sau đó dán vào Zalo")
                .color(pal.text_secondary)
                .size(12.0),
        );
        ui.add_space(6.0);
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                for (val, lbl) in [
                    ("image", "Dạng hình"),
                    ("text", "Dạng chữ"),
                    ("ask", "Luôn hỏi"),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(lbl).color(pal.text_primary).size(13.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.radio_value(&mut self.settings.excel_paste_mode, val.into(), "");
                        });
                    });
                    ui.add_space(4.0);
                }
            });
        ui.add_space(14.0);

        ui.label(
            RichText::new("Phím tắt gửi tin nhắn")
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
                for (val, lbl) in [
                    ("enter", "Nhấn Enter để gửi (Shift + Enter để xuống dòng)"),
                    (
                        "cmd_enter",
                        "Nhấn Cmd/Ctrl + Enter để gửi (Enter để xuống dòng)",
                    ),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(lbl).color(pal.text_primary).size(13.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.radio_value(&mut self.settings.send_message_key, val.into(), "");
                        });
                    });
                    ui.add_space(4.0);
                }
            });
        ui.add_space(14.0);

        ui.label(
            RichText::new("Cài đặt khác")
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
                        RichText::new("Bấm đúp vào vùng cạnh tin nhắn để trả lời")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.double_click_to_reply, pal);
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Bạn sẽ thấy bạn bè bạn đang soạn tin nhắn")
                            .color(pal.text_primary)
                            .size(13.2),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        render_toggle_switch(ui, &mut self.settings.show_typing_indicator, pal);
                    });
                });
            });
    }
}

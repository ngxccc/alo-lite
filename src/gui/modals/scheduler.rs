use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_schedule_modal(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
    ) {
        if !self.schedule_modal_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.schedule_modal_open = false;
            self.schedule_editing_id = None;
            return;
        }

        let mut open = true;
        let cur_thread = self
            .threads
            .get(self.selected_thread_index)
            .map(|t| (t.id.clone(), t.name.clone(), t.is_group));
        let now_ts = crate::clock::unix_now();
        let mut schedule_at: Option<i64> = None;
        let mut action_send_now: Option<crate::scheduler::ScheduledMessage> = None;
        let mut action_cancel: Option<i64> = None;
        let mut action_edit: Option<crate::scheduler::ScheduledMessage> = None;

        let screen = ctx.screen_rect();
        egui::Area::new(Id::new("schedule_modal_backdrop"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(
                    r,
                    Rounding::ZERO,
                    Color32::from_rgba_unmultiplied(0, 0, 0, if pal.is_dark { 155 } else { 95 }),
                );
                if resp.clicked() {
                    open = false;
                }
            });

        let card_w = (screen.width() - 48.0).clamp(380.0, 520.0);
        egui::Area::new(Id::new("schedule_modal_card"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(pal.bg_elevated)
                    .rounding(Rounding::same(14.0))
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, 8.0),
                        blur: 24.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(if pal.is_dark { 120 } else { 40 }),
                    })
                    .inner_margin(Margin::symmetric(20.0, 16.0))
                    .show(ui, |ui| {
                        let inner_w = card_w - 40.0;
                        ui.set_width(inner_w);

                        // 1. Header Row
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("🕒 Hẹn giờ gửi tin nhắn & Nhắc hẹn")
                                    .size(15.5)
                                    .color(pal.text_primary)
                                    .strong(),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let (close_r, close_resp) =
                                    ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                                if close_resp.hovered() {
                                    ui.painter().rect_filled(
                                        close_r,
                                        Rounding::same(7.0),
                                        pal.bg_hover,
                                    );
                                }
                                draw_lucide_icon(
                                    ui.painter(),
                                    close_r.center(),
                                    lucide::X,
                                    14.0,
                                    if close_resp.hovered() {
                                        pal.text_primary
                                    } else {
                                        pal.text_secondary
                                    },
                                );
                                if close_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Đóng (Esc)")
                                    .clicked()
                                {
                                    open = false;
                                }
                            });
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // Notice Banner: Client-side scheduling only (no remote server)
                        let (warn_bg, warn_border, warn_fg) = if pal.is_dark {
                            (
                                Color32::from_rgb(52, 41, 22),
                                Color32::from_rgb(245, 158, 11),
                                Color32::from_rgb(254, 243, 199),
                            )
                        } else {
                            (
                                Color32::from_rgb(255, 251, 235),
                                Color32::from_rgb(217, 119, 6),
                                Color32::from_rgb(146, 64, 14),
                            )
                        };
                        egui::Frame::none()
                            .fill(warn_bg)
                            .stroke(Stroke::new(1.2_f32, warn_border))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::symmetric(12.0, 9.0))
                            .show(ui, |ui| {
                                ui.set_width(inner_w - 24.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "{} Lưu ý: Hiện tại ứng dụng không sử dụng máy chủ trung gian, nên chức năng hẹn giờ gửi tin nhắn chỉ khả dụng khi đang chạy AloLite.",
                                            lucide::ALERT_TRIANGLE
                                        ))
                                        .size(12.5)
                                        .color(warn_fg)
                                        .strong(),
                                    )
                                    .wrap(),
                                );
                            });
                        ui.add_space(8.0);

                        // 2. Target Conversation Badge
                        if let Some((_, ref tname, is_grp)) = cur_thread {
                            egui::Frame::none()
                                .fill(pal.bg_input)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(8.0))
                                .inner_margin(Margin::symmetric(12.0, 7.0))
                                .show(ui, |ui| {
                                    ui.set_width(inner_w - 24.0);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(if is_grp {
                                                "👥 Nhóm đích:"
                                            } else {
                                                "👤 Người nhận:"
                                            })
                                            .size(12.0)
                                            .color(pal.text_secondary),
                                        );
                                        ui.label(
                                            RichText::new(truncate_chars(tname, 38))
                                                .size(12.8)
                                                .color(pal.primary_hover)
                                                .strong(),
                                        );
                                    });
                                });
                            ui.add_space(10.0);
                        }

                        // 3. Message Content Input
                        ui.label(
                            RichText::new("Nội dung tin nhắn sẽ gửi:")
                                .size(12.2)
                                .color(pal.text_secondary)
                                .strong(),
                        );
                        ui.add_space(4.0);
                        egui::Frame::none()
                            .fill(pal.bg_input)
                            .stroke(Stroke::new(1.0_f32, pal.border))
                            .rounding(Rounding::same(9.0))
                            .inner_margin(Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.input_message)
                                        .desired_rows(3)
                                        .frame(false)
                                        .desired_width(inner_w - 20.0)
                                        .hint_text(
                                            "Nhập nội dung tin nhắn hẹn giờ hoặc lời nhắc...",
                                        ),
                                );
                            });

                        // 4. Quick Time Presets (Balanced 4-column grid)
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Chọn mốc thời gian nhanh:")
                                .size(12.2)
                                .color(pal.text_secondary)
                                .strong(),
                        );
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            let btn_w = (inner_w - 18.0) / 4.0;
                            for (preset, short_lbl) in [
                                (crate::scheduler::SchedulePreset::In15Minutes, "⚡ +15 phút"),
                                (crate::scheduler::SchedulePreset::In1Hour, "⏱ +1 giờ"),
                                (
                                    crate::scheduler::SchedulePreset::Today18h,
                                    "🌆 Tối nay 18:00",
                                ),
                                (
                                    crate::scheduler::SchedulePreset::Tomorrow08h,
                                    "🌅 Mai 08:00",
                                ),
                            ] {
                                let btn = egui::Button::new(
                                    RichText::new(short_lbl)
                                        .size(12.0)
                                        .color(pal.text_primary)
                                        .strong(),
                                )
                                .fill(pal.bg_input)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(8.0))
                                .min_size(Vec2::new(btn_w, 32.0));
                                if ui
                                    .add(btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text(preset.label())
                                    .clicked()
                                {
                                    schedule_at = Some(preset.resolve_unix_ts(now_ts));
                                }
                            }
                        });

                        // 5. Custom Time Input + Primary Schedule Button
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Hoặc nhập giờ tùy chỉnh:")
                                .size(12.2)
                                .color(pal.text_secondary)
                                .strong(),
                        );
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            let action_btn_w = 114.0_f32;
                            let custom_w = (inner_w - action_btn_w - 8.0).max(160.0);
                            let te_resp = ui.add_sized(
                                [custom_w, 34.0],
                                egui::TextEdit::singleline(&mut self.schedule_custom_input)
                                    .hint_text("VD: +30m, 18:30, 25/12/2026 09:00")
                                    .margin(Margin::symmetric(10.0, 7.0)),
                            );
                            let enter_custom = te_resp.lost_focus()
                                && ui.input(|i| i.key_pressed(Key::Enter));
                            let sub_btn = egui::Button::new(
                                RichText::new(if self.schedule_editing_id.is_some() {
                                    "Cập nhật lịch"
                                } else {
                                    "🕒 Đặt lịch"
                                })
                                .size(12.5)
                                .color(Color32::WHITE)
                                .strong(),
                            )
                            .fill(pal.primary)
                            .rounding(Rounding::same(8.0))
                            .min_size(Vec2::new(action_btn_w, 34.0));
                            if ui
                                .add(sub_btn)
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                                || enter_custom
                            {
                                match crate::scheduler::parse_custom_schedule_input(
                                    &self.schedule_custom_input,
                                    now_ts,
                                ) {
                                    Ok(ts) => schedule_at = Some(ts),
                                    Err(e) => self.show_toast(e.to_string()),
                                }
                            }
                        });

                        // 6. Pending Scheduled Messages List
                        if !self.scheduled_items.is_empty() {
                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(format!(
                                    "Danh sách tin nhắn đang chờ gửi ({})",
                                    self.scheduled_items.len()
                                ))
                                .size(12.5)
                                .color(pal.text_primary)
                                .strong(),
                            );
                            ui.add_space(4.0);
                            egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .show(ui, |ui| {
                                    for item in &self.scheduled_items {
                                        egui::Frame::none()
                                            .fill(pal.bg_input)
                                            .stroke(Stroke::new(1.0_f32, pal.border))
                                            .rounding(Rounding::same(8.0))
                                            .inner_margin(Margin::symmetric(10.0, 7.0))
                                            .show(ui, |ui| {
                                                ui.set_width(inner_w - 20.0);
                                                ui.horizontal(|ui| {
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "{} · 🕒 {}",
                                                                truncate_chars(
                                                                    &item.thread_name,
                                                                    24
                                                                ),
                                                                crate::scheduler::format_schedule_time(
                                                                    item.deliver_at
                                                                )
                                                            ))
                                                            .size(11.5)
                                                            .color(pal.primary_hover)
                                                            .strong(),
                                                        );
                                                        ui.label(
                                                            RichText::new(truncate_chars(
                                                                &item.text, 42,
                                                            ))
                                                            .size(12.0)
                                                            .color(pal.text_primary),
                                                        );
                                                    });
                                                    ui.with_layout(
                                                        Layout::right_to_left(Align::Center),
                                                        |ui| {
                                                            if ui.small_button("Hủy").clicked() {
                                                                action_cancel = Some(item.id);
                                                            }
                                                            if ui.small_button("Sửa").clicked() {
                                                                action_edit = Some(item.clone());
                                                            }
                                                            if ui.small_button("Gửi ngay").clicked()
                                                            {
                                                                action_send_now =
                                                                    Some(item.clone());
                                                            }
                                                        },
                                                    );
                                                });
                                            });
                                        ui.add_space(4.0);
                                    }
                                });
                        }
                    });
            });
        if let Some(target_ts) = schedule_at {
            let text = self.input_message.trim().to_string();
            if text.is_empty() {
                self.show_toast("Vui lòng nhập nội dung tin nhắn trước khi hẹn giờ");
            } else if let Some((tid, tname, is_group)) = cur_thread {
                if let Some(edit_id) = self.schedule_editing_id.take() {
                    let _ = crate::scheduler::update_scheduled_message(
                        "zalo_history.db",
                        edit_id,
                        &text,
                        target_ts,
                    );
                    self.input_message.clear();
                    self.refresh_scheduled_items();
                    self.show_toast(format!(
                        "Đã cập nhật lịch gửi lúc {}",
                        crate::scheduler::format_schedule_time(target_ts)
                    ));
                } else if crate::scheduler::schedule_message(
                    "zalo_history.db",
                    &tid,
                    &tname,
                    is_group,
                    &text,
                    target_ts,
                    now_ts,
                )
                .is_ok()
                {
                    self.input_message.clear();
                    self.refresh_scheduled_items();
                    self.show_toast(format!(
                        "Đã hẹn giờ gửi tới {tname} lúc {}",
                        crate::scheduler::format_schedule_time(target_ts)
                    ));
                }
            }
        }
        if let Some(id) = action_cancel {
            let _ = crate::scheduler::cancel_scheduled_message("zalo_history.db", id);
            self.refresh_scheduled_items();
            self.show_toast("Đã hủy tin nhắn hẹn giờ");
        }
        if let Some(item) = action_edit {
            self.input_message = item.text;
            self.schedule_editing_id = Some(item.id);
            self.schedule_custom_input = crate::scheduler::format_schedule_time(item.deliver_at);
        }
        if let Some(item) = action_send_now {
            let _ = crate::scheduler::cancel_scheduled_message("zalo_history.db", item.id);
            self.refresh_scheduled_items();
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::SendMessage {
                    to_id: item.thread_id,
                    text: item.text,
                    is_group: item.is_group,
                });
            }
            self.show_toast("Đang gửi ngay tin nhắn hẹn giờ…");
        }
        if !open {
            self.schedule_modal_open = false;
            self.schedule_editing_id = None;
        }
    }
}

use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_phone_lookup_modal(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
    ) {
        if !self.phone_modal_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.phone_modal_open = false;
            return;
        }

        let mut open = true;
        let mut trigger_lookup: Option<String> = None;
        let mut open_chat_user: Option<crate::protocol::social::PhoneLookupResult> = None;
        let mut add_friend_user: Option<(String, String)> = None;

        egui::Window::new("Tìm kiếm Số điện thoại & Kết bạn Zalo")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width((ctx.screen_rect().width() * 0.78).clamp(280.0, 390.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(pal.bg_elevated)
                    .rounding(Rounding::same(14.0))
                    .stroke(Stroke::new(1.2_f32, pal.primary))
                    .inner_margin(Margin::symmetric(20.0, 16.0)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(
                        "Nhập số điện thoại để tìm tài khoản Zalo (VD: 0912345678 hoặc +84...):",
                    )
                    .size(12.2)
                    .color(pal.text_secondary),
                );
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    let te = egui::TextEdit::singleline(&mut self.phone_modal_input)
                        .hint_text("Số điện thoại Zalo...")
                        .margin(Margin::symmetric(10.0, 7.0))
                        .desired_width(250.0);
                    let resp = ui.add(te);
                    let enter = (resp.lost_focus() || resp.has_focus())
                        && ui.input(|i| i.key_pressed(Key::Enter));

                    let search_btn = egui::Button::new(
                        RichText::new("Tìm kiếm")
                            .size(12.8)
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(pal.primary)
                    .rounding(Rounding::same(8.0))
                    .min_size(Vec2::new(88.0, 32.0));

                    if (ui
                        .add(search_btn)
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .clicked()
                        || enter)
                        && !self.phone_modal_input.trim().is_empty()
                    {
                        trigger_lookup = Some(self.phone_modal_input.trim().to_string());
                    }
                });

                ui.add_space(12.0);

                if self.phone_lookup_loading {
                    ui.vertical_centered(|ui| {
                        ui.add_space(16.0);
                        ui.spinner();
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Đang tra cứu hồ sơ Zalo theo số điện thoại…")
                                .size(12.5)
                                .color(pal.text_secondary),
                        );
                        ui.add_space(12.0);
                    });
                } else if let Some(ref err) = self.phone_lookup_error {
                    egui::Frame::none()
                        .fill(pal.danger.linear_multiply(0.14))
                        .stroke(Stroke::new(1.0_f32, pal.danger))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(12.0, 10.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("Không tìm thấy: {err}"))
                                    .size(12.5)
                                    .color(pal.text_primary)
                                    .strong(),
                            );
                        });
                } else if let Some(ref prof) = self.phone_lookup_result {
                    egui::Frame::none()
                        .fill(pal.bg_input)
                        .stroke(Stroke::new(1.0_f32, pal.border))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(Margin::symmetric(14.0, 12.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (av_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(54.0), Sense::hover());
                                paint_user_avatar(
                                    ui.painter(),
                                    av_rect.center(),
                                    27.0,
                                    &prof.display_name,
                                    false,
                                    Some(prof.avatar.as_str()),
                                    &self.sticker_textures,
                                    &mut self.loading_stickers,
                                );

                                ui.add_space(6.0);
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(&prof.display_name)
                                                .size(15.0)
                                                .color(pal.text_primary)
                                                .strong(),
                                        );
                                        let (f_lbl, f_col) = match prof.friendship {
                                            crate::protocol::social::FriendshipStatus::Friend => {
                                                ("✓ Bạn bè", pal.accent_online)
                                            }
                                            crate::protocol::social::FriendshipStatus::IncomingPending => {
                                                ("Chờ bạn đồng ý", pal.primary_hover)
                                            }
                                            crate::protocol::social::FriendshipStatus::OutgoingPending => {
                                                ("Đã gửi lời mời", pal.warning)
                                            }
                                            crate::protocol::social::FriendshipStatus::Stranger => {
                                                ("Chưa kết bạn", Color32::from_rgb(249, 115, 22))
                                            }
                                            crate::protocol::social::FriendshipStatus::Unknown => {
                                                ("Đang kiểm tra…", pal.text_muted)
                                            }
                                        };
                                        egui::Frame::none()
                                            .fill(f_col.linear_multiply(0.15))
                                            .rounding(Rounding::same(5.0))
                                            .inner_margin(Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(f_lbl)
                                                        .size(10.5)
                                                        .color(f_col)
                                                        .strong(),
                                                );
                                            });
                                    });
                                    ui.label(
                                        RichText::new(format!(
                                            "SĐT: {} · UID: {}",
                                            prof.phone, prof.user_id
                                        ))
                                        .size(11.5)
                                        .color(pal.text_secondary),
                                    );
                                    if !prof.status.is_empty() {
                                        ui.label(
                                            RichText::new(&prof.status)
                                                .size(11.5)
                                                .color(pal.text_muted),
                                        );
                                    }
                                });
                            });

                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                let chat_btn = egui::Button::new(
                                    RichText::new("Nhắn tin ngay")
                                        .size(12.5)
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .fill(pal.primary)
                                .rounding(Rounding::same(8.0))
                                .min_size(Vec2::new(155.0, 32.0));
                                if ui
                                    .add(chat_btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    open_chat_user = Some(prof.clone());
                                }

                                match prof.friendship {
                                    crate::protocol::social::FriendshipStatus::Friend => {
                                        let badge_btn = egui::Button::new(
                                            RichText::new(format!(
                                                "{} Đã là bạn bè",
                                                lucide::CHECK
                                            ))
                                            .size(12.2)
                                            .color(pal.accent_online)
                                            .strong(),
                                        )
                                        .fill(pal.accent_online.linear_multiply(0.14))
                                        .stroke(Stroke::new(1.0_f32, pal.accent_online))
                                        .rounding(Rounding::same(8.0))
                                        .min_size(Vec2::new(155.0, 32.0));
                                        ui.add_enabled(false, badge_btn);
                                    }
                                    crate::protocol::social::FriendshipStatus::OutgoingPending => {
                                        let pending_btn = egui::Button::new(
                                            RichText::new("Đã gửi lời mời")
                                                .size(12.2)
                                                .color(pal.warning)
                                                .strong(),
                                        )
                                        .fill(pal.warning.linear_multiply(0.14))
                                        .stroke(Stroke::new(1.0_f32, pal.warning))
                                        .rounding(Rounding::same(8.0))
                                        .min_size(Vec2::new(155.0, 32.0));
                                        ui.add_enabled(false, pending_btn);
                                    }
                                    crate::protocol::social::FriendshipStatus::IncomingPending => {
                                        let accept_btn = egui::Button::new(
                                            RichText::new(format!(
                                                "{} Đồng ý kết bạn",
                                                lucide::CHECK
                                            ))
                                            .size(12.5)
                                            .color(Color32::WHITE)
                                            .strong(),
                                        )
                                        .fill(pal.accent_online)
                                        .rounding(Rounding::same(8.0))
                                        .min_size(Vec2::new(155.0, 32.0));
                                        if ui
                                            .add(accept_btn)
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            add_friend_user = Some((
                                                prof.user_id.clone(),
                                                prof.display_name.clone(),
                                            ));
                                        }
                                    }
                                    _ => {
                                        let friend_btn = egui::Button::new(
                                            RichText::new(format!(
                                                "{} Kết bạn Zalo",
                                                lucide::USER_PLUS
                                            ))
                                            .size(12.5)
                                            .color(pal.accent_online)
                                            .strong(),
                                        )
                                        .fill(pal.accent_online.linear_multiply(0.15))
                                        .stroke(Stroke::new(1.0_f32, pal.accent_online))
                                        .rounding(Rounding::same(8.0))
                                        .min_size(Vec2::new(155.0, 32.0));
                                        if ui
                                            .add(friend_btn)
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            add_friend_user = Some((
                                                prof.user_id.clone(),
                                                prof.display_name.clone(),
                                            ));
                                        }
                                    }
                                }
                            });
                        });
                }
            });

        if let Some(phone_q) = trigger_lookup {
            self.phone_lookup_loading = true;
            self.phone_lookup_error = None;
            self.phone_lookup_result = None;
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::LookupPhone { phone: phone_q });
            }
        }

        if let Some(prof) = open_chat_user {
            if let Some(idx) = self.threads.iter().position(|t| t.id == prof.user_id) {
                self.selected_thread_index = idx;
            } else {
                self.threads.insert(
                    0,
                    Thread {
                        presentation: Default::default(),
                        unread: Default::default(),
                        id: prof.user_id.clone(),
                        name: prof.display_name.clone(),
                        preview: format!("SĐT: {}", prof.phone),
                        time: crate::ui::clock(),
                        avatar: if prof.avatar.is_empty() {
                            None
                        } else {
                            Some(prof.avatar)
                        },
                        is_group: false,
                        messages: Vec::new(),
                    },
                );
                self.selected_thread_index = 0;
                self.persist_threads_async();
            }
            self.search_query.clear();
            self.phone_modal_open = false;
            self.show_toast(format!("Đã mở cuộc trò chuyện với {}", prof.display_name));
        }

        if let Some((uid, uname)) = add_friend_user {
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::AddFriend {
                    user_id: uid,
                    user_name: uname.clone(),
                });
            }
            self.show_toast(format!("Đang gửi lời mời kết bạn tới {uname}…"));
        }

        if !open {
            self.phone_modal_open = false;
        }
    }
}

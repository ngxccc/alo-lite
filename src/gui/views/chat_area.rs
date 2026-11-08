use super::quote_target::render_quote_target;
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_chat_area(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
        thread_meta: &Option<(String, String, Option<String>, bool, usize)>,
    ) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(pal.bg_chat_canvas))
            .show(ctx, |ui| {
                let is_compact_w = false;
                if let Some((thread_id, thread_name, thread_avatar, is_group, _msg_count)) =
            thread_meta.clone()
                {
                    let is_cloud_thread = thread_name.contains("Cloud của tôi");
                    if !is_group
                        && !is_cloud_thread
                        && !self.friend_status_map.contains_key(&thread_id)
                        && self.checking_friend_status.insert(thread_id.clone())
                    {
                        if let Some(tx) = WORKER_TX.get() {
                            let _ = tx.send(WorkerCmd::CheckFriendStatus {
                                user_id: thread_id.clone(),
                            });
                        }
                    }
                    let cur_friendship = if is_group || is_cloud_thread {
                        crate::protocol::social::FriendshipStatus::Friend
                    } else {
                        self.friend_status_map
                            .get(&thread_id)
                            .copied()
                            .unwrap_or(crate::protocol::social::FriendshipStatus::Unknown)
                    };
                    // 1. Clean Conversation Header Bar (No raw 19-digit ID, no bulky sync button)
                    egui::Frame::none()
                        .fill(pal.bg_sidebar)
                        .inner_margin(Margin::symmetric(18.0, 11.0))
                        .stroke(Stroke::new(1.0_f32, pal.border))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // Header Avatar (40x40) with Cutout Online Dot
                                let (av_rect, _) =
                                    ui.allocate_exact_size(Vec2::new(40.0, 40.0), Sense::hover());
                                paint_user_avatar(
                                    ui.painter(),
                                    av_rect.center(),
                                    20.0,
                                    &thread_name,
                                    is_group,
                                    thread_avatar.as_deref(),
                                    &self.sticker_textures,
                                    &mut self.loading_stickers,
                                );
                                let cur_onl = self.user_online_map.get(&thread_id);
                                if !is_group
                                    && cur_onl.map(|(onl, _)| *onl).unwrap_or(false)
                                {
                                    let dot_pos =
                                        Pos2::new(av_rect.right() - 4.0, av_rect.bottom() - 4.0);
                                    ui.painter().circle_filled(dot_pos, 6.2, pal.bg_sidebar);
                                    ui.painter().circle_filled(dot_pos, 4.2, pal.accent_online);
                                }
                                ui.add_space(6.0);
                                // Reserve room for the right-hand action cluster.
                                let actions_w = if is_compact_w { 80.0 } else { 100.0 };
                                let left_info_w =
                                    (ui.available_width() - actions_w).max(60.0);
                                ui.allocate_ui_with_layout(
                                    Vec2::new(left_info_w, 40.0),
                                    Layout::top_down(Align::Min),
                                    |ui| {
                                        ui.spacing_mut().item_spacing.y = 2.0;
                                        ui.horizontal(|ui| {
                                            let name_max = if is_compact_w { 12 } else { 22 };
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(truncate_chars(
                                                        &thread_name,
                                                        name_max,
                                                    ))
                                                    .size(15.0)
                                                    .color(pal.text_primary)
                                                    .strong(),
                                                )
                                                .truncate(),
                                            );
                                            if !is_compact_w && left_info_w > 170.0 {
                                                let (tag_lbl, tag_fg, tag_bg) =
                                                    if is_cloud_thread {
                                                        (
                                                            "Đám mây",
                                                            pal.primary_hover,
                                                            pal.primary.linear_multiply(0.14),
                                                        )
                                                    } else if is_group {
                                                        ("Nhóm", pal.text_secondary, pal.bg_elevated)
                                                    } else {
                                                        match cur_friendship {
                                                            crate::protocol::social::FriendshipStatus::Friend => (
                                                                "✓ Bạn bè",
                                                                pal.accent_online,
                                                                pal.accent_online.linear_multiply(0.14),
                                                            ),
                                                            crate::protocol::social::FriendshipStatus::IncomingPending => (
                                                                "Chờ đồng ý kết bạn",
                                                                pal.primary_hover,
                                                                pal.primary.linear_multiply(0.16),
                                                            ),
                                                            crate::protocol::social::FriendshipStatus::OutgoingPending => (
                                                                "Đã gửi lời mời",
                                                                pal.warning,
                                                                pal.warning.linear_multiply(0.15),
                                                            ),
                                                            crate::protocol::social::FriendshipStatus::Stranger => (
                                                                "Người lạ",
                                                                Color32::from_rgb(249, 115, 22),
                                                                Color32::from_rgb(249, 115, 22)
                                                                    .linear_multiply(0.15),
                                                            ),
                                                            crate::protocol::social::FriendshipStatus::Unknown => (
                                                                "Đang kiểm tra…",
                                                                pal.text_muted,
                                                                pal.bg_elevated,
                                                            ),
                                                        }
                                                    };
                                                egui::Frame::none()
                                                    .fill(tag_bg)
                                                    .rounding(Rounding::same(4.0))
                                                    .inner_margin(Margin::symmetric(6.0, 1.5))
                                                    .show(ui, |ui| {
                                                        ui.label(
                                                            RichText::new(tag_lbl)
                                                                .size(10.2)
                                                                .color(tag_fg)
                                                                .strong(),
                                                        );
                                                    });
                                            }
                                        });
                                        let (sub_status_lbl, sub_status_col): (String, Color32) =
                                            if is_group {
                                                ("Nhóm trò chuyện".into(), pal.text_secondary)
                                            } else if is_cloud_thread {
                                                ("Lưu trữ đám mây cá nhân".into(), pal.text_secondary)
                                            } else {
                                                match cur_friendship {
                                                    crate::protocol::social::FriendshipStatus::Friend => {
                                                        match cur_onl {
                                                            Some((true, lbl)) if !lbl.is_empty() => (
                                                                format!("Đã kết bạn · {lbl}"),
                                                                pal.accent_online,
                                                            ),
                                                            Some((false, lbl)) if !lbl.is_empty() => (
                                                                format!("Đã kết bạn · {lbl}"),
                                                                pal.text_secondary,
                                                            ),
                                                            _ => (
                                                                "Đã kết bạn".into(),
                                                                pal.text_secondary,
                                                            ),
                                                        }
                                                    }
                                                    crate::protocol::social::FriendshipStatus::IncomingPending => (
                                                        "Đã gửi lời mời kết bạn cho bạn".into(),
                                                        pal.primary_hover,
                                                    ),
                                                    crate::protocol::social::FriendshipStatus::OutgoingPending => (
                                                        "Đã gửi lời mời kết bạn · Chờ đồng ý".into(),
                                                        pal.warning,
                                                    ),
                                                    crate::protocol::social::FriendshipStatus::Stranger => (
                                                        "Người lạ · Chưa kết bạn trên Zalo".into(),
                                                        Color32::from_rgb(249, 115, 22),
                                                    ),
                                                    crate::protocol::social::FriendshipStatus::Unknown => {
                                                        match cur_onl {
                                                            Some((true, lbl)) if !lbl.is_empty() => (
                                                                lbl.clone(),
                                                                pal.accent_online,
                                                            ),
                                                            Some((false, lbl)) if !lbl.is_empty() => (
                                                                lbl.clone(),
                                                                pal.text_secondary,
                                                            ),
                                                            _ => (
                                                                "Trò chuyện cá nhân".into(),
                                                                pal.text_secondary,
                                                            ),
                                                        }
                                                    }
                                                }
                                            };
                                        ui.label(
                                            RichText::new(sub_status_lbl)
                                                .size(11.6)
                                                .color(sub_status_col),
                                        );
                                    },
                                );

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    // Info Drawer Toggle Button (hidden on compact widths: auto-collapsed)
                                    if !is_compact_w {
                                        let (drw_rect, drw_resp) = ui.allocate_exact_size(
                                            Vec2::new(34.0, 34.0),
                                            Sense::click(),
                                        );
                                        ui.painter().rect_filled(
                                            drw_rect,
                                            Rounding::same(8.0),
                                            if self.info_drawer_open {
                                                pal.primary.linear_multiply(0.18)
                                            } else if drw_resp.hovered() {
                                                pal.bg_hover
                                            } else {
                                                Color32::TRANSPARENT
                                            },
                                        );
                                        draw_icon_drawer(
                                            ui.painter(),
                                            drw_rect.center(),
                                            17.0,
                                            if self.info_drawer_open {
                                                pal.primary_hover
                                            } else if drw_resp.hovered() {
                                                pal.text_primary
                                            } else {
                                                pal.text_secondary
                                            },
                                        );
                                        if drw_resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .on_hover_text("Thông tin hội thoại")
                                            .clicked()
                                        {
                                            self.info_drawer_open = !self.info_drawer_open;
                                            self.settings.info_drawer_open = self.info_drawer_open;
                                            self.settings.save();
                                        }
                                    }

                                    // Search Messages in Conversation Button (`Ctrl+F`)
                                    let (srch_rect, srch_resp) = ui.allocate_exact_size(
                                        Vec2::new(34.0, 34.0),
                                        Sense::click(),
                                    );
                                    ui.painter().rect_filled(
                                        srch_rect,
                                        Rounding::same(8.0),
                                        if self.in_chat_search_open {
                                            pal.primary.linear_multiply(0.18)
                                        } else if srch_resp.hovered() {
                                            pal.bg_hover
                                        } else {
                                            Color32::TRANSPARENT
                                        },
                                    );
                                    draw_icon_search(
                                        ui.painter(),
                                        srch_rect.center(),
                                        16.5,
                                        if self.in_chat_search_open {
                                            pal.primary_hover
                                        } else if srch_resp.hovered() {
                                            pal.text_primary
                                        } else {
                                            pal.text_secondary
                                        },
                                    );
                                    if srch_resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text("Tìm kiếm tin nhắn trong hội thoại này (Ctrl+F)")
                                        .clicked()
                                        || ui.input(|i| {
                                            (i.modifiers.ctrl || i.modifiers.command)
                                                && i.key_pressed(Key::F)
                                        })
                                    {
                                        self.in_chat_search_open = !self.in_chat_search_open;
                                        if self.in_chat_search_open {
                                            self.in_chat_search_focus_pending = true;
                                        } else {
                                            self.in_chat_search_query.clear();
                                        }
                                    }
                                });
                            });
                        });
                    // 1a. Stranger / Pending Friend Request Banner (matching Zalo PC's top relationship bar)
                    if !is_group
                        && !is_cloud_thread
                        && matches!(
                            cur_friendship,
                            crate::protocol::social::FriendshipStatus::Stranger
                                | crate::protocol::social::FriendshipStatus::IncomingPending
                                | crate::protocol::social::FriendshipStatus::OutgoingPending
                        )
                    {
                        egui::Frame::none()
                            .fill(pal.bg_elevated)
                            .stroke(Stroke::new(1.0_f32, pal.border))
                            .inner_margin(Margin::symmetric(18.0, 7.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    match cur_friendship {
                                        crate::protocol::social::FriendshipStatus::IncomingPending => {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}  {} đã gửi lời mời kết bạn cho bạn",
                                                    lucide::USER_PLUS,
                                                    truncate_chars(&thread_name, 26)
                                                ))
                                                .size(12.2)
                                                .color(pal.primary_hover)
                                                .strong(),
                                            );
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    ui.spacing_mut().item_spacing.x = 6.0;
                                                    let rej_btn = egui::Button::new(
                                                        RichText::new("Từ chối")
                                                            .size(11.5)
                                                            .color(pal.text_secondary),
                                                    )
                                                    .fill(pal.bg_input)
                                                    .stroke(Stroke::new(1.0_f32, pal.border))
                                                    .rounding(Rounding::same(6.0))
                                                    .min_size(Vec2::new(68.0, 26.0));
                                                    if ui
                                                        .add(rej_btn)
                                                        .on_hover_cursor(CursorIcon::PointingHand)
                                                        .clicked()
                                                    {
                                                        if let Some(tx) = WORKER_TX.get() {
                                                            let _ = tx.send(
                                                                WorkerCmd::RejectFriend {
                                                                    user_id: thread_id.clone(),
                                                                    user_name: thread_name.clone(),
                                                                },
                                                            );
                                                        }
                                                    }

                                                    let acc_btn = egui::Button::new(
                                                        RichText::new(format!(
                                                            "{} Đồng ý kết bạn",
                                                            lucide::CHECK
                                                        ))
                                                        .size(11.8)
                                                        .color(Color32::WHITE)
                                                        .strong(),
                                                    )
                                                    .fill(pal.accent_online)
                                                    .rounding(Rounding::same(6.0))
                                                    .min_size(Vec2::new(124.0, 26.0));
                                                    if ui
                                                        .add(acc_btn)
                                                        .on_hover_cursor(CursorIcon::PointingHand)
                                                        .clicked()
                                                    {
                                                        if let Some(tx) = WORKER_TX.get() {
                                                            let _ = tx.send(
                                                                WorkerCmd::AcceptFriend {
                                                                    user_id: thread_id.clone(),
                                                                    user_name: thread_name.clone(),
                                                                },
                                                            );
                                                        }
                                                    }
                                                },
                                            );
                                        }
                                        crate::protocol::social::FriendshipStatus::OutgoingPending => {
                                            ui.label(
                                                RichText::new(format!(
                                                    "Đã gửi lời mời kết bạn tới {} · Đang chờ đồng ý",
                                                    truncate_chars(&thread_name, 24)
                                                ))
                                                .size(12.0)
                                                .color(pal.warning),
                                            );
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    let undo_btn = egui::Button::new(
                                                        RichText::new("Hủy lời mời")
                                                            .size(11.5)
                                                            .color(pal.text_secondary),
                                                    )
                                                    .fill(pal.bg_input)
                                                    .stroke(Stroke::new(1.0_f32, pal.border))
                                                    .rounding(Rounding::same(6.0))
                                                    .min_size(Vec2::new(86.0, 26.0));
                                                    if ui
                                                        .add(undo_btn)
                                                        .on_hover_cursor(CursorIcon::PointingHand)
                                                        .clicked()
                                                    {
                                                        if let Some(tx) = WORKER_TX.get() {
                                                            let _ = tx.send(WorkerCmd::UndoFriend {
                                                                user_id: thread_id.clone(),
                                                                user_name: thread_name.clone(),
                                                            });
                                                        }
                                                    }
                                                },
                                            );
                                        }
                                        crate::protocol::social::FriendshipStatus::Stranger => {
                                            ui.label(
                                                RichText::new(format!(
                                                    "Bạn và {} chưa là bạn bè trên Zalo",
                                                    truncate_chars(&thread_name, 26)
                                                ))
                                                .size(12.0)
                                                .color(pal.text_secondary),
                                            );
                                            ui.with_layout(
                                                Layout::right_to_left(Align::Center),
                                                |ui| {
                                                    let add_btn = egui::Button::new(
                                                        RichText::new(format!(
                                                            "{} Kết bạn",
                                                            lucide::USER_PLUS
                                                        ))
                                                        .size(11.8)
                                                        .color(Color32::WHITE)
                                                        .strong(),
                                                    )
                                                    .fill(pal.primary)
                                                    .rounding(Rounding::same(6.0))
                                                    .min_size(Vec2::new(92.0, 26.0));
                                                    if ui
                                                        .add(add_btn)
                                                        .on_hover_cursor(CursorIcon::PointingHand)
                                                        .clicked()
                                                    {
                                                        if let Some(tx) = WORKER_TX.get() {
                                                            let _ = tx.send(WorkerCmd::AddFriend {
                                                                user_id: thread_id.clone(),
                                                                user_name: thread_name.clone(),
                                                            });
                                                        }
                                                        self.show_toast(format!(
                                                            "Đã gửi lời mời kết bạn tới {}",
                                                            thread_name
                                                        ));
                                                    }
                                                },
                                            );
                                        }
                                        _ => {}
                                    }
                                });
                            });
                    }
                    // 1b. Pinned Message Banner (if any message is pinned in this thread)
                    let pinned_msg_info = self
                        .threads
                        .get(self.selected_thread_index)
                        .and_then(|t| {
                            t.messages
                                .iter()
                                .enumerate()
                                .find(|(_, m)| m.is_pinned)
                                .map(|(idx, m)| {
                                    (
                                        idx,
                                        if m.is_outgoing {
                                            "Tôi".to_string()
                                        } else if !m.sender.is_empty() && m.sender != "0" {
                                            m.sender.clone()
                                        } else {
                                            thread_name.clone()
                                        },
                                        if m.sticker_id.is_some() {
                                            "[Nhãn dán]".to_string()
                                        } else {
                                            truncate_chars(&m.text, 80).into_owned()
                                        },
                                    )
                                })
                        });

                    let mut unpin_idx: Option<usize> = None;
                    if let Some((pidx, psender, ptext)) = pinned_msg_info {
                        egui::Frame::none()
                            .fill(pal.bg_elevated)
                            .stroke(Stroke::new(1.0_f32, pal.primary.linear_multiply(0.4)))
                            .inner_margin(Margin::symmetric(14.0, 7.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("{} Tin ghim:", lucide::PIN))
                                            .size(12.5)
                                            .color(pal.primary)
                                            .strong(),
                                    );
                                    ui.label(
                                        RichText::new(format!("{psender}: {ptext}"))
                                            .size(12.5)
                                            .color(pal.text_primary),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui
                                            .small_button(format!("Bỏ ghim {}", lucide::X))
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            unpin_idx = Some(pidx);
                                        }
                                    });
                                });
                            });
                    }
                    if let Some(uidx) = unpin_idx {
                        if let Some(real) = self
                            .threads
                            .get_mut(self.selected_thread_index)
                            .and_then(|t| t.messages.get_mut(uidx))
                        {
                            real.is_pinned = false;
                        }
                        self.persist_threads_async();
                        self.status_msg = "Đã bỏ ghim tin nhắn".into();
                    }

                    // 1c-0. GitHub Release Update Available Notification Banner
                    if let Some(upd) = self.update_info.clone() {
                        if upd.has_update
                            && !self.update_banner_dismissed
                            && self.settings.skipped_update_version != upd.latest_version
                        {
                            egui::Frame::none()
                                .fill(pal.accent_online.linear_multiply(0.14))
                                .stroke(Stroke::new(1.0_f32, pal.accent_online))
                                .inner_margin(Margin::symmetric(14.0, 6.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let msg_txt = if self.update_downloading {
                                            let (dl, tot) = self.update_progress.unwrap_or((0, 0));
                                            if tot > 0 {
                                                let pct = ((dl as f64 / tot as f64) * 100.0)
                                                    .clamp(0.0, 100.0);
                                                format!(
                                                    "⏳ Đang tải bản cập nhật v{} từ GitHub ({pct:.0}%)…",
                                                    upd.latest_version
                                                )
                                            } else {
                                                format!(
                                                    "⏳ Đang tải bản cập nhật v{} từ GitHub…",
                                                    upd.latest_version
                                                )
                                            }
                                        } else {
                                            format!(
                                                "{} Đã có bản cập nhật mới AloLite v{} trên GitHub Releases!",
                                                lucide::SPARKLES,
                                                upd.latest_version
                                            )
                                        };
                                        let banner_lbl = ui.label(
                                            RichText::new(msg_txt)
                                                .size(12.0)
                                                .color(pal.accent_online)
                                                .strong(),
                                        );
                                        if !upd.release_notes.trim().is_empty() {
                                            banner_lbl.on_hover_text(upd.release_notes.trim());
                                        }
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button(lucide::X)
                                                    .on_hover_cursor(CursorIcon::PointingHand)
                                                    .on_hover_text("Ẩn thanh thông báo cập nhật")
                                                    .clicked()
                                                {
                                                    self.update_banner_dismissed = true;
                                                }
                                                if ui
                                                    .small_button("Chi tiết")
                                                    .on_hover_cursor(CursorIcon::PointingHand)
                                                    .clicked()
                                                {
                                                    self.update_modal_open = true;
                                                }
                                                if !self.update_downloading {
                                                    if let (Some(dl_url), Some(asset_name)) =
                                                        (upd.download_url.clone(), upd.asset_name.clone())
                                                    {
                                                        if ui
                                                            .add(
                                                                egui::Button::new(
                                                                    RichText::new(format!(
                                                                        "⬇ Cập nhật v{}",
                                                                        upd.latest_version
                                                                    ))
                                                                    .size(11.5)
                                                                    .color(Color32::WHITE)
                                                                    .strong(),
                                                                )
                                                                .fill(pal.accent_online)
                                                                .rounding(Rounding::same(5.0)),
                                                            )
                                                            .on_hover_cursor(
                                                                CursorIcon::PointingHand,
                                                            )
                                                            .clicked()
                                                        {
                                                            self.update_downloading = true;
                                                            self.update_progress =
                                                                Some((0, upd.asset_size));
                                                            self.update_modal_open = true;
                                                            if let Some(tx) = WORKER_TX.get() {
                                                                let _ = tx.send(
                                                                    WorkerCmd::DownloadAndApplyUpdate {
                                                                        download_url: dl_url,
                                                                        asset_name,
                                                                        target_version: upd
                                                                            .latest_version
                                                                            .clone(),
                                                                        expected_sha256: upd
                                                                            .expected_sha256
                                                                            .clone(),
                                                                    },
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            },
                                        );
                                    });
                                });
                        }
                    }

                    // 1c. Starred filter active banner
                    if self.filter_starred_only {
                        egui::Frame::none()
                            .fill(Color32::from_rgb(234, 179, 8).linear_multiply(0.12))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(234, 179, 8)))
                            .inner_margin(Margin::symmetric(14.0, 6.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("{} Đang lọc tin nhắn đã đánh dấu sao", lucide::STAR))
                                            .size(12.0)
                                            .color(Color32::from_rgb(234, 179, 8))
                                            .strong(),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui
                                            .small_button(format!("Xem tất cả {}", lucide::X))
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            self.filter_starred_only = false;
                                        }
                                    });
                                });
                            });
                    }

                    // 1d. In-Conversation Message Search Bar (`self.in_chat_search_open`)
                    if self.in_chat_search_open
                        && ui.input(|i| i.key_pressed(Key::Escape)) {
                            self.in_chat_search_open = false;
                            self.in_chat_search_query.clear();
                        }
                    if self.in_chat_search_open {
                        let q_low = self.in_chat_search_query.trim().to_lowercase();
                        let q_folded = fold_vn_search(&q_low);
                        let match_cnt = if q_low.is_empty() {
                            0
                        } else {
                            self.threads
                                .get(self.selected_thread_index)
                                .map(|t| {
                                    t.messages
                                        .iter()
                                        .filter(|m| {
                                            matches_vn_query(&m.text, &q_low, &q_folded)
                                                || matches_vn_query(&m.sender, &q_low, &q_folded)
                                        })
                                        .count()
                                })
                                .unwrap_or(0)
                        };
                        egui::Frame::none()
                            .fill(pal.bg_sidebar)
                            .stroke(Stroke::new(1.0_f32, pal.border))
                            .inner_margin(Margin::symmetric(18.0, 8.0))
                            .show(ui, |ui| {
                                let bar_avail_w = ui.available_width();
                                ui.set_width(bar_avail_w);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;
                                    let right_reserve = if q_low.is_empty() {
                                        86.0
                                    } else {
                                        174.0
                                    };
                                    let pill_w = (bar_avail_w - right_reserve).max(140.0);
                                    egui::Frame::none()
                                        .fill(pal.bg_input)
                                        .rounding(Rounding::same(8.0))
                                        .stroke(Stroke::new(
                                            1.0_f32,
                                            pal.primary.linear_multiply(0.65),
                                        ))
                                        .inner_margin(Margin::symmetric(10.0, 5.0))
                                        .show(ui, |ui| {
                                            let inner_w = (pill_w - 20.0).max(110.0);
                                            ui.set_width(inner_w);
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 6.0;
                                                let (ir, _) = ui.allocate_exact_size(
                                                    Vec2::new(15.0, 18.0),
                                                    Sense::hover(),
                                                );
                                                draw_icon_search(
                                                    ui.painter(),
                                                    ir.center(),
                                                    13.5,
                                                    pal.primary_hover,
                                                );
                                                let has_txt =
                                                    !self.in_chat_search_query.is_empty();
                                                let te_w = if has_txt {
                                                    (inner_w - 44.0).max(60.0)
                                                } else {
                                                    (inner_w - 22.0).max(80.0)
                                                };
                                                let te_resp = ui.add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.in_chat_search_query,
                                                    )
                                                    .id(Id::new((
                                                        "in_chat_search_input",
                                                        &thread_id,
                                                    )))
                                                    .frame(false)
                                                    .hint_text(
                                                        "Tìm tin nhắn trong hội thoại này...",
                                                    )
                                                    .desired_width(te_w),
                                                );
                                                if self.in_chat_search_focus_pending {
                                                    te_resp.request_focus();
                                                    self.in_chat_search_focus_pending = false;
                                                }
                                                if has_txt {
                                                    let (clr_r, clr_resp) = ui
                                                        .allocate_exact_size(
                                                            Vec2::splat(16.0),
                                                            Sense::click(),
                                                        );
                                                    if clr_resp.hovered() {
                                                        ui.painter().circle_filled(
                                                            clr_r.center(),
                                                            8.0,
                                                            pal.bg_hover,
                                                        );
                                                    }
                                                    draw_lucide_icon(
                                                        ui.painter(),
                                                        clr_r.center(),
                                                        lucide::X,
                                                        11.5,
                                                        pal.text_muted,
                                                    );
                                                    if clr_resp
                                                        .on_hover_cursor(CursorIcon::PointingHand)
                                                        .on_hover_text("Xóa từ khóa")
                                                        .clicked()
                                                    {
                                                        self.in_chat_search_query.clear();
                                                    }
                                                }
                                            });
                                        });

                                    if !q_low.is_empty() {
                                        let badge_col = if match_cnt > 0 {
                                            pal.accent_online
                                        } else {
                                            pal.danger
                                        };
                                        egui::Frame::none()
                                            .fill(badge_col.linear_multiply(0.14))
                                            .rounding(Rounding::same(6.0))
                                            .inner_margin(Margin::symmetric(8.0, 4.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(if match_cnt > 0 {
                                                        format!("{match_cnt} kết quả")
                                                    } else {
                                                        "0 kết quả".to_string()
                                                    })
                                                    .size(11.5)
                                                    .color(badge_col)
                                                    .strong(),
                                                );
                                            });
                                    }

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let close_btn = egui::Button::new(
                                            RichText::new(format!("{} Đóng", lucide::X))
                                                .size(11.8)
                                                .color(pal.text_secondary)
                                                .strong(),
                                        )
                                        .fill(pal.bg_elevated)
                                        .stroke(Stroke::new(1.0_f32, pal.border))
                                        .rounding(Rounding::same(6.0))
                                        .min_size(Vec2::new(66.0, 28.0));
                                        if ui
                                            .add(close_btn)
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .on_hover_text("Đóng thanh tìm kiếm (Esc)")
                                            .clicked()
                                        {
                                            self.in_chat_search_open = false;
                                            self.in_chat_search_query.clear();
                                        }
                                    });
                                });
                            });
                    }

                    // 2. Scrollable Messages Area
                    let picker_reserve = (if self.sticker_picker_open { 268.0 } else { 0.0 })
                        + (if self.replying_to.is_some() { 48.0 } else { 0.0 })
                        + (if self.pending_clipboard_image.is_some() {
                            64.0
                        } else {
                            0.0
                        });
                    let mut pending_action: Option<(usize, BubbleAction)> = None;
                    let mut load_older_clicked = false;
                    let mut custom_toast: Option<String> = None;
                    // each frame, so a 2 000-message group re-laid out 2 000 galleys per repaint.
                    // "Tải thêm tin nhắn cũ" widens the window before asking the server.
                    if self.chat_render_window.0 != thread_id {
                        self.chat_render_window = (thread_id.clone(), CHAT_RENDER_STEP);
                    }
                    let jump_index = self.quote_jump.as_ref().and_then(|jump| {
                        self.threads
                            .get(self.selected_thread_index)
                            .and_then(|thread| jump.target_index(&thread_id, &thread.messages))
                    });
                    if jump_index.is_none() {
                        self.quote_jump = None;
                    }
                    let jump_scroll_pending = self
                        .quote_jump
                        .as_ref()
                        .map(|jump| jump.scroll_pending)
                        .unwrap_or(false);
                    let mut jump_scrolled = false;
                    if let Some(index) = jump_index {
                        let count = self
                            .threads
                            .get(self.selected_thread_index)
                            .map(|thread| thread.messages.len())
                            .unwrap_or(0);
                        self.chat_render_window.1 = crate::message_navigation::render_limit_for_target(
                            self.chat_render_window.1,
                            count,
                            index,
                        );
                    }
                    let render_limit = self.chat_render_window.1;
                    let mut widen_window = false;
                    // Group sender -> avatar, built once per frame instead of a scan per bubble.
                    let avatar_by_name: HashMap<&str, &str> = if is_group {
                        self.threads
                            .iter()
                            .filter(|th| !th.is_group)
                            .filter_map(|th| Some((th.name.as_str(), th.avatar.as_deref()?)))
                            .collect()
                    } else {
                        HashMap::new()
                    };
                    let composer_lines = self
                        .input_message
                        .split('\n')
                        .count()
                        .clamp(1, 5);
                    let composer_extra_h = (composer_lines.saturating_sub(1) as f32) * 20.0;
                    let msg_area_height = (ui.available_height()
                        - 68.0
                        - composer_extra_h
                        - picker_reserve)
                        .max(100.0);
                    let chat_scroll_out = egui::ScrollArea::vertical()
                        .id_salt(Id::new(("chat_scroll", &thread_id)))
                        .max_height(msg_area_height)
                        .auto_shrink([false, false])
                        .stick_to_bottom(jump_index.is_none())
                        .show(ui, |ui| {
                            egui::Frame::none()
                                .inner_margin(Margin::symmetric(18.0, 12.0))
                                .show(ui, |ui| {
                                    if let Some(thread) =
                                        self.threads.get(self.selected_thread_index)
                                    {
                                        if thread.messages.is_empty() {
                                            ui.vertical_centered(|ui| {
                                                ui.add_space(60.0);
                                                ui.label(
                                                    RichText::new(
                                                        "Chưa có tin nhắn trong cuộc trò chuyện này.",
                                                    )
                                                    .size(14.0)
                                                    .color(pal.text_muted),
                                                );
                                            });
                                        } else {
                                            if !self.in_chat_search_open
                                                && !self.filter_starred_only
                                            {
                                                let has_more_in_memory = thread.messages.len() > render_limit;
                                                let reached_start = self.history_reached_start.contains(&thread_id);

                                                if reached_start && !has_more_in_memory {
                                                    ui.vertical_centered(|ui| {
                                                        ui.add_space(8.0);
                                                        ui.label(
                                                            RichText::new("— Đây là tin nhắn đầu tiên của cuộc trò chuyện —")
                                                                .size(11.5)
                                                                .color(pal.text_muted),
                                                        );
                                                        ui.add_space(8.0);
                                                    });
                                                } else {
                                                    ui.vertical_centered(|ui| {
                                                        let hist_btn = egui::Button::new(
                                                            RichText::new(format!(
                                                                "{} Tải thêm tin nhắn cũ",
                                                                lucide::REFRESH
                                                            ))
                                                            .size(11.5)
                                                            .color(pal.text_secondary),
                                                        )
                                                        .fill(pal.bg_elevated)
                                                        .stroke(Stroke::new(1.0_f32, pal.border))
                                                        .rounding(Rounding::same(12.0));
                                                        if ui
                                                            .add(hist_btn)
                                                            .on_hover_cursor(CursorIcon::PointingHand)
                                                            .on_hover_text(
                                                                "Đồng bộ thêm lịch sử tin nhắn cũ từ máy chủ Zalo",
                                                            )
                                                            .clicked()
                                                        {
                                                            if has_more_in_memory {
                                                                widen_window = true;
                                                                let new_count = (render_limit + CHAT_RENDER_STEP).min(thread.messages.len());
                                                                custom_toast = Some(format!("Đã hiển thị thêm tin nhắn cũ ({new_count}/{})", thread.messages.len()));
                                                            } else {
                                                                if let Some(tx) = WORKER_TX.get() {
                                                                    let _ = tx.send(
                                                                        WorkerCmd::LoadGroupHistory {
                                                                            group_id: thread_id.clone(),
                                                                            is_group,
                                                                            have: thread.messages.len(),
                                                                        },
                                                                    );
                                                                }
                                                                load_older_clicked = true;
                                                            }
                                                        }
                                                        ui.add_space(6.0);
                                                    });
                                                }
                                            }
                                            let only_starred = self.filter_starred_only;
                                            let in_chat_q = if self.in_chat_search_open {
                                                self.in_chat_search_query.trim().to_lowercase()
                                            } else {
                                                String::new()
                                            };
                                            let in_chat_folded = fold_vn_search(&in_chat_q);
                                            let visible_indices: Vec<usize> = thread
                                                .messages
                                                .iter()
                                                .enumerate()
                                                .filter(|(_, m)| {
                                                    (!only_starred || m.is_starred)
                                                        && (in_chat_q.is_empty()
                                                            || matches_vn_query(
                                                                &m.text,
                                                                &in_chat_q,
                                                                &in_chat_folded,
                                                            )
                                                            || matches_vn_query(
                                                                &m.sender,
                                                                &in_chat_q,
                                                                &in_chat_folded,
                                                            ))
                                                })
                                                .map(|(i, _)| i)
                                                .collect();
                                            // Search and the starred filter still scan everything.
                                            let visible_indices: Vec<usize> =
                                                if only_starred || !in_chat_q.is_empty() {
                                                    visible_indices
                                                } else {
                                                    let skip = visible_indices
                                                        .len()
                                                        .saturating_sub(render_limit);
                                                    visible_indices[skip..].to_vec()
                                                };
                                            if visible_indices.is_empty() && !in_chat_q.is_empty() {
                                                ui.vertical_centered(|ui| {
                                                    ui.add_space(56.0);
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "Không tìm thấy tin nhắn nào khớp từ khóa \"{}\".",
                                                            self.in_chat_search_query.trim()
                                                        ))
                                                        .size(13.5)
                                                        .color(pal.text_muted),
                                                    );
                                                });
                                            }
                                            let last_outgoing_idx = thread
                                                .messages
                                                .iter()
                                                .enumerate()
                                                .rev()
                                                .find(|(_, m)| {
                                                    m.is_outgoing && !is_system_notice_message(m)
                                                })
                                                .map(|(i, _)| i);

                                            for (pos, &idx) in visible_indices.iter().enumerate() {
                                                let msg = &thread.messages[idx];
                                                let prev_msg = pos
                                                    .checked_sub(1)
                                                    .and_then(|p| visible_indices.get(p))
                                                    .map(|&pi| &thread.messages[pi]);
                                                let next_msg = visible_indices
                                                    .get(pos + 1)
                                                    .map(|&ni| &thread.messages[ni]);

                                                if should_show_time_divider(prev_msg, msg) {
                                                    if pos > 0 {
                                                        ui.add_space(6.0);
                                                    }
                                                    render_date_divider(ui, msg, pal);
                                                    ui.add_space(6.0);
                                                }

                                                let row = ui.scope(|ui| {
                                                    if is_system_notice_message(msg) {
                                                        render_system_notice_pill(ui, msg, pal);
                                                        ui.add_space(6.0);
                                                        return;
                                                    }

                                                    let prev_same_sender = prev_msg
                                                        .map(|pm| same_message_cluster(pm, msg))
                                                        .unwrap_or(false);
                                                    let next_same_sender = next_msg
                                                        .map(|nm| same_message_cluster(msg, nm))
                                                        .unwrap_or(false);
                                                    let is_latest_outgoing =
                                                        last_outgoing_idx == Some(idx);

                                                    let sender_ava_opt = if !is_group {
                                                        thread_avatar.as_deref()
                                                    } else {
                                                        avatar_by_name.get(msg.sender.as_str()).copied()
                                                    };
                                                    if let Some(action) = render_chat_bubble(
                                                        ui,
                                                        msg,
                                                        prev_same_sender,
                                                        next_same_sender,
                                                        is_latest_outgoing,
                                                        is_group,
                                                        &thread_name,
                                                        sender_ava_opt,
                                                        pal,
                                                        &self.sticker_textures,
                                                        &mut self.loading_stickers,
                                                        &self.emoji_textures,
                                                        idx,
                                                    ) {
                                                        pending_action = Some((idx, action));
                                                    }
                                                    ui.add_space(if next_same_sender {
                                                        2.5
                                                    } else {
                                                        9.0
                                                    });
                                                });
                                                if jump_index == Some(idx) {
                                                    render_quote_target(
                                                        ui,
                                                        row.response.rect,
                                                        jump_scroll_pending,
                                                        pal.primary,
                                                    );
                                                    jump_scrolled |= jump_scroll_pending;
                                                }
                                            }
                                            let (bottom_rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 1.0), Sense::hover());
                                            if self.force_scroll_to_bottom {
                                                ui.scroll_to_rect(bottom_rect, Some(Align::BOTTOM));
                                                self.force_scroll_to_bottom = false;
                                            }
                                        }
                                    }
                                });
                        });

                    let dist_from_bottom = (chat_scroll_out.content_size.y
                        - (chat_scroll_out.state.offset.y + chat_scroll_out.inner_rect.height()))
                        .max(0.0);
                    if dist_from_bottom > 180.0 {
                        let btn_size = 36.0;
                        let btn_pos = Pos2::new(
                            chat_scroll_out.inner_rect.right() - btn_size - 22.0,
                            chat_scroll_out.inner_rect.bottom() - btn_size - 16.0,
                        );
                        egui::Area::new(Id::new(("auto_scroll_btn", &thread_id)))
                            .order(egui::Order::Middle)
                            .fixed_pos(btn_pos)
                            .show(ctx, |ui| {
                                egui::Frame::none()
                                    .fill(pal.bg_elevated)
                                    .stroke(Stroke::new(1.0_f32, pal.border))
                                    .rounding(Rounding::same(btn_size / 2.0))
                                    .shadow(egui::epaint::Shadow {
                                        offset: Vec2::new(0.0, 2.0),
                                        blur: 8.0,
                                        spread: 0.0,
                                        color: Color32::from_black_alpha(if pal.is_dark { 80 } else { 35 }),
                                    })
                                    .show(ui, |ui| {
                                        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(btn_size), Sense::click());
                                        if resp.hovered() {
                                            ui.painter().circle_filled(rect.center(), btn_size / 2.0, pal.bg_hover);
                                        }
                                        draw_lucide_icon(
                                            ui.painter(),
                                            rect.center(),
                                            lucide::CHEVRON_DOWN,
                                            16.0,
                                            if resp.hovered() { pal.primary_hover } else { pal.text_primary },
                                        );
                                        if resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .on_hover_text("Cuộn xuống tin nhắn mới nhất")
                                            .clicked()
                                        {
                                            self.force_scroll_to_bottom = true;
                                            ui.ctx().request_repaint();
                                        }
                                    });
                            });
                    }
                    if jump_scrolled {
                        if let Some(jump) = &mut self.quote_jump {
                            jump.scroll_pending = false;
                        }
                    }
                    if widen_window {
                        self.chat_render_window.1 += CHAT_RENDER_STEP;
                    }
                    if let Some(msg) = custom_toast {
                        self.show_toast(msg);
                    } else if load_older_clicked {
                        self.show_toast("Đang kiểm tra tin nhắn cũ từ máy chủ Zalo…");
                    }
                    if let Some((idx, action)) = pending_action {
                        let target = self
                            .threads
                            .get(self.selected_thread_index)
                            .and_then(|t| t.messages.get(idx))
                            .cloned();
                        if let Some(m) = target {
                            match action {
                                BubbleAction::Reply => self.replying_to = Some(m),
                                BubbleAction::JumpToQuoted(reference) => {
                                    let index = self
                                        .threads
                                        .get(self.selected_thread_index)
                                        .and_then(|thread| {
                                            crate::message_navigation::unique_reference_index(&thread.messages, &reference)
                                        });
                                    if index.is_some() {
                                        self.filter_starred_only = false;
                                        self.in_chat_search_open = false;
                                        self.in_chat_search_query.clear();
                                        self.quote_jump = Some(QuoteJump {
                                            thread_id: thread_id.clone(),
                                            reference,
                                            highlight_until: std::time::Instant::now() + std::time::Duration::from_secs(3),
                                            scroll_pending: true,
                                        });
                                        ctx.request_repaint();
                                    } else {
                                        self.show_toast("Chưa có tin nhắn gốc đã tải với ID duy nhất. Hãy tải thêm lịch sử rồi thử lại.");
                                    }
                                }
                                BubbleAction::Forward => {
                                    self.forward_msg = Some(m);
                                    self.forward_filter.clear();
                                }
                                BubbleAction::React(r) => {
                                    if let Some(real) = self
                                        .threads
                                        .get_mut(self.selected_thread_index)
                                        .and_then(|t| t.messages.get_mut(idx))
                                    {
                                        if real.reactions.contains(&r.to_string()) {
                                            real.reactions.clear();
                                            self.status_msg = "Đã bỏ cảm xúc".into();
                                        } else {
                                            real.reactions = vec![r.to_string()];
                                            self.status_msg = format!("Đã thả cảm xúc {r}");
                                        }
                                    }
                                    self.persist_threads_async();
                                    if let Some(tx) = WORKER_TX.get() {
                                        let _ = tx.send(WorkerCmd::React {
                                            to_id: thread_id.clone(),
                                            is_group,
                                            msg: m,
                                            reaction: r.to_string(),
                                        });
                                    }
                                }
                                BubbleAction::TogglePin | BubbleAction::ToggleStar => {
                                    if let Some(real) = self
                                        .threads
                                        .get_mut(self.selected_thread_index)
                                        .and_then(|t| t.messages.get_mut(idx))
                                    {
                                        if action == BubbleAction::TogglePin {
                                            real.is_pinned = !real.is_pinned;
                                            self.status_msg = if real.is_pinned {
                                                "Đã ghim tin nhắn lên đầu hội thoại".into()
                                            } else {
                                                "Đã bỏ ghim tin nhắn".into()
                                            };
                                        } else {
                                            real.is_starred = !real.is_starred;
                                            self.status_msg = if real.is_starred {
                                                "Đã đánh dấu sao tin nhắn".into()
                                            } else {
                                                "Đã bỏ đánh dấu sao".into()
                                            };
                                        }
                                    }
                                    self.persist_threads_async();
                                }
                                BubbleAction::DeleteForMe => {
                                    if let Some(tx) = WORKER_TX.get() {
                                        let _ = tx.send(WorkerCmd::DeleteForMe {
                                            to_id: thread_id.clone(),
                                            is_group,
                                            msg: m,
                                        });
                                    }
                                }
                                BubbleAction::RecallForEveryone => {
                                    if let Some(real) = self
                                        .threads
                                        .get_mut(self.selected_thread_index)
                                        .and_then(|t| t.messages.get_mut(idx))
                                    {
                                        real.text = "[Tin nhắn đã thu hồi]".into();
                                        real.sticker_id = None;
                                        real.link_url = None;
                                        real.link_thumb = None;
                                    }
                                    self.persist_threads_async();
                                    if let Some(tx) = WORKER_TX.get() {
                                        let _ = tx.send(WorkerCmd::RecallMessage {
                                            to_id: thread_id.clone(),
                                            is_group,
                                            msg: m,
                                        });
                                    }
                                }
                                BubbleAction::OpenImagePopup {
                                    tex_key,
                                    full_url,
                                    title,
                                    sender,
                                    time,
                                } => {
                                    let hd_tex_key = self.request_hd_texture(&full_url);
                                    self.image_viewer = Some(ImageViewerModal {
                                        tex_key,
                                        hd_tex_key,
                                        full_url,
                                        title,
                                        sender,
                                        time,
                                        zoom: 1.0,
                                        pan: Vec2::ZERO,
                                    });
                                }
                                BubbleAction::DownloadFile { url, file_name } => {
                                    self.show_toast(format!(
                                        "⏳ Đang tải tập tin \"{file_name}\"…"
                                    ));
                                    if let Some(tx) = WORKER_TX.get() {
                                        let _ = tx.send(WorkerCmd::DownloadFile {
                                            url,
                                            file_name,
                                            download_dir: self.settings.download_dir.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }

                    // 3. Optional Sticker Picker Drawer
                    if self.sticker_picker_open {
                        self.render_sticker_picker(ui, &thread_id, is_group, pal);
                    }

                    // 3b. Reply banner (quoted message preview above the composer)
                    if self.replying_to.is_some() {
                        let mut cancel = false;
                        if let Some(q) = &self.replying_to {
                            let who = if q.is_outgoing {
                                "chính bạn".to_string()
                            } else if !q.sender.is_empty() && q.sender != "0" {
                                q.sender.clone()
                            } else {
                                thread_name.clone()
                            };
                            egui::Frame::none()
                                .fill(pal.bg_elevated)
                                .inner_margin(Margin::symmetric(14.0, 6.0))
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(format!("Đang trả lời {who}"))
                                                    .size(12.0)
                                                    .color(pal.primary)
                                                    .strong(),
                                            );
                                            let preview = if q.sticker_id.is_some() {
                                                "[Nhãn dán]".to_string()
                                            } else {
                                                truncate_chars(&q.text, 100).into_owned()
                                            };
                                            ui.label(
                                                RichText::new(preview)
                                                    .size(12.0)
                                                    .color(pal.text_muted),
                                            );
                                        });
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button(lucide::X)
                                                    .on_hover_cursor(CursorIcon::PointingHand)
                                                    .on_hover_text("Hủy trả lời")
                                                    .clicked()
                                                {
                                                    cancel = true;
                                                }
                                            },
                                        );
                                    });
                                });
                        }
                        if cancel {
                            self.replying_to = None;
                        }
                    }
                    // 3c. Clipboard Image Preview Banner (`Ctrl+V` / `Shift+Insert` screenshot or image file paste)
                    if is_clipboard_paste_triggered(ctx) {
                        if let Some((path, w, h, col_img)) = read_clipboard_image_to_png() {
                            // Consume any text Paste event on this frame so the composer doesn't paste a file path
                            ctx.input_mut(|i| {
                                i.events.retain(|e| !matches!(e, egui::Event::Paste(_)));
                            });
                            let tex = ctx.load_texture(
                                format!("clip_preview_{}", crate::clock::unix_now()),
                                col_img,
                                TextureOptions::LINEAR,
                            );
                            self.pending_clipboard_image = Some((path, w, h, tex));
                            self.composer_focus_pending = true;
                            self.show_toast(format!(
                                "Đã dán ảnh ({w}×{h} px) từ Clipboard — nhấn Enter để gửi"
                            ));
                        }
                    }
                    if self.pending_clipboard_image.is_some() {
                        let mut clear_clip = false;
                        if ui.input(|i| i.key_pressed(Key::Escape)) {
                            clear_clip = true;
                        }
                        if let Some((ref path, w, h, ref tex)) = self.pending_clipboard_image {
                            egui::Frame::none()
                                .fill(pal.bg_elevated)
                                .inner_margin(Margin::symmetric(14.0, 6.0))
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    pal.primary.linear_multiply(0.55),
                                ))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::Image::new(tex)
                                                .fit_to_exact_size(Vec2::new(48.0, 48.0))
                                                .rounding(Rounding::same(6.0)),
                                        );
                                        ui.add_space(6.0);
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} Ảnh chụp từ bộ nhớ tạm (Ctrl+V)",
                                                    lucide::IMAGE
                                                ))
                                                .size(12.2)
                                                .color(pal.primary)
                                                .strong(),
                                            );
                                            let fname = path
                                                .file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("clipboard.png");
                                            ui.label(
                                                RichText::new(format!(
                                                    "{fname} · {w}×{h} px — Nhập chú thích bên dưới rồi nhấn Enter để gửi"
                                                ))
                                                .size(11.3)
                                                .color(pal.text_secondary),
                                            );
                                        });
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button(format!("{} Hủy", lucide::X))
                                                    .on_hover_cursor(CursorIcon::PointingHand)
                                                    .on_hover_text("Hủy ảnh dán (Esc)")
                                                    .clicked()
                                                {
                                                    clear_clip = true;
                                                }
                                            },
                                        );
                                    });
                                });
                        }
                        if clear_clip {
                            self.pending_clipboard_image = None;
                        }
                    }


                    // 4. Bottom Message Composer Bar (extracted to composer.rs)
                    self.render_composer_bar(ui, ctx, pal, &thread_id, &thread_name, is_group);
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(120.0);
                        ui.label(
                            RichText::new("Chọn một hội thoại bên trái để bắt đầu trò chuyện")
                                .size(15.0)
                                .color(pal.text_muted),
                        );
                    });
                }
            });
    }
}

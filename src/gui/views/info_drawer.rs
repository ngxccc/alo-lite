use crate::gui::messages::is_sticker_message;
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_info_drawer(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
        drawer_w: f32,
        thread_meta: &Option<(String, String, Option<String>, bool, usize)>,
    ) {
        if let Some((d_id, d_name, d_ava, d_is_group, d_msg_count)) = thread_meta.clone() {
            let shared_photos: Vec<(String, String, String, String)> = self
                .threads
                .get(self.selected_thread_index)
                .map(|t| {
                    t.messages
                        .iter()
                        .rev()
                        .filter_map(|m| {
                            let is_photo = m.link_src.as_deref() == Some("Hình ảnh Zalo")
                                || m.text.trim() == "[Hình ảnh]"
                                || m.text.trim().starts_with("[Hình ảnh] ");
                            if is_sticker_message(m).is_some() {
                                return None;
                            }
                            if !is_photo {
                                return None;
                            }
                            let thumb = m
                                .link_thumb
                                .as_deref()
                                .or(m.link_url.as_deref())
                                .filter(|u| !u.is_empty())?;
                            let full = m.link_url.as_deref().unwrap_or(thumb).to_string();
                            let title = m
                                .link_title
                                .clone()
                                .unwrap_or_else(|| "Hình ảnh đã chia sẻ".into());
                            Some((
                                format!("thumb_{}", crate::crypto::md5_hex(thumb)),
                                thumb.to_string(),
                                full,
                                title,
                            ))
                        })
                        .take(9)
                        .collect()
                })
                .unwrap_or_default();

            egui::SidePanel::right("conversation_info_drawer")
                .resizable(false)
                .exact_width(drawer_w)
                .frame(
                    egui::Frame::none()
                        .fill(pal.bg_sidebar)
                        .stroke(Stroke::new(1.0_f32, pal.border)),
                )
                .show(ctx, |ui| {
                    ui.set_max_width(drawer_w);
                    egui::Frame::none()
                        .inner_margin(Margin::symmetric(16.0, 18.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                let (av_rect, _) =
                                    ui.allocate_exact_size(Vec2::new(64.0, 64.0), Sense::hover());
                                paint_user_avatar(
                                    ui.painter(),
                                    av_rect.center(),
                                    32.0,
                                    &d_name,
                                    d_is_group,
                                    d_ava.as_deref(),
                                    &self.sticker_textures,
                                    &mut self.loading_stickers,
                                );
                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new(&d_name)
                                        .size(14.5)
                                        .color(pal.text_primary)
                                        .strong(),
                                );
                                ui.add_space(2.0);
                                let d_is_cloud = d_name.contains("Cloud của tôi");
                                let d_fr_status = if d_is_group || d_is_cloud {
                                    crate::protocol::social::FriendshipStatus::Friend
                                } else {
                                    self.friend_status_map
                                        .get(&d_id)
                                        .copied()
                                        .unwrap_or(crate::protocol::social::FriendshipStatus::Unknown)
                                };
                                let d_onl = self.user_online_map.get(&d_id);
                                let (sub_txt, sub_col): (String, Color32) = if d_is_group {
                                    ("Nhóm đang hoạt động".into(), pal.accent_online)
                                } else if d_is_cloud {
                                    ("Lưu trữ đám mây cá nhân".into(), pal.primary_hover)
                                } else {
                                    match d_fr_status {
                                        crate::protocol::social::FriendshipStatus::Friend => match d_onl {
                                            Some((true, lbl)) if !lbl.is_empty() => {
                                                (format!("✓ Đã kết bạn · {lbl}"), pal.accent_online)
                                            }
                                            Some((false, lbl)) if !lbl.is_empty() => (
                                                format!("✓ Đã kết bạn · {lbl}"),
                                                pal.text_secondary,
                                            ),
                                            _ => ("✓ Đã kết bạn".into(), pal.text_secondary),
                                        },
                                        crate::protocol::social::FriendshipStatus::IncomingPending => (
                                            "📩 Đã gửi lời mời kết bạn cho bạn".into(),
                                            pal.primary_hover,
                                        ),
                                        crate::protocol::social::FriendshipStatus::OutgoingPending => {
                                            ("⏳ Đã gửi lời mời kết bạn".into(), pal.warning)
                                        }
                                        crate::protocol::social::FriendshipStatus::Stranger => (
                                            "Người lạ · Chưa kết bạn".into(),
                                            Color32::from_rgb(249, 115, 22),
                                        ),
                                        crate::protocol::social::FriendshipStatus::Unknown => {
                                            ("Đang kiểm tra kết bạn…".into(), pal.text_muted)
                                        }
                                    }
                                };
                                ui.label(RichText::new(sub_txt).size(12.0).color(sub_col));

                                ui.add_space(14.0);
                                // Quick Action Buttons (Sync & Pin/Star Filter)
                                ui.horizontal(|ui| {
                                    ui.add_space(34.0);
                                    let sync_btn = egui::Button::new(
                                        RichText::new("Đồng bộ")
                                            .size(11.5)
                                            .color(pal.text_secondary),
                                    )
                                    .fill(pal.bg_elevated)
                                    .stroke(Stroke::new(1.0_f32, pal.border))
                                    .rounding(Rounding::same(8.0))
                                    .min_size(Vec2::new(92.0, 30.0));
                                    if ui
                                        .add(sync_btn)
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        if let Some(tx) = WORKER_TX.get() {
                                            let _ = tx.send(WorkerCmd::RefreshThreads);
                                            let _ = tx.send(WorkerCmd::LoadGroupHistory {
                                                group_id: d_id.clone(),
                                                is_group: d_is_group,
                                                have: d_msg_count,
                                            });
                                        }
                                        self.show_toast("Đang đồng bộ lịch sử hội thoại…");
                                    }

                                    let star_active = self.filter_starred_only;
                                    let star_btn = egui::Button::new(
                                        RichText::new(if star_active {
                                            "Hiện tất cả"
                                        } else {
                                            "Tin đánh dấu"
                                        })
                                        .size(11.5)
                                        .color(
                                            if star_active {
                                                Color32::WHITE
                                            } else {
                                                pal.text_secondary
                                            },
                                        ),
                                    )
                                    .fill(if star_active {
                                        pal.primary
                                    } else {
                                        pal.bg_elevated
                                    })
                                    .stroke(Stroke::new(1.0_f32, pal.border))
                                    .rounding(Rounding::same(8.0))
                                    .min_size(Vec2::new(92.0, 30.0));
                                    if ui
                                        .add(star_btn)
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        self.filter_starred_only = !self.filter_starred_only;
                                    }
                                });
                            });
                        });

                    ui.add(egui::Separator::default().spacing(1.0));

                    // Shared Media Grid (`Ảnh / Video đã chia sẻ`)
                    egui::Frame::none()
                        .inner_margin(Margin::symmetric(16.0, 14.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Ảnh / Video đã chia sẻ")
                                        .size(12.5)
                                        .color(pal.text_primary)
                                        .strong(),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("({})", shared_photos.len()))
                                            .size(11.5)
                                            .color(pal.primary_hover),
                                    );
                                });
                            });
                            ui.add_space(8.0);

                            if shared_photos.is_empty() {
                                ui.label(
                                    RichText::new("Chưa có ảnh hoặc video chia sẻ.")
                                        .size(11.5)
                                        .color(pal.text_muted),
                                );
                            } else {
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                                    for (tkey, thumb_u, full_u, ptitle) in &shared_photos {
                                        if !self.sticker_textures.contains_key(tkey)
                                            && self.loading_stickers.insert(tkey.clone())
                                        {
                                            if let Some(tx) = WORKER_TX.get() {
                                                let _ = tx.send(WorkerCmd::FetchStickerImage {
                                                    key: tkey.clone(),
                                                    url: thumb_u.clone(),
                                                });
                                            }
                                        }
                                        let (cell_r, cell_resp) = ui
                                            .allocate_exact_size(Vec2::splat(74.0), Sense::click());
                                        ui.painter().rect_filled(
                                            cell_r,
                                            Rounding::same(8.0),
                                            pal.bg_elevated,
                                        );
                                        if let Some(tex) = self.sticker_textures.get(tkey) {
                                            ui.painter().image(
                                                tex.id(),
                                                cell_r,
                                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                                Color32::WHITE,
                                            );
                                        } else {
                                            draw_icon_image(
                                                ui.painter(),
                                                cell_r.center(),
                                                20.0,
                                                pal.text_muted,
                                            );
                                        }
                                        ui.painter().rect_stroke(
                                            cell_r,
                                            Rounding::same(8.0),
                                            Stroke::new(
                                                1.0_f32,
                                                if cell_resp.hovered() {
                                                    pal.primary
                                                } else {
                                                    pal.border
                                                },
                                            ),
                                        );
                                        if cell_resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .on_hover_text("Xem ảnh phóng to")
                                            .clicked()
                                        {
                                            self.image_viewer = Some(ImageViewerModal {
                                                tex_key: tkey.clone(),
                                                hd_tex_key: self.request_hd_texture(full_u),
                                                full_url: full_u.clone(),
                                                title: ptitle.clone(),
                                                sender: d_name.clone(),
                                                time: String::new(),
                                                zoom: 1.0,
                                                pan: Vec2::ZERO,
                                            });
                                        }
                                    }
                                });
                            }
                        });

                    // Push technical metadata to the bottom of the drawer
                    let rem = (ui.available_height() - 88.0).max(8.0);
                    ui.add_space(rem);
                    ui.add(egui::Separator::default().spacing(1.0));

                    egui::Frame::none()
                        .fill(pal.bg_app)
                        .inner_margin(Margin::symmetric(16.0, 12.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Tổng số tin nhắn:")
                                        .size(11.5)
                                        .color(pal.text_muted),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{d_msg_count} tin nhắn"))
                                            .size(11.5)
                                            .color(pal.text_secondary)
                                            .strong(),
                                    );
                                });
                            });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Mã định danh (ID):")
                                        .size(11.5)
                                        .color(pal.text_muted),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let char_cnt = d_id.chars().count();
                                    let short_id = if char_cnt > 10 {
                                        let prefix: String = d_id.chars().take(4).collect();
                                        let suffix: String =
                                            d_id.chars().skip(char_cnt - 4).collect();
                                        format!("{prefix}...{suffix}")
                                    } else {
                                        d_id.clone()
                                    };
                                    let (id_rect, id_resp) = ui
                                        .allocate_exact_size(Vec2::new(98.0, 22.0), Sense::click());
                                    ui.painter().rect_filled(
                                        id_rect,
                                        Rounding::same(5.0),
                                        if id_resp.hovered() {
                                            pal.bg_hover
                                        } else {
                                            pal.bg_elevated
                                        },
                                    );
                                    ui.painter().text(
                                        Pos2::new(id_rect.left() + 7.0, id_rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        short_id,
                                        FontId::monospace(10.5),
                                        pal.text_secondary,
                                    );
                                    draw_icon_copy(
                                        ui.painter(),
                                        Pos2::new(id_rect.right() - 10.0, id_rect.center().y),
                                        11.5,
                                        pal.text_secondary,
                                    );
                                    if id_resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text(format!("Sao chép ID: {d_id}"))
                                        .clicked()
                                    {
                                        ui.ctx().copy_text(d_id.clone());
                                        self.show_toast("Đã sao chép ID hội thoại vào bộ nhớ tạm");
                                    }
                                });
                            });
                        });
                });
        }
    }
}

use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn trigger_feed_timeline_load(&mut self) {
        if self.feed_loading_timeline {
            return;
        }
        self.feed_loading_timeline = true;
        let mut friend_uids = Vec::new();
        let mut shared_media_by_uid: HashMap<String, Vec<crate::feed::FeedMedia>> = HashMap::new();
        for t in &self.threads {
            if !t.is_group && !t.name.contains("Cloud của tôi") {
                friend_uids.push(t.id.clone());
                let mut photos = Vec::new();
                for m in t.messages.iter().rev() {
                    let is_photo = m.link_src.as_deref() == Some("Hình ảnh Zalo")
                        || m.text.trim() == "[Hình ảnh]"
                        || m.text.trim().starts_with("[Hình ảnh] ");
                    if !is_photo {
                        continue;
                    }
                    if let Some(thumb) = m
                        .link_thumb
                        .as_deref()
                        .or(m.link_url.as_deref())
                        .filter(|u| !u.is_empty())
                    {
                        let full = m.link_url.as_deref().unwrap_or(thumb).to_string();
                        photos.push(crate::feed::FeedMedia {
                            photo_id: m.id.clone(),
                            media_type: "image".to_string(),
                            url: full,
                            thumbnail: thumb.to_string(),
                            width: 640,
                            height: 480,
                        });
                        if photos.len() >= 6 {
                            break;
                        }
                    }
                }
                if !photos.is_empty() {
                    shared_media_by_uid.insert(t.id.clone(), photos);
                }
            }
        }
        if let Some(tx) = WORKER_TX.get() {
            let _ = tx.send(WorkerCmd::FetchTimelineFeed {
                friend_uids,
                shared_media_by_uid,
            });
        }
    }

    pub(in crate::gui) fn render_feed_page(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        // Left Friends / Filter Sidebar (292px)
        egui::SidePanel::left("feed_friends_sidebar")
            .resizable(false)
            .exact_width(292.0)
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_sidebar)
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .inner_margin(Margin::symmetric(14.0, 14.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Tường nhà")
                            .size(17.5)
                            .color(pal.text_primary)
                            .strong(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let refresh_btn = egui::Button::new(
                            RichText::new(format!("{} Làm mới", lucide::REFRESH))
                                .size(12.0)
                                .color(pal.text_primary),
                        )
                        .fill(pal.bg_elevated)
                        .stroke(Stroke::new(1.0_f32, pal.border))
                        .rounding(Rounding::same(8.0))
                        .min_size(Vec2::new(82.0, 28.0));
                        if ui
                            .add_enabled(!self.feed_loading_timeline, refresh_btn)
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.trigger_feed_timeline_load();
                        }
                    });
                });
                ui.add_space(10.0);

                // Search bar (matching Chat / Contacts sidebar search box)
                let has_q = !self.feed_search_query.is_empty();
                let (sbox_rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::hover());
                ui.painter()
                    .rect_filled(sbox_rect, Rounding::same(8.0), pal.bg_input);
                ui.painter().rect_stroke(
                    sbox_rect,
                    Rounding::same(8.0),
                    Stroke::new(
                        1.0_f32,
                        if has_q {
                            pal.primary.linear_multiply(0.65)
                        } else {
                            pal.border
                        },
                    ),
                );
                draw_icon_search(
                    ui.painter(),
                    Pos2::new(sbox_rect.left() + 15.0, sbox_rect.center().y),
                    13.0,
                    if has_q {
                        pal.primary_hover
                    } else {
                        pal.text_muted
                    },
                );
                let edit_rect = Rect::from_min_max(
                    Pos2::new(sbox_rect.left() + 31.0, sbox_rect.top() + 5.0),
                    Pos2::new(sbox_rect.right() - 8.0, sbox_rect.bottom() - 5.0),
                );
                ui.put(
                    edit_rect,
                    egui::TextEdit::singleline(&mut self.feed_search_query)
                        .id(Id::new("feed_sidebar_search"))
                        .frame(false)
                        .hint_text("Tìm kiếm bạn bè trên Nhật ký…")
                        .desired_width(edit_rect.width()),
                );
                ui.add_space(10.0);

                // All Timeline button
                let all_selected = self.feed_target_uid.is_empty();
                let all_btn = egui::Button::new(
                    RichText::new(format!("{}  Nhật ký tất cả mọi người", lucide::GLOBE))
                        .size(12.8)
                        .color(if all_selected {
                            Color32::WHITE
                        } else {
                            pal.text_primary
                        })
                        .strong(),
                )
                .fill(if all_selected {
                    pal.primary
                } else {
                    pal.bg_elevated
                })
                .stroke(if all_selected {
                    Stroke::NONE
                } else {
                    Stroke::new(1.0_f32, pal.border)
                })
                .rounding(Rounding::same(8.0))
                .min_size(Vec2::new(ui.available_width(), 34.0));
                if ui
                    .add(all_btn)
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .clicked()
                {
                    self.feed_target_uid.clear();
                    self.feed_profile = None;
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "BẠN BÈ TRÊN TƯỜNG NHÀ ({})",
                        self.feed_friend_profiles.len()
                    ))
                    .size(11.0)
                    .color(pal.text_muted)
                    .strong(),
                );
                ui.add_space(6.0);

                let q_low = self.feed_search_query.trim().to_lowercase();
                let q_fold = fold_vn_search(&q_low);
                let profiles_snap = self.feed_friend_profiles.clone();
                egui::ScrollArea::vertical()
                    .id_salt("feed_friends_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 3.0;
                        for prof in &profiles_snap {
                            if !matches_vn_query(&prof.display_name, &q_low, &q_fold)
                                && !matches_vn_query(&prof.status, &q_low, &q_fold)
                            {
                                continue;
                            }
                            let is_sel = self.feed_target_uid == prof.uid;
                            let (row_rect, row_resp) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 52.0),
                                Sense::click(),
                            );
                            if is_sel {
                                let active_bg = if pal.is_dark {
                                    Color32::from_rgb(28, 56, 98)
                                } else {
                                    pal.primary.linear_multiply(0.16)
                                };
                                ui.painter()
                                    .rect_filled(row_rect, Rounding::same(8.0), active_bg);
                                ui.painter().rect_stroke(
                                    row_rect,
                                    Rounding::same(8.0),
                                    Stroke::new(1.0_f32, pal.primary.linear_multiply(0.55)),
                                );
                            } else if row_resp.hovered() {
                                ui.painter().rect_filled(
                                    row_rect,
                                    Rounding::same(8.0),
                                    pal.bg_hover,
                                );
                            }
                            let av_c = Pos2::new(row_rect.left() + 26.0, row_rect.center().y);
                            paint_user_avatar(
                                ui.painter(),
                                av_c,
                                18.0,
                                &prof.display_name,
                                false,
                                Some(&prof.avatar_url),
                                &self.sticker_textures,
                                &mut self.loading_stickers,
                            );
                            ui.painter().text(
                                Pos2::new(row_rect.left() + 52.0, row_rect.top() + 17.0),
                                egui::Align2::LEFT_CENTER,
                                truncate_chars(&prof.display_name, 22),
                                FontId::proportional(13.2),
                                if is_sel && pal.is_dark {
                                    Color32::WHITE
                                } else {
                                    pal.text_primary
                                },
                            );
                            let sub_raw = prof.status.replace('\n', " ");
                            let sub_trim = sub_raw.trim();
                            let sub_line = if !sub_trim.is_empty() {
                                truncate_chars(sub_trim, 24).into_owned()
                            } else if !prof.sdob.is_empty() {
                                format!("Sinh nhật: {}", prof.sdob)
                            } else {
                                "Xem tường cá nhân".to_string()
                            };
                            ui.painter().text(
                                Pos2::new(row_rect.left() + 52.0, row_rect.top() + 36.0),
                                egui::Align2::LEFT_CENTER,
                                sub_line,
                                FontId::proportional(11.3),
                                if is_sel && pal.is_dark {
                                    Color32::from_rgb(190, 212, 248)
                                } else {
                                    pal.text_secondary
                                },
                            );
                            if row_resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                self.feed_target_uid = prof.uid.clone();
                                self.feed_profile = Some(prof.clone());
                            }
                        }
                    });
            });

        // Main Central Panel — Social Timeline Stream
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(pal.bg_chat_canvas))
            .show(ctx, |ui| {
                // Full-width Top Header Bar (54px)
                let (hdr_rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 54.0), Sense::hover());
                ui.painter()
                    .rect_filled(hdr_rect, Rounding::ZERO, pal.bg_sidebar);
                ui.painter().line_segment(
                    [
                        Pos2::new(hdr_rect.left(), hdr_rect.bottom()),
                        Pos2::new(hdr_rect.right(), hdr_rect.bottom()),
                    ],
                    Stroke::new(1.0_f32, pal.border),
                );
                let tab_inner = Rect::from_min_max(
                    Pos2::new(hdr_rect.left() + 24.0, hdr_rect.top() + 10.0),
                    Pos2::new(hdr_rect.right() - 24.0, hdr_rect.bottom() - 10.0),
                );
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(tab_inner), |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        for (tab, icon, title) in [
                            (FeedModalTab::Profile, lucide::GLOBE, "Nhật Ký"),
                            (FeedModalTab::Avatars, lucide::IMAGE, "Kho Ảnh Cá Nhân"),
                            (
                                FeedModalTab::Packet50001,
                                lucide::FILE_TEXT,
                                "Giao thức Feed (Cmd 50001)",
                            ),
                        ] {
                            let active = self.feed_modal_tab == tab;
                            let btn = egui::Button::new(
                                RichText::new(format!("{icon}  {title}"))
                                    .size(13.2)
                                    .color(if active {
                                        Color32::WHITE
                                    } else {
                                        pal.text_secondary
                                    })
                                    .strong(),
                            )
                            .fill(if active { pal.primary } else { pal.bg_elevated })
                            .stroke(if active {
                                Stroke::NONE
                            } else {
                                Stroke::new(1.0_f32, pal.border)
                            })
                            .rounding(Rounding::same(8.0))
                            .min_size(Vec2::new(145.0, 34.0));
                            if ui
                                .add(btn)
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.feed_modal_tab = tab;
                                if tab == FeedModalTab::Avatars
                                    && self.feed_avatars.is_empty()
                                    && !self.feed_loading_avatars
                                {
                                    self.feed_loading_avatars = true;
                                    if let Some(tx) = WORKER_TX.get() {
                                        let _ = tx.send(WorkerCmd::FetchMySocialAvatars {
                                            page: 1,
                                            count: 20,
                                        });
                                    }
                                }
                            }
                        }
                    });
                });

                egui::ScrollArea::vertical()
                    .id_salt("main_social_feed_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let avail = ui.available_width();
                        let feed_col_w = (avail - 48.0).clamp(420.0, 680.0);
                        let side_pad = ((avail - feed_col_w) * 0.5).max(16.0);
                        ui.add_space(18.0);
                        ui.horizontal(|ui| {
                            ui.add_space(side_pad);
                            ui.vertical(|ui| {
                                ui.set_width(feed_col_w);
                                match self.feed_modal_tab {
                                    FeedModalTab::Profile => {
                                        self.render_feed_timeline_column(ui, pal, feed_col_w);
                                    }
                                    FeedModalTab::Avatars => {
                                        self.render_feed_avatars_column(ui, pal, feed_col_w);
                                    }
                                    FeedModalTab::Packet50001 => {
                                        self.render_feed_packet_column(ui, pal, feed_col_w);
                                    }
                                }
                                ui.add_space(32.0);
                            });
                        });
                    });
            });
    }

    fn render_feed_timeline_column(&mut self, ui: &mut egui::Ui, pal: &ThemePalette, col_w: f32) {
        // 1. Selected Friend Header Banner (if filtering by a specific friend)
        if let Some(prof) = self.feed_profile.clone() {
            egui::Frame::none()
                .fill(pal.bg_elevated)
                .stroke(Stroke::new(1.0_f32, pal.border))
                .rounding(Rounding::same(14.0))
                .inner_margin(Margin::same(18.0))
                .show(ui, |ui| {
                    ui.set_width(col_w - 36.0);
                    if !prof.cover_url.is_empty() && !prof.cover_url.ends_with("/default") {
                        let ckey = format!("cov_{}", crate::crypto::md5_hex(&prof.cover_url));
                        if !self.sticker_textures.contains_key(&ckey)
                            && self.loading_stickers.insert(ckey.clone())
                        {
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::FetchStickerImage {
                                    key: ckey.clone(),
                                    url: prof.cover_url.clone(),
                                });
                            }
                        }
                        if let Some(tex) = self.sticker_textures.get(&ckey) {
                            let w = ui.available_width();
                            let resp = ui.add(
                                egui::Image::new((tex.id(), Vec2::new(w, 180.0)))
                                    .rounding(Rounding::same(10.0))
                                    .sense(Sense::click()),
                            );
                            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                let hd = self.request_hd_texture(&prof.cover_url);
                                self.image_viewer = Some(ImageViewerModal {
                                    tex_key: ckey,
                                    hd_tex_key: hd,
                                    full_url: prof.cover_url.clone(),
                                    title: format!("Ảnh bìa của {}", prof.display_name),
                                    sender: prof.display_name.clone(),
                                    time: String::new(),
                                    zoom: 1.0,
                                    pan: Vec2::ZERO,
                                });
                            }
                            ui.add_space(12.0);
                        }
                    }
                    ui.horizontal(|ui| {
                        let (av_r, _) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::hover());
                        paint_user_avatar(
                            ui.painter(),
                            av_r.center(),
                            28.0,
                            &prof.display_name,
                            false,
                            Some(&prof.avatar_url),
                            &self.sticker_textures,
                            &mut self.loading_stickers,
                        );
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&prof.display_name)
                                    .size(16.5)
                                    .color(pal.text_primary)
                                    .strong(),
                            );
                            if !prof.status.is_empty() {
                                ui.label(
                                    RichText::new(&prof.status)
                                        .size(13.0)
                                        .color(pal.text_secondary),
                                );
                            }
                            ui.label(
                                RichText::new(format!(
                                    "UID: {}  ·  Ngày sinh: {}",
                                    prof.uid,
                                    if prof.sdob.is_empty() {
                                        "Ẩn"
                                    } else {
                                        &prof.sdob
                                    }
                                ))
                                .size(11.8)
                                .color(pal.text_muted),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let back_btn = egui::Button::new(
                                RichText::new("Xem tất cả mọi người")
                                    .size(12.2)
                                    .color(pal.text_primary),
                            )
                            .fill(pal.bg_input)
                            .stroke(Stroke::new(1.0_f32, pal.border))
                            .rounding(Rounding::same(8.0))
                            .min_size(Vec2::new(140.0, 32.0));
                            if ui
                                .add(back_btn)
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.feed_target_uid.clear();
                                self.feed_profile = None;
                            }
                        });
                    });
                });
            ui.add_space(14.0);
        }

        // 2. Info Banner: Nguồn dữ liệu Nhật ký đồng bộ
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(12.0))
            .inner_margin(Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.set_width(col_w - 32.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{}  Dòng thời gian Nhật ký ({} bài đăng)",
                            lucide::GLOBE,
                            self.feed_items.len()
                        ))
                        .size(13.2)
                        .color(pal.text_primary)
                        .strong(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let btn = egui::Button::new(
                            RichText::new(format!("{}  Kho ảnh cá nhân", lucide::IMAGE))
                                .size(12.0)
                                .color(pal.text_primary),
                        )
                        .fill(pal.bg_input)
                        .stroke(Stroke::new(1.0_f32, pal.border))
                        .rounding(Rounding::same(8.0))
                        .min_size(Vec2::new(130.0, 28.0));
                        if ui
                            .add(btn)
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.feed_modal_tab = FeedModalTab::Avatars;
                            if self.feed_avatars.is_empty() && !self.feed_loading_avatars {
                                self.feed_loading_avatars = true;
                                if let Some(tx) = WORKER_TX.get() {
                                    let _ = tx.send(WorkerCmd::FetchMySocialAvatars {
                                        page: 1,
                                        count: 20,
                                    });
                                }
                            }
                        }
                    });
                });
            });

        ui.add_space(14.0);

        if self.feed_loading_timeline && self.feed_items.is_empty() {
            ui.label(
                RichText::new("Đang đồng bộ Nhật ký & bài đăng của bạn bè…")
                    .size(13.5)
                    .color(pal.text_secondary),
            );
            return;
        }

        let q_low = self.feed_search_query.trim().to_lowercase();
        let q_fold = fold_vn_search(&q_low);
        let target_filter = self.feed_target_uid.clone();
        let mut clicked_image: Option<(String, String, String, String)> = None;
        let mut select_uid: Option<String> = None;

        let items_len = self.feed_items.len();
        let mut rendered_count = 0;
        for idx in 0..items_len {
            let item = self.feed_items[idx].clone();
            if !target_filter.is_empty() && item.author.uid != target_filter {
                let matches_name = self
                    .feed_profile
                    .as_ref()
                    .map(|p| {
                        p.display_name
                            .trim()
                            .eq_ignore_ascii_case(item.author.display_name.trim())
                            || p.zalo_name
                                .trim()
                                .eq_ignore_ascii_case(item.author.display_name.trim())
                            || item
                                .author
                                .display_name
                                .trim()
                                .eq_ignore_ascii_case(p.display_name.trim())
                    })
                    .unwrap_or(false);
                if !matches_name {
                    continue;
                }
            }
            if !matches_vn_query(&item.author.display_name, &q_low, &q_fold)
                && !matches_vn_query(&item.content, &q_low, &q_fold)
            {
                continue;
            }
            rendered_count += 1;

            egui::Frame::none()
                .fill(pal.bg_elevated)
                .stroke(Stroke::new(1.0_f32, pal.border))
                .rounding(Rounding::same(14.0))
                .inner_margin(Margin::same(18.0))
                .show(ui, |ui| {
                    ui.set_width(col_w - 36.0);

                    // Author Row
                    ui.horizontal(|ui| {
                        let (av_r, av_resp) =
                            ui.allocate_exact_size(Vec2::splat(42.0), Sense::click());
                        paint_user_avatar(
                            ui.painter(),
                            av_r.center(),
                            21.0,
                            &item.author.display_name,
                            false,
                            Some(&item.author.avatar_url),
                            &self.sticker_textures,
                            &mut self.loading_stickers,
                        );
                        if av_resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                            select_uid = Some(item.author.uid.clone());
                        }
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                let name_resp = ui.add(
                                    egui::Label::new(
                                        RichText::new(&item.author.display_name)
                                            .size(14.5)
                                            .color(pal.primary_hover)
                                            .strong(),
                                    )
                                    .sense(Sense::click()),
                                );
                                if name_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    select_uid = Some(item.author.uid.clone());
                                }
                                if let Some(badge) = &item.status_badge {
                                    ui.label(
                                        RichText::new(format!("· {badge}"))
                                            .size(11.8)
                                            .color(pal.text_muted),
                                    );
                                }
                            });
                            ui.label(
                                RichText::new(&item.date_str)
                                    .size(11.8)
                                    .color(pal.text_muted),
                            );
                        });
                    });

                    if !item.content.trim().is_empty() {
                        ui.add_space(12.0);
                        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                            ui.label(
                                RichText::new(&item.content)
                                    .size(14.2)
                                    .color(pal.text_primary),
                            );
                        });
                    }

                    // Media Collage Grid
                    if !item.media_list.is_empty() {
                        ui.add_space(12.0);
                        let media_cnt = item.media_list.len();
                        let avail_w = ui.available_width();
                        if media_cnt == 1 {
                            let m = &item.media_list[0];
                            let mkey = format!("fmed_{}", crate::crypto::md5_hex(&m.thumbnail));
                            if !self.sticker_textures.contains_key(&mkey)
                                && self.loading_stickers.insert(mkey.clone())
                            {
                                if let Some(tx) = WORKER_TX.get() {
                                    let _ = tx.send(WorkerCmd::FetchStickerImage {
                                        key: mkey.clone(),
                                        url: m.thumbnail.clone(),
                                    });
                                }
                            }
                            let img_h = if m.width > 0 && m.height > 0 {
                                (avail_w * (m.height as f32 / m.width as f32)).clamp(200.0, 420.0)
                            } else {
                                280.0
                            };
                            let (r, resp) =
                                ui.allocate_exact_size(Vec2::new(avail_w, img_h), Sense::click());
                            if let Some(tex) = self.sticker_textures.get(&mkey) {
                                ui.painter().image(
                                    tex.id(),
                                    r,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            } else {
                                ui.painter()
                                    .rect_filled(r, Rounding::same(10.0), pal.bg_input);
                            }
                            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                clicked_image = Some((
                                    mkey,
                                    m.url.clone(),
                                    item.content.clone(),
                                    item.author.display_name.clone(),
                                ));
                            }
                        } else {
                            let cell_w = ((avail_w - 12.0) / 3.0).max(96.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                                for m in item.media_list.iter().take(6) {
                                    let mkey =
                                        format!("fmed_{}", crate::crypto::md5_hex(&m.thumbnail));
                                    if !self.sticker_textures.contains_key(&mkey)
                                        && self.loading_stickers.insert(mkey.clone())
                                    {
                                        if let Some(tx) = WORKER_TX.get() {
                                            let _ = tx.send(WorkerCmd::FetchStickerImage {
                                                key: mkey.clone(),
                                                url: m.thumbnail.clone(),
                                            });
                                        }
                                    }
                                    let (r, resp) = ui.allocate_exact_size(
                                        Vec2::new(cell_w, cell_w),
                                        Sense::click(),
                                    );
                                    if let Some(tex) = self.sticker_textures.get(&mkey) {
                                        ui.painter().image(
                                            tex.id(),
                                            r,
                                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                            Color32::WHITE,
                                        );
                                    } else {
                                        ui.painter().rect_filled(
                                            r,
                                            Rounding::same(8.0),
                                            pal.bg_input,
                                        );
                                    }
                                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                        clicked_image = Some((
                                            mkey,
                                            m.url.clone(),
                                            item.content.clone(),
                                            item.author.display_name.clone(),
                                        ));
                                    }
                                }
                            });
                        }
                    }

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Reaction & Comment Footer Pills (clean vector glyphs, no tofu boxes)
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        egui::Frame::none()
                            .fill(pal.bg_input)
                            .stroke(Stroke::new(1.0_f32, pal.border))
                            .rounding(Rounding::same(16.0))
                            .inner_margin(Margin::symmetric(12.0, 5.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 5.0;
                                    ui.label(
                                        RichText::new("♥")
                                            .size(13.5)
                                            .color(Color32::from_rgb(239, 68, 68)),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} lượt thích",
                                            item.reaction.like_count
                                        ))
                                        .size(12.2)
                                        .color(pal.text_secondary)
                                        .strong(),
                                    );
                                });
                            });

                        egui::Frame::none()
                            .fill(pal.bg_input)
                            .stroke(Stroke::new(1.0_f32, pal.border))
                            .rounding(Rounding::same(16.0))
                            .inner_margin(Margin::symmetric(12.0, 5.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.label(
                                        RichText::new(lucide::MESSAGE_SQUARE)
                                            .size(12.5)
                                            .color(pal.primary_hover),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} bình luận",
                                            item.reaction.comment_count
                                        ))
                                        .size(12.2)
                                        .color(pal.text_secondary)
                                        .strong(),
                                    );
                                });
                            });
                    });
                });

            ui.add_space(14.0);
        }

        if rendered_count == 0 && !self.feed_items.is_empty() {
            ui.add_space(28.0);
            egui::Frame::none()
                .fill(pal.bg_elevated)
                .stroke(Stroke::new(1.0_f32, pal.border))
                .rounding(Rounding::same(14.0))
                .inner_margin(Margin::symmetric(24.0, 28.0))
                .show(ui, |ui| {
                    ui.set_width(col_w - 48.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("Người dùng này chưa có bài viết nào trên Nhật ký")
                                .size(14.5)
                                .color(pal.text_secondary)
                                .strong(),
                        );
                        ui.add_space(10.0);
                        if !target_filter.is_empty() {
                            let all_b = egui::Button::new(
                                RichText::new("Xem Nhật ký tất cả mọi người")
                                    .size(12.5)
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(pal.primary)
                            .rounding(Rounding::same(8.0))
                            .min_size(Vec2::new(200.0, 34.0));
                            if ui
                                .add(all_b)
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.feed_target_uid.clear();
                                self.feed_profile = None;
                            }
                        }
                    });
                });
        }

        if let Some(uid) = select_uid {
            self.feed_target_uid = uid.clone();
            if let Some(p) = self
                .feed_friend_profiles
                .iter()
                .find(|p| p.uid == uid)
                .cloned()
            {
                self.feed_profile = Some(p);
            }
        }
        if let Some((tkey, full_u, title, sender)) = clicked_image {
            let hd = self.request_hd_texture(&full_u);
            self.image_viewer = Some(ImageViewerModal {
                tex_key: tkey,
                hd_tex_key: hd,
                full_url: full_u,
                title,
                sender,
                time: String::new(),
                zoom: 1.0,
                pan: Vec2::ZERO,
            });
        }
    }

    fn render_feed_avatars_column(&mut self, ui: &mut egui::Ui, pal: &ThemePalette, col_w: f32) {
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(14.0))
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_width(col_w - 32.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Kho ảnh Nhật ký cá nhân ({} ảnh)",
                            self.feed_avatars.len()
                        ))
                        .size(14.5)
                        .color(pal.text_primary)
                        .strong(),
                    );
                    if ui
                        .add_enabled(
                            !self.feed_loading_avatars,
                            egui::Button::new(RichText::new("Làm mới").size(12.0)),
                        )
                        .clicked()
                    {
                        self.feed_loading_avatars = true;
                        if let Some(tx) = WORKER_TX.get() {
                            let _ = tx.send(WorkerCmd::FetchMySocialAvatars { page: 1, count: 20 });
                        }
                    }
                });
                ui.add_space(10.0);
                let avatars_snap = self.feed_avatars.clone();
                ui.horizontal_wrapped(|ui| {
                    for item in &avatars_snap {
                        let tkey = format!("fava_{}", crate::crypto::md5_hex(&item.thumbnail));
                        if !self.sticker_textures.contains_key(&tkey)
                            && self.loading_stickers.insert(tkey.clone())
                        {
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::FetchStickerImage {
                                    key: tkey.clone(),
                                    url: item.thumbnail.clone(),
                                });
                            }
                        }
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(130.0), Sense::click());
                        if let Some(tex) = self.sticker_textures.get(&tkey) {
                            ui.painter().image(
                                tex.id(),
                                r,
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        } else {
                            ui.painter()
                                .rect_filled(r, Rounding::same(8.0), pal.bg_input);
                        }
                        if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                            let hd = self.request_hd_texture(&item.url);
                            self.image_viewer = Some(ImageViewerModal {
                                tex_key: tkey,
                                hd_tex_key: hd,
                                full_url: item.url.clone(),
                                title: format!("Ảnh Nhật ký ({})", item.photo_id),
                                sender: self.user_name.clone(),
                                time: String::new(),
                                zoom: 1.0,
                                pan: Vec2::ZERO,
                            });
                        }
                    }
                });
            });
    }

    fn render_feed_packet_column(&mut self, ui: &mut egui::Ui, pal: &ThemePalette, col_w: f32) {
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(14.0))
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_width(col_w - 32.0);
                ui.horizontal(|ui| {
                    ui.label("Owner UID:");
                    ui.add_sized(
                        [140.0, 26.0],
                        egui::TextEdit::singleline(&mut self.feed_pkt_owner_id),
                    );
                    ui.label("Viewer UID:");
                    ui.add_sized(
                        [140.0, 26.0],
                        egui::TextEdit::singleline(&mut self.feed_pkt_viewer_id),
                    );
                });
                ui.add_space(6.0);
                let owner_u64 = self.feed_pkt_owner_id.trim().parse::<u64>().unwrap_or(0);
                let viewer_u64 = self.feed_pkt_viewer_id.trim().parse::<u64>().unwrap_or(0);
                let payload = crate::feed::packet::build_get_feed_payload(
                    owner_u64,
                    viewer_u64,
                    self.feed_pkt_tab,
                    self.feed_pkt_page,
                    self.feed_pkt_count,
                );
                let req_pkt = crate::feed::packet::build_request_packet(
                    crate::feed::packet::CMD_FEED_TIMELINE,
                    crate::feed::packet::SUBCMD_GET_FEED,
                    &payload,
                );
                let hex_str: String = req_pkt
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                ui.code(&hex_str);
                if ui
                    .button(format!("{} Sao chép Hex Packet", lucide::COPY))
                    .clicked()
                {
                    ui.ctx().copy_text(hex_str);
                    self.show_toast("Đã sao chép gói tin Command 50001 vào clipboard");
                }
            });
    }
}

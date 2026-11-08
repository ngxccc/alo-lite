use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_threads_sidebar(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
        sidebar_exact_w: f32,
    ) {
        let is_compact_w = false;
        let is_narrow_w = false;
        let sidebar_id = "conversation_sidebar_fixed";
        egui::SidePanel::left(sidebar_id)
            .resizable(false)
            .exact_width(sidebar_exact_w)
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_sidebar)
                    .stroke(Stroke::new(1.0_f32, pal.border)),
            )
            .show(ctx, |ui| {
                ui.set_min_width(sidebar_exact_w);
                ui.set_max_width(sidebar_exact_w);
                egui::Frame::none()
                    .inner_margin(Margin {
                        left: if is_narrow_w { 4.0 } else { 14.0 },
                        right: if is_narrow_w { 4.0 } else { 14.0 },
                        top: 12.0,
                        bottom: 0.0,
                    })
                    .show(ui, |ui| {
                        if is_narrow_w {
                            // Compact mode: icon-only search / lookup button
                            ui.vertical_centered(|ui| {
                                let (srch_r, srch_resp) =
                                    ui.allocate_exact_size(Vec2::new(42.0, 32.0), Sense::click());
                                ui.painter()
                                    .rect_filled(srch_r, Rounding::same(8.0), pal.bg_input);
                                ui.painter().rect_stroke(
                                    srch_r,
                                    Rounding::same(8.0),
                                    Stroke::new(1.0_f32, pal.border),
                                );
                                draw_icon_search(
                                    ui.painter(),
                                    srch_r.center(),
                                    16.0,
                                    pal.primary_hover,
                                );
                                if srch_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Tìm kiếm hội thoại & Tra cứu SĐT")
                                    .clicked()
                                {
                                    self.phone_modal_open = true;
                                }
                            });
                            ui.add_space(6.0);
                        } else {
                            // Search Box + Quick Phone Lookup (`+👤`) Button
                            let avail_row_w = sidebar_exact_w - 28.0;
                            let mut trigger_phone_lookup: Option<String> = None;
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let search_box_w = (avail_row_w - 38.0).max(110.0);
                                let has_q = !self.search_query.is_empty();
                                let (sbox_rect, _) = ui.allocate_exact_size(
                                    Vec2::new(search_box_w, 32.0),
                                    Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    sbox_rect,
                                    Rounding::same(8.0),
                                    pal.bg_input,
                                );
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
                                    Pos2::new(sbox_rect.left() + 16.0, sbox_rect.center().y),
                                    13.5,
                                    if has_q {
                                        pal.primary_hover
                                    } else {
                                        pal.text_muted
                                    },
                                );
                                let edit_rect = Rect::from_min_max(
                                    Pos2::new(sbox_rect.left() + 28.0, sbox_rect.top() + 5.0),
                                    Pos2::new(
                                        sbox_rect.right() - if has_q { 26.0 } else { 8.0 },
                                        sbox_rect.bottom() - 5.0,
                                    ),
                                );
                                let s_resp = ui.put(
                                    edit_rect,
                                    egui::TextEdit::singleline(&mut self.search_query)
                                        .id(Id::new("sidebar_search_input"))
                                        .frame(false)
                                        .hint_text("Tìm hội thoại, tin nhắn, SĐT...")
                                        .desired_width(edit_rect.width()),
                                );
                                let is_ime_busy = self.ime_active || self.ime_just_committed;
                                if is_ime_busy && s_resp.lost_focus() {
                                    s_resp.request_focus();
                                }
                                let s_enter = (s_resp.lost_focus() || s_resp.has_focus())
                                    && ui.input(|i| i.key_pressed(Key::Enter))
                                    && !is_ime_busy;
                                let digits_cnt = self
                                    .search_query
                                    .chars()
                                    .filter(|c| c.is_ascii_digit())
                                    .count();
                                if s_enter && digits_cnt >= 8 {
                                    trigger_phone_lookup =
                                        Some(self.search_query.trim().to_string());
                                }
                                if has_q {
                                    let clr_rect = Rect::from_center_size(
                                        Pos2::new(sbox_rect.right() - 14.0, sbox_rect.center().y),
                                        Vec2::new(18.0, 18.0),
                                    );
                                    let clr_resp = ui.interact(
                                        clr_rect,
                                        Id::new("sidebar_search_clear_btn"),
                                        Sense::click(),
                                    );
                                    if clr_resp.hovered() {
                                        ui.painter().circle_filled(
                                            clr_rect.center(),
                                            8.5,
                                            pal.bg_hover,
                                        );
                                    }
                                    draw_lucide_icon(
                                        ui.painter(),
                                        clr_rect.center(),
                                        lucide::X,
                                        11.5,
                                        if clr_resp.hovered() {
                                            pal.text_primary
                                        } else {
                                            pal.text_muted
                                        },
                                    );
                                    if clr_resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text("Xóa tìm kiếm")
                                        .clicked()
                                    {
                                        self.search_query.clear();
                                    }
                                }

                                // Add Friend / Lookup by Phone Button (`+👤`)
                                let (add_rect, add_resp) =
                                    ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::click());
                                ui.painter().rect_filled(
                                    add_rect,
                                    Rounding::same(8.0),
                                    if self.phone_modal_open {
                                        pal.primary.linear_multiply(0.22)
                                    } else if add_resp.hovered() {
                                        pal.bg_hover
                                    } else {
                                        pal.bg_input
                                    },
                                );
                                ui.painter().rect_stroke(
                                    add_rect,
                                    Rounding::same(8.0),
                                    Stroke::new(
                                        1.0_f32,
                                        if self.phone_modal_open || add_resp.hovered() {
                                            pal.primary
                                        } else {
                                            pal.border
                                        },
                                    ),
                                );
                                draw_icon_user_plus(
                                    ui.painter(),
                                    add_rect.center(),
                                    16.5,
                                    if self.phone_modal_open || add_resp.hovered() {
                                        pal.primary_hover
                                    } else {
                                        pal.text_secondary
                                    },
                                );
                                if add_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Tìm kiếm số điện thoại Zalo & Kết bạn")
                                    .clicked()
                                {
                                    self.phone_modal_open = true;
                                    let digits_cnt = self
                                        .search_query
                                        .chars()
                                        .filter(|c| c.is_ascii_digit())
                                        .count();
                                    if digits_cnt >= 8 && self.phone_modal_input.is_empty() {
                                        self.phone_modal_input =
                                            self.search_query.trim().to_string();
                                    }
                                }
                            });

                            if let Some(phone_str) = trigger_phone_lookup {
                                self.phone_modal_open = true;
                                self.phone_modal_input = phone_str.clone();
                                self.phone_lookup_loading = true;
                                self.phone_lookup_error = None;
                                self.phone_lookup_result = None;
                                if let Some(tx) = WORKER_TX.get() {
                                    let _ = tx.send(WorkerCmd::LookupPhone { phone: phone_str });
                                }
                            }
                            ui.add_space(10.0);

                            // Clean Underline Segmented Tabs (`Tất cả`, `Cá nhân`, `Nhóm`) + `Đã ghim (1)`
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 16.0;
                                let group_tab_cnt =
                                    self.threads.iter().filter(|t| t.is_group).count();
                                let group_tab_label = if group_tab_cnt > 0 {
                                    format!("Nhóm ({group_tab_cnt})")
                                } else {
                                    "Nhóm".to_string()
                                };
                                for (filter, label) in [
                                    (ThreadFilter::All, "Tất cả".to_string()),
                                    (ThreadFilter::Direct, "Cá nhân".to_string()),
                                    (ThreadFilter::Groups, group_tab_label),
                                ] {
                                    let active = self.thread_filter == filter;
                                    let text_w = ui.fonts(|f| {
                                        f.layout_no_wrap(
                                            label.clone(),
                                            FontId::proportional(12.5),
                                            pal.text_primary,
                                        )
                                        .size()
                                        .x
                                    });
                                    let (tab_rect, tab_resp) = ui.allocate_exact_size(
                                        Vec2::new(text_w + 4.0, 26.0),
                                        Sense::click(),
                                    );
                                    let fg = if active {
                                        pal.primary_hover
                                    } else if tab_resp.hovered() {
                                        pal.text_primary
                                    } else {
                                        pal.text_secondary
                                    };
                                    ui.painter().text(
                                        Pos2::new(tab_rect.center().x, tab_rect.top() + 8.0),
                                        egui::Align2::CENTER_CENTER,
                                        &label,
                                        FontId::proportional(12.5),
                                        fg,
                                    );
                                    if active {
                                        let line_y = tab_rect.bottom() - 1.0;
                                        ui.painter().line_segment(
                                            [
                                                Pos2::new(tab_rect.left(), line_y),
                                                Pos2::new(tab_rect.right(), line_y),
                                            ],
                                            Stroke::new(2.2_f32, pal.primary),
                                        );
                                    }
                                    if tab_resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
                                    {
                                        self.thread_filter = filter;
                                        if filter == ThreadFilter::Groups {
                                            if let Some((gidx, gthread)) = self
                                                .threads
                                                .iter()
                                                .enumerate()
                                                .find(|(_, t)| t.is_group)
                                            {
                                                let gid = gthread.id.clone();
                                                self.selected_thread_index = gidx;
                                                if let Some(tx) = WORKER_TX.get() {
                                                    let _ = tx.send(WorkerCmd::LoadGroupHistory {
                                                        group_id: gid,
                                                        is_group: true,
                                                        have: 0,
                                                    });
                                                }
                                            }
                                            if group_tab_cnt == 0
                                                || self.threads.iter().any(|t| {
                                                    t.is_group
                                                        && crate::protocol::messaging::threads::is_generic_thread_name(
                                                            &t.name,
                                                        )
                                                })
                                            {
                                                if let Some(tx) = WORKER_TX.get() {
                                                    let _ = tx.send(WorkerCmd::RefreshThreads);
                                                }
                                            }
                                        }
                                    }
                                }

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let pinned_cnt = self
                                        .threads
                                        .iter()
                                        .filter(|t| {
                                            self.settings.pinned_threads.contains(&t.id)
                                                || t.presentation.pinned
                                        })
                                        .count();
                                    if pinned_cnt > 0 {
                                        ui.label(
                                            RichText::new(format!("Đã ghim ({pinned_cnt})"))
                                                .size(11.0)
                                                .color(pal.text_muted),
                                        );
                                    }
                                });
                            });
                        }
                    });

                ui.add(egui::Separator::default().spacing(1.0));

                // Sort threads: "Cloud của tôi" pinned first, then descending by latest message timestamp
                let query = self.search_query.trim().to_lowercase();
                let query_folded = fold_vn_search(&query);
                let query_digits: String = self
                    .search_query
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                let is_phone_like_query = query_digits.len() >= 8
                    && self
                        .search_query
                        .trim()
                        .chars()
                        .all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '-' | '.'));

                if is_phone_like_query {
                    let raw_q = self.search_query.trim().to_string();
                    egui::Frame::none()
                        .fill(pal.primary.linear_multiply(0.14))
                        .stroke(Stroke::new(1.0_f32, pal.primary))
                        .rounding(Rounding::same(9.0))
                        .inner_margin(Margin::symmetric(10.0, 7.0))
                        .outer_margin(Margin::symmetric(8.0, 4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (ir, _) =
                                    ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                                draw_icon_user_plus(
                                    ui.painter(),
                                    ir.center(),
                                    16.0,
                                    pal.primary_hover,
                                );
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(format!("Tìm Zalo qua SĐT: {raw_q}"))
                                            .size(12.2)
                                            .color(pal.text_primary)
                                            .strong(),
                                    );
                                    ui.label(
                                        RichText::new("Nhấn nút Tìm hoặc phím Enter để tra cứu")
                                            .size(10.8)
                                            .color(pal.text_secondary),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let btn = egui::Button::new(
                                        RichText::new("Tìm SĐT")
                                            .size(11.5)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(pal.primary)
                                    .rounding(Rounding::same(6.0));
                                    if ui
                                        .add(btn)
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        self.phone_modal_open = true;
                                        self.phone_modal_input = raw_q.clone();
                                        self.phone_lookup_loading = true;
                                        self.phone_lookup_error = None;
                                        self.phone_lookup_result = None;
                                        if let Some(tx) = WORKER_TX.get() {
                                            let _ =
                                                tx.send(WorkerCmd::LookupPhone { phone: raw_q });
                                        }
                                    }
                                });
                            });
                        });
                }

                let search_outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut filtered_indices: Vec<usize> = self
                        .threads
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| {
                            if query.is_empty() && self.settings.hidden_threads.contains(&t.id) {
                                return false;
                            }
                            let pass_filter = match self.thread_filter {
                                ThreadFilter::All => true,
                                ThreadFilter::Direct => !t.is_group,
                                ThreadFilter::Groups => t.is_group,
                            };
                            let not_hidden = !t.presentation.hidden || !query.is_empty();
                            pass_filter
                                && not_hidden
                                && (query.is_empty()
                                    || matches_vn_query(&t.name, &query, &query_folded)
                                    || t.id.contains(&query))
                        })
                        .map(|(i, _)| i)
                        .collect();

                    filtered_indices.sort_by(|&a, &b| {
                        let ta = &self.threads[a];
                        let tb = &self.threads[b];
                        let a_pinned =
                            self.settings.pinned_threads.contains(&ta.id) || ta.presentation.pinned;
                        let b_pinned =
                            self.settings.pinned_threads.contains(&tb.id) || tb.presentation.pinned;
                        if a_pinned != b_pinned {
                            return b_pinned.cmp(&a_pinned);
                        }
                        let ts_a = ta
                            .messages
                            .last()
                            .map(|m| normalize_ts_ms(m.ts))
                            .unwrap_or(ta.presentation.last_message_ts);
                        let ts_b = tb
                            .messages
                            .last()
                            .map(|m| normalize_ts_ms(m.ts))
                            .unwrap_or(tb.presentation.last_message_ts);
                        if ts_a != ts_b {
                            return ts_b.cmp(&ts_a);
                        }
                        ta.name.to_lowercase().cmp(&tb.name.to_lowercase())
                    });

                    let msg_hits: Vec<(usize, String, String, String, bool)> =
                        if query.chars().count() >= 2 {
                            let mut hits = Vec::new();
                            for (t_idx, t) in self.threads.iter().enumerate() {
                                for m in t.messages.iter().rev() {
                                    if matches_vn_query(&m.text, &query, &query_folded) {
                                        hits.push((
                                            t_idx,
                                            t.name.clone(),
                                            m.text.clone(),
                                            m.time.clone(),
                                            t.is_group,
                                        ));
                                        if hits.len() >= 25 {
                                            break;
                                        }
                                    }
                                }
                                if hits.len() >= 25 {
                                    break;
                                }
                            }
                            hits
                        } else {
                            Vec::new()
                        };
                    (filtered_indices, msg_hits)
                }));
                let (filtered_indices, msg_hits) = match search_outcome {
                    Ok(pair) => pair,
                    Err(_) => {
                        self.search_query.clear();
                        self.show_toast("Đã đặt lại bộ lọc tìm kiếm");
                        ((0..self.threads.len()).collect(), Vec::new())
                    }
                };

                let mut clicked_thread: Option<(usize, String, bool)> = None;
                let mut clicked_msg_hit: Option<(usize, String)> = None;
                #[derive(Clone)]
                enum ThreadCtxAction {
                    TogglePin(String, bool, bool),
                    SetLabel(String, Option<String>),
                    ToggleUnread(String, bool, bool),
                    SetMute(String, bool, Option<i64>, String),
                    ToggleHide(String, bool, bool),
                    OpenSchedule(usize),
                    DeleteConv(usize, String, String, bool, String),
                    ReportConv(String),
                }
                let mut pending_ctx_action: Option<ThreadCtxAction> = None;
                let now_unix = crate::clock::unix_now();
                let row_target_w = (sidebar_exact_w - 14.0).max(44.0);
                egui::ScrollArea::vertical()
                    .id_salt("threads_list_scroll")
                    .max_width(sidebar_exact_w)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(sidebar_exact_w - 6.0);
                        ui.set_max_width(sidebar_exact_w - 6.0);
                        ui.add_space(4.0);
                        for &i in &filtered_indices {
                            let t = &self.threads[i];
                            let is_selected = i == self.selected_thread_index;
                            let is_user_pinned = self.settings.pinned_threads.contains(&t.id)
                                || t.presentation.pinned;
                            let is_pinned = is_user_pinned;
                            let is_muted = self
                                .settings
                                .muted_threads
                                .get(&t.id)
                                .map(|&until| until == -1 || until > now_unix)
                                .unwrap_or(false)
                                || t.presentation.muted
                                || t.unread.is_muted();
                            let is_hidden = self.settings.hidden_threads.contains(&t.id)
                                || t.presentation.hidden;
                            let cur_label = self.settings.thread_labels.get(&t.id).cloned();
                            let row_height = if is_narrow_w { 50.0 } else { 58.0 };
                            let (alloc_rect, row_resp) = ui.allocate_exact_size(
                                Vec2::new(row_target_w, row_height),
                                Sense::click(),
                            );
                            let row_rect = alloc_rect.translate(Vec2::new(4.0, 0.0));
                            // Space is still allocated so the scrollbar is right, but off-screen rows
                            // skip painting, and with it the avatar download/texture for every thread.
                            if !ui.is_rect_visible(row_rect) {
                                continue;
                            }

                            let bg = if is_selected {
                                pal.bg_selected
                            } else if row_resp.hovered() {
                                pal.bg_hover
                            } else {
                                Color32::TRANSPARENT
                            };

                            ui.painter().rect_filled(row_rect, Rounding::same(11.0), bg);

                            // Left active indicator bar (`w-1 bg-zalo-500 rounded-r-full`)
                            if is_selected {
                                let bar_rect = Rect::from_min_size(
                                    Pos2::new(row_rect.left() + 1.0, row_rect.top() + 10.0),
                                    Vec2::new(3.5, 38.0),
                                );
                                ui.painter().rect_filled(
                                    bar_rect,
                                    Rounding::same(2.0),
                                    pal.primary,
                                );
                            }

                            // Avatar circle with real user/group image & Cutout Status Dot
                            let av_radius = if is_narrow_w { 18.0 } else { 20.5 };
                            let avatar_center = if is_narrow_w {
                                row_rect.center()
                            } else {
                                Pos2::new(row_rect.left() + 29.0, row_rect.center().y)
                            };
                            paint_user_avatar(
                                ui.painter(),
                                avatar_center,
                                av_radius,
                                &t.name,
                                t.is_group,
                                t.avatar.as_deref(),
                                &self.sticker_textures,
                                &mut self.loading_stickers,
                            );
                            // Cutout status dot (`2.5px` ring matching row background) only when user is actively online
                            let is_user_online = !t.is_group
                                && self
                                    .user_online_map
                                    .get(&t.id)
                                    .map(|(onl, _)| *onl)
                                    .unwrap_or(false);
                            if is_user_online {
                                let cutout_col = if is_selected {
                                    pal.bg_selected
                                } else {
                                    pal.bg_sidebar
                                };
                                let badge_pos = Pos2::new(
                                    avatar_center.x + av_radius * 0.71,
                                    avatar_center.y + av_radius * 0.71,
                                );
                                ui.painter().circle_filled(badge_pos, 6.0, cutout_col);
                                ui.painter()
                                    .circle_filled(badge_pos, 4.0, pal.accent_online);
                            }
                            // Only show text labels when sidebar is expanded
                            if !is_narrow_w {
                                // Thread Name
                                let text_left = row_rect.left() + 58.0;
                                let name_color = if is_selected && pal.is_dark {
                                    Color32::WHITE
                                } else {
                                    pal.text_primary
                                };
                                let max_name_chars = if is_compact_w { 15 } else { 21 };
                                let name_x = if t.is_group {
                                    draw_icon_contacts(
                                        ui.painter(),
                                        Pos2::new(text_left + 7.0, row_rect.top() + 19.5),
                                        12.5,
                                        pal.primary,
                                    );
                                    text_left + 18.0
                                } else {
                                    text_left
                                };
                                let display_name = truncate_chars(&t.name, max_name_chars);
                                ui.painter().text(
                                    Pos2::new(name_x, row_rect.top() + 12.0),
                                    egui::Align2::LEFT_TOP,
                                    display_name,
                                    FontId::proportional(13.6),
                                    name_color,
                                );

                                // Pin icon + Timestamp (top-right)
                                let time_right = row_rect.right() - 9.0;
                                if !t.time.is_empty() {
                                    let tw = ui.fonts(|f| {
                                        f.layout_no_wrap(
                                            t.time.clone(),
                                            FontId::proportional(11.0),
                                            pal.text_muted,
                                        )
                                        .size()
                                        .x
                                    });
                                    ui.painter().text(
                                        Pos2::new(time_right, row_rect.top() + 13.5),
                                        egui::Align2::RIGHT_TOP,
                                        &t.time,
                                        FontId::proportional(11.0),
                                        pal.text_muted,
                                    );
                                    let mut icon_right = time_right - tw - 9.0;
                                    if is_pinned {
                                        draw_icon_pin(
                                            ui.painter(),
                                            Pos2::new(icon_right, row_rect.top() + 19.5),
                                            11.0,
                                            pal.text_muted,
                                        );
                                        icon_right -= 14.0;
                                    }
                                    if is_muted {
                                        ui.painter().text(
                                            Pos2::new(icon_right + 4.0, row_rect.top() + 13.5),
                                            egui::Align2::RIGHT_TOP,
                                            "🔕",
                                            FontId::proportional(10.5),
                                            pal.text_muted,
                                        );
                                    }
                                }
                                if let Some(ref lbl) = cur_label {
                                    let tag_col = match lbl.as_str() {
                                        "Khách hàng" => Color32::from_rgb(239, 68, 68),
                                        "Gia đình" => Color32::from_rgb(34, 197, 94),
                                        "Công việc" => Color32::from_rgb(249, 115, 22),
                                        "Bạn bè" => Color32::from_rgb(168, 85, 247),
                                        "Trả lời sau" => Color32::from_rgb(234, 179, 8),
                                        _ => Color32::from_rgb(59, 130, 246),
                                    };
                                    ui.painter().circle_filled(
                                        Pos2::new(text_left - 8.0, row_rect.top() + 39.5),
                                        3.5,
                                        tag_col,
                                    );
                                }

                                // Snippet with `Bạn:` prefix when latest message is outgoing
                                let last_outgoing =
                                    t.messages.last().map(|m| m.is_outgoing).unwrap_or(false);
                                let raw_preview = if t.preview.is_empty() {
                                    if t.is_group {
                                        "Nhóm trò chuyện".to_string()
                                    } else {
                                        "Chưa có tin nhắn".to_string()
                                    }
                                } else if last_outgoing && !t.preview.starts_with("Bạn:") {
                                    format!("Bạn: {}", t.preview)
                                } else {
                                    t.preview.clone()
                                };
                                let unread_n = self.unread_counts.get(&t.id).copied().unwrap_or(0);
                                let max_snip_chars = if is_compact_w {
                                    16
                                } else if unread_n > 0 {
                                    23
                                } else {
                                    28
                                };
                                let snippet = truncate_chars(&raw_preview, max_snip_chars);
                                ui.painter().text(
                                    Pos2::new(text_left, row_rect.top() + 33.5),
                                    egui::Align2::LEFT_TOP,
                                    snippet,
                                    FontId::proportional(12.0),
                                    if unread_n > 0 {
                                        pal.text_primary
                                    } else {
                                        pal.text_secondary
                                    },
                                );
                                if unread_n > 0 {
                                    let badge_txt = if unread_n > 99 {
                                        "99+".to_string()
                                    } else {
                                        unread_n.to_string()
                                    };
                                    let bw = if unread_n >= 10 { 22.0 } else { 18.0 };
                                    let b_rect = Rect::from_min_size(
                                        Pos2::new(
                                            row_rect.right() - bw - 8.0,
                                            row_rect.top() + 31.5,
                                        ),
                                        Vec2::new(bw, 17.0),
                                    );
                                    ui.painter().rect_filled(
                                        b_rect,
                                        Rounding::same(8.5),
                                        pal.danger,
                                    );
                                    ui.painter().text(
                                        b_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        badge_txt,
                                        FontId::proportional(10.2),
                                        Color32::WHITE,
                                    );
                                }
                            }

                            let row_resp = row_resp.on_hover_cursor(CursorIcon::PointingHand);
                            let row_resp = if is_narrow_w {
                                row_resp.on_hover_text(&t.name)
                            } else {
                                row_resp
                            };
                            let tid_cl = t.id.clone();
                            let tname_cl = t.name.clone();
                            let tgroup_cl = t.is_group;
                            let unread_now =
                                self.unread_counts.get(&t.id).copied().unwrap_or(0) > 0;
                            let last_msg_id_cl =
                                t.messages.last().map(|m| m.id.clone()).unwrap_or_default();
                            row_resp.context_menu(|ui| {
                                ui.set_min_width(205.0);
                                ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                                ui.spacing_mut().button_padding = Vec2::new(10.0, 6.5);

                                // 1. Ghim / Bỏ ghim hội thoại
                                let pin_label = if is_user_pinned {
                                    "Bỏ ghim hội thoại"
                                } else {
                                    "Ghim hội thoại"
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(pin_label)
                                                .size(13.5)
                                                .color(pal.text_primary),
                                        )
                                        .frame(false)
                                        .min_size(Vec2::new(195.0, 28.0)),
                                    )
                                    .clicked()
                                {
                                    pending_ctx_action = Some(ThreadCtxAction::TogglePin(
                                        tid_cl.clone(),
                                        tgroup_cl,
                                        !is_user_pinned,
                                    ));
                                    ui.close_menu();
                                }

                                ui.separator();

                                // 2. Phân loại >
                                ui.menu_button(
                                    RichText::new("Phân loại")
                                        .size(13.5)
                                        .color(pal.text_primary),
                                    |ui| {
                                        ui.set_min_width(185.0);
                                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                                        ui.spacing_mut().button_padding = Vec2::new(10.0, 6.0);
                                        let categories = [
                                            ("Khách hàng", Color32::from_rgb(239, 68, 68)),
                                            ("Gia đình", Color32::from_rgb(34, 197, 94)),
                                            ("Công việc", Color32::from_rgb(249, 115, 22)),
                                            ("Bạn bè", Color32::from_rgb(168, 85, 247)),
                                            ("Trả lời sau", Color32::from_rgb(234, 179, 8)),
                                            ("Đồng nghiệp", Color32::from_rgb(59, 130, 246)),
                                        ];
                                        for (cat_name, dot_col) in categories {
                                            let is_cur = cur_label.as_deref() == Some(cat_name);
                                            let txt = if is_cur {
                                                format!("● {cat_name}  ✓")
                                            } else {
                                                format!("● {cat_name}")
                                            };
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new(txt)
                                                            .size(13.2)
                                                            .color(dot_col),
                                                    )
                                                    .frame(false)
                                                    .min_size(Vec2::new(175.0, 26.0)),
                                                )
                                                .clicked()
                                            {
                                                let next_lbl = if is_cur {
                                                    None
                                                } else {
                                                    Some(cat_name.to_string())
                                                };
                                                pending_ctx_action =
                                                    Some(ThreadCtxAction::SetLabel(
                                                        tid_cl.clone(),
                                                        next_lbl,
                                                    ));
                                                ui.close_menu();
                                            }
                                        }
                                        if cur_label.is_some() {
                                            ui.separator();
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new("Bỏ phân loại")
                                                            .size(13.0)
                                                            .color(pal.text_secondary),
                                                    )
                                                    .frame(false)
                                                    .min_size(Vec2::new(175.0, 26.0)),
                                                )
                                                .clicked()
                                            {
                                                pending_ctx_action = Some(
                                                    ThreadCtxAction::SetLabel(tid_cl.clone(), None),
                                                );
                                                ui.close_menu();
                                            }
                                        }
                                    },
                                );

                                // 3. Đánh dấu chưa đọc / Đánh dấu đã đọc
                                let unread_label = if unread_now {
                                    "Đánh dấu đã đọc"
                                } else {
                                    "Đánh dấu chưa đọc"
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(unread_label)
                                                .size(13.5)
                                                .color(pal.text_primary),
                                        )
                                        .frame(false)
                                        .min_size(Vec2::new(195.0, 28.0)),
                                    )
                                    .clicked()
                                {
                                    pending_ctx_action = Some(ThreadCtxAction::ToggleUnread(
                                        tid_cl.clone(),
                                        tgroup_cl,
                                        !unread_now,
                                    ));
                                    ui.close_menu();
                                }

                                ui.separator();

                                // 4. Tắt thông báo > (hoặc Bật lại thông báo)
                                if is_muted {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Mở lại thông báo")
                                                    .size(13.5)
                                                    .color(pal.text_primary),
                                            )
                                            .frame(false)
                                            .min_size(Vec2::new(195.0, 28.0)),
                                        )
                                        .clicked()
                                    {
                                        pending_ctx_action = Some(ThreadCtxAction::SetMute(
                                            tid_cl.clone(),
                                            tgroup_cl,
                                            None,
                                            "Đã mở lại thông báo hội thoại".into(),
                                        ));
                                        ui.close_menu();
                                    }
                                } else {
                                    ui.menu_button(
                                        RichText::new("Tắt thông báo")
                                            .size(13.5)
                                            .color(pal.text_primary),
                                        |ui| {
                                            ui.set_min_width(195.0);
                                            ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                                            ui.spacing_mut().button_padding = Vec2::new(10.0, 6.5);
                                            let rem_today = 86_400 - (now_unix % 86_400);
                                            let dur_8am = (rem_today + 8 * 3600) % 86_400;
                                            let dur_8am = if dur_8am < 600 {
                                                dur_8am + 86_400
                                            } else {
                                                dur_8am
                                            };
                                            let opts: [(&str, i64); 4] = [
                                                ("Trong 1 giờ", 3600),
                                                ("Trong 4 giờ", 14_400),
                                                ("Cho đến 8:00 AM", dur_8am),
                                                ("Cho đến khi được mở lại", -1),
                                            ];
                                            for (opt_label, dur_s) in opts {
                                                if ui
                                                    .add(
                                                        egui::Button::new(
                                                            RichText::new(opt_label)
                                                                .size(13.5)
                                                                .color(pal.text_primary),
                                                        )
                                                        .frame(false)
                                                        .min_size(Vec2::new(185.0, 28.0)),
                                                    )
                                                    .clicked()
                                                {
                                                    pending_ctx_action =
                                                        Some(ThreadCtxAction::SetMute(
                                                            tid_cl.clone(),
                                                            tgroup_cl,
                                                            Some(dur_s),
                                                            format!(
                                                                "🔕 Đã tắt thông báo ({opt_label})"
                                                            ),
                                                        ));
                                                    ui.close_menu();
                                                }
                                            }
                                        },
                                    );
                                }

                                // 5. Ẩn trò chuyện / Bỏ ẩn trò chuyện
                                let hide_label = if is_hidden {
                                    "Bỏ ẩn trò chuyện"
                                } else {
                                    "Ẩn trò chuyện"
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(hide_label)
                                                .size(13.5)
                                                .color(pal.text_primary),
                                        )
                                        .frame(false)
                                        .min_size(Vec2::new(195.0, 28.0)),
                                    )
                                    .clicked()
                                {
                                    pending_ctx_action = Some(ThreadCtxAction::ToggleHide(
                                        tid_cl.clone(),
                                        tgroup_cl,
                                        !is_hidden,
                                    ));
                                    ui.close_menu();
                                }

                                // 5b. Hẹn giờ gửi tin nhắn / Nhắc hẹn (#38)
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Hẹn giờ gửi tin nhắn / Nhắc hẹn")
                                                .size(13.5)
                                                .color(pal.text_primary),
                                        )
                                        .frame(false)
                                        .min_size(Vec2::new(195.0, 28.0)),
                                    )
                                    .clicked()
                                {
                                    pending_ctx_action = Some(ThreadCtxAction::OpenSchedule(i));
                                    ui.close_menu();
                                }

                                ui.separator();

                                // 6. Xóa hội thoại (màu đỏ)
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Xóa hội thoại")
                                                .size(13.5)
                                                .color(Color32::from_rgb(239, 68, 68)),
                                        )
                                        .frame(false)
                                        .min_size(Vec2::new(195.0, 28.0)),
                                    )
                                    .clicked()
                                {
                                    pending_ctx_action = Some(ThreadCtxAction::DeleteConv(
                                        i,
                                        tid_cl.clone(),
                                        tname_cl.clone(),
                                        tgroup_cl,
                                        last_msg_id_cl.clone(),
                                    ));
                                    ui.close_menu();
                                }

                                ui.separator();

                                // 7. Báo xấu
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Báo xấu")
                                                .size(13.5)
                                                .color(pal.text_primary),
                                        )
                                        .frame(false)
                                        .min_size(Vec2::new(195.0, 28.0)),
                                    )
                                    .clicked()
                                {
                                    pending_ctx_action =
                                        Some(ThreadCtxAction::ReportConv(tname_cl.clone()));
                                    ui.close_menu();
                                }
                            });

                            if row_resp.clicked() {
                                clicked_thread = Some((i, t.id.clone(), t.is_group));
                            }
                        }

                        // Global Message Content Matches (`Tin nhắn tìm thấy`)
                        if !msg_hits.is_empty() {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.add_space(12.0);
                                ui.label(
                                    RichText::new(format!(
                                        "TIN NHẮN TÌM THẤY ({})",
                                        msg_hits.len()
                                    ))
                                    .size(11.0)
                                    .color(pal.primary_hover)
                                    .strong(),
                                );
                            });
                            ui.add_space(4.0);
                            for (t_idx, t_name, m_text, m_time, t_group) in &msg_hits {
                                let (alloc_r, r_resp) = ui.allocate_exact_size(
                                    Vec2::new(row_target_w, 52.0),
                                    Sense::click(),
                                );
                                let r_rect = alloc_r.translate(Vec2::new(4.0, 0.0));
                                if r_resp.hovered() {
                                    ui.painter().rect_filled(
                                        r_rect,
                                        Rounding::same(10.0),
                                        pal.bg_hover,
                                    );
                                }
                                let av_c = Pos2::new(r_rect.left() + 24.0, r_rect.center().y);
                                let hit_ava =
                                    self.threads.get(*t_idx).and_then(|th| th.avatar.as_deref());
                                paint_user_avatar(
                                    ui.painter(),
                                    av_c,
                                    16.0,
                                    t_name,
                                    *t_group,
                                    hit_ava,
                                    &self.sticker_textures,
                                    &mut self.loading_stickers,
                                );
                                let tl = r_rect.left() + 50.0;
                                ui.painter().text(
                                    Pos2::new(tl, r_rect.top() + 9.0),
                                    egui::Align2::LEFT_TOP,
                                    truncate_chars(t_name, 20),
                                    FontId::proportional(12.8),
                                    pal.text_primary,
                                );
                                if !m_time.is_empty() {
                                    ui.painter().text(
                                        Pos2::new(r_rect.right() - 10.0, r_rect.top() + 10.0),
                                        egui::Align2::RIGHT_TOP,
                                        m_time,
                                        FontId::proportional(10.5),
                                        pal.text_muted,
                                    );
                                }
                                ui.painter().text(
                                    Pos2::new(tl, r_rect.top() + 28.0),
                                    egui::Align2::LEFT_TOP,
                                    truncate_chars(m_text, 30),
                                    FontId::proportional(11.8),
                                    pal.primary_hover,
                                );
                                if r_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text(m_text)
                                    .clicked()
                                {
                                    clicked_msg_hit = Some((*t_idx, query.clone()));
                                }
                            }
                        }

                        if !query.is_empty() && filtered_indices.is_empty() && msg_hits.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(32.0);
                                ui.label(
                                    RichText::new(
                                        "Không tìm thấy hội thoại hoặc tin nhắn phù hợp.",
                                    )
                                    .size(12.5)
                                    .color(pal.text_muted),
                                );
                            });
                        }
                    });

                if let Some((idx, q_str)) = clicked_msg_hit {
                    self.selected_thread_index = idx;
                    self.in_chat_search_open = true;
                    self.in_chat_search_focus_pending = true;
                    self.in_chat_search_query = q_str;
                }
                if let Some((idx, tid, is_group)) = clicked_thread {
                    let resolved_idx = self.threads.iter().position(|t| t.id == tid).unwrap_or(idx);
                    if self.selected_thread_index != resolved_idx {
                        self.replying_to = None;
                        self.filter_starred_only = false;
                        self.force_scroll_to_bottom = true;
                        self.chat_render_window = (tid.clone(), CHAT_RENDER_STEP);
                    }
                    self.selected_thread_index = resolved_idx;
                    if self.unread_counts.remove(&tid).is_some() {
                        let total_unread: usize = self.unread_counts.values().sum();
                        let title = if total_unread > 0 {
                            format!("({total_unread}) Zalo Lite — Native Rust Client")
                        } else {
                            "Zalo Lite — Native Rust Client".to_string()
                        };
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
                    }
                    if !self.settings.block_seen {
                        let last_mid = self
                            .threads
                            .get(resolved_idx)
                            .and_then(|t| t.messages.last())
                            .map(|m| m.id.clone())
                            .unwrap_or_default();
                        if !last_mid.is_empty() {
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::SendSeen {
                                    thread_id: tid.clone(),
                                    is_group,
                                    last_msg_id: last_mid,
                                });
                            }
                        }
                    }
                    let is_cloud = self
                        .threads
                        .get(resolved_idx)
                        .map(|t| t.name.contains("Cloud của tôi"))
                        .unwrap_or(false);
                    let needs_history = self
                        .threads
                        .get(resolved_idx)
                        .is_none_or(|t| t.messages.is_empty());
                    if (is_group || is_cloud) && needs_history {
                        if let Some(tx) = WORKER_TX.get() {
                            let _ = tx.send(WorkerCmd::LoadGroupHistory {
                                group_id: tid,
                                is_group,
                                have: 0,
                            });
                        }
                    }
                }
                if let Some(act) = pending_ctx_action {
                    match act {
                        ThreadCtxAction::TogglePin(tid, is_group, pin) => {
                            if pin {
                                if !self.settings.pinned_threads.contains(&tid) {
                                    self.settings.pinned_threads.push(tid.clone());
                                }
                                self.show_toast("📌 Đã ghim hội thoại");
                            } else {
                                self.settings.pinned_threads.retain(|x| x != &tid);
                                self.show_toast("Đã bỏ ghim hội thoại");
                            }
                            if let Some(th) = self.threads.iter_mut().find(|t| t.id == tid) {
                                th.presentation.pinned = pin;
                            }
                            self.settings.save();
                            self.persist_threads_async();
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::PinThread {
                                    thread_id: tid,
                                    is_group,
                                    pin,
                                });
                            }
                        }
                        ThreadCtxAction::SetLabel(tid, lbl_opt) => {
                            if let Some(lbl) = lbl_opt {
                                self.settings.thread_labels.insert(tid, lbl.clone());
                                self.show_toast(format!("Đã phân loại: {lbl}"));
                            } else {
                                self.settings.thread_labels.remove(&tid);
                                self.show_toast("Đã bỏ thẻ phân loại");
                            }
                            self.settings.save();
                        }
                        ThreadCtxAction::ToggleUnread(tid, is_group, unread) => {
                            if unread {
                                self.unread_counts.insert(tid.clone(), 1);
                                self.show_toast("Đã đánh dấu chưa đọc");
                            } else {
                                self.unread_counts.remove(&tid);
                                self.show_toast("Đã đánh dấu đã đọc");
                            }
                            let total_unread: usize = self.unread_counts.values().sum();
                            let title = if total_unread > 0 {
                                format!("({total_unread}) Zalo Lite — Native Rust Client")
                            } else {
                                "Zalo Lite — Native Rust Client".to_string()
                            };
                            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::MarkThreadUnread {
                                    thread_id: tid,
                                    is_group,
                                    unread,
                                });
                            }
                        }
                        ThreadCtxAction::SetMute(tid, is_group, dur_opt, toast_txt) => {
                            let (dur_sec, mute) = match dur_opt {
                                Some(-1) => {
                                    self.settings.muted_threads.insert(tid.clone(), -1);
                                    (-1, true)
                                }
                                Some(d) => {
                                    self.settings
                                        .muted_threads
                                        .insert(tid.clone(), now_unix + d);
                                    (d, true)
                                }
                                None => {
                                    self.settings.muted_threads.remove(&tid);
                                    (-1, false)
                                }
                            };
                            if let Some(th) = self.threads.iter_mut().find(|t| t.id == tid) {
                                th.presentation.muted = mute;
                                th.unread.muted = mute;
                                th.unread.muted_until = if mute && dur_sec > 0 {
                                    Some(now_unix + dur_sec)
                                } else {
                                    None
                                };
                                th.unread.settings_fresh = true;
                                th.unread.fresh = true;
                            }
                            self.settings.save();
                            self.persist_threads_async();
                            self.show_toast(toast_txt);
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::MuteThread {
                                    thread_id: tid,
                                    is_group,
                                    duration_sec: dur_sec,
                                    mute,
                                });
                            }
                        }
                        ThreadCtxAction::ToggleHide(tid, is_group, hide) => {
                            if hide {
                                self.settings.hidden_threads.insert(tid.clone());
                                self.show_toast(
                                    "🙈 Đã ẩn trò chuyện (gõ tên vào ô Tìm kiếm để mở lại)",
                                );
                            } else {
                                self.settings.hidden_threads.remove(&tid);
                                self.show_toast("Đã bỏ ẩn trò chuyện");
                            }
                            if let Some(th) = self.threads.iter_mut().find(|t| t.id == tid) {
                                th.presentation.hidden = hide;
                            }
                            self.settings.save();
                            self.persist_threads_async();
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::HideThread {
                                    thread_id: tid,
                                    is_group,
                                    hide,
                                });
                            }
                        }
                        ThreadCtxAction::OpenSchedule(idx) => {
                            self.selected_thread_index = idx;
                            self.schedule_modal_open = true;
                        }
                        ThreadCtxAction::DeleteConv(idx, tid, tname, is_group, last_msg_id) => {
                            if let Ok(mut store) =
                                crate::store::MessageStore::open("zalo_history.db")
                            {
                                let _ = store.delete_thread(&tid);
                            }
                            if idx < self.threads.len() && self.threads[idx].id == tid {
                                self.threads.remove(idx);
                                if self.selected_thread_index >= self.threads.len() {
                                    self.selected_thread_index =
                                        self.threads.len().saturating_sub(1);
                                }
                                self.unread_counts.remove(&tid);
                                self.settings.pinned_threads.retain(|x| x != &tid);
                                self.settings.save();
                                self.persist_threads_async();
                            }
                            self.show_toast(format!("Đã xóa hội thoại \"{tname}\""));
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::DeleteThread {
                                    thread_id: tid,
                                    is_group,
                                    last_cli_msg_id: last_msg_id,
                                });
                            }
                        }
                        ThreadCtxAction::ReportConv(tname) => {
                            self.show_toast(format!(
                                "🚩 Đã gửi báo xấu hội thoại \"{tname}\" tới hệ thống Zalo"
                            ));
                        }
                    }
                }
            });
    }
}

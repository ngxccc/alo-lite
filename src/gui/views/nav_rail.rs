use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_nav_rail(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
        left_rail_w: f32,
        is_narrow_w: bool,
    ) {
        egui::SidePanel::left("left_nav_rail")
            .resizable(false)
            .exact_width(left_rail_w)
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_app)
                    .inner_margin(Margin::symmetric(if is_narrow_w { 4.0 } else { 8.0 }, 14.0))
                    .stroke(Stroke::new(1.0_f32, pal.border)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // User Avatar Button with Cutout Online Status Dot
                    let (av_rect, av_resp) =
                        ui.allocate_exact_size(Vec2::new(40.0, 40.0), Sense::click());
                    let user_disp = if self.user_name.is_empty() {
                        "Người dùng Zalo"
                    } else {
                        &self.user_name
                    };
                    paint_user_avatar(
                        ui.painter(),
                        av_rect.center(),
                        20.0,
                        user_disp,
                        false,
                        self.user_avatar.as_deref(),
                        &self.sticker_textures,
                        &mut self.loading_stickers,
                    );
                    if av_resp.hovered() || self.profile_menu_open {
                        ui.painter().circle_stroke(
                            av_rect.center(),
                            21.0,
                            Stroke::new(2.0_f32, pal.primary_hover),
                        );
                    }
                    // Cutout status dot
                    if self.settings.show_online_status {
                        let dot_c = Pos2::new(av_rect.right() - 4.0, av_rect.bottom() - 4.0);
                        ui.painter().circle_filled(dot_c, 6.5, pal.bg_app);
                        ui.painter().circle_filled(dot_c, 4.5, pal.accent_online);
                    }

                    if av_resp
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text(format!(
                            "{} (Đang trực tuyến — Bấm để mở menu tài khoản)",
                            if self.user_name.is_empty() {
                                "Tài khoản Zalo"
                            } else {
                                &self.user_name
                            }
                        ))
                        .clicked()
                    {
                        self.profile_menu_open = !self.profile_menu_open;
                    }

                    ui.add_space(18.0);

                    // Navigation Icon 1: Tin nhắn (All / Active Chat)
                    let is_msg_nav =
                        !self.contacts_view_open && self.thread_filter == ThreadFilter::All;
                    let (nav1_rect, nav1_resp) =
                        ui.allocate_exact_size(Vec2::new(42.0, 42.0), Sense::click());
                    if is_msg_nav {
                        ui.painter().rect_filled(
                            nav1_rect,
                            Rounding::same(11.0),
                            pal.primary.linear_multiply(0.18),
                        );
                        let pill = Rect::from_min_size(
                            Pos2::new(nav1_rect.left() - 6.0, nav1_rect.center().y - 10.0),
                            Vec2::new(3.5, 20.0),
                        );
                        ui.painter()
                            .rect_filled(pill, Rounding::same(2.0), pal.primary);
                    } else if nav1_resp.hovered() {
                        ui.painter()
                            .rect_filled(nav1_rect, Rounding::same(11.0), pal.bg_hover);
                    }
                    draw_icon_chat_nav(
                        ui.painter(),
                        nav1_rect.center(),
                        19.0,
                        if is_msg_nav {
                            pal.primary_hover
                        } else {
                            pal.text_secondary
                        },
                    );
                    let nav_unread: usize = self.unread_counts.values().sum();
                    if nav_unread > 0 {
                        let badge_txt = if nav_unread > 99 {
                            "99+".to_string()
                        } else {
                            nav_unread.to_string()
                        };
                        let bw = if badge_txt.len() >= 3 {
                            24.0
                        } else if badge_txt.len() == 2 {
                            19.0
                        } else {
                            15.0
                        };
                        let badge_rect = Rect::from_center_size(
                            Pos2::new(nav1_rect.right() - 6.0, nav1_rect.top() + 7.0),
                            Vec2::new(bw, 15.0),
                        );
                        ui.painter().rect_filled(
                            badge_rect,
                            Rounding::same(7.5),
                            Color32::from_rgb(239, 56, 56),
                        );
                        ui.painter().rect_stroke(
                            badge_rect,
                            Rounding::same(7.5),
                            Stroke::new(1.2_f32, pal.bg_app),
                        );
                        ui.painter().text(
                            badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            badge_txt,
                            FontId::proportional(9.8),
                            Color32::WHITE,
                        );
                    }
                    if nav1_resp
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Tin nhắn")
                        .clicked()
                    {
                        self.contacts_view_open = false;
                        self.thread_filter = ThreadFilter::All;
                    }

                    ui.add_space(6.0);

                    // Navigation Icon 2: Danh bạ (Unified Friends + Groups + Invites matching Zalo PC)
                    let direct_cnt = self
                        .threads
                        .iter()
                        .filter(|t| !t.is_group && !t.name.contains("Cloud của tôi"))
                        .count();
                    let group_cnt = self.threads.iter().filter(|t| t.is_group).count();
                    let is_contacts_nav = self.contacts_view_open;
                    let (nav2_rect, nav2_resp) =
                        ui.allocate_exact_size(Vec2::new(42.0, 42.0), Sense::click());
                    if is_contacts_nav {
                        ui.painter().rect_filled(
                            nav2_rect,
                            Rounding::same(11.0),
                            pal.primary.linear_multiply(0.18),
                        );
                        let pill = Rect::from_min_size(
                            Pos2::new(nav2_rect.left() - 6.0, nav2_rect.center().y - 10.0),
                            Vec2::new(3.5, 20.0),
                        );
                        ui.painter()
                            .rect_filled(pill, Rounding::same(2.0), pal.primary);
                    } else if nav2_resp.hovered() {
                        ui.painter()
                            .rect_filled(nav2_rect, Rounding::same(11.0), pal.bg_hover);
                    }
                    draw_icon_contacts(
                        ui.painter(),
                        nav2_rect.center(),
                        19.0,
                        if is_contacts_nav {
                            pal.primary_hover
                        } else {
                            pal.text_secondary
                        },
                    );
                    if nav2_resp
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text(format!("Danh bạ ({direct_cnt} bạn bè · {group_cnt} nhóm)"))
                        .clicked()
                    {
                        self.contacts_view_open = !self.contacts_view_open;
                    }

                    ui.add_space(6.0);

                    // Navigation Icon 3: Cloud của tôi
                    let is_cloud_selected = !self.contacts_view_open
                        && self
                            .threads
                            .get(self.selected_thread_index)
                            .map(|t| t.name.contains("Cloud của tôi"))
                            .unwrap_or(false);
                    let (nav3_rect, nav3_resp) =
                        ui.allocate_exact_size(Vec2::new(42.0, 42.0), Sense::click());
                    if is_cloud_selected {
                        ui.painter().rect_filled(
                            nav3_rect,
                            Rounding::same(11.0),
                            pal.primary.linear_multiply(0.18),
                        );
                    } else if nav3_resp.hovered() {
                        ui.painter()
                            .rect_filled(nav3_rect, Rounding::same(11.0), pal.bg_hover);
                    }
                    draw_icon_cloud(
                        ui.painter(),
                        nav3_rect.center(),
                        19.0,
                        if is_cloud_selected {
                            pal.primary_hover
                        } else {
                            pal.text_secondary
                        },
                    );
                    if nav3_resp
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Cloud của tôi")
                        .clicked()
                    {
                        self.contacts_view_open = false;
                        if let Some((cidx, cthread)) = self
                            .threads
                            .iter()
                            .enumerate()
                            .find(|(_, t)| t.name.contains("Cloud của tôi"))
                        {
                            let cid = cthread.id.clone();
                            self.selected_thread_index = cidx;
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::LoadGroupHistory {
                                    group_id: cid,
                                    is_group: false,
                                    have: 0,
                                });
                            }
                        }
                    }

                    // Navigation Icon 4 (Nhật ký / Feed) hidden for minimal Linux client.
                    // Push bottom actions to the bottom of the Left Rail
                    let rem_h = (ui.available_height() - 100.0).max(12.0);
                    ui.add_space(rem_h);

                    // Bottom Action 1: Background Sync Indicator Icon
                    let (sync_rect, sync_resp) =
                        ui.allocate_exact_size(Vec2::new(40.0, 40.0), Sense::click());
                    if sync_resp.hovered() {
                        ui.painter()
                            .rect_filled(sync_rect, Rounding::same(10.0), pal.bg_hover);
                    }
                    draw_icon_refresh(
                        ui.painter(),
                        sync_rect.center(),
                        17.0,
                        if sync_resp.hovered() {
                            pal.accent_online
                        } else {
                            pal.text_secondary
                        },
                    );
                    ui.painter().circle_filled(
                        Pos2::new(sync_rect.right() - 9.0, sync_rect.top() + 9.0),
                        3.0,
                        pal.accent_online,
                    );
                    if sync_resp
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text(format!(
                            "{} Đồng bộ tin nhắn từ điện thoại & Làm mới (Bấm để đồng bộ) · {}",
                            lucide::REFRESH,
                            self.status_msg
                        ))
                        .clicked()
                    {
                        self.phone_sync_modal_open = true;
                        self.phone_sync_progress = 0.05;
                        self.phone_sync_status =
                            "Đang gửi yêu cầu đồng bộ tin nhắn tới điện thoại…".into();
                        let threads = self.threads.clone();
                        if let Some(tx) = WORKER_TX.get() {
                            let _ = tx.send(WorkerCmd::SyncPhoneMessages {
                                threads,
                                send_phone_prompt: true,
                            });
                        }
                    }

                    ui.add_space(6.0);

                    // Bottom Action 2: Theme / Settings Icon
                    // Bottom Action 2: Settings Modal Button
                    let has_upd_badge = self
                        .update_info
                        .as_ref()
                        .map(|u| u.has_update)
                        .unwrap_or(false);
                    let (set_rect, set_resp) =
                        ui.allocate_exact_size(Vec2::new(40.0, 40.0), Sense::click());
                    if self.settings_modal_open {
                        ui.painter().rect_filled(
                            set_rect,
                            Rounding::same(10.0),
                            pal.primary.linear_multiply(0.18),
                        );
                    } else if set_resp.hovered() {
                        ui.painter()
                            .rect_filled(set_rect, Rounding::same(10.0), pal.bg_hover);
                    }
                    draw_icon_settings(
                        ui.painter(),
                        set_rect.center(),
                        17.5,
                        if self.settings_modal_open {
                            pal.primary_hover
                        } else if set_resp.hovered() {
                            pal.text_primary
                        } else {
                            pal.text_secondary
                        },
                    );
                    if has_upd_badge {
                        ui.painter().circle_filled(
                            Pos2::new(set_rect.right() - 8.0, set_rect.top() + 8.0),
                            4.2,
                            pal.accent_online,
                        );
                    }
                    let set_tip = if let Some(ref u) = self.update_info {
                        if u.has_update {
                            format!("Cài đặt Zalo (Có bản cập nhật mới v{})", u.latest_version)
                        } else {
                            format!("Cài đặt Zalo (v{})", env!("CARGO_PKG_VERSION"))
                        }
                    } else {
                        format!("Cài đặt Zalo (v{})", env!("CARGO_PKG_VERSION"))
                    };
                    if set_resp
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text(set_tip)
                        .clicked()
                    {
                        self.settings_modal_open = !self.settings_modal_open;
                        if self.settings_modal_open && has_upd_badge {
                            self.settings_tab = SettingsTab::Update;
                        }
                    }
                });
            });
    }
}

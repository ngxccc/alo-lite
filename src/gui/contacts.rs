//! Contacts directory page (Danh bạ bạn bè, nhóm và cộng đồng).

use crate::gui::theme::{
    draw_icon_contacts, draw_icon_search, draw_icon_user_plus, draw_lucide_icon, fold_vn_search,
    lucide, ThemePalette,
};
use crate::gui::worker::{WorkerCmd, WORKER_TX};
use crate::gui::{ContactsSubTab, ThreadFilter, ZaloGuiApp};
use eframe::egui::{
    self, Color32, CursorIcon, FontId, Id, Margin, Pos2, Rect, RichText, Rounding, Sense, Stroke,
    Vec2,
};

fn matches_vn_query(name: &str, q_low: &str, q_fold: &str) -> bool {
    if q_low.is_empty() {
        return true;
    }
    let n_low = name.to_lowercase();
    n_low.contains(q_low) || fold_vn_search(&n_low).contains(q_fold)
}

impl ZaloGuiApp {
    /// Renders the full Contacts view (Danh bạ).
    pub(super) fn render_contacts_page(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        // 1. Left Contacts Menu Sidebar (300px, matching Zalo PC Danh bạ screenshot)
        egui::SidePanel::left("contacts_left_sidebar")
            .resizable(false)
            .exact_width(300.0)
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_sidebar)
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .inner_margin(Margin::symmetric(12.0, 14.0)),
            )
            .show(ctx, |ui| {
                // Search bar + Add friend / Create group buttons
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let sbox_w = (ui.available_width() - 74.0).max(120.0);
                    let (sbox_rect, _) =
                        ui.allocate_exact_size(Vec2::new(sbox_w, 32.0), Sense::hover());
                    let has_q = !self.contacts_search_query.is_empty();
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
                        13.5,
                        if has_q {
                            pal.primary_hover
                        } else {
                            pal.text_muted
                        },
                    );
                    let edit_rect = Rect::from_min_max(
                        Pos2::new(sbox_rect.left() + 28.0, sbox_rect.top() + 5.0),
                        Pos2::new(sbox_rect.right() - 8.0, sbox_rect.bottom() - 5.0),
                    );
                    ui.put(
                        edit_rect,
                        egui::TextEdit::singleline(&mut self.contacts_search_query)
                            .id(Id::new("contacts_sidebar_search"))
                            .frame(false)
                            .hint_text("Tìm kiếm")
                            .desired_width(edit_rect.width()),
                    );

                    let (add_r, add_resp) =
                        ui.allocate_exact_size(Vec2::new(31.0, 32.0), Sense::click());
                    if add_resp.hovered() {
                        ui.painter()
                            .rect_filled(add_r, Rounding::same(7.0), pal.bg_hover);
                    }
                    draw_icon_user_plus(
                        ui.painter(),
                        add_r.center(),
                        16.5,
                        if add_resp.hovered() {
                            pal.primary_hover
                        } else {
                            pal.text_secondary
                        },
                    );
                    if add_resp
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Thêm bạn qua số điện thoại")
                        .clicked()
                    {
                        self.phone_modal_open = true;
                    }

                    let (grp_r, grp_resp) =
                        ui.allocate_exact_size(Vec2::new(31.0, 32.0), Sense::click());
                    if grp_resp.hovered() {
                        ui.painter()
                            .rect_filled(grp_r, Rounding::same(7.0), pal.bg_hover);
                    }
                    draw_icon_contacts(
                        ui.painter(),
                        grp_r.center(),
                        16.5,
                        if grp_resp.hovered() {
                            pal.primary_hover
                        } else {
                            pal.text_secondary
                        },
                    );
                    if grp_resp
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Danh sách nhóm và cộng đồng")
                        .clicked()
                    {
                        self.contacts_sub_tab = ContactsSubTab::Groups;
                    }
                });

                ui.add_space(12.0);

                let active_row_bg = if pal.is_dark {
                    Color32::from_rgb(28, 56, 98)
                } else {
                    Color32::from_rgb(229, 241, 255)
                };

                for (tab, icon, label) in [
                    (ContactsSubTab::Friends, lucide::USER, "Danh sách bạn bè"),
                    (
                        ContactsSubTab::Groups,
                        lucide::USERS,
                        "Danh sách nhóm và cộng đồng",
                    ),
                    (
                        ContactsSubTab::FriendRequests,
                        lucide::USER_PLUS,
                        "Lời mời kết bạn",
                    ),
                    (
                        ContactsSubTab::GroupInvites,
                        lucide::USERS,
                        "Lời mời vào nhóm và cộng đồng",
                    ),
                ] {
                    let active = self.contacts_sub_tab == tab;
                    let (row_r, row_resp) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::click());
                    if active {
                        ui.painter()
                            .rect_filled(row_r, Rounding::same(8.0), active_row_bg);
                    } else if row_resp.hovered() {
                        ui.painter()
                            .rect_filled(row_r, Rounding::same(8.0), pal.bg_hover);
                    }
                    draw_lucide_icon(
                        ui.painter(),
                        Pos2::new(row_r.left() + 20.0, row_r.center().y),
                        icon,
                        17.0,
                        if active {
                            Color32::WHITE
                        } else {
                            pal.text_secondary
                        },
                    );
                    ui.painter().text(
                        Pos2::new(row_r.left() + 44.0, row_r.center().y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        FontId::proportional(13.5),
                        if active {
                            Color32::WHITE
                        } else {
                            pal.text_primary
                        },
                    );
                    if row_resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        self.contacts_sub_tab = tab;
                    }
                    ui.add_space(4.0);
                }
            });

        // 2. Main Central Directory Panel
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(pal.bg_chat_canvas))
            .show(ctx, |ui| {
                let (header_icon, header_title) = match self.contacts_sub_tab {
                    ContactsSubTab::Friends => (lucide::USER, "Danh sách bạn bè"),
                    ContactsSubTab::Groups => (lucide::USERS, "Danh sách nhóm và cộng đồng"),
                    ContactsSubTab::FriendRequests => (lucide::USER_PLUS, "Lời mời kết bạn"),
                    ContactsSubTab::GroupInvites => {
                        (lucide::USERS, "Lời mời vào nhóm và cộng đồng")
                    }
                };

                // Full-width Top Header Bar
                let (hdr_rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), Sense::hover());
                ui.painter()
                    .rect_filled(hdr_rect, Rounding::ZERO, pal.bg_sidebar);
                ui.painter().line_segment(
                    [hdr_rect.left_bottom(), hdr_rect.right_bottom()],
                    Stroke::new(1.0_f32, pal.border),
                );
                draw_lucide_icon(
                    ui.painter(),
                    Pos2::new(hdr_rect.left() + 30.0, hdr_rect.center().y),
                    header_icon,
                    18.5,
                    pal.text_primary,
                );
                ui.painter().text(
                    Pos2::new(hdr_rect.left() + 54.0, hdr_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    header_title,
                    FontId::proportional(15.5),
                    pal.text_primary,
                );

                let q_low = self.contacts_search_query.trim().to_lowercase();
                let q_fold = fold_vn_search(&q_low);
                let mut open_thread_id: Option<(String, bool)> = None;

                egui::ScrollArea::vertical()
                    .id_salt(("contacts_main_scroll", self.contacts_sub_tab as u8))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::none()
                            .inner_margin(Margin::symmetric(24.0, 18.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                match self.contacts_sub_tab {
                                    ContactsSubTab::Friends | ContactsSubTab::Groups => {
                                        let want_group =
                                            self.contacts_sub_tab == ContactsSubTab::Groups;
                                        let mut items: Vec<(String, String, Option<String>, bool)> =
                                            self.threads
                                                .iter()
                                                .filter(|t| {
                                                    t.is_group == want_group
                                                        && !t.name.contains("Cloud của tôi")
                                                        && matches_vn_query(
                                                            &t.name, &q_low, &q_fold,
                                                        )
                                                })
                                                .map(|t| {
                                                    (
                                                        t.id.clone(),
                                                        t.name.clone(),
                                                        t.avatar.clone(),
                                                        t.is_group,
                                                    )
                                                })
                                                .collect();
                                        items.sort_by(|a, b| {
                                            fold_vn_search(&a.1).cmp(&fold_vn_search(&b.1))
                                        });

                                        ui.label(
                                            RichText::new(if want_group {
                                                format!("Nhóm và cộng đồng ({})", items.len())
                                            } else {
                                                format!("Bạn bè ({})", items.len())
                                            })
                                            .size(13.5)
                                            .color(pal.text_primary)
                                            .strong(),
                                        );
                                        ui.add_space(10.0);

                                        egui::Frame::none()
                                            .fill(pal.bg_sidebar)
                                            .stroke(Stroke::new(1.0_f32, pal.border))
                                            .rounding(Rounding::same(10.0))
                                            .inner_margin(Margin::symmetric(20.0, 16.0))
                                            .show(ui, |ui| {
                                                let has_q = !self.contacts_search_query.is_empty();
                                                if items.is_empty() {
                                                    ui.add_space(20.0);
                                                    ui.vertical_centered(|ui| {
                                                        ui.label(
                                                            RichText::new(if has_q {
                                                                "Không tìm thấy kết quả phù hợp"
                                                            } else if want_group {
                                                                "Chưa có nhóm nào"
                                                            } else {
                                                                "Chưa có bạn bè nào"
                                                            })
                                                            .size(13.0)
                                                            .color(pal.text_muted),
                                                        );
                                                    });
                                                    ui.add_space(20.0);
                                                } else {
                                                    for (id, name, _avatar, is_grp) in items {
                                                        let (row_r, row_resp) = ui
                                                            .allocate_exact_size(
                                                                Vec2::new(
                                                                    ui.available_width(),
                                                                    48.0,
                                                                ),
                                                                Sense::click(),
                                                            );
                                                        if row_resp.hovered() {
                                                            ui.painter().rect_filled(
                                                                row_r,
                                                                Rounding::same(6.0),
                                                                pal.bg_hover,
                                                            );
                                                        }
                                                        let av_r = Rect::from_center_size(
                                                            Pos2::new(
                                                                row_r.left() + 26.0,
                                                                row_r.center().y,
                                                            ),
                                                            Vec2::splat(34.0),
                                                        );
                                                        ui.painter().rect_filled(
                                                            av_r,
                                                            Rounding::same(17.0),
                                                            if is_grp {
                                                                pal.primary
                                                            } else {
                                                                pal.bg_elevated
                                                            },
                                                        );
                                                        let initial = name
                                                            .chars()
                                                            .next()
                                                            .unwrap_or('?')
                                                            .to_uppercase()
                                                            .to_string();
                                                        ui.painter().text(
                                                            av_r.center(),
                                                            egui::Align2::CENTER_CENTER,
                                                            initial,
                                                            FontId::proportional(14.0),
                                                            if is_grp {
                                                                Color32::WHITE
                                                            } else {
                                                                pal.text_primary
                                                            },
                                                        );

                                                        ui.painter().text(
                                                            Pos2::new(
                                                                row_r.left() + 54.0,
                                                                row_r.center().y,
                                                            ),
                                                            egui::Align2::LEFT_CENTER,
                                                            &name,
                                                            FontId::proportional(13.5),
                                                            pal.text_primary,
                                                        );

                                                        if row_resp.clicked() {
                                                            open_thread_id = Some((id, is_grp));
                                                        }
                                                    }
                                                }
                                            });
                                    }
                                    ContactsSubTab::FriendRequests
                                    | ContactsSubTab::GroupInvites => {
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(40.0);
                                            ui.label(
                                                RichText::new("Chưa có lời mời mới")
                                                    .size(14.0)
                                                    .color(pal.text_muted),
                                            );
                                            ui.add_space(12.0);
                                            if ui
                                                .button(
                                                    RichText::new(format!(
                                                        "{} Tìm kiếm bạn mới",
                                                        lucide::SEARCH
                                                    ))
                                                    .size(12.5),
                                                )
                                                .clicked()
                                            {
                                                self.phone_modal_open = true;
                                            }
                                        });
                                    }
                                }
                            });
                    });

                if let Some((tid, is_grp)) = open_thread_id {
                    if let Some(idx) = self.threads.iter().position(|t| t.id == tid) {
                        self.selected_thread_index = idx;
                        self.contacts_view_open = false;
                        self.thread_filter = ThreadFilter::All;
                        if is_grp {
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::LoadGroupHistory {
                                    group_id: tid,
                                    is_group: true,
                                    have: 0,
                                });
                            }
                        }
                    }
                }
            });
    }
}

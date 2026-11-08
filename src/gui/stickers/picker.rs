use super::packs::*;
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_sticker_picker(
        &mut self,
        ui: &mut egui::Ui,
        thread_id: &str,
        is_group: bool,
        pal: &ThemePalette,
    ) {
        // Trigger background store catalog load once when drawer opens
        if self.sticker_store_packs.is_empty() && !self.sticker_store_loading {
            self.sticker_store_loading = true;
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::FetchStickerStore);
            }
        }

        egui::Frame::none()
            .fill(pal.bg_sidebar)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .inner_margin(Margin::symmetric(14.0, 8.0))
            .show(ui, |ui| {
                ui.set_height(248.0);

                // 1. Top Pack & Store Navigation Bar
                ui.horizontal(|ui| {
                    let store_count_lbl = if !self.sticker_store_packs.is_empty() {
                        format!("{} Kho Nhãn Dán ({})", lucide::STORE, self.sticker_store_packs.len())
                    } else {
                        format!("{} Kho Nhãn Dán", lucide::STORE)
                    };

                    for (tab, title) in [
                        (StickerTab::Popular, format!("{} Phổ biến", lucide::FLAME)),
                        (StickerTab::Onion, "Củ hành (30)".to_string()),
                        (StickerTab::Search, format!("{} Tìm nhãn", lucide::SEARCH)),
                        (StickerTab::Store, store_count_lbl),
                    ] {
                        let active = self.sticker_tab == tab;
                        let btn = egui::Button::new(
                            RichText::new(title)
                                .size(12.2)
                                .color(if active {
                                    Color32::WHITE
                                } else {
                                    pal.text_primary
                                })
                                .strong(),
                        )
                        .fill(if active { pal.primary } else { pal.bg_input })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if active { pal.primary } else { pal.border },
                        ))
                        .rounding(Rounding::same(6.0));
                        if ui
                            .add(btn)
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.sticker_tab = tab;
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .small_button(format!("{} Đóng", lucide::X))
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.sticker_picker_open = false;
                        }
                    });
                });

                ui.add_space(3.0);

                // 1b. Pinned / Featured Pack Strip (10+ packs + any pack opened from the Store)
                egui::ScrollArea::horizontal()
                    .id_salt("sticker_pack_tabs_scroll")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            let pinned_snapshot = self.pinned_sticker_packs.clone();
                            for (cid, pname) in pinned_snapshot {
                                let tab = StickerTab::Pack(cid);
                                let active = self.sticker_tab == tab;
                                let btn = egui::Button::new(
                                    RichText::new(&pname)
                                        .size(11.8)
                                        .color(if active {
                                            Color32::WHITE
                                        } else {
                                            pal.text_secondary
                                        })
                                        .strong(),
                                )
                                .fill(if active {
                                    pal.primary
                                } else {
                                    pal.bg_elevated
                                })
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    if active { pal.primary } else { pal.border },
                                ))
                                .rounding(Rounding::same(6.0));
                                if ui
                                    .add(btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    self.sticker_tab = tab;
                                    if !self.sticker_pack_items.contains_key(&cid)
                                        && self.loading_sticker_packs.insert(cid)
                                    {
                                        if let Some(tx) = WORKER_TX.get() {
                                            let _ = tx.send(WorkerCmd::FetchStickerPack {
                                                cate_id: cid,
                                                name: pname,
                                            });
                                        }
                                    }
                                }
                            }
                        });
                    });

                ui.add_space(4.0);

                // 2. Tab Body
                match self.sticker_tab {
                    StickerTab::Store => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Lọc bộ nhãn:")
                                    .size(12.0)
                                    .color(pal.text_secondary),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.sticker_store_filter)
                                    .hint_text("Tìm trong 390+ bộ nhãn Zalo (VD: Zapy, Mèo, Gấu, Tết, Cò Lõ…)")
                                    .desired_width(340.0),
                            );
                            if !self.sticker_store_filter.is_empty()
                                && ui.small_button("Xóa").clicked()
                            {
                                self.sticker_store_filter.clear();
                            }
                            if ui
                                .small_button(format!("{} Tải lại kho", lucide::REFRESH))
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.sticker_store_loading = true;
                                if let Some(tx) = WORKER_TX.get() {
                                    let _ = tx.send(WorkerCmd::FetchStickerStore);
                                }
                            }
                        });
                        ui.add_space(4.0);

                        if self.sticker_store_loading && self.sticker_store_packs.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(36.0);
                                ui.spinner();
                                ui.label("Đang tải danh mục Kho Nhãn Dán Zalo…");
                            });
                            return;
                        }

                        let q = self.sticker_store_filter.trim().to_lowercase();
                        let packs_snapshot: Vec<crate::protocol::messaging::StickerPackInfo> = self
                            .sticker_store_packs
                            .iter()
                            .filter(|p| q.is_empty() || p.name.to_lowercase().contains(&q))
                            .take(120)
                            .cloned()
                            .collect();

                        let mut open_pack: Option<(u64, String)> = None;
                        egui::ScrollArea::vertical()
                            .id_salt("sticker_store_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                                    for pack in &packs_snapshot {
                                        let icon_key = format!("pack_ic_{}", pack.id);
                                        if !pack.icon_url.is_empty()
                                            && !self.sticker_textures.contains_key(&icon_key)
                                            && self.loading_stickers.insert(icon_key.clone())
                                        {
                                            if let Some(tx) = WORKER_TX.get() {
                                                let _ = tx.send(WorkerCmd::FetchStickerImage {
                                                    key: icon_key.clone(),
                                                    url: pack.icon_url.clone(),
                                                });
                                            }
                                        }

                                        let (card_rect, card_resp) = ui.allocate_exact_size(
                                            Vec2::new(195.0, 56.0),
                                            Sense::click(),
                                        );
                                        let card_bg = if card_resp.hovered() {
                                            pal.bg_hover
                                        } else {
                                            pal.bg_elevated
                                        };
                                        let card_border = if card_resp.hovered() {
                                            pal.primary
                                        } else {
                                            pal.border
                                        };
                                        ui.painter().rect_filled(
                                            card_rect,
                                            Rounding::same(8.0),
                                            card_bg,
                                        );
                                        ui.painter().rect_stroke(
                                            card_rect,
                                            Rounding::same(8.0),
                                            Stroke::new(1.0_f32, card_border),
                                        );

                                        let ic_rect = Rect::from_min_size(
                                            Pos2::new(card_rect.left() + 6.0, card_rect.top() + 6.0),
                                            Vec2::splat(44.0),
                                        );
                                        if let Some(tex) = self.sticker_textures.get(&icon_key) {
                                            ui.painter().image(
                                                tex.id(),
                                                ic_rect,
                                                Rect::from_min_max(
                                                    Pos2::ZERO,
                                                    Pos2::new(1.0, 1.0),
                                                ),
                                                Color32::WHITE,
                                            );
                                        } else {
                                            ui.painter().rect_filled(
                                                ic_rect,
                                                Rounding::same(6.0),
                                                pal.primary.linear_multiply(0.15),
                                            );
                                            draw_icon_sticker(
                                                ui.painter(),
                                                ic_rect.center(),
                                                18.0,
                                                pal.primary,
                                            );
                                        }

                                        ui.painter().text(
                                            Pos2::new(
                                                card_rect.left() + 56.0,
                                                card_rect.top() + 19.0,
                                            ),
                                            egui::Align2::LEFT_CENTER,
                                            truncate_chars(&pack.name, 18),
                                            FontId::proportional(12.5),
                                            pal.text_primary,
                                        );
                                        ui.painter().text(
                                            Pos2::new(
                                                card_rect.left() + 56.0,
                                                card_rect.top() + 38.0,
                                            ),
                                            egui::Align2::LEFT_CENTER,
                                            format!("{} nhãn · Bấm để mở", pack.total_image),
                                            FontId::proportional(11.0),
                                            pal.primary,
                                        );

                                        if card_resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .on_hover_text(format!(
                                                "Mở bộ nhãn \"{}\" (ID: {})",
                                                pack.name, pack.id
                                            ))
                                            .clicked()
                                        {
                                            open_pack = Some((pack.id, pack.name.clone()));
                                        }
                                    }
                                });
                            });

                        if let Some((cid, pname)) = open_pack {
                            if !self.pinned_sticker_packs.iter().any(|(id, _)| *id == cid) {
                                self.pinned_sticker_packs.insert(0, (cid, pname.clone()));
                            }
                            self.sticker_tab = StickerTab::Pack(cid);
                            if !self.sticker_pack_items.contains_key(&cid)
                                && self.loading_sticker_packs.insert(cid)
                            {
                                if let Some(tx) = WORKER_TX.get() {
                                    let _ = tx.send(WorkerCmd::FetchStickerPack {
                                        cate_id: cid,
                                        name: pname,
                                    });
                                }
                            }
                        }
                    }
                    StickerTab::Search => {
                        let mut trigger_kw: Option<String> = None;
                        ui.horizontal(|ui| {
                            let te = egui::TextEdit::singleline(&mut self.sticker_search_query)
                                .hint_text("Gõ để tự động tìm nhãn dán (xin chào, cảm ơn, khóc, nhớ, cố lên, mệt quá…)")
                                .desired_width(250.0);
                            let resp = ui.add(te);
                            if resp.changed() {
                                self.last_sticker_search_edit_at = std::time::Instant::now();
                            }
                            let enter = (resp.lost_focus() || resp.has_focus())
                                && ui.input(|i| i.key_pressed(Key::Enter));
                            if (ui
                                .button(format!("{} Tìm", lucide::SEARCH))
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                                || enter)
                                && !self.sticker_search_query.trim().is_empty()
                            {
                                trigger_kw = Some(self.sticker_search_query.trim().to_string());
                            }

                            for chip in [
                                "xin chào",
                                "cảm ơn",
                                "haha",
                                "ok",
                                "yêu",
                                "buồn",
                                "khóc",
                                "nhớ",
                                "cố lên",
                                "mệt quá",
                                "ngủ ngon",
                                "chúc mừng",
                            ] {
                                if ui
                                    .small_button(chip)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    self.sticker_search_query = chip.to_string();
                                    trigger_kw = Some(chip.to_string());
                                }
                            }
                        });

                        let q_trimmed = self.sticker_search_query.trim();
                        if trigger_kw.is_none()
                            && !q_trimmed.is_empty()
                            && q_trimmed != self.last_sticker_search_dispatched
                        {
                            let elapsed = self.last_sticker_search_edit_at.elapsed();
                            let debounce = Duration::from_millis(200);
                            if elapsed >= debounce {
                                trigger_kw = Some(q_trimmed.to_string());
                            } else {
                                ui.ctx().request_repaint_after(
                                    debounce - elapsed + Duration::from_millis(10),
                                );
                            }
                        }

                        if let Some(kw) = trigger_kw {
                            self.last_sticker_search_dispatched = kw.clone();
                            self.sticker_search_loading = true;
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::SearchStickers { keyword: kw });
                            }
                        }
                        ui.add_space(4.0);
                        let items = self.sticker_search_results.clone();
                        if self.sticker_search_loading {
                            ui.vertical_centered(|ui| {
                                ui.add_space(28.0);
                                ui.spinner();
                                ui.label("Đang tìm nhãn dán phù hợp…");
                            });
                        } else if items.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(28.0);
                                ui.label(
                                    RichText::new(
                                        "Chọn một từ khóa gợi ý bên trên hoặc nhập từ khóa để tìm nhãn dán Zalo.",
                                    )
                                    .size(12.5)
                                    .color(pal.text_muted),
                                );
                            });
                        } else {
                            self.render_sticker_items_grid(ui, &items, thread_id, is_group, pal);
                        }
                    }
                    StickerTab::Popular | StickerTab::Onion => {
                        let presets = if self.sticker_tab == StickerTab::Popular {
                            POPULAR_STICKERS
                        } else {
                            ONION_STICKERS
                        };
                        let items: Vec<crate::protocol::messaging::StickerItem> = presets
                            .iter()
                            .map(|p| crate::protocol::messaging::StickerItem {
                                sticker_id: p.sticker_id,
                                cate_id: p.cate_id,
                                sticker_type: p.sticker_type,
                                label: p.label.to_string(),
                            })
                            .collect();
                        self.render_sticker_items_grid(ui, &items, thread_id, is_group, pal);
                    }
                    StickerTab::Pack(cid) => {
                        if let Some(items) = self.sticker_pack_items.get(&cid).cloned() {
                            self.render_sticker_items_grid(ui, &items, thread_id, is_group, pal);
                        } else {
                            ui.vertical_centered(|ui| {
                                ui.add_space(36.0);
                                ui.spinner();
                                ui.label("Đang tải bộ nhãn dán từ máy chủ Zalo…");
                            });
                        }
                    }
                }
            });
    }

    pub(in crate::gui) fn render_sticker_items_grid(
        &mut self,
        ui: &mut egui::Ui,
        items: &[crate::protocol::messaging::StickerItem],
        thread_id: &str,
        is_group: bool,
        pal: &ThemePalette,
    ) {
        egui::ScrollArea::vertical()
            .id_salt(("sticker_items_grid", self.sticker_tab_salt()))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for item in items {
                        let key = format!("stk_{}_{}", item.cate_id, item.sticker_id);
                        let url = if item.cate_id == 0 && (1..=30).contains(&item.sticker_id) {
                            format!(
                                "https://stc-chat.zdn.vn/images/stickers/default/thumb/{}.png",
                                item.sticker_id
                            )
                        } else {
                            format!(
                                "https://api.zaloapp.com/api/emoticon/sticker/webpc?eid={}&size=130",
                                item.sticker_id
                            )
                        };

                        if !self.sticker_textures.contains_key(&key)
                            && self.loading_stickers.insert(key.clone())
                        {
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::FetchStickerImage {
                                    key: key.clone(),
                                    url,
                                });
                            }
                        }

                        let (cell_rect, cell_resp) =
                            ui.allocate_exact_size(Vec2::new(68.0, 68.0), Sense::click());
                        if cell_resp.hovered() {
                            ui.painter().rect_filled(
                                cell_rect,
                                Rounding::same(8.0),
                                pal.bg_hover,
                            );
                            ui.painter().rect_stroke(
                                cell_rect,
                                Rounding::same(8.0),
                                Stroke::new(1.0_f32, pal.primary),
                            );
                        }

                        if let Some(tex) = self.sticker_textures.get(&key) {
                            let img_rect = cell_rect.shrink(4.0);
                            ui.painter().image(
                                tex.id(),
                                img_rect,
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        } else {
                            ui.painter().text(
                                cell_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                truncate_chars(&item.label, 10),
                                FontId::proportional(10.5),
                                pal.text_secondary,
                            );
                        }

                        if cell_resp
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .on_hover_text(format!(
                                "Gửi nhãn dán: {} (ID: {})",
                                item.label, item.sticker_id
                            ))
                            .clicked()
                        {
                            self.sticker_picker_open = false;
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::SendSticker {
                                    to_id: thread_id.to_string(),
                                    sticker_id: item.sticker_id,
                                    cate_id: item.cate_id,
                                    sticker_type: item.sticker_type,
                                    is_group,
                                });
                            }
                        }
                    }
                });
            });
    }

    pub fn sticker_tab_salt(&self) -> u64 {
        match self.sticker_tab {
            StickerTab::Popular => 1,
            StickerTab::Onion => 2,
            StickerTab::Search => 3,
            StickerTab::Store => 4,
            StickerTab::Pack(cid) => 1000 + cid,
        }
    }
}

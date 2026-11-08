use super::packs::*;
use super::search::*;
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub fn resolve_contextual_sticker_suggestions(
        &mut self,
        ctx: &egui::Context,
    ) -> Vec<crate::protocol::messaging::StickerItem> {
        if !self.settings.suggest_stickers_while_typing
            || !self.settings.enable_sticker_keyword_search
        {
            self.composer_sticker_selected = None;
            return Vec::new();
        }
        if self.input_message != self.last_composer_text {
            self.last_composer_text = self.input_message.clone();
            self.last_composer_edit_at = std::time::Instant::now();
            self.composer_sticker_selected = None;
        }

        let trimmed = self.input_message.trim();
        if trimmed.is_empty() {
            self.dismissed_sticker_suggest_for.clear();
            self.composer_sticker_selected = None;
            return Vec::new();
        }
        if trimmed.contains('\n')
            || trimmed.starts_with('#')
            || trimmed.starts_with('@')
            || trimmed.starts_with('/')
            || trimmed.split_whitespace().count() > 5
            || trimmed.chars().count() > 36
        {
            self.composer_sticker_selected = None;
            return Vec::new();
        }
        if self.dismissed_sticker_suggest_for == trimmed {
            return Vec::new();
        }

        let has_alpha = trimmed.chars().any(|c| c.is_alphabetic());
        let low = crate::protocol::messaging::stickers::normalize_sticker_keyword(trimmed);
        let folded = fold_vn_search(&low);
        let instant_match = instant_contextual_stickers(trimmed);
        let word_count = low.split_whitespace().count();
        let char_count = low.chars().count();
        let digit_count = low.chars().filter(|c| c.is_ascii_digit()).count();
        let is_url_like = low.contains("://")
            || low.starts_with("www.")
            || low.contains(".com")
            || low.contains(".vn")
            || low.contains(".net");

        let query_kw = if !has_alpha {
            // Pure emoji / emoticon (`:)`, `🙂`, `😂`, `....`, `???`) -> map to canonical emotion keyword
            instant_match
                .as_ref()
                .map(|(api_kw, _)| (*api_kw).to_string())
        } else if !is_url_like
            && digit_count <= 2
            && (1..=5).contains(&word_count)
            && (2..=32).contains(&char_count)
        {
            // Alphabetic Vietnamese/English chat input: query the exact normalized phrase
            // so Zalo's live `/suggest/stickers` + `/search` APIs return specific stickers
            // for `"khóc"`, `"nhớ"`, `"cố lên"`, `"mệt quá"`, `"đi chơi"`, `"trà sữa"`, etc.
            Some(low.clone())
        } else if let Some((api_kw, _)) = &instant_match {
            Some((*api_kw).to_string())
        } else if self.sticker_suggest_keywords.contains(&low) {
            Some(low.clone())
        } else if !folded.is_empty() && self.sticker_suggest_keywords.contains(&folded) {
            Some(folded)
        } else {
            None
        };

        let Some(kw) = query_kw else {
            self.composer_sticker_selected = None;
            return Vec::new();
        };

        // Debounce alphabetic typing by 145ms before firing network requests so Vietnamese
        // Telex/VNI composition (`b-u-o-o-n-f` -> `"buồn"`) completes cleanly without API spam,
        // while already-cached keywords and pure emojis/emoticons render at 0ms.
        let already_cached = self.composer_sticker_cache.contains_key(&kw);
        if has_alpha && !already_cached {
            let elapsed = self.last_composer_edit_at.elapsed();
            let debounce = Duration::from_millis(145);
            if elapsed < debounce {
                ctx.request_repaint_after(debounce - elapsed + Duration::from_millis(8));
                return Vec::new();
            }
        }

        if !already_cached && self.composer_sticker_loading.insert(kw.clone()) {
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::SuggestComposerStickers {
                    keyword: kw.clone(),
                });
            }
        }

        const EXCLUDED_PACKS: &[u64] = &[
            0, 50, 10788, 11190, 11984, 11998, 12002, 12003, 12024, 12658,
        ];
        let mut out: Vec<crate::protocol::messaging::StickerItem> = Vec::new();
        let mut seen = HashSet::new();

        // Prioritize live Zalo API stickers first once loaded; use curated presets as instant 0ms
        // fallback while the live query is in flight or to top up sparse results.
        if let Some(api_items) = self.composer_sticker_cache.get(&kw) {
            for it in api_items.iter().take(18) {
                if !EXCLUDED_PACKS.contains(&it.cate_id) && seen.insert(it.sticker_id) {
                    out.push(it.clone());
                }
            }
        }
        if let Some((_, preset_items)) = instant_match {
            for it in preset_items {
                if !EXCLUDED_PACKS.contains(&it.cate_id) && seen.insert(it.sticker_id) {
                    out.push(it);
                }
            }
        }
        out.truncate(16);
        out
    }

    /// Render the floating contextual sticker suggestion popover (`egui::Area`) anchored directly
    /// above `composer_rect` (matching Zalo PC's `PopoverIdentitiesDefine.STICKER_SUGGESTION`).
    pub(in crate::gui) fn render_contextual_sticker_popover(
        &mut self,
        ctx: &egui::Context,
        composer_rect: Rect,
        items: &[crate::protocol::messaging::StickerItem],
        thread_id: &str,
        is_group: bool,
        pal: &ThemePalette,
    ) {
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.dismissed_sticker_suggest_for = self.input_message.trim().to_string();
            self.composer_sticker_selected = None;
            self.composer_focus_pending = true;
            return;
        }

        let strip_h = 80.0_f32;
        let strip_w = composer_rect.width().max(200.0);
        let pos = Pos2::new(
            composer_rect.left(),
            (composer_rect.top() - strip_h).max(48.0),
        );

        let mut send_item: Option<crate::protocol::messaging::StickerItem> = None;
        let mut dismiss_clicked = false;
        let mut open_search_clicked = false;
        let is_loading_live = !self.composer_sticker_loading.is_empty();
        let selected_idx = self.composer_sticker_selected;

        egui::Area::new(Id::new("zalo_contextual_sticker_popover"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(if pal.is_dark {
                        Color32::from_rgba_unmultiplied(33, 37, 43, 248)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 250)
                    })
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .rounding(Rounding {
                        nw: 12.0,
                        ne: 12.0,
                        sw: 0.0,
                        se: 0.0,
                    })
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, -4.0),
                        blur: 14.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(if pal.is_dark { 95 } else { 28 }),
                    })
                    .inner_margin(Margin::symmetric(14.0, 7.0))
                    .show(ui, |ui| {
                        let inner_w = (strip_w - 28.0).max(160.0);
                        ui.set_width(inner_w);
                        ui.set_max_height(64.0);
                        ui.horizontal_top(|ui| {
                            let scroll_w = (inner_w - 64.0).max(120.0);
                            let wheel_y = ui.input(|i| i.smooth_scroll_delta.y);
                            egui::ScrollArea::horizontal()
                                .id_salt("contextual_sticker_suggest_scroll")
                                .max_width(scroll_w)
                                .max_height(64.0)
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                                )
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    if wheel_y.abs() > 0.1
                                        && ui.rect_contains_pointer(ui.max_rect())
                                    {
                                        ui.scroll_with_delta(Vec2::new(wheel_y, 0.0));
                                    }
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 10.0;
                                        for (idx, item) in items.iter().enumerate() {
                                            let key = format!(
                                                "stk_{}_{}",
                                                item.cate_id, item.sticker_id
                                            );
                                            let url = if item.cate_id == 0
                                                && (1..=30).contains(&item.sticker_id)
                                            {
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

                                            let (cell_rect, cell_resp) = ui.allocate_exact_size(
                                                Vec2::new(62.0, 62.0),
                                                Sense::click(),
                                            );
                                            let hovered = cell_resp.hovered();
                                            let is_kb_sel = selected_idx == Some(idx);
                                            if hovered || is_kb_sel {
                                                ui.painter().rect_filled(
                                                    cell_rect,
                                                    Rounding::same(10.0),
                                                    if is_kb_sel {
                                                        pal.primary.linear_multiply(0.18)
                                                    } else {
                                                        pal.bg_hover
                                                    },
                                                );
                                                ui.painter().rect_stroke(
                                                    cell_rect,
                                                    Rounding::same(10.0),
                                                    Stroke::new(1.3_f32, pal.primary),
                                                );
                                            }

                                            if let Some(tex) = self.sticker_textures.get(&key) {
                                                let [tw, th] = tex.size();
                                                let tw_f = (tw.max(1)) as f32;
                                                let th_f = (th.max(1)) as f32;
                                                let box_side = if hovered || is_kb_sel {
                                                    58.0_f32
                                                } else {
                                                    54.0_f32
                                                };
                                                let scale =
                                                    (box_side / tw_f).min(box_side / th_f);
                                                let draw_size =
                                                    Vec2::new(tw_f * scale, th_f * scale);
                                                let img_rect = Rect::from_center_size(
                                                    cell_rect.center(),
                                                    draw_size,
                                                );
                                                ui.painter().image(
                                                    tex.id(),
                                                    img_rect,
                                                    Rect::from_min_max(
                                                        Pos2::ZERO,
                                                        Pos2::new(1.0, 1.0),
                                                    ),
                                                    Color32::WHITE,
                                                );
                                            } else {
                                                ui.painter().rect_filled(
                                                    cell_rect.shrink(6.0),
                                                    Rounding::same(8.0),
                                                    pal.bg_elevated,
                                                );
                                                ui.painter().text(
                                                    cell_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    truncate_chars(&item.label, 8),
                                                    FontId::proportional(9.5),
                                                    pal.text_muted,
                                                );
                                            }

                                            if cell_resp
                                                .on_hover_cursor(CursorIcon::PointingHand)
                                                .on_hover_text(
                                                    "Bấm để gửi nhãn dán (hoặc dùng phím ↑ ← → Enter)",
                                                )
                                                .clicked()
                                            {
                                                send_item = Some(item.clone());
                                            }
                                        }
                                    });
                                });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let (close_slot, close_resp) =
                                    ui.allocate_exact_size(Vec2::new(26.0, 62.0), Sense::click());
                                let close_center = close_slot.center();
                                if close_resp.hovered() {
                                    ui.painter()
                                        .circle_filled(close_center, 12.0, pal.bg_hover);
                                }
                                draw_lucide_icon(
                                    ui.painter(),
                                    close_center,
                                    lucide::X,
                                    13.5,
                                    if close_resp.hovered() {
                                        pal.text_primary
                                    } else {
                                        pal.text_muted
                                    },
                                );
                                if close_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Ẩn thanh gợi ý nhãn dán (Esc)")
                                    .clicked()
                                {
                                    dismiss_clicked = true;
                                }

                                let (more_slot, more_resp) =
                                    ui.allocate_exact_size(Vec2::new(26.0, 62.0), Sense::click());
                                let more_center = more_slot.center();
                                if more_resp.hovered() {
                                    ui.painter()
                                        .circle_filled(more_center, 12.0, pal.bg_hover);
                                }
                                if is_loading_live {
                                    ui.put(
                                        Rect::from_center_size(more_center, Vec2::new(14.0, 14.0)),
                                        egui::Spinner::new().size(13.0).color(pal.primary),
                                    );
                                } else {
                                    draw_lucide_icon(
                                        ui.painter(),
                                        more_center,
                                        lucide::SEARCH,
                                        13.0,
                                        if more_resp.hovered() {
                                            pal.primary
                                        } else {
                                            pal.text_muted
                                        },
                                    );
                                }
                                if more_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Xem thêm nhãn dán cho từ khóa này")
                                    .clicked()
                                {
                                    open_search_clicked = true;
                                }
                            });
                        });
                    });
            });

        if open_search_clicked {
            let q = crate::protocol::messaging::stickers::normalize_sticker_keyword(
                self.input_message.trim(),
            );
            if !q.is_empty() {
                self.sticker_picker_open = true;
                self.sticker_tab = StickerTab::Search;
                self.sticker_search_query = q.clone();
                self.last_sticker_search_dispatched = q.clone();
                self.sticker_search_loading = true;
                if let Some(tx) = WORKER_TX.get() {
                    let _ = tx.send(WorkerCmd::SearchStickers { keyword: q });
                }
            }
        }
        if dismiss_clicked {
            self.dismissed_sticker_suggest_for = self.input_message.trim().to_string();
            self.composer_sticker_selected = None;
            self.composer_focus_pending = true;
        }
        if let Some(chosen) = send_item {
            self.input_message.clear();
            self.dismissed_sticker_suggest_for.clear();
            self.composer_sticker_selected = None;
            self.composer_focus_pending = true;
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::SendSticker {
                    to_id: thread_id.to_string(),
                    sticker_id: chosen.sticker_id,
                    cate_id: chosen.cate_id,
                    sticker_type: chosen.sticker_type,
                    is_group,
                });
            }
        }
    }
}

use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui::{self, Rect};

/// Compute optimal Zoom-style gallery tile rectangles so each tile (`aspect` width/height)
/// has maximum area and the bottom row is horizontally centered.
pub(crate) fn compute_call_gallery_tiles(
    count: usize,
    rect: Rect,
    gap: f32,
    aspect: f32,
) -> Vec<Rect> {
    let total_w = rect.width();
    let total_h = rect.height();
    if count == 0 || total_w <= 0.0 || total_h <= 0.0 {
        return Vec::new();
    }
    let aspect = if aspect > 0.05 { aspect } else { 16.0 / 9.0 };
    let mut best: Option<(usize, usize, f32, f32, f32, usize)> = None;
    for cols in 1..=count {
        let rows = count.div_ceil(cols);
        let cell_w = (total_w - ((cols - 1) as f32) * gap) / (cols as f32);
        let cell_h = (total_h - ((rows - 1) as f32) * gap) / (rows as f32);
        if cell_w <= 0.0 || cell_h <= 0.0 {
            continue;
        }
        let w = cell_w.min(cell_h * aspect);
        let h = w / aspect;
        let area = w * h;
        let waste = rows * cols - count;
        let take = match best {
            None => true,
            Some((_, _, _, _, best_area, best_waste)) => {
                area > best_area + 0.5 || ((area - best_area).abs() <= 0.5 && waste < best_waste)
            }
        };
        if take {
            best = Some((cols, rows, w.floor(), h.floor(), area, waste));
        }
    }
    let Some((cols, rows, tw, th, _, _)) = best else {
        return Vec::new();
    };
    let grid_h = (rows as f32) * th + ((rows - 1) as f32) * gap;
    let y0 = rect.top() + ((total_h - grid_h) * 0.5).max(0.0).floor();
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let row = i / cols;
        let col = i % cols;
        let in_row = if row + 1 == rows {
            count - cols * (rows - 1)
        } else {
            cols
        };
        let row_w = (in_row as f32) * tw + ((in_row - 1) as f32) * gap;
        let x0 = rect.left() + ((total_w - row_w) * 0.5).max(0.0).floor();
        out.push(Rect::from_min_size(
            Pos2::new(
                x0 + (col as f32) * (tw + gap),
                y0 + (row as f32) * (th + gap),
            ),
            Vec2::new(tw, th),
        ));
    }
    out
}

/// Compute per-stream aspect-preserving tile rectangles (e.g. mobile portrait `9:16` next to PC landscape `16:9`).
pub(crate) fn compute_call_fit_aspects(aspects: &[f32], rect: Rect, gap: f32) -> Vec<Rect> {
    let box_w = rect.width();
    let box_h = rect.height();
    if aspects.is_empty() || box_w <= 0.0 || box_h <= 0.0 {
        return Vec::new();
    }
    let clean: Vec<f32> = aspects
        .iter()
        .map(|&a| if a > 0.05 { a } else { 16.0 / 9.0 })
        .collect();
    if clean.len() == 1 {
        let a = clean[0];
        let mut w = box_w.min(box_h * a);
        let mut h = w / a;
        if h > box_h {
            h = box_h;
            w = h * a;
        }
        return vec![Rect::from_center_size(
            rect.center(),
            Vec2::new(w.floor(), h.floor()),
        )];
    }
    if clean.len() == 2 {
        let (a0, a1) = (clean[0], clean[1]);
        if box_h * (a0 + a1) + gap <= box_w {
            let w0 = (box_h * a0).floor();
            let w1 = (box_h * a1).floor();
            let total_w = w0 + w1 + gap;
            let x0 = rect.left() + ((box_w - total_w) * 0.5).floor();
            return vec![
                Rect::from_min_size(Pos2::new(x0, rect.top()), Vec2::new(w0, box_h)),
                Rect::from_min_size(Pos2::new(x0 + w0 + gap, rect.top()), Vec2::new(w1, box_h)),
            ];
        }
        if box_w / a0 + box_w / a1 + gap <= box_h {
            let h0 = (box_w / a0).floor();
            let h1 = (box_w / a1).floor();
            let total_h = h0 + h1 + gap;
            let y0 = rect.top() + ((box_h - total_h) * 0.5).floor();
            return vec![
                Rect::from_min_size(Pos2::new(rect.left(), y0), Vec2::new(box_w, h0)),
                Rect::from_min_size(Pos2::new(rect.left(), y0 + h0 + gap), Vec2::new(box_w, h1)),
            ];
        }
        let sum_a = a0 + a1;
        let hh = ((box_w - gap) / sum_a).floor();
        let ww0 = (hh * a0).floor();
        let ww1 = (hh * a1).floor();
        let tw = ww0 + ww1 + gap;
        let xx = rect.left() + ((box_w - tw) * 0.5).max(0.0).floor();
        let yy = rect.top() + ((box_h - hh) * 0.5).max(0.0).floor();
        return vec![
            Rect::from_min_size(Pos2::new(xx, yy), Vec2::new(ww0, hh)),
            Rect::from_min_size(Pos2::new(xx + ww0 + gap, yy), Vec2::new(ww1, hh)),
        ];
    }
    let avg = clean.iter().sum::<f32>() / (clean.len() as f32);
    compute_call_gallery_tiles(clean.len(), rect, gap, avg)
}

/// Compute Speaker View layout: `(main_stage_rect, thumbnail_strip_rects)`.
pub(crate) fn compute_call_speaker_layout(
    strip_count: usize,
    rect: Rect,
    gap: f32,
    aspect: f32,
) -> (Rect, Vec<Rect>) {
    let box_w = rect.width();
    let box_h = rect.height();
    if strip_count == 0 || box_w <= 0.0 || box_h <= 0.0 {
        return (rect, Vec::new());
    }
    let aspect = if aspect > 0.05 { aspect } else { 16.0 / 9.0 };
    let wide = box_w >= box_h * 1.15;
    if wide {
        let strip_w = (box_w * 0.22).floor().clamp(140.0, 210.0);
        let tile_w = strip_w;
        let tile_h = (tile_w / aspect).floor();
        let fit = (((box_h + gap) / (tile_h + gap)).floor() as usize).max(1);
        let vis = strip_count.min(fit);
        let total_h = (vis as f32) * tile_h + ((vis.saturating_sub(1)) as f32) * gap;
        let y0 = rect.top() + ((box_h - total_h) * 0.5).max(0.0).floor();
        let mut strip = Vec::with_capacity(vis);
        for i in 0..vis {
            strip.push(Rect::from_min_size(
                Pos2::new(rect.right() - strip_w, y0 + (i as f32) * (tile_h + gap)),
                Vec2::new(tile_w, tile_h),
            ));
        }
        let main = Rect::from_min_size(
            rect.min,
            Vec2::new((box_w - strip_w - gap).max(120.0), box_h),
        );
        (main, strip)
    } else {
        let strip_h = (box_h * 0.20).floor().clamp(84.0, 128.0);
        let tile_h = strip_h;
        let tile_w = (tile_h * aspect).floor();
        let fit = (((box_w + gap) / (tile_w + gap)).floor() as usize).max(1);
        let vis = strip_count.min(fit);
        let total_w = (vis as f32) * tile_w + ((vis.saturating_sub(1)) as f32) * gap;
        let x0 = rect.left() + ((box_w - total_w) * 0.5).max(0.0).floor();
        let mut strip = Vec::with_capacity(vis);
        for i in 0..vis {
            strip.push(Rect::from_min_size(
                Pos2::new(x0 + (i as f32) * (tile_w + gap), rect.bottom() - strip_h),
                Vec2::new(tile_w, tile_h),
            ));
        }
        let main = Rect::from_min_size(
            rect.min,
            Vec2::new(box_w, (box_h - strip_h - gap).max(120.0)),
        );
        (main, strip)
    }
}

impl ZaloGuiApp {
    pub(in crate::gui) fn render_call_overlay(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        let Some(mut call) = self.active_call.clone() else {
            return;
        };
        // Keep timer and video textures updating smoothly while call window is open
        ctx.request_repaint_after(Duration::from_millis(66));

        let elapsed_f = call.started_at.elapsed().as_secs_f32();
        let elapsed = elapsed_f as u64;
        let mins = elapsed / 60;
        let secs = elapsed % 60;
        let timer_str = format!("{mins:02}:{secs:02}");

        let mut hangup_clicked = false;
        let mut retry_clicked = false;

        let has_err = call.error_reason.is_some();
        let is_video_stage = call.is_video && !has_err;
        let default_w = if call.is_video { 820.0 } else { 560.0 };
        let default_h = if call.is_video { 640.0 } else { 520.0 };
        let win_title = if call.is_video {
            format!("Zalo Video Call — {}", call.partner_name)
        } else {
            format!("Zalo Voice Call — {}", call.partner_name)
        };

        let call_ava = self
            .threads
            .iter()
            .find(|t| !call.partner_id.is_empty() && t.id == call.partner_id)
            .or_else(|| self.threads.iter().find(|t| t.name == call.partner_name))
            .and_then(|t| t.avatar.clone());

        // Extract remote participants for group call multi-tile view (excluding our own user name)
        let mut group_remotes: Vec<(String, Option<String>)> = Vec::new();
        if call.is_group || self.sticker_textures.contains_key("call_remote_video_1") {
            let my_low = self.user_name.trim().to_lowercase();
            let my_tokens: Vec<&str> = my_low.split_whitespace().collect();
            let is_me = |cand: &str| -> bool {
                let c_low = cand.trim().to_lowercase();
                if c_low.is_empty() || my_low.is_empty() {
                    return false;
                }
                c_low == my_low
                    || (c_low.split_whitespace().count() >= 2
                        && c_low.split_whitespace().all(|t| my_tokens.contains(&t)))
            };
            for part in call
                .partner_name
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                if !is_me(part) {
                    let p_low = part.to_lowercase();
                    let matched_ava = self
                        .threads
                        .iter()
                        .find(|t| !t.is_group && t.name.to_lowercase().contains(&p_low))
                        .and_then(|t| t.avatar.clone());
                    group_remotes.push((part.to_string(), matched_ava));
                }
            }
            if group_remotes.len() < 2 {
                if let Some(rest) = call_ava.as_deref().and_then(|s| s.strip_prefix("multi:")) {
                    for (idx, seg) in rest.split(';').enumerate() {
                        if idx == 0 && !seg.contains('|') {
                            continue;
                        }
                        if let Some((m_name, m_url)) = seg.split_once('|') {
                            let mn = m_name.trim();
                            if !mn.is_empty()
                                && !is_me(mn)
                                && !group_remotes.iter().any(|(n, _)| n == mn)
                            {
                                let url_opt = if m_url.trim().is_empty() {
                                    None
                                } else {
                                    Some(m_url.trim().to_string())
                                };
                                group_remotes.push((mn.to_string(), url_opt));
                            }
                        }
                    }
                }
            }
            while group_remotes.len() < 2 {
                let n = group_remotes.len() + 1;
                group_remotes.push((format!("Thành viên {n}"), None));
            }
        }
        let viewport_id = egui::ViewportId::from_hash_of("zalo_dedicated_call_window");
        let builder = egui::ViewportBuilder::default()
            .with_title(win_title)
            .with_inner_size([default_w, default_h])
            .with_min_inner_size([520.0, 460.0]);

        ctx.show_viewport_immediate(viewport_id, builder, |vp_ctx, _class| {
            if vp_ctx.input(|i| i.viewport().close_requested()) {
                hangup_clicked = true;
            }
            if vp_ctx.input(|i| i.key_pressed(Key::Escape)) && call.error_reason.is_some() {
                hangup_clicked = true;
            }

            let vp_w = vp_ctx.screen_rect().width();

            // 1. TOP TOOLBAR PANEL: Badge · Mic · Camera · Refresh (vertically centered, responsive width)
            egui::TopBottomPanel::top("call_vp_top_panel")
                .resizable(false)
                .exact_height(60.0)
                .show_separator_line(false)
                .frame(
                    egui::Frame::none()
                        .fill(pal.bg_sidebar)
                        .inner_margin(Margin {
                            left: 16.0,
                            right: 16.0,
                            top: 12.0,
                            bottom: 4.0,
                        }),
                )
                .show(vp_ctx, |ui| {
                    let max_bar_w = (vp_w - 32.0).max(320.0);
                    let badge_w = 132.0_f32;
                    let ref_w = 28.0_f32;
                    let gap = 8.0_f32;
                    let pad_x = 10.0_f32;
                    let combo_w = ((max_bar_w - badge_w - ref_w - gap * 3.0 - pad_x * 2.0) * 0.5)
                        .clamp(110.0, 195.0);
                    let bar_w = badge_w + combo_w * 2.0 + ref_w + gap * 3.0 + pad_x * 2.0;
                    let combo_chars = if combo_w < 145.0 { 13 } else { 20 };

                    ui.vertical_centered(|ui| {
                        let (bar_rect, _) =
                            ui.allocate_exact_size(Vec2::new(bar_w, 40.0), Sense::hover());
                        ui.painter()
                            .rect_filled(bar_rect, Rounding::same(14.0), pal.bg_elevated);
                        ui.painter().rect_stroke(
                            bar_rect,
                            Rounding::same(14.0),
                            Stroke::new(1.0_f32, pal.border),
                        );

                        let item_cy = bar_rect.center().y;
                        let item_h = 28.0_f32;
                        let x0 = bar_rect.left() + pad_x;
                        let x1 = x0 + badge_w + gap;
                        let x2 = x1 + combo_w + gap;
                        let x3 = x2 + combo_w + gap;

                        let badge_rect = Rect::from_center_size(
                            Pos2::new(x0 + badge_w * 0.5, item_cy),
                            Vec2::new(badge_w, item_h),
                        );
                        let mic_rect = Rect::from_center_size(
                            Pos2::new(x1 + combo_w * 0.5, item_cy),
                            Vec2::new(combo_w, item_h),
                        );
                        let cam_rect = Rect::from_center_size(
                            Pos2::new(x2 + combo_w * 0.5, item_cy),
                            Vec2::new(combo_w, item_h),
                        );
                        let ref_rect = Rect::from_center_size(
                            Pos2::new(x3 + ref_w * 0.5, item_cy),
                            Vec2::new(ref_w, item_h),
                        );

                        // 1) Status Badge (`Zalo Video · ZRTP`)
                        let pill_col = if has_err {
                            pal.danger.linear_multiply(0.18)
                        } else {
                            pal.accent_online.linear_multiply(0.16)
                        };
                        let pill_fg = if has_err {
                            Color32::from_rgb(252, 165, 165)
                        } else {
                            Color32::from_rgb(110, 231, 183)
                        };
                        let badge_txt = if call.is_video {
                            format!("{} Zalo Video · ZRTP", lucide::VIDEO)
                        } else {
                            format!("{} Zalo Voice · ZRTP", lucide::SHIELD_CHECK)
                        };
                        ui.painter()
                            .rect_filled(badge_rect, Rounding::same(8.0), pill_col);
                        ui.painter().text(
                            badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            badge_txt,
                            FontId::proportional(11.5),
                            pill_fg,
                        );

                        let (mics, cams) = crate::media::list_media_devices(false);

                        // 2) Microphone selector button + popup menu
                        let cur_mic_lbl = if self.settings.selected_mic_device.is_empty() {
                            mics.first().map(String::as_str).unwrap_or("Mic mặc định")
                        } else {
                            &self.settings.selected_mic_device
                        };
                        let mic_id = Id::new("call_vp_mic_popup");
                        let mic_resp = ui.interact(mic_rect, mic_id, Sense::click());
                        if mic_resp.clicked() {
                            ui.memory_mut(|m| m.toggle_popup(mic_id));
                        }
                        ui.painter().rect_filled(
                            mic_rect,
                            Rounding::same(8.0),
                            if mic_resp.hovered() {
                                pal.bg_hover
                            } else {
                                pal.bg_input
                            },
                        );
                        ui.painter().rect_stroke(
                            mic_rect,
                            Rounding::same(8.0),
                            Stroke::new(1.0_f32, pal.border),
                        );
                        ui.painter().text(
                            Pos2::new(mic_rect.left() + 10.0, item_cy),
                            egui::Align2::LEFT_CENTER,
                            format!(
                                "{} {}",
                                lucide::MIC,
                                truncate_chars(cur_mic_lbl, combo_chars)
                            ),
                            FontId::proportional(11.5),
                            pal.text_primary,
                        );
                        draw_lucide_icon(
                            ui.painter(),
                            Pos2::new(mic_rect.right() - 12.0, item_cy),
                            lucide::CHEVRON_DOWN,
                            11.5,
                            pal.text_secondary,
                        );
                        egui::popup::popup_below_widget(
                            ui,
                            mic_id,
                            &mic_resp,
                            egui::PopupCloseBehavior::CloseOnClick,
                            |ui| {
                                ui.set_min_width(combo_w.max(190.0));
                                if ui
                                    .selectable_label(
                                        self.settings.selected_mic_device.is_empty(),
                                        "Mặc định hệ thống",
                                    )
                                    .clicked()
                                {
                                    self.settings.selected_mic_device.clear();
                                    crate::media::set_selected_mic_device("");
                                    self.settings.save();
                                }
                                for m in &mics {
                                    if ui
                                        .selectable_label(
                                            self.settings.selected_mic_device == *m,
                                            m,
                                        )
                                        .clicked()
                                    {
                                        self.settings.selected_mic_device = m.clone();
                                        crate::media::set_selected_mic_device(m);
                                        self.settings.save();
                                    }
                                }
                            },
                        );

                        // 3) Camera selector button + popup menu
                        let cur_cam_lbl = if self.settings.selected_camera_device.is_empty() {
                            cams.first()
                                .map(String::as_str)
                                .unwrap_or("Camera mặc định")
                        } else {
                            &self.settings.selected_camera_device
                        };
                        let cam_id = Id::new("call_vp_cam_popup");
                        let cam_resp = ui.interact(cam_rect, cam_id, Sense::click());
                        if cam_resp.clicked() {
                            ui.memory_mut(|m| m.toggle_popup(cam_id));
                        }
                        ui.painter().rect_filled(
                            cam_rect,
                            Rounding::same(8.0),
                            if cam_resp.hovered() {
                                pal.bg_hover
                            } else {
                                pal.bg_input
                            },
                        );
                        ui.painter().rect_stroke(
                            cam_rect,
                            Rounding::same(8.0),
                            Stroke::new(1.0_f32, pal.border),
                        );
                        ui.painter().text(
                            Pos2::new(cam_rect.left() + 10.0, item_cy),
                            egui::Align2::LEFT_CENTER,
                            format!(
                                "{} {}",
                                lucide::VIDEO,
                                truncate_chars(cur_cam_lbl, combo_chars)
                            ),
                            FontId::proportional(11.5),
                            pal.text_primary,
                        );
                        draw_lucide_icon(
                            ui.painter(),
                            Pos2::new(cam_rect.right() - 12.0, item_cy),
                            lucide::CHEVRON_DOWN,
                            11.5,
                            pal.text_secondary,
                        );
                        egui::popup::popup_below_widget(
                            ui,
                            cam_id,
                            &cam_resp,
                            egui::PopupCloseBehavior::CloseOnClick,
                            |ui| {
                                ui.set_min_width(combo_w.max(190.0));
                                if ui
                                    .selectable_label(
                                        self.settings.selected_camera_device.is_empty(),
                                        "Tự động (Camera đầu tiên)",
                                    )
                                    .clicked()
                                {
                                    self.settings.selected_camera_device.clear();
                                    crate::media::set_selected_camera_device("");
                                    self.settings.save();
                                }
                                for c in &cams {
                                    if ui
                                        .selectable_label(
                                            self.settings.selected_camera_device == *c,
                                            c,
                                        )
                                        .clicked()
                                    {
                                        self.settings.selected_camera_device = c.clone();
                                        crate::media::set_selected_camera_device(c);
                                        self.settings.save();
                                    }
                                }
                            },
                        );

                        // 4) Refresh Devices Button
                        let ref_resp =
                            ui.interact(ref_rect, Id::new("call_vp_ref_btn"), Sense::click());
                        ui.painter().rect_filled(
                            ref_rect,
                            Rounding::same(8.0),
                            if ref_resp.hovered() {
                                pal.bg_hover
                            } else {
                                pal.bg_input
                            },
                        );
                        ui.painter().rect_stroke(
                            ref_rect,
                            Rounding::same(8.0),
                            Stroke::new(1.0_f32, pal.border),
                        );
                        draw_icon_refresh(
                            ui.painter(),
                            ref_rect.center(),
                            13.5,
                            if ref_resp.hovered() {
                                pal.text_primary
                            } else {
                                pal.text_secondary
                            },
                        );
                        if ref_resp
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .on_hover_text("Quét lại danh sách Mic & Camera")
                            .clicked()
                        {
                            let _ = crate::media::list_media_devices(true);
                        }
                    });
                });

            // 2. BOTTOM STATUS PILL + ACTION DOCK PANEL (never clipped at bottom edge)
            let bottom_panel_h = if has_err {
                142.0
            } else if call.is_incoming_ringing {
                132.0
            } else {
                138.0
            };
            egui::TopBottomPanel::bottom("call_vp_bottom_panel")
                .resizable(false)
                .exact_height(bottom_panel_h)
                .show_separator_line(false)
                .frame(
                    egui::Frame::none()
                        .fill(pal.bg_sidebar)
                        .inner_margin(Margin {
                            left: 20.0,
                            right: 20.0,
                            top: 4.0,
                            bottom: 16.0,
                        }),
                )
                .show(vp_ctx, |ui| {
                    let max_bot_w = (vp_w - 40.0).max(280.0);
                    ui.set_max_width(max_bot_w);
                    ui.vertical_centered(|ui| {
                        if let Some(err_msg) = &call.error_reason {
                            ui.label(
                                RichText::new("Cuộc gọi chưa kết nối được")
                                    .size(12.5)
                                    .color(Color32::from_rgb(248, 113, 113))
                                    .strong(),
                            );
                            ui.add_space(6.0);
                            let clean_err = err_msg
                                .strip_prefix("Lỗi khởi tạo cuộc gọi (401): ")
                                .unwrap_or(err_msg);
                            let banner_w = max_bot_w.min(440.0);
                            let (err_rect, _) =
                                ui.allocate_exact_size(Vec2::new(banner_w, 34.0), Sense::hover());
                            ui.painter().rect_filled(
                                err_rect,
                                Rounding::same(10.0),
                                Color32::from_rgba_unmultiplied(127, 29, 29, 68),
                            );
                            ui.painter().rect_stroke(
                                err_rect,
                                Rounding::same(10.0),
                                Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgba_unmultiplied(239, 68, 68, 110),
                                ),
                            );
                            ui.painter().text(
                                err_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                truncate_chars(clean_err, 58),
                                FontId::proportional(12.0),
                                Color32::from_rgb(254, 226, 226),
                            );
                        } else {
                            let status_full = format!("{}  ·  {}", call.state_label, timer_str);
                            let text_w = ui.fonts(|f| {
                                f.layout_no_wrap(
                                    status_full,
                                    FontId::proportional(13.0),
                                    Color32::WHITE,
                                )
                                .size()
                                .x
                            });
                            let pill_w = (text_w + 32.0).clamp(180.0, max_bot_w);
                            let (pill_rect, _) =
                                ui.allocate_exact_size(Vec2::new(pill_w, 28.0), Sense::hover());
                            ui.painter().rect_filled(
                                pill_rect,
                                Rounding::same(14.0),
                                Color32::from_rgb(30, 41, 59),
                            );
                            ui.painter().rect_stroke(
                                pill_rect,
                                Rounding::same(14.0),
                                Stroke::new(1.0_f32, pal.border),
                            );
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                &call.state_label,
                                0.0,
                                egui::TextFormat {
                                    font_id: FontId::proportional(12.5),
                                    color: Color32::from_rgb(203, 213, 225),
                                    ..Default::default()
                                },
                            );
                            job.append(
                                "  ·  ",
                                0.0,
                                egui::TextFormat {
                                    font_id: FontId::proportional(12.5),
                                    color: pal.text_muted,
                                    ..Default::default()
                                },
                            );
                            job.append(
                                &timer_str,
                                0.0,
                                egui::TextFormat {
                                    font_id: FontId::monospace(13.0),
                                    color: pal.accent_online,
                                    ..Default::default()
                                },
                            );
                            let galley = ui.fonts(|f| f.layout_job(job));
                            ui.painter().galley(
                                Pos2::new(
                                    pill_rect.center().x - galley.size().x * 0.5,
                                    pill_rect.center().y - galley.size().y * 0.5,
                                ),
                                galley,
                                Color32::WHITE,
                            );
                        }

                        ui.add_space(10.0);

                        if has_err {
                            let dock_w = 216.0_f32;
                            let (dock_rect, _) =
                                ui.allocate_exact_size(Vec2::new(dock_w, 40.0), Sense::hover());
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(dock_rect), |ui| {
                                ui.horizontal_centered(|ui| {
                                    ui.spacing_mut().item_spacing.x = 12.0;
                                    let retry_btn = egui::Button::new(
                                        RichText::new(format!("{} Gọi lại", lucide::PHONE_CALL))
                                            .size(12.8)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(pal.primary)
                                    .rounding(Rounding::same(18.0))
                                    .min_size(Vec2::new(104.0, 36.0));
                                    if ui
                                        .add(retry_btn)
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        retry_clicked = true;
                                    }

                                    let close_btn = egui::Button::new(
                                        RichText::new(format!("{} Đóng", lucide::X))
                                            .size(12.8)
                                            .color(Color32::from_rgb(226, 232, 240))
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(30, 41, 59))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(51, 65, 85)))
                                    .rounding(Rounding::same(18.0))
                                    .min_size(Vec2::new(100.0, 36.0));
                                    if ui
                                        .add(close_btn)
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        hangup_clicked = true;
                                    }
                                });
                            });
                        } else if call.is_incoming_ringing {
                            let dock_w = if call.is_video { 256.0_f32 } else { 156.0_f32 };
                            let (dock_rect, _) =
                                ui.allocate_exact_size(Vec2::new(dock_w, 74.0), Sense::hover());
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(dock_rect), |ui| {
                                ui.horizontal_centered(|ui| {
                                    ui.spacing_mut().item_spacing.x =
                                        if call.is_video { 28.0 } else { 44.0 };
                                    // 1) Decline button
                                    ui.vertical_centered(|ui| {
                                        let (dec_r, dec_resp) = ui
                                            .allocate_exact_size(Vec2::splat(52.0), Sense::click());
                                        ui.painter().circle_filled(
                                            dec_r.center(),
                                            25.0,
                                            pal.danger,
                                        );
                                        draw_lucide_icon(
                                            ui.painter(),
                                            dec_r.center(),
                                            lucide::PHONE_OFF,
                                            20.0,
                                            Color32::WHITE,
                                        );
                                        if dec_resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .on_hover_text("Từ chối cuộc gọi (Esc)")
                                            .clicked()
                                            || vp_ctx.input(|i| i.key_pressed(Key::Escape))
                                        {
                                            if let Some(tx) = WORKER_TX.get() {
                                                let _ = tx.send(WorkerCmd::AnswerCall {
                                                    accept: false,
                                                    cam_start_off: false,
                                                });
                                            }
                                            self.active_call = None;
                                        }
                                        ui.label(
                                            RichText::new("Từ chối")
                                                .size(11.0)
                                                .color(pal.text_secondary),
                                        );
                                    });

                                    // 2) Answer Video Call with Camera Off (`camStartOff`)
                                    if call.is_video {
                                        ui.vertical_centered(|ui| {
                                            let (nc_r, nc_resp) = ui.allocate_exact_size(
                                                Vec2::splat(52.0),
                                                Sense::click(),
                                            );
                                            ui.painter().circle_filled(
                                                nc_r.center(),
                                                25.0,
                                                Color32::from_rgb(51, 65, 85),
                                            );
                                            ui.painter().circle_stroke(
                                                nc_r.center(),
                                                25.0,
                                                Stroke::new(1.2_f32, pal.primary_hover),
                                            );
                                            draw_lucide_icon(
                                                ui.painter(),
                                                nc_r.center(),
                                                lucide::VIDEO_OFF,
                                                19.5,
                                                Color32::WHITE,
                                            );
                                            if nc_resp
                                                .on_hover_cursor(CursorIcon::PointingHand)
                                                .on_hover_text("Trả lời không mở camera")
                                                .clicked()
                                            {
                                                call.is_incoming_ringing = false;
                                                call.cam_enabled = false;
                                                call.state_label = "Đang kết nối cuộc gọi…".into();
                                                if let Some(tx) = WORKER_TX.get() {
                                                    let _ = tx.send(WorkerCmd::AnswerCall {
                                                        accept: true,
                                                        cam_start_off: true,
                                                    });
                                                }
                                            }
                                            ui.label(
                                                RichText::new("Tắt sẵn cam")
                                                    .size(11.0)
                                                    .color(pal.text_primary),
                                            );
                                        });
                                    }

                                    // 3) Accept Call (with camera on if video call)
                                    ui.vertical_centered(|ui| {
                                        let (acc_r, acc_resp) = ui
                                            .allocate_exact_size(Vec2::splat(52.0), Sense::click());
                                        ui.painter().circle_filled(
                                            acc_r.center(),
                                            25.0,
                                            pal.accent_online,
                                        );
                                        draw_lucide_icon(
                                            ui.painter(),
                                            acc_r.center(),
                                            if call.is_video {
                                                lucide::VIDEO
                                            } else {
                                                lucide::PHONE_CALL
                                            },
                                            20.0,
                                            Color32::WHITE,
                                        );
                                        if acc_resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .on_hover_text("Nghe máy (Enter)")
                                            .clicked()
                                            || vp_ctx.input(|i| i.key_pressed(Key::Enter))
                                        {
                                            call.is_incoming_ringing = false;
                                            call.state_label = "Đang kết nối cuộc gọi…".into();
                                            if let Some(tx) = WORKER_TX.get() {
                                                let _ = tx.send(WorkerCmd::AnswerCall {
                                                    accept: true,
                                                    cam_start_off: false,
                                                });
                                            }
                                        }
                                        ui.label(
                                            RichText::new(if call.is_video {
                                                "Bật cam"
                                            } else {
                                                "Nghe máy"
                                            })
                                            .size(11.0)
                                            .color(pal.accent_online),
                                        );
                                    });
                                });
                            });
                        } else {
                            let dock_w = if call.is_group
                                || self.sticker_textures.contains_key("call_remote_video_1")
                            {
                                260.0_f32
                            } else {
                                196.0_f32
                            };
                            let (dock_rect, _) =
                                ui.allocate_exact_size(Vec2::new(dock_w, 58.0), Sense::hover());
                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(dock_rect), |ui| {
                                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                    ui.spacing_mut().item_spacing.x = 22.0;

                                    // 1) Circular Mic Button
                                    let (mic_r, mic_resp) =
                                        ui.allocate_exact_size(Vec2::splat(48.0), Sense::click());
                                    let mic_bg = if call.mic_muted {
                                        pal.warning.linear_multiply(0.28)
                                    } else if mic_resp.hovered() {
                                        Color32::from_rgb(51, 65, 85)
                                    } else {
                                        Color32::from_rgb(30, 41, 59)
                                    };
                                    ui.painter().circle_filled(mic_r.center(), 23.0, mic_bg);
                                    ui.painter().circle_stroke(
                                        mic_r.center(),
                                        23.0,
                                        Stroke::new(
                                            1.0_f32,
                                            if call.mic_muted {
                                                pal.warning
                                            } else {
                                                Color32::from_rgb(71, 85, 105)
                                            },
                                        ),
                                    );
                                    draw_lucide_icon(
                                        ui.painter(),
                                        mic_r.center(),
                                        if call.mic_muted {
                                            lucide::MIC_OFF
                                        } else {
                                            lucide::MIC
                                        },
                                        18.5,
                                        if call.mic_muted {
                                            pal.warning
                                        } else {
                                            Color32::WHITE
                                        },
                                    );
                                    if mic_resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text(if call.mic_muted {
                                            "Bật Mic"
                                        } else {
                                            "Tắt Mic"
                                        })
                                        .clicked()
                                    {
                                        call.mic_muted = !call.mic_muted;
                                        if let Some(tx) = WORKER_TX.get() {
                                            let _ = tx.send(WorkerCmd::SetCallMediaState {
                                                is_on_mic: !call.mic_muted,
                                                is_on_cam: call.cam_enabled,
                                            });
                                        }
                                    }

                                    // 2) Prominent Circular Red Hangup Button (dead-center of window)
                                    let (end_r, end_resp) =
                                        ui.allocate_exact_size(Vec2::splat(56.0), Sense::click());
                                    ui.painter().circle_filled(
                                        end_r.center(),
                                        27.0,
                                        if end_resp.hovered() {
                                            Color32::from_rgb(220, 38, 38)
                                        } else {
                                            pal.danger
                                        },
                                    );
                                    draw_lucide_icon(
                                        ui.painter(),
                                        end_r.center(),
                                        lucide::PHONE_OFF,
                                        21.0,
                                        Color32::WHITE,
                                    );
                                    if end_resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text("Kết thúc cuộc gọi")
                                        .clicked()
                                    {
                                        hangup_clicked = true;
                                    }

                                    // 3) Circular Camera Button
                                    let (cam_r, cam_resp) =
                                        ui.allocate_exact_size(Vec2::splat(48.0), Sense::click());
                                    let cam_bg = if call.cam_enabled {
                                        pal.primary.linear_multiply(0.32)
                                    } else if cam_resp.hovered() {
                                        Color32::from_rgb(51, 65, 85)
                                    } else {
                                        Color32::from_rgb(30, 41, 59)
                                    };
                                    ui.painter().circle_filled(cam_r.center(), 23.0, cam_bg);
                                    ui.painter().circle_stroke(
                                        cam_r.center(),
                                        23.0,
                                        Stroke::new(
                                            1.0_f32,
                                            if call.cam_enabled {
                                                pal.primary_hover
                                            } else {
                                                Color32::from_rgb(71, 85, 105)
                                            },
                                        ),
                                    );
                                    draw_lucide_icon(
                                        ui.painter(),
                                        cam_r.center(),
                                        if call.cam_enabled {
                                            lucide::VIDEO
                                        } else {
                                            lucide::VIDEO_OFF
                                        },
                                        18.5,
                                        Color32::WHITE,
                                    );
                                    if cam_resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text(if call.cam_enabled {
                                            "Tắt Camera"
                                        } else {
                                            "Bật Camera"
                                        })
                                        .clicked()
                                    {
                                        if !call.cam_enabled {
                                            if !crate::media::is_ffmpeg_available() {
                                                self.show_toast(
                                                    "Cần cài đặt ffmpeg trong PATH để bật Camera",
                                                );
                                            } else if crate::media::list_media_devices(false)
                                                .1
                                                .is_empty()
                                            {
                                                self.show_toast(
                                                    "Không tìm thấy thiết bị Camera nào trên máy",
                                                );
                                            } else {
                                                call.cam_enabled = true;
                                                call.is_video = true;
                                            }
                                        } else {
                                            call.cam_enabled = false;
                                            self.sticker_textures.remove("call_local_video");
                                        }
                                        if let Some(tx) = WORKER_TX.get() {
                                            let _ = tx.send(WorkerCmd::SetCallMediaState {
                                                is_on_mic: !call.mic_muted,
                                                is_on_cam: call.cam_enabled,
                                            });
                                        }
                                    }
                                    // 4) Layout Switcher (Gallery <-> Speaker View) & Live Mic Level Meter
                                    if call.is_group
                                        || self.sticker_textures.contains_key("call_remote_video_1")
                                    {
                                        let (lay_r, lay_resp) = ui
                                            .allocate_exact_size(Vec2::splat(42.0), Sense::click());
                                        let lay_bg = if call.speaker_view {
                                            pal.primary.linear_multiply(0.32)
                                        } else if lay_resp.hovered() {
                                            Color32::from_rgb(51, 65, 85)
                                        } else {
                                            Color32::from_rgb(30, 41, 59)
                                        };
                                        ui.painter().circle_filled(lay_r.center(), 20.0, lay_bg);
                                        ui.painter().circle_stroke(
                                            lay_r.center(),
                                            20.0,
                                            Stroke::new(1.0_f32, pal.border),
                                        );
                                        draw_lucide_icon(
                                            ui.painter(),
                                            lay_r.center(),
                                            lucide::PANEL_RIGHT,
                                            16.5,
                                            Color32::WHITE,
                                        );
                                        if lay_resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .on_hover_text(if call.speaker_view {
                                                "Chế độ Người nói (Bấm để chuyển sang Lưới Gallery)"
                                            } else {
                                                "Chế độ Lưới Gallery (Bấm để chuyển sang Người nói)"
                                            })
                                            .clicked()
                                        {
                                            call.speaker_view = !call.speaker_view;
                                        }
                                    }
                                });
                            });

                            // Live Mic Level Bar + Quick Gain Slider under dock
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                let lvl = (crate::media::mic_level_permille() as f32 / 1000.0)
                                    .clamp(0.0, 1.0);
                                let (bar_r, _) =
                                    ui.allocate_exact_size(Vec2::new(76.0, 5.0), Sense::hover());
                                ui.painter().rect_filled(
                                    bar_r,
                                    Rounding::same(2.5),
                                    Color32::from_rgb(30, 41, 59),
                                );
                                if !call.mic_muted && lvl > 0.005 {
                                    let fw = (bar_r.width() * lvl).max(3.0);
                                    let fcol = if lvl > 0.85 {
                                        pal.danger
                                    } else if lvl > 0.6 {
                                        pal.warning
                                    } else {
                                        pal.accent_online
                                    };
                                    ui.painter().rect_filled(
                                        Rect::from_min_size(
                                            bar_r.min,
                                            Vec2::new(fw, bar_r.height()),
                                        ),
                                        Rounding::same(2.5),
                                        fcol,
                                    );
                                }
                                let mut g = self.settings.mic_gain_percent;
                                if ui
                                    .add_sized(
                                        [92.0, 16.0],
                                        egui::Slider::new(&mut g, 0..=200)
                                            .show_value(true)
                                            .suffix("%"),
                                    )
                                    .on_hover_text("Độ nhạy Micro (0% – 200%)")
                                    .changed()
                                {
                                    self.settings.mic_gain_percent = g;
                                    crate::media::set_mic_gain_percent(g);
                                    self.settings.save();
                                }
                            });
                        }
                    });
                });

            // 3. CENTRAL VIDEO / AVATAR STAGE PANEL (uses exact remaining height between top & bottom panels)
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(pal.bg_sidebar)
                        .inner_margin(Margin::symmetric(20.0, 8.0)),
                )
                .show(vp_ctx, |ui| {
                    let stage_avail_w = (vp_w - 40.0).max(280.0);
                    let stage_avail_h = ui.available_height().max(180.0);
                    ui.set_max_width(stage_avail_w);

                    ui.vertical_centered(|ui| {
                        if is_video_stage {
                            let (stage_rect, _) = ui.allocate_exact_size(
                                Vec2::new(stage_avail_w, stage_avail_h),
                                Sense::hover(),
                            );
                            let painter = ui.painter();
                            painter.rect_filled(
                                stage_rect,
                                Rounding::same(16.0),
                                Color32::from_rgb(15, 20, 30),
                            );
                            painter.rect_stroke(
                                stage_rect,
                                Rounding::same(16.0),
                                Stroke::new(1.0_f32, pal.border),
                            );

                            let is_multi_stage = call.is_group
                                || self.sticker_textures.contains_key("call_remote_video_1");
                            if is_multi_stage {
                                let inner = stage_rect.shrink(12.0);
                                let n_tiles = group_remotes.len().clamp(2, 6);
                                let gap = 10.0_f32;
                                let mut aspects = Vec::with_capacity(n_tiles);
                                for idx in 0..n_tiles {
                                    let tk = if idx == 0 {
                                        "call_remote_video".to_string()
                                    } else {
                                        format!("call_remote_video_{idx}")
                                    };
                                    let a = self
                                        .sticker_textures
                                        .get(&tk)
                                        .map(|t| {
                                            let s = t.size_vec2();
                                            if s.y > 0.0 {
                                                s.x / s.y
                                            } else {
                                                16.0 / 9.0
                                            }
                                        })
                                        .unwrap_or(16.0 / 9.0);
                                    aspects.push(a);
                                }
                                let tile_rects: Vec<Rect> = if call.speaker_view && n_tiles >= 2 {
                                    let (main_r, strip_rs) = compute_call_speaker_layout(
                                        n_tiles - 1,
                                        inner,
                                        gap,
                                        16.0 / 9.0,
                                    );
                                    let pinned = call.pinned_tile_idx.min(n_tiles - 1);
                                    let mut out = vec![Rect::NOTHING; n_tiles];
                                    out[pinned] = main_r;
                                    let mut s_i = 0;
                                    for (i, item) in out.iter_mut().enumerate() {
                                        if i == pinned {
                                            continue;
                                        }
                                        if let Some(sr) = strip_rs.get(s_i).copied() {
                                            *item = sr;
                                        } else {
                                            *item = main_r;
                                        }
                                        s_i += 1;
                                    }
                                    out
                                } else if n_tiles <= 2 {
                                    compute_call_fit_aspects(&aspects, inner, gap)
                                } else {
                                    compute_call_gallery_tiles(n_tiles, inner, gap, 16.0 / 9.0)
                                };
                                let remote_speaking = crate::media::is_remote_speaking();

                                for (idx, &tile_rect) in tile_rects.iter().enumerate() {
                                    let tile_resp = ui.interact(
                                        tile_rect,
                                        Id::new(("call_group_tile", idx)),
                                        Sense::click(),
                                    );
                                    if tile_resp
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text(
                                            "Bấm để ghim người này lên khung chính (Speaker View)",
                                        )
                                        .clicked()
                                    {
                                        call.pinned_tile_idx = idx;
                                        call.speaker_view = true;
                                    }
                                    painter.rect_filled(
                                        tile_rect,
                                        Rounding::same(14.0),
                                        Color32::from_rgb(10, 14, 22),
                                    );
                                    let is_active_speaker = idx == 0 && remote_speaking;
                                    painter.rect_stroke(
                                        tile_rect,
                                        Rounding::same(14.0),
                                        Stroke::new(
                                            if is_active_speaker { 2.4_f32 } else { 1.0_f32 },
                                            if is_active_speaker {
                                                pal.accent_online
                                            } else {
                                                pal.border
                                            },
                                        ),
                                    );

                                    let tex_key = if idx == 0 {
                                        "call_remote_video".to_string()
                                    } else {
                                        format!("call_remote_video_{idx}")
                                    };
                                    let (ref p_name, ref p_ava) = group_remotes[idx];
                                    let has_vid = if let Some(vtex) =
                                        self.sticker_textures.get(&tex_key)
                                    {
                                        let t_inner = tile_rect.shrink(6.0);
                                        let tex_size = vtex.size_vec2();
                                        let tex_aspect = if tex_size.y > 0.0 {
                                            tex_size.x / tex_size.y
                                        } else {
                                            4.0 / 3.0
                                        };
                                        let box_aspect =
                                            t_inner.width() / t_inner.height().max(1.0);
                                        let draw_size = if box_aspect > tex_aspect {
                                            Vec2::new(
                                                t_inner.height() * tex_aspect,
                                                t_inner.height(),
                                            )
                                        } else {
                                            Vec2::new(t_inner.width(), t_inner.width() / tex_aspect)
                                        };
                                        let draw_rect =
                                            Rect::from_center_size(t_inner.center(), draw_size);
                                        painter.image(
                                            vtex.id(),
                                            draw_rect,
                                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                            Color32::WHITE,
                                        );
                                        true
                                    } else {
                                        let center = Pos2::new(
                                            tile_rect.center().x,
                                            tile_rect.center().y - 16.0,
                                        );
                                        let pulse = ((elapsed_f * 2.6 + idx as f32).sin() * 0.5
                                            + 0.5)
                                            .clamp(0.0, 1.0);
                                        painter.circle_filled(
                                            center,
                                            40.0 + pulse * 5.0,
                                            pal.primary.linear_multiply(0.12),
                                        );
                                        painter.circle_stroke(
                                            center,
                                            37.0,
                                            Stroke::new(
                                                1.6_f32,
                                                pal.primary_hover.linear_multiply(0.8),
                                            ),
                                        );
                                        paint_user_avatar(
                                            painter,
                                            center,
                                            34.0,
                                            p_name,
                                            false,
                                            p_ava.as_deref(),
                                            &self.sticker_textures,
                                            &mut self.loading_stickers,
                                        );
                                        painter.text(
                                            Pos2::new(tile_rect.center().x, center.y + 52.0),
                                            egui::Align2::CENTER_CENTER,
                                            p_name,
                                            FontId::proportional(15.5),
                                            pal.text_primary,
                                        );
                                        painter.text(
                                            Pos2::new(tile_rect.center().x, center.y + 72.0),
                                            egui::Align2::CENTER_CENTER,
                                            &call.state_label,
                                            FontId::proportional(11.5),
                                            pal.text_muted,
                                        );
                                        false
                                    };

                                    // Participant Name Badge in bottom-left of tile
                                    let lbl_w = (p_name.chars().count() as f32 * 7.2 + 34.0)
                                        .clamp(76.0, tile_rect.width() - 20.0);
                                    let tag_rect = Rect::from_min_size(
                                        Pos2::new(
                                            tile_rect.left() + 10.0,
                                            tile_rect.bottom() - 30.0,
                                        ),
                                        Vec2::new(lbl_w, 22.0),
                                    );
                                    painter.rect_filled(
                                        tag_rect,
                                        Rounding::same(11.0),
                                        Color32::from_rgba_unmultiplied(8, 11, 18, 195),
                                    );
                                    painter.circle_filled(
                                        Pos2::new(tag_rect.left() + 11.0, tag_rect.center().y),
                                        3.8,
                                        if has_vid {
                                            pal.accent_online
                                        } else {
                                            pal.warning
                                        },
                                    );
                                    painter.text(
                                        Pos2::new(tag_rect.left() + 20.0, tag_rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        truncate_chars(p_name, 18),
                                        FontId::proportional(11.5),
                                        Color32::WHITE,
                                    );
                                }
                            } else if let Some(vtex) =
                                self.sticker_textures.get("call_remote_video")
                            {
                                let inner = stage_rect.shrink(10.0);
                                let tex_size = vtex.size_vec2();
                                let tex_aspect = if tex_size.y > 0.0 {
                                    tex_size.x / tex_size.y
                                } else {
                                    4.0 / 3.0
                                };
                                let box_aspect = inner.width() / inner.height().max(1.0);
                                let draw_size = if box_aspect > tex_aspect {
                                    Vec2::new(inner.height() * tex_aspect, inner.height())
                                } else {
                                    Vec2::new(inner.width(), inner.width() / tex_aspect)
                                };
                                let draw_rect = Rect::from_center_size(inner.center(), draw_size);
                                painter.rect_filled(
                                    draw_rect.expand(2.0),
                                    Rounding::same(12.0),
                                    Color32::from_rgb(8, 11, 18),
                                );
                                painter.image(
                                    vtex.id(),
                                    draw_rect,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                                painter.rect_stroke(
                                    draw_rect.expand(1.0),
                                    Rounding::same(12.0),
                                    Stroke::new(
                                        1.2_f32,
                                        Color32::from_rgba_unmultiplied(148, 163, 184, 70),
                                    ),
                                );
                            } else {
                                let center =
                                    Pos2::new(stage_rect.center().x, stage_rect.center().y - 16.0);
                                let pulse = ((elapsed_f * 2.6).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                                painter.circle_filled(
                                    center,
                                    46.0 + pulse * 6.0,
                                    pal.primary.linear_multiply(0.12),
                                );
                                painter.circle_stroke(
                                    center,
                                    43.0,
                                    Stroke::new(1.8_f32, pal.primary_hover.linear_multiply(0.8)),
                                );
                                paint_user_avatar(
                                    painter,
                                    center,
                                    40.0,
                                    &call.partner_name,
                                    call.is_group,
                                    call_ava.as_deref(),
                                    &self.sticker_textures,
                                    &mut self.loading_stickers,
                                );
                                painter.text(
                                    Pos2::new(stage_rect.center().x, center.y + 58.0),
                                    egui::Align2::CENTER_CENTER,
                                    &call.partner_name,
                                    FontId::proportional(17.5),
                                    pal.text_primary,
                                );
                            }

                            // Local Camera Picture-in-Picture (PIP) preview in bottom-right corner
                            let pip_w = 184.0_f32;
                            let pip_h = 138.0_f32;
                            let pip_rect = Rect::from_min_size(
                                Pos2::new(
                                    stage_rect.right() - pip_w - 16.0,
                                    stage_rect.bottom() - pip_h - 16.0,
                                ),
                                Vec2::new(pip_w, pip_h),
                            );
                            painter.rect_filled(pip_rect, Rounding::same(12.0), pal.bg_elevated);
                            if call.cam_enabled {
                                if let Some(ltex) = self.sticker_textures.get("call_local_video") {
                                    let inner_pip = pip_rect.shrink(2.0);
                                    let lsize = ltex.size_vec2();
                                    let laspect = if lsize.y > 0.0 {
                                        lsize.x / lsize.y
                                    } else {
                                        4.0 / 3.0
                                    };
                                    let paspect = inner_pip.width() / inner_pip.height().max(1.0);
                                    let uv = if laspect > paspect {
                                        let frac = paspect / laspect;
                                        let m = (1.0 - frac) * 0.5;
                                        Rect::from_min_max(
                                            Pos2::new(m, 0.0),
                                            Pos2::new(1.0 - m, 1.0),
                                        )
                                    } else {
                                        let frac = laspect / paspect;
                                        let m = (1.0 - frac) * 0.5;
                                        Rect::from_min_max(
                                            Pos2::new(0.0, m),
                                            Pos2::new(1.0, 1.0 - m),
                                        )
                                    };
                                    painter.image(ltex.id(), inner_pip, uv, Color32::WHITE);
                                } else {
                                    let self_disp = if self.user_name.is_empty() {
                                        "Tôi"
                                    } else {
                                        &self.user_name
                                    };
                                    paint_user_avatar(
                                        painter,
                                        Pos2::new(pip_rect.center().x, pip_rect.center().y - 10.0),
                                        20.0,
                                        self_disp,
                                        false,
                                        self.user_avatar.as_deref(),
                                        &self.sticker_textures,
                                        &mut self.loading_stickers,
                                    );
                                }
                            } else {
                                let self_disp = if self.user_name.is_empty() {
                                    "Tôi"
                                } else {
                                    &self.user_name
                                };
                                paint_user_avatar(
                                    painter,
                                    Pos2::new(pip_rect.center().x, pip_rect.center().y - 10.0),
                                    20.0,
                                    self_disp,
                                    false,
                                    self.user_avatar.as_deref(),
                                    &self.sticker_textures,
                                    &mut self.loading_stickers,
                                );
                            }
                            let pip_badge_rect = Rect::from_center_size(
                                Pos2::new(pip_rect.center().x, pip_rect.bottom() - 14.0),
                                Vec2::new(124.0, 20.0),
                            );
                            painter.rect_filled(
                                pip_badge_rect,
                                Rounding::same(10.0),
                                Color32::from_rgba_unmultiplied(8, 11, 18, 185),
                            );
                            let pip_status = if call.cam_enabled {
                                format!("{} Camera đang bật", lucide::VIDEO)
                            } else {
                                format!("{} Camera đã tắt", lucide::VIDEO_OFF)
                            };
                            painter.text(
                                pip_badge_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                pip_status,
                                FontId::proportional(10.5),
                                if call.cam_enabled {
                                    pal.accent_online
                                } else {
                                    pal.text_muted
                                },
                            );
                            painter.rect_stroke(
                                pip_rect,
                                Rounding::same(12.0),
                                Stroke::new(
                                    1.4_f32,
                                    if call.cam_enabled {
                                        pal.primary_hover
                                    } else {
                                        pal.border
                                    },
                                ),
                            );
                        } else {
                            let top_pad = ((stage_avail_h - 150.0) * 0.45).max(16.0);
                            ui.add_space(top_pad);
                            let (av_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(96.0), Sense::hover());
                            let center = av_rect.center();
                            let ring_col = if has_err {
                                pal.danger
                            } else if call.is_incoming_ringing {
                                pal.accent_online
                            } else {
                                pal.primary_hover
                            };
                            let pulse = ((elapsed_f * 2.6).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                            ui.painter().circle_filled(
                                center,
                                44.0 + pulse * 5.0,
                                ring_col.linear_multiply(0.10),
                            );
                            ui.painter().circle_stroke(
                                center,
                                42.0,
                                Stroke::new(1.8_f32, ring_col.linear_multiply(0.75)),
                            );
                            paint_user_avatar(
                                ui.painter(),
                                center,
                                38.0,
                                &call.partner_name,
                                call.is_group,
                                call_ava.as_deref(),
                                &self.sticker_textures,
                                &mut self.loading_stickers,
                            );

                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(&call.partner_name)
                                    .size(18.5)
                                    .color(pal.text_primary)
                                    .strong(),
                            );
                        }
                    });
                });
        });

        if retry_clicked {
            self.sticker_textures.remove("call_remote_video");
            self.sticker_textures.remove("call_remote_video_1");
            self.sticker_textures.remove("call_remote_video_2");
            self.sticker_textures.remove("call_remote_video_3");
            call.error_code = None;
            call.error_reason = None;
            call.started_at = std::time::Instant::now();
            call.state_label = if call.is_video {
                "Đang kết nối cuộc gọi video…".into()
            } else {
                "Đang kết nối cuộc gọi thoại…".into()
            };
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::StartCall {
                    partner_id: call.partner_id.clone(),
                    partner_name: call.partner_name.clone(),
                    partner_avatar: None,
                    is_video: call.is_video,
                    is_group: call.is_group,
                });
            }
            self.active_call = Some(call);
        } else if hangup_clicked {
            self.active_call = None;
            self.sticker_textures.remove("call_remote_video");
            self.sticker_textures.remove("call_remote_video_1");
            self.sticker_textures.remove("call_remote_video_2");
            self.sticker_textures.remove("call_remote_video_3");
            self.sticker_textures.remove("call_local_video");
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::EndCall);
            }
        } else if self.active_call.is_some() {
            self.active_call = Some(call);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_gallery_fit_aspects_and_speaker_layout_stay_within_bounds() {
        let stage = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(800.0, 500.0));
        for n in [1, 2, 3, 4, 6, 9] {
            let tiles = compute_call_gallery_tiles(n, stage, 8.0, 16.0 / 9.0);
            assert_eq!(tiles.len(), n);
            for r in tiles {
                assert!(r.width() > 0.0 && r.height() > 0.0);
                assert!(r.left() >= stage.left() - 1.0 && r.right() <= stage.right() + 1.0);
                assert!(r.top() >= stage.top() - 1.0 && r.bottom() <= stage.bottom() + 1.0);
            }
        }

        let mixed = compute_call_fit_aspects(&[9.0 / 16.0, 16.0 / 9.0], stage, 10.0);
        assert_eq!(mixed.len(), 2);
        assert!(mixed[0].width() < mixed[0].height());
        assert!(mixed[1].width() > mixed[1].height());

        let (main_r, strip) = compute_call_speaker_layout(3, stage, 8.0, 16.0 / 9.0);
        assert!(main_r.width() > 400.0);
        assert_eq!(strip.len(), 3);
    }
}

//! Image viewer modal for Alolite (Full-screen preview, pan, zoom, copy, and open external).

use crate::gui::messages::open_external_url;
use crate::gui::theme::truncate_chars;
use crate::gui::theme::{draw_icon_image, draw_lucide_icon, lucide, ThemePalette};
use crate::gui::ZaloGuiApp;
use eframe::egui::{
    self, Align, Color32, CursorIcon, FontId, Id, Key, Layout, Margin, Pos2, Rect, RichText,
    Rounding, Sense, Stroke, Vec2,
};

impl ZaloGuiApp {
    pub(super) fn render_image_viewer_modal(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        let Some(mut viewer) = self.image_viewer.clone() else {
            return;
        };

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.image_viewer = None;
            return;
        }

        let screen_rect = ctx.screen_rect();
        let viewer_top = (screen_rect.top() + 32.0).min(screen_rect.bottom());
        let toolbar_h = 46.0_f32;
        let toolbar_rect = Rect::from_min_max(
            Pos2::new(screen_rect.left(), viewer_top),
            Pos2::new(
                screen_rect.right(),
                (viewer_top + toolbar_h).min(screen_rect.bottom()),
            ),
        );
        let stage_area_rect = Rect::from_min_max(
            Pos2::new(screen_rect.left(), toolbar_rect.bottom()),
            screen_rect.max,
        );
        let mut should_close = false;

        let active_tex = viewer
            .hd_tex_key
            .as_ref()
            .and_then(|hk| self.sticker_textures.get(hk))
            .or_else(|| self.sticker_textures.get(&viewer.tex_key))
            .cloned();

        let max_stage_w = (stage_area_rect.width() - 48.0).max(160.0);
        let max_stage_h = (stage_area_rect.height() - 36.0).max(120.0);

        let (base_w, base_h, natural_dims) = if let Some(ref tex) = active_tex {
            let [tw, th] = tex.size();
            let tw_f = (tw.max(1)) as f32;
            let th_f = (th.max(1)) as f32;
            let fit_scale = (max_stage_w / tw_f).min(max_stage_h / th_f).min(1.5);
            (tw_f * fit_scale, th_f * fit_scale, Some((tw, th)))
        } else {
            (340.0_f32.min(max_stage_w), 220.0_f32.min(max_stage_h), None)
        };

        // Mouse-wheel zoom
        let scroll_y = ctx.input(|i| {
            if i.raw_scroll_delta.y.abs() > 0.1 {
                i.raw_scroll_delta.y
            } else {
                i.smooth_scroll_delta.y
            }
        });
        if scroll_y.abs() > 0.1 {
            let old_zoom = viewer.zoom;
            let factor = if scroll_y > 0.0 { 1.15 } else { 1.0 / 1.15 };
            let new_zoom = (old_zoom * factor).clamp(0.25, 8.0);
            if let Some(ptr) = ctx.input(|i| i.pointer.hover_pos()) {
                if stage_area_rect.contains(ptr) {
                    let rel = ptr - (stage_area_rect.center() + viewer.pan);
                    viewer.pan -= rel * (new_zoom / old_zoom - 1.0);
                }
            }
            viewer.zoom = new_zoom;
        }

        let disp_w = (base_w * viewer.zoom).max(48.0);
        let disp_h = (base_h * viewer.zoom).max(48.0);

        let max_pan_x = ((disp_w - stage_area_rect.width() + 40.0) * 0.5).max(0.0);
        let max_pan_y = ((disp_h - stage_area_rect.height() + 40.0) * 0.5).max(0.0);
        viewer.pan.x = viewer.pan.x.clamp(-max_pan_x, max_pan_x);
        viewer.pan.y = viewer.pan.y.clamp(-max_pan_y, max_pan_y);

        // 1. Unified Clipped Stage Canvas
        egui::Area::new(Id::new("image_viewer_backdrop"))
            .order(egui::Order::Foreground)
            .constrain(false)
            .fixed_pos(stage_area_rect.min)
            .show(ctx, |ui| {
                let (bg_rect, bg_resp) =
                    ui.allocate_exact_size(stage_area_rect.size(), Sense::click_and_drag());
                let painter = ui.painter().with_clip_rect(bg_rect);
                let bg_col = if pal.is_dark {
                    Color32::from_rgba_unmultiplied(24, 26, 31, 244)
                } else {
                    Color32::from_rgba_unmultiplied(24, 28, 36, 228)
                };
                painter.rect_filled(bg_rect, Rounding::ZERO, bg_col);

                let img_center = bg_rect.center() + viewer.pan;
                let img_rect = Rect::from_center_size(img_center, Vec2::new(disp_w, disp_h));

                if let Some(ref tex) = active_tex {
                    painter.image(
                        tex.id(),
                        img_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    painter.rect_stroke(img_rect, Rounding::ZERO, Stroke::new(1.0_f32, pal.border));

                    if bg_resp.dragged_by(egui::PointerButton::Primary) {
                        viewer.pan += bg_resp.drag_delta();
                        viewer.pan.x = viewer.pan.x.clamp(-max_pan_x, max_pan_x);
                        viewer.pan.y = viewer.pan.y.clamp(-max_pan_y, max_pan_y);
                        ctx.set_cursor_icon(CursorIcon::Grabbing);
                    } else if let Some(ptr) = bg_resp.hover_pos() {
                        if img_rect.contains(ptr) && (max_pan_x > 1.0 || max_pan_y > 1.0) {
                            ctx.set_cursor_icon(CursorIcon::Grab);
                        }
                    }

                    if bg_resp.double_clicked() {
                        if (viewer.zoom - 1.0).abs() < 0.05 {
                            viewer.zoom = 2.0;
                        } else {
                            viewer.zoom = 1.0;
                            viewer.pan = Vec2::ZERO;
                        }
                    } else if bg_resp.clicked() {
                        let ptr = bg_resp
                            .interact_pointer_pos()
                            .or_else(|| ctx.input(|i| i.pointer.latest_pos()));
                        if ptr.map(|p| !img_rect.contains(p)).unwrap_or(false) {
                            should_close = true;
                        }
                    }
                } else {
                    painter.rect_filled(img_rect, Rounding::same(8.0), pal.bg_elevated);
                    painter.rect_stroke(
                        img_rect,
                        Rounding::same(8.0),
                        Stroke::new(1.0_f32, pal.border),
                    );
                    painter.text(
                        img_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Đang tải hình ảnh…",
                        FontId::proportional(13.0),
                        pal.text_primary,
                    );
                    if bg_resp.clicked() {
                        should_close = true;
                    }
                }
            });

        // 2. Clean Unified Top Toolbar Bar
        let clean_title = {
            let mut t = viewer.title.trim().trim_start_matches(lucide::IMAGE).trim();
            if let Some(paren_idx) = t.rfind(" (") {
                if t[paren_idx..].contains('×')
                    || t[paren_idx..].contains("KB")
                    || t[paren_idx..].contains("MB")
                {
                    t = t[..paren_idx].trim();
                }
            }
            if t.is_empty() {
                "Hình ảnh đính kèm"
            } else {
                t
            }
        };

        egui::Area::new(Id::new("image_viewer_top_bar"))
            .order(egui::Order::Tooltip)
            .fixed_pos(toolbar_rect.min)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(pal.bg_sidebar)
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .inner_margin(Margin::symmetric(16.0, 6.0))
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(
                            (toolbar_rect.width() - 32.0).max(200.0),
                            toolbar_h - 12.0,
                        ));
                        ui.set_max_size(Vec2::new(
                            (toolbar_rect.width() - 32.0).max(200.0),
                            toolbar_h - 12.0,
                        ));
                        ui.horizontal_centered(|ui| {
                            let (badge_r, _) =
                                ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                            ui.painter()
                                .rect_filled(badge_r, Rounding::same(6.0), pal.bg_elevated);
                            draw_icon_image(
                                ui.painter(),
                                badge_r.center(),
                                15.0,
                                pal.primary_hover,
                            );
                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.label(
                                    RichText::new(truncate_chars(clean_title, 44))
                                        .size(13.0)
                                        .color(pal.text_primary)
                                        .strong(),
                                );
                                let dim_str = match natural_dims {
                                    Some((tw, th)) => format!(" · {tw}×{th} px"),
                                    None => String::new(),
                                };
                                let sub = if viewer.time.is_empty() {
                                    format!("{}{dim_str}", viewer.sender)
                                } else {
                                    format!(
                                        "Gửi bởi {} lúc {}{dim_str}",
                                        viewer.sender, viewer.time
                                    )
                                };
                                ui.label(RichText::new(sub).size(11.0).color(pal.text_muted));
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;

                                let (close_r, close_resp) =
                                    ui.allocate_exact_size(Vec2::new(30.0, 28.0), Sense::click());
                                ui.painter().rect_filled(
                                    close_r,
                                    Rounding::same(6.0),
                                    if close_resp.hovered() {
                                        pal.danger
                                    } else {
                                        pal.bg_elevated
                                    },
                                );
                                ui.painter().rect_stroke(
                                    close_r,
                                    Rounding::same(6.0),
                                    Stroke::new(1.0_f32, pal.border),
                                );
                                draw_lucide_icon(
                                    ui.painter(),
                                    close_r.center(),
                                    lucide::X,
                                    14.5,
                                    if close_resp.hovered() {
                                        Color32::WHITE
                                    } else {
                                        pal.text_secondary
                                    },
                                );
                                if close_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Đóng xem ảnh (Esc)")
                                    .clicked()
                                {
                                    should_close = true;
                                }

                                if !viewer.full_url.is_empty() {
                                    let ext_btn = egui::Button::new(
                                        RichText::new(format!("{} Mở gốc", lucide::EXTERNAL_LINK))
                                            .size(11.8)
                                            .color(pal.text_primary),
                                    )
                                    .fill(pal.bg_elevated)
                                    .stroke(Stroke::new(1.0_f32, pal.border))
                                    .rounding(Rounding::same(6.0))
                                    .min_size(Vec2::new(72.0, 28.0));
                                    if ui
                                        .add(ext_btn)
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text(&viewer.full_url)
                                        .clicked()
                                    {
                                        open_external_url(&viewer.full_url);
                                    }

                                    let copy_btn = egui::Button::new(
                                        RichText::new(format!("{} Sao chép", lucide::COPY))
                                            .size(11.8)
                                            .color(pal.text_secondary),
                                    )
                                    .fill(pal.bg_elevated)
                                    .stroke(Stroke::new(1.0_f32, pal.border))
                                    .rounding(Rounding::same(6.0))
                                    .min_size(Vec2::new(78.0, 28.0));
                                    if ui
                                        .add(copy_btn)
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        ui.ctx().copy_text(viewer.full_url.clone());
                                        self.show_toast("Đã sao chép đường dẫn ảnh vào bộ nhớ tạm");
                                    }
                                }

                                let zin_btn = egui::Button::new(
                                    RichText::new(lucide::PLUS)
                                        .size(12.5)
                                        .color(pal.text_secondary),
                                )
                                .fill(pal.bg_elevated)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(28.0, 28.0));
                                if ui
                                    .add(zin_btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Phóng to (+25%)")
                                    .clicked()
                                {
                                    viewer.zoom = (viewer.zoom + 0.25).min(8.0);
                                }

                                let zoom_pct = format!("{:.0}%", viewer.zoom * 100.0);
                                let zreset_btn = egui::Button::new(
                                    RichText::new(zoom_pct)
                                        .size(11.8)
                                        .color(pal.primary_hover)
                                        .strong(),
                                )
                                .fill(pal.bg_input)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(50.0, 28.0));
                                if ui
                                    .add(zreset_btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Đặt lại 100% (hoặc nhấp đúp vào ảnh)")
                                    .clicked()
                                {
                                    viewer.zoom = 1.0;
                                    viewer.pan = Vec2::ZERO;
                                }

                                let zout_btn = egui::Button::new(
                                    RichText::new(lucide::MINUS)
                                        .size(12.5)
                                        .color(pal.text_secondary),
                                )
                                .fill(pal.bg_elevated)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(28.0, 28.0));
                                if ui
                                    .add(zout_btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Thu nhỏ (-25%)")
                                    .clicked()
                                {
                                    viewer.zoom = (viewer.zoom - 0.25).max(0.25);
                                    if viewer.zoom <= 1.0 {
                                        viewer.pan = Vec2::ZERO;
                                    }
                                }
                            });
                        });
                    });
            });

        if should_close {
            self.image_viewer = None;
        } else {
            self.image_viewer = Some(viewer);
        }
    }
}

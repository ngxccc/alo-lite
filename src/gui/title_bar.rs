//! Custom title bar and frameless window resizing logic for Alolite.

use crate::gui::theme::{lucide, ThemePalette};
use crate::gui::ZaloGuiApp;
use eframe::egui::{
    self, Align, Color32, CursorIcon, Id, Layout, Margin, Pos2, Rect, RichText, Rounding, Sense,
    Stroke, Vec2,
};

impl ZaloGuiApp {
    /// Handles drag-resizing from edges on custom frameless windows.
    pub(super) fn handle_frameless_resize(ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
            return;
        }
        let screen = ctx.screen_rect();
        let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
            return;
        };
        let border = 5.0_f32;
        let corner = 9.0_f32;
        let left = (pos.x - screen.left()).abs() <= border;
        let right = (screen.right() - pos.x).abs() <= border;
        let top = (pos.y - screen.top()).abs() <= border;
        let bottom = (screen.bottom() - pos.y).abs() <= border;

        let left_c = (pos.x - screen.left()).abs() <= corner;
        let right_c = (screen.right() - pos.x).abs() <= corner;
        let top_c = (pos.y - screen.top()).abs() <= corner;
        let bottom_c = (screen.bottom() - pos.y).abs() <= corner;

        let dir_and_cursor = if top_c && left_c && (top || left) {
            Some((egui::ResizeDirection::NorthWest, CursorIcon::ResizeNwSe))
        } else if top_c && right_c && (top || right) {
            Some((egui::ResizeDirection::NorthEast, CursorIcon::ResizeNeSw))
        } else if bottom_c && left_c && (bottom || left) {
            Some((egui::ResizeDirection::SouthWest, CursorIcon::ResizeNeSw))
        } else if bottom_c && right_c && (bottom || right) {
            Some((egui::ResizeDirection::SouthEast, CursorIcon::ResizeNwSe))
        } else if left {
            Some((egui::ResizeDirection::West, CursorIcon::ResizeHorizontal))
        } else if right {
            Some((egui::ResizeDirection::East, CursorIcon::ResizeHorizontal))
        } else if top {
            Some((egui::ResizeDirection::North, CursorIcon::ResizeVertical))
        } else if bottom {
            Some((egui::ResizeDirection::South, CursorIcon::ResizeVertical))
        } else {
            None
        };

        if let Some((dir, cursor)) = dir_and_cursor {
            ctx.set_cursor_icon(cursor);
            if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
            }
        }
    }

    /// Renders the custom native title bar (App icon, title, profile, window controls).
    pub(super) fn render_custom_title_bar(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let total_unread: usize = self.unread_counts.values().sum();
        if self.last_synced_icon_unread != Some(total_unread) {
            self.last_synced_icon_unread = Some(total_unread);
            if let Some(icon) = crate::gui::theme::load_app_icon_with_badge(total_unread) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Icon(Some(std::sync::Arc::new(icon))));
            }
        }

        egui::TopBottomPanel::top("custom_title_bar")
            .exact_height(32.0)
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_app)
                    .inner_margin(Margin {
                        left: 12.0,
                        right: 0.0,
                        top: 0.0,
                        bottom: 0.0,
                    })
                    .stroke(Stroke::new(1.0_f32, pal.border)),
            )
            .show(ctx, |ui| {
                let bar_rect = ui.max_rect();
                let mut buttons_left_x = bar_rect.right();

                ui.horizontal_centered(|ui| {
                    // 1. App Logo (20x20)
                    let (logo_rect, _) =
                        ui.allocate_exact_size(Vec2::new(20.0, 20.0), Sense::hover());
                    if let Some(tex) = &self.app_logo_texture {
                        ui.painter().image(
                            tex.id(),
                            logo_rect,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    } else {
                        ui.painter()
                            .rect_filled(logo_rect, Rounding::same(5.0), pal.primary);
                    }

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("AloLite")
                            .color(pal.text_primary)
                            .size(13.5)
                            .strong(),
                    );

                    // Unread badge pill in titlebar
                    if total_unread > 0 {
                        egui::Frame::none()
                            .fill(pal.accent_online.linear_multiply(0.18))
                            .stroke(Stroke::new(1.0_f32, pal.accent_online))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::symmetric(6.0, 1.5))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("{total_unread} tin mới"))
                                        .size(10.5)
                                        .color(pal.accent_online)
                                        .strong(),
                                );
                            });
                    } else if self.is_logged_in && !self.user_name.is_empty() {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!("·  {}", self.user_name))
                                .color(pal.text_secondary)
                                .size(12.5),
                        );
                    }

                    // 2. Right-side Window Controls
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Close button (Hover turns Red)
                        let (rect_close, resp_close) =
                            ui.allocate_exact_size(Vec2::new(36.0, 32.0), Sense::click());
                        if resp_close.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let is_hover_close = resp_close.hovered();
                        if is_hover_close {
                            ui.painter().rect_filled(
                                rect_close,
                                Rounding::ZERO,
                                Color32::from_rgb(220, 38, 38),
                            );
                        }
                        let close_icon_color = if is_hover_close {
                            Color32::WHITE
                        } else {
                            pal.text_secondary
                        };
                        let c = rect_close.center();
                        let half = 4.5;
                        ui.painter().line_segment(
                            [
                                Pos2::new(c.x - half, c.y - half),
                                Pos2::new(c.x + half, c.y + half),
                            ],
                            Stroke::new(1.2, close_icon_color),
                        );
                        ui.painter().line_segment(
                            [
                                Pos2::new(c.x + half, c.y - half),
                                Pos2::new(c.x - half, c.y + half),
                            ],
                            Stroke::new(1.2, close_icon_color),
                        );

                        // Maximize / Restore button
                        let (rect_max, resp_max) =
                            ui.allocate_exact_size(Vec2::new(34.0, 32.0), Sense::click());
                        if resp_max.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        }
                        if resp_max.hovered() {
                            ui.painter()
                                .rect_filled(rect_max, Rounding::ZERO, pal.bg_elevated);
                        }
                        let max_icon_color = if resp_max.hovered() {
                            pal.text_primary
                        } else {
                            pal.text_secondary
                        };
                        let c = rect_max.center();
                        if is_maximized {
                            ui.painter().rect_stroke(
                                Rect::from_center_size(
                                    Pos2::new(c.x - 1.5, c.y + 1.5),
                                    Vec2::splat(8.0),
                                ),
                                Rounding::ZERO,
                                Stroke::new(1.2, max_icon_color),
                            );
                            ui.painter().rect_stroke(
                                Rect::from_center_size(
                                    Pos2::new(c.x + 1.5, c.y - 1.5),
                                    Vec2::splat(8.0),
                                ),
                                Rounding::ZERO,
                                Stroke::new(1.2, max_icon_color),
                            );
                        } else {
                            ui.painter().rect_stroke(
                                Rect::from_center_size(c, Vec2::splat(9.0)),
                                Rounding::ZERO,
                                Stroke::new(1.2, max_icon_color),
                            );
                        }

                        // Minimize button
                        let (rect_min, resp_min) =
                            ui.allocate_exact_size(Vec2::new(34.0, 32.0), Sense::click());
                        if resp_min.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        if resp_min.hovered() {
                            ui.painter()
                                .rect_filled(rect_min, Rounding::ZERO, pal.bg_elevated);
                        }
                        let min_icon_color = if resp_min.hovered() {
                            pal.text_primary
                        } else {
                            pal.text_secondary
                        };
                        let c = rect_min.center();
                        ui.painter().line_segment(
                            [Pos2::new(c.x - 4.5, c.y), Pos2::new(c.x + 4.5, c.y)],
                            Stroke::new(1.2, min_icon_color),
                        );

                        // Dev inspector toggle (only in dev mode)
                        if self.dev_mode {
                            let (dev_rect, dev_resp) =
                                ui.allocate_exact_size(Vec2::new(30.0, 32.0), Sense::click());
                            if dev_resp.clicked() {
                                self.dev_panel_open = !self.dev_panel_open;
                            }
                            if dev_resp.hovered() || self.dev_panel_open {
                                ui.painter()
                                    .rect_filled(dev_rect, Rounding::ZERO, pal.bg_elevated);
                            }
                            let dev_color = if self.dev_panel_open {
                                pal.primary
                            } else {
                                pal.text_muted
                            };
                            ui.painter().text(
                                dev_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "</>",
                                egui::FontId::monospace(11.5),
                                dev_color,
                            );
                        }

                        // Quick Theme Toggle (36x32)
                        let (th_rect, th_resp) =
                            ui.allocate_exact_size(Vec2::new(36.0, 32.0), Sense::click());
                        buttons_left_x = th_rect.left();
                        if th_resp.hovered() {
                            ui.painter()
                                .rect_filled(th_rect, Rounding::ZERO, pal.bg_hover);
                        }
                        let th_icon = if self.dark_mode {
                            lucide::SUN
                        } else {
                            lucide::MOON
                        };
                        ui.painter().text(
                            th_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            th_icon,
                            egui::FontId::proportional(13.5),
                            if th_resp.hovered() {
                                pal.text_primary
                            } else {
                                pal.text_muted
                            },
                        );
                        if th_resp
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .on_hover_text("Chuyển đổi giao diện Sáng / Tối")
                            .clicked()
                        {
                            self.dark_mode = !self.dark_mode;
                            self.theme_dirty = true;
                        }

                        // Compact update pill in Title Bar
                        if let Some(upd) = &self.update_info {
                            if upd.has_update {
                                let (pill_r, resp_pill) =
                                    ui.allocate_exact_size(Vec2::new(76.0, 22.0), Sense::click());
                                buttons_left_x = pill_r.left();
                                let is_hov = resp_pill.hovered();
                                ui.painter().rect_filled(
                                    pill_r,
                                    Rounding::same(11.0),
                                    if is_hov {
                                        Color32::from_rgb(234, 179, 8).linear_multiply(0.28)
                                    } else {
                                        Color32::from_rgb(234, 179, 8).linear_multiply(0.18)
                                    },
                                );
                                ui.painter().rect_stroke(
                                    pill_r,
                                    Rounding::same(11.0),
                                    Stroke::new(1.0, Color32::from_rgb(234, 179, 8)),
                                );
                                ui.painter().text(
                                    pill_r.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{} v{}", lucide::SPARKLES, upd.latest_version),
                                    egui::FontId::proportional(11.0),
                                    Color32::from_rgb(234, 179, 8),
                                );
                                if resp_pill.clicked() {
                                    self.update_modal_open = true;
                                    self.update_banner_dismissed = false;
                                }
                            }
                        }

                        // Offline saved history button
                        if !self.is_logged_in && !self.threads.is_empty() {
                            ui.add_space(8.0);
                            let hist_btn = egui::Button::new(
                                RichText::new("Xem lịch sử đã lưu")
                                    .size(11.5)
                                    .color(pal.primary)
                                    .strong(),
                            )
                            .fill(pal.primary.linear_multiply(0.12))
                            .stroke(Stroke::new(1.0_f32, pal.primary))
                            .rounding(Rounding::same(5.0));
                            let r = ui.add(hist_btn).on_hover_cursor(CursorIcon::PointingHand);
                            buttons_left_x = r.rect.left() - 8.0;
                            if r.clicked() {
                                self.is_logged_in = true;
                            }
                        }
                    });
                });

                // 3. Interactive drag region
                let drag_rect = Rect::from_min_max(
                    bar_rect.min,
                    Pos2::new(
                        (buttons_left_x - 2.0).max(bar_rect.left() + 40.0),
                        bar_rect.max.y,
                    ),
                );
                let drag_resp = ui.interact(
                    drag_rect,
                    Id::new("custom_title_bar_drag"),
                    Sense::click_and_drag(),
                );
                if drag_resp.double_clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                } else if drag_resp.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });
    }
}
